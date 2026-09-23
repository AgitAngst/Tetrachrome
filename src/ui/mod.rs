//! Отрисовка. Каждая часть окна — в своём файле.

mod batch_view;
mod dialogs;
mod output;
mod presets_panel;
mod slots;
mod topbar;

use eframe::egui::{self, Color32, Margin, Rect, Stroke, Ui, Vec2};

use crate::app::{App, Mode, ToastKind};
use crate::icons::{self, Icon};
use crate::theme::{self, Palette};

pub fn draw(app: &mut App, ui: &mut Ui) {
    let p = Palette::DARK;
    topbar::show(app, ui);
    status_bar(app, ui);

    egui::Panel::left("presets")
        .resizable(true)
        .default_size(272.0)
        .size_range(232.0..=420.0)
        .frame(egui::Frame::new().fill(p.panel).inner_margin(Margin::symmetric(14, 12)))
        .show(ui, |ui| presets_panel::show(app, ui));

    match app.mode {
        Mode::Pack => {
            egui::Panel::right("output")
                .resizable(true)
                .default_size(430.0)
                .size_range(340.0..=760.0)
                .frame(egui::Frame::new().fill(p.panel).inner_margin(Margin::symmetric(16, 12)))
                .show(ui, |ui| output::show(app, ui));
            egui::CentralPanel::no_frame()
                .frame(egui::Frame::new().fill(p.bg).inner_margin(Margin::symmetric(20, 16)))
                .show(ui, |ui| slots::show(app, ui));
        }
        Mode::Batch => {
            egui::CentralPanel::no_frame()
                .frame(egui::Frame::new().fill(p.bg).inner_margin(Margin::symmetric(20, 16)))
                .show(ui, |ui| batch_view::show(app, ui));
        }
    }

    let ctx = ui.ctx().clone();
    drop_overlay(app, &ctx);
    dialogs::show(app, &ctx);
    toasts(app, &ctx);
}

fn status_bar(app: &mut App, ui: &mut Ui) {
    let p = Palette::DARK;
    egui::Panel::bottom("status")
        .exact_size(26.0)
        .frame(egui::Frame::new().fill(p.panel).inner_margin(Margin::symmetric(14, 0)))
        .show(ui, |ui| {
            ui.horizontal_centered(|ui| {
                let small = |t: &str| egui::RichText::new(t).size(12.0).color(p.faint);
                if app.loading > 0 {
                    ui.add(egui::Spinner::new().size(12.0).color(p.accent_text));
                    ui.label(small(&format!("Loading {} image(s)…", app.loading)));
                } else if app.exporting() {
                    ui.add(egui::Spinner::new().size(12.0).color(p.accent_text));
                    ui.label(small("Exporting…"));
                } else {
                    let n = app.library.len();
                    ui.label(small(&match n {
                        0 => "Drop maps anywhere, or press Ctrl+O".to_owned(),
                        1 => "1 image loaded".to_owned(),
                        n => format!("{n} images loaded"),
                    }));
                }
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    ui.label(small("Linear · CPU"));
                    ui.label(small("·"));
                    let path = app.store.display().to_string();
                    let r = ui.add(egui::Label::new(small("Presets file")).sense(egui::Sense::click()));
                    if r.on_hover_text(format!("{path}\nClick to show in Explorer")).clicked() {
                        if app.store.exists() {
                            crate::platform::reveal(&app.store);
                        } else if let Some(dir) = app.store.parent() {
                            let _ = std::fs::create_dir_all(dir);
                            crate::platform::reveal(dir);
                        }
                    }
                });
            });
        });
}

/// Пока над окном несут файлы — подсказать, что будет, если отпустить.
fn drop_overlay(app: &App, ctx: &egui::Context) {
    let count = ctx.input(|i| i.raw.hovered_files.len());
    if count == 0 {
        return;
    }
    let p = Palette::DARK;
    let painter = ctx.layer_painter(egui::LayerId::new(egui::Order::Foreground, egui::Id::new("drop")));
    let screen = ctx.content_rect();
    if app.mode == crate::app::Mode::Pack
        && let Some(channel) = app.drop_target
        && let Some(rect) = app.card_rects[channel.index()]
    {
        let color = theme::channel_color(channel);
        painter.rect_filled(rect, 12, color.gamma_multiply(0.10));
        painter.rect_stroke(rect, 12, Stroke::new(2.0, color), egui::StrokeKind::Inside);
        let role = &app.work.slots[channel.index()].role;
        let text = if role.is_empty() {
            format!("Drop into {}", channel.letter())
        } else {
            format!("Drop into {} · {role}", channel.letter())
        };
        pill(&painter, rect.center_bottom() - egui::vec2(0.0, 26.0), &text, color);
        return;
    }
    let text = match (app.mode, count) {
        (Mode::Batch, _) => "Drop files or folders to add them to the batch".to_owned(),
        (_, 1) => "Drop on a channel card, or anywhere to auto-assign".to_owned(),
        (_, n) => format!("Drop {n} files to auto-assign by name suffix"),
    };
    painter.rect_filled(screen, 0.0, Color32::from_black_alpha(90));
    pill(&painter, screen.center_bottom() - egui::vec2(0.0, 70.0), &text, p.accent_text);
}

