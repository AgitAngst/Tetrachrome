//! Значки, нарисованные линиями: одинаково чёткие при любом масштабе и не
//! зависят от того, какие глифы есть в шрифте.

use std::f32::consts::PI;

use eframe::egui::{self, Color32, Painter, Pos2, Rect, Shape, Stroke};

use crate::theme::{self, Palette};

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum Icon {
    Plus,
    Close,
    Trash,
    Copy,
    ArrowUp,
    ArrowDown,
    Folder,
    Export,
    Lock,
    Undo,
    Redo,
    Image,
    Warning,
    Check,
    Save,
    Refresh,
    Fit,
    Minus,
    Layers,
    Info,
}

/// Нарисовать значок в квадрате `rect`. Рисунок задан в сетке 24×24.
pub fn paint(painter: &Painter, rect: Rect, icon: Icon, color: Color32) {
    let scale = rect.width().min(rect.height()) / 24.0;
    let origin = rect.center() - egui::vec2(12.0, 12.0) * scale;
    let at = |x: f32, y: f32| origin + egui::vec2(x, y) * scale;
    let stroke = Stroke::new((1.7 * scale).max(1.1), color);
    let line = |points: Vec<Pos2>| {
        painter.add(Shape::line(points, stroke));
    };
    let rect_at = |x0: f32, y0: f32, x1: f32, y1: f32, r: f32| {
        painter.rect_stroke(
            Rect::from_min_max(at(x0, y0), at(x1, y1)),
            r * scale,
            stroke,
            egui::StrokeKind::Middle,
        );
    };

    match icon {
        Icon::Plus => {
            line(vec![at(12.0, 5.0), at(12.0, 19.0)]);
            line(vec![at(5.0, 12.0), at(19.0, 12.0)]);
        }
        Icon::Minus => line(vec![at(5.0, 12.0), at(19.0, 12.0)]),
        Icon::Close => {
            line(vec![at(6.5, 6.5), at(17.5, 17.5)]);
            line(vec![at(17.5, 6.5), at(6.5, 17.5)]);
        }
        Icon::Trash => {
            line(vec![at(4.5, 7.0), at(19.5, 7.0)]);
            line(vec![at(9.5, 7.0), at(10.0, 4.5), at(14.0, 4.5), at(14.5, 7.0)]);
            line(vec![at(6.5, 7.0), at(7.5, 19.5), at(16.5, 19.5), at(17.5, 7.0)]);
            line(vec![at(10.5, 10.5), at(10.5, 16.5)]);
            line(vec![at(13.5, 10.5), at(13.5, 16.5)]);
        }
        Icon::Copy => {
            rect_at(8.5, 8.5, 19.0, 19.0, 2.0);
            line(vec![at(5.0, 15.0), at(5.0, 5.0), at(15.0, 5.0)]);
        }
        Icon::ArrowUp => line(vec![at(6.0, 14.5), at(12.0, 8.5), at(18.0, 14.5)]),
        Icon::ArrowDown => line(vec![at(6.0, 9.5), at(12.0, 15.5), at(18.0, 9.5)]),
        Icon::Folder => {
            line(vec![
                at(3.5, 7.0),
                at(3.5, 18.5),
                at(20.5, 18.5),
                at(20.5, 8.5),
                at(11.5, 8.5),
                at(9.5, 5.5),
                at(3.5, 5.5),
                at(3.5, 7.0),
            ]);
        }
        Icon::Export => {
            line(vec![at(12.0, 4.0), at(12.0, 15.0)]);
            line(vec![at(7.5, 10.5), at(12.0, 15.0), at(16.5, 10.5)]);
            line(vec![at(4.5, 15.5), at(4.5, 19.5), at(19.5, 19.5), at(19.5, 15.5)]);
        }
        Icon::Lock => {
            let body = Rect::from_min_max(at(6.0, 11.0), at(18.0, 20.0));
            painter.rect_filled(body, 2.0 * scale, color);
            line(arc(12.0, 11.0, 3.8, PI, 2.0 * PI, 10).into_iter().map(|(x, y)| at(x, y)).collect());
        }
        Icon::Undo => {
            line(vec![at(8.5, 5.5), at(4.5, 9.5), at(8.5, 13.5)]);
            let mut points = vec![at(4.5, 9.5), at(14.0, 9.5)];
            points.extend(arc(14.0, 14.0, 4.5, -PI / 2.0, PI / 2.0, 10).into_iter().map(|(x, y)| at(x, y)));
            points.push(at(9.0, 18.5));
            line(points);
        }
        Icon::Redo => {
            line(vec![at(15.5, 5.5), at(19.5, 9.5), at(15.5, 13.5)]);
            let mut points = vec![at(19.5, 9.5), at(10.0, 9.5)];
            points.extend(
                arc(10.0, 14.0, 4.5, -PI / 2.0, -1.5 * PI, 10)
                    .into_iter()
                    .map(|(x, y)| at(x, y)),
            );
            points.push(at(15.0, 18.5));
            line(points);
        }
        Icon::Image => {
            rect_at(3.5, 5.0, 20.5, 19.0, 2.0);
            painter.circle_filled(at(8.5, 9.5), 1.6 * scale, color);
            line(vec![at(4.0, 17.0), at(9.5, 12.0), at(13.0, 15.0), at(15.5, 12.5), at(20.0, 17.0)]);
        }
        Icon::Warning => {
            line(vec![at(12.0, 3.8), at(21.0, 19.5), at(3.0, 19.5), at(12.0, 3.8)]);
            line(vec![at(12.0, 9.5), at(12.0, 14.0)]);
            painter.circle_filled(at(12.0, 16.8), 1.1 * scale, color);
        }
        Icon::Info => {
            painter.circle_stroke(at(12.0, 12.0), 8.5 * scale, stroke);
            line(vec![at(12.0, 11.0), at(12.0, 16.5)]);
            painter.circle_filled(at(12.0, 7.8), 1.1 * scale, color);
        }
        Icon::Check => line(vec![at(5.0, 12.5), at(10.0, 17.5), at(19.5, 7.0)]),
        Icon::Save => {
            line(vec![
                at(4.5, 4.5),
                at(16.5, 4.5),
                at(19.5, 7.5),
                at(19.5, 19.5),
                at(4.5, 19.5),
                at(4.5, 4.5),
            ]);
            rect_at(8.0, 4.5, 15.0, 9.0, 0.5);
            rect_at(7.5, 13.0, 16.5, 19.5, 0.5);
        }
        Icon::Refresh => {
            line(
                arc(12.0, 12.0, 7.0, -0.35 * PI, 1.35 * PI, 18)
                    .into_iter()
                    .map(|(x, y)| at(x, y))
                    .collect(),
            );
            line(vec![at(15.5, 3.8), at(17.8, 6.2), at(14.6, 7.6)]);
        }
        Icon::Fit => {
            line(vec![at(4.0, 9.0), at(4.0, 4.0), at(9.0, 4.0)]);
            line(vec![at(15.0, 4.0), at(20.0, 4.0), at(20.0, 9.0)]);
            line(vec![at(20.0, 15.0), at(20.0, 20.0), at(15.0, 20.0)]);
            line(vec![at(9.0, 20.0), at(4.0, 20.0), at(4.0, 15.0)]);
        }
        Icon::Layers => {
            line(vec![at(12.0, 4.0), at(20.5, 8.5), at(12.0, 13.0), at(3.5, 8.5), at(12.0, 4.0)]);
            line(vec![at(3.5, 12.5), at(12.0, 17.0), at(20.5, 12.5)]);
            line(vec![at(3.5, 16.0), at(12.0, 20.5), at(20.5, 16.0)]);
        }
    }
}

