//! Embedded terminal application for Capsule OS.

use crate::config::Config;
use crate::gui::state::DesktopState;
use crate::gui::theme_bridge::hex_to_color32;
use crate::shell::{eval, EvalSignal, OutputLine};
use crate::theme::ThemeRole;
use egui::{Color32, RichText, ScrollArea};

const NEOFETCH_INFO_KEYS: [&str; 10] = [
    "OS",
    "Version",
    "User",
    "Host",
    "Shell CWD",
    "VFS Root",
    "Uptime",
    "Host Home",
    "Theme",
    "Prompt",
];

pub fn render(ui: &mut egui::Ui, state: &mut DesktopState) {
    let cfg = match Config::snapshot() {
        Ok(c) => c,
        Err(_) => return,
    };

    let _bg_color = hex_to_color32(&cfg.theme.background);
    let _border_color = hex_to_color32(&cfg.theme.border);
    let _input_bg = hex_to_color32(&cfg.theme.selection);
    let fg_color = hex_to_color32(&cfg.theme.foreground);
    let accent = hex_to_color32(&cfg.theme.accent);
    // let width = ui.available_width();

    // let input_height = 30.0;
    // let scroll_height = (available.y - input_height - 12.0).max(60.0);

    ui.vertical(|ui| {
        let scroll_height = ui.available_height() - 40.0;

        egui::ScrollArea::vertical()
            .max_height(scroll_height)
            .show(ui, |ui| {
                let scroll = ScrollArea::vertical()
                    .max_height(scroll_height)
                    .auto_shrink([false, false])
                    .id_source("terminal_scroll");

                scroll.show(ui, |ui| {
                    ui.add_space(2.0);
                    ui.set_min_width(ui.available_width());

                    let mut index = 0usize;
                    while index < state.terminal.output.len() {
                        let line = &state.terminal.output[index];
                        let clean = strip_ansi(&line.text);
                        let clean = clean.trim_end();

                        if should_render_neofetch_block(&state.terminal.output[index..]) {
                            index += render_neofetch_block(
                                ui,
                                &state.terminal.output[index..],
                                accent,
                                fg_color,
                            );
                            continue;
                        }

                        render_terminal_line(ui, clean, line, &cfg);
                        index += 1;
                    }

                    // Scroll to bottom flag
                    if state.terminal.scroll_to_bottom {
                        ui.scroll_to_cursor(Some(egui::Align::BOTTOM));
                        state.terminal.scroll_to_bottom = false;
                    }
                });
            });

        ui.separator();

        ui.horizontal(|ui| {
            let raw_prompt = crate::prompt::render_prompt(&state.theme, &cfg, &state.terminal.cwd);
            let prompt_clean = strip_ansi(&raw_prompt);
            if !render_ansi_segments_inline(ui, &raw_prompt, accent, 13.0) {
                ui.label(
                    RichText::new(&prompt_clean)
                        .monospace()
                        .color(accent)
                        .size(13.0),
                );
            }

            // Single-line text input
            let input_field = egui::TextEdit::singleline(&mut state.terminal.input)
                .font(egui::TextStyle::Monospace)
                .desired_width(f32::INFINITY)
                .frame(false)
                .text_color(fg_color)
                .hint_text("type a command…");

            let resp = ui.add(input_field);

            // Submit on Enter
            if resp.lost_focus() && ui.input(|i| i.key_pressed(egui::Key::Enter)) {
                let raw = state.terminal.input.trim().to_string();
                state.terminal.input.clear();

                if raw.is_empty() {
                    return;
                }

                // Echo the command line
                let prompt_echo = format!("{}{}", raw_prompt, raw);
                state
                    .terminal
                    .output
                    .push(OutputLine::new(prompt_echo, ThemeRole::Muted));

                // Evaluate
                let width = ui.max_rect().width().max(200.0);
                let result = eval(&raw, &mut state.vfs, state.terminal.start_time, width);

                // Handle signals
                match result.signal {
                    EvalSignal::Clear => {
                        state.terminal.output.clear();
                    }
                    EvalSignal::Shutdown => {
                        for line in &result.lines {
                            state.terminal.output.push(line.clone());
                        }
                        state.shutdown_requested = true;
                    }
                    EvalSignal::Continue => {
                        for line in result.lines {
                            state.terminal.output.push(line);
                        }
                    }
                }

                // Update shown CWD
                state.terminal.cwd = state.vfs.prompt_path();
                state.terminal.scroll_to_bottom = true;

                state.refresh_cfg();

                resp.request_focus();
            }

            // Auto-focus input when window gains interaction
            if resp.gained_focus() || !resp.has_focus() {
                resp.request_focus();
            }
        });
    });

    ui.add_space(4.0);
    ui.add_space(2.0);
}

