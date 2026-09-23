//! Пакетный режим: таблица материалов, настройки вывода, прогресс и журнал.

use std::sync::atomic::Ordering;

use anvil_ui::theme::radius;
use anvil_ui::widgets as w;
use anvil_ui::{Icon, Kind, Palette, semibold};
use eframe::egui::{self, Color32, Sense, Stroke, Ui, Vec2};

use crate::app::App;
use crate::batch::{self, LogLevel};
use crate::lang::{count, fill, t};
use crate::model::{Channel, OutputFormat, SizePolicy};
use crate::source;
use crate::theme;

pub fn show(app: &mut App, ui: &mut Ui) {
    let p = Palette::of(ui);
    toolbar(app, ui);
    ui.add_space(12.0);

    let running = app.batch.run.is_some();
    // Низ окна: вывод, кнопка, журнал. Таблица занимает остальное.
    let bottom = 300.0f32.min(ui.available_height() * 0.5);
    let table_height = (ui.available_height() - bottom).max(160.0);
    egui::Frame::new()
        .fill(p.card)
        .stroke(Stroke::new(1.0, p.border))
        .corner_radius(radius::CARD)
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
    let p = Palette::of(ui);
    ui.horizontal(|ui| {
        ui.label(egui::RichText::new(t("Batch")).font(semibold(20.0)).color(p.text));
        ui.add_space(4.0);
        theme::chip(ui, &app.work.name, theme::engine_color(&p, app.work.engine));
        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
            let running = app.batch.run.is_some();
            ui.add_enabled_ui(!running && !app.batch.files.is_empty(), |ui| {
                if w::button(ui, Kind::Secondary, Some(Icon::Trash), t("Clear")).clicked() {
                    app.batch.files.clear();
                    app.batch.disabled.clear();
                    app.batch.status.clear();
                    app.refresh_plan();
                }
            });
            ui.add_enabled_ui(!running, |ui| {
                if w::button(ui, Kind::Secondary, Some(Icon::Folder), t("Add folder…")).clicked()
                    && let Some(dir) = rfd::FileDialog::new().pick_folder()
                {
                    app.batch_add(vec![dir]);
                }
                if w::button(ui, Kind::Secondary, Some(Icon::Plus), t("Add files…")).clicked()
                    && let Some(files) = rfd::FileDialog::new().add_filter(t("Images"), source::EXTENSIONS).pick_files()
                {
                    app.batch_add(files);
                }
            });
        });
    });
    let plan = &app.batch.plan;
    let text = fill(
        t("{} · {} · {} unmatched — maps are grouped by name and sorted into channels by the preset's suffixes"),
        &[
            &count(app.batch.files.len(), ["file", "files"], ["файл", "файла", "файлов"]),
            &count(plan.groups.len(), ["material", "materials"], ["материал", "материала", "материалов"]),
            &plan.unmatched.len(),
        ],
    );
    ui.label(egui::RichText::new(text).color(p.weak));
}

