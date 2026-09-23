//! Упаковка каналов. Вся арифметика — в линейном пространстве, в `f32` 0..1.

use image::imageops::FilterType;
use image::{DynamicImage, GenericImageView, ImageBuffer, Luma};
use rayon::prelude::*;

use crate::model::{MAX_SIZE, SizePolicy, SourceChannel};
use crate::source::PREVIEW_MAX;

/// Что нужно знать об одном выходном канале, чтобы его собрать.
#[derive(Clone, Copy)]
pub struct SlotInput<'a> {
    pub image: Option<&'a DynamicImage>,
    pub source: SourceChannel,
    pub srgb: bool,
    pub invert: bool,
    pub fill: u8,
}

/// sRGB → линейное, точная кривая IEC 61966-2-1.
pub fn srgb_to_linear(v: f32) -> f32 {
    if v <= 0.04045 { v / 12.92 } else { ((v + 0.055) / 1.055).powf(2.4) }
}

/// Размер результата по размерам исходников. `None` — исходников нет, а размер
/// не задан вручную.
pub fn target_size(sizes: &[(u32, u32)], policy: SizePolicy, custom: (u32, u32)) -> Option<(u32, u32)> {
    let area = |s: &&(u32, u32)| u64::from(s.0) * u64::from(s.1);
    match policy {
        SizePolicy::Custom => Some((custom.0.clamp(1, MAX_SIZE), custom.1.clamp(1, MAX_SIZE))),
        SizePolicy::Largest => sizes.iter().max_by_key(area).copied(),
        SizePolicy::Smallest => sizes.iter().min_by_key(area).copied(),
    }
}

/// Размер предпросмотра: результат, вписанный в `PREVIEW_MAX`.
pub fn preview_size((w, h): (u32, u32)) -> (u32, u32) {
    let scale = (PREVIEW_MAX as f32 / w.max(h) as f32).min(1.0);
    (((w as f32 * scale).round() as u32).max(1), ((h as f32 * scale).round() as u32).max(1))
}

/// Один канал размером `w`×`h`: чтение, перевод из sRGB, масштаб, инверсия.
pub fn plane(slot: &SlotInput, w: u32, h: u32) -> Vec<f32> {
    let Some(image) = slot.image else {
        return vec![f32::from(slot.fill) / 255.0; w as usize * h as usize];
    };
    let mut values = extract(image, slot.source, slot.srgb);
    let (sw, sh) = image.dimensions();
    if (sw, sh) != (w, h) {
        let buffer: ImageBuffer<Luma<f32>, Vec<f32>> =
            ImageBuffer::from_raw(sw, sh, values).expect("plane matches its image");
        let filter = if sw * sh > w * h { FilterType::Triangle } else { FilterType::CatmullRom };
        values = image::imageops::resize(&buffer, w, h, filter).into_raw();
    }
    let invert = slot.invert;
    values.par_iter_mut().for_each(|v| {
        let c = v.clamp(0.0, 1.0);
        *v = if invert { 1.0 - c } else { c };
    });
    values
}

/// Значение одного пикселя результата — для подсказки под курсором. Берётся
/// ближайший пиксель исходника, без фильтра масштабирования.
pub fn sample(slot: &SlotInput, x: u32, y: u32, (w, h): (u32, u32)) -> f32 {
    let Some(image) = slot.image else {
        return f32::from(slot.fill) / 255.0;
    };
    let (sw, sh) = image.dimensions();
    let sx = ((u64::from(x) * u64::from(sw)) / u64::from(w.max(1))).min(u64::from(sw - 1)) as u32;
    let sy = ((u64::from(y) * u64::from(sh)) / u64::from(h.max(1))).min(u64::from(sh - 1)) as u32;
    let v = extract(&image.crop_imm(sx, sy, 1, 1), slot.source, slot.srgb)[0].clamp(0.0, 1.0);
    if slot.invert { 1.0 - v } else { v }
}

