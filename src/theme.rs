//! Оформление: палитра, шрифты, стиль egui и свои виджеты.
//!
//! Тема одна — тёмная: текстуры смотрят в тёмных редакторах, и светлый фон
//! вокруг предпросмотра искажает восприятие значений.

use std::sync::Arc;

use eframe::egui::{
    self, Color32, CornerRadius, FontData, FontDefinitions, FontFamily, FontId, Margin, Pos2, Rect, Response,
    Sense, Shadow, Stroke, TextStyle, Ui, Vec2,
};

use crate::model::{Channel, Engine};

pub const SEMIBOLD: &str = "semibold";

#[derive(Clone, Copy)]
pub struct Palette {
    /// Фон центральной области.
    pub bg: Color32,
    /// Боковые панели и верхняя полоса.
    pub panel: Color32,
    pub card: Color32,
    pub card_hover: Color32,
    pub field: Color32,
    pub hover: Color32,
    pub border: Color32,
    pub text: Color32,
    pub weak: Color32,
    pub faint: Color32,
    pub accent: Color32,
    pub accent_text: Color32,
    pub good: Color32,
    pub warn: Color32,
    pub danger: Color32,
}

impl Palette {
    pub const DARK: Palette = Palette {
        bg: Color32::from_rgb(0x11, 0x13, 0x18),
        panel: Color32::from_rgb(0x17, 0x1A, 0x20),
        card: Color32::from_rgb(0x1D, 0x21, 0x28),
        card_hover: Color32::from_rgb(0x22, 0x26, 0x2F),
        field: Color32::from_rgb(0x26, 0x2B, 0x34),
        hover: Color32::from_rgb(0x2C, 0x32, 0x3D),
        border: Color32::from_rgb(0x2A, 0x2F, 0x39),
        text: Color32::from_rgb(0xE9, 0xEC, 0xF2),
        weak: Color32::from_rgb(0x96, 0x9D, 0xAB),
        faint: Color32::from_rgb(0x6A, 0x71, 0x7E),
        accent: Color32::from_rgb(0x74, 0x66, 0xF2),
        accent_text: Color32::from_rgb(0xA9, 0xA0, 0xFF),
        good: Color32::from_rgb(0x46, 0xD1, 0x8F),
        warn: Color32::from_rgb(0xF2, 0xB5, 0x4A),
        danger: Color32::from_rgb(0xFF, 0x6B, 0x6E),
    };
}

pub fn channel_color(channel: Channel) -> Color32 {
    match channel {
        Channel::R => Color32::from_rgb(0xFF, 0x5F, 0x6D),
        Channel::G => Color32::from_rgb(0x45, 0xD4, 0x8A),
        Channel::B => Color32::from_rgb(0x4F, 0x9D, 0xFF),
        Channel::A => Color32::from_rgb(0xC8, 0xCD, 0xD8),
    }
}

pub fn engine_color(engine: Engine) -> Color32 {
    match engine {
        Engine::Unity => Color32::from_rgb(0x5E, 0xB5, 0xF7),
        Engine::Unreal => Color32::from_rgb(0xE8, 0x8E, 0x5A),
        Engine::Godot => Color32::from_rgb(0x6F, 0xD1, 0xB8),
        Engine::Other => Color32::from_rgb(0xB0, 0x9C, 0xF5),
    }
}

pub fn install(ctx: &egui::Context) {
    ctx.set_fonts(fonts());
    ctx.set_theme(egui::ThemePreference::Dark);
    ctx.style_mut_of(egui::Theme::Dark, apply);
}

