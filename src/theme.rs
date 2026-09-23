//! Своё оформление Tetrachrome поверх набора `anvil-ui`: цвета каналов и движков,
//! шахматка, сегменты с цветом варианта.
//!
//! Базовые цвета, шрифты, кнопки и диалоги — из набора: тема (светлая / тёмная) и акцент
//! там же. Здесь только то, чего у набора нет. Предпросмотр остаётся тёмным и в светлой
//! теме: светлый фон вокруг текстуры искажает восприятие значений.

use anvil_ui::theme::radius;
use anvil_ui::widgets as w;
use anvil_ui::{Accent, Icon, Palette};
use eframe::egui::{self, Color32, FontId, Pos2, Rect, Response, Sense, Stroke, Ui};

use crate::model::{Channel, Engine};

/// Бирюза Tetrachrome.
pub const ACCENT: Accent = Accent::TEAL;

/// Фон предпросмотра — нейтрально-тёмный в любой теме.
pub const PREVIEW_BG: Color32 = Color32::from_rgb(0x0C, 0x0E, 0x12);
/// Текст поверх предпросмотра.
pub const PREVIEW_TEXT: Color32 = Color32::from_rgb(0xA8, 0xAE, 0xBA);

/// Цвет канала: в тёмной теме светлее, в светлой — темнее, чтобы подписи читались.
pub fn channel_color(p: &Palette, channel: Channel) -> Color32 {
    match (channel, p.dark) {
        (Channel::R, true) => Color32::from_rgb(0xFF, 0x5F, 0x6D),
        (Channel::G, true) => Color32::from_rgb(0x45, 0xD4, 0x8A),
        (Channel::B, true) => Color32::from_rgb(0x4F, 0x9D, 0xFF),
        (Channel::A, true) => Color32::from_rgb(0xC8, 0xCD, 0xD8),
        (Channel::R, false) => Color32::from_rgb(0xB0, 0x1F, 0x31),
        (Channel::G, false) => Color32::from_rgb(0x13, 0x6B, 0x3E),
        (Channel::B, false) => Color32::from_rgb(0x1C, 0x57, 0xB8),
        (Channel::A, false) => Color32::from_rgb(0x55, 0x5B, 0x68),
    }
}

/// Яркость (Lum) — пятый вариант «откуда читать».
pub fn luminance_color(p: &Palette) -> Color32 {
    if p.dark { Color32::from_rgb(0xE6, 0xD3, 0x8C) } else { Color32::from_rgb(0x76, 0x5A, 0x00) }
}

pub fn engine_color(p: &Palette, engine: Engine) -> Color32 {
    match (engine, p.dark) {
        (Engine::Unity, true) => Color32::from_rgb(0x5E, 0xB5, 0xF7),
        (Engine::Unreal, true) => Color32::from_rgb(0xE8, 0x8E, 0x5A),
        (Engine::Godot, true) => Color32::from_rgb(0x6F, 0xD1, 0xB8),
        (Engine::Other, true) => Color32::from_rgb(0xB0, 0x9C, 0xF5),
        (Engine::Unity, false) => Color32::from_rgb(0x18, 0x5E, 0x9E),
        (Engine::Unreal, false) => Color32::from_rgb(0x96, 0x40, 0x10),
        (Engine::Godot, false) => Color32::from_rgb(0x12, 0x6A, 0x56),
        (Engine::Other, false) => Color32::from_rgb(0x62, 0x47, 0xC2),
    }
}

/// Подпись поля: мелко и приглушённо.
pub fn field_label(ui: &mut Ui, text: &str) {
    let p = Palette::of(ui);
    ui.label(egui::RichText::new(text).size(12.0).color(p.weak));
}

