//! Левая панель: список пресетов и настройки выбранного.

use anvil_ui::theme::radius;
use anvil_ui::widgets as w;
use anvil_ui::{Icon, Kind, Palette, semibold};
use eframe::egui::{self, Color32, Rect, Sense, Stroke, Ui, Vec2};

use crate::app::{App, Dialog};
use crate::lang::t;
use crate::model::{Channel, Engine};
use crate::theme;

use super::{divider, panel_header};

pub fn show(app: &mut App, ui: &mut Ui) {
    panel_header(ui, t("Presets"), |ui| {
        if theme::icon_button(ui, Icon::Plus, 26.0, t("New preset")).clicked() {
            app.new_preset();
        }
        if theme::icon_button(ui, Icon::Copy, 26.0, t("Duplicate selected preset")).clicked() {
            app.duplicate_preset();
        }
    });
    ui.add_space(4.0);

    let list_height = (ui.available_height() * 0.5).clamp(180.0, 470.0);
    egui::ScrollArea::vertical()
        .id_salt("preset-list")
        .max_height(list_height)
        .auto_shrink([false, true])
        .show(ui, |ui| list(app, ui));

    divider(ui);
    egui::ScrollArea::vertical()
        .id_salt("preset-settings")
        .auto_shrink([false, false])
        .show(ui, |ui| settings(app, ui));
}

fn list(app: &mut App, ui: &mut Ui) {
    let p = Palette::of(ui);
    for engine in Engine::ALL {
        let indices: Vec<usize> =
            (0..app.presets.len()).filter(|&i| app.presets[i].builtin && app.presets[i].engine == engine).collect();
        if indices.is_empty() {
            continue;
        }
        ui.add_space(4.0);
        ui.label(egui::RichText::new(engine.label()).size(12.0).color(p.weak));
        for i in indices {
            row(app, ui, i);
        }
    }
    ui.add_space(10.0);
    w::section_label(ui, t("My presets"));
    let mine: Vec<usize> = (0..app.presets.len()).filter(|&i| !app.presets[i].builtin).collect();
    if mine.is_empty() {
        ui.add_space(2.0);
        ui.label(
            egui::RichText::new(t("Duplicate a built-in preset or change one and save it to make your own."))
                .size(12.0)
                .color(p.weak),
        );
    }
    for i in mine {
        row(app, ui, i);
    }
}

