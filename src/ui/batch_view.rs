//! Пакетный режим: таблица материалов, настройки вывода, прогресс и журнал.

use std::sync::atomic::Ordering;

use eframe::egui::{self, Color32, Sense, Stroke, Ui, Vec2};

use crate::app::App;
use crate::batch::{self, LogLevel};
use crate::icons::{self, Icon};
use crate::model::{Channel, OutputFormat, SizePolicy};
use crate::source;
use crate::theme::{self, Palette};

pub fn show(app: &mut App, ui: &mut Ui) {
    let p = Palette::DARK;
    toolbar(app, ui);
    ui.add_space(12.0);

    let running = app.batch.run.is_some();
    // Низ окна: вывод, кнопка, журнал. Таблица занимает остальное.
    let bottom = 300.0f32.min(ui.available_height() * 0.5);
    let table_height = (ui.available_height() - bottom).max(160.0);
    egui::Frame::new()
        .fill(p.panel)
        .stroke(Stroke::new(1.0, p.border))
        .corner_radius(12)
        .inner_margin(egui::Margin::same(12))
        .show(ui, |ui| {
            ui.set_width(ui.available_width());
            ui.set_height(table_height - 26.0);
            ui.add_enabled_ui(!running, |ui| table(app, ui));
        });
    ui.add_space(12.0);

    ui.columns(2, |cols| {
        settings(app, &mut cols[0]);
        run_panel(app, &mut cols[1]);
    });
}

fn toolbar(app: &mut App, ui: &mut Ui) {
    let p = Palette::DARK;
    ui.horizontal(|ui| {
        ui.label(egui::RichText::new("Batch").font(theme::semibold(20.0)).color(p.text));
        ui.add_space(4.0);
        theme::chip(ui, &app.work.name, theme::engine_color(app.work.engine));
        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
            let running = app.batch.run.is_some();
            ui.add_enabled_ui(!running && !app.batch.files.is_empty(), |ui| {
                if theme::soft_button(ui, Some(Icon::Trash), "Clear").clicked() {
                    app.batch.files.clear();
                    app.batch.disabled.clear();
                    app.batch.status.clear();
                    app.refresh_plan();
                }
            });
            ui.add_enabled_ui(!running, |ui| {
                if theme::soft_button(ui, Some(Icon::Folder), "Add folder…").clicked()
                    && let Some(dir) = rfd::FileDialog::new().pick_folder()
                {
                    app.batch_add(vec![dir]);
                }
                if theme::soft_button(ui, Some(Icon::Plus), "Add files…").clicked()
                    && let Some(files) = rfd::FileDialog::new().add_filter("Images", source::EXTENSIONS).pick_files()
                {
                    app.batch_add(files);
                }
            });
        });
    });
    let plan = &app.batch.plan;
    ui.label(theme::weak(format!(
        "{} files · {} materials · {} unmatched — maps are grouped by name and sorted into channels by the preset's suffixes",
        app.batch.files.len(),
        plan.groups.len(),
        plan.unmatched.len()
    )));
}

