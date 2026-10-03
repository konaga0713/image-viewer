/// lib.rs
use std::ffi::{c_char, c_int, CStr};
use std::path::Path;
use std::time::Duration;

use image::{DynamicImage, RgbaImage};

// Keep these trait definitions and method order in sync with the host plugin ABI.
pub trait ImageDecoderPlugin: Send + Sync {
    fn supported_extensions(&self) -> Vec<&'static str>;
    fn decode(&self, path: &Path) -> Result<Box<dyn ImageContent>, String>;
}

pub trait ImageContent: Send {
    fn current_image(&self) -> &RgbaImage;
    fn size(&self) -> egui::Vec2;
    fn rotate_right(&mut self);
    fn rotate_left(&mut self);
    fn rotation(&self) -> Rotation;
    fn is_animated(&self) -> bool { false }
    fn next_frame(&mut self) -> Option<Duration> { None }
    fn current_delay(&self) -> Option<Duration> { None }
    fn frame_count(&self) -> usize { 1 }
    fn is_modified(&self) -> bool { false }
    fn save(&self, path: &Path) -> Result<(), String>;
    fn process_loading(&mut self) -> bool { false }
    fn is_loading(&self) -> bool { false }
    fn update_loading(&mut self) -> bool { false }
}

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum Rotation { None, Right, Rotate180, Left }

impl Rotation {
    fn right(self) -> Self {
        match self { Self::None => Self::Right, Self::Right => Self::Rotate180,
            Self::Rotate180 => Self::Left, Self::Left => Self::None }
    }
    fn left(self) -> Self {
        match self { Self::None => Self::Left, Self::Right => Self::None,
            Self::Rotate180 => Self::Right, Self::Left => Self::Rotate180 }
    }
}

struct AvifPlugin;

impl ImageDecoderPlugin for AvifPlugin {
    fn supported_extensions(&self) -> Vec<&'static str> { vec!["avif"] }

    fn decode(&self, path: &Path) -> Result<Box<dyn ImageContent>, String> {
        let data = std::fs::read(path).map_err(|e| format!("AVIF read failed: {e}"))?;
        let (frames, delays) = decode_frames(&data)?;
        Ok(Box::new(AvifImage::new(frames, delays)?))
    }
}

struct AvifImage {
    frames: Vec<RgbaImage>,
    delays: Vec<Duration>,
    current_frame: usize,
    displayed: RgbaImage,
    rotation: Rotation,
}

impl AvifImage {
    fn new(frames: Vec<RgbaImage>, delays: Vec<Duration>) -> Result<Self, String> {
        let first = frames.first().ok_or("AVIF contains no decoded frames")?.clone();
        Ok(Self {
            frames,
            delays,
            current_frame: 0,
            displayed: first,
            rotation: Rotation::None,
        })
    }

    fn refresh(&mut self) {
        let frame = &self.frames[self.current_frame];
        self.displayed = match self.rotation {
            Rotation::None => frame.clone(),
            Rotation::Right => DynamicImage::ImageRgba8(frame.clone()).rotate90().to_rgba8(),
            Rotation::Rotate180 => DynamicImage::ImageRgba8(frame.clone()).rotate180().to_rgba8(),
            Rotation::Left => DynamicImage::ImageRgba8(frame.clone()).rotate270().to_rgba8(),
        };
    }
}

