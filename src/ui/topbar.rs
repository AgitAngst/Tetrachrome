//! Верхняя полоса: логотип, меню, переключатель режима, отмена и масштаб.

use eframe::egui::{self, Margin, Rect, Stroke, Ui, Vec2};

use crate::app::{App, Dialog, LoadTarget, Mode};
use crate::icons::{self, Icon};
use crate::model::Channel;
use crate::source;
use crate::theme::{self, Palette};

pub fn show(app: &mut App, ui: &mut Ui) {
    let p = Palette::DARK;
    egui::Panel::top("topbar")
        .exact_size(48.0)
        .frame(
            egui::Frame::new()
                .fill(p.panel)
                .inner_margin(Margin::symmetric(14, 0))
                .stroke(Stroke::new(1.0, p.border)),
        )
        .show(ui, |ui| {
            ui.horizontal_centered(|ui| {
                logo(ui);
                ui.add_space(14.0);
                menus(app, ui);

                // Режим — посередине окна.
                let full = ui.max_rect();
                let mut mode = app.mode;
                let seg_width = 190.0;
                let seg_rect = Rect::from_center_size(full.center(), egui::vec2(seg_width, 28.0));
                let mut seg_ui = ui.new_child(egui::UiBuilder::new().max_rect(seg_rect).layout(egui::Layout::left_to_right(egui::Align::Center)));
                let r = theme::segmented(&mut seg_ui, &mut mode, &[(Mode::Pack, "Pack"), (Mode::Batch, "Batch")], None, 92.0);
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
                    let redo = ui.add_enabled_ui(app.can_redo(), |ui| icons::button(ui, Icon::Redo, 28.0, "Redo  (Ctrl+Y)")).inner;
                    if redo.clicked() {
                        app.redo();
                    }
                    let undo = ui.add_enabled_ui(app.can_undo(), |ui| icons::button(ui, Icon::Undo, 28.0, "Undo  (Ctrl+Z)")).inner;
                    if undo.clicked() {
                        app.undo();
                    }
                });
            });
        });
}

/// Знак: четыре плитки R, G, B, A и название.
fn logo(ui: &mut Ui) {
    let p = Palette::DARK;
    let (rect, _) = ui.allocate_exact_size(Vec2::splat(22.0), egui::Sense::hover());
    let tile = 9.5;
    let gap = 3.0;
    for (i, channel) in Channel::ALL.into_iter().enumerate() {
        let x = rect.min.x + (i % 2) as f32 * (tile + gap);
        let y = rect.min.y + (i / 2) as f32 * (tile + gap);
        let r = Rect::from_min_size(egui::pos2(x, y), Vec2::splat(tile));
        ui.painter().rect_filled(r, 3, theme::channel_color(channel));
    }
    ui.add_space(4.0);
    ui.label(egui::RichText::new("Tetrachrome").font(theme::semibold(15.5)).color(p.text));
}