/// Прочитать канал исходника как есть по размеру, в линейных значениях.
fn extract(image: &DynamicImage, source: SourceChannel, srgb: bool) -> Vec<f32> {
    const U8: f32 = 1.0 / 255.0;
    const U16: f32 = 1.0 / 65535.0;
    match image {
        DynamicImage::ImageLuma8(b) => gather(b.as_raw(), 1, source, srgb, |v| f32::from(v) * U8),
        DynamicImage::ImageLumaA8(b) => gather(b.as_raw(), 2, source, srgb, |v| f32::from(v) * U8),
        DynamicImage::ImageRgb8(b) => gather(b.as_raw(), 3, source, srgb, |v| f32::from(v) * U8),
        DynamicImage::ImageRgba8(b) => gather(b.as_raw(), 4, source, srgb, |v| f32::from(v) * U8),
        DynamicImage::ImageLuma16(b) => gather(b.as_raw(), 1, source, srgb, |v| f32::from(v) * U16),
        DynamicImage::ImageLumaA16(b) => gather(b.as_raw(), 2, source, srgb, |v| f32::from(v) * U16),
        DynamicImage::ImageRgb16(b) => gather(b.as_raw(), 3, source, srgb, |v| f32::from(v) * U16),
        DynamicImage::ImageRgba16(b) => gather(b.as_raw(), 4, source, srgb, |v| f32::from(v) * U16),
        DynamicImage::ImageRgb32F(b) => gather(b.as_raw(), 3, source, srgb, |v| v),
        DynamicImage::ImageRgba32F(b) => gather(b.as_raw(), 4, source, srgb, |v| v),
        other => extract(&DynamicImage::ImageRgba32F(other.to_rgba32f()), source, srgb),
    }
}

/// Общая часть `extract` для любого типа отсчёта.
fn gather<T: Copy + Sync>(
    raw: &[T],
    channels: usize,
    source: SourceChannel,
    srgb: bool,
    norm: impl Fn(T) -> f32 + Sync,
) -> Vec<f32> {
    let gray = channels <= 2;
    let alpha = match channels {
        2 => Some(1),
        4 => Some(3),
        _ => None,
    };
    // Альфа всегда линейна: sRGB касается только цвета.
    let color = |v: T| {
        let v = norm(v);
        if srgb { srgb_to_linear(v.clamp(0.0, 1.0)) } else { v }
    };
    let index = |c: usize| if gray { 0 } else { c };
    let mut out = vec![0.0f32; raw.len() / channels];
    out.par_chunks_mut(8192).enumerate().for_each(|(chunk, dst)| {
        let start = chunk * 8192;
        for (i, o) in dst.iter_mut().enumerate() {
            let px = &raw[(start + i) * channels..(start + i + 1) * channels];
            *o = match source {
                SourceChannel::R => color(px[0]),
                SourceChannel::G => color(px[index(1)]),
                SourceChannel::B => color(px[index(2)]),
                SourceChannel::A => alpha.map_or(1.0, |a| norm(px[a])),
                SourceChannel::Lum if gray => color(px[0]),
                SourceChannel::Lum => 0.2126 * color(px[0]) + 0.7152 * color(px[1]) + 0.0722 * color(px[2]),
            };
        }
    });
    out
}

/// Каналы предпросмотра: четыре плоскости размером `w`×`h`.
pub fn compose(slots: &[SlotInput; 4], (w, h): (u32, u32)) -> [Vec<f32>; 4] {
    let mut planes: [Vec<f32>; 4] = Default::default();
    planes.par_iter_mut().enumerate().for_each(|(i, p)| *p = plane(&slots[i], w, h));
    planes
}

pub fn to_u8(v: f32) -> u8 {
    (v.clamp(0.0, 1.0) * 255.0 + 0.5) as u8
}

fn to_u16(v: f32) -> u16 {
    (v.clamp(0.0, 1.0) * 65535.0 + 0.5) as u16
}

