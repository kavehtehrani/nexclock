use std::collections::BTreeMap;
use std::fs;
use std::path::PathBuf;

use directories::ProjectDirs;
use serde::{Deserialize, Serialize};
use tracing::{info, warn};

use crate::component::{
    parse_component, rects_overlap, CalendarSettings, ClockSettings, ClockStyle, ComponentConfig,
    ComponentEntry, GridPlacement, SystemStatsSettings, WeatherSettings,
};
use crate::constants;
use crate::defaults::{
    default_date_format, default_latitude, default_longitude, default_stats_refresh,
    default_temp_unit, default_time_format, default_true, default_weather_refresh,
};
use crate::error::NexClockError;

// ── Theme ───────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ThemeConfig {
    #[serde(default = "default_theme_primary")]
    pub primary: String,
    #[serde(default = "default_theme_secondary")]
    pub secondary: String,
    #[serde(default = "default_theme_tertiary")]
    pub tertiary: String,
    #[serde(default = "default_theme_info")]
    pub info: String,
    #[serde(default = "default_theme_muted")]
    pub muted: String,
    #[serde(default = "default_theme_text")]
    pub text: String,
    #[serde(default = "default_theme_error")]
    pub error: String,
    #[serde(default = "default_theme_focus")]
    pub focus: String,
}

impl Default for ThemeConfig {
    fn default() -> Self {
        Self {
            primary: default_theme_primary(),
            secondary: default_theme_secondary(),
            tertiary: default_theme_tertiary(),
            info: default_theme_info(),
            muted: default_theme_muted(),
            text: default_theme_text(),
            error: default_theme_error(),
            focus: default_theme_focus(),
        }
    }
}

// ── Grid ────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GridConfig {
    #[serde(default = "default_grid_columns")]
    pub columns: u16,
    #[serde(default = "default_grid_rows")]
    pub rows: u16,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub row_heights: Option<Vec<u16>>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub column_widths: Option<Vec<u16>>,
}

impl Default for GridConfig {
    fn default() -> Self {
        Self {
            columns: default_grid_columns(),
            rows: default_grid_rows(),
            row_heights: Some(vec![40, 30, 30]),
            column_widths: None,
        }
    }
}

// ── Network ─────────────────────────────────────────────────────────

#[derive(Debug, Serialize, Deserialize)]
pub struct NetworkConfig {
    #[serde(default = "default_ip_refresh")]
    pub ip_refresh_interval_minutes: u64,
}

impl Default for NetworkConfig {
    fn default() -> Self {
        Self {
            ip_refresh_interval_minutes: default_ip_refresh(),
        }
    }
}

// ── Appearance ──────────────────────────────────────────────────────

#[derive(Debug, Serialize, Deserialize)]
pub struct AppearanceConfig {
    #[serde(default = "default_tick_rate")]
    pub tick_rate_ms: u64,
}

impl Default for AppearanceConfig {
    fn default() -> Self {
        Self {
            tick_rate_ms: default_tick_rate(),
        }
    }
}

// ── Legacy config structs (for migration only) ─────────────────────

#[derive(Debug, Default, Deserialize)]
pub struct LegacyClockConfig {
    #[serde(default = "default_time_format")]
    pub time_format: String,
    #[serde(default = "default_date_format")]
    pub date_format: String,
    #[serde(default = "default_true")]
    pub show_seconds: bool,
    #[serde(default = "default_true")]
    pub blink_separator: bool,
}

#[derive(Debug, Default, Deserialize)]
pub struct LegacySecondaryClockConfig {
    #[serde(default = "default_true")]
    pub enabled: bool,
    #[serde(default = "default_secondary_timezone")]
    pub timezone: String,
    #[serde(default = "default_secondary_label")]
    pub label: String,
    #[serde(default = "default_time_format")]
    pub time_format: String,
    #[serde(default = "default_secondary_date_format")]
    pub date_format: String,
}

#[derive(Debug, Default, Deserialize)]
pub struct LegacyWeatherConfig {
    #[serde(default = "default_true")]
    pub enabled: bool,
    #[serde(default = "default_latitude")]
    pub latitude: f64,
    #[serde(default = "default_longitude")]
    pub longitude: f64,
    #[serde(default = "default_temp_unit")]
    pub temperature_unit: String,
    #[serde(default = "default_weather_refresh")]
    pub refresh_interval_minutes: u64,
}

#[derive(Debug, Default, Deserialize)]
pub struct LegacyCalendarConfig {
    #[serde(default = "default_true")]
    pub show_gregorian: bool,
}

