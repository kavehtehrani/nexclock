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
}

#[derive(Debug, Serialize, Deserialize)]
pub struct CalendarConfig {
    #[serde(default = "default_true")]
    pub show_gregorian: bool,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct LayoutConfig {
    #[serde(default = "default_clock_height")]
    pub clock_height_percent: u16,
    #[serde(default = "default_info_height")]
    pub info_height_percent: u16,
    #[serde(default = "default_column_split")]
    pub left_column_percent: u16,
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
}

// Default value functions for serde
fn default_true() -> bool { true }
fn default_time_format() -> String { constants::DEFAULT_TIME_FORMAT.to_string() }
fn default_secondary_timezone() -> String { constants::DEFAULT_SECONDARY_TIMEZONE.to_string() }
fn default_secondary_label() -> String { constants::DEFAULT_SECONDARY_LABEL.to_string() }
fn default_clock_height() -> u16 { constants::DEFAULT_CLOCK_HEIGHT_PERCENT }
fn default_info_height() -> u16 { constants::DEFAULT_INFO_HEIGHT_PERCENT }
fn default_column_split() -> u16 { constants::DEFAULT_LEFT_COLUMN_PERCENT }
fn default_ip_refresh() -> u64 { constants::DEFAULT_IP_REFRESH_MINUTES }
fn default_latitude() -> f64 { constants::DEFAULT_LATITUDE }
fn default_longitude() -> f64 { constants::DEFAULT_LONGITUDE }
fn default_temp_unit() -> String { constants::DEFAULT_TEMP_UNIT.to_string() }
fn default_weather_refresh() -> u64 { constants::DEFAULT_WEATHER_REFRESH_MINUTES }
fn default_stats_refresh() -> u64 { constants::DEFAULT_STATS_REFRESH_SECONDS }
fn default_tick_rate() -> u64 { constants::DEFAULT_TICK_RATE.as_millis() as u64 }


impl Default for ClockConfig {
    fn default() -> Self {
        Self {
            time_format: default_time_format(),
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
            clock_height_percent: default_clock_height(),
            info_height_percent: default_info_height(),
            left_column_percent: default_column_split(),
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
    fn save(&self) -> Result<(), Box<dyn std::error::Error>> {
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

        if self.layout.clock_height_percent > 90 {
            issues.push("layout.clock_height_percent must be <= 90".to_string());
        }
        if self.layout.info_height_percent > 90 {
            issues.push("layout.info_height_percent must be <= 90".to_string());
        }
        if self.layout.left_column_percent == 0 || self.layout.left_column_percent > 90 {
            issues.push("layout.left_column_percent must be 1-90".to_string());
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