fn fonts() -> FontDefinitions {
    let mut fonts = FontDefinitions::default();
    let fallback = fonts.families.get(&FontFamily::Proportional).cloned().unwrap_or_default();
    let regular = load_font(&mut fonts, "ui-regular", &[r"C:\Windows\Fonts\segoeui.ttf"]);
    let semibold = load_font(
        &mut fonts,
        "ui-semibold",
        &[r"C:\Windows\Fonts\seguisb.ttf", r"C:\Windows\Fonts\segoeuib.ttf"],
    );
    let mono = load_font(&mut fonts, "ui-mono", &[r"C:\Windows\Fonts\CascadiaMono.ttf", r"C:\Windows\Fonts\consola.ttf"]);

    let mut proportional = fallback.clone();
    if let Some(name) = &regular {
        proportional.insert(0, name.clone());
    }
    let mut bold = proportional.clone();
    if let Some(name) = &semibold {
        bold.insert(0, name.clone());
    }
    if let Some(name) = &mono {
        fonts.families.entry(FontFamily::Monospace).or_default().insert(0, name.clone());
    }
    fonts.families.insert(FontFamily::Proportional, proportional);
    fonts.families.insert(FontFamily::Name(SEMIBOLD.into()), bold);
    fonts
}

fn load_font(fonts: &mut FontDefinitions, name: &str, paths: &[&str]) -> Option<String> {
    let bytes = paths.iter().find_map(|path| std::fs::read(path).ok())?;
    fonts.font_data.insert(name.to_owned(), Arc::new(FontData::from_owned(bytes)));
    Some(name.to_owned())
}

fn apply(style: &mut egui::Style) {
    let p = Palette::DARK;
    let mut v = egui::Visuals::dark();
    v.panel_fill = p.panel;
    v.window_fill = p.panel;
    v.faint_bg_color = p.card;
    v.extreme_bg_color = p.field;
    v.text_edit_bg_color = Some(p.field);
    v.code_bg_color = p.field;
    v.hyperlink_color = p.accent_text;
    v.error_fg_color = p.danger;
    v.warn_fg_color = p.warn;
    v.selection.bg_fill = p.accent.gamma_multiply(0.55);
    v.selection.stroke = Stroke::new(1.0, p.text);
    v.window_corner_radius = CornerRadius::same(12);
    v.menu_corner_radius = CornerRadius::same(10);
    v.window_stroke = Stroke::new(1.0, p.border);
    v.window_shadow = Shadow {
        offset: [0, 12],
        blur: 36,
        spread: 0,
        color: Color32::from_black_alpha(140),
    };
    v.popup_shadow = Shadow {
        offset: [0, 6],
        blur: 18,
        spread: 0,
        color: Color32::from_black_alpha(120),
    };
    v.striped = true;

    let radius = CornerRadius::same(7);
    let w = &mut v.widgets;
    w.noninteractive.bg_stroke = Stroke::new(1.0, p.border);
    w.noninteractive.fg_stroke = Stroke::new(1.0, p.text);
    w.noninteractive.corner_radius = radius;
    for (state, fill) in [
        (&mut w.inactive, p.field),
        (&mut w.hovered, p.hover),
        (&mut w.active, p.accent.gamma_multiply(0.6)),
        (&mut w.open, p.hover),
    ] {
        state.bg_fill = fill;
        state.weak_bg_fill = fill;
        state.bg_stroke = Stroke::NONE;
        state.corner_radius = radius;
        state.expansion = 0.0;
        state.fg_stroke = Stroke::new(1.5, p.text);
    }
    w.inactive.fg_stroke = Stroke::new(1.5, p.text.gamma_multiply(0.9));
    w.hovered.bg_stroke = Stroke::new(1.0, p.accent.gamma_multiply(0.7));
    style.visuals = v;

    let s = &mut style.spacing;
    s.item_spacing = egui::vec2(8.0, 6.0);
    s.button_padding = egui::vec2(10.0, 4.0);
    s.interact_size.y = 26.0;
    s.window_margin = Margin::same(18);
    s.menu_margin = Margin::same(6);
    s.icon_width = 16.0;
    s.icon_width_inner = 9.0;
    s.combo_width = 120.0;
    s.scroll = egui::style::ScrollStyle::floating();

    style.text_styles = [
        (TextStyle::Small, FontId::proportional(11.5)),
        (TextStyle::Body, FontId::proportional(13.5)),
        (TextStyle::Button, FontId::proportional(13.5)),
        (TextStyle::Heading, FontId::new(17.0, FontFamily::Name(SEMIBOLD.into()))),
        (TextStyle::Monospace, FontId::monospace(12.5)),
    ]
    .into();
}

