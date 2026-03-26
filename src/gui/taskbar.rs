//! Taskbar renderer for Capsule OS.
//! Renders a 48-pixel panel pinned to the bottom of the screen

use crate::config::Config;
use crate::gui::state::{AppType, DesktopState};
use crate::gui::theme_bridge::{hex_to_color32, hex_to_color32_alpha};
use chrono::Local;
use egui::{Align, Color32, Context, Frame, Layout, Rounding, Stroke, Vec2};

pub const TASKBAR_HEIGHT: f32 = 48.0;

pub fn render(ctx: &Context, state: &mut DesktopState) {
    let cfg = match Config::snapshot() {
        Ok(c) => c,
        Err(_) => return,
    };

    let bg = hex_to_color32(&cfg.theme.background);
    let border = hex_to_color32(&cfg.theme.border);
    let bar_bg = lerp_darker(bg, 0.06);

    let panel_frame = Frame {
        fill: bar_bg,
        stroke: Stroke::new(1.0, border),
        inner_margin: egui::Margin::symmetric(8.0, 0.0),
        ..Default::default()
    };

    let mut launcher_button_rect = egui::Rect::NOTHING;

    egui::TopBottomPanel::bottom("taskbar")
        .exact_height(TASKBAR_HEIGHT)
        .frame(panel_frame)
        .show(ctx, |ui| {
            ui.horizontal_centered(|ui| {
                launcher_button_rect = render_launcher(ui, state, &cfg);

                ui.separator();

                render_window_list(ui, state, &cfg);

                ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                    render_system_tray(ui, &cfg);
                });
            });
        });

    if state.taskbar.launcher_open {
        render_launcher_popup(ctx, state, &cfg, launcher_button_rect);
    }
}

fn render_launcher(ui: &mut egui::Ui, state: &mut DesktopState, cfg: &Config) -> egui::Rect {
    let accent = hex_to_color32(&cfg.theme.accent);
    let fg = hex_to_color32(&cfg.theme.foreground);
    let label = format!("□ {}", cfg.system.name);

    let btn = egui::Button::new(egui::RichText::new(&label).color(fg).strong())
        .fill(if state.taskbar.launcher_open {
            hex_to_color32_alpha(&cfg.theme.accent, 80)
        } else {
            hex_to_color32_alpha(&cfg.theme.secondary, 60)
        })
        .stroke(Stroke::new(1.0, accent))
        .rounding(Rounding::same(20.0))
        .min_size(Vec2::new(120.0, 32.0));

    let resp = ui.add(btn);
    paint_hover_overlay(
        ui,
        &resp,
        hex_to_color32_alpha(&cfg.theme.accent, 28),
        Stroke::new(1.0, hex_to_color32_alpha(&cfg.theme.accent, 180)),
        20.0,
    );

    if resp.clicked() {
        state.taskbar.launcher_open = !state.taskbar.launcher_open;
    }

    resp.rect
}

