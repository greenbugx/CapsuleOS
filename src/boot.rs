//! Boot and shutdown presentation for Capsule OS.
//! This module renders startup/shutdown sequences and boot status messaging.

use crate::config::Config;
use crate::theme::{ThemeEngine, ThemeRole};
use anyhow::Result;
use crossterm::{
    cursor::MoveTo,
    execute,
    terminal::{Clear, ClearType},
};
use std::io::{self, Write};
use std::path::Path;
use std::thread;
use std::time::Duration;

pub const BOOT_MEDIA_PATH: &str = "boot/animated/Capsule_only_animate.mp4"; //TODO: Will be implemented after GUI is built - "Boot media missing" is expected

const SPINNER_FRAMES: [&str; 10] = [
    "\u{280B}", "\u{2819}", "\u{2839}", "\u{2838}", "\u{283C}", "\u{2834}", "\u{2826}", "\u{2827}",
    "\u{2807}", "\u{280F}",
];

pub fn run_boot_sequence(theme: &ThemeEngine) -> Result<()> {
    clear_screen()?;

    let cfg = Config::snapshot()?;
    print_logo(theme);
    println!();

    let version_line = format!("{} v{}", cfg.system.name, env!("CARGO_PKG_VERSION"));
    println!("{}", theme.apply(&version_line, ThemeRole::Accent));
    print_boot_media_status(theme);
    println!();

    if cfg.boot.show_post {
        print_post(theme, cfg.boot.post_delay_ms);
    }

    if cfg.boot.show_animation {
        animate_spinner(
            theme,
            "Loading app runtime...",
            16,
            cfg.boot.animation_speed_ms.max(5),
        )?;
    }

    print_step(theme, "Initializing core...")?;
    print_step(theme, "Mounting filesystem...")?;
    print_step(theme, "Loading theme engine...")?;
    print_step(theme, "Starting shell...")?;

    println!();
    if cfg.behavior.welcome_message {
        println!(
            "{}",
            theme.apply("Welcome to Capsule OS", ThemeRole::Success)
        );
    }
    println!(
        "{}",
        theme.apply(
            "\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}",
            ThemeRole::Muted
        )
    );

    Ok(())
}

pub fn run_shutdown_sequence(theme: &ThemeEngine) -> Result<()> {
    println!();
    println!(
        "{}",
        theme.apply("Shutting down Capsule OS...", ThemeRole::Warning)
    );
    animate_spinner(theme, "Saving session...", 12, 60)?;
    println!(
        "{} {}",
        theme.apply("\u{2713}", ThemeRole::Success),
        theme.apply("Shutdown complete.", ThemeRole::Primary)
    );
    Ok(())
}

pub fn clear_screen() -> io::Result<()> {
    let mut stdout = io::stdout();
    execute!(stdout, Clear(ClearType::All), MoveTo(0, 0))
}

fn print_logo(theme: &ThemeEngine) {
    let logo = [
        "  ______                                           __                   ______    ______  ",
        " /      \\                                         /  |                 /      \\  /      \\ ",
        "/$$$$$$  |  ______    ______    _______  __    __ $$ |  ______        /$$$$$$  |/$$$$$$  |",
        "$$ |  $$/  /      \\  /      \\  /       |/  |  /  |$$ | /      \\       $$ |  $$ |$$ \\__$$/ ",
        "$$ |       $$$$$$  |/$$$$$$  |/$$$$$$$/ $$ |  $$ |$$ |/$$$$$$  |      $$ |  $$ |$$      \\ ",
        "$$ |   __  /    $$ |$$ |  $$ |$$      \\ $$ |  $$ |$$ |$$    $$ |      $$ |  $$ | $$$$$$  |",
        "$$ \\__/  |/$$$$$$$ |$$ |__$$ | $$$$$$  |$$ \\__$$ |$$ |$$$$$$$$/       $$ \\__$$ |/  \\__$$ |",
        "$$    $$/ $$    $$ |$$    $$/ /     $$/ $$    $$/ $$ |$$       |      $$    $$/ $$    $$/ ",
        " $$$$$$/   $$$$$$$/ $$$$$$$/  $$$$$$$/   $$$$$$/  $$/  $$$$$$$/        $$$$$$/   $$$$$$/  ",
        "                    $$ |                                                                  ",
        "                    $$ |                                                                  ",
        "                    $$/                                                                   ",
    ];

    for line in logo {
        println!("{}", theme.apply(line, ThemeRole::Accent));
    }
}

fn print_boot_media_status(theme: &ThemeEngine) {
    if Path::new(BOOT_MEDIA_PATH).exists() {
        println!(
            "{} {}",
            theme.apply("Boot media:", ThemeRole::Primary),
            theme.apply(BOOT_MEDIA_PATH, ThemeRole::Accent)
        );
    } else {
        println!(
            "{} {}",
            theme.apply("Boot media missing:", ThemeRole::Warning),
            theme.apply(BOOT_MEDIA_PATH, ThemeRole::Error)
        );
    }
}

fn print_post(theme: &ThemeEngine, delay_ms: u64) {
    let post_lines = [
        "POST: CPU check........................................OK",
        "POST: Memory map.......................................OK",
        "POST: Virtual devices..................................OK",
        "POST: Capsule runtime..................................OK",
    ];

    for line in post_lines {
        println!("{}", theme.apply(line, ThemeRole::Muted));
        thread::sleep(Duration::from_millis(delay_ms.max(5)));
    }
}

fn print_step(theme: &ThemeEngine, label: &str) -> io::Result<()> {
    thread::sleep(Duration::from_millis(220));
    println!(
        "  {} {}",
        theme.apply(label, ThemeRole::Primary),
        theme.apply("\u{2713}", ThemeRole::Success)
    );
    io::stdout().flush()
}

fn animate_spinner(theme: &ThemeEngine, label: &str, ticks: usize, tick_ms: u64) -> io::Result<()> {
    let mut stdout = io::stdout();

    for i in 0..ticks {
        let frame = SPINNER_FRAMES[i % SPINNER_FRAMES.len()];
        print!(
            "\r  {} {}",
            theme.apply(frame, ThemeRole::Accent),
            theme.apply(label, ThemeRole::Primary)
        );
        stdout.flush()?;
        thread::sleep(Duration::from_millis(tick_ms));
    }

    print!(
        "\r  {} {}\n",
        theme.apply("\u{2713}", ThemeRole::Success),
        theme.apply(label, ThemeRole::Primary)
    );
    stdout.flush()
}
