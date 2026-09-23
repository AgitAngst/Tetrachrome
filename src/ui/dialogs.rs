//! Модальные окна: подтверждения и сочетания клавиш. «О программе» и «Настройки» — из набора.

use anvil_ui::chrome;
use anvil_ui::widgets as w;
use anvil_ui::{Kind, Palette};
use eframe::egui::{self, Key, Ui};

use crate::app::{App, Dialog};
use crate::lang::{fill, t};

enum Choice {
    None,
    Close,
    Primary,
    Secondary,
}

pub fn show(app: &mut App, ctx: &egui::Context) {
    shortcuts(app, ctx);
    let Some(dialog) = &app.dialog else { return };
    let mut choice = Choice::None;
    match dialog {
        Dialog::DeletePreset(i) => {
            let name = app.presets.get(*i).map(|p| p.name.clone()).unwrap_or_default();
            let body = |ui: &mut Ui| {
                w::note(ui, t("The preset is removed from the presets file. This can't be undone."));
            };
            match w::confirm(ctx, "delete-preset", &fill(t("Delete “{}”?"), &[&name]), body, t("Delete"), true) {
                Some(true) => choice = Choice::Primary,
                Some(false) => choice = Choice::Close,
                None => {}
            }
        }
        Dialog::Overwrite(path) => {
            let name = crate::source::display_name(path);
            let dir = path.parent().map(|d| d.display().to_string()).unwrap_or_default();
            let body = |ui: &mut Ui| {
                w::note(ui, fill(t("{} already exists in {}"), &[&name, &dir]));
            };
            match w::confirm(ctx, "overwrite", t("Replace existing file?"), body, t("Replace"), true) {
                Some(true) => choice = Choice::Primary,
                Some(false) => choice = Choice::Close,
                None => {}
            }
        }
        Dialog::SwitchPreset(_) => {
            let name = app.work.name.clone();
            let builtin = app.current().builtin;
            let mut open = true;
            chrome::dialog(
                ctx,
                "switch-preset",
                &fill(t("Save changes to “{}”?"), &[&name]),
                440.0,
                &mut open,
                |ui| {
                    w::note(
                        ui,
                        if builtin {
                            t("This built-in preset can't be overwritten — the changes will be saved as a new preset.")
                        } else {
                            t("Your changes will be lost if you switch without saving.")
                        },
                    );
                    ui.add_space(16.0);
                    ui.horizontal(|ui| {
                        if w::button(ui, Kind::Danger, None, t("Discard")).clicked() {
                            choice = Choice::Secondary;
                        }
                        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                            let save = if builtin { t("Save as new") } else { t("Save") };
                            if w::button(ui, Kind::Primary, None, save).clicked() {
                                choice = Choice::Primary;
                            }
                            if w::button(ui, Kind::Secondary, None, t("Cancel")).clicked() {
                                choice = Choice::Close;
                            }
                        });
                    });
                    if matches!(choice, Choice::None)
                        && ui.input_mut(|i| i.consume_key(egui::Modifiers::NONE, Key::Enter))
                    {
                        choice = Choice::Primary;
                    }
                },
            );
            if !open && matches!(choice, Choice::None) {
                choice = Choice::Close;
            }
        }
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

fn shortcuts(app: &mut App, ctx: &egui::Context) {
    let rows = [
        ("Ctrl+O", t("Open images")),
        ("Ctrl+E", t("Export (batch: process all)")),
        ("Ctrl+Z", t("Undo")),
        ("Ctrl+Y / Ctrl+Shift+Z", t("Redo")),
        ("Ctrl+S", t("Save preset changes")),
        ("F5", t("Reload sources from disk")),
        ("Ctrl+0", t("Fit preview")),
        ("Ctrl+,", t("Settings")),
        (t("Wheel / drag"), t("Zoom / pan the preview")),
        (t("Double-click"), t("Fit preview")),
    ];
    chrome::dialog(ctx, "shortcuts", t("Keyboard shortcuts"), 460.0, &mut app.shortcuts_open, |ui| {
        let p = Palette::of(ui);
        egui::Grid::new("shortcuts").num_columns(2).spacing(egui::vec2(24.0, 10.0)).show(ui, |ui| {
            for (keys, what) in rows {
                ui.horizontal(|ui| {
                    ui.spacing_mut().item_spacing.x = 3.0;
                    for (i, combo) in keys.split(" / ").enumerate() {
                        if i > 0 {
                            ui.label(egui::RichText::new(" / ").color(p.faint));
                        }
                        for key in combo.split('+') {
                            w::kbd(ui, key);
                        }
                    }
                });
                ui.label(egui::RichText::new(what).color(p.text));
                ui.end_row();
            }
        });
        ui.add_space(8.0);
    });
}
