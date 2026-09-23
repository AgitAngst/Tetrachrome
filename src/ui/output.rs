//! Правая панель: предпросмотр результата и экспорт.

use anvil_ui::theme::radius;
use anvil_ui::widgets as w;
use anvil_ui::{Icon, Kind, Palette, Tone, semibold};
use eframe::egui::{self, Color32, Pos2, Rect, Sense, Stroke, Ui, Vec2};

use crate::app::{App, ViewMode};
use crate::lang::{fill, t};
use crate::model::{Channel, OutputFormat, SizePolicy};
use crate::naming;
use crate::theme;

use super::panel_header;

pub fn show(app: &mut App, ui: &mut Ui) {
    let p = Palette::of(ui);
    let target = app.target_size();
    panel_header(ui, t("Output"), |ui| {
        if let Some((w, h)) = target {
            let layout = if app.work.alpha { "RGBA" } else { "RGB" };
            ui.label(egui::RichText::new(format!("{w} × {h} · {layout}")).size(12.0).color(p.weak));
        }
    });
    ui.add_space(6.0);

    if app.sizes_differ() {
        size_warning(app, ui);
        ui.add_space(8.0);
    }

    // Кнопка экспорта всегда на виду, внизу панели.
    egui::Panel::bottom("export-bar")
        .frame(egui::Frame::new().inner_margin(egui::Margin { top: 10, ..Default::default() }))
        .show_separator_line(false)
        .show(ui, |ui| export_bar(app, ui));

    view_modes(app, ui);
    ui.add_space(8.0);

    let width = ui.available_width();
    let height = (ui.available_height() - 300.0).clamp(200.0, width);
    canvas(app, ui, egui::vec2(width, height));
    ui.add_space(4.0);
    readout(app, ui);
    ui.add_space(10.0);

    egui::ScrollArea::vertical().auto_shrink([false, false]).show(ui, |ui| {
        strips(app, ui);
        ui.add_space(14.0);
        export_section(app, ui);
    });
}

fn size_warning(app: &mut App, ui: &mut Ui) {
    let p = Palette::of(ui);
    let color = Tone::Warning.color(&p);
    egui::Frame::new()
        .fill(p.soft(color))
        .stroke(Stroke::new(1.0, color.gamma_multiply(0.35)))
        .corner_radius(radius::CARD)
        .inner_margin(egui::Margin::same(10))
        .show(ui, |ui| {
            ui.set_width(ui.available_width());
            ui.horizontal(|ui| {
                theme::icon(ui, Icon::Warning, 16.0, color);
                ui.label(egui::RichText::new(t("Sources have different sizes")).font(semibold(13.0)).color(p.text));
            });
            let mut sizes = app.source_sizes();
            sizes.sort_unstable();
            sizes.dedup();
            let list: Vec<String> = sizes.iter().map(|(w, h)| format!("{w}×{h}")).collect();
            ui.label(
                egui::RichText::new(fill(t("{} — they will be resampled."), &[&list.join(", ")]))
                    .size(12.0)
                    .color(p.weak),
            );
            ui.add_space(4.0);
            theme::segmented(
                ui,
                &mut app.output.policy,
                &[
                    (SizePolicy::Largest, t("Scale to largest")),
                    (SizePolicy::Smallest, t("Scale to smallest")),
                    (SizePolicy::Custom, t("Custom")),
                ],
                None,
                0.0,
            );
        });
}

fn view_modes(app: &mut App, ui: &mut Ui) {
    let p = Palette::of(ui);
    let mut options = vec![(ViewMode::Rgba, "RGBA"), (ViewMode::Rgb, "RGB")];
    for c in Channel::ALL {
        options.push((ViewMode::Channel(c), c.letter()));
    }
    let colors = [
        p.accent_text,
        p.accent_text,
        theme::channel_color(&p, Channel::R),
        theme::channel_color(&p, Channel::G),
        theme::channel_color(&p, Channel::B),
        theme::channel_color(&p, Channel::A),
    ];
    let width = ((ui.available_width() - 4.0) / 6.0).max(36.0);
    theme::segmented(ui, &mut app.view.mode, &options, Some(&colors), width);
}

