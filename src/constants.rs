use std::time::Duration;

// Tick / refresh rates
pub const DEFAULT_TICK_RATE: Duration = Duration::from_millis(250);

// Clock display
pub const DEFAULT_TIME_FORMAT: &str = "24h";
pub const DEFAULT_BLINK_SEPARATOR: bool = true;

// Secondary clock
pub const DEFAULT_SECONDARY_TIMEZONE: &str = "US/Eastern";
pub const DEFAULT_SECONDARY_LABEL: &str = "New York";

// Layout defaults (percentages)
pub const DEFAULT_CLOCK_HEIGHT_PERCENT: u16 = 40;
pub const DEFAULT_INFO_HEIGHT_PERCENT: u16 = 50;
pub const DEFAULT_LEFT_COLUMN_PERCENT: u16 = 50;
pub const STATUS_BAR_HEIGHT: u16 = 3;

// tui-big-text pixel size thresholds (minimum height in rows)
pub const PIXEL_SIZE_FULL_MIN_HEIGHT: u16 = 8;
pub const PIXEL_SIZE_HALF_MIN_HEIGHT: u16 = 4;

// Network defaults
pub const DEFAULT_IP_REFRESH_MINUTES: u64 = 10;

// Weather defaults
pub const DEFAULT_LATITUDE: f64 = 35.6892;
pub const DEFAULT_LONGITUDE: f64 = 51.3890;
pub const DEFAULT_TEMP_UNIT: &str = "celsius";
pub const DEFAULT_WEATHER_REFRESH_MINUTES: u64 = 30;

// System stats defaults
pub const DEFAULT_STATS_REFRESH_SECONDS: u64 = 5;