impl ImageContent for AvifImage {
    fn current_image(&self) -> &RgbaImage { &self.displayed }
    fn size(&self) -> egui::Vec2 {
        egui::vec2(self.displayed.width() as f32, self.displayed.height() as f32)
    }
    fn rotate_right(&mut self) { self.rotation = self.rotation.right(); self.refresh(); }
    fn rotate_left(&mut self) { self.rotation = self.rotation.left(); self.refresh(); }
    fn rotation(&self) -> Rotation { self.rotation }
    fn is_animated(&self) -> bool { self.frames.len() > 1 }
    fn next_frame(&mut self) -> Option<Duration> {
        if self.frames.len() < 2 { return None; }
        self.current_frame = (self.current_frame + 1) % self.frames.len();
        self.refresh();
        self.current_delay()
    }
    fn current_delay(&self) -> Option<Duration> { self.delays.get(self.current_frame).copied() }
    fn frame_count(&self) -> usize { self.frames.len() }
    fn is_modified(&self) -> bool { self.rotation != Rotation::None }
    fn save(&self, path: &Path) -> Result<(), String> {
        if self.is_animated() {
            return Err("Saving animated AVIF is not supported; refusing to save only one frame".into());
        }
        self.displayed.save(path).map_err(|e| e.to_string())
    }
}

fn decode_frames(data: &[u8]) -> Result<(Vec<RgbaImage>, Vec<Duration>), String> {
    let decoder = unsafe { avifDecoderCreate() };
    if decoder.is_null() { return Err("libavif could not create a decoder".into()); }
    let _guard = DecoderGuard(decoder);

    check_result(unsafe { avifDecoderSetIOMemory(decoder, data.as_ptr(), data.len()) })?;
    check_result(unsafe { avifDecoderParse(decoder) })?;

    let prefix = unsafe { &*decoder.cast::<AvifDecoderPrefix>() };
    if prefix.image_count <= 0 {
        return Err("libavif reported no AVIF frames".into());
    }
    let frame_count = usize::try_from(prefix.image_count)
        .map_err(|_| "AVIF frame count is invalid")?;
    if frame_count > 100_000 {
        return Err(format!("AVIF frame count is unreasonably large: {frame_count}"));
    }

    let mut frames = Vec::new();
    let mut delays = Vec::new();
    frames.try_reserve_exact(frame_count).map_err(|e| format!("AVIF frame allocation failed: {e}"))?;
    delays.try_reserve_exact(frame_count).map_err(|e| format!("AVIF timing allocation failed: {e}"))?;

    for index in 0..frame_count {
        check_result(unsafe { avifDecoderNthImage(decoder, index as u32) })?;
        let prefix = unsafe { &*decoder.cast::<AvifDecoderPrefix>() };
        if prefix.image.is_null() { return Err("libavif returned a null frame".into()); }
        frames.push(unsafe { image_to_rgba(prefix.image)? });

        let mut timing = AvifImageTiming::default();
        let delay = if unsafe { avifDecoderNthImageTiming(decoder, index as u32, &mut timing) } == 0
            && timing.duration.is_finite() && timing.duration > 0.0
        {
            Duration::from_secs_f64(timing.duration)
        } else {
            Duration::from_millis(100)
        };
        delays.push(delay.max(Duration::from_millis(10)));
    }

    Ok((frames, delays))
}

unsafe fn image_to_rgba(image: *const AvifImageRaw) -> Result<RgbaImage, String> {
    let image_ref = unsafe { &*image };
    if image_ref.width == 0 || image_ref.height == 0 {
        return Err("libavif returned an empty frame".into());
    }
    let byte_len = (image_ref.width as usize)
        .checked_mul(image_ref.height as usize)
        .and_then(|n| n.checked_mul(4))
        .ok_or("AVIF frame dimensions overflow")?;
    let mut rgb = AvifRgbImage::default();
    unsafe { avifRGBImageSetDefaults(&mut rgb, image); }
    rgb.format = 1; // AVIF_RGB_FORMAT_RGBA
    rgb.depth = 8;
    check_result(unsafe { avifRGBImageAllocatePixels(&mut rgb) })?;

    let conversion = unsafe { avifImageYUVToRGB(image, &mut rgb) };
    if conversion != 0 {
        unsafe { avifRGBImageFreePixels(&mut rgb); }
        return Err(avif_error(conversion));
    }

    let row_len = (image_ref.width as usize) * 4;
    let mut pixels = Vec::new();
    pixels.try_reserve_exact(byte_len).map_err(|e| {
        unsafe { avifRGBImageFreePixels(&mut rgb); }
        format!("AVIF pixel allocation failed: {e}")
    })?;
    for y in 0..image_ref.height as usize {
        let row = unsafe { rgb.pixels.add(y * rgb.row_bytes as usize) };
        pixels.extend_from_slice(unsafe { std::slice::from_raw_parts(row, row_len) });
    }
    unsafe { avifRGBImageFreePixels(&mut rgb); }
    RgbaImage::from_raw(image_ref.width, image_ref.height, pixels)
        .ok_or_else(|| "Could not create an RGBA frame from libavif output".into())
}

