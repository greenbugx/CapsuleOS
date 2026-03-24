//! Theme engine for Capsule OS.
//! This module caches resolved colors, exposes style helpers, and hot-reloads on config changes.

use crate::config::{self, Config, ThemeConfig};
use anyhow::{anyhow, Result};
use colored::Colorize;
use notify::{EventKind, RecommendedWatcher, RecursiveMode, Watcher};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{mpsc, Arc, RwLock};
use std::thread;

#[derive(Debug, Clone, Copy)]
pub enum ThemeRole {
    Primary,
    Secondary,
    Accent,
    Success,
    Warning,
    Error,
    Muted,
    Border,
}

#[derive(Debug, Clone, Copy)]
pub struct RgbColor {
    pub r: u8,
    pub g: u8,
    pub b: u8,
}

impl RgbColor {
    pub fn parse(hex: &str) -> Option<Self> {
        if !config::is_hex_color(hex) {
            return None;
        }

        let r = u8::from_str_radix(&hex[1..3], 16).ok()?;
        let g = u8::from_str_radix(&hex[3..5], 16).ok()?;
        let b = u8::from_str_radix(&hex[5..7], 16).ok()?;
        Some(Self { r, g, b })
    }
}

#[derive(Debug, Clone)]
struct ThemeCache {
    theme_name: String,
    primary: String,
    secondary: String,
    accent: String,
    success: String,
    warning: String,
    error: String,
    muted: String,
    border: String,
}

impl ThemeCache {
    fn from_theme(theme: &ThemeConfig) -> Self {
        Self {
            theme_name: theme.name.clone(),
            primary: theme.foreground.clone(),
            secondary: theme.secondary.clone(),
            accent: theme.accent.clone(),
            success: theme.success.clone(),
            warning: theme.warning.clone(),
            error: theme.error.clone(),
            muted: theme.muted.clone(),
            border: theme.border.clone(),
        }
    }

    fn color_for(&self, role: ThemeRole) -> &str {
        match role {
            ThemeRole::Primary => &self.primary,
            ThemeRole::Secondary => &self.secondary,
            ThemeRole::Accent => &self.accent,
            ThemeRole::Success => &self.success,
            ThemeRole::Warning => &self.warning,
            ThemeRole::Error => &self.error,
            ThemeRole::Muted => &self.muted,
            ThemeRole::Border => &self.border,
        }
    }
}

#[derive(Clone)]
pub struct ThemeEngine {
    inner: Arc<ThemeEngineInner>,
}

struct ThemeEngineInner {
    config_path: PathBuf,
    cache: RwLock<ThemeCache>,
    warnings: RwLock<Vec<String>>,
    watcher_running: AtomicBool,
}

impl ThemeEngine {
    pub fn new(config_path: PathBuf) -> Result<Self> {
        let cfg = Config::snapshot()?;
        let cache = ThemeCache::from_theme(&cfg.theme);

        Ok(Self {
            inner: Arc::new(ThemeEngineInner {
                config_path,
                cache: RwLock::new(cache),
                warnings: RwLock::new(Vec::new()),
                watcher_running: AtomicBool::new(false),
            }),
        })
    }

