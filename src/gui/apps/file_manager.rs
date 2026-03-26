//! File Manager application for Capsule OS.
//! A full file manager with editing, copy/paste, and previews comes in future updates.

use crate::config::Config;
use crate::gui::state::DesktopState;
use crate::gui::theme_bridge::hex_to_color32;
use egui::{Color32, RichText, Stroke, Vec2};

/// Render the file manager UI inside the calling window's content area
pub fn render(ui: &mut egui::Ui, state: &mut DesktopState) {
    let cfg = match Config::snapshot() {
        Ok(c) => c,
        Err(_) => return,
    };

    let fg = hex_to_color32(&cfg.theme.foreground);
    let accent = hex_to_color32(&cfg.theme.accent);
    let muted = hex_to_color32(&cfg.theme.muted);
    let border = hex_to_color32(&cfg.theme.border);
    let _sel_bg = hex_to_color32(&cfg.theme.selection);
    let err = hex_to_color32(&cfg.theme.error);
    let _success = hex_to_color32(&cfg.theme.success);

    // Refresh listing if dirty
    if state.file_manager.dirty {
        match state.vfs.ls(None) {
            Ok(entries) => {
                state.file_manager.entries = entries;
                state.file_manager.dirty = false;
            }
            Err(e) => {
                ui.label(RichText::new(format!("Error: {e}")).color(err));
                return;
            }
        }
    }

    let entries = state.file_manager.entries.clone();

    // Path bar
    ui.horizontal(|ui| {
        ui.label(RichText::new("📁").size(14.0));
        ui.label(
            RichText::new(state.vfs.prompt_path())
                .monospace()
                .color(accent)
                .size(13.0),
        );
    });
    ui.separator();

    // Toolbar
    ui.horizontal(|ui| {
        if ui
            .button(RichText::new("＋ New Folder").color(fg).size(12.0))
            .clicked()
        {
            // Use a counter-based name to avoid conflicts
            let name = format!("folder_{}", entries.len() + 1);
            if let Err(e) = state.vfs.mkdir(&name) {
                eprintln!("mkdir error: {e}");
            }
            state.file_manager.dirty = true;
        }
        ui.add_space(4.0);
        if ui
            .button(RichText::new("＋ New File").color(fg).size(12.0))
            .clicked()
        {
            let name = format!("file_{}.txt", entries.len() + 1);
            if let Err(e) = state.vfs.touch(&name) {
                eprintln!("touch error: {e}");
            }
            state.file_manager.dirty = true;
        }
        ui.add_space(4.0);
        if ui
            .button(RichText::new("↑ Up").color(muted).size(12.0))
            .clicked()
        {
            let _ = state.vfs.cd("..");
            state.file_manager.dirty = true;
        }
    });
    ui.add_space(6.0);

    // File grid
    let item_size = Vec2::new(130.0, 64.0);
    let available_width = ui.available_width();
    let cols = ((available_width / (item_size.x + 8.0)).floor() as usize).max(1);

    let mut navigate_to: Option<String> = None;

    egui::ScrollArea::vertical()
        .auto_shrink([false, false])
        .show(ui, |ui| {
            egui::Grid::new("fm_grid")
                .num_columns(cols)
                .spacing(Vec2::splat(8.0))
                .show(ui, |ui| {
                    for (i, entry) in entries.iter().enumerate() {
                        let icon = if entry.is_dir {
                            "📁"
                        } else {
                            icon_for_file(&entry.name)
                        };
                        let color = if entry.is_dir { accent } else { fg };

                        let btn = egui::Button::new(
                            RichText::new(format!("{icon}\n{}", truncate(&entry.name, 14)))
                                .color(color)
                                .size(12.0),
                        )
                        .min_size(item_size)
                        .fill(Color32::TRANSPARENT)
                        .stroke(Stroke::new(0.5, border));

                        let resp = ui.add(btn);

                        // Double-click to open directory
                        if resp.double_clicked() && entry.is_dir {
                            navigate_to = Some(entry.name.clone());
                        }

                        if (i + 1) % cols == 0 {
                            ui.end_row();
                        }
                    }
                });
        });

    // Apply navigation after borrow ends
    if let Some(path) = navigate_to {
        let _ = state.vfs.cd(&path);
        state.file_manager.dirty = true;
    }
}

// Helpers

fn icon_for_file(name: &str) -> &'static str {
    let ext = name.rsplit('.').next().unwrap_or("").to_lowercase();
    match ext.as_str() {
        "rs" => "🦀",
        "toml" | "json" | "yaml" | "yml" => "⚙",
        "txt" => "📄",
        "md" => "📝",
        "png" | "jpg" | "jpeg" | "gif" | "webp" => "🖼",
        "mp4" | "webm" | "avi" => "🎬",
        "mp3" | "ogg" | "wav" => "🎵",
        "zip" | "tar" | "gz" => "📦",
        _ => "📄",
    }
}

fn truncate(s: &str, max: usize) -> String {
    if s.chars().count() <= max {
        s.to_string()
    } else {
        format!(
            "{}…",
            &s[..s.char_indices().nth(max - 1).map(|(i, _)| i).unwrap_or(max)]
        )
    }
}