/// Сегментный выбор с шириной варианта не меньше `min_item`. `colors` — свой цвет у каждого
/// варианта (каналы, движки); без них выбранный вариант выглядит как в наборе.
pub fn segmented<T: PartialEq + Copy>(
    ui: &mut Ui,
    value: &mut T,
    options: &[(T, &str)],
    colors: Option<&[Color32]>,
    min_item: f32,
) -> Response {
    let p = Palette::of(ui);
    let font = FontId::proportional(13.0);
    let pad = 10.0;
    let widths: Vec<f32> = options
        .iter()
        .map(|(_, t)| {
            (ui.painter().layout_no_wrap((*t).to_owned(), font.clone(), p.text).size().x + pad * 2.0).max(min_item)
        })
        .collect();
    let height = 28.0;
    let total: f32 = widths.iter().sum::<f32>() + 4.0;
    let (rect, mut response) = ui.allocate_exact_size(egui::vec2(total, height), Sense::hover());
    let painter = ui.painter().clone();
    painter.rect(rect, radius::CONTROL + 1, p.raised, Stroke::new(1.0, p.border), egui::StrokeKind::Inside);
    let enabled = ui.is_enabled();
    let mut x = rect.min.x + 2.0;
    for (i, ((option, text), w)) in options.iter().zip(&widths).enumerate() {
        let item = Rect::from_min_size(Pos2::new(x, rect.min.y + 2.0), egui::vec2(*w, height - 4.0));
        x += w;
        let r = ui.interact(item, response.id.with(i), Sense::click());
        let selected = *value == *option;
        let tint = colors.and_then(|c| c.get(i).copied());
        let corner = radius::CONTROL - 1;
        match (selected, tint) {
            (true, Some(tint)) => {
                painter.rect(
                    item,
                    corner,
                    p.soft(tint),
                    Stroke::new(1.0, tint.gamma_multiply(0.7)),
                    egui::StrokeKind::Inside,
                );
            }
            (true, None) => {
                painter.rect(item, corner, p.card, Stroke::new(1.0, p.border_strong), egui::StrokeKind::Inside);
            }
            (false, _) if r.hovered() && enabled => {
                painter.rect_filled(item, corner, p.hover);
            }
            _ => {}
        }
        let color = match (selected, tint) {
            (true, Some(tint)) => tint,
            (true, None) => p.text,
            _ if r.hovered() && enabled => p.text,
            _ => p.weak,
        };
        let color = if enabled { color } else { color.gamma_multiply(0.45) };
        painter.text(item.center(), egui::Align2::CENTER_CENTER, *text, font.clone(), color);
        w::focus_ring(ui, item, &r, corner);
        if r.clicked() && !selected {
            *value = *option;
            response.mark_changed();
        }
        r.widget_info(|| egui::WidgetInfo::selected(egui::WidgetType::RadioButton, enabled, selected, *text));
        if enabled {
            r.on_hover_cursor(egui::CursorIcon::PointingHand);
        }
    }
    response
}

/// Квадратная кнопка-значок размера `size`: без рамки, подсвечивается под курсором.
/// В наборе такая кнопка одного размера — здесь бывают и мельче (стрелки в строке списка).
pub fn icon_button(ui: &mut Ui, icon: Icon, size: f32, hint: &str) -> Response {
    icon_button_colored(ui, icon, size, hint, None)
}

/// То же, но значок своего цвета: например, красная корзина.
pub fn icon_button_colored(ui: &mut Ui, icon: Icon, size: f32, hint: &str, color: Option<Color32>) -> Response {
    let p = Palette::of(ui);
    let (rect, response) = ui.allocate_exact_size(egui::vec2(size, size), Sense::click());
    let enabled = response.enabled();
    let hovered = enabled && (response.hovered() || response.is_pointer_button_down_on());
    if hovered {
        ui.painter().rect_filled(rect, radius::CONTROL, p.hover);
    }
    let base = color.unwrap_or(if hovered { p.text } else { p.weak });
    let tint = if enabled { base } else { base.gamma_multiply(0.35) };
    anvil_ui::icons::paint(ui.painter(), Rect::from_center_size(rect.center(), rect.size() * 0.6), icon, tint);
    w::focus_ring(ui, rect, &response, radius::CONTROL);
    response.widget_info(|| egui::WidgetInfo::labeled(egui::WidgetType::Button, enabled, hint));
    let response = if enabled { response.on_hover_cursor(egui::CursorIcon::PointingHand) } else { response };
    if hint.is_empty() { response } else { response.on_hover_text(hint) }
}

