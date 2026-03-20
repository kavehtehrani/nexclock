//! Shared serde default-value functions used by both `config.rs` and `component.rs`.

use crate::constants;

pub fn default_true() -> bool {
    true
}
pub fn default_time_format() -> String {
    constants::DEFAULT_TIME_FORMAT.to_string()
}
pub fn default_date_format() -> String {
    constants::DEFAULT_DATE_FORMAT.to_string()
}
pub fn default_font_style() -> String {
    constants::DEFAULT_FONT_STYLE.to_string()
}
pub fn default_latitude() -> f64 {
    constants::DEFAULT_LATITUDE
}
pub fn default_longitude() -> f64 {
    constants::DEFAULT_LONGITUDE
}
pub fn default_temp_unit() -> String {
    constants::DEFAULT_TEMP_UNIT.to_string()
}
pub fn default_weather_refresh() -> u64 {
    constants::DEFAULT_WEATHER_REFRESH_MINUTES
}
pub fn default_stats_refresh() -> u64 {
    constants::DEFAULT_STATS_REFRESH_SECONDS
}
