//! Модальные окна: подтверждения, «О программе», сочетания клавиш.

use eframe::egui::{self, Color32, Key, Margin, Rect, Stroke, Ui, Vec2};

use crate::app::{App, Dialog};
use crate::appicon;
use crate::theme::{self, Palette};

enum Choice {
    None,
    Close,
    Primary,
    Secondary,
}

pub fn show(app: &mut App, ctx: &egui::Context) {
    let Some(dialog) = &app.dialog else { return };
    let p = Palette::DARK;
    let mut choice = Choice::None;
    let modal = egui::Modal::new(egui::Id::new("dialog"))
        .backdrop_color(Color32::from_black_alpha(150))
        .frame(
            egui::Frame::new()
                .fill(p.panel)
                .stroke(Stroke::new(1.0, p.border))
                .corner_radius(14)
                .inner_margin(Margin::same(22))
                .shadow(egui::Shadow {
                    offset: [0, 16],
                    blur: 40,
                    spread: 0,
                    color: Color32::from_black_alpha(160),
                }),
        );
    let response = modal.show(ctx, |ui| {
        ui.set_width(400.0);
        match dialog {
            Dialog::DeletePreset(i) => {
                let name = app.presets.get(*i).map(|p| p.name.clone()).unwrap_or_default();
                title(ui, &format!("Delete “{name}”?"));
                body(ui, "The preset is removed from the presets file. This can't be undone.");
                choice = buttons(ui, "Delete", Some("Cancel"), None, true);
            }
            Dialog::SwitchPreset(_) => {
                let name = app.work.name.clone();
                let builtin = app.current().builtin;
                title(ui, &format!("Save changes to “{name}”?"));
                body(
                    ui,
                    if builtin {
                        "This built-in preset can't be overwritten — the changes will be saved as a new preset."
                    } else {
                        "Your changes will be lost if you switch without saving."
                    },
                );
                choice = buttons(ui, if builtin { "Save as new" } else { "Save" }, Some("Cancel"), Some("Discard"), false);
            }
            Dialog::Overwrite(path) => {
                title(ui, "Replace existing file?");
                body(ui, &format!("{} already exists in\n{}", crate::source::display_name(path), path.parent().map(|d| d.display().to_string()).unwrap_or_default()));
                choice = buttons(ui, "Replace", Some("Cancel"), None, true);
            }
            Dialog::About => {
                about(ui, app);
                choice = buttons(ui, "Close", None, None, false);
            }
            Dialog::Shortcuts => {
                shortcuts(ui);
                choice = buttons(ui, "Close", None, None, false);
            }
        }
    });
    if response.should_close() && matches!(choice, Choice::None) {
        choice = Choice::Close;
    }
    let dialog = match choice {
        Choice::None => return,
        _ => app.dialog.take().expect("dialog is open"),
    };
    match (dialog, choice) {
        (Dialog::DeletePreset(i), Choice::Primary) => app.delete_preset(i),
        (Dialog::SwitchPreset(i), Choice::Primary) => {
            // «Сохранить как новый» добавляет пресет в конец — индекс цели не
            // сдвигается. Не сохранилось (имя занято) — остаёмся, правки целы.
            app.save_preset();
            if !app.modified() {
                app.force_select(i);
            }
        }
        (Dialog::SwitchPreset(i), Choice::Secondary) => app.force_select(i),
        (Dialog::Overwrite(_), Choice::Primary) => app.export(ctx, true),
        _ => {}
    }
}

fn title(ui: &mut Ui, text: &str) {
    ui.label(egui::RichText::new(text).font(theme::semibold(17.0)).color(Palette::DARK.text));
    ui.add_space(6.0);
}

fn body(ui: &mut Ui, text: &str) {
    ui.label(egui::RichText::new(text).color(Palette::DARK.weak));
    ui.add_space(18.0);
}