/// Значок без кнопки.
pub fn icon(ui: &mut Ui, icon: Icon, size: f32, color: Color32) {
    let (rect, _) = ui.allocate_exact_size(egui::Vec2::splat(size), Sense::hover());
    anvil_ui::icons::paint(ui.painter(), rect, icon, color);
}

/// Плашка с текстом своего цвета: метка движка, «Изменён».
pub fn chip(ui: &mut Ui, text: &str, color: Color32) -> Response {
    let p = Palette::of(ui);
    let galley = ui.painter().layout_no_wrap(text.to_owned(), anvil_ui::semibold(11.0), color);
    let size = galley.size() + egui::vec2(12.0, 5.0);
    let (rect, response) = ui.allocate_exact_size(size, Sense::hover());
    ui.painter().rect_filled(rect, 5, p.soft(color));
    ui.painter().galley(rect.center() - galley.size() / 2.0, galley, color);
    response
}

/// Шахматка под прозрачностью.
pub fn checkerboard(painter: &egui::Painter, rect: Rect, cell: f32) {
    let light = Color32::from_gray(0x4A);
    let dark = Color32::from_gray(0x36);
    painter.rect_filled(rect, 0.0, dark);
    let clip = painter.clip_rect().intersect(rect);
    if clip.width() <= 0.0 || clip.height() <= 0.0 {
        return;
    }
    // Рисуем только видимое: при сильном увеличении прямоугольник огромный.
    let start_x = ((clip.min.x - rect.min.x) / cell).floor().max(0.0) as i64;
    let start_y = ((clip.min.y - rect.min.y) / cell).floor().max(0.0) as i64;
    let end_x = ((clip.max.x - rect.min.x) / cell).ceil() as i64;
    let end_y = ((clip.max.y - rect.min.y) / cell).ceil() as i64;
    let mut mesh = egui::Mesh::default();
    for y in start_y..end_y {
        for x in start_x..end_x {
            if (x + y) % 2 == 0 {
                continue;
            }
            let min = rect.min + egui::vec2(x as f32 * cell, y as f32 * cell);
            let r = Rect::from_min_size(min, egui::Vec2::splat(cell)).intersect(rect);
            mesh.add_colored_rect(r, light);
        }
    }
    painter.add(egui::Shape::mesh(mesh));
}

#[cfg(test)]
mod tests {
    use super::*;

    fn luminance(c: Color32) -> f32 {
        let lin = |v: u8| {
            let v = v as f32 / 255.0;
            if v <= 0.04045 { v / 12.92 } else { ((v + 0.055) / 1.055).powf(2.4) }
        };
        0.2126 * lin(c.r()) + 0.7152 * lin(c.g()) + 0.0722 * lin(c.b())
    }

    fn contrast(a: Color32, b: Color32) -> f32 {
        let (x, y) = (luminance(a), luminance(b));
        (x.max(y) + 0.05) / (x.min(y) + 0.05)
    }

    /// Цвета каналов и движков — это и подписи: 4.5:1 к карточке и к мягкой подложке в обеих темах.
    #[test]
    fn channel_and_engine_colors_are_readable() {
        for dark in [true, false] {
            let p = Palette::new(dark, ACCENT);
            let mut colors: Vec<Color32> = Channel::ALL.iter().map(|c| channel_color(&p, *c)).collect();
            colors.extend(Engine::ALL.iter().map(|e| engine_color(&p, *e)));
            colors.push(luminance_color(&p));
            for color in colors {
                for bg in [p.card, p.surface, p.bg] {
                    let soft = bg.lerp_to_gamma(color, if dark { 0.16 } else { 0.12 });
                    for back in [bg, soft] {
                        let ratio = contrast(color, back);
                        assert!(ratio >= 4.5, "{color:?} на {back:?} (тёмная: {dark}) — {ratio:.2}:1");
                    }
                }
            }
        }
        assert!(contrast(PREVIEW_TEXT, PREVIEW_BG) >= 4.5);
    }
}
