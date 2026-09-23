//! Tetrachrome — упаковщик каналов текстур для игровых художников.

// В релизе не поднимаем консольное окно.
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod app;
mod appicon;
mod batch;
mod icons;
mod model;
mod naming;
mod pack;
mod platform;
mod presets;
mod source;
mod theme;
mod ui;

fn main() -> eframe::Result<()> {
    let icon = eframe::egui::IconData {
        rgba: appicon::render_icon(64),
        width: 64,
        height: 64,
    };
    let options = eframe::NativeOptions {
        viewport: eframe::egui::ViewportBuilder::default()
            .with_title("Tetrachrome")
            .with_inner_size([1480.0, 920.0])
            .with_min_inner_size([1060.0, 640.0])
            .with_icon(std::sync::Arc::new(icon))
            .with_drag_and_drop(true),
        centered: true,
        ..Default::default()
    };
    // Файлы и папки из командной строки — как если бы их бросили в окно.
    // Так работает и «Открыть с помощью», и перетаскивание на значок exe.
    let files: Vec<std::path::PathBuf> = std::env::args_os().skip(1).map(Into::into).collect();
    eframe::run_native(
        "Tetrachrome",
        options,
        Box::new(move |cc| {
            let mut app = app::App::new(cc);
            if !files.is_empty() {
                app.open_paths(files);
            }
            Ok(Box::new(app))
        }),
    )
}