/// Кнопки справа внизу. Enter — главная, Esc — закрыть.
fn buttons(ui: &mut Ui, primary: &str, cancel: Option<&str>, secondary: Option<&str>, danger: bool) -> Choice {
    let p = Palette::DARK;
    let mut choice = Choice::None;
    ui.horizontal(|ui| {
        if let Some(text) = secondary
            && theme::soft_button_colored(ui, None, text, p.danger).clicked()
        {
            choice = Choice::Secondary;
        }
        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
            let galley_w = ui.painter().layout_no_wrap(primary.to_owned(), theme::semibold(14.0), p.text).size().x;
            let r = if danger {
                danger_button(ui, primary, egui::vec2(galley_w + 36.0, 34.0))
            } else {
                theme::primary_button(ui, None, primary, egui::vec2(galley_w + 36.0, 34.0), true)
            };
            if r.clicked() {
                choice = Choice::Primary;
            }
            if let Some(text) = cancel
                && theme::soft_button(ui, None, text).clicked()
            {
                choice = Choice::Close;
            }
        });
    });
    if matches!(choice, Choice::None) && ui.input_mut(|i| i.consume_key(egui::Modifiers::NONE, Key::Enter)) {
        choice = Choice::Primary;
    }
    choice
}

fn danger_button(ui: &mut Ui, text: &str, size: Vec2) -> egui::Response {
    let p = Palette::DARK;
    let (rect, response) = ui.allocate_exact_size(size, egui::Sense::click());
    let fill = if response.hovered() { p.danger } else { p.danger.gamma_multiply(0.85) };
    ui.painter().rect_filled(rect, 9, fill);
    ui.painter().text(rect.center(), egui::Align2::CENTER_CENTER, text, theme::semibold(14.0), Color32::WHITE);
    theme::focus_ring(ui, rect, &response, 9);
    response.on_hover_cursor(egui::CursorIcon::PointingHand)
}

fn about(ui: &mut Ui, app: &App) {
    let p = Palette::DARK;
    ui.horizontal(|ui| {
        let id = egui::Id::new("about-icon");
        let texture = ui.data(|d| d.get_temp::<egui::TextureHandle>(id)).unwrap_or_else(|| {
            let size = 112usize;
            let texture = ui.ctx().load_texture(
                "about-icon",
                egui::ColorImage::from_rgba_unmultiplied([size, size], &appicon::render_icon(size as u32)),
                egui::TextureOptions::LINEAR,
            );
            ui.data_mut(|d| d.insert_temp(id, texture.clone()));
            texture
        });
        let (rect, _) = ui.allocate_exact_size(Vec2::splat(56.0), egui::Sense::hover());
        ui.painter().image(texture.id(), rect, Rect::from_min_max(egui::pos2(0.0, 0.0), egui::pos2(1.0, 1.0)), Color32::WHITE);
        ui.add_space(8.0);
        ui.vertical(|ui| {
            ui.label(egui::RichText::new("Tetrachrome").font(theme::semibold(22.0)).color(p.text));
            ui.label(egui::RichText::new(format!("Version {}", env!("CARGO_PKG_VERSION"))).color(p.weak));
        });
    });
    ui.add_space(12.0);
    ui.label(
        egui::RichText::new(
            "Packs grayscale texture maps into the RGBA channels of one image, following the conventions of Unity, Unreal and Godot. All math runs in linear space on the CPU.",
        )
        .color(p.weak),
    );
    ui.add_space(10.0);
    ui.label(egui::RichText::new("Presets file").size(12.0).color(p.faint));
    ui.label(egui::RichText::new(app.store.display().to_string()).monospace().size(12.0).color(p.weak));
    ui.add_space(18.0);
}

fn shortcuts(ui: &mut Ui) {
    let p = Palette::DARK;
    title(ui, "Keyboard shortcuts");
    let rows = [
        ("Ctrl+O", "Open images"),
        ("Ctrl+E", "Export (batch: process all)"),
        ("Ctrl+Z", "Undo"),
        ("Ctrl+Y / Ctrl+Shift+Z", "Redo"),
        ("Ctrl+S", "Save preset changes"),
        ("F5", "Reload sources from disk"),
        ("Ctrl+0", "Fit preview"),
        ("Wheel / drag", "Zoom / pan the preview"),
        ("Double-click", "Fit preview"),
    ];
    egui::Grid::new("shortcuts").num_columns(2).spacing(egui::vec2(24.0, 8.0)).show(ui, |ui| {
        for (keys, what) in rows {
            ui.label(egui::RichText::new(keys).monospace().color(p.accent_text));
            ui.label(egui::RichText::new(what).color(p.text));
            ui.end_row();
        }
    });
    ui.add_space(18.0);
}
