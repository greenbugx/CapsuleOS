//! Capsule OS entry point.
//! Initializes config/theme services, runs the terminal boot sequence, then hands off to the egui GUI desktop environment.

mod boot;
mod config;
mod fs;
mod gui;
mod prompt;
mod shell;
mod theme;

use anyhow::Result;
use config::Config;
use fs::VirtualFs;
use theme::ThemeEngine;

fn main() {
    if let Err(error) = run() {
        eprintln!("Capsule OS failed: {error}");
        std::process::exit(1);
    }
}

fn run() -> Result<()> {
    let config_path = config::config_path();
    let init_warnings = Config::init_global(&config_path)?;

    let theme_engine = ThemeEngine::new(config_path.clone())?;
    theme_engine.start_hot_reload()?;

    for warning in init_warnings {
        println!(
            "{}",
            theme_engine.apply(&format!("warning: {warning}"), theme::ThemeRole::Warning)
        );
    }

    let vfs = VirtualFs::new("runtime").map_err(|err| anyhow::anyhow!(err))?;

    boot::run_boot_sequence(&theme_engine)?;
    gui::run(config_path, theme_engine, vfs)?;

    Ok(())
}