fn menus(app: &mut App, ui: &mut Ui) {
    let item = |text: &str, shortcut: &str| egui::Button::new(text.to_owned()).shortcut_text(shortcut.to_owned());
    egui::MenuBar::new().ui(ui, |ui| {
        ui.menu_button("File", |ui| {
            ui.set_min_width(230.0);
            if ui.add(item("Open images…", "Ctrl+O")).clicked() {
                ui.close();
                match app.mode {
                    Mode::Pack => app.pick_images(LoadTarget::Auto { single: false }),
                    Mode::Batch => {
                        if let Some(files) = rfd::FileDialog::new().add_filter("Images", source::EXTENSIONS).pick_files() {
                            app.batch_add(files);
                        }
                    }
                }
            }
            if ui.add_enabled(app.slots.iter().any(Option::is_some), item("Reload sources", "F5")).clicked() {
                ui.close();
                app.reload_sources();
            }
            if ui.add_enabled(app.slots.iter().any(Option::is_some), item("Clear all channels", "")).clicked() {
                ui.close();
                app.clear_all();
            }
            ui.separator();
            let can = app.can_export().is_ok();
            if ui.add_enabled(can, item("Export", "Ctrl+E")).clicked() {
                ui.close();
                let ctx = ui.ctx().clone();
                app.export(&ctx, false);
            }
            if ui.add_enabled(app.last_export.is_some(), item("Show last export", "")).clicked() {
                ui.close();
                if let Some(path) = &app.last_export {
                    crate::platform::reveal(path);
                }
            }
            ui.separator();
            if ui.add(item("Quit", "Alt+F4")).clicked() {
                ui.ctx().send_viewport_cmd(egui::ViewportCommand::Close);
            }
        });
        ui.menu_button("Edit", |ui| {
            ui.set_min_width(200.0);
            if ui.add_enabled(app.can_undo(), item("Undo", "Ctrl+Z")).clicked() {
                ui.close();
                app.undo();
            }
            if ui.add_enabled(app.can_redo(), item("Redo", "Ctrl+Y")).clicked() {
                ui.close();
                app.redo();
            }
        });
        ui.menu_button("Presets", |ui| {
            ui.set_min_width(230.0);
            if ui.add(item("New preset", "")).clicked() {
                ui.close();
                app.new_preset();
            }
            if ui.add(item("Duplicate", "")).clicked() {
                ui.close();
                app.duplicate_preset();
            }
            let modified = app.modified();
            let builtin = app.current().builtin;
            let save_label = if builtin { "Save as new preset" } else { "Save changes" };
            if ui.add_enabled(modified, item(save_label, "Ctrl+S")).clicked() {
                ui.close();
                app.save_preset();
            }
            if ui.add_enabled(modified, item("Revert changes", "")).clicked() {
                ui.close();
                app.revert_preset();
            }
            ui.separator();
            if ui.add_enabled(!builtin, item("Delete…", "")).clicked() {
                ui.close();
                app.dialog = Some(Dialog::DeletePreset(app.selected));
            }
            ui.separator();
            if ui.add(item("Show presets file", "")).clicked() {
                ui.close();
                if app.store.exists() {
                    crate::platform::reveal(&app.store);
                } else if let Some(dir) = app.store.parent() {
                    let _ = std::fs::create_dir_all(dir);
                    crate::platform::reveal(dir);
                }
            }
        });
        ui.menu_button("Help", |ui| {
            ui.set_min_width(200.0);
            if ui.add(item("Keyboard shortcuts", "")).clicked() {
                ui.close();
                app.dialog = Some(Dialog::Shortcuts);
            }
            if ui.add(item("About Tetrachrome", "")).clicked() {
                ui.close();
                app.dialog = Some(Dialog::About);
            }
        });
    });
}

fn zoom_controls(app: &mut App, ui: &mut Ui) {
    let p = Palette::DARK;
    let fit = icons::button(ui, Icon::Fit, 28.0, "Fit to panel  (Ctrl+0)");
    if fit.clicked() {
        app.view.zoom = None;
        app.view.pan = Vec2::ZERO;
    }
    if icons::button(ui, Icon::Plus, 28.0, "Zoom in").clicked() {
        zoom_step(app, true);
    }
    let current = app.view.zoom.unwrap_or(app.view.fit_zoom);
    let label = if app.view.zoom.is_none() {
        format!("Fit · {:.0}%", current * 100.0)
    } else {
        format!("{:.0}%", current * 100.0)
    };
    let r = ui.add_sized(
        [78.0, 24.0],
        egui::Label::new(egui::RichText::new(label).size(12.5).color(p.weak)).sense(egui::Sense::click()),
    );
    if r.on_hover_text("Click for 100%").clicked() {
        app.view.zoom = Some(1.0);
        app.view.pan = Vec2::ZERO;
    }
    if icons::button(ui, Icon::Minus, 28.0, "Zoom out").clicked() {
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
