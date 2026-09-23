//! Tetrachrome — упаковщик каналов текстур для игровых художников.

// В релизе не поднимаем консольное окно.
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod app;
mod batch;
mod lang;
mod model;
mod naming;
mod pack;
mod platform;
mod presets;
mod settings;
mod source;
mod theme;
mod ui;

fn main() -> eframe::Result<()> {
    // Следы прошлого обновления (*.old-…, папка загрузки) — прочь.
    anvil_update::cleanup();
    let options = eframe::NativeOptions {
        viewport: eframe::egui::ViewportBuilder::default()
            .with_title("Tetrachrome")
            .with_inner_size([1480.0, 920.0])
            .with_min_inner_size([1060.0, 640.0])
            .with_icon(std::sync::Arc::new(anvil_ui::appicon::icon_data(theme::ACCENT, anvil_ui::Icon::Tiles)))
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
