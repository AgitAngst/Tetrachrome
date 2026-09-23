//! Шапка: знак, меню, переключатель режима, отмена и масштаб.

use anvil_ui::chrome;
use anvil_ui::{Icon, Palette};
use eframe::egui::{self, Rect, Ui, Vec2};

use crate::app::{App, Dialog, LoadTarget, Mode};
use crate::lang::t;
use crate::source;
use crate::theme;

pub fn show(app: &mut App, ui: &mut Ui) {
    chrome::top_bar(ui, |ui| {
        let p = Palette::of(ui);
        let info = super::info();
        chrome::brand(ui, info.icon, info.name);
        ui.add_space(14.0);
        menus(app, ui);

        // Режим — посередине окна.
        let full = ui.max_rect();
        let mut mode = app.mode;
        let seg_rect = Rect::from_center_size(full.center(), egui::vec2(200.0, 30.0));
        let mut seg_ui = ui.new_child(
            egui::UiBuilder::new().max_rect(seg_rect).layout(egui::Layout::left_to_right(egui::Align::Center)),
        );
        let r =
            theme::segmented(&mut seg_ui, &mut mode, &[(Mode::Pack, t("Pack")), (Mode::Batch, t("Batch"))], None, 98.0);
        if r.changed() {
            app.mode = mode;
        }

        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
            if app.mode == Mode::Pack {
                zoom_controls(app, ui);
                ui.add_space(10.0);
                let (sep, _) = ui.allocate_exact_size(egui::vec2(1.0, 20.0), egui::Sense::hover());
                ui.painter().rect_filled(sep, 0.0, p.border);
                ui.add_space(6.0);
            }
            let redo = ui
                .add_enabled_ui(app.can_redo(), |ui| theme::icon_button(ui, Icon::Redo, 30.0, t("Redo  (Ctrl+Y)")))
                .inner;
            if redo.clicked() {
                app.redo();
            }
            let undo = ui
                .add_enabled_ui(app.can_undo(), |ui| theme::icon_button(ui, Icon::Undo, 30.0, t("Undo  (Ctrl+Z)")))
                .inner;
            if undo.clicked() {
                app.undo();
            }
        });
    });
}

