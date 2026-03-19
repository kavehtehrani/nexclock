use std::fs;
use std::path::PathBuf;

use directories::ProjectDirs;
use serde::{Deserialize, Serialize};
use tracing::{info, warn};

use crate::constants;
use crate::error::NexClockError;

#[derive(Debug, Default, Serialize, Deserialize)]
pub struct AppConfig {
    #[serde(default)]
    pub clock: ClockConfig,
    #[serde(default)]
    pub secondary_clock: SecondaryClockConfig,
    #[serde(default)]
    pub calendar: CalendarConfig,
    #[serde(default)]
    pub layout: LayoutConfig,
    #[serde(default)]
    pub network: NetworkConfig,
    #[serde(default)]
    pub weather: WeatherConfig,
    #[serde(default)]
    pub system_stats: SystemStatsConfig,
    #[serde(default)]
    pub appearance: AppearanceConfig,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ClockConfig {
    #[serde(default = "default_time_format")]
    pub time_format: String,
    #[serde(default = "default_date_format")]
    pub date_format: String,
    #[serde(default = "default_true")]
    pub show_seconds: bool,
    #[serde(default = "default_true")]
    pub blink_separator: bool,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct SecondaryClockConfig {
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

#[derive(Debug, Serialize, Deserialize)]
pub struct CalendarConfig {
    #[serde(default = "default_true")]
    pub show_gregorian: bool,
}

/// Identifies which slot in the 5-slot layout grid a panel occupies.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Slot {
    Top,
    LeftTop,
    LeftBottom,
    RightTop,
    RightBottom,
}

impl Slot {
    pub const ALL: &[Self] = &[
        Self::Top,
        Self::LeftTop,
        Self::RightTop,
        Self::LeftBottom,
        Self::RightBottom,
    ];

    /// Which sizing field controls this slot's vertical size.
    pub fn height_field(self) -> SizingField {
        match self {
            Self::Top => SizingField::TopHeight,
            Self::LeftTop | Self::LeftBottom => SizingField::LeftSplit,
            Self::RightTop | Self::RightBottom => SizingField::RightSplit,
        }
    }

    /// Whether increasing the height field makes this slot taller.
    pub fn grow_means_taller(self) -> bool {
        matches!(self, Self::Top | Self::LeftTop | Self::RightTop)
    }

    /// Whether this slot has horizontal resize control (Top is full-width, so no).
    pub fn has_width_control(self) -> bool {
        !matches!(self, Self::Top)
    }

    /// Whether increasing `left_column_percent` makes this slot wider.
    pub fn grow_means_wider(self) -> bool {
        matches!(self, Self::LeftTop | Self::LeftBottom)
    }
}

/// Identifies a layout sizing field that can be adjusted.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SizingField {
    TopHeight,
    LeftColumn,
    LeftSplit,
    RightSplit,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct LayoutConfig {
    // Panel placement
    #[serde(default = "default_top_panel")]
    pub top: String,
    #[serde(default = "default_left_top_panel")]
    pub left_top: String,
    #[serde(default = "default_left_bottom_panel")]
    pub left_bottom: String,
    #[serde(default = "default_right_top_panel")]
    pub right_top: String,
    #[serde(default = "default_right_bottom_panel")]
    pub right_bottom: String,

    // Sizing (aliases for backwards compat with old field names)
    #[serde(default = "default_top_height", alias = "clock_height_percent")]
    pub top_height_percent: u16,
    #[serde(default = "default_bottom_height", alias = "info_height_percent")]
    pub bottom_height_percent: u16,
    #[serde(default = "default_column_split")]
    pub left_column_percent: u16,
    #[serde(default = "default_left_split", alias = "left_top_percent")]
    pub left_split_percent: u16,
    #[serde(default = "default_right_split", alias = "right_top_percent")]
    pub right_split_percent: u16,
}

impl LayoutConfig {
    /// Returns the panel config name assigned to a given slot.
    pub fn panel_at(&self, slot: Slot) -> &str {
        match slot {
            Slot::Top => &self.top,
            Slot::LeftTop => &self.left_top,
            Slot::LeftBottom => &self.left_bottom,
            Slot::RightTop => &self.right_top,
            Slot::RightBottom => &self.right_bottom,
        }
    }

    /// Sets the panel config name for a given slot.
    pub fn set_panel_at(&mut self, slot: Slot, name: String) {
        match slot {
            Slot::Top => self.top = name,
            Slot::LeftTop => self.left_top = name,
            Slot::LeftBottom => self.left_bottom = name,
            Slot::RightTop => self.right_top = name,
            Slot::RightBottom => self.right_bottom = name,
        }
    }

    /// Finds which slot a panel name is assigned to.
    pub fn slot_of(&self, panel_name: &str) -> Option<Slot> {
        Slot::ALL.iter().copied().find(|&s| self.panel_at(s) == panel_name)
    }

    /// Swaps the panel assignments of two slots.
    pub fn swap_slots(&mut self, a: Slot, b: Slot) {
        let name_a = self.panel_at(a).to_string();
        let name_b = self.panel_at(b).to_string();
        self.set_panel_at(a, name_b);
        self.set_panel_at(b, name_a);
    }

    /// Returns a mutable reference to the sizing value for a given field.
    pub fn sizing_field_mut(&mut self, field: SizingField) -> &mut u16 {
        match field {
            SizingField::TopHeight => &mut self.top_height_percent,
            SizingField::LeftColumn => &mut self.left_column_percent,
            SizingField::LeftSplit => &mut self.left_split_percent,
            SizingField::RightSplit => &mut self.right_split_percent,
        }
    }
}

#[derive(Debug, Serialize, Deserialize)]
pub struct NetworkConfig {
    #[serde(default = "default_ip_refresh")]
    pub ip_refresh_interval_minutes: u64,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct WeatherConfig {
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

#[derive(Debug, Serialize, Deserialize)]
pub struct SystemStatsConfig {
    #[serde(default = "default_true")]
    pub enabled: bool,
    #[serde(default = "default_stats_refresh")]
    pub refresh_interval_seconds: u64,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct AppearanceConfig {
    #[serde(default = "default_tick_rate")]
    pub tick_rate_ms: u64,
    #[serde(default = "default_font_style")]
    pub font_style: String,
}

// Default value functions for serde
fn default_true() -> bool { true }
fn default_time_format() -> String { constants::DEFAULT_TIME_FORMAT.to_string() }
fn default_secondary_timezone() -> String { constants::DEFAULT_SECONDARY_TIMEZONE.to_string() }
fn default_secondary_label() -> String { constants::DEFAULT_SECONDARY_LABEL.to_string() }
fn default_top_panel() -> String { constants::DEFAULT_TOP_PANEL.to_string() }
fn default_left_top_panel() -> String { constants::DEFAULT_LEFT_TOP_PANEL.to_string() }
fn default_left_bottom_panel() -> String { constants::DEFAULT_LEFT_BOTTOM_PANEL.to_string() }
fn default_right_top_panel() -> String { constants::DEFAULT_RIGHT_TOP_PANEL.to_string() }
fn default_right_bottom_panel() -> String { constants::DEFAULT_RIGHT_BOTTOM_PANEL.to_string() }
fn default_top_height() -> u16 { constants::DEFAULT_TOP_HEIGHT_PERCENT }
fn default_bottom_height() -> u16 { constants::DEFAULT_BOTTOM_HEIGHT_PERCENT }
fn default_column_split() -> u16 { constants::DEFAULT_LEFT_COLUMN_PERCENT }
fn default_left_split() -> u16 { constants::DEFAULT_LEFT_SPLIT_PERCENT }
fn default_right_split() -> u16 { constants::DEFAULT_RIGHT_SPLIT_PERCENT }
fn default_ip_refresh() -> u64 { constants::DEFAULT_IP_REFRESH_MINUTES }
fn default_latitude() -> f64 { constants::DEFAULT_LATITUDE }
fn default_longitude() -> f64 { constants::DEFAULT_LONGITUDE }
fn default_temp_unit() -> String { constants::DEFAULT_TEMP_UNIT.to_string() }
fn default_weather_refresh() -> u64 { constants::DEFAULT_WEATHER_REFRESH_MINUTES }
fn default_stats_refresh() -> u64 { constants::DEFAULT_STATS_REFRESH_SECONDS }
fn default_tick_rate() -> u64 { constants::DEFAULT_TICK_RATE.as_millis() as u64 }
fn default_font_style() -> String { constants::DEFAULT_FONT_STYLE.to_string() }
fn default_date_format() -> String { constants::DEFAULT_DATE_FORMAT.to_string() }
fn default_secondary_date_format() -> String { constants::DEFAULT_SECONDARY_DATE_FORMAT.to_string() }


impl Default for ClockConfig {
    fn default() -> Self {
        Self {
            time_format: default_time_format(),
            date_format: default_date_format(),
            show_seconds: true,
            blink_separator: true,
        }
    }
}

impl Default for SecondaryClockConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            timezone: default_secondary_timezone(),
            label: default_secondary_label(),
            time_format: default_time_format(),
            date_format: default_secondary_date_format(),
        }
    }
}

impl Default for CalendarConfig {
    fn default() -> Self {
        Self {
            show_gregorian: true,
        }
    }
}

impl Default for LayoutConfig {
    fn default() -> Self {
        Self {
            top: default_top_panel(),
            left_top: default_left_top_panel(),
            left_bottom: default_left_bottom_panel(),
            right_top: default_right_top_panel(),
            right_bottom: default_right_bottom_panel(),
            top_height_percent: default_top_height(),
            bottom_height_percent: default_bottom_height(),
            left_column_percent: default_column_split(),
            left_split_percent: default_left_split(),
            right_split_percent: default_right_split(),
        }
    }
}

impl Default for NetworkConfig {
    fn default() -> Self {
        Self {
            ip_refresh_interval_minutes: default_ip_refresh(),
        }
    }
}

impl Default for WeatherConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            latitude: default_latitude(),
            longitude: default_longitude(),
            temperature_unit: default_temp_unit(),
            refresh_interval_minutes: default_weather_refresh(),
        }
    }
}

