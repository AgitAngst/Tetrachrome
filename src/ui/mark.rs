//! Знак Tetrachrome в интерфейсе: буква T из четырёх квадратов — ряд R-G-B цветами каналов и ножка-альфа
//! с шахматкой — на плитке тёмной бирюзы. Геометрия — из `assets/icon/tetrachrome-small.svg`, цвета от
//! темы окна не зависят: это знак программы, а не деталь интерфейса. Набор рисует в «О программе» свой
//! значок из `Icon`, поэтому диалог тут собран заново из его частей.

use anvil_ui::chrome::{self, AboutAction, AppInfo};
use anvil_ui::widgets as w;
use anvil_ui::{Icon, Kind, Palette, semibold};
use eframe::egui::{self, Color32, CornerRadius, Painter, Rect, Response, RichText, Sense, Ui, Vec2, pos2};

/// Плитка — середина её градиента `#0F3532 → #041312`.
const TILE: Color32 = Color32::from_rgb(0x09, 0x24, 0x22);
/// Цвета каналов программы (тёмная тема), шахматка альфы.
const RED: Color32 = Color32::from_rgb(0xFF, 0x5F, 0x6D);
const GREEN: Color32 = Color32::from_rgb(0x45, 0xD4, 0x8A);
const BLUE: Color32 = Color32::from_rgb(0x4F, 0x9D, 0xFF);
const CHECK_LIGHT: Color32 = Color32::from_rgb(0xF1, 0xF3, 0xF8);
const CHECK_DARK: Color32 = Color32::from_rgb(0xA7, 0xAE, 0xBD);

/// Размеры в единицах SVG (плитка 1024×1024): квадрат, зазор, радиус скругления, сдвиг центровки.
const SQUARE: f32 = 272.0;
const GAP: f32 = 36.0;
const RADIUS: f32 = 32.0;
const SHIFT: (f32, f32) = (1.3, -17.4);
/// Верх ряда из трёх квадратов и левый край ряда.
const ROW_TOP: f32 = 264.0;
const ROW_LEFT: f32 = 68.0;

/// Знак в квадрате `rect`.
pub fn paint(painter: &Painter, rect: Rect) {
    let k = rect.width() / 1024.0;
    let at = |x: f32, y: f32| pos2(rect.min.x + (x + SHIFT.0) * k, rect.min.y + (y + SHIFT.1) * k);
    let corner = |r: f32| (r * k).round().min(255.0) as u8;
    let square = |x: f32, y: f32| Rect::from_min_size(at(x, y), Vec2::splat(SQUARE * k));
    painter.rect_filled(rect, corner(1024.0 * 0.26), TILE);

    let step = SQUARE + GAP;
    for (i, color) in [RED, GREEN, BLUE].into_iter().enumerate() {
        painter.rect_filled(square(ROW_LEFT + step * i as f32, ROW_TOP), corner(RADIUS), color);
    }
    // Ножка — альфа: шахматка 2×2, скруглены только внешние углы.
    let stem = square(ROW_LEFT + step, ROW_TOP + step);
    let r = corner(RADIUS);
    painter.rect_filled(stem, r, CHECK_LIGHT);
    let half = stem.width() / 2.0;
    let cell = |dx: f32, dy: f32| Rect::from_min_size(stem.min + Vec2::new(dx, dy), Vec2::splat(half));
    painter.rect_filled(cell(half, 0.0), CornerRadius { nw: 0, ne: r, sw: 0, se: 0 }, CHECK_DARK);
    painter.rect_filled(cell(0.0, half), CornerRadius { nw: 0, ne: 0, sw: r, se: 0 }, CHECK_DARK);
}

/// Знак `size` на `size` в потоке интерфейса.
pub fn app_mark(ui: &mut Ui, size: f32) -> Response {
    let (rect, response) = ui.allocate_exact_size(Vec2::splat(size), Sense::hover());
    paint(ui.painter(), rect);
    response
}

/// Знак и название в шапке (как `chrome::brand` набора, но со своим знаком).
pub fn brand(ui: &mut Ui, name: &str) {
    let p = Palette::of(ui);
    app_mark(ui, 28.0);
    ui.add_space(2.0);
    ui.label(RichText::new(name).font(semibold(16.0)).color(p.text));
}

/// «О программе» — как `chrome::about` набора, но со своим знаком. `status` — строка `anvil-update`.
pub fn about(ctx: &egui::Context, open: &mut bool, info: &AppInfo, status: Option<&str>) -> Option<AboutAction> {
    let mut action = None;
    chrome::dialog(ctx, "tetrachrome-about", anvil_ui::tr(ctx, "О программе"), 400.0, open, |ui| {
        let p = Palette::of(ui);
        ui.vertical_centered(|ui| {
            ui.add_space(8.0);
            app_mark(ui, 56.0);
            ui.add_space(10.0);
            ui.label(RichText::new(info.name).font(semibold(20.0)).color(p.text));
            ui.label(
                RichText::new(format!("{} {}", anvil_ui::tr(ctx, "Версия"), info.version)).size(13.0).color(p.weak),
            );
            ui.add_space(6.0);
            ui.label(RichText::new(info.tagline).color(p.text));
            ui.add_space(12.0);
            if w::button(ui, Kind::Secondary, Some(Icon::Refresh), anvil_ui::tr(ctx, "Проверить обновления")).clicked()
            {
                action = Some(AboutAction::CheckUpdates);
            }
            if let Some(status) = status {
                ui.add_space(4.0);
                w::note(ui, status);
            }
            ui.add_space(10.0);
            if !info.repository.is_empty() {
                ui.hyperlink_to(RichText::new(anvil_ui::tr(ctx, "Исходный код")).size(13.0), info.repository);
            }
            ui.label(RichText::new(anvil_ui::tr(ctx, "Часть семьи Anvil")).size(12.0).color(p.faint));
        });
    });
    action
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_t_stays_inside_the_tile() {
        let step = SQUARE + GAP;
        let (left, right) = (ROW_LEFT + SHIFT.0, ROW_LEFT + 2.0 * step + SQUARE + SHIFT.0);
        let (top, bottom) = (ROW_TOP + SHIFT.1, ROW_TOP + step + SQUARE + SHIFT.1);
        assert!(left > 0.0 && right < 1024.0 && top > 0.0 && bottom < 1024.0);
        // Поля справа и слева равны с точностью до сдвига.
        assert!((left - (1024.0 - right)).abs() < 3.0);
    }
}
