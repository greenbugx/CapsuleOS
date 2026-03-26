//! Settings application for Capsule OS.
//! A full settings app comes in future updates.

use crate::config::Config;
use crate::gui::state::DesktopState;
use crate::gui::theme_bridge::hex_to_color32;
use egui::{Color32, RichText, Rounding, Stroke, Vec2};

/// Render the settings UI inside the calling window's content area
pub fn render(ui: &mut egui::Ui, state: &mut DesktopState) {
    let cfg = match Config::snapshot() {
        Ok(c) => c,
        Err(_) => return,
    };

    let fg = hex_to_color32(&cfg.theme.foreground);
    let accent = hex_to_color32(&cfg.theme.accent);
    let muted = hex_to_color32(&cfg.theme.muted);
    let _success = hex_to_color32(&cfg.theme.success);
    let _err = hex_to_color32(&cfg.theme.error);

    egui::ScrollArea::vertical()
        .auto_shrink([false, false])
        .show(ui, |ui| {
            // Theme section
            section_header(ui, "🎨  Theme", accent);

            ui.horizontal_wrapped(|ui| {
                for (label, hex) in [
                    ("background", cfg.theme.background.as_str()),
                    ("foreground", cfg.theme.foreground.as_str()),
                    ("accent", cfg.theme.accent.as_str()),
                    ("secondary", cfg.theme.secondary.as_str()),
                    ("success", cfg.theme.success.as_str()),
                    ("warning", cfg.theme.warning.as_str()),
                    ("error", cfg.theme.error.as_str()),
                    ("muted", cfg.theme.muted.as_str()),
                    ("border", cfg.theme.border.as_str()),
                    ("selection", cfg.theme.selection.as_str()),
                ] {
                    swatch(ui, label, hex, fg, muted);
                }
            });

            ui.add_space(10.0);

            // Theme selector dropdown
            ui.horizontal(|ui| {
                ui.label(RichText::new("Active theme:").color(muted).size(12.0));
                let mut selected = cfg.theme.name.clone();
                let names = Config::available_theme_names();

                egui::ComboBox::from_id_source("theme_select")
                    .selected_text(RichText::new(&selected).color(fg).size(13.0))
                    .width(180.0)
                    .show_ui(ui, |ui| {
                        for name in &names {
                            ui.selectable_value(
                                &mut selected,
                                name.clone(),
                                RichText::new(name).color(fg).size(13.0),
                            );
                        }
                    });

                if selected != cfg.theme.name {
                    let config_path = state.config_path.clone();
                    match Config::set_theme(&config_path, &selected) {
                        Ok(warnings) => {
                            let _ = state.theme.refresh_from_config();
                            state.refresh_cfg();
                            for w in warnings {
                                eprintln!("warning: {w}");
                            }
                        }
                        Err(e) => eprintln!("theme set error: {e}"),
                    }
                }
            });

            ui.add_space(16.0);
            ui.separator();

            // Shell section
            section_header(ui, "⬛  Shell", accent);

            ui.label(RichText::new("Prompt style:").color(muted).size(12.0));
            ui.add_space(4.0);

            let styles = ["arrow", "minimal", "powerline", "classic"];
            let mut current = cfg.shell.prompt_style.clone();

            ui.horizontal_wrapped(|ui| {
                for style in styles {
                    let selected = current == style;
                    let btn = egui::SelectableLabel::new(
                        selected,
                        RichText::new(style)
                            .color(if selected { accent } else { fg })
                            .size(13.0),
                    );
                    if ui.add(btn).clicked() && !selected {
                        current = style.to_string();
                        let config_path = state.config_path.clone();
                        match Config::set_key(&config_path, "shell.prompt_style", style) {
                            Ok(_) => state.refresh_cfg(),
                            Err(e) => eprintln!("config set error: {e}"),
                        }
                    }
                }
            });

            ui.add_space(16.0);
            ui.separator();

            // System info section
            section_header(ui, "ℹ  System", accent);

            let rows = [
                ("OS Name", cfg.system.name.as_str()),
                ("Version", env!("CARGO_PKG_VERSION")),
                ("Username", cfg.system.username.as_str()),
                ("Hostname", cfg.system.hostname.as_str()),
                ("Language", cfg.system.language.as_str()),
            ];

            egui::Grid::new("sysinfo_grid")
                .num_columns(2)
                .spacing(Vec2::new(16.0, 6.0))
                .striped(true)
                .show(ui, |ui| {
                    for (key, value) in rows {
                        ui.label(RichText::new(key).color(muted).size(12.0));
                        ui.label(RichText::new(value).color(fg).size(13.0));
                        ui.end_row();
                    }
                });

            ui.add_space(8.0);
        });
}

// Helpers

fn section_header(ui: &mut egui::Ui, title: &str, color: Color32) {
    ui.add_space(4.0);
    ui.label(RichText::new(title).color(color).strong().size(14.0));
    ui.add_space(6.0);
}

fn swatch(ui: &mut egui::Ui, label: &str, hex: &str, _fg: Color32, muted: Color32) {
    ui.vertical(|ui| {
        let color = hex_to_color32(hex);
        let (rect, _) = ui.allocate_exact_size(Vec2::new(64.0, 28.0), egui::Sense::hover());
        ui.painter().rect_filled(rect, Rounding::same(4.0), color);
        ui.painter()
            .rect_stroke(rect, Rounding::same(4.0), Stroke::new(0.5, muted));
        ui.label(
            RichText::new(format!("{label}\n{hex}"))
                .color(muted)
                .size(9.0),
        );
    });
}
