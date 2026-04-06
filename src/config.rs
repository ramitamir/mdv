use crate::keys::{deserialize_key_option, KeyBinding, KeyBindings};
use crate::theme::{self, ColorsConfig, Theme};
use serde::Deserialize;

// ── Top-level config ──────────────────────────────────────────────────────────

#[derive(Debug, Default, Deserialize)]
pub struct Config {
    pub theme: Option<String>,
    pub font: Option<String>,
    pub font_size: Option<f32>,
    pub keys: Option<KeysConfig>,
    pub colors: Option<ColorsConfig>,
    pub spacing: Option<SpacingConfig>,
}

/// Spacing values as multipliers of font_size.
#[derive(Debug, Default, Deserialize)]
pub struct SpacingConfig {
    pub block_gap: Option<f32>,
    pub heading_before: Option<f32>,
    pub heading_after: Option<f32>,
}

#[derive(Debug, Default, Deserialize)]
pub struct KeysConfig {
    pub normal: Option<NormalKeys>,
    pub source: Option<SourceKeys>,
    pub search: Option<SearchKeys>,
}

// ── Key config structs (Optional fields, user overrides) ─────────────────────

macro_rules! key_fields {
    ($name:ident { $($field:ident),* $(,)? }) => {
        #[derive(Debug, Default, Deserialize)]
        pub struct $name {
            $(
                #[serde(default, deserialize_with = "deserialize_key_option")]
                pub $field: Option<KeyBindings>,
            )*
        }
    };
}

key_fields!(NormalKeys {
    quit, scroll_down, scroll_up,
    page_down, page_up, top, bottom, search, help,
    next_match, prev_match, source_mode,
});

key_fields!(SourceKeys {
    quit, cursor_down, cursor_up, cursor_left, cursor_right,
    word_forward, word_back, line_start, line_end,
    half_page_down, half_page_up, top, bottom,
    select, yank, view_mode, search, help,
    next_match, prev_match,
});

key_fields!(SearchKeys {
    cancel, confirm,
});

// ── Resolved key structs (defaults applied, no Option) ───────────────────────

#[allow(dead_code)]
pub struct ResolvedKeys {
    pub normal: ResolvedNormalKeys,
    pub source: ResolvedSourceKeys,
    pub search: ResolvedSearchKeys,
}

#[allow(dead_code)]
pub struct ResolvedNormalKeys {
    pub quit: KeyBindings,
    pub scroll_down: KeyBindings,
    pub scroll_up: KeyBindings,
    pub page_down: KeyBindings,
    pub page_up: KeyBindings,
    pub top: KeyBindings,
    pub bottom: KeyBindings,
    pub search: KeyBindings,
    pub help: KeyBindings,
    pub next_match: KeyBindings,
    pub prev_match: KeyBindings,
    pub source_mode: KeyBindings,
}

#[allow(dead_code)]
pub struct ResolvedSourceKeys {
    pub quit: KeyBindings,
    pub cursor_down: KeyBindings,
    pub cursor_up: KeyBindings,
    pub cursor_left: KeyBindings,
    pub cursor_right: KeyBindings,
    pub word_forward: KeyBindings,
    pub word_back: KeyBindings,
    pub line_start: KeyBindings,
    pub line_end: KeyBindings,
    pub half_page_down: KeyBindings,
    pub half_page_up: KeyBindings,
    pub top: KeyBindings,
    pub bottom: KeyBindings,
    pub select: KeyBindings,
    pub yank: KeyBindings,
    pub view_mode: KeyBindings,
    pub search: KeyBindings,
    pub help: KeyBindings,
    pub next_match: KeyBindings,
    pub prev_match: KeyBindings,
}

#[allow(dead_code)]
pub struct ResolvedSearchKeys {
    pub cancel: KeyBindings,
    pub confirm: KeyBindings,
}

/// Resolved spacing values (multipliers of font_size).
pub struct ResolvedSpacing {
    pub block_gap: f32,
    pub heading_before: f32,
    pub heading_after: f32,
}

