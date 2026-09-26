
use std::path::PathBuf;
use image_viewer::gif_animation::GifAnimation;

const GIF_IMAGE : &str = "./pics/gif/1079547319185.gif";
#[test]
fn test_load_gif() {
    let gif_path = PathBuf::from(GIF_IMAGE);
    let animation = GifAnimation::load(&gif_path)
        .expect("faled to read gif file");

    assert!(
        animation.frame_count() > 0,
        "No gif frame"
    );

    println!("frame_count = {}",animation.frame_count());
    println!("current_frame = {}",animation.current_frame());
    println!("current_delay = {:?}",animation.current_delay());
}

#[test]
fn test_next_frame(){
    let gif_path = PathBuf::from(GIF_IMAGE);

    let mut animation = GifAnimation::load(&gif_path)
        .expect("GIFの読み込みに失敗しました");

    let frame_count = animation.frame_count();

    assert!(frame_count > 0);

    assert_eq!(animation.current_frame(), 0);

    for expected in 1..frame_count {
        animation.next_frame();

        assert_eq!(
            animation.current_frame(),
            expected
        );
    }

    // 最終フレームの次は先頭に戻る
    animation.next_frame();

    assert_eq!(
        animation.current_frame(),
        0
    );    
}


#[test]
fn test_frame_image() {
    let gif_path = PathBuf::from(GIF_IMAGE);

    let animation = GifAnimation::load(&gif_path)
        .expect("GIFの読み込みに失敗しました");

    let image = animation.current_image();

    assert!(
        image.width() > 0,
        "画像の幅が0です"
    );

    assert!(
        image.height() > 0,
        "画像の高さが0です"
    );

    println!(
        "size = {} x {}",
        image.width(),
        image.height()
    );

}