#[derive(Debug, Default, Deserialize)]
pub struct LegacySystemStatsConfig {
    #[serde(default = "default_true")]
    pub enabled: bool,
    #[serde(default = "default_stats_refresh")]
    pub refresh_interval_seconds: u64,
}

#[derive(Debug, Default, Deserialize)]
#[allow(dead_code)]
pub struct LegacyLayoutConfig {
    #[serde(default)]
    pub top: Option<String>,
    #[serde(default)]
    pub left_top: Option<String>,
    #[serde(default)]
    pub left_bottom: Option<String>,
    #[serde(default)]
    pub right_top: Option<String>,
    #[serde(default)]
    pub right_bottom: Option<String>,
    #[serde(default)]
    pub top_height_percent: Option<u16>,
    #[serde(default, alias = "info_height_percent")]
    pub bottom_height_percent: Option<u16>,
    #[serde(default)]
    pub left_column_percent: Option<u16>,
    #[serde(default, alias = "left_top_percent")]
    pub left_split_percent: Option<u16>,
    #[serde(default, alias = "right_top_percent")]
    pub right_split_percent: Option<u16>,
}

// ── Main AppConfig ──────────────────────────────────────────────────

#[derive(Debug, Serialize, Deserialize)]
pub struct AppConfig {
    #[serde(default)]
    pub theme: ThemeConfig,
    #[serde(default)]
    pub grid: GridConfig,
    #[serde(default)]
    pub appearance: AppearanceConfig,
    #[serde(default)]
    pub network: NetworkConfig,

    /// Component definitions keyed by stable ID.
    #[serde(default)]
    pub components: BTreeMap<String, toml::Table>,

    // Legacy fields -- read but never written
    #[serde(default, skip_serializing)]
    pub clock: Option<LegacyClockConfig>,
    #[serde(default, skip_serializing)]
    pub secondary_clock: Option<LegacySecondaryClockConfig>,
    #[serde(default, skip_serializing)]
    pub weather: Option<LegacyWeatherConfig>,
    #[serde(default, skip_serializing)]
    pub calendar: Option<LegacyCalendarConfig>,
    #[serde(default, skip_serializing)]
    pub system_stats: Option<LegacySystemStatsConfig>,
    #[serde(default, skip_serializing)]
    pub layout: Option<LegacyLayoutConfig>,
}

impl Default for AppConfig {
    fn default() -> Self {
        let mut config = Self {
            theme: ThemeConfig::default(),
            grid: GridConfig::default(),
            appearance: AppearanceConfig::default(),
            network: NetworkConfig::default(),
            components: BTreeMap::new(),
            clock: None,
            secondary_clock: None,
            weather: None,
            calendar: None,
            system_stats: None,
            layout: None,
        };
        config.components = default_components();
        config
    }
}

impl AppConfig {
    /// Returns the config directory path (~/.config/nexclock/).
    fn config_dir() -> Option<PathBuf> {
        ProjectDirs::from("", "", "nexclock").map(|dirs| dirs.config_dir().to_path_buf())
    }

    /// Returns the path to the config file.
    fn config_path() -> Option<PathBuf> {
        Self::config_dir().map(|dir| dir.join("config.toml"))
    }

    /// Returns the data directory path (~/.local/share/nexclock/).
    pub fn data_dir() -> Option<PathBuf> {
        ProjectDirs::from("", "", "nexclock").map(|dirs| dirs.data_dir().to_path_buf())
    }

    /// Loads the config from disk, or creates a default one if it doesn't exist.
    pub fn load() -> Self {
        let Some(path) = Self::config_path() else {
            warn!("Could not determine config directory, using defaults");
            return Self::default();
        };

        let mut config: Self = if path.exists() {
            match fs::read_to_string(&path) {
                Ok(contents) => match toml::from_str(&contents) {
                    Ok(config) => config,
                    Err(err) => {
                        warn!("Failed to parse config: {err}. Using defaults.");
                        Self::default()
                    }
                },
                Err(err) => {
                    warn!("Failed to read config: {err}. Using defaults.");
                    Self::default()
                }
            }
        } else {
            info!(
                "No config found, generating default at {}",
                path.display()
            );
            let config = Self::default();
            if let Err(err) = config.save() {
                warn!("Could not write default config: {err}");
            }
            return config;
        };

        // Migrate legacy config if needed
        if config.components.is_empty() && config.has_legacy_fields() {
            info!("Migrating legacy config to component-based format");
            let bak_path = path.with_extension("toml.bak");
            if let Err(err) = fs::copy(&path, &bak_path) {
                warn!("Could not back up old config: {err}");
            }
            config.migrate_legacy();
            if let Err(err) = config.save() {
                warn!("Could not save migrated config: {err}");
            }
        }

        if let Err(err) = config.validate() {
            warn!("Config validation: {err}. Some values may be clamped.");
        }

        config
    }

