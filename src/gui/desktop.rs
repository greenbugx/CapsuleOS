//! Desktop wallpaper renderer for Capsule OS.
//! Fills the screen with a theme-driven gradient and paints a faint dot-grid pattern for visual texture. A version watermark is shown in the bottom-right corner just above the taskbar.

use crate::config::Config;
use crate::gui::state::DesktopState;
use crate::gui::taskbar::TASKBAR_HEIGHT;
use crate::gui::theme_bridge::{hex_to_color32, lerp_color};
use egui::{Context, Painter, Pos2, Rect, Rounding};

/// Draw the desktop background. Must be called before taskbar and windows
pub fn render(ctx: &Context, _state: &DesktopState) {
    let cfg = match Config::snapshot() {
        Ok(c) => c,
        Err(_) => return,
    };

    // Cover the full screen with a CentralPanel that has no padding
    egui::CentralPanel::default()
        .frame(egui::Frame::none())
        .show(ctx, |ui| {
            let full_rect = ui.available_rect_before_wrap();
            let painter = ui.painter();

            draw_gradient(painter, full_rect, &cfg);
            draw_dot_grid(painter, full_rect, &cfg);
            draw_watermark(painter, full_rect, &cfg);
        });
}

// Private helpers

fn draw_gradient(painter: &Painter, rect: Rect, cfg: &Config) {
    let bg = hex_to_color32(&cfg.theme.background);
    let accent = hex_to_color32(&cfg.theme.accent);

    let tint = lerp_color(bg, accent, 0.04);

    painter.rect_filled(rect, Rounding::ZERO, bg);

    let bottom_rect = Rect::from_min_max(
        Pos2::new(rect.min.x, rect.min.y + rect.height() * 0.55),
        rect.max,
    );
    painter.rect_filled(bottom_rect, Rounding::ZERO, tint);
}

fn draw_dot_grid(painter: &Painter, rect: Rect, cfg: &Config) {
    let dot_color = hex_to_color32(&cfg.theme.border).linear_multiply(0.35);
    let spacing = 28.0_f32;
    let radius = 1.0_f32;

    let taskbar_top = rect.max.y - TASKBAR_HEIGHT;

    let mut y = rect.min.y + spacing;
    while y < taskbar_top {
        let mut x = rect.min.x + spacing;
        while x < rect.max.x {
            painter.circle_filled(Pos2::new(x, y), radius, dot_color);
            x += spacing;
        }
        y += spacing;
    }
}

fn draw_watermark(painter: &Painter, rect: Rect, cfg: &Config) {
    let text = format!("Capsule OS v{}", env!("CARGO_PKG_VERSION"));
    let color = hex_to_color32(&cfg.theme.muted).linear_multiply(0.6);

    let font_id = egui::FontId::monospace(11.0);
    let galley = painter.layout_no_wrap(text, font_id, color);
    let galley_size = galley.size();

    // Bottom-right, 12px from edge and TASKBAR_HEIGHT above the taskbar
    let pos = Pos2::new(
        rect.max.x - galley_size.x - 12.0,
        rect.max.y - TASKBAR_HEIGHT - galley_size.y - 6.0,
    );

    painter.galley(pos, galley, color);
}
