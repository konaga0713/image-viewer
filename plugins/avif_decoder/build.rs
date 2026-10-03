/// build.rs
use std::path::PathBuf;

fn main() {
    println!("cargo:rerun-if-env-changed=VCPKG_ROOT");
    let configured_root = std::env::var_os("VCPKG_ROOT").map(PathBuf::from);
    let candidates = configured_root
        .into_iter()
        .chain(std::iter::once(PathBuf::from(r"C:\vcpkg")));
    let lib_dir = candidates
        .map(|root| root.join("installed").join("x64-windows").join("lib"))
        .find(|path| path.join("avif.lib").is_file())
        .unwrap_or_else(|| PathBuf::from(r"C:\vcpkg\installed\x64-windows\lib"));
    let import_library = lib_dir.join("avif.lib");
    if !import_library.is_file() {
        panic!("{} not found; install libavif:x64-windows with vcpkg", import_library.display());
    }
    println!("cargo:rustc-link-search=native={}", lib_dir.display());
    println!("cargo:rustc-link-lib=dylib=avif");
}