fn row(app: &mut App, ui: &mut Ui, index: usize) {
    let p = Palette::of(ui);
    let preset = &app.presets[index];
    let selected = index == app.selected;
    let (rect, response) = ui.allocate_exact_size(egui::vec2(ui.available_width(), 30.0), Sense::click());
    let hovered = response.hovered() || response.context_menu_opened();
    let painter = ui.painter();
    if selected {
        painter.rect_filled(rect, radius::CONTROL, p.soft(p.accent));
        painter.rect_filled(
            Rect::from_min_size(rect.min + egui::vec2(0.0, 7.0), egui::vec2(3.0, rect.height() - 14.0)),
            2,
            p.accent,
        );
    } else if hovered {
        painter.rect_filled(rect, radius::CONTROL, p.hover);
    }
    let dot = egui::pos2(rect.min.x + 14.0, rect.center().y);
    painter.circle_filled(dot, 4.0, theme::engine_color(&p, preset.engine));

    let modified = selected && app.modified();
    // Справа: замок у встроенных, стрелки у своих — при наведении.
    let mut right = rect.max.x - 8.0;
    let show_arrows = !preset.builtin && hovered;
    if preset.builtin {
        let r = Rect::from_center_size(egui::pos2(right - 7.0, rect.center().y), Vec2::splat(13.0));
        anvil_ui::icons::paint(painter, r, Icon::Lock, p.faint);
        right -= 20.0;
    } else if show_arrows {
        right -= 48.0;
    }
    if modified {
        painter.circle_filled(egui::pos2(right - 5.0, rect.center().y), 3.5, p.warning);
        right -= 14.0;
    }
    let font = if selected { semibold(13.5) } else { egui::FontId::proportional(13.5) };
    let max_w = right - (dot.x + 12.0);
    let galley = painter.layout(preset.name.clone(), font, p.text, f32::INFINITY);
    let text_pos = egui::pos2(dot.x + 12.0, rect.center().y - galley.size().y / 2.0);
    let clip = Rect::from_min_max(text_pos, egui::pos2(text_pos.x + max_w.max(0.0), rect.max.y));
    painter.with_clip_rect(clip.intersect(ui.clip_rect())).galley(text_pos, galley, p.text);

    let name = preset.name.clone();
    let builtin = preset.builtin;
    if show_arrows {
        let up = Rect::from_center_size(egui::pos2(rect.max.x - 40.0, rect.center().y), Vec2::splat(22.0));
        let down = Rect::from_center_size(egui::pos2(rect.max.x - 16.0, rect.center().y), Vec2::splat(22.0));
        let r_up = ui.put(up, |ui: &mut Ui| theme::icon_button(ui, Icon::ChevronUp, 22.0, t("Move up")));
        let r_down = ui.put(down, |ui: &mut Ui| theme::icon_button(ui, Icon::ArrowDown, 22.0, t("Move down")));
        if r_up.clicked() {
            app.move_preset(index, true);
            return;
        }
        if r_down.clicked() {
            app.move_preset(index, false);
            return;
        }
    }

    let response = response.on_hover_cursor(egui::CursorIcon::PointingHand);
    if response.clicked() {
        app.select_preset(index);
    }
    response.context_menu(|ui| {
        ui.set_min_width(190.0);
        ui.label(egui::RichText::new(&name).color(p.weak).size(12.0));
        w::menu_separator(ui);
        if w::menu_item(ui, Some(Icon::Copy), t("Duplicate"), None).clicked() {
            if index != app.selected {
                app.select_preset(index);
            }
            if index == app.selected {
                app.duplicate_preset();
            }
        }
        if !builtin {
            if w::menu_item(ui, Some(Icon::ChevronUp), t("Move up"), None).clicked() {
                app.move_preset(index, true);
            }
            if w::menu_item(ui, Some(Icon::ArrowDown), t("Move down"), None).clicked() {
                app.move_preset(index, false);
            }
            w::menu_separator(ui);
            if w::menu_item_danger(ui, Some(Icon::Trash), t("Delete…")).clicked() {
                app.dialog = Some(Dialog::DeletePreset(index));
            }
        }
    });
}