fn table(app: &mut App, ui: &mut Ui) {
    let p = Palette::DARK;
    if app.batch.files.is_empty() {
        ui.centered_and_justified(|ui| {
            ui.label(
                egui::RichText::new("Drop a folder with maps here, or use “Add folder…”.\nRock_Metallic.png, Rock_AO.png, Rock_Smoothness.png → Rock_MaskMap.png")
                    .color(p.faint),
            );
        });
        return;
    }
    let channels = app.work.channels();
    egui::ScrollArea::vertical().auto_shrink([false, false]).show(ui, |ui| {
        egui::Grid::new("batch-table")
            .num_columns(3 + channels.len())
            .spacing(egui::vec2(14.0, 8.0))
            .striped(true)
            .min_col_width(20.0)
            .show(ui, |ui| {
                ui.label("");
                ui.label(egui::RichText::new("Material").font(theme::semibold(12.0)).color(p.faint));
                for &c in channels {
                    ui.horizontal(|ui| {
                        ui.label(egui::RichText::new(c.letter()).font(theme::semibold(12.0)).color(theme::channel_color(c)));
                        let role = app.work.slots[c.index()].role.trim();
                        ui.label(
                            egui::RichText::new(if role.is_empty() { "constant" } else { role })
                                .size(12.0)
                                .color(p.faint),
                        );
                    });
                }
                ui.label(egui::RichText::new("Status").font(theme::semibold(12.0)).color(p.faint));
                ui.end_row();

                let groups = app.batch.plan.groups.clone();
                for group in &groups {
                    let key = group.base.to_lowercase();
                    let mut on = !app.batch.disabled.contains(&key);
                    let filled = batch::filled(group, &app.work);
                    if ui.checkbox(&mut on, "").changed() {
                        if on {
                            app.batch.disabled.remove(&key);
                        } else {
                            app.batch.disabled.insert(key.clone());
                        }
                    }
                    let name_color = if on { p.text } else { p.faint };
                    ui.label(egui::RichText::new(&group.base).font(theme::semibold(13.0)).color(name_color));
                    for &c in channels {
                        cell(app, ui, group.files[c.index()].as_deref(), c);
                    }
                    let status = app.batch.status.get(&key).copied();
                    match (status, filled) {
                        (Some(true), _) => status_label(ui, Icon::Check, "done", p.good),
                        (Some(false), _) => status_label(ui, Icon::Warning, "failed", p.danger),
                        (None, 0) => status_label(ui, Icon::Warning, "no maps", p.warn),
                        (None, n) if n < channels.iter().filter(|c| !app.work.slots[c.index()].role.is_empty()).count() => {
                            status_label(ui, Icon::Info, "partial", p.warn)
                        }
                        _ => status_label(ui, Icon::Check, "ready", p.weak),
                    }
                    ui.end_row();
                }
            });

        let plan = &app.batch.plan;
        if !plan.unmatched.is_empty() {
            ui.add_space(10.0);
            egui::CollapsingHeader::new(
                egui::RichText::new(format!("{} files matched no channel", plan.unmatched.len())).color(p.warn),
            )
            .id_salt("unmatched")
            .show(ui, |ui| {
                ui.label(
                    egui::RichText::new("Add their suffixes to the preset's channels in the left panel.")
                        .size(12.0)
                        .color(p.faint),
                );
                for path in &plan.unmatched {
                    ui.label(egui::RichText::new(source::display_name(path)).monospace().size(12.0).color(p.weak));
                }
            });
        }
        if !plan.conflicts.is_empty() {
            ui.add_space(6.0);
            egui::CollapsingHeader::new(
                egui::RichText::new(format!("{} duplicate maps ignored", plan.conflicts.len())).color(p.warn),
            )
            .id_salt("conflicts")
            .show(ui, |ui| {
                for (path, what) in &plan.conflicts {
                    ui.label(
                        egui::RichText::new(format!("{} — {what} already taken", source::display_name(path)))
                            .monospace()
                            .size(12.0)
                            .color(p.weak),
                    );
                }
            });
        }
    });
}

fn cell(app: &App, ui: &mut Ui, file: Option<&std::path::Path>, channel: Channel) {
    let p = Palette::DARK;
    match file {
        Some(path) => {
            ui.horizontal(|ui| {
                ui.spacing_mut().item_spacing.x = 5.0;
                let (r, _) = ui.allocate_exact_size(Vec2::splat(8.0), Sense::hover());
                ui.painter().circle_filled(r.center(), 3.5, theme::channel_color(channel));
                ui.add(egui::Label::new(egui::RichText::new(source::display_name(path)).size(12.5).color(p.weak)).truncate())
                    .on_hover_text(path.display().to_string());
            });
        }
        None => {
            let config = &app.work.slots[channel.index()];
            let text = if config.role.is_empty() { format!("= {}", config.fill) } else { format!("— = {}", config.fill) };
            ui.label(egui::RichText::new(text).size(12.0).color(p.faint));
        }
    }
}

fn status_label(ui: &mut Ui, icon: Icon, text: &str, color: Color32) {
    ui.horizontal(|ui| {
        ui.spacing_mut().item_spacing.x = 4.0;
        let (r, _) = ui.allocate_exact_size(Vec2::splat(14.0), Sense::hover());
        icons::paint(ui.painter(), r, icon, color);
        ui.label(egui::RichText::new(text).size(12.0).color(color));
    });
}

