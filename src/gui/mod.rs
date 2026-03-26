//! GUI desktop environment for Capsule OS.
//! After the terminal boot sequence completes, `gui::run` opens a native egui window that becomes the primary interface. The old terminal REPL is embedded as a window inside the desktop.

pub mod apps;
pub mod desktop;
pub mod state;
pub mod taskbar;
pub mod theme_bridge;
pub mod window_manager;

use crate::config::Config;
use crate::fs::VirtualFs;
use crate::theme::ThemeEngine;
use anyhow::Result;
use eframe::NativeOptions;
use egui::Vec2;
use state::DesktopState;
use std::path::PathBuf;

/// Launch the egui desktop window
pub fn run(config_path: PathBuf, theme: ThemeEngine, vfs: VirtualFs) -> Result<()> {
    let options = NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_title("Capsule OS")
            .with_inner_size(Vec2::new(1280.0, 800.0))
            .with_min_inner_size(Vec2::new(800.0, 500.0)),
        ..Default::default()
    };

    let cfg = Config::snapshot().unwrap_or_default();
    let app = DesktopApp::new(config_path, theme, vfs, cfg);

    eframe::run_native("Capsule OS", options, Box::new(|_cc| Box::new(app)))
        .map_err(|e| anyhow::anyhow!("eframe error: {e}"))?;

    Ok(())
}

// App wrapper

struct DesktopApp {
    state: DesktopState,
}

impl DesktopApp {
    fn new(
        config_path: PathBuf,
        theme: ThemeEngine,
        vfs: VirtualFs,
        cfg: crate::config::Config,
    ) -> Self {
        let mut state = DesktopState::new(config_path, theme, vfs, cfg);
        // Automatically open a terminal window on startup
        state.open_terminal();
        Self { state }
    }
}

impl eframe::App for DesktopApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        // Apply theme-driven visuals every frame so hot-reload works instantly
        if let Ok(cfg) = Config::snapshot() {
            let visuals = theme_bridge::capsule_visuals(&self.state.theme, &cfg);
            ctx.set_visuals(visuals);
        }

        // Drain any hot-reload warnings
        for w in self.state.theme.take_warnings() {
            eprintln!("theme warning: {w}");
        }

        // Render layers bottom-to-top
        desktop::render(ctx, &self.state);
        taskbar::render(ctx, &mut self.state);
        window_manager::render(ctx, &mut self.state);

        // Request continuous repaint so the clock updates every second
        ctx.request_repaint_after(std::time::Duration::from_millis(500));
    }
}