fn menus(app: &mut App, ui: &mut Ui) {
    let item = |text: &str, shortcut: &str| egui::Button::new(text.to_owned()).shortcut_text(shortcut.to_owned());
    egui::MenuBar::new().ui(ui, |ui| {
        ui.menu_button(t("File"), |ui| {
            ui.set_min_width(240.0);
            if ui.add(item(t("Open images…"), "Ctrl+O")).clicked() {
                ui.close();
                match app.mode {
                    Mode::Pack => app.pick_images(LoadTarget::Auto { single: false }),
                    Mode::Batch => {
                        if let Some(files) =
                            rfd::FileDialog::new().add_filter(t("Images"), source::EXTENSIONS).pick_files()
                        {
                            app.batch_add(files);
                        }
                    }
                }
            }
            if ui.add_enabled(app.slots.iter().any(Option::is_some), item(t("Reload sources"), "F5")).clicked() {
                ui.close();
                app.reload_sources();
            }
            if ui.add_enabled(app.slots.iter().any(Option::is_some), item(t("Clear all channels"), "")).clicked() {
                ui.close();
                app.clear_all();
            }
            ui.separator();
            let can = app.can_export().is_ok();
            if ui.add_enabled(can, item(t("Export"), "Ctrl+E")).clicked() {
                ui.close();
                let ctx = ui.ctx().clone();
                app.export(&ctx, false);
            }
            if ui.add_enabled(app.last_export.is_some(), item(t("Show last export"), "")).clicked() {
                ui.close();
                if let Some(path) = &app.last_export {
                    crate::platform::reveal(path);
                }
            }
            ui.separator();
            if ui.add(item(t("Settings…"), "Ctrl+,")).clicked() {
                ui.close();
                app.settings_open = true;
            }
            ui.separator();
            if ui.add(item(t("Quit"), "Alt+F4")).clicked() {
                ui.ctx().send_viewport_cmd(egui::ViewportCommand::Close);
            }
        });
        ui.menu_button(t("Edit"), |ui| {
            ui.set_min_width(200.0);
            if ui.add_enabled(app.can_undo(), item(t("Undo"), "Ctrl+Z")).clicked() {
                ui.close();
                app.undo();
            }
            if ui.add_enabled(app.can_redo(), item(t("Redo"), "Ctrl+Y")).clicked() {
                ui.close();
                app.redo();
            }
        });
        ui.menu_button(t("Presets"), |ui| {
            ui.set_min_width(240.0);
            if ui.add(item(t("New preset"), "")).clicked() {
                ui.close();
                app.new_preset();
            }
            if ui.add(item(t("Duplicate"), "")).clicked() {
                ui.close();
                app.duplicate_preset();
            }
            let modified = app.modified();
            let builtin = app.current().builtin;
            let save_label = if builtin { t("Save as new preset") } else { t("Save changes") };
            if ui.add_enabled(modified, item(save_label, "Ctrl+S")).clicked() {
                ui.close();
                app.save_preset();
            }
            if ui.add_enabled(modified, item(t("Revert changes"), "")).clicked() {
                ui.close();
                app.revert_preset();
            }
            ui.separator();
            if ui.add_enabled(!builtin, item(t("Delete…"), "")).clicked() {
                ui.close();
                app.dialog = Some(Dialog::DeletePreset(app.selected));
            }
            ui.separator();
            if ui.add(item(t("Show presets file"), "")).clicked() {
                ui.close();
                super::reveal_store(app);
            }
        });
        ui.menu_button(t("Help"), |ui| {
            ui.set_min_width(220.0);
            if ui.add(item(t("Keyboard shortcuts"), "")).clicked() {
                ui.close();
                app.shortcuts_open = true;
            }
            if ui.add(item(t("Check for updates"), "")).clicked() {
                ui.close();
                app.updater.check(app.settings.common.prerelease, None, true);
                app.about_open = true;
            }
            if ui.add(item(t("About Tetrachrome"), "")).clicked() {
                ui.close();
                app.about_open = true;
            }
        });
    });
}

fn zoom_controls(app: &mut App, ui: &mut Ui) {
    let p = Palette::of(ui);
    if theme::icon_button(ui, Icon::Expand, 30.0, t("Fit to panel  (Ctrl+0)")).clicked() {
        app.view.zoom = None;
        app.view.pan = Vec2::ZERO;
    }
    if theme::icon_button(ui, Icon::Plus, 30.0, t("Zoom in")).clicked() {
        zoom_step(app, true);
    }
    let current = app.view.zoom.unwrap_or(app.view.fit_zoom);
    let label = if app.view.zoom.is_none() {
        format!("{} · {:.0}%", t("Fit"), current * 100.0)
    } else {
        format!("{:.0}%", current * 100.0)
    };
    let r = ui.add_sized(
        [84.0, 24.0],
        egui::Label::new(egui::RichText::new(label).size(12.5).color(p.weak)).sense(egui::Sense::click()),
    );
    if r.on_hover_text(t("Click for 100%")).clicked() {
        app.view.zoom = Some(1.0);
        app.view.pan = Vec2::ZERO;
    }
    if theme::icon_button(ui, Icon::Minus, 30.0, t("Zoom out")).clicked() {
        zoom_step(app, false);
    }
}

/// Ступени масштаба: удобные числа, а не «×1.1» до бесконечности.
const STEPS: &[f32] = &[0.0625, 0.125, 0.25, 0.33, 0.5, 0.67, 1.0, 1.5, 2.0, 3.0, 4.0, 6.0, 8.0, 12.0, 16.0, 32.0];

pub fn zoom_step(app: &mut App, up: bool) {
    let current = app.view.zoom.unwrap_or(app.view.fit_zoom);
    let next = if up {
        STEPS.iter().copied().find(|s| *s > current * 1.01).unwrap_or(32.0)
    } else {
        STEPS.iter().rev().copied().find(|s| *s < current * 0.99).unwrap_or(STEPS[0])
    };
    // Масштаб меняется вокруг центра: сдвиг растёт вместе с картинкой.
    app.view.pan *= next / current;
    app.view.zoom = Some(next);
}