    /// Checks if any legacy config fields are populated.
    fn has_legacy_fields(&self) -> bool {
        self.clock.is_some()
            || self.secondary_clock.is_some()
            || self.weather.is_some()
            || self.calendar.is_some()
            || self.system_stats.is_some()
            || self.layout.is_some()
    }

    /// Converts old-format config sections into component entries.
    fn migrate_legacy(&mut self) {
        let legacy_clock = self.clock.take();
        let legacy_secondary = self.secondary_clock.take();
        let legacy_weather = self.weather.take();
        let legacy_calendar = self.calendar.take();
        let legacy_stats = self.system_stats.take();
        self.layout.take();

        self.grid = GridConfig {
            columns: 2,
            rows: 3,
            row_heights: Some(vec![40, 30, 30]),
            column_widths: None,
        };

        let clock_cfg = legacy_clock.unwrap_or_default();
        let main_clock = ComponentEntry {
            id: "main_clock".to_string(),
            placement: GridPlacement {
                row: 0,
                column: 0,
                row_span: 1,
                col_span: 2,
            },
            config: ComponentConfig::Clock(ClockSettings {
                style: ClockStyle::Large,
                time_format: clock_cfg.time_format,
                date_format: clock_cfg.date_format,
                show_seconds: clock_cfg.show_seconds,
                blink_separator: clock_cfg.blink_separator,
                timezone: None,
                label: None,
                font_style: constants::DEFAULT_FONT_STYLE.to_string(),
                colors: Vec::new(),
            }),
            visible: true,
        };

        let mut components = BTreeMap::new();
        components.insert("main_clock".to_string(), main_clock.to_toml_table());

        let sec_cfg = legacy_secondary.unwrap_or_default();
        if sec_cfg.enabled {
            let sec_clock = ComponentEntry {
                id: "secondary_clock".to_string(),
                placement: GridPlacement {
                    row: 1,
                    column: 0,
                    row_span: 1,
                    col_span: 1,
                },
                config: ComponentConfig::Clock(ClockSettings {
                    style: ClockStyle::Compact,
                    time_format: sec_cfg.time_format,
                    date_format: sec_cfg.date_format,
                    show_seconds: false,
                    blink_separator: false,
                    timezone: Some(sec_cfg.timezone),
                    label: Some(sec_cfg.label),
                    font_style: constants::DEFAULT_FONT_STYLE.to_string(),
                    colors: Vec::new(),
                }),
                visible: true,
            };
            components.insert("secondary_clock".to_string(), sec_clock.to_toml_table());
        }

        let weather_cfg = legacy_weather.unwrap_or_default();
        if weather_cfg.enabled {
            let weather = ComponentEntry {
                id: "weather".to_string(),
                placement: GridPlacement {
                    row: 1,
                    column: 1,
                    row_span: 1,
                    col_span: 1,
                },
                config: ComponentConfig::Weather(WeatherSettings {
                    latitude: weather_cfg.latitude,
                    longitude: weather_cfg.longitude,
                    temperature_unit: weather_cfg.temperature_unit,
                    refresh_interval_minutes: weather_cfg.refresh_interval_minutes,
                }),
                visible: true,
            };
            components.insert("weather".to_string(), weather.to_toml_table());
        }

        let cal_cfg = legacy_calendar.unwrap_or_default();
        if cal_cfg.show_gregorian {
            let calendar = ComponentEntry {
                id: "calendar".to_string(),
                placement: GridPlacement {
                    row: 2,
                    column: 0,
                    row_span: 1,
                    col_span: 1,
                },
                config: ComponentConfig::Calendar(CalendarSettings {}),
                visible: true,
            };
            components.insert("calendar".to_string(), calendar.to_toml_table());
        }

        let stats_cfg = legacy_stats.unwrap_or_default();
        if stats_cfg.enabled {
            let stats = ComponentEntry {
                id: "sys".to_string(),
                placement: GridPlacement {
                    row: 2,
                    column: 1,
                    row_span: 1,
                    col_span: 1,
                },
                config: ComponentConfig::SystemStats(SystemStatsSettings {
                    refresh_interval_seconds: stats_cfg.refresh_interval_seconds,
                }),
                visible: true,
            };
            components.insert("sys".to_string(), stats.to_toml_table());
        }

        self.components = components;
    }

