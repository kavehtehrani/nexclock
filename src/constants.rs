use std::time::Duration;

// Tick / refresh rates
pub const DEFAULT_TICK_RATE: Duration = Duration::from_millis(250);

// Clock display
pub const DEFAULT_TIME_FORMAT_24H: bool = true;
pub const DEFAULT_SHOW_SECONDS: bool = true;
pub const DEFAULT_BLINK_SEPARATOR: bool = true;

// Layout defaults (percentages)
pub const DEFAULT_CLOCK_HEIGHT_PERCENT: u16 = 40;
pub const DEFAULT_INFO_HEIGHT_PERCENT: u16 = 50;
pub const DEFAULT_LEFT_COLUMN_PERCENT: u16 = 50;
pub const STATUS_BAR_HEIGHT: u16 = 3;

// tui-big-text pixel size thresholds (minimum height in rows)
pub const PIXEL_SIZE_FULL_MIN_HEIGHT: u16 = 8;
pub const PIXEL_SIZE_HALF_MIN_HEIGHT: u16 = 4;
// Below HALF we fall back to Quadrant (needs ~2 rows)
