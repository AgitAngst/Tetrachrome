//! Значок exe: знак семьи Anvil (бирюза и четыре плитки) — тот же, что в окне и в шапке.

use std::path::PathBuf;

use anvil_ui::{Accent, Icon};
use image::ExtendedColorType;
use image::codecs::ico::{IcoEncoder, IcoFrame};

fn main() {
    println!("cargo:rerun-if-changed=build.rs");
    if std::env::var("CARGO_CFG_TARGET_OS").as_deref() != Ok("windows") {
        return;
    }
    let out = PathBuf::from(std::env::var("OUT_DIR").expect("OUT_DIR"));
    let ico = out.join("tetrachrome.ico");
    let frames: Vec<IcoFrame> = [16u32, 24, 32, 48, 64, 128, 256]
        .iter()
        .map(|&size| {
            let rgba = anvil_ui::appicon::rgba(Accent::TEAL, Icon::Tiles, size);
            IcoFrame::as_png(&rgba, size, size, ExtendedColorType::Rgba8).expect("icon frame")
        })
        .collect();
    IcoEncoder::new(std::fs::File::create(&ico).expect("create icon")).encode_images(&frames).expect("write icon");

    let mut res = winresource::WindowsResource::new();
    res.set_icon(ico.to_str().expect("utf-8 path"));
    res.set("ProductName", "Tetrachrome");
    res.set("FileDescription", "Tetrachrome — texture channel packer");
    if let Err(e) = res.compile() {
        // Без rc.exe программа соберётся, просто без значка у exe.
        println!("cargo:warning=exe icon skipped: {e}");
    }
}
