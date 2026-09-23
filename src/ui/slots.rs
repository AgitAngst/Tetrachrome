//! Середина окна: карточки четырёх каналов.

use std::sync::Arc;

use anvil_ui::theme::radius;
use anvil_ui::widgets as w;
use anvil_ui::{Icon, Kind, Palette, semibold};
use eframe::egui::{self, Color32, Pos2, Rect, Sense, Shape, Stroke, Ui, Vec2};

use crate::app::{App, LoadTarget};
use crate::lang::{fill, t};
use crate::model::{Channel, SourceChannel};
use crate::source::Source;
use crate::theme;

use super::presets_panel::paint_badge;

const CARD_HEIGHT: f32 = 236.0;
const THUMB: f32 = 128.0;

pub fn show(app: &mut App, ui: &mut Ui) {
    header(app, ui);
    ui.add_space(14.0);
    app.card_rects = [None; 4];
    egui::ScrollArea::vertical().auto_shrink([false, false]).show(ui, |ui| {
        let gap = 14.0;
        let width = ui.available_width();
        let columns = if width >= 700.0 { 2 } else { 1 };
        let card_w = (width - gap * (columns - 1) as f32) / columns as f32;
        for row in Channel::ALL.chunks(columns) {
            ui.horizontal_top(|ui| {
                ui.spacing_mut().item_spacing.x = gap;
                for &channel in row {
                    let (rect, _) = ui.allocate_exact_size(egui::vec2(card_w, CARD_HEIGHT), Sense::hover());
                    let mut child = ui.new_child(
                        egui::UiBuilder::new().max_rect(rect).layout(egui::Layout::top_down(egui::Align::Min)),
                    );
                    card(app, &mut child, channel, rect);
                    app.card_rects[channel.index()] = Some(rect.intersect(ui.clip_rect()));
                }
            });
            ui.add_space(gap);
        }
        hint(app, ui);
    });
}

fn header(app: &mut App, ui: &mut Ui) {
    let p = Palette::of(ui);
    ui.horizontal(|ui| {
        // Кнопки справа — первыми, а имя пресета берёт остаток и обрезается, а не лезет под них.
        // В узкой середине окна от кнопок остаются значки, подпись — в подсказке.
        let compact = ui.available_width() < 640.0;
        let label = |text: &'static str| if compact { "" } else { text };
        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
            let any = app.slots.iter().any(Option::is_some);
            ui.add_enabled_ui(any, |ui| {
                if w::button(ui, Kind::Secondary, Some(Icon::Trash), label(t("Clear all")))
                    .on_hover_text(t("Clear all"))
                    .clicked()
                {
                    app.clear_all();
                }
            });
            if w::button(ui, Kind::Secondary, Some(Icon::Folder), label(t("Load maps…")))
                .on_hover_text(t("Pick several maps of one material — they are assigned by name suffix"))
                .clicked()
            {
                app.pick_images(LoadTarget::Auto { single: false });
            }
            ui.with_layout(egui::Layout::left_to_right(egui::Align::Center), |ui| {
                let modified = app.modified();
                let chips = 80.0 + if modified { 100.0 } else { 0.0 };
                ui.scope(|ui| {
                    ui.set_max_width((ui.available_width() - chips).max(60.0));
                    let name = egui::RichText::new(&app.work.name).font(semibold(20.0)).color(p.text);
                    ui.add(egui::Label::new(name).truncate());
                });
                ui.add_space(4.0);
                theme::chip(ui, app.work.engine.label(), theme::engine_color(&p, app.work.engine));
                if modified {
                    theme::chip(ui, t("MODIFIED"), p.warning)
                        .on_hover_text(t("Settings differ from the saved preset. Ctrl+S to save."));
                }
            });
        });
    });
    let target = app.target_size();
    let layout = if app.work.alpha { "RGBA" } else { "RGB" };
    let size = match target {
        Some((w, h)) => format!("{w} × {h}"),
        None => t("no size yet").to_owned(),
    };
    let filled = app.work.channels().iter().filter(|c| app.slots[c.index()].is_some()).count();
    let label = if app.work.label.is_empty() { "—" } else { &app.work.label };
    let text = fill(
        t("{} · {} · {} of {} channels from images · label {}"),
        &[&layout, &size, &filled, &app.work.channels().len(), &label],
    );
    ui.label(egui::RichText::new(text).color(p.weak));
}