fn pill(painter: &egui::Painter, center: egui::Pos2, text: &str, color: Color32) {
    let p = Palette::DARK;
    let galley = painter.layout_no_wrap(text.to_owned(), theme::semibold(14.0), p.text);
    let rect = Rect::from_center_size(center, galley.size() + egui::vec2(28.0, 16.0));
    painter.rect_filled(rect, 20, p.panel);
    painter.rect_stroke(rect, 20, Stroke::new(1.5, color), egui::StrokeKind::Inside);
    painter.galley(rect.center() - galley.size() / 2.0, galley, p.text);
}

fn toasts(app: &mut App, ctx: &egui::Context) {
    if app.toasts.is_empty() {
        return;
    }
    let p = Palette::DARK;
    let screen = ctx.content_rect();
    let mut y = screen.bottom() - 40.0;
    let mut close = None;
    let mut reveal = None;
    for (i, toast) in app.toasts.iter().enumerate().rev().take(4) {
        let (icon, color) = match toast.kind {
            ToastKind::Info => (Icon::Info, p.accent_text),
            ToastKind::Success => (Icon::Check, p.good),
            ToastKind::Warning => (Icon::Warning, p.warn),
            ToastKind::Error => (Icon::Warning, p.danger),
        };
        let age = toast.born.elapsed().as_secs_f32();
        let fade = (age / 0.18).min(1.0);
        let area = egui::Area::new(egui::Id::new(("toast", i, toast.born)))
            .order(egui::Order::Tooltip)
            .pivot(egui::Align2::CENTER_BOTTOM)
            .fixed_pos(egui::pos2(screen.center().x, y + (1.0 - fade) * 12.0))
            .interactable(true);
        let response = area.show(ctx, |ui| {
            ui.set_opacity(fade);
            egui::Frame::new()
                .fill(p.card)
                .stroke(Stroke::new(1.0, p.border))
                .corner_radius(10)
                .shadow(egui::Shadow {
                    offset: [0, 6],
                    blur: 20,
                    spread: 0,
                    color: Color32::from_black_alpha(120),
                })
                .inner_margin(Margin::symmetric(12, 9))
                .show(ui, |ui| {
                    ui.set_max_width(420.0);
                    ui.horizontal(|ui| {
                        let (r, _) = ui.allocate_exact_size(Vec2::splat(18.0), egui::Sense::hover());
                        icons::paint(ui.painter(), r, icon, color);
                        ui.add(egui::Label::new(egui::RichText::new(&toast.text).color(p.text)).wrap());
                        if let Some(path) = &toast.reveal
                            && theme::soft_button(ui, Some(Icon::Folder), "Show").clicked()
                        {
                            reveal = Some(path.clone());
                        }
                        if icons::button(ui, Icon::Close, 20.0, "Dismiss").clicked() {
                            close = Some(i);
                        }
                    });
                });
        });
        y -= response.response.rect.height() + 8.0;
    }
    if let Some(path) = reveal {
        crate::platform::reveal(&path);
    }
    if let Some(i) = close {
        app.toasts.remove(i);
    }
}

/// Заголовок панели с кнопками справа.
pub fn panel_header(ui: &mut Ui, title: &str, right: impl FnOnce(&mut Ui)) {
    ui.horizontal(|ui| {
        theme::section_label(ui, title);
        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), right);
    });
}

/// Тонкая горизонтальная линия.
pub fn divider(ui: &mut Ui) {
    let p = Palette::DARK;
    ui.add_space(6.0);
    let (rect, _) = ui.allocate_exact_size(egui::vec2(ui.available_width(), 1.0), egui::Sense::hover());
    ui.painter().rect_filled(rect, 0.0, p.border);
    ui.add_space(6.0);
}
