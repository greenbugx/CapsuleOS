//! Central state for the Capsule OS desktop environment.
//! `DesktopState` owns every piece of runtime data that the GUI needs: the theme engine, open windows, taskbar metadata, the virtual filesystem, and the terminal input/output buffer.

use crate::config::Config;
use crate::fs::VirtualFs;
use crate::shell::OutputLine;
use crate::theme::ThemeEngine;
use egui::{Pos2, Vec2};
use std::path::PathBuf;
use std::time::Instant;

// App type enum

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AppType {
    Terminal,
    FileManager,
    Settings,
    About,
}

impl AppType {
    pub fn icon(&self) -> &'static str {
        match self {
            AppType::Terminal => "⬛",
            AppType::FileManager => "📁",
            AppType::Settings => "⚙",
            AppType::About => "ℹ",
        }
    }

    pub fn title(&self) -> &'static str {
        match self {
            AppType::Terminal => "Terminal",
            AppType::FileManager => "File Manager",
            AppType::Settings => "Settings",
            AppType::About => "About",
        }
    }
}

// AppWindow

/// A floating window open on the desktop
#[derive(Debug, Clone)]
pub struct AppWindow {
    pub id: usize,
    pub app_type: AppType,
    pub is_minimized: bool,
    pub is_maximized: bool,
    pub pending_restore: bool,
    pub z_order: usize,
    pub initial_pos: Pos2,
    pub initial_size: Vec2,
}

impl AppWindow {
    pub fn egui_id(&self) -> egui::Id {
        egui::Id::new(format!("appwindow_{}", self.id))
    }

    pub fn window_title(&self) -> String {
        format!("{} {}", self.app_type.icon(), self.app_type.title())
    }
}

// Terminal state

/// Runtime state for the embedded terminal
#[derive(Debug, Clone)]
pub struct TerminalState {
    pub output: Vec<OutputLine>,
    pub input: String,
    pub cwd: String,
    pub start_time: Instant,
    pub scroll_to_bottom: bool,
}

impl Default for TerminalState {
    fn default() -> Self {
        Self {
            output: vec![OutputLine::new(
                "Capsule OS Terminal — type `help` for available commands.",
                crate::theme::ThemeRole::Accent,
            )],
            input: String::new(),
            cwd: "~".to_string(),
            start_time: Instant::now(),
            scroll_to_bottom: true,
        }
    }
}

// Taskbar state

#[derive(Debug, Default)]
pub struct TaskbarState {
    pub launcher_open: bool,
    pub focused_id: Option<usize>,
}

// File manager state

#[derive(Debug, Clone, Default)]
pub struct FileManagerState {
    pub entries: Vec<crate::fs::FsEntry>,
    pub dirty: bool,
}

// Desktop state

/// Top-level state struct passed around the GUI render functions.
pub struct DesktopState {
    pub config_path: PathBuf,
    pub theme: ThemeEngine,
    pub cfg: Config,
    pub windows: Vec<AppWindow>,
    pub taskbar: TaskbarState,
    pub next_window_id: usize,
    pub vfs: VirtualFs,
    pub terminal: TerminalState,
    pub file_manager: FileManagerState,
    pub shutdown_requested: bool,
}

impl DesktopState {
    pub fn new(config_path: PathBuf, theme: ThemeEngine, vfs: VirtualFs, cfg: Config) -> Self {
        Self {
            config_path,
            theme,
            cfg,
            windows: Vec::new(),
            taskbar: TaskbarState::default(),
            next_window_id: 1,
            vfs,
            terminal: TerminalState::default(),
            file_manager: FileManagerState {
                dirty: true,
                ..Default::default()
            },
            shutdown_requested: false,
        }
    }

    /// Open a new window, cascading slightly from previous windows
    pub fn open_window(&mut self, app_type: AppType) -> usize {
        let id = self.next_window_id;
        self.next_window_id += 1;

        // Cascade: each window is offset 24px from the last.
        let offset = ((id as f32 - 1.0) * 24.0).min(200.0);
        let pos = Pos2::new(80.0 + offset, 80.0 + offset);
        let size = match app_type {
            AppType::Terminal => Vec2::new(700.0, 420.0),
            AppType::FileManager => Vec2::new(640.0, 460.0),
            AppType::Settings => Vec2::new(520.0, 480.0),
            AppType::About => Vec2::new(400.0, 300.0),
        };

        let max_z = self.windows.iter().map(|w| w.z_order).max().unwrap_or(0);

        self.windows.push(AppWindow {
            id,
            app_type,
            is_minimized: false,
            is_maximized: false,
            pending_restore: false,
            z_order: max_z + 1,
            initial_pos: pos,
            initial_size: size,
        });

        self.taskbar.focused_id = Some(id);
        id
    }

    /// Convenience: open a terminal window
    pub fn open_terminal(&mut self) -> usize {
        // If one already exists and is minimized, restore it
        if let Some(w) = self
            .windows
            .iter_mut()
            .find(|w| w.app_type == AppType::Terminal)
        {
            w.is_minimized = false;
            let id = w.id;
            self.focus_window(id);
            return id;
        }
        self.open_window(AppType::Terminal)
    }

    /// Bring a window to the front
    pub fn focus_window(&mut self, id: usize) {
        let max_z = self.windows.iter().map(|w| w.z_order).max().unwrap_or(0);
        if let Some(w) = self.windows.iter_mut().find(|w| w.id == id) {
            w.z_order = max_z + 1;
            w.is_minimized = false;
        }
        self.taskbar.focused_id = Some(id);
    }

    /// Close (remove) a window by id
    pub fn close_window(&mut self, id: usize) {
        self.windows.retain(|w| w.id != id);
        if self.taskbar.focused_id == Some(id) {
            self.taskbar.focused_id = self.windows.last().map(|w| w.id);
        }
    }

    /// Refresh the cached config snapshot
    pub fn refresh_cfg(&mut self) {
        if let Ok(c) = Config::snapshot() {
            self.cfg = c;
        }
    }
}