struct DecoderGuard(*mut AvifDecoderPrefix);
impl Drop for DecoderGuard {
    fn drop(&mut self) { unsafe { avifDecoderDestroy(self.0); } }
}

fn check_result(result: c_int) -> Result<(), String> {
    if result == 0 { Ok(()) } else { Err(avif_error(result)) }
}

fn avif_error(result: c_int) -> String {
    let message = unsafe { avifResultToString(result) };
    if message.is_null() { return format!("libavif error {result}"); }
    unsafe { CStr::from_ptr(message) }.to_string_lossy().into_owned()
}

#[repr(C)]
struct AvifDecoderPrefix {
    codec_choice: c_int,
    max_threads: c_int,
    requested_source: c_int,
    allow_progressive: c_int,
    allow_incremental: c_int,
    ignore_exif: c_int,
    ignore_xmp: c_int,
    image_size_limit: u32,
    image_dimension_limit: u32,
    image_count_limit: u32,
    strict_flags: u32,
    image: *mut AvifImageRaw,
    image_index: c_int,
    image_count: c_int,
    progressive_state: c_int,
    image_timing: AvifImageTiming,
}

#[repr(C)]
struct AvifImageRaw {
    width: u32,
    height: u32,
    _depth: u32,
}

#[repr(C)]
#[derive(Default)]
struct AvifImageTiming {
    timescale: u64,
    pts: f64,
    pts_in_timescales: u64,
    duration: f64,
    duration_in_timescales: u64,
}

#[repr(C)]
#[derive(Default)]
struct AvifRgbImage {
    width: u32,
    height: u32,
    depth: u32,
    format: c_int,
    chroma_upsampling: c_int,
    chroma_downsampling: c_int,
    avoid_lib_yuv: c_int,
    ignore_alpha: c_int,
    alpha_premultiplied: c_int,
    is_float: c_int,
    max_threads: c_int,
    pixels: *mut u8,
    row_bytes: u32,
}

#[link(name = "avif")]
unsafe extern "C" {
    fn avifDecoderCreate() -> *mut AvifDecoderPrefix;
    fn avifDecoderDestroy(decoder: *mut AvifDecoderPrefix);
    fn avifDecoderSetIOMemory(decoder: *mut AvifDecoderPrefix, data: *const u8, size: usize) -> c_int;
    fn avifDecoderParse(decoder: *mut AvifDecoderPrefix) -> c_int;
    fn avifDecoderNthImage(decoder: *mut AvifDecoderPrefix, index: u32) -> c_int;
    fn avifDecoderNthImageTiming(decoder: *const AvifDecoderPrefix, index: u32, timing: *mut AvifImageTiming) -> c_int;
    fn avifResultToString(result: c_int) -> *const c_char;
    fn avifRGBImageSetDefaults(rgb: *mut AvifRgbImage, image: *const AvifImageRaw);
    fn avifRGBImageAllocatePixels(rgb: *mut AvifRgbImage) -> c_int;
    fn avifRGBImageFreePixels(rgb: *mut AvifRgbImage);
    fn avifImageYUVToRGB(image: *const AvifImageRaw, rgb: *mut AvifRgbImage) -> c_int;
}

#[unsafe(no_mangle)]
pub fn create_plugin() -> Box<dyn ImageDecoderPlugin> { Box::new(AvifPlugin) }
