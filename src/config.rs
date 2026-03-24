//! Configuration manager for Capsule OS.
//! This module loads, validates, mutates, and persists the global runtime configuration.

use anyhow::{anyhow, Context, Result};
use include_dir::{include_dir, Dir};
use once_cell::sync::OnceCell;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::{Arc, RwLock};

static BUILTIN_THEMES: Dir = include_dir!("$CARGO_MANIFEST_DIR/themes");

static CONFIG_STORE: OnceCell<Arc<RwLock<Config>>> = OnceCell::new();

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct Config {
    pub system: SystemConfig,
    pub theme: ThemeConfig,
    pub shell: ShellConfig,
    pub boot: BootConfig,
    pub font: FontConfig,
    pub behavior: BehaviorConfig,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            system: SystemConfig::default(),
            theme: ThemeConfig::default(),
            shell: ShellConfig::default(),
            boot: BootConfig::default(),
            font: FontConfig::default(),
            behavior: BehaviorConfig::default(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct SystemConfig {
    pub name: String,
    pub username: String,
    pub hostname: String,
    pub language: String,
}

impl Default for SystemConfig {
    fn default() -> Self {
        Self {
            name: "Capsule OS".to_string(),
            username: "dev".to_string(),
            hostname: "capsule".to_string(),
            language: "en".to_string(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct ThemeConfig {
    pub name: String,
    pub background: String,
    pub foreground: String,
    pub accent: String,
    pub secondary: String,
    pub success: String,
    pub warning: String,
    pub error: String,
    pub muted: String,
    pub border: String,
    pub selection: String,
    pub cursor: ThemeCursorConfig,
    pub syntax: ThemeSyntaxConfig,
}

impl Default for ThemeConfig {
    fn default() -> Self {
        Self {
            name: "default-dark".to_string(),
            background: "#0d0d0d".to_string(),
            foreground: "#e0e0e0".to_string(),
            accent: "#89b4fa".to_string(),
            secondary: "#cba6f7".to_string(),
            success: "#a6e3a1".to_string(),
            warning: "#fab387".to_string(),
            error: "#f38ba8".to_string(),
            muted: "#6c7086".to_string(),
            border: "#313244".to_string(),
            selection: "#45475a".to_string(),
            cursor: ThemeCursorConfig::default(),
            syntax: ThemeSyntaxConfig::default(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct ThemeCursorConfig {
    pub shape: String,
    pub blink: bool,
    pub color: String,
}

impl Default for ThemeCursorConfig {
    fn default() -> Self {
        Self {
            shape: "block".to_string(),
            blink: true,
            color: "#89b4fa".to_string(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct ThemeSyntaxConfig {
    pub keyword: String,
    pub string: String,
    pub number: String,
    pub comment: String,
}

impl Default for ThemeSyntaxConfig {
    fn default() -> Self {
        Self {
            keyword: "#cba6f7".to_string(),
            string: "#a6e3a1".to_string(),
            number: "#fab387".to_string(),
            comment: "#6c7086".to_string(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct ShellConfig {
    pub prompt_style: String,
    pub show_git: bool,
    pub show_time: bool,
    pub history_size: usize,
    pub aliases: HashMap<String, String>,
}

impl Default for ShellConfig {
    fn default() -> Self {
        let mut aliases = HashMap::new();
        aliases.insert("ll".to_string(), "ls -la".to_string());
        aliases.insert("h".to_string(), "help".to_string());

        Self {
            prompt_style: "arrow".to_string(),
            show_git: false,
            show_time: false,
            history_size: 1000,
            aliases,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct BootConfig {
    pub show_animation: bool,
    pub animation_type: String,
    pub animation_speed_ms: u64,
    pub show_post: bool,
    pub post_delay_ms: u64,
}

impl Default for BootConfig {
    fn default() -> Self {
        Self {
            show_animation: true,
            animation_type: "text".to_string(),
            animation_speed_ms: 40,
            show_post: true,
            post_delay_ms: 60,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct FontConfig {
    pub family: String,
    pub size: u16,
    pub bold_ui: bool,
}

impl Default for FontConfig {
    fn default() -> Self {
        Self {
            family: "monospace".to_string(),
            size: 14,
            bold_ui: false,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct BehaviorConfig {
    pub autosave_config: bool,
    pub confirm_shutdown: bool,
    pub welcome_message: bool,
}

impl Default for BehaviorConfig {
    fn default() -> Self {
        Self {
            autosave_config: true,
            confirm_shutdown: true,
            welcome_message: true,
        }
    }
}

#[derive(Debug, Clone, Deserialize)]
#[serde(default)]
struct ThemePresetDocument {
    theme: ThemePreset,
}

impl Default for ThemePresetDocument {
    fn default() -> Self {
        Self {
            theme: ThemePreset::default(),
        }
    }
}

#[derive(Debug, Clone, Deserialize, Default)]
#[serde(default)]
struct ThemePreset {
    name: Option<String>,
    background: Option<String>,
    foreground: Option<String>,
    accent: Option<String>,
    secondary: Option<String>,
    success: Option<String>,
    warning: Option<String>,
    error: Option<String>,
    muted: Option<String>,
    border: Option<String>,
    selection: Option<String>,
}

impl Config {
    pub fn load(path: &Path) -> Result<(Self, Vec<String>)> {
        if !path.exists() {
            let default_cfg = Config::default();
            default_cfg.save(path)?;
        }

        let raw = fs::read_to_string(path)
            .with_context(|| format!("Failed reading {}", path.display()))?;
        let mut cfg: Config =
            toml::from_str(&raw).with_context(|| format!("Failed parsing {}", path.display()))?;

        let mut warnings = Vec::new();
        warnings.extend(cfg.merge_preset_if_available(path.parent().unwrap_or(Path::new("."))));
        warnings.extend(cfg.validate_and_fix_colors());
        cfg.normalize_misc_fields(&mut warnings);

        Ok((cfg, warnings))
    }

    pub fn save(&self, path: &Path) -> Result<()> {
        let text = toml::to_string_pretty(self).context("Failed serializing config")?;
        fs::write(path, format!("{text}\n"))
            .with_context(|| format!("Failed writing {}", path.display()))?;
        Ok(())
    }

    pub fn get() -> Result<Arc<RwLock<Config>>> {
        CONFIG_STORE
            .get()
            .cloned()
            .ok_or_else(|| anyhow!("Global config is not initialized"))
    }

    pub fn init_global(path: &Path) -> Result<Vec<String>> {
        let (cfg, warnings) = Config::load(path)?;

        if let Some(store) = CONFIG_STORE.get() {
            let mut guard = store
                .write()
                .map_err(|_| anyhow!("Config lock poisoned during init"))?;
            *guard = cfg;
            return Ok(warnings);
        }

        let store = Arc::new(RwLock::new(cfg));
        let _ = CONFIG_STORE.set(store);
        Ok(warnings)
    }

    pub fn snapshot() -> Result<Config> {
        let store = Config::get()?;
        let guard = store
            .read()
            .map_err(|_| anyhow!("Config lock poisoned while reading"))?;
        Ok(guard.clone())
    }

    pub fn reload_global(path: &Path) -> Result<Vec<String>> {
        let (cfg, warnings) = Config::load(path)?;
        let store = Config::get()?;
        let mut guard = store
            .write()
            .map_err(|_| anyhow!("Config lock poisoned while reloading"))?;
        *guard = cfg;
        Ok(warnings)
    }

    pub fn list_as_toml() -> Result<String> {
        let cfg = Config::snapshot()?;
        toml::to_string_pretty(&cfg).context("Failed converting config to TOML")
    }

    pub fn get_key(key: &str) -> Result<String> {
        let cfg = Config::snapshot()?;
        match key {
            "system.name" => Ok(cfg.system.name),
            "system.username" => Ok(cfg.system.username),
            "system.hostname" => Ok(cfg.system.hostname),
            "system.language" => Ok(cfg.system.language),
            "theme.name" => Ok(cfg.theme.name),
            "theme.background" => Ok(cfg.theme.background),
            "theme.foreground" => Ok(cfg.theme.foreground),
            "theme.accent" => Ok(cfg.theme.accent),
            "theme.secondary" => Ok(cfg.theme.secondary),
            "theme.success" => Ok(cfg.theme.success),
            "theme.warning" => Ok(cfg.theme.warning),
            "theme.error" => Ok(cfg.theme.error),
            "theme.muted" => Ok(cfg.theme.muted),
            "theme.border" => Ok(cfg.theme.border),
            "theme.selection" => Ok(cfg.theme.selection),
            "theme.cursor.shape" => Ok(cfg.theme.cursor.shape),
            "theme.cursor.blink" => Ok(cfg.theme.cursor.blink.to_string()),
            "theme.cursor.color" => Ok(cfg.theme.cursor.color),
            "theme.syntax.keyword" => Ok(cfg.theme.syntax.keyword),
            "theme.syntax.string" => Ok(cfg.theme.syntax.string),
            "theme.syntax.number" => Ok(cfg.theme.syntax.number),
            "theme.syntax.comment" => Ok(cfg.theme.syntax.comment),
            "shell.prompt_style" => Ok(cfg.shell.prompt_style),
            "shell.show_git" => Ok(cfg.shell.show_git.to_string()),
            "shell.show_time" => Ok(cfg.shell.show_time.to_string()),
            "shell.history_size" => Ok(cfg.shell.history_size.to_string()),
            "boot.show_animation" => Ok(cfg.boot.show_animation.to_string()),
            "boot.animation_type" => Ok(cfg.boot.animation_type),
            "boot.animation_speed_ms" => Ok(cfg.boot.animation_speed_ms.to_string()),
            "boot.show_post" => Ok(cfg.boot.show_post.to_string()),
            "boot.post_delay_ms" => Ok(cfg.boot.post_delay_ms.to_string()),
            "font.family" => Ok(cfg.font.family),
            "font.size" => Ok(cfg.font.size.to_string()),
            "font.bold_ui" => Ok(cfg.font.bold_ui.to_string()),
            "behavior.autosave_config" => Ok(cfg.behavior.autosave_config.to_string()),
            "behavior.confirm_shutdown" => Ok(cfg.behavior.confirm_shutdown.to_string()),
            "behavior.welcome_message" => Ok(cfg.behavior.welcome_message.to_string()),
            _ if key.starts_with("shell.aliases.") => {
                let alias = key.trim_start_matches("shell.aliases.");
                cfg.shell
                    .aliases
                    .get(alias)
                    .cloned()
                    .ok_or_else(|| anyhow!("Unknown alias key: {key}"))
            }
            _ => Err(anyhow!("{}", unknown_key_error(key))),
        }
    }

    pub fn set_key(path: &Path, key: &str, value: &str) -> Result<Vec<String>> {
        let store = Config::get()?;
        {
            let mut cfg = store
                .write()
                .map_err(|_| anyhow!("Config lock poisoned while updating"))?;

            match key {
                "system.name" => cfg.system.name = value.to_string(),
                "system.username" => cfg.system.username = value.to_string(),
                "system.hostname" => cfg.system.hostname = value.to_string(),
                "system.language" => cfg.system.language = value.to_string(),
                "theme.name" => cfg.theme.name = value.to_string(),
                "theme.background" => cfg.theme.background = value.to_string(),
                "theme.foreground" => cfg.theme.foreground = value.to_string(),
                "theme.accent" => cfg.theme.accent = value.to_string(),
                "theme.secondary" => cfg.theme.secondary = value.to_string(),
                "theme.success" => cfg.theme.success = value.to_string(),
                "theme.warning" => cfg.theme.warning = value.to_string(),
                "theme.error" => cfg.theme.error = value.to_string(),
                "theme.muted" => cfg.theme.muted = value.to_string(),
                "theme.border" => cfg.theme.border = value.to_string(),
                "theme.selection" => cfg.theme.selection = value.to_string(),
                "theme.cursor.shape" => {
                    const VALID: &[&str] = &["block", "underline", "bar"];
                    if !VALID.contains(&value) {
                        return Err(anyhow!(
                            "Invalid value '{}' for theme.cursor.shape. Valid options: {}",
                            value,
                            VALID.join(", ")
                        ));
                    }
                    cfg.theme.cursor.shape = value.to_string();
                }
                "theme.cursor.blink" => cfg.theme.cursor.blink = parse_bool(value, key)?,
                "theme.cursor.color" => cfg.theme.cursor.color = value.to_string(),
                "theme.syntax.keyword" => cfg.theme.syntax.keyword = value.to_string(),
                "theme.syntax.string" => cfg.theme.syntax.string = value.to_string(),
                "theme.syntax.number" => cfg.theme.syntax.number = value.to_string(),
                "theme.syntax.comment" => cfg.theme.syntax.comment = value.to_string(),
                "shell.prompt_style" => {
                    const VALID: &[&str] = &["arrow", "minimal", "powerline", "classic"];
                    if !VALID.contains(&value) {
                        return Err(anyhow!(
                            "Invalid value '{}' for shell.prompt_style. Valid options: {}",
                            value,
                            VALID.join(", ")
                        ));
                    }
                    cfg.shell.prompt_style = value.to_string();
                }
                "shell.show_git" => cfg.shell.show_git = parse_bool(value, key)?,
                "shell.show_time" => cfg.shell.show_time = parse_bool(value, key)?,
                "shell.history_size" => {
                    cfg.shell.history_size = value
                        .parse::<usize>()
                        .with_context(|| format!("Invalid usize for {key}"))?
                }
                "boot.show_animation" => cfg.boot.show_animation = parse_bool(value, key)?,
                "boot.animation_type" => {
                    const VALID: &[&str] = &["text", "gif"];
                    if !VALID.contains(&value) {
                        return Err(anyhow!(
                            "Invalid value '{}' for boot.animation_type. Valid options: {}",
                            value,
                            VALID.join(", ")
                        ));
                    }
                    cfg.boot.animation_type = value.to_string();
                }
                "boot.animation_speed_ms" => {
                    cfg.boot.animation_speed_ms = value
                        .parse::<u64>()
                        .with_context(|| format!("Invalid u64 for {key}"))?
                }
                "boot.show_post" => cfg.boot.show_post = parse_bool(value, key)?,
                "boot.post_delay_ms" => {
                    cfg.boot.post_delay_ms = value
                        .parse::<u64>()
                        .with_context(|| format!("Invalid u64 for {key}"))?
                }
                "font.family" => cfg.font.family = value.to_string(),
                "font.size" => {
                    cfg.font.size = value
                        .parse::<u16>()
                        .with_context(|| format!("Invalid u16 for {key}"))?
                }
                "font.bold_ui" => cfg.font.bold_ui = parse_bool(value, key)?,
                "behavior.autosave_config" => {
                    cfg.behavior.autosave_config = parse_bool(value, key)?
                }
                "behavior.confirm_shutdown" => {
                    cfg.behavior.confirm_shutdown = parse_bool(value, key)?
                }
                "behavior.welcome_message" => {
                    cfg.behavior.welcome_message = parse_bool(value, key)?
                }
                _ if key.starts_with("shell.aliases.") => {
                    let alias_key = key.trim_start_matches("shell.aliases.");
                    if alias_key.is_empty() {
                        return Err(anyhow!("Alias key cannot be empty"));
                    }
                    cfg.shell
                        .aliases
                        .insert(alias_key.to_string(), value.to_string());
                }
                _ => return Err(anyhow!("{}", unknown_key_error(key))),
            }

            let mut warnings = cfg.validate_and_fix_colors();
            cfg.normalize_misc_fields(&mut warnings);
            cfg.save(path)?;
            return Ok(warnings);
        }
    }

    pub fn reset_theme_to_default(path: &Path) -> Result<Vec<String>> {
        Config::set_theme(path, "default-dark")
    }

    pub fn set_theme(path: &Path, theme_name: &str) -> Result<Vec<String>> {
        let store = Config::get()?;
        let mut cfg = store
            .write()
            .map_err(|_| anyhow!("Config lock poisoned while setting theme"))?;
        cfg.theme.name = theme_name.to_string();

        let mut warnings = cfg.merge_preset_if_available(path.parent().unwrap_or(Path::new(".")));
        warnings.extend(cfg.validate_and_fix_colors());
        cfg.normalize_misc_fields(&mut warnings);
        cfg.save(path)?;

        Ok(warnings)
    }

    pub fn available_theme_names() -> Vec<String> {
        let mut names = vec![
            "default-dark".to_string(),
            "default-light".to_string(),
            "catppuccin".to_string(),
            "gruvbox".to_string(),
        ];

        let mut embedded = Vec::new();

        for file in BUILTIN_THEMES.files() {
            if let Some(name) = file.path().file_stem().and_then(|s| s.to_str()) {
                embedded.push(name.to_string());
            }
        }

        names.extend(embedded);
        names.sort();
        names.dedup();
        names
    }

    fn merge_preset_if_available(&mut self, _config_dir: &Path) -> Vec<String> {
        let mut warnings = Vec::new();
        let preset_name = self.theme.name.clone();

        let file_name = format!("{preset_name}.toml");

        if let Some(file) = BUILTIN_THEMES.get_file(&file_name) {
            match file.contents_utf8() {
                Some(raw) => match toml::from_str::<ThemePresetDocument>(raw) {
                    Ok(preset_doc) => {
                        apply_theme_preset(&mut self.theme, &preset_doc.theme);
                    }
                    Err(err) => warnings.push(format!("Failed parsing preset: {err}")),
                },
                None => warnings.push("Invalid UTF-8 in theme file".to_string()),
            }
        } else {
            warnings.push(format!(
                "Theme '{}' not found in embedded themes.",
                preset_name
            ));
        }

        warnings
    }

    fn validate_and_fix_colors(&mut self) -> Vec<String> {
        let mut warnings = Vec::new();
        let defaults = ThemeConfig::default();

        validate_color_field(
            &mut self.theme.background,
            &defaults.background,
            "theme.background",
            &mut warnings,
        );
        validate_color_field(
            &mut self.theme.foreground,
            &defaults.foreground,
            "theme.foreground",
            &mut warnings,
        );
        validate_color_field(
            &mut self.theme.accent,
            &defaults.accent,
            "theme.accent",
            &mut warnings,
        );
        validate_color_field(
            &mut self.theme.secondary,
            &defaults.secondary,
            "theme.secondary",
            &mut warnings,
        );
        validate_color_field(
            &mut self.theme.success,
            &defaults.success,
            "theme.success",
            &mut warnings,
        );
        validate_color_field(
            &mut self.theme.warning,
            &defaults.warning,
            "theme.warning",
            &mut warnings,
        );
        validate_color_field(
            &mut self.theme.error,
            &defaults.error,
            "theme.error",
            &mut warnings,
        );
        validate_color_field(
            &mut self.theme.muted,
            &defaults.muted,
            "theme.muted",
            &mut warnings,
        );
        validate_color_field(
            &mut self.theme.border,
            &defaults.border,
            "theme.border",
            &mut warnings,
        );
        validate_color_field(
            &mut self.theme.selection,
            &defaults.selection,
            "theme.selection",
            &mut warnings,
        );
        validate_color_field(
            &mut self.theme.cursor.color,
            &defaults.cursor.color,
            "theme.cursor.color",
            &mut warnings,
        );
        validate_color_field(
            &mut self.theme.syntax.keyword,
            &defaults.syntax.keyword,
            "theme.syntax.keyword",
            &mut warnings,
        );
        validate_color_field(
            &mut self.theme.syntax.string,
            &defaults.syntax.string,
            "theme.syntax.string",
            &mut warnings,
        );
        validate_color_field(
            &mut self.theme.syntax.number,
            &defaults.syntax.number,
            "theme.syntax.number",
            &mut warnings,
        );
        validate_color_field(
            &mut self.theme.syntax.comment,
            &defaults.syntax.comment,
            "theme.syntax.comment",
            &mut warnings,
        );

        warnings
    }

    fn normalize_misc_fields(&mut self, warnings: &mut Vec<String>) {
        let allowed_prompt_styles = ["arrow", "minimal", "powerline", "classic"];
        if !allowed_prompt_styles.contains(&self.shell.prompt_style.as_str()) {
            warnings.push(format!(
                "Invalid shell.prompt_style '{}'. Falling back to 'arrow'.",
                self.shell.prompt_style
            ));
            self.shell.prompt_style = "arrow".to_string();
        }

        let allowed_cursor_shapes = ["block", "underline", "bar"];
        if !allowed_cursor_shapes.contains(&self.theme.cursor.shape.as_str()) {
            warnings.push(format!(
                "Invalid theme.cursor.shape '{}'. Falling back to 'block'.",
                self.theme.cursor.shape
            ));
            self.theme.cursor.shape = "block".to_string();
        }

        let allowed_animation_types = ["text", "gif"];
        if !allowed_animation_types.contains(&self.boot.animation_type.as_str()) {
            warnings.push(format!(
                "Invalid boot.animation_type '{}'. Falling back to 'text'.",
                self.boot.animation_type
            ));
            self.boot.animation_type = "text".to_string();
        }
    }
}

fn parse_bool(value: &str, key: &str) -> Result<bool> {
    value
        .parse::<bool>()
        .with_context(|| format!("Invalid bool for {key}: {value}"))
}

fn apply_theme_preset(theme: &mut ThemeConfig, preset: &ThemePreset) {
    if let Some(value) = &preset.name {
        theme.name = value.clone();
    }
    if let Some(value) = &preset.background {
        theme.background = value.clone();
    }
    if let Some(value) = &preset.foreground {
        theme.foreground = value.clone();
    }
    if let Some(value) = &preset.accent {
        theme.accent = value.clone();
    }
    if let Some(value) = &preset.secondary {
        theme.secondary = value.clone();
    }
    if let Some(value) = &preset.success {
        theme.success = value.clone();
    }
    if let Some(value) = &preset.warning {
        theme.warning = value.clone();
    }
    if let Some(value) = &preset.error {
        theme.error = value.clone();
    }
    if let Some(value) = &preset.muted {
        theme.muted = value.clone();
    }
    if let Some(value) = &preset.border {
        theme.border = value.clone();
    }
    if let Some(value) = &preset.selection {
        theme.selection = value.clone();
    }
}

fn validate_color_field(value: &mut String, fallback: &str, key: &str, warnings: &mut Vec<String>) {
    if !is_hex_color(value) {
        warnings.push(format!(
            "Invalid color '{}' for {}. Falling back to {}.",
            value, key, fallback
        ));
        *value = fallback.to_string();
    }
}

pub fn is_hex_color(value: &str) -> bool {
    let bytes = value.as_bytes();
    if bytes.len() != 7 || bytes[0] != b'#' {
        return false;
    }

    bytes[1..].iter().all(|byte| byte.is_ascii_hexdigit())
}

pub fn config_path() -> PathBuf {
    PathBuf::from("capsule.toml")
}

fn unknown_key_error(key: &str) -> String {
    const ALL_KEYS: &[&str] = &[
        "system.name",
        "system.version",
        "system.username",
        "system.hostname",
        "system.language",
        "theme.name",
        "theme.background",
        "theme.foreground",
        "theme.accent",
        "theme.secondary",
        "theme.success",
        "theme.warning",
        "theme.error",
        "theme.muted",
        "theme.border",
        "theme.selection",
        "theme.cursor.shape",
        "theme.cursor.blink",
        "theme.cursor.color",
        "theme.syntax.keyword",
        "theme.syntax.string",
        "theme.syntax.number",
        "theme.syntax.comment",
        "shell.prompt_style",
        "shell.show_git",
        "shell.show_time",
        "shell.history_size",
        "boot.show_animation",
        "boot.animation_type",
        "boot.animation_speed_ms",
        "boot.show_post",
        "boot.post_delay_ms",
        "font.family",
        "font.size",
        "font.bold_ui",
        "behavior.autosave_config",
        "behavior.confirm_shutdown",
        "behavior.welcome_message",
    ];

    let bare = key.split('.').last().unwrap_or(key);
    let suggestion = ALL_KEYS
        .iter()
        .find(|k| k.split('.').last() == Some(bare) && **k != key)
        .map(|k| format!(" Did you mean '{k}'?"));

    match suggestion {
        Some(hint) => format!("Unknown config key: '{key}'.{hint}"),
        None => {
            format!("Unknown config key: '{key}'. Run `capsule config list` to see all valid keys.")
        }
    }
}