fn settings(app: &mut App, ui: &mut Ui) {
    let p = Palette::of(ui);
    let builtin = app.current().builtin;
    panel_header(ui, t("Preset settings"), |ui| {
        if builtin {
            theme::chip(ui, t("BUILT-IN"), p.weak);
        }
    });
    ui.add_space(6.0);

    let field_width = ui.available_width();
    theme::field_label(ui, t("Name"));
    ui.add(egui::TextEdit::singleline(&mut app.work.name).desired_width(field_width).margin(egui::vec2(8.0, 5.0)));
    ui.add_space(6.0);

    theme::field_label(ui, t("Engine"));
    let colors: Vec<Color32> = Engine::ALL.iter().map(|e| theme::engine_color(&p, *e)).collect();
    let options: Vec<(Engine, &str)> = Engine::ALL.iter().map(|e| (*e, e.label())).collect();
    ui.scope(|ui| {
        theme::segmented(ui, &mut app.work.engine, &options, Some(&colors), (field_width - 4.0) / 4.0);
    });
    ui.add_space(6.0);

    theme::field_label(ui, t("Output label"));
    ui.add(
        egui::TextEdit::singleline(&mut app.work.label)
            .desired_width(field_width)
            .margin(egui::vec2(8.0, 5.0))
            .hint_text("_MaskMap"),
    )
    .on_hover_text(t("Inserted into the file name via the {label} token"));
    ui.add_space(8.0);
    w::toggle(ui, &mut app.work.alpha, t("Write alpha channel")).on_hover_text(t("Off: the output is RGB"));
    ui.add_space(10.0);

    w::section_label(ui, t("Roles & auto-assign suffixes"));
    ui.add_space(2.0);
    ui.label(
        egui::RichText::new(t("Files whose names contain a suffix land in that channel automatically."))
            .size(12.0)
            .color(p.weak),
    );
    ui.add_space(4.0);
    for channel in Channel::ALL {
        let i = channel.index();
        let enabled = channel != Channel::A || app.work.alpha;
        ui.add_enabled_ui(enabled, |ui| {
            egui::Frame::new()
                .fill(p.card)
                .stroke(Stroke::new(1.0, p.border))
                .corner_radius(radius::CARD)
                .inner_margin(egui::Margin::same(8))
                .show(ui, |ui| {
                    ui.set_width(ui.available_width());
                    ui.horizontal(|ui| {
                        channel_badge(ui, channel, 22.0);
                        ui.add(
                            egui::TextEdit::singleline(&mut app.work.slots[i].role)
                                .desired_width(ui.available_width())
                                .hint_text(t("Unused — constant fill"))
                                .margin(egui::vec2(6.0, 4.0)),
                        );
                    });
                    ui.add_space(2.0);
                    list_edit(ui, ("suffixes", i), &mut app.work.slots[i].suffixes);
                });
        });
        ui.add_space(4.0);
    }

    ui.add_space(8.0);
    ui.horizontal(|ui| {
        let modified = app.modified();
        ui.add_enabled_ui(modified, |ui| {
            let text = if builtin { t("Save as new") } else { t("Save") };
            if w::button(ui, Kind::Primary, Some(Icon::Save), text).clicked() {
                app.save_preset();
            }
            if w::button(ui, Kind::Secondary, Some(Icon::Undo), t("Revert")).clicked() {
                app.revert_preset();
            }
        });
        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
            if !builtin
                && theme::icon_button_colored(ui, Icon::Trash, 30.0, t("Delete preset"), Some(p.danger)).clicked()
            {
                app.dialog = Some(Dialog::DeletePreset(app.selected));
            }
        });
    });
    ui.add_space(12.0);
}

/// Цветной квадратик с буквой канала.
pub fn channel_badge(ui: &mut Ui, channel: Channel, size: f32) -> egui::Response {
    let p = Palette::of(ui);
    let (rect, response) = ui.allocate_exact_size(Vec2::splat(size), Sense::hover());
    paint_badge(ui.painter(), &p, rect, channel);
    response
}

pub fn paint_badge(painter: &egui::Painter, p: &Palette, rect: Rect, channel: Channel) {
    let color = theme::channel_color(p, channel);
    let corner = (rect.height() * 0.28) as u8;
    painter.rect_filled(rect, corner, p.soft(color));
    painter.rect_stroke(rect, corner, Stroke::new(1.0, color.gamma_multiply(0.55)), egui::StrokeKind::Inside);
    painter.text(rect.center(), egui::Align2::CENTER_CENTER, channel.letter(), semibold(rect.height() * 0.55), color);
}

/// Список строк одной строкой через запятую. Пока поле в фокусе, текст
/// хранится как набран — иначе запятая в конце исчезала бы на полуслове.
fn list_edit(ui: &mut Ui, salt: impl std::hash::Hash + std::fmt::Debug, values: &mut Vec<String>) {
    let p = Palette::of(ui);
    let id = ui.make_persistent_id(salt);
    let joined = values.join(", ");
    let mut text: String = ui.data(|d| d.get_temp(id)).unwrap_or_else(|| joined.clone());
    let response = ui.add(
        egui::TextEdit::singleline(&mut text)
            .id(id.with("edit"))
            .desired_width(ui.available_width())
            .hint_text("_Suffix, _Other")
            .font(egui::TextStyle::Monospace)
            .text_color(p.weak)
            .margin(egui::vec2(6.0, 4.0)),
    );
    if response.changed() {
        *values = parse_list(&text);
    } else if !response.has_focus() && parse_list(&text) != *values {
        // Пресет сменился или правку отменили — показываем актуальное.
        text = joined;
    }
    ui.data_mut(|d| d.insert_temp(id, text));
}

fn parse_list(text: &str) -> Vec<String> {
    text.split([',', ';']).map(str::trim).filter(|s| !s.is_empty()).map(str::to_owned).collect()
}