/// Большой предпросмотр: колесо — масштаб вокруг курсора, перетаскивание —
/// сдвиг, двойной щелчок — вписать. Фон тёмный в любой теме.
fn canvas(app: &mut App, ui: &mut Ui, size: Vec2) {
    let p = Palette::of(ui);
    let (rect, response) = ui.allocate_exact_size(size, Sense::click_and_drag());
    let painter = ui.painter().with_clip_rect(rect.intersect(ui.clip_rect()));
    painter.rect_filled(rect, radius::CARD, theme::PREVIEW_BG);
    app.hovered_pixel = None;

    let (Some(target), Some(texture)) = (app.target_size(), app.preview.main.as_ref()) else {
        painter.text(
            rect.center(),
            egui::Align2::CENTER_CENTER,
            t("Nothing to preview yet"),
            egui::FontId::proportional(13.0),
            theme::PREVIEW_TEXT,
        );
        painter.rect_stroke(rect, radius::CARD, Stroke::new(1.0, p.border), egui::StrokeKind::Inside);
        return;
    };
    let texture = texture.id();
    let (tw, th) = (target.0 as f32, target.1 as f32);
    let fit = ((rect.width() - 20.0) / tw).min((rect.height() - 20.0) / th).max(0.001);
    app.view.fit_zoom = fit;
    let mut zoom = app.view.zoom.unwrap_or(fit);

    if response.hovered() {
        let scroll = ui.input(|i| i.smooth_scroll_delta.y + i.zoom_delta().ln() * 200.0);
        if scroll.abs() > 0.0
            && let Some(cursor) = response.hover_pos()
        {
            let new = (zoom * (scroll * 0.003).exp()).clamp(fit.min(0.05), 64.0);
            let from_center = cursor - rect.center();
            app.view.pan = from_center - (from_center - app.view.pan) * (new / zoom);
            zoom = new;
            app.view.zoom = Some(zoom);
        }
    }
    if response.dragged() {
        app.view.pan += response.drag_delta();
        app.view.zoom = Some(zoom);
    }
    if response.double_clicked() {
        app.view.zoom = None;
        app.view.pan = Vec2::ZERO;
        zoom = fit;
    }
    if app.view.zoom.is_none() {
        app.view.pan = Vec2::ZERO;
    }

    let image_rect = Rect::from_center_size(rect.center() + app.view.pan, egui::vec2(tw * zoom, th * zoom));
    let alpha_view = app.view.mode == ViewMode::Rgba && app.alpha_visible();
    if alpha_view {
        theme::checkerboard(&painter, image_rect, 10.0);
    }
    painter.image(texture, image_rect, Rect::from_min_max(Pos2::ZERO, Pos2::new(1.0, 1.0)), Color32::WHITE);
    if app.view.mode == ViewMode::Rgba && app.work.alpha && !app.alpha_visible() {
        let value = app.work.slots[Channel::A.index()].fill;
        let text = fill(t("Alpha is a constant {} — shown opaque"), &[&value]);
        let galley = painter.layout_no_wrap(text, egui::FontId::proportional(11.5), theme::PREVIEW_TEXT);
        let chip = Rect::from_min_size(rect.min + egui::vec2(10.0, 10.0), galley.size() + egui::vec2(14.0, 8.0));
        painter.rect_filled(chip, 6, Color32::from_black_alpha(170));
        painter.galley(chip.min + egui::vec2(7.0, 4.0), galley, theme::PREVIEW_TEXT);
    }
    // Сетка пикселей при сильном увеличении.
    if zoom >= 16.0 {
        pixel_grid(&painter, image_rect, zoom, rect);
    }
    painter.rect_stroke(rect, radius::CARD, Stroke::new(1.0, p.border), egui::StrokeKind::Inside);

    if let Some(pos) = response.hover_pos()
        && image_rect.contains(pos)
    {
        let x = (((pos.x - image_rect.min.x) / zoom) as u32).min(target.0 - 1);
        let y = (((pos.y - image_rect.min.y) / zoom) as u32).min(target.1 - 1);
        app.hovered_pixel = Some((x, y));
        ui.ctx().set_cursor_icon(if response.dragged() {
            egui::CursorIcon::Grabbing
        } else {
            egui::CursorIcon::Crosshair
        });
    }
}