pub fn semibold(size: f32) -> FontId {
    FontId::new(size, FontFamily::Name(SEMIBOLD.into()))
}

/// Рамка фокуса для самодельных кнопок: без неё интерфейс не пройти клавишей Tab.
pub fn focus_ring(ui: &Ui, rect: Rect, response: &Response, radius: u8) {
    if response.has_focus() {
        ui.painter().rect_stroke(
            rect.expand(2.0),
            radius,
            Stroke::new(2.0, Palette::DARK.accent_text),
            egui::StrokeKind::Outside,
        );
    }
}

/// Имя для кнопки без подписи — для экранного диктора.
pub fn name_button(response: &Response, name: &str) {
    let enabled = response.enabled();
    response.widget_info(|| egui::WidgetInfo::labeled(egui::WidgetType::Button, enabled, name));
}

/// Мелкий заголовок раздела прописными: «PRESETS».
pub fn section_label(ui: &mut Ui, text: &str) {
    ui.label(
        egui::RichText::new(text.to_uppercase())
            .font(semibold(11.0))
            .color(Palette::DARK.faint)
            .extra_letter_spacing(0.8),
    );
}

/// Второстепенный текст.
pub fn weak(text: impl Into<String>) -> egui::RichText {
    egui::RichText::new(text.into()).color(Palette::DARK.weak)
}

/// Главная кнопка: залита акцентом. `icon` рисуется слева от текста.
pub fn primary_button(
    ui: &mut Ui,
    icon: Option<crate::icons::Icon>,
    text: &str,
    size: Vec2,
    enabled: bool,
) -> Response {
    let p = Palette::DARK;
    let (rect, response) = ui.allocate_exact_size(size, if enabled { Sense::click() } else { Sense::hover() });
    let fill = if !enabled {
        p.field
    } else if response.is_pointer_button_down_on() {
        p.accent.gamma_multiply(0.85)
    } else if response.hovered() {
        p.accent.lerp_to_gamma(Color32::WHITE, 0.1)
    } else {
        p.accent
    };
    let painter = ui.painter();
    painter.rect_filled(rect, 9, fill);
    if enabled {
        // Лёгкий блик сверху — кнопка читается как объёмная.
        let top = Rect::from_min_max(rect.min, Pos2::new(rect.max.x, rect.center().y));
        painter.rect_filled(
            top,
            CornerRadius { nw: 9, ne: 9, sw: 0, se: 0 },
            Color32::from_white_alpha(10),
        );
    }
    let color = if enabled { Color32::WHITE } else { p.faint };
    let galley = painter.layout_no_wrap(text.to_owned(), semibold(14.0), color);
    let icon_w = if icon.is_some() { 22.0 } else { 0.0 };
    let total = galley.size().x + icon_w;
    let mut x = rect.center().x - total / 2.0;
    if let Some(icon) = icon {
        let r = Rect::from_center_size(Pos2::new(x + 8.0, rect.center().y), Vec2::splat(16.0));
        crate::icons::paint(painter, r, icon, color);
        x += icon_w;
    }
    painter.galley(Pos2::new(x, rect.center().y - galley.size().y / 2.0), galley, color);
    focus_ring(ui, rect, &response, 9);
    name_button(&response, text);
    if enabled { response.on_hover_cursor(egui::CursorIcon::PointingHand) } else { response }
}

