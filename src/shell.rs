//! Interactive shell for Capsule OS.
//! This module runs the REPL, command routing, and runtime config/theme control commands.

use crate::boot;
use crate::config::{self, Config};
use crate::fs::VirtualFs;
use crate::prompt;
use crate::theme::{ThemeEngine, ThemeRole};
use anyhow::{anyhow, Result};
use crossterm::terminal;
use std::env;
use std::io::{self, Write};
use std::path::Path;
use std::process::Command;
use std::time::{Duration, Instant};

const NEOFETCH_ASCII: &str = include_str!("../boot/static/ascii-art.txt");

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

        let command_line = input.trim();
        if command_line.is_empty() {
            continue;
        }

        let expanded = expand_alias(command_line)?;
        let should_continue = handle_command(&expanded, &theme, &mut vfs, start_time)?;
        if !should_continue {
            break;
        }
    }

    Ok(())
}

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
        if rest.is_empty() {
            Ok(expanded.clone())
        } else {
            Ok(format!("{expanded} {rest}"))
        }
    } else {
        Ok(command_line.to_string())
    }
}

fn handle_command(
    command_line: &str,
    theme: &ThemeEngine,
    vfs: &mut VirtualFs,
    start_time: Instant,
) -> Result<bool> {
    let mut parts = command_line.split_whitespace();
    let Some(command) = parts.next() else {
        return Ok(true);
    };

    match command {
        "help" => print_help(theme),
        "ls" => {
            let path = parts.next();
            if parts.next().is_some() {
                print_error(theme, "Usage: ls [path]");
            } else {
                match vfs.ls(path) {
                    Ok(entries) => {
                        if entries.is_empty() {
                            println!("{}", theme.apply("(empty directory)", ThemeRole::Muted));
                        } else {
                            for entry in entries {
                                if entry.is_dir {
                                    println!(
                                        "{}",
                                        theme.apply(&format!("{}/", entry.name), ThemeRole::Accent)
                                    );
                                } else {
                                    println!("{}", theme.apply(&entry.name, ThemeRole::Primary));
                                }
                            }
                        }
                    }
                    Err(err) => print_error(theme, &err),
                }
            }
        }
        "cd" => {
            let Some(path) = parts.next() else {
                print_error(theme, "Usage: cd <path>");
                return Ok(true);
            };
            if parts.next().is_some() {
                print_error(theme, "Usage: cd <path>");
            } else if let Err(err) = vfs.cd(path) {
                print_error(theme, &err);
            }
        }
        "mkdir" => {
            let Some(path) = parts.next() else {
                print_error(theme, "Usage: mkdir <name>");
                return Ok(true);
            };
            if parts.next().is_some() {
                print_error(theme, "Usage: mkdir <name>");
            } else if let Err(err) = vfs.mkdir(path) {
                print_error(theme, &err);
            }
        }
        "touch" => {
            let Some(path) = parts.next() else {
                print_error(theme, "Usage: touch <name>");
                return Ok(true);
            };
            if parts.next().is_some() {
                print_error(theme, "Usage: touch <name>");
            } else if let Err(err) = vfs.touch(path) {
                print_error(theme, &err);
            }
        }
        "cat" => {
            let Some(path) = parts.next() else {
                print_error(theme, "Usage: cat <file>");
                return Ok(true);
            };
            if parts.next().is_some() {
                print_error(theme, "Usage: cat <file>");
            } else {
                match vfs.cat(path) {
                    Ok(contents) => {
                        if contents.is_empty() {
                            println!("{}", theme.apply("(empty file)", ThemeRole::Muted));
                        } else {
                            print!("{}", theme.apply(&contents, ThemeRole::Primary));
                            if !contents.ends_with('\n') {
                                println!();
                            }
                        }
                    }
                    Err(err) => print_error(theme, &err),
                }
            }
        }
        "clear" => {
            boot::clear_screen()?;
        }
        "neofetch" => print_neofetch(theme, vfs, start_time)?,
        "capsule" => {
            let args = parts.collect::<Vec<_>>();
            handle_capsule_command(theme, args)?;
        }
        "shutdown" => {
            if confirm_shutdown(theme)? {
                boot::run_shutdown_sequence(theme)?;
                return Ok(false);
            }
        }
        _ => print_error(
            theme,
            "Unknown command. Run `help` to list available commands.",
        ),
    }

    Ok(true)
}

