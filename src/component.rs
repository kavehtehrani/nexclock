use serde::{Deserialize, Serialize};

use crate::defaults::{
    default_date_format, default_font_style, default_latitude, default_longitude,
    default_stats_refresh, default_temp_unit, default_time_format, default_true,
    default_weather_refresh,
};

// ── Component type identification ───────────────────────────────────

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ComponentType {
    Clock,
    Weather,
    Calendar,
    SystemStats,
}

impl ComponentType {
    pub fn label(self) -> &'static str {
        match self {
            Self::Clock => "Clock",
            Self::Weather => "Weather",
            Self::Calendar => "Calendar",
            Self::SystemStats => "System Stats",
        }
    }

    pub fn type_name(self) -> &'static str {
        match self {
            Self::Clock => "clock",
            Self::Weather => "weather",
            Self::Calendar => "calendar",
            Self::SystemStats => "system_stats",
        }
    }

    pub fn from_name(name: &str) -> Option<Self> {
        match name {
            "clock" => Some(Self::Clock),
            "weather" => Some(Self::Weather),
            "calendar" => Some(Self::Calendar),
            "system_stats" => Some(Self::SystemStats),
            _ => None,
        }
    }
}

// ── Grid placement ──────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GridPlacement {
    pub row: u16,
    pub column: u16,
    #[serde(default = "default_span")]
    pub row_span: u16,
    #[serde(default = "default_span")]
    pub col_span: u16,
}

fn default_span() -> u16 {
    1
}

// ── Clock style ─────────────────────────────────────────────────────

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
#[derive(Default)]
pub enum ClockStyle {
    #[default]
    Large,
    Compact,
}


// ── Per-component settings ──────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ClockSettings {
    #[serde(default)]
    pub style: ClockStyle,
    #[serde(default = "default_time_format")]
    pub time_format: String,
    #[serde(default = "default_date_format")]
    pub date_format: String,
    #[serde(default = "default_true")]
    pub show_seconds: bool,
    #[serde(default = "default_true")]
    pub blink_separator: bool,
    #[serde(default)]
    pub timezone: Option<String>,
    #[serde(default)]
    pub label: Option<String>,
    #[serde(default = "default_font_style")]
    pub font_style: String,
    #[serde(default)]
    pub colors: Vec<String>,
}