fn arc(cx: f32, cy: f32, r: f32, from: f32, to: f32, steps: usize) -> Vec<(f32, f32)> {
    (0..=steps)
        .map(|i| {
            let a = from + (to - from) * i as f32 / steps as f32;
            (cx + r * a.cos(), cy + r * a.sin())
        })
        .collect()
}

/// Квадратная кнопка-значок: без рамки, подсвечивается под курсором.
pub fn button(ui: &mut egui::Ui, icon: Icon, size: f32, hint: &str) -> egui::Response {
    let (rect, response) = ui.allocate_exact_size(egui::vec2(size, size), egui::Sense::click());
    decorate(ui, rect, response, icon, hint, None)
}

/// То же, но значок своего цвета: например, красная корзина.
pub fn button_colored(ui: &mut egui::Ui, icon: Icon, size: f32, hint: &str, color: Color32) -> egui::Response {
    let (rect, response) = ui.allocate_exact_size(egui::vec2(size, size), egui::Sense::click());
    decorate(ui, rect, response, icon, hint, Some(color))
}

fn decorate(
    ui: &mut egui::Ui,
    rect: Rect,
    response: egui::Response,
    icon: Icon,
    hint: &str,
    color: Option<Color32>,
) -> egui::Response {
    let p = Palette::DARK;
    let enabled = response.enabled();
    let hovered = enabled && (response.hovered() || response.is_pointer_button_down_on());
    if hovered {
        ui.painter().rect_filled(rect, 6.0, p.hover);
    }
    let base = color.unwrap_or(if hovered { p.text } else { p.weak });
    let tint = if enabled { base } else { base.gamma_multiply(0.35) };
    paint(ui.painter(), Rect::from_center_size(rect.center(), rect.size() * 0.62), icon, tint);
    theme::focus_ring(ui, rect, &response, 6);
    theme::name_button(&response, hint);
    let response = if enabled {
        response.on_hover_cursor(egui::CursorIcon::PointingHand)
    } else {
        response
    };
    if hint.is_empty() { response } else { response.on_hover_text(hint) }
}