    /// Parses all component entries from the raw TOML tables.
    pub fn parse_components(&self) -> Vec<ComponentEntry> {
        let mut entries = Vec::new();
        for (id, table) in &self.components {
            match parse_component(id, table) {
                Ok(entry) => entries.push(entry),
                Err(err) => warn!("Skipping component '{id}': {err}"),
            }
        }
        entries
    }

    /// Syncs a Vec<ComponentEntry> back into the config's components BTreeMap.
    pub fn sync_components(&mut self, entries: &[ComponentEntry]) {
        self.components.clear();
        for entry in entries {
            self.components
                .insert(entry.id.clone(), entry.to_toml_table());
        }
    }

    /// Writes the current config to disk.
    pub fn save(&self) -> Result<(), Box<dyn std::error::Error>> {
        let path = Self::config_path().ok_or("Could not determine config path")?;
        if let Some(dir) = path.parent() {
            fs::create_dir_all(dir)?;
        }
        let contents = toml::to_string_pretty(self)?;
        fs::write(&path, contents)?;
        Ok(())
    }

    /// Validates config values, returning an error describing all issues found.
    pub fn validate(&self) -> Result<(), NexClockError> {
        let mut issues = Vec::new();

        if self.appearance.tick_rate_ms < constants::MIN_TICK_RATE_MS {
            issues.push(format!(
                "appearance.tick_rate_ms must be >= {}",
                constants::MIN_TICK_RATE_MS
            ));
        }

        if self.grid.rows == 0 {
            issues.push("grid.rows must be > 0".to_string());
        }
        if self.grid.columns == 0 {
            issues.push("grid.columns must be > 0".to_string());
        }

        if let Some(ref rh) = self.grid.row_heights
            && rh.len() != self.grid.rows as usize {
                issues.push(format!(
                    "grid.row_heights has {} entries but grid.rows is {}",
                    rh.len(),
                    self.grid.rows
                ));
            }
        if let Some(ref cw) = self.grid.column_widths
            && cw.len() != self.grid.columns as usize {
                issues.push(format!(
                    "grid.column_widths has {} entries but grid.columns is {}",
                    cw.len(),
                    self.grid.columns
                ));
            }

        // Validate individual components
        let entries = self.parse_components();
        for entry in &entries {
            let p = &entry.placement;
            if p.row + p.row_span > self.grid.rows {
                issues.push(format!(
                    "component '{}' extends beyond grid rows (row {} + span {} > {})",
                    entry.id, p.row, p.row_span, self.grid.rows
                ));
            }
            if p.column + p.col_span > self.grid.columns {
                issues.push(format!(
                    "component '{}' extends beyond grid columns (col {} + span {} > {})",
                    entry.id, p.column, p.col_span, self.grid.columns
                ));
            }

            if let ComponentConfig::Clock(ref s) = entry.config {
                if s.time_format != "12h" && s.time_format != "24h" {
                    issues.push(format!(
                        "component '{}': time_format must be \"12h\" or \"24h\"",
                        entry.id
                    ));
                }
                if let Some(ref tz) = s.timezone
                    && tz.parse::<chrono_tz::Tz>().is_err() {
                        issues.push(format!(
                            "component '{}': invalid timezone '{tz}'",
                            entry.id
                        ));
                    }
            }
            if let ComponentConfig::Weather(ref s) = entry.config {
                if s.temperature_unit != "celsius" && s.temperature_unit != "fahrenheit" {
                    issues.push(format!(
                        "component '{}': temperature_unit must be \"celsius\" or \"fahrenheit\"",
                        entry.id
                    ));
                }
                if s.latitude < -90.0 || s.latitude > 90.0 {
                    issues.push(format!(
                        "component '{}': latitude must be between -90 and 90",
                        entry.id
                    ));
                }
                if s.longitude < -180.0 || s.longitude > 180.0 {
                    issues.push(format!(
                        "component '{}': longitude must be between -180 and 180",
                        entry.id
                    ));
                }
            }
        }

        check_overlaps(&entries, &mut issues);

        if issues.is_empty() {
            Ok(())
        } else {
            Err(NexClockError::Config(issues.join("; ")))
        }
    }

    /// Returns the tick rate as a Duration.
    pub fn tick_rate(&self) -> std::time::Duration {
        std::time::Duration::from_millis(
            self.appearance.tick_rate_ms.max(constants::MIN_TICK_RATE_MS),
        )
    }
}

