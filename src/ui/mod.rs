//! Отрисовка. Каждая часть окна — в своём файле.

mod batch_view;
mod dialogs;
mod output;
mod presets_panel;
mod slots;
mod topbar;

use anvil_ui::chrome::{self, AboutAction, AppInfo};
use anvil_ui::theme::radius;
use anvil_ui::widgets as w;
use anvil_ui::{Icon, Kind, Palette, Tone};
use eframe::egui::{self, Color32, Margin, Rect, Stroke, Ui};

use crate::app::{App, Mode, ToastKind};
use crate::lang::{self, count, fill, t};
use crate::theme;

pub fn info() -> AppInfo {
    AppInfo {
        name: "Tetrachrome",
        icon: Icon::Tiles,
        version: env!("CARGO_PKG_VERSION"),
        tagline: t("Packs grayscale maps into the RGBA channels of one texture"),
        repository: env!("CARGO_PKG_REPOSITORY"),
    }
}

pub fn draw(app: &mut App, ui: &mut Ui) {
    let p = Palette::of(ui);
    topbar::show(app, ui);
    status_bar(app, ui);

    egui::Panel::left("presets")
        .resizable(true)
        .default_size(272.0)
        .size_range(232.0..=420.0)
        .frame(
            egui::Frame::new()
                .fill(p.surface)
                .inner_margin(Margin::symmetric(14, 12))
                .stroke(Stroke::new(1.0, p.border)),
        )
        .show(ui, |ui| presets_panel::show(app, ui));

    match app.mode {
        Mode::Pack => {
            egui::Panel::right("output")
                .resizable(true)
                .default_size(430.0)
                .size_range(340.0..=760.0)
                .frame(
                    egui::Frame::new()
                        .fill(p.surface)
                        .inner_margin(Margin::symmetric(16, 12))
                        .stroke(Stroke::new(1.0, p.border)),
                )
                .show(ui, |ui| output::show(app, ui));
            egui::CentralPanel::no_frame()
                .frame(egui::Frame::new().fill(p.bg).inner_margin(Margin::symmetric(20, 16)))
                .show(ui, |ui| {
                    update_banner(app, ui);
                    slots::show(app, ui);
                });
        }
        Mode::Batch => {
            egui::CentralPanel::no_frame()
                .frame(egui::Frame::new().fill(p.bg).inner_margin(Margin::symmetric(20, 16)))
                .show(ui, |ui| {
                    update_banner(app, ui);
                    batch_view::show(app, ui);
                });
        }
    }

    let ctx = ui.ctx().clone();
    drop_overlay(app, &ctx);
    dialogs::show(app, &ctx);
    settings(app, &ctx);
    let info = info();
    let status = anvil_update::ui::about_status(&ctx, &app.updater);
    if chrome::about(&ctx, &mut app.about_open, &info, status.as_deref()) == Some(AboutAction::CheckUpdates) {
        app.updater.check(app.settings.common.prerelease, None, true);
    }
    toasts(app, &ctx);
}

/// Баннер «Вышла новая версия» над содержимым — пока обновление есть и его не отложили.
fn update_banner(app: &mut App, ui: &mut Ui) {
    let updater = app.updater.clone();
    if anvil_update::ui::banner(ui, &updater, &mut app.settings.common) {
        app.save_settings();
    }
}

fn settings(app: &mut App, ctx: &egui::Context) {
    let mut open = app.settings_open;
    let mut changed = false;
    chrome::dialog(ctx, "settings", t("Settings"), 460.0, &mut open, |ui| {
        changed = chrome::common_settings(ui, &mut app.settings.common);
    });
    app.settings_open = open;
    if changed {
        lang::set(ctx, app.settings.common.language);
        app.save_settings();
    }
}

fn status_bar(app: &mut App, ui: &mut Ui) {
    chrome::status_bar(ui, |ui| {
        let p = Palette::of(ui);
        let small = |t: &str| egui::RichText::new(t).size(12.5).color(p.weak);
        if app.loading > 0 {
            w::spinner(ui, 14.0);
            ui.label(small(&fill(
                t("Loading {}…"),
                &[&count(app.loading, ["image", "images"], ["картинка", "картинки", "картинок"])],
            )));
        } else if app.exporting() {
            w::spinner(ui, 14.0);
            ui.label(small(t("Exporting…")));
        } else {
            let n = app.library.len();
            ui.label(small(&match n {
                0 => t("Drop maps anywhere, or press Ctrl+O").to_owned(),
                n => fill(t("{} loaded"), &[&count(n, ["image", "images"], ["картинка", "картинки", "картинок"])]),
            }));
        }
        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
            ui.label(small(t("Linear · CPU")));
            ui.label(small("·"));
            let path = app.store.display().to_string();
            let r = ui.add(egui::Label::new(small(t("Presets file"))).sense(egui::Sense::click()));
            if r.on_hover_text(format!("{path}\n{}", t("Click to show in Explorer"))).clicked() {
                reveal_store(app);
            }
        });
    });
}

