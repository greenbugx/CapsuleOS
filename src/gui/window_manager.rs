//! Window manager for Capsule OS.
//! Renders each open `AppWindow` as a floating egui window.

use crate::config::Config;
use crate::gui::apps;
use crate::gui::state::{AppType, DesktopState};
use crate::gui::taskbar::TASKBAR_HEIGHT;
use crate::gui::theme_bridge::hex_to_color32;
use egui::{Color32, Context, Pos2, Rect, Rounding, Stroke, Vec2};

/// Render all open windows each frame
pub fn render(ctx: &Context, state: &mut DesktopState) {
    let cfg = match Config::snapshot() {
        Ok(c) => c,
        Err(_) => return,
    };

    let mut indices: Vec<usize> = (0..state.windows.len()).collect();
    indices.sort_by_key(|&i| state.windows[i].z_order);

    let screen = ctx.screen_rect();
    let desktop_rect = Rect::from_min_max(
        screen.min,
        Pos2::new(screen.max.x, screen.max.y - TASKBAR_HEIGHT),
    );

    // We need to collect ids for actions to avoid borrowing issues
    let mut to_close: Vec<usize> = Vec::new();
    let mut to_minimize: Vec<usize> = Vec::new();
    let mut to_maximize: Vec<usize> = Vec::new();
    let mut to_focus: Vec<usize> = Vec::new();
    let mut saved_bounds: Vec<(usize, Pos2, Vec2)> = Vec::new();
    let mut restored_windows: Vec<usize> = Vec::new();

    for &idx in &indices {
        let win = &state.windows[idx];

        // Minimised windows live only on the taskbar
        if win.is_minimized {
            continue;
        }

        let win_id = win.id;
        let is_focused = state.taskbar.focused_id == Some(win_id);
        let is_max = win.is_maximized;
        let app_type = win.app_type.clone();
        let initial_pos = win.initial_pos;
        let initial_size = win.initial_size;
        let pending_restore = win.pending_restore;
        let egui_id = win.egui_id();
        let title = win.window_title();

        let fg = hex_to_color32(&cfg.theme.foreground);
        let accent = hex_to_color32(&cfg.theme.accent);
        let secondary = hex_to_color32(&cfg.theme.secondary);
        let muted = hex_to_color32(&cfg.theme.muted);
        let border = hex_to_color32(&cfg.theme.border);

        // Build egui Window
        let egui_window = egui::Window::new(&title)
            .id(egui_id)
            .collapsible(false)
            .resizable(!is_max)
            .movable(!is_max)
            .default_pos(initial_pos)
            .default_size(initial_size)
            .min_size(Vec2::new(320.0, 200.0))
            .title_bar(false)
            .frame(egui::Frame {
                fill: hex_to_color32(&cfg.theme.background),
                stroke: Stroke::new(
                    if is_focused { 1.5 } else { 1.0 },
                    if is_focused { accent } else { border },
                ),
                rounding: Rounding::same(6.0),
                inner_margin: egui::Margin::same(0.0),
                shadow: egui::epaint::Shadow {
                    offset: Vec2::new(4.0, 6.0),
                    blur: if is_focused { 16.0 } else { 8.0 },
                    spread: 0.0,
                    color: Color32::from_black_alpha(if is_focused { 120 } else { 60 }),
                },
                ..Default::default()
            });

        let egui_window = if is_max {
            egui_window
                .fixed_pos(desktop_rect.min)
                .fixed_size(desktop_rect.size())
        } else if pending_restore {
            egui_window.fixed_pos(initial_pos).fixed_size(initial_size)
        } else {
            egui_window
        };

        let response = egui_window.show(ctx, |ui| {
            // Custom title bar
            let titlebar_height = 30.0;
            let titlebar_color = secondary;
            let width = ui.max_rect().width();

            let (titlebar_rect, titlebar_resp) =
                ui.allocate_exact_size(Vec2::new(width, titlebar_height), egui::Sense::click());

            // Background
            ui.painter()
                .rect_filled(titlebar_rect, Rounding::same(4.0), titlebar_color);

            // Title text
            let title_pos = Pos2::new(titlebar_rect.min.x + 10.0, titlebar_rect.center().y - 7.0);
            ui.painter().text(
                title_pos,
                egui::Align2::LEFT_TOP,
                &title,
                egui::FontId::proportional(13.0),
                fg,
            );

            // Control buttons
            let btn_size = Vec2::new(22.0, 22.0);
            let btn_y = titlebar_rect.center().y - btn_size.y / 2.0;
            let close_x = titlebar_rect.max.x - 28.0;
            let max_x = close_x - 28.0;
            let min_x = max_x - 28.0;

            let close_rect = Rect::from_min_size(Pos2::new(close_x, btn_y), btn_size);
            let max_rect = Rect::from_min_size(Pos2::new(max_x, btn_y), btn_size);
            let min_rect = Rect::from_min_size(Pos2::new(min_x, btn_y), btn_size);

            draw_titlebar_btn(ui, close_rect, "×", hex_to_color32(&cfg.theme.error));
            draw_titlebar_btn(ui, max_rect, "□", accent);
            draw_titlebar_btn(ui, min_rect, "─", muted);

            // Check button interactions
            let pointer = ctx.input(|i| i.pointer.interact_pos());

            if ctx.input(|i| i.pointer.any_click()) {
                if let Some(pos) = pointer {
                    if close_rect.contains(pos) {
                        to_close.push(win_id);
                    } else if max_rect.contains(pos) {
                        to_maximize.push(win_id);
                    } else if min_rect.contains(pos) {
                        to_minimize.push(win_id);
                    }
                }
            }

            // Clicking title bar (outside buttons) focuses the window
            if titlebar_resp.clicked() {
                to_focus.push(win_id);
            }

            // App content
            ui.add_space(2.0);
            match app_type {
                AppType::Terminal => apps::terminal::render(ui, state),
                AppType::FileManager => apps::file_manager::render(ui, state),
                AppType::Settings => apps::settings::render(ui, state),
                AppType::About => render_about(ui, &cfg),
            }
        });

        // Clicking anywhere in the window (not just title bar) brings it to front
        if let Some(resp) = &response {
            if resp.response.clicked() {
                to_focus.push(win_id);
            }
            if !is_max {
                saved_bounds.push((win_id, resp.response.rect.min, resp.response.rect.size()));
                if pending_restore {
                    restored_windows.push(win_id);
                }
            }
        }
    }

    // Apply deferred mutations
    for id in to_close {
        state.close_window(id);
    }
    for id in to_minimize {
        if let Some(w) = state.windows.iter_mut().find(|w| w.id == id) {
            w.is_minimized = true;
        }
    }
    for id in to_maximize {
        if let Some(w) = state.windows.iter_mut().find(|w| w.id == id) {
            if w.is_maximized {
                w.is_maximized = false;
                w.pending_restore = true;
            } else {
                w.is_maximized = true;
            }
        }
    }
    for (id, pos, size) in saved_bounds {
        if let Some(w) = state.windows.iter_mut().find(|w| w.id == id) {
            w.initial_pos = pos;
            w.initial_size = size;
        }
    }
    for id in restored_windows {
        if let Some(w) = state.windows.iter_mut().find(|w| w.id == id) {
            w.pending_restore = false;
        }
    }
    for id in to_focus {
        state.focus_window(id);
    }

    // Propagate shutdown from terminal eval
    if state.shutdown_requested {
        ctx.send_viewport_cmd(egui::ViewportCommand::Close);
    }
}