fn handle_capsule_command(theme: &ThemeEngine, args: Vec<&str>) -> Result<()> {
    if args.is_empty() {
        print_error(theme, "Usage: capsule <install|theme|config> ...");
        return Ok(());
    }

    match args[0] {
        "install" => {
            if args.len() < 2 {
                print_error(theme, "Usage: capsule install <package>");
                return Ok(());
            }
            let package = args[1..].join(" ");
            println!(
                "{}",
                theme.apply(
                    "Capsule package manager is in skeleton mode.",
                    ThemeRole::Warning
                )
            );
            println!(
                "{} {}",
                theme.apply("Requested package:", ThemeRole::Primary),
                theme.apply(&package, ThemeRole::Accent)
            );
            println!(
                "{}",
                theme.apply(
                    "`capsule install` will be fully wired in Phase 6.",
                    ThemeRole::Muted
                )
            );
        }
        "theme" => handle_theme_command(theme, &args[1..])?,
        "config" => handle_config_command(theme, &args[1..])?,
        _ => print_error(theme, "Usage: capsule <install|theme|config> ..."),
    }

    Ok(())
}

fn handle_theme_command(theme: &ThemeEngine, args: &[&str]) -> Result<()> {
    if args.is_empty() {
        print_error(theme, "Usage: capsule theme <list|set|show|edit|reset>");
        return Ok(());
    }

    match args[0] {
        "list" => {
            let current = theme.current_theme_name();
            println!("{}", theme.apply("Available themes:", ThemeRole::Secondary));
            for item in config::Config::available_theme_names() {
                let marker = if item == current { "\u{2713}" } else { " " };
                println!(
                    "{} {}",
                    theme.apply(marker, ThemeRole::Success),
                    theme.apply(&item, ThemeRole::Primary)
                );
            }
        }
        "set" => {
            let Some(name) = args.get(1) else {
                print_error(theme, "Usage: capsule theme set <name>");
                return Ok(());
            };
            let warnings = Config::set_theme(theme.config_path(), name)?;
            theme.refresh_from_config()?;
            println!(
                "{} {}",
                theme.apply("Switched theme to", ThemeRole::Success),
                theme.apply(name, ThemeRole::Accent)
            );
            print_warnings(theme, warnings);
        }
        "show" => {
            let cfg = Config::snapshot()?;
            println!(
                "{} {}",
                theme.apply("Capsule OS - current theme:", ThemeRole::Primary),
                theme.apply(&cfg.theme.name, ThemeRole::Accent)
            );
            println!();
            print_theme_swatch(theme, "background", &cfg.theme.background);
            print_theme_swatch(theme, "foreground", &cfg.theme.foreground);
            print_theme_swatch(theme, "accent", &cfg.theme.accent);
            print_theme_swatch(theme, "secondary", &cfg.theme.secondary);
            print_theme_swatch(theme, "success", &cfg.theme.success);
            print_theme_swatch(theme, "warning", &cfg.theme.warning);
            print_theme_swatch(theme, "error", &cfg.theme.error);
            print_theme_swatch(theme, "muted", &cfg.theme.muted);
            print_theme_swatch(theme, "border", &cfg.theme.border);
            print_theme_swatch(theme, "selection", &cfg.theme.selection);
        }
        "edit" => {
            open_config_editor(theme.config_path())?;
            let warnings = Config::reload_global(theme.config_path())?;
            theme.refresh_from_config()?;
            print_warnings(theme, warnings);
            println!(
                "{}",
                theme.apply(
                    "Configuration reloaded after editor close.",
                    ThemeRole::Success
                )
            );
        }
        "reset" => {
            let warnings = Config::reset_theme_to_default(theme.config_path())?;
            theme.refresh_from_config()?;
            print_warnings(theme, warnings);
            println!(
                "{}",
                theme.apply("Theme reset to default-dark.", ThemeRole::Success)
            );
        }
        _ => print_error(theme, "Usage: capsule theme <list|set|show|edit|reset>"),
    }

    Ok(())
}

