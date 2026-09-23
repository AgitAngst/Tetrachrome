//! Тестовые карты для ручной проверки: `cargo run --example samples -- <папка>`.
//!
//! Два материала: Rock (1024², все карты) и Wood (512², без AO, с цветной
//! Albedo, которую пакет не должен никуда определить).

use std::path::PathBuf;

use image::{GrayImage, Luma, Rgb, RgbImage};

fn main() {
    let dir = PathBuf::from(std::env::args().nth(1).unwrap_or_else(|| "samples".into()));
    std::fs::create_dir_all(&dir).expect("create dir");

    let gray = |size: u32, f: &dyn Fn(f32, f32) -> f32| {
        GrayImage::from_fn(size, size, |x, y| {
            let v = f(x as f32 / size as f32, y as f32 / size as f32);
            Luma([(v.clamp(0.0, 1.0) * 255.0) as u8])
        })
    };
    let tiles = |u: f32, v: f32| {
        if ((u * 8.0) as i32 + (v * 8.0) as i32) % 2 == 0 { 1.0 } else { 0.0 }
    };
    let rings = |u: f32, v: f32| {
        let d = ((u - 0.5).powi(2) + (v - 0.5).powi(2)).sqrt();
        0.5 + 0.5 * (d * 40.0).sin()
    };
    let ao = |u: f32, v: f32| 1.0 - 0.8 * ((u - 0.5).powi(2) + (v - 0.5).powi(2)).sqrt() * 1.6;
    let gradient = |u: f32, _v: f32| u;

    gray(1024, &tiles).save(dir.join("Rock_Metallic.png")).unwrap();
    gray(1024, &ao).save(dir.join("Rock_AO.png")).unwrap();
    gray(1024, &rings).save(dir.join("Rock_Roughness.png")).unwrap();
    gray(1024, &|u, v| 1.0 - rings(u, v)).save(dir.join("Rock_Smoothness.png")).unwrap();
    gray(1024, &|u, v| {
        if (u - 0.3).abs() < 0.2 && (v - 0.6).abs() < 0.25 { 1.0 } else { 0.0 }
    })
    .save(dir.join("Rock_DetailMask.png"))
    .unwrap();

    gray(512, &gradient).save(dir.join("Wood_Metal.png")).unwrap();
    gray(512, &|u, v| 0.5 + 0.5 * (v * 30.0 + u * 4.0).sin()).save(dir.join("Wood_Rough.png")).unwrap();
    RgbImage::from_fn(512, 512, |x, y| Rgb([120 + (x % 60) as u8, 80 + (y % 40) as u8, 40]))
        .save(dir.join("Wood_Albedo.png"))
        .unwrap();
    println!("samples written to {}", dir.display());
}