fn pixel_grid(painter: &egui::Painter, image: Rect, zoom: f32, clip: Rect) {
    let visible = image.intersect(clip);
    let stroke = Stroke::new(1.0, Color32::from_black_alpha(60));
    let start_x = ((visible.min.x - image.min.x) / zoom).floor() as i64;
    let end_x = ((visible.max.x - image.min.x) / zoom).ceil() as i64;
    for i in start_x..=end_x {
        let x = image.min.x + i as f32 * zoom;
        painter.line_segment([Pos2::new(x, visible.min.y), Pos2::new(x, visible.max.y)], stroke);
    }
    let start_y = ((visible.min.y - image.min.y) / zoom).floor() as i64;
    let end_y = ((visible.max.y - image.min.y) / zoom).ceil() as i64;
    for i in start_y..=end_y {
        let y = image.min.y + i as f32 * zoom;
        painter.line_segment([Pos2::new(visible.min.x, y), Pos2::new(visible.max.x, y)], stroke);
    }
}

/// Значения под курсором — точные, из полноразмерных исходников.
fn readout(app: &App, ui: &mut Ui) {
    let p = Palette::of(ui);
    let height = 22.0;
    let (rect, _) = ui.allocate_exact_size(egui::vec2(ui.available_width(), height), Sense::hover());
    let painter = ui.painter();
    let Some((x, y)) = app.hovered_pixel else {
        painter.text(
            rect.left_center(),
            egui::Align2::LEFT_CENTER,
            t("Scroll to zoom · drag to pan · double-click to fit"),
            egui::FontId::proportional(12.0),
            p.weak,
        );
        return;
    };
    let Some(value) = app.pixel_value(x, y) else {
        return;
    };
    let mono = egui::FontId::monospace(12.5);
    let mut pos = rect.left_center();
    let mut put = |text: String, color: Color32| {
        let r = painter.text(pos, egui::Align2::LEFT_CENTER, text, mono.clone(), color);
        pos.x = r.max.x;
    };
    put(format!("{x:>4},{y:<4}"), p.weak);
    put("  ".to_owned(), p.weak);
    for &c in app.work.channels() {
        put(format!(" {}", c.letter()), theme::channel_color(&p, c));
        put(format!(" {:<3}", value[c.index()]), p.text);
    }
    // Образец цвета справа.
    let swatch = Rect::from_center_size(rect.right_center() - egui::vec2(10.0, 0.0), Vec2::splat(16.0));
    painter.rect_filled(swatch, 4, Color32::from_rgb(value[0], value[1], value[2]));
    painter.rect_stroke(swatch, 4, Stroke::new(1.0, p.border_strong), egui::StrokeKind::Outside);
}