// Helpers

fn draw_titlebar_btn(ui: &mut egui::Ui, rect: Rect, label: &str, color: Color32) {
    let response = ui.allocate_rect(rect, egui::Sense::click());
    let fill = if response.hovered() {
        color.linear_multiply(0.7)
    } else {
        Color32::TRANSPARENT
    };
    ui.painter().rect_filled(rect, Rounding::same(4.0), fill);
    ui.painter().text(
        rect.center(),
        egui::Align2::CENTER_CENTER,
        label,
        egui::FontId::proportional(14.0),
        color,
    );
}

fn render_about(ui: &mut egui::Ui, cfg: &Config) {
    let fg = hex_to_color32(&cfg.theme.foreground);
    let accent = hex_to_color32(&cfg.theme.accent);
    let muted = hex_to_color32(&cfg.theme.muted);

    ui.add_space(20.0);
    ui.vertical_centered(|ui| {
        ui.label(
            egui::RichText::new("Capsule OS")
                .color(accent)
                .size(22.0)
                .strong(),
        );
        ui.add_space(8.0);
        ui.label(
            egui::RichText::new(format!("Version {}", env!("CARGO_PKG_VERSION")))
                .color(fg)
                .size(14.0),
        );
        ui.add_space(4.0);
        ui.label(
            egui::RichText::new("Lightweight on the outside, a system on the inside.")
                .color(muted)
                .size(12.0)
                .italics(),
        );
    });
}
