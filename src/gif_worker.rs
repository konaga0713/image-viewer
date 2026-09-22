///gif_worker

use std::path::PathBuf;
use std::sync::mpsc::Sender;
use std::time::Duration;

use image::{AnimationDecoder, codecs::gif::GifDecoder};

pub enum GifLoadMessage {
    Frame {
        image: image::RgbaImage,
        delay: Duration,
    },
    Finished,
    Error(String),
}


pub fn load_gif_worker(path: PathBuf, tx: Sender<GifLoadMessage>, ctx: egui::Context,) {
println!("[GIF WORKER] START: {:?}", path);    
    std::thread::spawn(move || {
        let result = (|| -> Result<(), Box<dyn std::error::Error>> {
            let file = std::fs::File::open(&path)?;
            let reader = std::io::BufReader::new(file);
            let decoder = GifDecoder::new(reader)?;

            for result in decoder.into_frames() {
                match result {
                    Ok(frame) => {
                        let image = frame.buffer().clone();
                        let delay = frame.delay();
                        let (numerator, denominator) = delay.numer_denom_ms();
                        let millis = if denominator == 0 {
                            0
                        } else {
                            numerator / denominator
                        };
                        tx.send(GifLoadMessage::Frame { 
                            image, 
                            delay: Duration::from_millis(millis as u64) 
                        })?;
                        ctx.request_repaint();
                    }
                    Err(err) => {
                        eprintln!(
                            "Gif frame decode waring: {}",
                            err
                        );
                        return Err(Box::new(err));
                    }
                }
            }
            Ok(())
        })();

        match result {
            Ok(()) => {
println!("[GIF WORKER] sending Finished");
                if let Err(e) = tx.send(GifLoadMessage::Finished) {
                    eprintln!("[GIF WORKER] Finished send failed: {}",e);
                } else {
                    println!("[GIF WORKER] Finished sent");
                }            
            }
            Err(err) => {
                eprintln!("[GIF WORKER] Error: {}", err);
                let _ = tx.send(GifLoadMessage::Error(err.to_string()));
            }
        }

        ctx.request_repaint();
    });
}