/// Кнопка второго плана: тихая подложка, значок и текст.
pub fn soft_button(ui: &mut Ui, icon: Option<crate::icons::Icon>, text: &str) -> Response {
    soft_button_colored(ui, icon, text, Palette::DARK.text)
}

pub fn soft_button_colored(ui: &mut Ui, icon: Option<crate::icons::Icon>, text: &str, color: Color32) -> Response {
    let p = Palette::DARK;
    let enabled = ui.is_enabled();
    let font = FontId::proportional(13.0);
    let galley = ui.painter().layout_no_wrap(text.to_owned(), font, color);
    let icon_w = if icon.is_some() { 20.0 } else { 0.0 };
    let pad = if text.is_empty() { 6.0 } else { 10.0 };
    let size = egui::vec2(galley.size().x + icon_w + pad * 2.0, 28.0);
    let (rect, response) = ui.allocate_exact_size(size, Sense::click());
    let fill = if response.is_pointer_button_down_on() {
        p.hover.lerp_to_gamma(p.accent, 0.25)
    } else if response.hovered() {
        p.hover
    } else {
        p.field
    };
    let painter = ui.painter();
    painter.rect_filled(rect, 7, fill);
    let tint = if enabled { color } else { color.gamma_multiply(0.4) };
    let mut x = rect.min.x + pad;
    if let Some(icon) = icon {
        crate::icons::paint(
            painter,
            Rect::from_center_size(Pos2::new(x + 8.0, rect.center().y), Vec2::splat(16.0)),
            icon,
            tint,
        );
        x += icon_w;
    }
    painter.galley(Pos2::new(x, rect.center().y - galley.size().y / 2.0), galley, tint);
    focus_ring(ui, rect, &response, 7);
    name_button(&response, text);
    if enabled { response.on_hover_cursor(egui::CursorIcon::PointingHand) } else { response }
}

/// Сегментированный переключатель: ряд взаимоисключающих вариантов.
/// `colors` — цвет подсветки выбранного варианта; по умолчанию акцент.
pub fn segmented<T: PartialEq + Copy>(
    ui: &mut Ui,
    value: &mut T,
    options: &[(T, &str)],
    colors: Option<&[Color32]>,
    min_item: f32,
) -> Response {
    let p = Palette::DARK;
    let font = semibold(12.5);
    let pad = 10.0;
    let widths: Vec<f32> = options
        .iter()
        .map(|(_, t)| (ui.painter().layout_no_wrap((*t).to_owned(), font.clone(), p.text).size().x + pad * 2.0).max(min_item))
        .collect();
    let height = 26.0;
    let total: f32 = widths.iter().sum::<f32>() + 4.0;
    let (rect, mut response) = ui.allocate_exact_size(egui::vec2(total, height), Sense::hover());
    let painter = ui.painter().clone();
    painter.rect_filled(rect, 8, p.field);
    let mut x = rect.min.x + 2.0;
    let enabled = ui.is_enabled();
    for (i, ((option, text), w)) in options.iter().zip(&widths).enumerate() {
        let item = Rect::from_min_size(Pos2::new(x, rect.min.y + 2.0), egui::vec2(*w, height - 4.0));
        x += w;
        let id = response.id.with(i);
        let r = ui.interact(item, id, Sense::click());
        let selected = *value == *option;
        let tint = colors.and_then(|c| c.get(i).copied()).unwrap_or(p.accent);
        if selected {
            painter.rect_filled(item, 6, tint.gamma_multiply(if colors.is_some() { 0.28 } else { 0.9 }));
            if colors.is_some() {
                painter.rect_stroke(item, 6, Stroke::new(1.0, tint.gamma_multiply(0.7)), egui::StrokeKind::Inside);
            }
        } else if r.hovered() && enabled {
            painter.rect_filled(item, 6, p.hover);
        }
        let color = match (selected, colors.is_some()) {
            (true, true) => tint.lerp_to_gamma(Color32::WHITE, 0.35),
            (true, false) => Color32::WHITE,
            _ if r.hovered() && enabled => p.text,
            _ => p.weak,
        };
        let color = if enabled { color } else { color.gamma_multiply(0.45) };
        painter.text(item.center(), egui::Align2::CENTER_CENTER, *text, font.clone(), color);
        focus_ring(ui, item, &r, 6);
        if r.clicked() && !selected {
            *value = *option;
            response.mark_changed();
        }
        if enabled {
            r.on_hover_cursor(egui::CursorIcon::PointingHand);
        }
    }
    response
}

