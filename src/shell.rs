//! Interactive shell for Capsule OS.
//! This module provides both the classic REPL (`run_shell`) and the pure-functional `eval` entry point used by the GUI terminal window.

use crate::boot;
use crate::config::{self, Config};
use crate::fs::VirtualFs;
use crate::prompt;
use crate::theme::{ThemeEngine, ThemeRole};
use anyhow::{anyhow, Result};
use std::env;
use std::io::{self, Write};
use std::path::Path;
use std::process::Command;
use std::time::{Duration, Instant};

const NEOFETCH_ASCII: &str = include_str!("../boot/static/ascii-art.txt");

// Public eval API

/// A single line of shell output annotated with a semantic colour role
#[derive(Debug, Clone)]
pub struct OutputLine {
    pub text: String,
    pub role: ThemeRole,
}

impl OutputLine {
    pub fn new(text: impl Into<String>, role: ThemeRole) -> Self {
        Self {
            text: text.into(),
            role,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum EvalSignal {
    /// Continue normally
    Continue,
    /// The terminal output buffer should be cleared
    Clear,
    /// Capsule OS shutdown was requested
    Shutdown,
}

/// Result of evaluating one command line
#[derive(Debug, Clone)]
pub struct EvalResult {
    pub lines: Vec<OutputLine>,
    pub signal: EvalSignal,
}

impl EvalResult {
    fn ok(lines: Vec<OutputLine>) -> Self {
        Self {
            lines,
            signal: EvalSignal::Continue,
        }
    }
    fn clear() -> Self {
        Self {
            lines: vec![],
            signal: EvalSignal::Clear,
        }
    }
    fn shutdown(lines: Vec<OutputLine>) -> Self {
        Self {
            lines,
            signal: EvalSignal::Shutdown,
        }
    }
}

pub fn eval(cmd: &str, vfs: &mut VirtualFs, start_time: Instant, width: f32) -> EvalResult {
    let command_line = match expand_alias(cmd) {
        Ok(c) => c,
        Err(e) => {
            return EvalResult::ok(vec![OutputLine::new(e.to_string(), ThemeRole::Error)]);
        }
    };

    let mut parts = command_line.split_whitespace();
    let command = match parts.next() {
        Some(c) => c,
        None => return EvalResult::ok(vec![]),
    };

    match command {
        "help" => EvalResult::ok(help_lines()),
        "clear" => EvalResult::clear(),

        "ls" => {
            let path = parts.next();
            if parts.next().is_some() {
                EvalResult::ok(err_lines("Usage: ls [path]"))
            } else {
                match vfs.ls(path) {
                    Ok(entries) if entries.is_empty() => {
                        EvalResult::ok(vec![OutputLine::new("(empty directory)", ThemeRole::Muted)])
                    }
                    Ok(entries) => {
                        let lines = entries
                            .iter()
                            .map(|e| {
                                if e.is_dir {
                                    OutputLine::new(format!("{}/", e.name), ThemeRole::Accent)
                                } else {
                                    OutputLine::new(&e.name, ThemeRole::Primary)
                                }
                            })
                            .collect();
                        EvalResult::ok(lines)
                    }
                    Err(e) => EvalResult::ok(err_lines(&e)),
                }
            }
        }

        "cd" => {
            let Some(path) = parts.next() else {
                return EvalResult::ok(err_lines("Usage: cd <path>"));
            };
            if parts.next().is_some() {
                return EvalResult::ok(err_lines("Usage: cd <path>"));
            }
            match vfs.cd(path) {
                Ok(()) => EvalResult::ok(vec![]),
                Err(e) => EvalResult::ok(err_lines(&e)),
            }
        }

        "mkdir" => {
            let Some(path) = parts.next() else {
                return EvalResult::ok(err_lines("Usage: mkdir <name>"));
            };
            if parts.next().is_some() {
                return EvalResult::ok(err_lines("Usage: mkdir <name>"));
            }
            match vfs.mkdir(path) {
                Ok(()) => EvalResult::ok(vec![OutputLine::new(
                    format!("Created directory '{path}'"),
                    ThemeRole::Success,
                )]),
                Err(e) => EvalResult::ok(err_lines(&e)),
            }
        }

        "touch" => {
            let Some(path) = parts.next() else {
                return EvalResult::ok(err_lines("Usage: touch <name>"));
            };
            if parts.next().is_some() {
                return EvalResult::ok(err_lines("Usage: touch <name>"));
            }
            match vfs.touch(path) {
                Ok(()) => EvalResult::ok(vec![OutputLine::new(
                    format!("Created file '{path}'"),
                    ThemeRole::Success,
                )]),
                Err(e) => EvalResult::ok(err_lines(&e)),
            }
        }

        "cat" => {
            let Some(path) = parts.next() else {
                return EvalResult::ok(err_lines("Usage: cat <file>"));
            };
            if parts.next().is_some() {
                return EvalResult::ok(err_lines("Usage: cat <file>"));
            }
            match vfs.cat(path) {
                Ok(contents) if contents.is_empty() => {
                    EvalResult::ok(vec![OutputLine::new("(empty file)", ThemeRole::Muted)])
                }
                Ok(contents) => {
                    let lines = contents
                        .lines()
                        .map(|l| OutputLine::new(l, ThemeRole::Primary))
                        .collect();
                    EvalResult::ok(lines)
                }
                Err(e) => EvalResult::ok(err_lines(&e)),
            }
        }

        "neofetch" => EvalResult::ok(neofetch_lines(vfs, start_time, width)),

        "capsule" => {
            let args: Vec<&str> = parts.collect();
            EvalResult::ok(capsule_command_lines(args))
        }

        "shutdown" => EvalResult::shutdown(vec![OutputLine::new(
            "Shutting down Capsule OS...",
            ThemeRole::Warning,
        )]),

        _ => EvalResult::ok(err_lines(
            "Unknown command. Run `help` to list available commands.",
        )),
    }
}

// Classic REPL

#[allow(dead_code)]
pub fn run_shell(theme: ThemeEngine, mut vfs: VirtualFs) -> Result<()> {
    let start_time = Instant::now();
    let stdin = io::stdin();

    loop {
        for warning in theme.take_warnings() {
            println!(
                "{}",
                theme.apply(&format!("warning: {warning}"), ThemeRole::Warning)
            );
        }

        print_prompt(&theme, &vfs)?;

        let mut input = String::new();
        let read = stdin.read_line(&mut input)?;
        if read == 0 {
            boot::run_shutdown_sequence(&theme)?;
            break;
        }

        let cmd = input.trim();
        if cmd.is_empty() {
            continue;
        }

        let result = eval(cmd, &mut vfs, start_time, 800.0);

        match result.signal {
            EvalSignal::Clear => {
                boot::clear_screen()?;
            }
            EvalSignal::Shutdown => {
                for line in &result.lines {
                    println!("{}", theme.apply(&line.text, line.role));
                }
                boot::run_shutdown_sequence(&theme)?;
                break;
            }
            EvalSignal::Continue => {}
        }

        for line in &result.lines {
            println!("{}", theme.apply(&line.text, line.role));
        }
    }

    Ok(())
}

// Internal helpers

#[allow(dead_code)]
fn print_prompt(theme: &ThemeEngine, vfs: &VirtualFs) -> Result<()> {
    let cfg = Config::snapshot()?;
    let prompt = prompt::render_prompt(theme, &cfg, &vfs.prompt_path());
    print!("{prompt}");
    io::stdout().flush()?;
    Ok(())
}

fn expand_alias(command_line: &str) -> Result<String> {
    let cfg = Config::snapshot()?;
    let mut parts = command_line.split_whitespace();
    let Some(first) = parts.next() else {
        return Ok(command_line.to_string());
    };
    if let Some(expanded) = cfg.shell.aliases.get(first) {
        let rest = parts.collect::<Vec<_>>().join(" ");
        Ok(if rest.is_empty() {
            expanded.clone()
        } else {
            format!("{expanded} {rest}")
        })
    } else {
        Ok(command_line.to_string())
    }
}

fn err_lines(msg: &str) -> Vec<OutputLine> {
    vec![OutputLine::new(msg, ThemeRole::Error)]
}

fn help_lines() -> Vec<OutputLine> {
    vec![
        OutputLine::new("Capsule OS — available commands", ThemeRole::Accent),
        OutputLine::new(
            "  help                    list commands",
            ThemeRole::Primary,
        ),
        OutputLine::new("  ls [path]               list files", ThemeRole::Primary),
        OutputLine::new(
            "  cd <path>               change directory",
            ThemeRole::Primary,
        ),
        OutputLine::new(
            "  mkdir <name>            create directory",
            ThemeRole::Primary,
        ),
        OutputLine::new("  touch <name>            create file", ThemeRole::Primary),
        OutputLine::new(
            "  cat <file>              print file contents",
            ThemeRole::Primary,
        ),
        OutputLine::new(
            "  clear                   clear terminal output",
            ThemeRole::Primary,
        ),
        OutputLine::new(
            "  neofetch                system information",
            ThemeRole::Primary,
        ),
        OutputLine::new(
            "  capsule install <pkg>   package manager (Phase 6)",
            ThemeRole::Primary,
        ),
        OutputLine::new(
            "  capsule theme ...       theme manager",
            ThemeRole::Primary,
        ),
        OutputLine::new(
            "  capsule config ...      config manager",
            ThemeRole::Primary,
        ),
        OutputLine::new(
            "  shutdown                exit Capsule OS",
            ThemeRole::Primary,
        ),
    ]
}

fn neofetch_lines(vfs: &VirtualFs, start_time: Instant, width: f32) -> Vec<OutputLine> {
    let cfg = match Config::snapshot() {
        Ok(c) => c,
        Err(e) => return err_lines(&e.to_string()),
    };

    let uptime = format_duration(start_time.elapsed());
    let host_home = dirs::home_dir()
        .map(|p| p.display().to_string())
        .unwrap_or_else(|| "unavailable".to_string());

    let max_chars = ((width / 8.4) as usize).max(72);
    let ascii_lines = normalize_neofetch_ascii();

    let info_lines: Vec<String> = vec![
        format_neofetch_kv("OS", &cfg.system.name),
        format_neofetch_kv("Version", env!("CARGO_PKG_VERSION")),
        format_neofetch_kv("User", &cfg.system.username),
        format_neofetch_kv("Host", &cfg.system.hostname),
        format_neofetch_kv("Shell CWD", &vfs.cwd()),
        format_neofetch_kv("VFS Root", &vfs.host_root().display().to_string()),
        format_neofetch_kv("Uptime", &uptime),
        format_neofetch_kv("Host Home", &host_home),
        format_neofetch_kv("Theme", &cfg.theme.name),
        format_neofetch_kv("Prompt", &cfg.shell.prompt_style),
    ];

    let mut lines: Vec<OutputLine> = Vec::new();

    let ascii_width = ascii_lines
        .iter()
        .map(|line| line.chars().count())
        .max()
        .unwrap_or(0);
    // Prefer side-by-side layout on typical GUI terminal widths.
    let use_side_by_side = max_chars >= 90;

    if use_side_by_side {
        let max_lines = ascii_lines.len().max(info_lines.len());

        for i in 0..max_lines {
            let ascii = ascii_lines.get(i).map(|s| s.as_str()).unwrap_or("");
            let info = info_lines.get(i).map(|s| s.as_str()).unwrap_or("");

            let line_text = if info.is_empty() {
                ascii.to_string()
            } else {
                format!("{ascii:ascii_width$}   {info}")
            };
            let role = if info.is_empty() {
                ThemeRole::Accent
            } else {
                ThemeRole::Primary
            };
            lines.push(OutputLine::new(line_text, role));
        }
    } else {
        // stack layout
        for line in &ascii_lines {
            lines.push(OutputLine::new(line, ThemeRole::Accent));
        }

        lines.push(OutputLine::new("", ThemeRole::Muted));

        for info in info_lines {
            lines.push(OutputLine::new(info, ThemeRole::Primary));
        }
    }
    lines
}

fn format_neofetch_kv(label: &str, value: &str) -> String {
    format!("{label:<10}: {value}")
}

fn normalize_neofetch_ascii() -> Vec<String> {
    let mut lines: Vec<String> = NEOFETCH_ASCII
        .lines()
        .map(|line| line.to_string())
        .collect();

    while lines
        .first()
        .map(|line| line.trim().is_empty())
        .unwrap_or(false)
    {
        lines.remove(0);
    }
    while lines
        .last()
        .map(|line| line.trim().is_empty())
        .unwrap_or(false)
    {
        lines.pop();
    }

    let common_indent = lines
        .iter()
        .filter(|line| !line.trim().is_empty())
        .map(|line| line.chars().take_while(|ch| *ch == ' ').count())
        .min()
        .unwrap_or(0);

    let mut normalized: Vec<String> = lines
        .into_iter()
        .map(|line| {
            let trimmed = if line.chars().count() >= common_indent {
                line.chars().skip(common_indent).collect::<String>()
            } else {
                line
            };
            trimmed.trim_end().to_string()
        })
        .collect();

    // If pasted ASCII art is centered, shift it left without clipping actual glyphs.
    let top_indent = normalized
        .iter()
        .find(|line| !line.trim().is_empty())
        .map(|line| count_leading_spaces(line))
        .unwrap_or(0);
    let target_top_indent = 8usize;
    if top_indent > target_top_indent {
        let shift = top_indent - target_top_indent;
        normalized = normalized
            .into_iter()
            .map(|line| {
                let leading = count_leading_spaces(&line);
                let cut = leading.min(shift);
                line.chars().skip(cut).collect::<String>()
            })
            .collect();
    }

    normalized
}

fn count_leading_spaces(line: &str) -> usize {
    line.chars().take_while(|ch| *ch == ' ').count()
}

fn capsule_command_lines(args: Vec<&str>) -> Vec<OutputLine> {
    if args.is_empty() {
        return err_lines("Usage: capsule <install|theme|config> ...");
    }

    match args[0] {
        "install" => {
            if args.len() < 2 {
                return err_lines("Usage: capsule install <package>");
            }
            let package = args[1..].join(" ");
            vec![
                OutputLine::new(
                    "Capsule package manager is in skeleton mode.",
                    ThemeRole::Warning,
                ),
                OutputLine::new(format!("Requested: {package}"), ThemeRole::Accent),
                OutputLine::new(
                    "`capsule install` will be fully wired in Phase 6.",
                    ThemeRole::Muted,
                ),
            ]
        }
        "theme" => capsule_theme_lines(&args[1..]),
        "config" => capsule_config_lines(&args[1..]),
        _ => err_lines("Usage: capsule <install|theme|config> ..."),
    }
}

fn capsule_theme_lines(args: &[&str]) -> Vec<OutputLine> {
    if args.is_empty() {
        return err_lines("Usage: capsule theme <list|set|show|edit|reset>");
    }

    match args[0] {
        "list" => {
            let current = Config::snapshot().map(|c| c.theme.name).unwrap_or_default();
            let mut lines = vec![OutputLine::new("Available themes:", ThemeRole::Secondary)];
            for name in config::Config::available_theme_names() {
                let marker = if name == current { "✓" } else { " " };
                lines.push(OutputLine::new(
                    format!("  {marker} {name}"),
                    ThemeRole::Primary,
                ));
            }
            lines
        }
        "set" => {
            let Some(name) = args.get(1) else {
                return err_lines("Usage: capsule theme set <name>");
            };
            let config_path = config::config_path();
            match Config::set_theme(&config_path, name) {
                Ok(warnings) => {
                    let mut lines = vec![OutputLine::new(
                        format!("Switched theme to '{name}'"),
                        ThemeRole::Success,
                    )];
                    for w in warnings {
                        lines.push(OutputLine::new(format!("warning: {w}"), ThemeRole::Warning));
                    }
                    lines
                }
                Err(e) => err_lines(&e.to_string()),
            }
        }
        "show" => {
            let cfg = match Config::snapshot() {
                Ok(c) => c,
                Err(e) => return err_lines(&e.to_string()),
            };
            let mut lines = vec![OutputLine::new(
                format!("Current theme: {}", cfg.theme.name),
                ThemeRole::Accent,
            )];
            for (key, hex) in [
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
                lines.push(OutputLine::new(
                    format!("  {key:<11} {hex}"),
                    ThemeRole::Primary,
                ));
            }
            lines
        }
        "reset" => {
            let config_path = config::config_path();
            match Config::reset_theme_to_default(&config_path) {
                Ok(warnings) => {
                    let mut lines = vec![OutputLine::new(
                        "Theme reset to default-dark.",
                        ThemeRole::Success,
                    )];
                    for w in warnings {
                        lines.push(OutputLine::new(format!("warning: {w}"), ThemeRole::Warning));
                    }
                    lines
                }
                Err(e) => err_lines(&e.to_string()),
            }
        }
        "edit" => vec![OutputLine::new(
            "Use `capsule theme set <name>` to switch themes inside the GUI.",
            ThemeRole::Muted,
        )],
        _ => err_lines("Usage: capsule theme <list|set|show|edit|reset>"),
    }
}

fn capsule_config_lines(args: &[&str]) -> Vec<OutputLine> {
    if args.is_empty() {
        return err_lines("Usage: capsule config <get|set|list|reload>");
    }

    match args[0] {
        "get" => {
            let Some(key) = args.get(1) else {
                return err_lines("Usage: capsule config get <key>");
            };
            match Config::get_key(key) {
                Ok(value) => vec![OutputLine::new(
                    format!("{key} = {value}"),
                    ThemeRole::Primary,
                )],
                Err(e) => err_lines(&e.to_string()),
            }
        }
        "set" => {
            if args.len() < 3 {
                return err_lines("Usage: capsule config set <key> <value>");
            }
            let key = args[1];
            let value = args[2..].join(" ");
            let config_path = config::config_path();
            match Config::set_key(&config_path, key, &value) {
                Ok(warnings) => {
                    let mut lines = vec![OutputLine::new(
                        format!("Updated '{key}' successfully."),
                        ThemeRole::Success,
                    )];
                    for w in warnings {
                        lines.push(OutputLine::new(format!("warning: {w}"), ThemeRole::Warning));
                    }
                    lines
                }
                Err(e) => err_lines(&e.to_string()),
            }
        }
        "list" => match Config::list_as_toml() {
            Ok(text) => text
                .lines()
                .map(|l| OutputLine::new(l, ThemeRole::Primary))
                .collect(),
            Err(e) => err_lines(&e.to_string()),
        },
        "reload" => {
            let config_path = config::config_path();
            match Config::reload_global(&config_path) {
                Ok(warnings) => {
                    let mut lines = vec![OutputLine::new(
                        "Config reloaded from disk.",
                        ThemeRole::Success,
                    )];
                    for w in warnings {
                        lines.push(OutputLine::new(format!("warning: {w}"), ThemeRole::Warning));
                    }
                    lines
                }
                Err(e) => err_lines(&e.to_string()),
            }
        }
        _ => err_lines("Usage: capsule config <get|set|list|reload>"),
    }
}

#[allow(dead_code)]
fn open_config_editor(config_path: &Path) -> Result<()> {
    let editor = env::var("EDITOR").ok().filter(|v| !v.trim().is_empty());
    let executable = if let Some(e) = editor {
        e
    } else if env::consts::OS == "windows" {
        "notepad".to_string()
    } else {
        "nano".to_string()
    };
    let status = Command::new(&executable)
        .arg(config_path)
        .status()
        .map_err(|e| anyhow!("Failed launching editor '{executable}': {e}"))?;
    if status.success() {
        Ok(())
    } else {
        Err(anyhow!("Editor exited with status: {status}"))
    }
}

fn format_duration(d: Duration) -> String {
    let s = d.as_secs();
    let h = s / 3600;
    let m = (s % 3600) / 60;
    let s = s % 60;
    if h > 0 {
        format!("{h}h {m}m {s}s")
    } else if m > 0 {
        format!("{m}m {s}s")
    } else {
        format!("{s}s")
    }
}