fn card(app: &mut App, ui: &mut Ui, channel: Channel, rect: Rect) {
    let p = Palette::of(ui);
    let i = channel.index();
    let color = theme::channel_color(&p, channel);
    let enabled = channel != Channel::A || app.work.alpha;
    let hovered = ui.rect_contains_pointer(rect);
    ui.painter().rect_filled(rect, radius::CARD, if hovered && enabled { p.raised } else { p.card });
    ui.painter().rect_stroke(rect, radius::CARD, Stroke::new(1.0, p.border), egui::StrokeKind::Inside);
    // Цветная полоска сверху: канал видно краем глаза.
    let strip = Rect::from_min_size(rect.min + egui::vec2(14.0, 0.0), egui::vec2(rect.width() - 28.0, 2.0));
    ui.painter().rect_filled(strip, 1, color.gamma_multiply(if enabled { 0.8 } else { 0.25 }));

    let inner = rect.shrink2(egui::vec2(16.0, 14.0));
    let mut ui = ui.new_child(egui::UiBuilder::new().max_rect(inner).layout(egui::Layout::top_down(egui::Align::Min)));
    let ui = &mut ui;

    // Шапка: буква, роль, кнопки.
    ui.horizontal(|ui| {
        let (badge, _) = ui.allocate_exact_size(Vec2::splat(28.0), Sense::hover());
        paint_badge(ui.painter(), &p, badge, channel);
        ui.add_space(2.0);
        ui.vertical(|ui| {
            ui.spacing_mut().item_spacing.y = 0.0;
            let role = app.work.slots[i].role.trim();
            let title = if role.is_empty() { t("Unused") } else { role };
            let title_color = if role.is_empty() { p.weak } else { p.text };
            ui.label(egui::RichText::new(title).font(semibold(15.0)).color(title_color));
            let sub = match (enabled, &app.slots[i]) {
                (false, _) => t("not written — RGB output").to_owned(),
                (true, Some(_)) => channel_name(channel).to_owned(),
                (true, None) => fill(t("constant {}"), &[&app.work.slots[i].fill]),
            };
            ui.label(egui::RichText::new(sub).size(12.0).color(p.weak));
        });
        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
            if channel == Channel::A {
                w::toggle(ui, &mut app.work.alpha, "").on_hover_text(t("Write the alpha channel"));
            }
            if enabled
                && app.slots[i].is_some()
                && theme::icon_button(ui, Icon::Close, 26.0, t("Clear this channel")).clicked()
            {
                app.clear_slot(channel);
            }
            if enabled {
                library_button(app, ui, channel);
            }
        });
    });
    ui.add_space(10.0);

    if !enabled {
        ui.add_space(30.0);
        ui.vertical_centered(|ui| {
            ui.label(egui::RichText::new(t("The output has no alpha channel.")).color(p.weak));
            ui.add_space(6.0);
            if w::button(ui, Kind::Secondary, Some(Icon::Plus), t("Add alpha channel")).clicked() {
                app.work.alpha = true;
            }
        });
        return;
    }

    ui.horizontal_top(|ui| {
        thumb(app, ui, channel);
        ui.add_space(8.0);
        ui.vertical(|ui| {
            ui.set_width(ui.available_width());
            match app.slots[i].clone() {
                Some(src) => filled_details(app, ui, channel, &src),
                None => empty_details(app, ui, channel),
            }
        });
    });
}

fn channel_name(channel: Channel) -> &'static str {
    match channel {
        Channel::R => t("Red channel"),
        Channel::G => t("Green channel"),
        Channel::B => t("Blue channel"),
        Channel::A => t("Alpha channel"),
    }
}