fn render_launcher_popup(
    ctx: &Context,
    state: &mut DesktopState,
    cfg: &Config,
    launcher_button_rect: egui::Rect,
) {
    let bg = hex_to_color32(&cfg.theme.background);
    let border = hex_to_color32(&cfg.theme.border);
    let accent = hex_to_color32(&cfg.theme.accent);
    let fg = hex_to_color32(&cfg.theme.foreground);
    let muted = hex_to_color32(&cfg.theme.muted);

    let popup_w = 240.0_f32;
    let popup_h = 170.0_f32;
    let popup_rect = egui::Rect::from_min_size(
        egui::Pos2::new(
            launcher_button_rect.left().max(8.0),
            launcher_button_rect.min.y - popup_h - 10.0,
        ),
        Vec2::new(popup_w, popup_h),
    );

    let frame = Frame {
        fill: bg,
        stroke: Stroke::new(1.0, border),
        rounding: Rounding::same(10.0),
        inner_margin: egui::Margin::same(10.0),
        shadow: egui::epaint::Shadow {
            offset: Vec2::new(4.0, 8.0),
            blur: 18.0,
            spread: 0.0,
            color: Color32::from_black_alpha(110),
        },
        ..Default::default()
    };

    egui::Area::new(egui::Id::new("launcher_popup"))
        .fixed_pos(popup_rect.min)
        .order(egui::Order::Foreground)
        .show(ctx, |ui| {
            frame.show(ui, |ui| {
                ui.set_min_width(popup_w - 20.0);
                ui.set_max_width(popup_w - 20.0);

                ui.label(
                    egui::RichText::new(&cfg.system.name)
                        .color(accent)
                        .strong()
                        .size(16.0),
                );
                ui.label(egui::RichText::new("Applications").color(muted).size(11.0));
                ui.add_space(8.0);

                for (app_type, icon, label) in [
                    (AppType::Terminal, "□", "Terminal"),
                    (AppType::FileManager, "◧", "File Manager"),
                    (AppType::Settings, "◉", "Settings"),
                    (AppType::About, "i", "About"),
                ] {
                    let row = egui::Button::new(
                        egui::RichText::new(format!("{icon}  {label}"))
                            .color(fg)
                            .size(14.0),
                    )
                    .fill(hex_to_color32_alpha(&cfg.theme.secondary, 26))
                    .stroke(Stroke::new(
                        1.0,
                        hex_to_color32_alpha(&cfg.theme.border, 160),
                    ))
                    .rounding(Rounding::same(6.0))
                    .min_size(Vec2::new(popup_w - 20.0, 34.0));

                    let resp = ui.add(row);
                    paint_hover_overlay(
                        ui,
                        &resp,
                        hex_to_color32_alpha(&cfg.theme.accent, 24),
                        Stroke::new(1.0, hex_to_color32_alpha(&cfg.theme.accent, 150)),
                        6.0,
                    );

                    if resp.clicked() {
                        state.open_window(app_type);
                        state.taskbar.launcher_open = false;
                    }

                    ui.add_space(4.0);
                }
            });
        });

    if ctx.input(|i| i.pointer.any_pressed()) {
        if let Some(pos) = ctx.input(|i| i.pointer.interact_pos()) {
            let clicked_outside_popup = !popup_rect.contains(pos);
            let clicked_on_button = launcher_button_rect.contains(pos);

            if clicked_outside_popup && !clicked_on_button {
                state.taskbar.launcher_open = false;
            }
        }
    }
}

fn render_window_list(ui: &mut egui::Ui, state: &mut DesktopState, cfg: &Config) {
    let accent = hex_to_color32(&cfg.theme.accent);
    let muted = hex_to_color32(&cfg.theme.muted);
    let focused = state.taskbar.focused_id;

    let window_info: Vec<(usize, String, bool)> = state
        .windows
        .iter()
        .map(|w| {
            let title = format!("{} {}", w.app_type.icon(), w.app_type.title());
            let active = Some(w.id) == focused && !w.is_minimized;
            (w.id, title, active)
        })
        .collect();

    let mut to_focus = None;
    for (id, title, active) in window_info {
        let fill = if active {
            hex_to_color32_alpha(&cfg.theme.accent, 60)
        } else {
            hex_to_color32_alpha(&cfg.theme.border, 40)
        };
        let text_color = if active { accent } else { muted };

        let btn = egui::Button::new(egui::RichText::new(&title).color(text_color).size(13.0))
            .fill(fill)
            .stroke(Stroke::new(
                if active { 1.0 } else { 0.5 },
                if active { accent } else { muted },
            ))
            .rounding(Rounding::same(4.0))
            .min_size(Vec2::new(100.0, 30.0));

        if ui.add(btn).clicked() {
            to_focus = Some(id);
        }
    }

    if let Some(id) = to_focus {
        state.focus_window(id);
    }
}

fn render_system_tray(ui: &mut egui::Ui, cfg: &Config) {
    let muted = hex_to_color32(&cfg.theme.muted);
    let accent = hex_to_color32(&cfg.theme.accent);
    let fg = hex_to_color32(&cfg.theme.foreground);

    let (rect, _) = ui.allocate_exact_size(Vec2::splat(18.0), egui::Sense::hover());
    ui.painter().rect_filled(rect, Rounding::same(3.0), accent);
    ui.painter()
        .rect_stroke(rect, Rounding::same(3.0), Stroke::new(1.0, muted));

    ui.add_space(4.0);
    ui.label(egui::RichText::new(&cfg.theme.name).color(muted).size(11.0));
    ui.add_space(8.0);

    let time_str = Local::now().format("%H:%M").to_string();
    ui.label(egui::RichText::new(&time_str).color(fg).strong().size(14.0));
}

fn lerp_darker(c: Color32, amount: f32) -> Color32 {
    let dark = Color32::from_rgb(0, 0, 0);
    crate::gui::theme_bridge::lerp_color(c, dark, amount)
}

fn paint_hover_overlay(
    ui: &egui::Ui,
    response: &egui::Response,
    fill: Color32,
    stroke: Stroke,
    rounding: f32,
) {
    if response.hovered() {
        ui.painter()
            .rect(response.rect, Rounding::same(rounding), fill, stroke);
    }
}
