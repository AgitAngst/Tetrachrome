//! Значок Tetrachrome из файлов `assets/icon` (их собирает `scripts/render-icon.ps1` из SVG): значок окна
//! и панели задач. Значок exe — `build.rs`, знак в шапке и «О программе» — `ui/mark.rs`.

use eframe::egui;

/// Значок окна: система сама уменьшает до нужного размера.
const WINDOW: &[u8] = include_bytes!("../assets/icon/tetrachrome-256.png");

/// Значок для `ViewportBuilder::with_icon`.
pub fn window() -> egui::IconData {
    // PNG зашит в программу и проверен тестом ниже: битый файл — ошибка сборки, не работы.
    let image =
        image::load_from_memory_with_format(WINDOW, image::ImageFormat::Png).expect("icon PNG decodes").into_rgba8();
    egui::IconData { width: image.width(), height: image.height(), rgba: image.into_raw() }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn window_icon_is_a_256_square() {
        let icon = window();
        assert_eq!((icon.width, icon.height), (256, 256));
        assert_eq!(icon.rgba.len(), 256 * 256 * 4);
        // Плитка залита в центре, а углы скруглены и прозрачны.
        assert_eq!(icon.rgba[(128 * 256 + 128) * 4 + 3], 255);
        assert_eq!(icon.rgba[3], 0);
    }
}