fn strip_ansi(input: &str) -> String {
    let mut result = String::new();
    let mut chars = input.chars().peekable();

    while let Some(c) = chars.next() {
        if c == '\x1b' {
            // skip until 'm'
            while let Some(&next) = chars.peek() {
                chars.next();
                if next == 'm' {
                    break;
                }
            }
        } else {
            result.push(c);
        }
    }

    result
}

fn render_terminal_line(ui: &mut egui::Ui, clean: &str, line: &OutputLine, cfg: &Config) {
    let color = role_to_color32(&line.role, cfg);
    let is_info = is_neofetch_info_line(clean);
    let is_ascii = is_ascii_art_line(clean);
    let size = if is_ascii {
        8.6
    } else if is_info {
        12.0
    } else {
        13.0
    };

    if render_ansi_text_line(ui, &line.text, color, size) {
        return;
    }

    ui.add(egui::Label::new(RichText::new(clean).monospace().color(color).size(size)).wrap(false));
}

fn render_neofetch_block(
    ui: &mut egui::Ui,
    lines: &[OutputLine],
    accent: Color32,
    fg_color: Color32,
) -> usize {
    let mut ascii_lines: Vec<String> = Vec::new();
    let mut info_lines: Vec<String> = Vec::new();
    let mut consumed = 0usize;
    let mut saw_split_row = false;

    for line in lines {
        let clean = strip_ansi(&line.text);
        let clean = clean.trim_end();

        if let Some((ascii_col, info_col)) = split_neofetch_columns(clean) {
            ascii_lines.push(ascii_col.trim_end().to_string());
            info_lines.push(info_col.to_string());
            consumed += 1;
            saw_split_row = true;
            continue;
        }

        if saw_split_row && is_ascii_art_line(clean) {
            ascii_lines.push(clean.to_string());
            info_lines.push(String::new());
            consumed += 1;
            continue;
        }

        break;
    }

    if consumed == 0 {
        return 0;
    }

    let ascii_text = ascii_lines.join("\n");
    let info_text = info_lines.join("\n");
    let ascii_width = ascii_lines
        .iter()
        .map(|line| line.chars().count())
        .max()
        .unwrap_or(0) as f32;
    let max_left = (ui.available_width() - 220.0).max(200.0);
    let left_col_width = (ascii_width * 6.6 + 8.0).clamp(200.0, max_left);

    ui.with_layout(egui::Layout::left_to_right(egui::Align::TOP), |ui| {
        ui.add_sized(
            [left_col_width, 0.0],
            egui::Label::new(
                RichText::new(ascii_text)
                    .monospace()
                    .color(accent)
                    .size(8.6),
            )
            .wrap(false),
        );

        if !info_text.trim().is_empty() {
            ui.add_space(8.0);
            ui.add(
                egui::Label::new(
                    RichText::new(info_text)
                        .monospace()
                        .color(fg_color)
                        .size(12.0),
                )
                .wrap(false),
            );
        }
    });

    consumed
}

#[derive(Clone, Copy, Default)]
struct AnsiStyle {
    fg: Option<Color32>,
    bg: Option<Color32>,
}

struct AnsiSpan {
    text: String,
    style: AnsiStyle,
}

fn render_ansi_text_line(
    ui: &mut egui::Ui,
    text: &str,
    fallback_color: Color32,
    size: f32,
) -> bool {
    let spans = parse_ansi_spans(text);
    if !spans_have_style(&spans) {
        return false;
    }

    ui.horizontal(|ui| {
        ui.spacing_mut().item_spacing.x = 0.0;
        render_ansi_spans(ui, &spans, fallback_color, size);
    });
    true
}

fn render_ansi_segments_inline(
    ui: &mut egui::Ui,
    text: &str,
    fallback_color: Color32,
    size: f32,
) -> bool {
    let spans = parse_ansi_spans(text);
    if !spans_have_style(&spans) {
        return false;
    }

    let old_spacing = ui.spacing().item_spacing.x;
    ui.spacing_mut().item_spacing.x = 0.0;
    render_ansi_spans(ui, &spans, fallback_color, size);
    ui.spacing_mut().item_spacing.x = old_spacing;
    true
}

fn render_ansi_spans(ui: &mut egui::Ui, spans: &[AnsiSpan], fallback_color: Color32, size: f32) {
    for span in spans {
        if span.text.is_empty() {
            continue;
        }

        let mut rich = RichText::new(&span.text)
            .monospace()
            .size(size)
            .color(span.style.fg.unwrap_or(fallback_color));
        if let Some(bg) = span.style.bg {
            rich = rich.background_color(bg);
        }
        ui.label(rich);
    }
}

fn spans_have_style(spans: &[AnsiSpan]) -> bool {
    spans
        .iter()
        .any(|span| span.style.fg.is_some() || span.style.bg.is_some())
}

