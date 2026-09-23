//! Значок exe: рисуется тем же кодом, что и значок окна, и вшивается ресурсом.

use std::path::PathBuf;

use image::ExtendedColorType;
use image::codecs::ico::{IcoEncoder, IcoFrame};

include!("src/appicon.rs");

fn main() {
    println!("cargo:rerun-if-changed=src/appicon.rs");
    println!("cargo:rerun-if-changed=build.rs");
    if std::env::var("CARGO_CFG_TARGET_OS").as_deref() != Ok("windows") {
        return;
    }
    let out = PathBuf::from(std::env::var("OUT_DIR").expect("OUT_DIR"));
    let ico = out.join("tetrachrome.ico");
    let frames: Vec<IcoFrame> = [16u32, 24, 32, 48, 64, 128, 256]
        .iter()
        .map(|&size| {
            IcoFrame::as_png(&render_icon(size), size, size, ExtendedColorType::Rgba8).expect("icon frame")
        })
        .collect();
    let file = std::fs::File::create(&ico).expect("create icon");
    IcoEncoder::new(file).encode_images(&frames).expect("write icon");

    let mut res = winresource::WindowsResource::new();
    res.set_icon(ico.to_str().expect("utf-8 path"));
    res.set("ProductName", "Tetrachrome");
    res.set("FileDescription", "Tetrachrome — texture channel packer");
    if let Err(e) = res.compile() {
        // Без rc.exe программа соберётся, просто без значка у exe.
        println!("cargo:warning=exe icon skipped: {e}");
    }
}