fn settings(app: &mut App, ui: &mut Ui) {
    let p = Palette::DARK;
    theme::card_frame().show(ui, |ui| {
        ui.set_width(ui.available_width());
        theme::section_label(ui, "Output");
        ui.add_space(6.0);
        label(ui, "Folder");
        ui.horizontal(|ui| {
            let dir = app.batch_out_dir();
            let text = match (&dir, app.output.folder.is_some()) {
                (Some(d), true) => d.display().to_string(),
                (Some(d), false) => format!("{}  (default)", d.display()),
                (None, _) => "“packed” next to the sources".to_owned(),
            };
            ui.add_sized(
                [ui.available_width() - 34.0, 26.0],
                egui::Label::new(egui::RichText::new(text).size(12.5).color(p.weak)).truncate(),
            );
            if icons::button(ui, Icon::Folder, 26.0, "Choose output folder").clicked() {
                app.pick_output_folder();
            }
        });
        ui.add_space(4.0);
        label(ui, "File name");
        ui.add(
            egui::TextEdit::singleline(&mut app.output.template)
                .desired_width(ui.available_width())
                .font(egui::TextStyle::Monospace)
                .margin(egui::vec2(8.0, 5.0)),
        )
        .on_hover_text("{basename} {label} {preset} {date}");
        ui.add_space(6.0);
        ui.horizontal(|ui| {
            let options: Vec<(OutputFormat, &str)> = OutputFormat::ALL.iter().map(|f| (*f, f.label())).collect();
            theme::segmented(ui, &mut app.output.format, &options, None, 46.0);
            let supports = app.output.format.supports_16bit();
            ui.add_enabled_ui(supports, |ui| {
                let mut sixteen = app.output.sixteen && supports;
                if theme::segmented(ui, &mut sixteen, &[(false, "8-bit"), (true, "16-bit")], None, 50.0).changed() {
                    app.output.sixteen = sixteen;
                }
            });
        });
        ui.add_space(6.0);
        label(ui, "When sizes differ");
        theme::segmented(
            ui,
            &mut app.output.policy,
            &[(SizePolicy::Largest, "Largest"), (SizePolicy::Smallest, "Smallest"), (SizePolicy::Custom, "Custom")],
            None,
            64.0,
        );
        if app.output.policy == SizePolicy::Custom {
            ui.horizontal(|ui| {
                ui.add(egui::DragValue::new(&mut app.output.custom.0).range(1..=crate::model::MAX_SIZE).suffix(" px"));
                ui.label("×");
                ui.add(egui::DragValue::new(&mut app.output.custom.1).range(1..=crate::model::MAX_SIZE).suffix(" px"));
            });
        }
        ui.add_space(6.0);
        theme::toggle(ui, &mut app.output.overwrite, "Overwrite existing files", "Off: existing files are skipped");
    });
}

fn run_panel(app: &mut App, ui: &mut Ui) {
    let p = Palette::DARK;
    theme::card_frame().show(ui, |ui| {
        ui.set_width(ui.available_width());
        let count = app.batch_enabled().len();
        let running = app.batch.run.is_some();
        let width = ui.available_width();
        if let Some(run) = &app.batch.run {
            let done = run.progress.done.load(Ordering::Relaxed);
            let fraction = done as f32 / run.total.max(1) as f32;
            ui.horizontal(|ui| {
                ui.add(
                    egui::ProgressBar::new(fraction)
                        .desired_width(width - 90.0)
                        .desired_height(12.0)
                        .fill(p.accent)
                        .corner_radius(6),
                );
                ui.label(egui::RichText::new(format!("{done} / {}", run.total)).color(p.weak));
            });
            ui.add_space(6.0);
            let cancel = run.progress.clone();
            if theme::soft_button_colored(ui, Some(Icon::Close), "Stop after current", p.danger).clicked() {
                cancel.cancel.store(true, Ordering::Relaxed);
            }
        } else {
            let text = match count {
                0 => "Nothing to process".to_owned(),
                1 => "Process 1 material".to_owned(),
                n => format!("Process {n} materials"),
            };
            let r = theme::primary_button(ui, Some(Icon::Layers), &text, egui::vec2(width, 40.0), count > 0 && !running);
            if r.on_hover_text("Ctrl+E").clicked() {
                let ctx = ui.ctx().clone();
                app.start_batch(&ctx);
            }
            if let Some(dir) = app.batch.finished_dir.clone() {
                ui.add_space(6.0);
                if theme::soft_button(ui, Some(Icon::Folder), "Open output folder").clicked() {
                    crate::platform::reveal(&dir);
                }
            }
        }
        ui.add_space(8.0);
        theme::section_label(ui, "Log");
        egui::Frame::new()
            .fill(Color32::from_rgb(0x0E, 0x10, 0x14))
            .corner_radius(8)
            .inner_margin(egui::Margin::same(8))
            .show(ui, |ui| {
                ui.set_width(ui.available_width());
                egui::ScrollArea::vertical()
                    .max_height(ui.available_height().max(90.0))
                    .auto_shrink([false, false])
                    .stick_to_bottom(true)
                    .show(ui, |ui| {
                        if app.batch.log.is_empty() {
                            ui.label(egui::RichText::new("Nothing yet").monospace().size(12.0).color(p.faint));
                        }
                        for (level, text) in &app.batch.log {
                            let color = match level {
                                LogLevel::Info => p.weak,
                                LogLevel::Ok => p.good,
                                LogLevel::Warn => p.warn,
                                LogLevel::Error => p.danger,
                            };
                            ui.label(egui::RichText::new(text).monospace().size(12.0).color(color));
                        }
                    });
            });
    });
}

fn label(ui: &mut Ui, text: &str) {
    ui.label(egui::RichText::new(text).size(12.0).color(Palette::DARK.weak));
}