impl ResolvedSpacing {
    pub fn with_overrides(config: &Option<SpacingConfig>) -> Self {
        let s = config.as_ref();
        ResolvedSpacing {
            block_gap: s.and_then(|c| c.block_gap).unwrap_or(0.0),
            heading_before: s.and_then(|c| c.heading_before).unwrap_or(0.0),
            heading_after: s.and_then(|c| c.heading_after).unwrap_or(0.0),
        }
    }
}

// ── Helper ───────────────────────────────────────────────────────────────────

fn kb(keys: &[&str]) -> KeyBindings {
    KeyBindings(keys.iter().map(|s| KeyBinding::parse(s).unwrap()).collect())
}

// ── ResolvedKeys construction ─────────────────────────────────────────────────

macro_rules! resolve_key {
    ($section:expr, $field:ident, $defaults:expr) => {
        $section
            .and_then(|s: &_| s.$field.clone())
            .unwrap_or_else(|| kb($defaults))
    };
}

impl ResolvedKeys {
    pub fn with_overrides(config: &Option<KeysConfig>) -> Self {
        let normal = config.as_ref().and_then(|c| c.normal.as_ref());
        let source = config.as_ref().and_then(|c| c.source.as_ref());
        let search = config.as_ref().and_then(|c| c.search.as_ref());

        ResolvedKeys {
            normal: ResolvedNormalKeys {
                quit:            resolve_key!(normal, quit,            &["q", "esc"]),
                scroll_down:     resolve_key!(normal, scroll_down,     &["down", "j"]),
                scroll_up:       resolve_key!(normal, scroll_up,       &["up", "k"]),
                page_down:       resolve_key!(normal, page_down,       &["pagedown"]),
                page_up:         resolve_key!(normal, page_up,         &["pageup"]),
                top:             resolve_key!(normal, top,             &["g", "home"]),
                bottom:          resolve_key!(normal, bottom,          &["G", "end"]),
                search:          resolve_key!(normal, search,          &["/"]),
                help:            resolve_key!(normal, help,            &["?"]),
                next_match:      resolve_key!(normal, next_match,      &["n"]),
                prev_match:      resolve_key!(normal, prev_match,      &["N"]),
                source_mode:     resolve_key!(normal, source_mode,     &["r"]),
            },
            source: ResolvedSourceKeys {
                quit:            resolve_key!(source, quit,            &["q", "esc"]),
                cursor_down:     resolve_key!(source, cursor_down,     &["down", "j"]),
                cursor_up:       resolve_key!(source, cursor_up,       &["up", "k"]),
                cursor_left:     resolve_key!(source, cursor_left,     &["left", "h"]),
                cursor_right:    resolve_key!(source, cursor_right,    &["right", "l"]),
                word_forward:    resolve_key!(source, word_forward,    &["w"]),
                word_back:       resolve_key!(source, word_back,       &["b"]),
                line_start:      resolve_key!(source, line_start,      &["0", "home"]),
                line_end:        resolve_key!(source, line_end,        &["$", "end"]),
                half_page_down:  resolve_key!(source, half_page_down,  &["pagedown"]),
                half_page_up:    resolve_key!(source, half_page_up,    &["pageup"]),
                top:             resolve_key!(source, top,             &["g"]),
                bottom:          resolve_key!(source, bottom,          &["G"]),
                select:          resolve_key!(source, select,          &["v"]),
                yank:            resolve_key!(source, yank,            &["y"]),
                view_mode:       resolve_key!(source, view_mode,       &["r"]),
                search:          resolve_key!(source, search,          &["/"]),
                help:            resolve_key!(source, help,            &["?"]),
                next_match:      resolve_key!(source, next_match,      &["n"]),
                prev_match:      resolve_key!(source, prev_match,      &["N"]),
            },
            search: ResolvedSearchKeys {
                cancel:  resolve_key!(search, cancel,  &["esc"]),
                confirm: resolve_key!(search, confirm, &["enter"]),
            },
        }
    }
}

// ── load() ────────────────────────────────────────────────────────────────────

