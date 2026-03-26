//! Theme bridge translates Capsule OS `capsule.toml` colors into egui `Visuals`.
//! `capsule_visuals` is called every frame so that live theme hot-reload is instantly reflected in the entire GUI without any additional plumbing.

use crate::config::Config;
use crate::theme::ThemeEngine;
use egui::{Color32, Stroke, Visuals};

// Public helpers

/// Parse a `#rrggbb` hex string into an `egui::Color32`
/// Returns a visible magenta sentinel on malformed input so bugs are obvious
pub fn hex_to_color32(hex: &str) -> Color32 {
    let bytes = hex.as_bytes();
    if bytes.len() != 7 || bytes[0] != b'#' {
        return Color32::from_rgb(255, 0, 255); // sentinel
    }
    let r = u8::from_str_radix(&hex[1..3], 16).unwrap_or(255);
    let g = u8::from_str_radix(&hex[3..5], 16).unwrap_or(0);
    let b = u8::from_str_radix(&hex[5..7], 16).unwrap_or(255);
    Color32::from_rgb(r, g, b)
}

/// Same as `hex_to_color32` but with a custom alpha value (0–255)
pub fn hex_to_color32_alpha(hex: &str, alpha: u8) -> Color32 {
    let c = hex_to_color32(hex);
    Color32::from_rgba_unmultiplied(c.r(), c.g(), c.b(), alpha)
}

/// Mix two colors by linear interpolation (t = 0 → a, t = 1 → b)
pub fn lerp_color(a: Color32, b: Color32, t: f32) -> Color32 {
    let t = t.clamp(0.0, 1.0);
    Color32::from_rgb(
        (a.r() as f32 + (b.r() as f32 - a.r() as f32) * t) as u8,
        (a.g() as f32 + (b.g() as f32 - a.g() as f32) * t) as u8,
        (a.b() as f32 + (b.b() as f32 - a.b() as f32) * t) as u8,
    )
}

/// Build a fully theme-driven `egui::Visuals` from the current `Config`
pub fn capsule_visuals(_theme: &ThemeEngine, cfg: &Config) -> Visuals {
    let bg = hex_to_color32(&cfg.theme.background);
    let fg = hex_to_color32(&cfg.theme.foreground);
    let accent = hex_to_color32(&cfg.theme.accent);
    let secondary = hex_to_color32(&cfg.theme.secondary);
    let muted = hex_to_color32(&cfg.theme.muted);
    let border = hex_to_color32(&cfg.theme.border);
    let selection = hex_to_color32(&cfg.theme.selection);
    let error = hex_to_color32(&cfg.theme.error);

    // Slightly lighter panel background
    let panel_fill = lerp_color(bg, Color32::WHITE, 0.05);
    // Title-bar or header background
    let header_fill = lerp_color(bg, Color32::WHITE, 0.08);

    let widget_style = |fill: Color32, stroke_color: Color32| -> egui::style::WidgetVisuals {
        egui::style::WidgetVisuals {
            bg_fill: fill,
            weak_bg_fill: fill,
            bg_stroke: Stroke::new(1.0, stroke_color),
            rounding: egui::Rounding::same(4.0),
            fg_stroke: Stroke::new(1.5, fg),
            expansion: 0.0,
        }
    };

    let mut visuals = Visuals::dark();

    // Window chrome
    visuals.window_fill = bg;
    visuals.window_stroke = Stroke::new(1.0, border);
    visuals.window_rounding = egui::Rounding::same(6.0);
    visuals.window_shadow = egui::epaint::Shadow {
        offset: egui::Vec2::new(4.0, 4.0),
        blur: 12.0,
        spread: 0.0,
        color: Color32::from_black_alpha(80),
    };

    // Panels
    visuals.panel_fill = panel_fill;
    visuals.faint_bg_color = hex_to_color32_alpha(&cfg.theme.border, 40);
    visuals.extreme_bg_color = selection;

    // Text
    visuals.override_text_color = Some(fg);

    // Selection
    visuals.selection.bg_fill = hex_to_color32_alpha(&cfg.theme.accent, 120);
    visuals.selection.stroke = Stroke::new(1.0, accent);

    // Hyperlinks
    visuals.hyperlink_color = accent;

    // Widgets
    visuals.widgets.noninteractive = widget_style(panel_fill, border);
    visuals.widgets.inactive = widget_style(header_fill, border);
    visuals.widgets.hovered = widget_style(hex_to_color32_alpha(&cfg.theme.accent, 60), accent);
    visuals.widgets.active = widget_style(accent, accent);
    visuals.widgets.open = widget_style(secondary, secondary);

    // Error & warning colors
    visuals.error_fg_color = error;
    visuals.warn_fg_color = hex_to_color32(&cfg.theme.warning);

    // Separator color
    visuals.widgets.noninteractive.bg_stroke = Stroke::new(1.0, muted);

    visuals
}