/// Четыре полосы — каждый канал отдельно, в оттенках серого.
fn strips(app: &mut App, ui: &mut Ui) {
    let p = Palette::of(ui);
    w::section_label(ui, t("Channels"));
    ui.add_space(4.0);
    let gap = 8.0;
    let width = (ui.available_width() - gap * 3.0) / 4.0;
    let (pw, ph) = app.preview.size;
    let aspect = if pw > 0 { ph as f32 / pw as f32 } else { 1.0 };
    let side = egui::vec2(width, (width * aspect).clamp(width * 0.4, width * 1.6));
    ui.horizontal_top(|ui| {
        ui.spacing_mut().item_spacing.x = gap;
        for channel in Channel::ALL {
            let i = channel.index();
            let enabled = channel != Channel::A || app.work.alpha;
            ui.vertical(|ui| {
                ui.set_width(width);
                let (rect, response) =
                    ui.allocate_exact_size(side, if enabled { Sense::click() } else { Sense::hover() });
                let painter = ui.painter();
                painter.rect_filled(rect, radius::CONTROL, theme::PREVIEW_BG);
                if enabled && let Some(texture) = &app.preview.strips[i] {
                    painter.image(
                        texture.id(),
                        rect.shrink(1.0),
                        Rect::from_min_max(Pos2::ZERO, Pos2::new(1.0, 1.0)),
                        Color32::WHITE,
                    );
                } else {
                    painter.text(
                        rect.center(),
                        egui::Align2::CENTER_CENTER,
                        "—",
                        egui::FontId::proportional(14.0),
                        theme::PREVIEW_TEXT,
                    );
                }
                let color = theme::channel_color(&p, channel);
                let active = app.view.mode == ViewMode::Channel(channel);
                let stroke = if active {
                    Stroke::new(2.0, color)
                } else if response.hovered() {
                    Stroke::new(1.0, color.gamma_multiply(0.7))
                } else {
                    Stroke::new(1.0, p.border)
                };
                painter.rect_stroke(rect, radius::CONTROL, stroke, egui::StrokeKind::Inside);
                if response
                    .on_hover_cursor(egui::CursorIcon::PointingHand)
                    .on_hover_text(t("Show this channel alone"))
                    .clicked()
                {
                    app.view.mode = if active { ViewMode::Rgba } else { ViewMode::Channel(channel) };
                }
                let role = app.work.slots[i].role.trim();
                ui.horizontal(|ui| {
                    ui.spacing_mut().item_spacing.x = 4.0;
                    ui.label(egui::RichText::new(channel.letter()).font(semibold(12.0)).color(color));
                    let text = if !enabled {
                        t("off").to_owned()
                    } else if role.is_empty() {
                        format!("= {}", app.work.slots[i].fill)
                    } else {
                        role.to_owned()
                    };
                    ui.add(egui::Label::new(egui::RichText::new(text).size(11.5).color(p.weak)).truncate());
                });
            });
        }
    });
}

