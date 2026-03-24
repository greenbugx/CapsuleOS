//! Prompt renderer for Capsule OS.
//! This module generates shell prompts from config styles and current runtime context.

use crate::config::Config;
use crate::theme::{ThemeEngine, ThemeRole};

pub fn render_prompt(theme: &ThemeEngine, cfg: &Config, path: &str) -> String {
    match cfg.shell.prompt_style.as_str() {
        "minimal" => theme.apply("$ ", ThemeRole::Primary),
        "powerline" => render_powerline(theme, cfg, path),
        "classic" => {
            let text = format!("{}@{}:{}$ ", cfg.system.username, cfg.system.hostname, path);
            theme.apply(&text, ThemeRole::Accent)
        }
        _ => theme.apply("\u{276F} ", ThemeRole::Accent),
    }
}

fn render_powerline(theme: &ThemeEngine, cfg: &Config, path: &str) -> String {
    let segment = format!(" {}@{} {} ", cfg.system.username, cfg.system.hostname, path);
    let pointer = " \u{276F} ";

    let left = theme.paint_bg(&segment, &cfg.theme.foreground, &cfg.theme.selection);
    let right = theme.paint_bg(pointer, &cfg.theme.background, &cfg.theme.accent);
    format!("{left}{right}")
}