/// Миниатюра-приёмник: щелчок — выбрать файл, сюда же бросают из Проводника.
fn thumb(app: &mut App, ui: &mut Ui, channel: Channel) {
    let p = Palette::of(ui);
    let (rect, response) = ui.allocate_exact_size(Vec2::splat(THUMB), Sense::click());
    let painter = ui.painter().clone();
    let color = theme::channel_color(&p, channel);
    match app.slots[channel.index()].clone() {
        Some(src) => {
            let ctx = ui.ctx().clone();
            let texture = app.thumb(&ctx, &src);
            let size = fit(Vec2::new(src.thumb.width() as f32, src.thumb.height() as f32), rect.size());
            let image_rect = Rect::from_center_size(rect.center(), size);
            painter.rect_filled(rect, 8, theme::PREVIEW_BG);
            theme::checkerboard(&painter.with_clip_rect(image_rect), image_rect, 8.0);
            painter.image(texture, image_rect, Rect::from_min_max(Pos2::ZERO, Pos2::new(1.0, 1.0)), Color32::WHITE);
            painter.rect_stroke(rect, 8, Stroke::new(1.0, p.border), egui::StrokeKind::Inside);
            if response.hovered() {
                // Поверх картинки — всегда тёмная вуаль и светлый текст, в любой теме.
                painter.rect_filled(rect, 8, Color32::from_black_alpha(160));
                let icon = Rect::from_center_size(rect.center() - egui::vec2(0.0, 10.0), Vec2::splat(22.0));
                anvil_ui::icons::paint(&painter, icon, Icon::Folder, Color32::WHITE);
                painter.text(
                    rect.center() + egui::vec2(0.0, 14.0),
                    egui::Align2::CENTER_CENTER,
                    t("Replace…"),
                    semibold(12.5),
                    Color32::WHITE,
                );
            }
        }
        None => {
            let fill = app.work.slots[channel.index()].fill;
            painter.rect_filled(rect, 8, Color32::from_gray(fill).gamma_multiply(0.18).lerp_to_gamma(p.raised, 0.6));
            let stroke = Stroke::new(1.2, if response.hovered() { color } else { p.border_strong });
            dashed_rect(&painter, rect.shrink(1.0), stroke);
            let icon = Rect::from_center_size(rect.center() - egui::vec2(0.0, 12.0), Vec2::splat(24.0));
            anvil_ui::icons::paint(&painter, icon, Icon::Plus, if response.hovered() { color } else { p.weak });
            painter.text(
                rect.center() + egui::vec2(0.0, 14.0),
                egui::Align2::CENTER_CENTER,
                t("Drop or click"),
                egui::FontId::proportional(12.0),
                p.weak,
            );
        }
    }
    let response = response.on_hover_cursor(egui::CursorIcon::PointingHand);
    if response.clicked() {
        app.pick_images(LoadTarget::Slot(channel));
    }
}

fn filled_details(app: &mut App, ui: &mut Ui, channel: Channel, src: &Arc<Source>) {
    let p = Palette::of(ui);
    let i = channel.index();
    ui.add(egui::Label::new(egui::RichText::new(&src.name).font(semibold(13.5)).color(p.text)).truncate())
        .on_hover_text(src.path.display().to_string());
    ui.label(egui::RichText::new(format!("{} × {} · {}", src.width, src.height, src.format)).size(12.0).color(p.weak));
    ui.add_space(8.0);

    theme::field_label(ui, t("Read from"));
    let options: Vec<(SourceChannel, &str)> = SourceChannel::ALL.iter().map(|s| (*s, s.label())).collect();
    let colors = [
        theme::channel_color(&p, Channel::R),
        theme::channel_color(&p, Channel::G),
        theme::channel_color(&p, Channel::B),
        theme::channel_color(&p, Channel::A),
        theme::luminance_color(&p),
    ];
    let width = ((ui.available_width() - 4.0) / 5.0).clamp(30.0, 52.0);
    theme::segmented(ui, &mut app.work.slots[i].source, &options, Some(&colors), width)
        .on_hover_text(t("Which channel of the source image goes into this output channel. Lum = Rec.709 luminance."));
    ui.add_space(8.0);
    ui.horizontal_wrapped(|ui| {
        ui.spacing_mut().item_spacing.x = 14.0;
        w::toggle(ui, &mut app.work.slots[i].invert, t("Invert")).on_hover_text(t("1 − value: roughness ⇄ smoothness"));
        w::toggle(ui, &mut app.work.slots[i].srgb, t("sRGB input"))
            .on_hover_text(t("Convert from sRGB to linear before packing. Use for maps authored as color textures."));
    });
}

fn empty_details(app: &mut App, ui: &mut Ui, channel: Channel) {
    let p = Palette::of(ui);
    let i = channel.index();
    ui.label(egui::RichText::new(t("No image")).font(semibold(13.5)).color(p.weak));
    let suffixes = &app.work.slots[i].suffixes;
    let hint = if suffixes.is_empty() {
        t("Filled with a constant value").to_owned()
    } else {
        let shown: Vec<&str> = suffixes.iter().take(2).map(String::as_str).collect();
        fill(t("Auto-assigns *{}*"), &[&shown.join("*, *")])
    };
    // Одной строкой: длинный список суффиксов не должен выталкивать кнопку за карточку.
    ui.add(egui::Label::new(egui::RichText::new(hint).size(12.0).color(p.weak)).truncate());
    ui.add_space(8.0);
    ui.horizontal(|ui| {
        theme::field_label(ui, t("Fill value"));
        ui.add(egui::DragValue::new(&mut app.work.slots[i].fill).range(0..=255).speed(1.0))
            .on_hover_text(t("Drag or type 0–255"));
    });
    let mut preset: u8 = app.work.slots[i].fill;
    let width = ((ui.available_width() - 4.0) / 3.0).min(60.0);
    let options = [(0u8, t("Black")), (128u8, t("Gray")), (255u8, t("White"))];
    if theme::segmented(ui, &mut preset, &options, None, width).changed() {
        app.work.slots[i].fill = preset;
    }
    ui.add_space(8.0);
    if w::button(ui, Kind::Secondary, Some(Icon::Image), t("Browse…")).clicked() {
        app.pick_images(LoadTarget::Slot(channel));
    }
}