impl Default for SystemStatsConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            refresh_interval_seconds: default_stats_refresh(),
        }
    }
}

impl Default for AppearanceConfig {
    fn default() -> Self {
        Self {
            tick_rate_ms: default_tick_rate(),
            font_style: default_font_style(),
        }
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

        let config = if path.exists() {
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
            info!("No config found, generating default at {}", path.display());
            let config = Self::default();
            if let Err(err) = config.save() {
                warn!("Could not write default config: {err}");
            }
            config
        };

        // Validate and return
        if let Err(err) = config.validate() {
            warn!("Config validation: {err}. Some values may be clamped.");
        }

        config
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

        if self.clock.time_format != "12h" && self.clock.time_format != "24h" {
            issues.push(format!(
                "clock.time_format must be \"12h\" or \"24h\", got \"{}\"",
                self.clock.time_format
            ));
        }

        if self.layout.top_height_percent > 90 {
            issues.push("layout.top_height_percent must be <= 90".to_string());
        }
        if self.layout.bottom_height_percent > 90 {
            issues.push("layout.bottom_height_percent must be <= 90".to_string());
        }
        if self.layout.left_column_percent == 0 || self.layout.left_column_percent > 90 {
            issues.push("layout.left_column_percent must be 1-90".to_string());
        }

        // Check for duplicate panel assignments
        let panels: Vec<&str> = Slot::ALL.iter().map(|&s| self.layout.panel_at(s)).collect();
        for (i, a) in panels.iter().enumerate() {
            for b in &panels[i + 1..] {
                if a == b {
                    issues.push(format!("panel \"{a}\" is assigned to multiple slots"));
                    break;
                }
            }
        }

        if self.weather.temperature_unit != "celsius"
            && self.weather.temperature_unit != "fahrenheit"
        {
            issues.push(format!(
                "weather.temperature_unit must be \"celsius\" or \"fahrenheit\", got \"{}\"",
                self.weather.temperature_unit
            ));
        }

        if self.weather.latitude < -90.0 || self.weather.latitude > 90.0 {
            issues.push("weather.latitude must be between -90 and 90".to_string());
        }
        if self.weather.longitude < -180.0 || self.weather.longitude > 180.0 {
            issues.push("weather.longitude must be between -180 and 180".to_string());
        }

        if self.appearance.tick_rate_ms < constants::MIN_TICK_RATE_MS {
            issues.push(format!(
                "appearance.tick_rate_ms must be >= {}",
                constants::MIN_TICK_RATE_MS
            ));
        }

        if self.secondary_clock.enabled
            && self.secondary_clock.timezone.parse::<chrono_tz::Tz>().is_err()
        {
            issues.push(format!(
                "secondary_clock.timezone \"{}\" is not a valid timezone",
                self.secondary_clock.timezone
            ));
        }

        if issues.is_empty() {
            Ok(())
        } else {
            Err(NexClockError::Config(issues.join("; ")))
        }
    }

    /// Returns the tick rate as a Duration.
    pub fn tick_rate(&self) -> std::time::Duration {
        std::time::Duration::from_millis(self.appearance.tick_rate_ms.max(constants::MIN_TICK_RATE_MS))
    }
}