/// Переключатель-«таблетка» с подписью справа.
pub fn toggle(ui: &mut Ui, on: &mut bool, label: &str, hint: &str) -> Response {
    let p = Palette::DARK;
    let font = FontId::proportional(13.0);
    let galley = ui.painter().layout_no_wrap(label.to_owned(), font, p.text);
    let track = egui::vec2(30.0, 17.0);
    let size = egui::vec2(track.x + 8.0 + galley.size().x, 24.0);
    let (rect, mut response) = ui.allocate_exact_size(size, Sense::click());
    if response.clicked() {
        *on = !*on;
        response.mark_changed();
    }
    let t = ui.ctx().animate_bool_responsive(response.id, *on);
    let track_rect = Rect::from_min_size(Pos2::new(rect.min.x, rect.center().y - track.y / 2.0), track);
    let enabled = ui.is_enabled();
    let fill = Color32::from_rgb(0x3A, 0x40, 0x4C).lerp_to_gamma(p.accent, t);
    let painter = ui.painter();
    painter.rect_filled(track_rect, 9, if enabled { fill } else { fill.gamma_multiply(0.5) });
    if response.hovered() && enabled {
        painter.rect_stroke(track_rect, 9, Stroke::new(1.0, p.accent.gamma_multiply(0.7)), egui::StrokeKind::Outside);
    }
    let knob_x = egui::lerp(track_rect.left() + 8.5..=track_rect.right() - 8.5, t);
    let knob = Color32::from_gray(0xB8).lerp_to_gamma(Color32::WHITE, t);
    painter.circle_filled(Pos2::new(knob_x, track_rect.center().y), 6.5, knob.gamma_multiply(if enabled { 1.0 } else { 0.5 }));
    let text_color = if enabled { p.text } else { p.faint };
    painter.galley(
        Pos2::new(track_rect.right() + 8.0, rect.center().y - galley.size().y / 2.0),
        galley,
        text_color,
    );
    focus_ring(ui, track_rect, &response, 9);
    let enabled_now = response.enabled();
    let checked = *on;
    response.widget_info(|| egui::WidgetInfo::selected(egui::WidgetType::Checkbox, enabled_now, checked, label));
    let response = if enabled { response.on_hover_cursor(egui::CursorIcon::PointingHand) } else { response };
    if hint.is_empty() { response } else { response.on_hover_text(hint) }
}

/// Плашка с текстом: метка движка, «Modified».
pub fn chip(ui: &mut Ui, text: &str, color: Color32) -> Response {
    let font = semibold(11.0);
    let galley = ui.painter().layout_no_wrap(text.to_owned(), font, color);
    let size = galley.size() + egui::vec2(12.0, 5.0);
    let (rect, response) = ui.allocate_exact_size(size, Sense::hover());
    ui.painter().rect_filled(rect, 5, color.gamma_multiply(0.16));
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
            let r = Rect::from_min_size(min, Vec2::splat(cell)).intersect(rect);
            mesh.add_colored_rect(r, light);
        }
    }
    painter.add(egui::Shape::mesh(mesh));
}

/// Карточка: скруглённая подложка с отступами.
pub fn card_frame() -> egui::Frame {
    egui::Frame::new()
        .fill(Palette::DARK.card)
        .corner_radius(12)
        .inner_margin(Margin::same(14))
        .stroke(Stroke::new(1.0, Palette::DARK.border))
}