fn handle_config_command(theme: &ThemeEngine, args: &[&str]) -> Result<()> {
    if args.is_empty() {
        print_error(theme, "Usage: capsule config <get|set|list|reload>");
        return Ok(());
    }

    match args[0] {
        "get" => {
            let Some(key) = args.get(1) else {
                print_error(theme, "Usage: capsule config get <key>");
                return Ok(());
            };
            match Config::get_key(key) {
                Ok(value) => println!("{} = {}", theme.apply(key, ThemeRole::Accent), value),
                Err(err) => print_error(theme, &err.to_string()),
            }
        }
        "set" => {
            if args.len() < 3 {
                print_error(theme, "Usage: capsule config set <key> <value>");
                return Ok(());
            }
            let key = args[1];
            let value = args[2..].join(" ");
            match Config::set_key(theme.config_path(), key, &value) {
                Err(err) => {
                    print_error(theme, &err.to_string());
                }
                Ok(warnings) => {
                    theme.refresh_from_config()?;
                    print_warnings(theme, warnings);
                    println!(
                        "{} {} {}",
                        theme.apply("Updated", ThemeRole::Success),
                        theme.apply(key, ThemeRole::Accent),
                        theme.apply("successfully.", ThemeRole::Success)
                    );
                }
            }
        }
        "list" => {
            let listing = Config::list_as_toml()?;
            println!("{}", theme.apply(&listing, ThemeRole::Primary));
        }
        "reload" => {
            let warnings = Config::reload_global(theme.config_path())?;
            theme.refresh_from_config()?;
            print_warnings(theme, warnings);
            println!(
                "{}",
                theme.apply("Config reloaded from disk.", ThemeRole::Success)
            );
        }
        _ => print_error(theme, "Usage: capsule config <get|set|list|reload>"),
    }

    Ok(())
}

fn open_config_editor(config_path: &Path) -> Result<()> {
    let editor = env::var("EDITOR")
        .ok()
        .filter(|value| !value.trim().is_empty());

    let executable = if let Some(editor) = editor {
        editor
    } else if env::consts::OS == "windows" {
        "notepad".to_string()
    } else {
        "nano".to_string()
    };

    let status = Command::new(&executable)
        .arg(config_path)
        .status()
        .map_err(|err| anyhow!("Failed launching editor '{executable}': {err}"))?;

    if status.success() {
        Ok(())
    } else {
        Err(anyhow!("Editor exited with status: {status}"))
    }
}

fn confirm_shutdown(theme: &ThemeEngine) -> Result<bool> {
    let cfg = Config::snapshot()?;
    if !cfg.behavior.confirm_shutdown {
        return Ok(true);
    }

    print!(
        "{}",
        theme.apply("Confirm shutdown? [y/N]: ", ThemeRole::Warning)
    );
    io::stdout().flush()?;

    let mut response = String::new();
    io::stdin().read_line(&mut response)?;
    let normalized = response.trim().to_lowercase();
    Ok(normalized == "y" || normalized == "yes")
}

fn print_help(theme: &ThemeEngine) {
    println!("{}", theme.apply("Capsule OS Commands", ThemeRole::Accent));
    println!(
        "{}",
        theme.apply("help                  list commands", ThemeRole::Primary)
    );
    println!(
        "{}",
        theme.apply("ls [path]             list files", ThemeRole::Primary)
    );
    println!(
        "{}",
        theme.apply("cd <path>             change directory", ThemeRole::Primary)
    );
    println!(
        "{}",
        theme.apply("mkdir <name>          create folder", ThemeRole::Primary)
    );
    println!(
        "{}",
        theme.apply("touch <name>          create file", ThemeRole::Primary)
    );
    println!(
        "{}",
        theme.apply("cat <file>            read file", ThemeRole::Primary)
    );
    println!(
        "{}",
        theme.apply("clear                 clear screen", ThemeRole::Primary)
    );
    println!(
        "{}",
        theme.apply("neofetch              show system info", ThemeRole::Primary)
    );
    println!(
        "{}",
        theme.apply(
            "capsule install <pkg> package manager skeleton",
            ThemeRole::Primary
        )
    );
    println!(
        "{}",
        theme.apply("capsule theme ...     theme manager", ThemeRole::Primary)
    );
    println!(
        "{}",
        theme.apply("capsule config ...    config manager", ThemeRole::Primary)
    );
    println!(
        "{}",
        theme.apply("shutdown              exit Capsule OS", ThemeRole::Primary)
    );
}

