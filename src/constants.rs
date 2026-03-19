use std::time::Duration;

use ratatui::style::Color;

// Tick / refresh rates
pub const DEFAULT_TICK_RATE: Duration = Duration::from_millis(250);
pub const MIN_TICK_RATE_MS: u64 = 50;

// Clock display
pub const DEFAULT_TIME_FORMAT: &str = "24h";

// Secondary clock
pub const DEFAULT_SECONDARY_TIMEZONE: &str = "US/Eastern";
pub const DEFAULT_SECONDARY_LABEL: &str = "New York";

// Layout defaults (percentages)
pub const DEFAULT_CLOCK_HEIGHT_PERCENT: u16 = 40;
pub const DEFAULT_INFO_HEIGHT_PERCENT: u16 = 50;
pub const DEFAULT_LEFT_COLUMN_PERCENT: u16 = 50;
pub const STATUS_BAR_HEIGHT: u16 = 3;

// Minimum terminal size before showing a "too small" message
pub const MIN_TERMINAL_WIDTH: u16 = 40;
pub const MIN_TERMINAL_HEIGHT: u16 = 10;

// Network defaults
pub const DEFAULT_IP_REFRESH_MINUTES: u64 = 10;

// Weather defaults
pub const DEFAULT_LATITUDE: f64 = 35.6892;
pub const DEFAULT_LONGITUDE: f64 = 51.3890;
pub const DEFAULT_TEMP_UNIT: &str = "celsius";
pub const DEFAULT_WEATHER_REFRESH_MINUTES: u64 = 30;

// System stats defaults
pub const DEFAULT_STATS_REFRESH_SECONDS: u64 = 5;

// Focus / interaction
pub const FOCUS_BORDER_COLOR: Color = Color::Cyan;
pub const RESIZE_STEP_PERCENT: u16 = 5;
pub const MIN_PANEL_PERCENT: u16 = 10;
pub const MAX_PANEL_PERCENT: u16 = 80;
pub const DEFAULT_FONT_STYLE: &str = "Standard";