impl Default for ClockSettings {
    fn default() -> Self {
        Self {
            style: ClockStyle::default(),
            time_format: default_time_format(),
            date_format: default_date_format(),
            show_seconds: true,
            blink_separator: true,
            timezone: None,
            label: None,
            font_style: default_font_style(),
            colors: Vec::new(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WeatherSettings {
    #[serde(default = "default_latitude")]
    pub latitude: f64,
    #[serde(default = "default_longitude")]
    pub longitude: f64,
    #[serde(default = "default_temp_unit")]
    pub temperature_unit: String,
    #[serde(default = "default_weather_refresh")]
    pub refresh_interval_minutes: u64,
}

impl Default for WeatherSettings {
    fn default() -> Self {
        Self {
            latitude: default_latitude(),
            longitude: default_longitude(),
            temperature_unit: default_temp_unit(),
            refresh_interval_minutes: default_weather_refresh(),
        }
    }
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct CalendarSettings {
    // Future: calendar_type for Gregorian vs Persian
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SystemStatsSettings {
    #[serde(default = "default_stats_refresh")]
    pub refresh_interval_seconds: u64,
}

impl Default for SystemStatsSettings {
    fn default() -> Self {
        Self {
            refresh_interval_seconds: default_stats_refresh(),
        }
    }
}

// ── Component config enum ───────────────────────────────────────────

#[derive(Debug, Clone)]
pub enum ComponentConfig {
    Clock(ClockSettings),
    Weather(WeatherSettings),
    Calendar(CalendarSettings),
    SystemStats(SystemStatsSettings),
}

impl ComponentConfig {
    pub fn component_type(&self) -> ComponentType {
        match self {
            Self::Clock(_) => ComponentType::Clock,
            Self::Weather(_) => ComponentType::Weather,
            Self::Calendar(_) => ComponentType::Calendar,
            Self::SystemStats(_) => ComponentType::SystemStats,
        }
    }
}

// ── Component entry ─────────────────────────────────────────────────

#[derive(Debug, Clone)]
pub struct ComponentEntry {
    pub id: String,
    pub placement: GridPlacement,
    pub config: ComponentConfig,
    pub visible: bool,
}

impl ComponentEntry {
    /// Creates a default component for the given type at the specified grid position.
    pub fn default_for_type(comp_type: ComponentType, row: u16, col: u16) -> Self {
        let id = generate_id(comp_type);
        let config = match comp_type {
            ComponentType::Clock => ComponentConfig::Clock(ClockSettings {
                style: ClockStyle::Compact,
                ..ClockSettings::default()
            }),
            ComponentType::Weather => ComponentConfig::Weather(WeatherSettings::default()),
            ComponentType::Calendar => ComponentConfig::Calendar(CalendarSettings::default()),
            ComponentType::SystemStats => {
                ComponentConfig::SystemStats(SystemStatsSettings::default())
            }
        };
        Self {
            id,
            placement: GridPlacement {
                row,
                column: col,
                row_span: 1,
                col_span: 1,
            },
            config,
            visible: true,
        }
    }

    /// Serializes this component entry back into a TOML table for config persistence.
    pub fn to_toml_table(&self) -> toml::Table {
        let mut table = toml::Table::new();

        // Type field
        table.insert(
            "type".to_string(),
            toml::Value::String(self.config.component_type().type_name().to_string()),
        );

        // Placement fields
        table.insert(
            "row".to_string(),
            toml::Value::Integer(i64::from(self.placement.row)),
        );
        table.insert(
            "column".to_string(),
            toml::Value::Integer(i64::from(self.placement.column)),
        );
        if self.placement.row_span != 1 {
            table.insert(
                "row_span".to_string(),
                toml::Value::Integer(i64::from(self.placement.row_span)),
            );
        }
        if self.placement.col_span != 1 {
            table.insert(
                "col_span".to_string(),
                toml::Value::Integer(i64::from(self.placement.col_span)),
            );
        }

        if !self.visible {
            table.insert("visible".to_string(), toml::Value::Boolean(false));
        }

        // Type-specific settings
        match &self.config {
            ComponentConfig::Clock(s) => {
                table.insert(
                    "style".to_string(),
                    toml::Value::String(match s.style {
                        ClockStyle::Large => "large".to_string(),
                        ClockStyle::Compact => "compact".to_string(),
                    }),
                );
                table.insert(
                    "time_format".to_string(),
                    toml::Value::String(s.time_format.clone()),
                );
                table.insert(
                    "date_format".to_string(),
                    toml::Value::String(s.date_format.clone()),
                );
                table.insert(
                    "show_seconds".to_string(),
                    toml::Value::Boolean(s.show_seconds),
                );
                table.insert(
                    "blink_separator".to_string(),
                    toml::Value::Boolean(s.blink_separator),
                );
                table.insert(
                    "font_style".to_string(),
                    toml::Value::String(s.font_style.clone()),
                );
                if let Some(tz) = &s.timezone {
                    table.insert("timezone".to_string(), toml::Value::String(tz.clone()));
                }
                if let Some(label) = &s.label {
                    table.insert("label".to_string(), toml::Value::String(label.clone()));
                }
                if !s.colors.is_empty() {
                    table.insert(
                        "colors".to_string(),
                        toml::Value::Array(
                            s.colors.iter().map(|c| toml::Value::String(c.clone())).collect(),
                        ),
                    );
                }
            }
            ComponentConfig::Weather(s) => {
                table.insert(
                    "latitude".to_string(),
                    toml::Value::Float(s.latitude),
                );
                table.insert(
                    "longitude".to_string(),
                    toml::Value::Float(s.longitude),
                );
                table.insert(
                    "temperature_unit".to_string(),
                    toml::Value::String(s.temperature_unit.clone()),
                );
                table.insert(
                    "refresh_interval_minutes".to_string(),
                    toml::Value::Integer(s.refresh_interval_minutes as i64),
                );
            }
            ComponentConfig::Calendar(_) => {
                // No extra fields yet
            }
            ComponentConfig::SystemStats(s) => {
                table.insert(
                    "refresh_interval_seconds".to_string(),
                    toml::Value::Integer(s.refresh_interval_seconds as i64),
                );
            }
        }

        table
    }
}

// ── Parsing ─────────────────────────────────────────────────────────

/// Parses a component from a raw TOML table. The `id` is the map key from `[components.ID]`.
pub fn parse_component(id: &str, table: &toml::Table) -> Result<ComponentEntry, String> {
    let type_str = table
        .get("type")
        .and_then(|v| v.as_str())
        .ok_or_else(|| format!("component '{id}' missing 'type' field"))?;

    let comp_type = ComponentType::from_name(type_str)
        .ok_or_else(|| format!("component '{id}' has unknown type '{type_str}'"))?;

    // Parse placement
    let row = table
        .get("row")
        .and_then(|v| v.as_integer())
        .unwrap_or(0) as u16;
    let column = table
        .get("column")
        .and_then(|v| v.as_integer())
        .unwrap_or(0) as u16;
    let row_span = table
        .get("row_span")
        .and_then(|v| v.as_integer())
        .unwrap_or(1) as u16;
    let col_span = table
        .get("col_span")
        .and_then(|v| v.as_integer())
        .unwrap_or(1) as u16;

    let visible = table
        .get("visible")
        .and_then(|v| v.as_bool())
        .unwrap_or(true);

    let placement = GridPlacement {
        row,
        column,
        row_span,
        col_span,
    };

    // Parse type-specific settings by deserializing the table.
    // serde will ignore unknown fields like "type", "row", etc.
    let value = toml::Value::Table(table.clone());
    let config = match comp_type {
        ComponentType::Clock => {
            let settings: ClockSettings =
                value.try_into().map_err(|e| format!("component '{id}': {e}"))?;
            ComponentConfig::Clock(settings)
        }
        ComponentType::Weather => {
            let settings: WeatherSettings =
                value.try_into().map_err(|e| format!("component '{id}': {e}"))?;
            ComponentConfig::Weather(settings)
        }
        ComponentType::Calendar => {
            let settings: CalendarSettings =
                value.try_into().map_err(|e| format!("component '{id}': {e}"))?;
            ComponentConfig::Calendar(settings)
        }
        ComponentType::SystemStats => {
            let settings: SystemStatsSettings =
                value.try_into().map_err(|e| format!("component '{id}': {e}"))?;
            ComponentConfig::SystemStats(settings)
        }
    };

    Ok(ComponentEntry {
        id: id.to_string(),
        placement,
        config,
        visible,
    })
}

// ── Helpers ─────────────────────────────────────────────────────────

fn generate_id(comp_type: ComponentType) -> String {
    use std::time::{SystemTime, UNIX_EPOCH};
    let ts = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis();
    format!("{}_{}", comp_type.type_name(), ts)
}


/// Returns true if two grid rectangles (row, col, row_span, col_span) overlap.
pub fn rects_overlap(a: (u16, u16, u16, u16), b: (u16, u16, u16, u16)) -> bool {
    a.0 < b.0 + b.2 && a.0 + a.2 > b.0 && a.1 < b.1 + b.3 && a.1 + a.3 > b.1
}

/// Finds the first empty cell in the grid that is not occupied by any existing component.
/// Returns (row, col) or None if the grid is fully packed.
pub fn find_empty_cell(
    components: &[ComponentEntry],
    grid_rows: u16,
    grid_cols: u16,
) -> Option<(u16, u16)> {
    // Build occupancy grid
    let mut occupied = vec![vec![false; grid_cols as usize]; grid_rows as usize];
    for comp in components.iter().filter(|c| c.visible) {
        let p = &comp.placement;
        for r in p.row..p.row + p.row_span {
            for c in p.column..p.column + p.col_span {
                if (r as usize) < occupied.len() && (c as usize) < occupied[0].len() {
                    occupied[r as usize][c as usize] = true;
                }
            }
        }
    }

    // Scan row-major for first empty cell
    for r in 0..grid_rows {
        for c in 0..grid_cols {
            if !occupied[r as usize][c as usize] {
                return Some((r, c));
            }
        }
    }
    None
}
