use std::time::Duration;

// Tick / refresh rates
pub const DEFAULT_TICK_RATE: Duration = Duration::from_millis(250);
pub const MIN_TICK_RATE_MS: u64 = 50;

// Clock display
pub const DEFAULT_TIME_FORMAT: &str = "24h";

// Status bar
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

// Font
pub const DEFAULT_FONT_STYLE: &str = "Standard";

// Date format
pub const DEFAULT_DATE_FORMAT: &str = "%A, %B %d, %Y";
pub const DEFAULT_SECONDARY_DATE_FORMAT: &str = "%a, %b %d";
pub const DATE_FORMAT_PRESETS: &[&str] = &[
    "%A, %B %d, %Y", // Wednesday, March 19, 2026
    "%Y-%m-%d",       // 2026-03-19
    "%d/%m/%Y",       // 19/03/2026
    "%B %d, %Y",      // March 19, 2026
    "%d %b %Y",       // 19 Mar 2026
];

// Grid defaults
pub const DEFAULT_GRID_ROWS: u16 = 3;
pub const DEFAULT_GRID_COLUMNS: u16 = 2;

// Theme color defaults
pub const DEFAULT_THEME_PRIMARY: &str = "cyan";
pub const DEFAULT_THEME_SECONDARY: &str = "yellow";
pub const DEFAULT_THEME_TERTIARY: &str = "green";
pub const DEFAULT_THEME_INFO: &str = "blue";
pub const DEFAULT_THEME_MUTED: &str = "dark_gray";
pub const DEFAULT_THEME_TEXT: &str = "white";
pub const DEFAULT_THEME_ERROR: &str = "red";
pub const DEFAULT_THEME_FOCUS: &str = "cyan";