fn print_neofetch(theme: &ThemeEngine, vfs: &VirtualFs, start_time: Instant) -> Result<()> {
    let cfg = Config::snapshot()?;
    let uptime = format_duration(start_time.elapsed());
    let host_home = dirs::home_dir()
        .map(|path| path.display().to_string())
        .unwrap_or_else(|| "unavailable".to_string());

    print_neofetch_ascii(theme);
    println!(
        "{} {}",
        theme.apply("OS:", ThemeRole::Primary),
        theme.apply(&cfg.system.name, ThemeRole::Accent)
    );
    println!(
        "{} {}",
        theme.apply("Version:", ThemeRole::Primary),
        theme.apply(env!("CARGO_PKG_VERSION"), ThemeRole::Primary)
    );
    println!(
        "{} {}",
        theme.apply("User:", ThemeRole::Primary),
        theme.apply(&cfg.system.username, ThemeRole::Primary)
    );
    println!(
        "{} {}",
        theme.apply("Host:", ThemeRole::Primary),
        theme.apply(&cfg.system.hostname, ThemeRole::Primary)
    );
    println!(
        "{} {}",
        theme.apply("Shell CWD:", ThemeRole::Primary),
        theme.apply(&vfs.cwd(), ThemeRole::Primary)
    );
    println!(
        "{} {}",
        theme.apply("VFS Root:", ThemeRole::Primary),
        theme.apply(&vfs.host_root().display().to_string(), ThemeRole::Primary)
    );
    println!(
        "{} {}",
        theme.apply("Uptime:", ThemeRole::Primary),
        theme.apply(&uptime, ThemeRole::Primary)
    );
    println!(
        "{} {}",
        theme.apply("Host Home:", ThemeRole::Primary),
        theme.apply(&host_home, ThemeRole::Primary)
    );
    println!(
        "{} {}",
        theme.apply("Theme:", ThemeRole::Primary),
        theme.apply(&cfg.theme.name, ThemeRole::Accent)
    );
    println!(
        "{} {}",
        theme.apply("Prompt Style:", ThemeRole::Primary),
        theme.apply(&cfg.shell.prompt_style, ThemeRole::Primary)
    );

    Ok(())
}

fn print_neofetch_ascii(theme: &ThemeEngine) {
    let mut lines: Vec<String> = NEOFETCH_ASCII
        .lines()
        .map(ToString::to_string)
        .collect();

    trim_empty_vertical_padding(&mut lines);

    let dedent = common_leading_spaces(&lines);
    let (terminal_width, _) = terminal::size().unwrap_or((100, 30));
    let max_width = usize::from(terminal_width.saturating_sub(4)).max(20);
    let left_padding = "  ";

    for raw_line in lines {
        let dedented = if raw_line.len() >= dedent {
            &raw_line[dedent..]
        } else {
            raw_line.as_str()
        };

        let fitted = fit_line_to_width(dedented, max_width);

        println!(
            "{}",
            theme.apply(&format!("{left_padding}{fitted}"), ThemeRole::Accent)
        );
    }

    let _ascii = if NEOFETCH_ASCII.trim().is_empty() {
        "Capsule OS"
    } else {
        NEOFETCH_ASCII
    };
}

fn print_theme_swatch(theme: &ThemeEngine, key: &str, hex: &str) {
    let key_role = if key == "border" {
        ThemeRole::Border
    } else {
        ThemeRole::Primary
    };
    println!(
        "  {:<11} {:<9} {}",
        theme.apply(key, key_role),
        theme.apply(hex, ThemeRole::Muted),
        theme.color_block(hex)
    );
}

fn print_warnings(theme: &ThemeEngine, warnings: Vec<String>) {
    for warning in warnings {
        println!(
            "{}",
            theme.apply(&format!("warning: {warning}"), ThemeRole::Warning)
        );
    }
}

fn print_error(theme: &ThemeEngine, message: &str) {
    println!("{}", theme.apply(message, ThemeRole::Error));
}

fn trim_empty_vertical_padding(lines: &mut Vec<String>) {
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
}

fn common_leading_spaces(lines: &[String]) -> usize {
    lines
        .iter()
        .filter(|line| !line.trim().is_empty())
        .map(|line| line.chars().take_while(|ch| *ch == ' ').count())
        .min()
        .unwrap_or(0)
}

fn fit_line_to_width(line: &str, max_width: usize) -> String {
    let chars: Vec<char> = line.chars().collect();
    if chars.len() <= max_width {
        return line.to_string();
    }

    let step = ((chars.len() as f32) / (max_width as f32)).ceil() as usize;
    chars
        .into_iter()
        .enumerate()
        .filter_map(|(index, ch)| if index % step == 0 { Some(ch) } else { None })
        .collect()
}

fn format_duration(duration: Duration) -> String {
    let total_seconds = duration.as_secs();
    let hours = total_seconds / 3_600;
    let minutes = (total_seconds % 3_600) / 60;
    let seconds = total_seconds % 60;

    if hours > 0 {
        format!("{hours}h {minutes}m {seconds}s")
    } else if minutes > 0 {
        format!("{minutes}m {seconds}s")
    } else {
        format!("{seconds}s")
    }
}