fn table(app: &mut App, ui: &mut Ui) {
    let p = Palette::of(ui);
    if app.batch.files.is_empty() {
        ui.centered_and_justified(|ui| {
            ui.label(
                egui::RichText::new(format!(
                    "{}\nRock_Metallic.png, Rock_AO.png, Rock_Smoothness.png → Rock_MaskMap.png",
                    t("Drop a folder with maps here, or use “Add folder…”."),
                ))
                .color(p.weak),
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
                ui.label(egui::RichText::new(t("Material")).font(semibold(12.0)).color(p.weak));
                for &c in channels {
                    ui.horizontal(|ui| {
                        ui.label(
                            egui::RichText::new(c.letter()).font(semibold(12.0)).color(theme::channel_color(&p, c)),
                        );
                        let role = app.work.slots[c.index()].role.trim();
                        let role = if role.is_empty() { t("constant") } else { role };
                        ui.label(egui::RichText::new(role).size(12.0).color(p.weak));
                    });
                }
                ui.label(egui::RichText::new(t("Status")).font(semibold(12.0)).color(p.weak));
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
                    ui.label(egui::RichText::new(&group.base).font(semibold(13.0)).color(name_color));
                    for &c in channels {
                        cell(app, ui, group.files[c.index()].as_deref(), c);
                    }
                    let status = app.batch.status.get(&key).copied();
                    let roles = channels.iter().filter(|c| !app.work.slots[c.index()].role.is_empty()).count();
                    match (status, filled) {
                        (Some(true), _) => status_label(ui, Icon::Check, t("done"), p.success),
                        (Some(false), _) => status_label(ui, Icon::Warning, t("failed"), p.danger),
                        (None, 0) => status_label(ui, Icon::Warning, t("no maps"), p.warning),
                        (None, n) if n < roles => status_label(ui, Icon::Info, t("partial"), p.warning),
                        _ => status_label(ui, Icon::Check, t("ready"), p.weak),
                    }
                    ui.end_row();
                }
            });

        let plan = &app.batch.plan;
        if !plan.unmatched.is_empty() {
            ui.add_space(10.0);
            egui::CollapsingHeader::new(
                egui::RichText::new(fill(t("{} files matched no channel"), &[&plan.unmatched.len()])).color(p.warning),
            )
            .id_salt("unmatched")
            .show(ui, |ui| {
                ui.label(
                    egui::RichText::new(t("Add their suffixes to the preset's channels in the left panel."))
                        .size(12.0)
                        .color(p.weak),
                );
                for path in &plan.unmatched {
                    ui.label(egui::RichText::new(source::display_name(path)).monospace().size(12.0).color(p.weak));
                }
            });
        }
        if !plan.conflicts.is_empty() {
            ui.add_space(6.0);
            egui::CollapsingHeader::new(
                egui::RichText::new(fill(t("{} duplicate maps ignored"), &[&plan.conflicts.len()])).color(p.warning),
            )
            .id_salt("conflicts")
            .show(ui, |ui| {
                for (path, what) in &plan.conflicts {
                    ui.label(
                        egui::RichText::new(fill(t("{} — {} already taken"), &[&source::display_name(path), what]))
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
    let p = Palette::of(ui);
    match file {
        Some(path) => {
            ui.horizontal(|ui| {
                ui.spacing_mut().item_spacing.x = 5.0;
                let (r, _) = ui.allocate_exact_size(Vec2::splat(8.0), Sense::hover());
                ui.painter().circle_filled(r.center(), 3.5, theme::channel_color(&p, channel));
                ui.add(
                    egui::Label::new(egui::RichText::new(source::display_name(path)).size(12.5).color(p.weak))
                        .truncate(),
                )
                .on_hover_text(path.display().to_string());
            });
        }
        None => {
            let config = &app.work.slots[channel.index()];
            let text =
                if config.role.is_empty() { format!("= {}", config.fill) } else { format!("— = {}", config.fill) };
            ui.label(egui::RichText::new(text).size(12.0).color(p.weak));
        }
    }
}

fn status_label(ui: &mut Ui, icon: Icon, text: &str, color: Color32) {
    ui.horizontal(|ui| {
        ui.spacing_mut().item_spacing.x = 4.0;
        theme::icon(ui, icon, 14.0, color);
        ui.label(egui::RichText::new(text).size(12.0).color(color));
    });
}

fn settings(app: &mut App, ui: &mut Ui) {
    w::card(ui, |ui| {
        let p = Palette::of(ui);
        w::section_label(ui, t("Output"));
        ui.add_space(6.0);
        theme::field_label(ui, t("Folder"));
        ui.horizontal(|ui| {
            let dir = app.batch_out_dir();
            let text = match (&dir, app.output.folder.is_some()) {
                (Some(d), true) => d.display().to_string(),
                (Some(d), false) => fill(t("{}  (default)"), &[&d.display()]),
                (None, _) => t("“packed” next to the sources").to_owned(),
            };
            ui.add_sized(
                [ui.available_width() - 36.0, 26.0],
                egui::Label::new(egui::RichText::new(text).size(12.5).color(p.weak)).truncate(),
            );
            if theme::icon_button(ui, Icon::Folder, 28.0, t("Choose output folder")).clicked() {
                app.pick_output_folder();
            }
        });
        ui.add_space(4.0);
        theme::field_label(ui, t("File name"));
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
                let options = [(false, t("8-bit")), (true, t("16-bit"))];
                if theme::segmented(ui, &mut sixteen, &options, None, 54.0).changed() {
                    app.output.sixteen = sixteen;
                }
            });
        });
        ui.add_space(6.0);
        theme::field_label(ui, t("When sizes differ"));
        theme::segmented(
            ui,
            &mut app.output.policy,
            &[
                (SizePolicy::Largest, t("Largest")),
                (SizePolicy::Smallest, t("Smallest")),
                (SizePolicy::Custom, t("Custom")),
            ],
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
        w::toggle(ui, &mut app.output.overwrite, t("Overwrite existing files"))
            .on_hover_text(t("Off: existing files are skipped"));
    });
}

fn run_panel(app: &mut App, ui: &mut Ui) {
    w::card(ui, |ui| {
        let p = Palette::of(ui);
        let materials = app.batch_enabled().len();
        let running = app.batch.run.is_some();
        let width = ui.available_width();
        if let Some(run) = &app.batch.run {
            let done = run.progress.done.load(Ordering::Relaxed);
            let fraction = done as f32 / run.total.max(1) as f32;
            ui.horizontal(|ui| {
                w::progress(ui, Some(fraction), width - 90.0);
                ui.label(egui::RichText::new(format!("{done} / {}", run.total)).color(p.weak));
            });
            ui.add_space(6.0);
            let cancel = run.progress.clone();
            if w::button(ui, Kind::Danger, Some(Icon::Stop), t("Stop after current")).clicked() {
                cancel.cancel.store(true, Ordering::Relaxed);
            }
        } else {
            let text = match materials {
                0 => t("Nothing to process").to_owned(),
                n => fill(
                    t("Process {}"),
                    &[&count(n, ["material", "materials"], ["материал", "материала", "материалов"])],
                ),
            };
            let size = egui::vec2(width, 40.0);
            let r = ui
                .add_enabled_ui(materials > 0 && !running, |ui| {
                    w::button_sized(ui, Kind::Primary, Some(Icon::Layers), &text, size)
                })
                .inner;
            if r.on_hover_text("Ctrl+E").clicked() {
                let ctx = ui.ctx().clone();
                app.start_batch(&ctx);
            }
            if let Some(dir) = app.batch.finished_dir.clone() {
                ui.add_space(6.0);
                if w::button(ui, Kind::Secondary, Some(Icon::Folder), t("Open output folder")).clicked() {
                    crate::platform::reveal(&dir);
                }
            }
        }
        ui.add_space(8.0);
        w::section_label(ui, t("Log"));
        egui::Frame::new()
            .fill(p.field)
            .stroke(Stroke::new(1.0, p.border))
            .corner_radius(radius::CONTROL)
            .inner_margin(egui::Margin::same(8))
            .show(ui, |ui| {
                ui.set_width(ui.available_width());
                egui::ScrollArea::vertical()
                    .max_height(ui.available_height().max(90.0))
                    .auto_shrink([false, false])
                    .stick_to_bottom(true)
                    .show(ui, |ui| {
                        if app.batch.log.is_empty() {
                            ui.label(egui::RichText::new(t("Nothing yet")).monospace().size(12.0).color(p.weak));
                        }
                        for (level, text) in &app.batch.log {
                            let color = match level {
                                LogLevel::Info => p.weak,
                                LogLevel::Ok => p.success,
                                LogLevel::Warn => p.warning,
                                LogLevel::Error => p.danger,
                            };
                            ui.label(egui::RichText::new(text).monospace().size(12.0).color(color));
                        }
                    });
            });
    });
}