fn parse_ansi_spans(input: &str) -> Vec<AnsiSpan> {
    let mut spans = Vec::new();
    let mut current = String::new();
    let mut style = AnsiStyle::default();
    let chars: Vec<char> = input.chars().collect();
    let mut i = 0usize;

    while i < chars.len() {
        if chars[i] == '\x1b' && i + 1 < chars.len() && chars[i + 1] == '[' {
            if !current.is_empty() {
                spans.push(AnsiSpan {
                    text: std::mem::take(&mut current),
                    style,
                });
            }

            i += 2;
            let mut code = String::new();
            while i < chars.len() && chars[i] != 'm' {
                code.push(chars[i]);
                i += 1;
            }

            if i < chars.len() && chars[i] == 'm' {
                apply_sgr(&code, &mut style);
            }
        } else {
            current.push(chars[i]);
        }
        i += 1;
    }

    if !current.is_empty() {
        spans.push(AnsiSpan {
            text: current,
            style,
        });
    }

    spans
}

fn apply_sgr(code: &str, style: &mut AnsiStyle) {
    let parts: Vec<u16> = if code.is_empty() {
        vec![0]
    } else {
        code.split(';')
            .filter_map(|part| part.parse::<u16>().ok())
            .collect()
    };

    let mut i = 0usize;
    while i < parts.len() {
        match parts[i] {
            0 => *style = AnsiStyle::default(),
            39 => style.fg = None,
            49 => style.bg = None,
            38 if i + 4 < parts.len() && parts[i + 1] == 2 => {
                style.fg = Some(Color32::from_rgb(
                    parts[i + 2] as u8,
                    parts[i + 3] as u8,
                    parts[i + 4] as u8,
                ));
                i += 4;
            }
            48 if i + 4 < parts.len() && parts[i + 1] == 2 => {
                style.bg = Some(Color32::from_rgb(
                    parts[i + 2] as u8,
                    parts[i + 3] as u8,
                    parts[i + 4] as u8,
                ));
                i += 4;
            }
            _ => {}
        }
        i += 1;
    }
}

fn should_render_neofetch_block(lines: &[OutputLine]) -> bool {
    for line in lines.iter().take(4) {
        let clean = strip_ansi(&line.text);
        let clean = clean.trim_end();

        let Some(idx) = find_neofetch_info_start(clean) else {
            if clean.trim().is_empty() {
                continue;
            }
            return false;
        };

        if !clean[..idx].trim().is_empty() {
            return true;
        }
    }

    false
}

fn is_neofetch_info_line(line: &str) -> bool {
    let trimmed = line.trim_start();
    NEOFETCH_INFO_KEYS
        .iter()
        .any(|key| starts_with_key_colon(trimmed, key))
}

fn is_ascii_art_line(line: &str) -> bool {
    let trimmed = line.trim();
    if trimmed.is_empty() {
        return false;
    }

    let symbol_count = trimmed
        .chars()
        .filter(|ch| !ch.is_alphanumeric() && !ch.is_whitespace())
        .count();
    let alpha_count = trimmed.chars().filter(|ch| ch.is_alphabetic()).count();

    symbol_count >= 6 && symbol_count > (alpha_count * 2)
}

fn split_neofetch_columns(line: &str) -> Option<(&str, &str)> {
    let idx = find_neofetch_info_start(line)?;
    let (left, right) = line.split_at(idx);
    Some((left, right.trim_start()))
}

fn find_neofetch_info_start(line: &str) -> Option<usize> {
    let mut best_match: Option<usize> = None;

    for key in NEOFETCH_INFO_KEYS {
        let mut search_from = 0usize;
        while let Some(rel) = line[search_from..].find(key) {
            let idx = search_from + rel;
            if starts_with_key_colon(&line[idx..], key) {
                best_match = Some(match best_match {
                    Some(current) => current.min(idx),
                    None => idx,
                });
                break;
            }
            search_from = idx + key.len();
        }
    }

    best_match
}

fn starts_with_key_colon(text: &str, key: &str) -> bool {
    let Some(rest) = text.strip_prefix(key) else {
        return false;
    };
    rest.trim_start().starts_with(':')
}

// Colour mapping

fn role_to_color32(role: &ThemeRole, cfg: &Config) -> Color32 {
    let hex = match role {
        ThemeRole::Primary => cfg.theme.foreground.as_str(),
        ThemeRole::Secondary => cfg.theme.secondary.as_str(),
        ThemeRole::Accent => cfg.theme.accent.as_str(),
        ThemeRole::Success => cfg.theme.success.as_str(),
        ThemeRole::Warning => cfg.theme.warning.as_str(),
        ThemeRole::Error => cfg.theme.error.as_str(),
        ThemeRole::Muted => cfg.theme.muted.as_str(),
        ThemeRole::Border => cfg.theme.border.as_str(),
    };
    hex_to_color32(hex)
}