/// Показать файл пресетов в Проводнике; нет файла — его папку.
pub fn reveal_store(app: &App) {
    if app.store.exists() {
        crate::platform::reveal(&app.store);
    } else if let Some(dir) = app.store.parent() {
        let _ = std::fs::create_dir_all(dir);
        crate::platform::reveal(dir);
    }
}

/// Пока над окном несут файлы — подсказать, что будет, если отпустить.
fn drop_overlay(app: &App, ctx: &egui::Context) {
    let count = ctx.input(|i| i.raw.hovered_files.len());
    if count == 0 {
        return;
    }
    let p = Palette::of_ctx(ctx);
    let painter = ctx.layer_painter(egui::LayerId::new(egui::Order::Foreground, egui::Id::new("drop")));
    let screen = ctx.content_rect();
    if app.mode == crate::app::Mode::Pack
        && let Some(channel) = app.drop_target
        && let Some(rect) = app.card_rects[channel.index()]
    {
        let color = theme::channel_color(&p, channel);
        painter.rect_filled(rect, radius::CARD, p.soft(color));
        painter.rect_stroke(rect, radius::CARD, Stroke::new(2.0, color), egui::StrokeKind::Inside);
        let role = &app.work.slots[channel.index()].role;
        let text = if role.is_empty() {
            fill(t("Drop into {}"), &[&channel.letter()])
        } else {
            fill(t("Drop into {} · {}"), &[&channel.letter(), role])
        };
        pill(&painter, &p, rect.center_bottom() - egui::vec2(0.0, 26.0), &text, color);
        return;
    }
    let text = match (app.mode, count) {
        (Mode::Batch, _) => t("Drop files or folders to add them to the batch").to_owned(),
        (_, 1) => t("Drop on a channel card, or anywhere to auto-assign").to_owned(),
        (_, n) => fill(t("Drop {} files to auto-assign by name suffix"), &[&n]),
    };
    painter.rect_filled(screen, 0.0, Color32::from_black_alpha(if p.dark { 90 } else { 50 }));
    pill(&painter, &p, screen.center_bottom() - egui::vec2(0.0, 70.0), &text, p.accent_text);
}

fn pill(painter: &egui::Painter, p: &Palette, center: egui::Pos2, text: &str, color: Color32) {
    let galley = painter.layout_no_wrap(text.to_owned(), anvil_ui::semibold(14.0), p.text);
    let rect = Rect::from_center_size(center, galley.size() + egui::vec2(28.0, 16.0));
    painter.rect_filled(rect, 20, p.card);
    painter.rect_stroke(rect, 20, Stroke::new(1.5, color), egui::StrokeKind::Inside);
    painter.galley(rect.center() - galley.size() / 2.0, galley, p.text);
}

/// Уведомления внизу посередине. Свои, а не из набора: у них бывает кнопка «Показать».
fn toasts(app: &mut App, ctx: &egui::Context) {
    if app.toasts.is_empty() {
        return;
    }
    let p = Palette::of_ctx(ctx);
    let screen = ctx.content_rect();
    let mut y = screen.bottom() - chrome::STATUS_BAR - 12.0;
    let mut close = None;
    let mut reveal = None;
    for (i, toast) in app.toasts.iter().enumerate().rev().take(4) {
        let tone = match toast.kind {
            ToastKind::Info => Tone::Accent,
            ToastKind::Success => Tone::Success,
            ToastKind::Warning => Tone::Warning,
            ToastKind::Error => Tone::Danger,
        };
        let color = tone.color(&p);
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
                .stroke(Stroke::new(1.0, p.border_strong))
                .corner_radius(radius::CARD)
                .shadow(ctx.global_style().visuals.popup_shadow)
                .inner_margin(Margin::symmetric(12, 9))
                .show(ui, |ui| {
                    ui.set_max_width(460.0);
                    ui.horizontal(|ui| {
                        theme::icon(ui, tone.icon(), 18.0, color);
                        ui.add(egui::Label::new(egui::RichText::new(&toast.text).color(p.text)).wrap());
                        if let Some(path) = &toast.reveal
                            && w::button(ui, Kind::Secondary, Some(Icon::Folder), t("Show")).clicked()
                        {
                            reveal = Some(path.clone());
                        }
                        if theme::icon_button(ui, Icon::Close, 24.0, t("Dismiss")).clicked() {
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
        ui.set_min_height(26.0);
        w::section_label(ui, title);
        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), right);
    });
}

/// Тонкая горизонтальная линия с отступами.
pub fn divider(ui: &mut Ui) {
    ui.add_space(6.0);
    w::divider(ui);
    ui.add_space(6.0);
}
