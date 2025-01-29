use crate::theme::{Theme, ThemeConfig};
use anyhow::{Context, Result};
use gpui::{App, Global};
use serde::Deserialize;
use std::fs;
use std::path::{Path, PathBuf};

#[derive(Debug, Deserialize)]
pub struct TomlConfig {
    pub theme: ThemeConfig
}

#[derive(Debug, Deserialize)]
#[derive(Default)]
pub struct Config {
    pub theme: Theme,
}

impl Config {
    pub fn new(cx: &mut App) {
        let config = Config::load().unwrap_or_else(|err| {
            eprintln!("Failed to load configuration: {err}");
            Config::default()
        });

        cx.set_global(config);
    }

    fn load() -> Result<Self> {
        let config_path = Self::config_path()?;
        if !config_path.exists() {
            Self::create_example_config(&config_path)?;
            return Ok(Self::default());
        }

        let content = fs::read_to_string(&config_path)
            .with_context(|| format!("Failed to read config file at {config_path:?}"))?;

        let config: TomlConfig = toml::from_str(&content)
            .with_context(|| format!("Failed to parse config file at {config_path:?}"))?;

        let default_theme = Theme::default();
        let result = Self {
            theme: Theme {
                primary: config.theme.primary.map_or(default_theme.primary, Into::into),
                background: config.theme.background.map_or(default_theme.background, Into::into),
                foreground: config.theme.foreground.map_or(default_theme.foreground, Into::into),
                muted: config.theme.muted.map_or(default_theme.muted, Into::into),
                muted_foreground: config.theme.muted_foreground.map_or(default_theme.muted_foreground, Into::into),
                border: config.theme.border.map_or(default_theme.border, Into::into),
            }
        };

        Ok(result)
    }

    fn create_example_config(config_path: &Path) -> Result<()> {
        let example_config = include_str!("../assets/config.toml");
        if let Some(parent) = config_path.parent() {
            fs::create_dir_all(parent)
                .context("Failed to create config directory")?;
        }

        fs::write(config_path, example_config)
            .with_context(|| format!("Failed to write example config to {config_path:?}"))?;

        Ok(())
    }

    pub fn config_path() -> Result<PathBuf> {
        let app_support_dir = dirs::config_dir()
            .context("Failed to get application config directory")?
            .join("FastForward");

        Ok(app_support_dir.join("config.toml"))
    }
}

impl Global for Config {}
