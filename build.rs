use std::path::PathBuf;

fn main() {
    println!("cargo:rerun-if-changed=assets/RixIcon.ico");

    if std::env::var("CARGO_CFG_TARGET_OS").as_deref() == Ok("windows") {
        let icon_path = PathBuf::from("assets/RixIcon.ico");
        let manifest_dir = 
            PathBuf::from(std::env::var("CARGO_MANIFEST_DIR").unwrap());
        let icon_path = manifest_dir.join("assets").join("RixIcon.ico");

        let mut resource = winres::WindowsResource::new();
        resource.set_icon(
            icon_path
                .to_str()
                .expect("icon path must be valid UTF-8"),
        );

        resource
            .compile()
            .expect("Failed to compile resources");
    }
}