pub fn load() -> Config {
    // Check ~/.config/mdv/ first (cross-platform convention), then platform config dir
    let path = std::env::var("HOME").ok()
        .map(|h| std::path::PathBuf::from(h).join(".config/mdv/config.toml"))
        .filter(|p| p.exists())
        .or_else(|| dirs::config_dir().map(|d| d.join("mdv").join("config.toml")).filter(|p| p.exists()));

    let path = match path {
        Some(p) => p,
        None => return Config::default(),
    };

    match std::fs::read_to_string(&path) {
        Ok(content) => match toml::from_str(&content) {
            Ok(config) => config,
            Err(e) => {
                eprintln!("mdv: error parsing {}: {}", path.display(), e);
                Config::default()
            }
        },
        Err(e) => {
            eprintln!("mdv: error reading {}: {}", path.display(), e);
            Config::default()
        }
    }
}

// ── resolve() ────────────────────────────────────────────────────────────────

pub fn resolve(config: &Config, theme_override: Option<&str>) -> (Theme, ResolvedKeys, ResolvedSpacing) {
    let theme_name = theme_override
        .or(config.theme.as_deref())
        .unwrap_or("default");

    let mut theme = match theme::builtin(theme_name) {
        Some(t) => t,
        None => {
            eprintln!("mdv: unknown theme {:?}, falling back to default", theme_name);
            theme::builtin("default").unwrap()
        }
    };

    if let Some(ref colors) = config.colors {
        theme.apply_overrides(colors);
    }

    let keys = ResolvedKeys::with_overrides(&config.keys);
    let spacing = ResolvedSpacing::with_overrides(&config.spacing);
    (theme, keys, spacing)
}

// ── Tests ─────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use crossterm::event::{KeyCode, KeyModifiers};

    #[test]
    fn empty_config_parses() {
        let config: Config = toml::from_str("").unwrap();
        assert!(config.theme.is_none());
        assert!(config.keys.is_none());
        assert!(config.colors.is_none());
    }

    #[test]
    fn full_config_parses() {
        let toml_str = r##"
theme = "catppuccin-mocha"

[keys.normal]
quit = ["q", "esc"]
scroll_down = "j"

[colors]
status_bar_view = "#FF0000"
"##;
        let config: Config = toml::from_str(toml_str).unwrap();
        assert_eq!(config.theme.as_deref(), Some("catppuccin-mocha"));
        let normal = config.keys.unwrap().normal.unwrap();
        assert_eq!(normal.quit.unwrap().0.len(), 2);
        assert_eq!(normal.scroll_down.unwrap().0.len(), 1);
    }

    #[test]
    fn defaults_are_sane() {
        let config = Config::default();
        let (theme, keys, _) = resolve(&config, None);
        assert_eq!(theme.status_bar_view, ratatui::style::Color::Rgb(40, 60, 120));
        let q = crossterm::event::KeyEvent::new(KeyCode::Char('q'), KeyModifiers::NONE);
        assert!(keys.normal.quit.matches(&q));
    }

    #[test]
    fn theme_override_from_cli() {
        let config = Config::default();
        let (theme, _, _) = resolve(&config, Some("catppuccin-mocha"));
        assert_ne!(theme.status_bar_view, ratatui::style::Color::Rgb(40, 60, 120));
    }

    #[test]
    fn key_override_merges() {
        let toml_str = r#"
[keys.normal]
quit = "x"
"#;
        let config: Config = toml::from_str(toml_str).unwrap();
        let (_, keys, _) = resolve(&config, None);
        let x = crossterm::event::KeyEvent::new(KeyCode::Char('x'), KeyModifiers::NONE);
        let q = crossterm::event::KeyEvent::new(KeyCode::Char('q'), KeyModifiers::NONE);
        assert!(keys.normal.quit.matches(&x));
        assert!(!keys.normal.quit.matches(&q));
        let j = crossterm::event::KeyEvent::new(KeyCode::Char('j'), KeyModifiers::NONE);
        assert!(keys.normal.scroll_down.matches(&j));
    }
}