/// Checks for overlapping components and appends issues.
fn check_overlaps(entries: &[ComponentEntry], issues: &mut Vec<String>) {
    for (i, a) in entries.iter().enumerate() {
        if !a.visible {
            continue;
        }
        for b in entries.iter().skip(i + 1) {
            if !b.visible {
                continue;
            }
            let ap = &a.placement;
            let bp = &b.placement;
            if rects_overlap(
                (ap.row, ap.column, ap.row_span, ap.col_span),
                (bp.row, bp.column, bp.row_span, bp.col_span),
            ) {
                issues.push(format!(
                    "components '{}' and '{}' overlap in the grid",
                    a.id, b.id
                ));
            }
        }
    }
}

/// Returns the default set of components matching the original 5-panel layout.
fn default_components() -> BTreeMap<String, toml::Table> {
    let entries = vec![
        ComponentEntry {
            id: "main_clock".to_string(),
            placement: GridPlacement {
                row: 0,
                column: 0,
                row_span: 1,
                col_span: 2,
            },
            config: ComponentConfig::Clock(ClockSettings {
                style: ClockStyle::Large,
                ..ClockSettings::default()
            }),
            visible: true,
        },
        ComponentEntry {
            id: "ny_clock".to_string(),
            placement: GridPlacement {
                row: 1,
                column: 0,
                row_span: 1,
                col_span: 1,
            },
            config: ComponentConfig::Clock(ClockSettings {
                style: ClockStyle::Compact,
                timezone: Some("US/Eastern".to_string()),
                label: Some("New York".to_string()),
                time_format: "12h".to_string(),
                date_format: constants::DEFAULT_SECONDARY_DATE_FORMAT.to_string(),
                show_seconds: false,
                blink_separator: false,
                ..ClockSettings::default()
            }),
            visible: true,
        },
        ComponentEntry {
            id: "weather".to_string(),
            placement: GridPlacement {
                row: 1,
                column: 1,
                row_span: 1,
                col_span: 1,
            },
            config: ComponentConfig::Weather(WeatherSettings::default()),
            visible: true,
        },
        ComponentEntry {
            id: "calendar".to_string(),
            placement: GridPlacement {
                row: 2,
                column: 0,
                row_span: 1,
                col_span: 1,
            },
            config: ComponentConfig::Calendar(CalendarSettings::default()),
            visible: true,
        },
        ComponentEntry {
            id: "sys".to_string(),
            placement: GridPlacement {
                row: 2,
                column: 1,
                row_span: 1,
                col_span: 1,
            },
            config: ComponentConfig::SystemStats(SystemStatsSettings::default()),
            visible: true,
        },
    ];

    let mut map = BTreeMap::new();
    for entry in entries {
        map.insert(entry.id.clone(), entry.to_toml_table());
    }
    map
}

// ── Default value functions (config-specific) ───────────────────────

fn default_secondary_timezone() -> String {
    "US/Eastern".to_string()
}
fn default_secondary_label() -> String {
    "New York".to_string()
}
fn default_secondary_date_format() -> String {
    constants::DEFAULT_SECONDARY_DATE_FORMAT.to_string()
}
fn default_ip_refresh() -> u64 {
    constants::DEFAULT_IP_REFRESH_MINUTES
}
fn default_tick_rate() -> u64 {
    constants::DEFAULT_TICK_RATE.as_millis() as u64
}
fn default_grid_rows() -> u16 {
    constants::DEFAULT_GRID_ROWS
}
fn default_grid_columns() -> u16 {
    constants::DEFAULT_GRID_COLUMNS
}
fn default_theme_primary() -> String {
    constants::DEFAULT_THEME_PRIMARY.to_string()
}
fn default_theme_secondary() -> String {
    constants::DEFAULT_THEME_SECONDARY.to_string()
}
fn default_theme_tertiary() -> String {
    constants::DEFAULT_THEME_TERTIARY.to_string()
}
fn default_theme_info() -> String {
    constants::DEFAULT_THEME_INFO.to_string()
}
fn default_theme_muted() -> String {
    constants::DEFAULT_THEME_MUTED.to_string()
}
fn default_theme_text() -> String {
    constants::DEFAULT_THEME_TEXT.to_string()
}
fn default_theme_error() -> String {
    constants::DEFAULT_THEME_ERROR.to_string()
}
fn default_theme_focus() -> String {
    constants::DEFAULT_THEME_FOCUS.to_string()
}
