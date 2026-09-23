//! То, чего нет в egui: курсор во время перетаскивания файлов и Проводник.

use std::path::Path;

use eframe::egui::{self, Pos2};

/// Где курсор, в точках окна.
///
/// Пока над окном несут файлы из Проводника, winit не присылает движения
/// мыши, и egui не знает, над какой карточкой курсор. Спрашиваем систему
/// напрямую.
pub fn cursor_pos(ctx: &egui::Context) -> Option<Pos2> {
    os_cursor(ctx).or_else(|| ctx.input(|i| i.pointer.latest_pos()))
}

#[cfg(windows)]
fn os_cursor(ctx: &egui::Context) -> Option<Pos2> {
    use windows_sys::Win32::Foundation::POINT;
    use windows_sys::Win32::UI::WindowsAndMessaging::GetCursorPos;

    let mut point = POINT { x: 0, y: 0 };
    // SAFETY: GetCursorPos пишет только в переданную структуру.
    if unsafe { GetCursorPos(&mut point) } == 0 {
        return None;
    }
    let (inner, native_ppp) = ctx.input(|i| (i.viewport().inner_rect, i.viewport().native_pixels_per_point));
    let inner = inner?;
    let native_ppp = native_ppp?;
    // inner_rect — в точках egui, но от начала экрана; переводим курсор туда же.
    let ppp = native_ppp * ctx.zoom_factor();
    let screen = Pos2::new(point.x as f32 / ppp, point.y as f32 / ppp);
    Some((screen - inner.min).to_pos2())
}

#[cfg(not(windows))]
fn os_cursor(_ctx: &egui::Context) -> Option<Pos2> {
    None
}

/// Показать файл в Проводнике, выделив его. Для папки — просто открыть.
pub fn reveal(path: &Path) {
    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;
        let mut cmd = std::process::Command::new("explorer");
        if path.is_dir() {
            cmd.arg(path);
        } else {
            // Проводник понимает только `/select,"путь"` одной строкой, а
            // обычная передача аргументов взяла бы в кавычки всё целиком.
            cmd.raw_arg(format!("/select,\"{}\"", path.display()));
        }
        let _ = cmd.spawn();
    }
    #[cfg(not(windows))]
    {
        let dir = if path.is_dir() { path } else { path.parent().unwrap_or(path) };
        let _ = std::process::Command::new("xdg-open").arg(dir).spawn();
    }
}
