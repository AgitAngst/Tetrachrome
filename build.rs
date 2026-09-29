//! Значок exe — `assets/icon/tetrachrome.ico`: знак Tetrachrome из SVG (`scripts/render-icon.ps1`), ресурсом.
//! Значок окна берёт те же PNG (`src/icon.rs`).

use std::path::PathBuf;

fn main() {
    let ico = PathBuf::from(std::env::var("CARGO_MANIFEST_DIR").expect("CARGO_MANIFEST_DIR"))
        .join("assets/icon/tetrachrome.ico");
    println!("cargo:rerun-if-changed=build.rs");
    println!("cargo:rerun-if-changed={}", ico.display());
    if std::env::var("CARGO_CFG_TARGET_OS").as_deref() != Ok("windows") {
        return;
    }
    let mut res = winresource::WindowsResource::new();
    res.set_icon(ico.to_str().expect("utf-8 path"));
    res.set("ProductName", "Tetrachrome");
    res.set("FileDescription", "Tetrachrome — texture channel packer");
    if let Err(e) = res.compile() {
        // Без rc.exe программа соберётся, просто без значка у exe.
        println!("cargo:warning=exe icon skipped: {e}");
    }
}