/// Кнопка со списком уже загруженных картинок.
fn library_button(app: &mut App, ui: &mut Ui, channel: Channel) {
    let p = Palette::of(ui);
    let response = theme::icon_button(ui, Icon::Layers, 26.0, t("Pick from loaded images"));
    let ctx = ui.ctx().clone();
    w::menu(&response, 300.0, |ui| {
        ui.set_min_width(280.0);
        if app.library.is_empty() {
            ui.add_space(4.0);
            ui.label(egui::RichText::new(t("Nothing loaded yet")).color(p.weak));
            ui.add_space(4.0);
        }
        let current = app.slots[channel.index()].as_ref().map(|s| s.id);
        let mut chosen = None;
        egui::ScrollArea::vertical().max_height(360.0).show(ui, |ui| {
            for src in app.library.clone() {
                let texture = app.thumb(&ctx, &src);
                let (rect, r) = ui.allocate_exact_size(egui::vec2(ui.available_width(), 40.0), Sense::click());
                if current == Some(src.id) {
                    ui.painter().rect_filled(rect, radius::CONTROL, p.soft(p.accent));
                } else if r.hovered() {
                    ui.painter().rect_filled(rect, radius::CONTROL, p.hover);
                }
                let tr = Rect::from_min_size(rect.min + egui::vec2(4.0, 4.0), Vec2::splat(32.0));
                ui.painter().image(texture, tr, Rect::from_min_max(Pos2::ZERO, Pos2::new(1.0, 1.0)), Color32::WHITE);
                ui.painter().text(
                    egui::pos2(tr.max.x + 8.0, rect.min.y + 12.0),
                    egui::Align2::LEFT_CENTER,
                    &src.name,
                    egui::FontId::proportional(13.0),
                    p.text,
                );
                ui.painter().text(
                    egui::pos2(tr.max.x + 8.0, rect.min.y + 28.0),
                    egui::Align2::LEFT_CENTER,
                    format!("{} × {} · {}", src.width, src.height, src.format),
                    egui::FontId::proportional(11.5),
                    p.weak,
                );
                if r.on_hover_cursor(egui::CursorIcon::PointingHand).clicked() {
                    chosen = Some(src.clone());
                }
            }
        });
        w::menu_separator(ui);
        if w::menu_item(ui, Some(Icon::Image), t("Browse…"), None).clicked() {
            app.pick_images(LoadTarget::Slot(channel));
        }
        if let Some(src) = chosen {
            if channel == Channel::A {
                app.work.alpha = true;
            }
            app.slots[channel.index()] = Some(src);
            ui.close();
        }
    });
}

fn hint(app: &App, ui: &mut Ui) {
    let p = Palette::of(ui);
    if app.slots.iter().any(Option::is_some) {
        return;
    }
    ui.add_space(4.0);
    ui.vertical_centered(|ui| {
        ui.label(
            egui::RichText::new(t(
                "Drop a folder or several maps onto the window — Tetrachrome sorts them into channels by name.",
            ))
            .color(p.weak),
        );
        ui.label(
            egui::RichText::new(t(
                "Switching presets keeps your maps: Metallic follows Metallic, Roughness turns into inverted Smoothness.",
            ))
            .size(12.0)
            .color(p.weak),
        );
    });
}

fn fit(size: Vec2, bounds: Vec2) -> Vec2 {
    let scale = (bounds.x / size.x).min(bounds.y / size.y);
    size * scale
}

fn dashed_rect(painter: &egui::Painter, rect: Rect, stroke: Stroke) {
    let points = [rect.left_top(), rect.right_top(), rect.right_bottom(), rect.left_bottom(), rect.left_top()];
    painter.extend(Shape::dashed_line(&points, stroke, 5.0, 4.0));
}
