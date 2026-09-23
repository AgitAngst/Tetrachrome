//! Исходные картинки: чтение с диска, уменьшенные копии для предпросмотра.

use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};

use eframe::egui::ColorImage;
use image::imageops::FilterType;
use image::{DynamicImage, GenericImageView, ImageReader};

use crate::model::MAX_SIZE;

/// Сторона предпросмотра: из копий такого размера собирается картинка справа.
pub const PREVIEW_MAX: u32 = 1024;
/// Сторона миниатюры в карточке канала.
const THUMB_MAX: u32 = 192;

/// Расширения, которые показываем в диалоге открытия и берём из папок.
pub const EXTENSIONS: &[&str] = &["png", "tga", "tif", "tiff", "jpg", "jpeg", "bmp", "exr"];

pub struct Source {
    /// Уникален в пределах запуска: по нему живёт текстура миниатюры.
    pub id: u64,
    pub path: PathBuf,
    pub name: String,
    pub width: u32,
    pub height: u32,
    /// «RGBA 8», «Gray 16», «RGB 32F».
    pub format: String,
    pub image: DynamicImage,
    pub preview: DynamicImage,
    pub thumb: ColorImage,
}

impl Source {
    pub fn size(&self) -> (u32, u32) {
        (self.width, self.height)
    }

    /// Имя файла без расширения.
    pub fn stem(&self) -> String {
        self.path.file_stem().map(|s| s.to_string_lossy().into_owned()).unwrap_or_default()
    }
}

pub fn is_image(path: &Path) -> bool {
    path.extension().and_then(|e| e.to_str()).is_some_and(|e| EXTENSIONS.iter().any(|x| x.eq_ignore_ascii_case(e)))
}

pub fn load(path: &Path) -> Result<Source, String> {
    let image = decode(path)?;
    let (width, height) = image.dimensions();
    let preview = shrink(&image, PREVIEW_MAX);
    let thumb = to_color_image(&shrink(&preview, THUMB_MAX));
    static NEXT_ID: AtomicU64 = AtomicU64::new(1);
    Ok(Source {
        id: NEXT_ID.fetch_add(1, Ordering::Relaxed),
        path: path.to_path_buf(),
        name: display_name(path),
        width,
        height,
        format: describe(&image),
        image,
        preview,
        thumb,
    })
}

/// Прочитать картинку целиком, не больше `MAX_SIZE` по стороне.
pub fn decode(path: &Path) -> Result<DynamicImage, String> {
    let fail = |e: &dyn std::fmt::Display| format!("{}: {e}", display_name(path));
    let (width, height) = ImageReader::open(path)
        .map_err(|e| fail(&e))?
        .with_guessed_format()
        .map_err(|e| fail(&e))?
        .into_dimensions()
        .map_err(|e| fail(&e))?;
    if width > MAX_SIZE || height > MAX_SIZE {
        let limit = format!("{MAX_SIZE}×{MAX_SIZE}");
        return Err(fail(&crate::lang::fill(
            crate::lang::t("{}×{} is larger than the {} limit"),
            &[&width, &height, &limit],
        )));
    }
    let mut reader = ImageReader::open(path).map_err(|e| fail(&e))?.with_guessed_format().map_err(|e| fail(&e))?;
    // Размер уже проверен, а стандартный предел памяти не пускает 8K в 16 бит.
    reader.no_limits();
    reader.decode().map_err(|e| fail(&e))
}

pub fn display_name(path: &Path) -> String {
    path.file_name().map(|s| s.to_string_lossy().into_owned()).unwrap_or_else(|| path.display().to_string())
}

/// Уменьшить, чтобы большая сторона была не больше `max`. Меньшее не трогаем.
fn shrink(image: &DynamicImage, max: u32) -> DynamicImage {
    let (w, h) = image.dimensions();
    if w <= max && h <= max {
        return image.clone();
    }
    image.resize(max, max, FilterType::Triangle)
}

fn to_color_image(image: &DynamicImage) -> ColorImage {
    let rgba = image.to_rgba8();
    ColorImage::from_rgba_unmultiplied([rgba.width() as usize, rgba.height() as usize], rgba.as_raw())
}

fn describe(image: &DynamicImage) -> String {
    use image::ColorType::*;
    match image.color() {
        L8 => "Gray 8",
        La8 => "Gray+A 8",
        Rgb8 => "RGB 8",
        Rgba8 => "RGBA 8",
        L16 => "Gray 16",
        La16 => "Gray+A 16",
        Rgb16 => "RGB 16",
        Rgba16 => "RGBA 16",
        Rgb32F => "RGB 32F",
        Rgba32F => "RGBA 32F",
        _ => "Other",
    }
    .to_owned()
}