/// Собрать результат в полном размере. Каналы считаются по одному, чтобы на
/// 8K в памяти не висели все четыре плоскости сразу.
pub fn pack(slots: &[SlotInput; 4], alpha: bool, (w, h): (u32, u32), sixteen: bool) -> DynamicImage {
    let n = if alpha { 4 } else { 3 };
    let pixels = w as usize * h as usize;
    if sixteen {
        let mut out = vec![0u16; pixels * n];
        for (c, slot) in slots.iter().take(n).enumerate() {
            let values = plane(slot, w, h);
            out.par_chunks_mut(n).zip(values.par_iter()).for_each(|(px, v)| px[c] = to_u16(*v));
        }
        if alpha {
            DynamicImage::ImageRgba16(ImageBuffer::from_raw(w, h, out).expect("sized buffer"))
        } else {
            DynamicImage::ImageRgb16(ImageBuffer::from_raw(w, h, out).expect("sized buffer"))
        }
    } else {
        let mut out = vec![0u8; pixels * n];
        for (c, slot) in slots.iter().take(n).enumerate() {
            let values = plane(slot, w, h);
            out.par_chunks_mut(n).zip(values.par_iter()).for_each(|(px, v)| px[c] = to_u8(*v));
        }
        if alpha {
            DynamicImage::ImageRgba8(ImageBuffer::from_raw(w, h, out).expect("sized buffer"))
        } else {
            DynamicImage::ImageRgb8(ImageBuffer::from_raw(w, h, out).expect("sized buffer"))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use image::{GrayImage, Rgb, RgbImage, Rgba, RgbaImage};

    fn slot(image: Option<&DynamicImage>, source: SourceChannel) -> SlotInput<'_> {
        SlotInput { image, source, srgb: false, invert: false, fill: 0 }
    }

    #[test]
    fn packs_channels_in_order() {
        let red = DynamicImage::ImageRgb8(RgbImage::from_pixel(2, 2, Rgb([200, 10, 20])));
        let gray = DynamicImage::ImageLuma8(GrayImage::from_pixel(2, 2, Luma([77])));
        let slots = [
            slot(Some(&red), SourceChannel::R),
            slot(Some(&gray), SourceChannel::Lum),
            SlotInput { fill: 255, ..slot(None, SourceChannel::Lum) },
            SlotInput { invert: true, ..slot(Some(&gray), SourceChannel::R) },
        ];
        let out = pack(&slots, true, (2, 2), false).to_rgba8();
        assert_eq!(out.get_pixel(1, 1), &Rgba([200, 77, 255, 178]));
        let rgb = pack(&slots, false, (2, 2), false);
        assert_eq!(rgb.color(), image::ColorType::Rgb8);
    }

    #[test]
    fn alpha_of_opaque_source_is_white() {
        let img = DynamicImage::ImageRgb8(RgbImage::from_pixel(1, 1, Rgb([1, 2, 3])));
        assert_eq!(plane(&slot(Some(&img), SourceChannel::A), 1, 1), vec![1.0]);
    }

    #[test]
    fn srgb_mid_gray_is_linearized() {
        let img = DynamicImage::ImageLuma8(GrayImage::from_pixel(1, 1, Luma([188])));
        let v = plane(&SlotInput { srgb: true, ..slot(Some(&img), SourceChannel::Lum) }, 1, 1)[0];
        assert!((v - 0.5).abs() < 0.01, "{v}");
        // Альфа не линеаризуется.
        let img = DynamicImage::ImageRgba8(RgbaImage::from_pixel(1, 1, Rgba([0, 0, 0, 188])));
        let a = plane(&SlotInput { srgb: true, ..slot(Some(&img), SourceChannel::A) }, 1, 1)[0];
        assert_eq!(to_u8(a), 188);
    }

    #[test]
    fn luminance_uses_rec709() {
        let img = DynamicImage::ImageRgb8(RgbImage::from_pixel(1, 1, Rgb([255, 0, 0])));
        let v = plane(&slot(Some(&img), SourceChannel::Lum), 1, 1)[0];
        assert!((v - 0.2126).abs() < 1e-4);
    }

    #[test]
    fn resizes_to_target() {
        let img = DynamicImage::ImageLuma8(GrayImage::from_pixel(4, 4, Luma([255])));
        let p = plane(&slot(Some(&img), SourceChannel::Lum), 8, 2);
        assert_eq!(p.len(), 16);
        assert!(p.iter().all(|v| (*v - 1.0).abs() < 1e-3));
    }

    #[test]
    fn sixteen_bit_output() {
        let img = DynamicImage::ImageLuma16(ImageBuffer::from_pixel(1, 1, Luma([12345u16])));
        let slots = [slot(Some(&img), SourceChannel::Lum); 4];
        let out = pack(&slots, false, (1, 1), true);
        assert_eq!(out.as_rgb16().unwrap().get_pixel(0, 0).0, [12345; 3]);
    }

    #[test]
    fn sample_matches_plane() {
        let mut img = GrayImage::new(4, 1);
        for x in 0..4 {
            img.put_pixel(x, 0, Luma([x as u8 * 60]));
        }
        let img = DynamicImage::ImageLuma8(img);
        let s = SlotInput { invert: true, ..slot(Some(&img), SourceChannel::Lum) };
        assert_eq!(to_u8(sample(&s, 3, 0, (4, 1))), 255 - 180);
        assert_eq!(to_u8(sample(&s, 1, 0, (2, 1))), 255 - 120);
    }

    #[test]
    fn target_policies() {
        let sizes = [(1024, 1024), (2048, 2048), (512, 512)];
        assert_eq!(target_size(&sizes, SizePolicy::Largest, (1, 1)), Some((2048, 2048)));
        assert_eq!(target_size(&sizes, SizePolicy::Smallest, (1, 1)), Some((512, 512)));
        assert_eq!(target_size(&[], SizePolicy::Custom, (99999, 300)), Some((8192, 300)));
        assert_eq!(target_size(&[], SizePolicy::Largest, (1, 1)), None);
        assert_eq!(preview_size((4096, 2048)), (1024, 512));
    }
}