    pub fn start_hot_reload(&self) -> Result<()> {
        if self.inner.watcher_running.swap(true, Ordering::SeqCst) {
            return Ok(());
        }

        let config_path = self.inner.config_path.clone();
        let inner = self.inner.clone();

        thread::spawn(move || {
            let (tx, rx) = mpsc::channel();
            let mut watcher = match build_watcher(tx) {
                Ok(watcher) => watcher,
                Err(err) => {
                    push_warning(&inner, format!("Theme watcher start failed: {err}"));
                    inner.watcher_running.store(false, Ordering::SeqCst);
                    return;
                }
            };

            if let Err(err) = watcher.watch(&config_path, RecursiveMode::NonRecursive) {
                push_warning(
                    &inner,
                    format!(
                        "Theme watcher failed to watch {}: {err}",
                        config_path.display()
                    ),
                );
                inner.watcher_running.store(false, Ordering::SeqCst);
                return;
            }

            loop {
                match rx.recv() {
                    Ok(Ok(event)) => {
                        if matches!(
                            event.kind,
                            EventKind::Modify(_) | EventKind::Create(_) | EventKind::Remove(_)
                        ) {
                            match Config::reload_global(&config_path) {
                                Ok(warnings) => {
                                    for warning in warnings {
                                        push_warning(&inner, warning);
                                    }
                                    if let Err(err) = refresh_cache_from_global(&inner) {
                                        push_warning(
                                            &inner,
                                            format!("Theme refresh failed after reload: {err}"),
                                        );
                                    }
                                }
                                Err(err) => push_warning(
                                    &inner,
                                    format!("Config reload failed after file change: {err}"),
                                ),
                            }
                        }
                    }
                    Ok(Err(err)) => {
                        push_warning(&inner, format!("Theme watcher event error: {err}"));
                    }
                    Err(_) => {
                        push_warning(&inner, "Theme watcher channel closed".to_string());
                        break;
                    }
                }
            }

            inner.watcher_running.store(false, Ordering::SeqCst);
        });

        Ok(())
    }

    pub fn refresh_from_config(&self) -> Result<()> {
        refresh_cache_from_global(&self.inner)
    }

    pub fn apply(&self, text: &str, role: ThemeRole) -> String {
        let color = {
            let cache = self.inner.cache.read();
            match cache {
                Ok(cache) => cache.color_for(role).to_string(),
                Err(_) => "#e0e0e0".to_string(),
            }
        };
        self.paint(text, &color)
    }

    pub fn paint(&self, text: &str, hex_color: &str) -> String {
        if let Some(rgb) = RgbColor::parse(hex_color) {
            text.truecolor(rgb.r, rgb.g, rgb.b).to_string()
        } else {
            text.to_string()
        }
    }

    pub fn paint_bg(&self, text: &str, fg_hex: &str, bg_hex: &str) -> String {
        let fg = RgbColor::parse(fg_hex);
        let bg = RgbColor::parse(bg_hex);

        match (fg, bg) {
            (Some(fg), Some(bg)) => text
                .truecolor(fg.r, fg.g, fg.b)
                .on_truecolor(bg.r, bg.g, bg.b)
                .to_string(),
            _ => text.to_string(),
        }
    }

    pub fn color_block(&self, hex_color: &str) -> String {
        self.paint("████████", hex_color)
    }

    pub fn current_theme_name(&self) -> String {
        let cache = self.inner.cache.read();
        match cache {
            Ok(cache) => cache.theme_name.clone(),
            Err(_) => "unknown".to_string(),
        }
    }

    pub fn take_warnings(&self) -> Vec<String> {
        let mut warnings = match self.inner.warnings.write() {
            Ok(guard) => guard,
            Err(_) => return vec!["Theme warning buffer lock poisoned".to_string()],
        };
        std::mem::take(&mut *warnings)
    }

    pub fn config_path(&self) -> &Path {
        &self.inner.config_path
    }
}

fn build_watcher(tx: mpsc::Sender<notify::Result<notify::Event>>) -> Result<RecommendedWatcher> {
    notify::recommended_watcher(move |res| {
        let _ = tx.send(res);
    })
    .map_err(|err| anyhow!("Failed to build file watcher: {err}"))
}

fn refresh_cache_from_global(inner: &Arc<ThemeEngineInner>) -> Result<()> {
    let cfg = Config::snapshot()?;
    let new_cache = ThemeCache::from_theme(&cfg.theme);

    let mut cache = inner
        .cache
        .write()
        .map_err(|_| anyhow!("Theme cache lock poisoned"))?;
    *cache = new_cache;
    Ok(())
}

fn push_warning(inner: &Arc<ThemeEngineInner>, warning: String) {
    if let Ok(mut warnings) = inner.warnings.write() {
        warnings.push(warning);
    }
}