fn export_section(app: &mut App, ui: &mut Ui) {
    let p = Palette::of(ui);
    w::section_label(ui, t("Export"));
    ui.add_space(6.0);

    theme::field_label(ui, t("File name"));
    ui.add(
        egui::TextEdit::singleline(&mut app.output.template)
            .desired_width(ui.available_width())
            .font(egui::TextStyle::Monospace)
            .margin(egui::vec2(8.0, 6.0)),
    );
    ui.add_space(2.0);
    ui.horizontal_wrapped(|ui| {
        ui.spacing_mut().item_spacing = egui::vec2(4.0, 4.0);
        for (token, what) in naming::tokens() {
            let r = ui.add(
                egui::Button::new(egui::RichText::new(token).monospace().size(11.5).color(p.accent_text))
                    .fill(p.soft(p.accent))
                    .corner_radius(radius::SMALL)
                    .min_size(egui::vec2(0.0, 20.0)),
            );
            if r.on_hover_text(fill(t("{} — click to append"), &[&what])).clicked() {
                app.output.template.push_str(token);
            }
        }
    });
    ui.add_space(8.0);

    theme::field_label(ui, t("Folder"));
    ui.horizontal(|ui| {
        let dir = app.output_dir();
        let text = match (&dir, app.output.folder.is_some()) {
            (Some(d), true) => d.display().to_string(),
            (Some(d), false) => fill(t("{}  (next to sources)"), &[&d.display()]),
            (None, _) => t("Next to the first source").to_owned(),
        };
        let avail = ui.available_width() - 70.0;
        ui.add_sized([avail, 26.0], egui::Label::new(egui::RichText::new(text).size(12.5).color(p.weak)).truncate())
            .on_hover_text(dir.map(|d| d.display().to_string()).unwrap_or_default());
        if theme::icon_button(ui, Icon::Folder, 28.0, t("Choose output folder")).clicked() {
            app.pick_output_folder();
        }
        ui.add_enabled_ui(app.output.folder.is_some(), |ui| {
            if theme::icon_button(ui, Icon::Refresh, 28.0, t("Back to “next to sources”")).clicked() {
                app.output.folder = None;
            }
        });
    });
    ui.add_space(8.0);

    ui.horizontal(|ui| {
        ui.vertical(|ui| {
            theme::field_label(ui, t("Format"));
            let options: Vec<(OutputFormat, &str)> = OutputFormat::ALL.iter().map(|f| (*f, f.label())).collect();
            theme::segmented(ui, &mut app.output.format, &options, None, 48.0);
        });
        ui.add_space(8.0);
        ui.vertical(|ui| {
            theme::field_label(ui, t("Bit depth"));
            let supports = app.output.format.supports_16bit();
            ui.add_enabled_ui(supports, |ui| {
                let mut sixteen = app.output.sixteen && supports;
                let options = [(false, t("8-bit")), (true, t("16-bit"))];
                if theme::segmented(ui, &mut sixteen, &options, None, 56.0).changed() {
                    app.output.sixteen = sixteen;
                }
            })
            .response
            .on_disabled_hover_text(t("TGA is always 8-bit"));
        });
    });
    ui.add_space(8.0);

    theme::field_label(ui, t("Size"));
    ui.horizontal(|ui| {
        let mut custom = app.output.policy == SizePolicy::Custom;
        if custom {
            ui.add(
                egui::DragValue::new(&mut app.output.custom.0)
                    .range(1..=crate::model::MAX_SIZE)
                    .speed(4.0)
                    .suffix(" px"),
            );
            ui.label(egui::RichText::new("×").color(p.weak));
            ui.add(
                egui::DragValue::new(&mut app.output.custom.1)
                    .range(1..=crate::model::MAX_SIZE)
                    .speed(4.0)
                    .suffix(" px"),
            );
        } else {
            let text = match app.target_size() {
                Some((w, h)) => format!("{w} × {h}"),
                None => "—".to_owned(),
            };
            ui.label(egui::RichText::new(text).color(p.text));
            let from = match app.output.policy {
                SizePolicy::Smallest => t("from smallest source"),
                _ => t("from largest source"),
            };
            ui.label(egui::RichText::new(from).size(12.0).color(p.weak));
        }
        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
            if w::toggle(ui, &mut custom, t("Custom")).on_hover_text(t("Set the output size by hand")).changed() {
                if custom {
                    if let Some(size) = app.target_size() {
                        app.output.custom = size;
                    }
                    app.output.policy = SizePolicy::Custom;
                } else {
                    app.output.policy = SizePolicy::Largest;
                }
            }
        });
    });
}

/// Итоговое имя и кнопка экспорта.
fn export_bar(app: &mut App, ui: &mut Ui) {
    let p = Palette::of(ui);
    ui.horizontal(|ui| {
        ui.label(egui::RichText::new("→").color(p.weak));
        ui.add(egui::Label::new(egui::RichText::new(app.output_name()).font(semibold(13.5)).color(p.text)).truncate());
    });
    ui.add_space(6.0);
    let ready = app.can_export();
    let label = if app.exporting() { t("Exporting…") } else { t("Export") };
    let width = ui.available_width();
    let r = ui
        .add_enabled_ui(ready.is_ok(), |ui| {
            w::button_sized(ui, Kind::Primary, Some(Icon::Download), label, egui::vec2(width, 40.0))
        })
        .inner;
    let r = match ready {
        Ok(()) => r.on_hover_text("Ctrl+E"),
        Err(why) => r.on_disabled_hover_text(why),
    };
    if r.clicked() {
        let ctx = ui.ctx().clone();
        app.export(&ctx, false);
    }
    if let Some(path) = app.last_export.clone() {
        ui.add_space(6.0);
        ui.horizontal(|ui| {
            theme::icon(ui, Icon::Check, 14.0, p.success);
            ui.add(
                egui::Label::new(egui::RichText::new(crate::source::display_name(&path)).size(12.0).color(p.weak))
                    .truncate(),
            );
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                if w::button(ui, Kind::Secondary, Some(Icon::Folder), t("Open folder")).clicked() {
                    crate::platform::reveal(&path);
                }
            });
        });
    }
    ui.add_space(4.0);
}
