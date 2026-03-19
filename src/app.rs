use std::collections::HashMap;

use ratatui::layout::Rect;
use ratatui::style::Color;
use tokio::sync::watch;
use tracing::{error, warn};

use crate::component::{
    find_empty_cell, ClockStyle, ComponentConfig, ComponentEntry, ComponentType,
};
use crate::config::{AppConfig, ThemeConfig};
use crate::constants;
use crate::data::system::{self, SystemStats};
use crate::data::weather_api::WeatherData;
use crate::data::{ip, weather_api};

// ── UI mode ─────────────────────────────────────────────────────────

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum UiMode {
    Normal,
    /// Arrow keys resize, Shift+arrows move the focused component.
    EditMode,
    ContextMenu,
    VisibilityMenu,
    AddComponentMenu,
    Help,
}

// ── Menu action ─────────────────────────────────────────────────────

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum MenuAction {
    ToggleTimeFormat,
    ToggleSeconds,
    ToggleBlink,
    CycleDateFormat,
    SpanMoreRows,
    SpanFewerRows,
    SpanMoreCols,
    SpanFewerCols,
    Remove,
}

const RESIZE_STEP: u16 = 5;
const MIN_ROW_HEIGHT_PCT: u16 = 10;
const MIN_COL_WIDTH_PCT: u16 = 10;

pub struct ContextMenuItem {
    pub label: String,
    pub action: MenuAction,
}

// ── Font style ──────────────────────────────────────────────────────

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FontStyle {
    Standard,
    Big,
    Small,
    Slant,
    SmBlock,
    Mono12,
    Future,
    Wideterm,
    Mono9,
}

impl FontStyle {
    const ALL: &[Self] = &[
        Self::Standard,
        Self::Big,
        Self::Small,
        Self::Slant,
        Self::SmBlock,
        Self::Mono12,
        Self::Future,
        Self::Wideterm,
        Self::Mono9,
    ];

    pub fn next(self) -> Self {
        let idx = Self::ALL.iter().position(|&s| s == self).unwrap_or(0);
        Self::ALL[(idx + 1) % Self::ALL.len()]
    }

    pub fn prev(self) -> Self {
        let idx = Self::ALL.iter().position(|&s| s == self).unwrap_or(0);
        Self::ALL[(idx + Self::ALL.len() - 1) % Self::ALL.len()]
    }

    pub fn name(self) -> &'static str {
        match self {
            Self::Standard => "Standard",
            Self::Big => "Big",
            Self::Small => "Small",
            Self::Slant => "Slant",
            Self::SmBlock => "SmBlock",
            Self::Mono12 => "Mono12",
            Self::Future => "Future",
            Self::Wideterm => "Wideterm",
            Self::Mono9 => "Mono9",
        }
    }

    pub fn from_name(name: &str) -> Self {
        Self::ALL
            .iter()
            .find(|s| s.name().eq_ignore_ascii_case(name))
            .copied()
            .unwrap_or(Self::Standard)
    }
}

// ── Resolved theme ──────────────────────────────────────────────────

#[derive(Debug, Clone)]
pub struct ResolvedTheme {
    pub primary: Color,
    pub secondary: Color,
    pub tertiary: Color,
    pub info: Color,
    pub muted: Color,
    pub text: Color,
    pub error: Color,
    pub focus: Color,
}

impl ResolvedTheme {
    pub fn from_config(cfg: &ThemeConfig) -> Self {
        Self {
            primary: parse_color(&cfg.primary),
            secondary: parse_color(&cfg.secondary),
            tertiary: parse_color(&cfg.tertiary),
            info: parse_color(&cfg.info),
            muted: parse_color(&cfg.muted),
            text: parse_color(&cfg.text),
            error: parse_color(&cfg.error),
            focus: parse_color(&cfg.focus),
        }
    }
}

/// Parses a color string into a ratatui Color. Supports named colors and hex (#RRGGBB).
pub fn parse_color(s: &str) -> Color {
    if let Some(hex) = s.strip_prefix('#')
        && hex.len() == 6
            && let (Ok(r), Ok(g), Ok(b)) = (
                u8::from_str_radix(&hex[0..2], 16),
                u8::from_str_radix(&hex[2..4], 16),
                u8::from_str_radix(&hex[4..6], 16),
            ) {
                return Color::Rgb(r, g, b);
            }

    match s.to_lowercase().as_str() {
        "black" => Color::Black,
        "red" => Color::Red,
        "green" => Color::Green,
        "yellow" => Color::Yellow,
        "blue" => Color::Blue,
        "magenta" => Color::Magenta,
        "cyan" => Color::Cyan,
        "white" => Color::White,
        "gray" | "grey" => Color::Gray,
        "dark_gray" | "dark_grey" | "darkgray" | "darkgrey" => Color::DarkGray,
        "light_red" | "lightred" => Color::LightRed,
        "light_green" | "lightgreen" => Color::LightGreen,
        "light_yellow" | "lightyellow" => Color::LightYellow,
        "light_blue" | "lightblue" => Color::LightBlue,
        "light_magenta" | "lightmagenta" => Color::LightMagenta,
        "light_cyan" | "lightcyan" => Color::LightCyan,
        _ => Color::White, // fallback
    }
}

// ── Component runtime ───────────────────────────────────────────────

/// Per-component runtime state (data receivers, rendered area, etc.).
pub enum ComponentRuntime {
    Clock {
        font_style: FontStyle,
        area: Rect,
    },
    Weather {
        data_rx: watch::Receiver<Option<WeatherData>>,
        area: Rect,
    },
    Calendar {
        area: Rect,
    },
    SystemStats {
        stats_rx: watch::Receiver<SystemStats>,
        area: Rect,
    },
}

impl ComponentRuntime {
    pub fn area(&self) -> Rect {
        match self {
            Self::Clock { area, .. } => *area,
            Self::Weather { area, .. } => *area,
            Self::Calendar { area } => *area,
            Self::SystemStats { area, .. } => *area,
        }
    }

    pub fn set_area(&mut self, new_area: Rect) {
        match self {
            Self::Clock { area, .. } => *area = new_area,
            Self::Weather { area, .. } => *area = new_area,
            Self::Calendar { area } => *area = new_area,
            Self::SystemStats { area, .. } => *area = new_area,
        }
    }
}

// ── App ─────────────────────────────────────────────────────────────

pub struct App {
    pub running: bool,
    pub tick_count: u64,
    pub config: AppConfig,
    pub theme: ResolvedTheme,
    pub components: Vec<ComponentEntry>,
    pub runtime: HashMap<String, ComponentRuntime>,

    pub ip_rx: watch::Receiver<Option<String>>,

    pub focused_index: usize,
    pub ui_mode: UiMode,
    pub context_menu_items: Vec<ContextMenuItem>,
    pub menu_cursor: usize,
}

impl App {
    pub fn new(config: AppConfig, ip_rx: watch::Receiver<Option<String>>) -> Self {
        let theme = ResolvedTheme::from_config(&config.theme);
        let components = config.parse_components();

        let mut runtime = HashMap::new();
        for entry in &components {
            let rt = spawn_component_runtime(entry);
            runtime.insert(entry.id.clone(), rt);
        }

        Self {
            running: true,
            tick_count: 0,
            config,
            theme,
            components,
            runtime,
            ip_rx,
            focused_index: 0,
            ui_mode: UiMode::Normal,
            context_menu_items: Vec::new(),
            menu_cursor: 0,
        }
    }

    pub fn tick(&mut self) {
        self.tick_count = self.tick_count.wrapping_add(1);
    }

    pub fn quit(&mut self) {
        self.running = false;
    }

    pub fn external_ip(&self) -> Option<String> {
        self.ip_rx.borrow().clone()
    }

    /// Returns visible component indices sorted by reading order (row, col).
    pub fn visible_components(&self) -> Vec<usize> {
        let mut indices: Vec<usize> = self
            .components
            .iter()
            .enumerate()
            .filter(|(_, c)| c.visible)
            .map(|(i, _)| i)
            .collect();
        indices.sort_by_key(|&i| {
            let p = &self.components[i].placement;
            (p.row, p.column)
        });
        indices
    }

    /// Returns the currently focused component, if any.
    pub fn focused_component(&self) -> Option<&ComponentEntry> {
        let vis = self.visible_components();
        vis.get(self.focused_index)
            .map(|&i| &self.components[i])
    }

    /// Returns the actual component vector index of the focused component.
    pub fn focused_component_idx(&self) -> Option<usize> {
        let vis = self.visible_components();
        vis.get(self.focused_index).copied()
    }

    pub fn focus_next(&mut self) {
        let vis = self.visible_components();
        if vis.is_empty() {
            return;
        }
        self.focused_index = (self.focused_index + 1) % vis.len();
    }

    pub fn focus_prev(&mut self) {
        let vis = self.visible_components();
        if vis.is_empty() {
            return;
        }
        self.focused_index = (self.focused_index + vis.len() - 1) % vis.len();
    }

    /// Spatial navigation: find the nearest component in the given direction.
    pub fn focus_direction(&mut self, dr: i16, dc: i16) {
        let vis = self.visible_components();
        if vis.is_empty() {
            return;
        }
        let Some(&cur_idx) = vis.get(self.focused_index) else {
            return;
        };
        let cur = &self.components[cur_idx];
        let cur_r = cur.placement.row as i16;
        let cur_c = cur.placement.column as i16;

        let mut best: Option<(usize, i16)> = None;

        for (vi, &ci) in vis.iter().enumerate() {
            if ci == cur_idx {
                continue;
            }
            let c = &self.components[ci];
            let r = c.placement.row as i16;
            let col = c.placement.column as i16;

            let row_diff = r - cur_r;
            let col_diff = col - cur_c;

            // Check if this component is in the right direction
            let in_direction = match (dr, dc) {
                (-1, 0) => row_diff < 0,  // Up
                (1, 0) => row_diff > 0,   // Down
                (0, -1) => col_diff < 0,  // Left
                (0, 1) => col_diff > 0,   // Right
                _ => false,
            };

            if !in_direction {
                continue;
            }

            let dist = row_diff.abs() + col_diff.abs();
            if best.is_none() || dist < best.unwrap().1 {
                best = Some((vi, dist));
            }
        }

        if let Some((vi, _)) = best {
            self.focused_index = vi;
        }
    }

    // ── Context menu ────────────────────────────────────────────────

    pub fn open_context_menu(&mut self) {
        let Some(comp) = self.focused_component() else {
            return;
        };

        let mut items = Vec::new();

        // Type-specific actions
        if let ComponentConfig::Clock(s) = &comp.config {
            items.push(ContextMenuItem {
                label: "Toggle 12h/24h".into(),
                action: MenuAction::ToggleTimeFormat,
            });
            items.push(ContextMenuItem {
                label: "Cycle date format".into(),
                action: MenuAction::CycleDateFormat,
            });
            if s.style == ClockStyle::Large {
                items.push(ContextMenuItem {
                    label: "Toggle seconds".into(),
                    action: MenuAction::ToggleSeconds,
                });
                items.push(ContextMenuItem {
                    label: "Toggle blink".into(),
                    action: MenuAction::ToggleBlink,
                });
            }
        }

        // Span controls (merge/unmerge cells)
        if comp.placement.row + comp.placement.row_span < self.config.grid.rows {
            items.push(ContextMenuItem {
                label: "Span more rows".into(),
                action: MenuAction::SpanMoreRows,
            });
        }
        if comp.placement.row_span > 1 {
            items.push(ContextMenuItem {
                label: "Span fewer rows".into(),
                action: MenuAction::SpanFewerRows,
            });
        }
        if comp.placement.column + comp.placement.col_span < self.config.grid.columns {
            items.push(ContextMenuItem {
                label: "Span more cols".into(),
                action: MenuAction::SpanMoreCols,
            });
        }
        if comp.placement.col_span > 1 {
            items.push(ContextMenuItem {
                label: "Span fewer cols".into(),
                action: MenuAction::SpanFewerCols,
            });
        }

        // Remove
        items.push(ContextMenuItem {
            label: "Remove".into(),
            action: MenuAction::Remove,
        });

        self.context_menu_items = items;
        self.menu_cursor = 0;
        self.ui_mode = UiMode::ContextMenu;
    }

    pub fn execute_menu_action(&mut self, action: MenuAction) {
        let Some(idx) = self.focused_component_idx() else {
            self.ui_mode = UiMode::Normal;
            return;
        };

        match action {
            MenuAction::ToggleTimeFormat => {
                if let ComponentConfig::Clock(ref mut s) = self.components[idx].config {
                    s.time_format = if s.time_format == "24h" {
                        "12h".to_string()
                    } else {
                        "24h".to_string()
                    };
                }
            }
            MenuAction::CycleDateFormat => {
                if let ComponentConfig::Clock(ref mut s) = self.components[idx].config {
                    s.date_format = next_date_preset(&s.date_format);
                }
            }
            MenuAction::ToggleSeconds => {
                if let ComponentConfig::Clock(ref mut s) = self.components[idx].config {
                    s.show_seconds = !s.show_seconds;
                }
            }
            MenuAction::ToggleBlink => {
                if let ComponentConfig::Clock(ref mut s) = self.components[idx].config {
                    s.blink_separator = !s.blink_separator;
                }
            }
            MenuAction::SpanMoreRows => self.adjust_span(idx, true, true),
            MenuAction::SpanFewerRows => self.adjust_span(idx, true, false),
            MenuAction::SpanMoreCols => self.adjust_span(idx, false, true),
            MenuAction::SpanFewerCols => self.adjust_span(idx, false, false),
            MenuAction::Remove => {
                self.remove_component(idx);
            }
        }
        self.ui_mode = UiMode::Normal;
    }

    pub fn move_component(&mut self, idx: usize, dr: i16, dc: i16) {
        let p = &mut self.components[idx].placement;
        let new_row = (p.row as i16 + dr).max(0) as u16;
        let new_col = (p.column as i16 + dc).max(0) as u16;

        if new_row + p.row_span <= self.config.grid.rows {
            p.row = new_row;
        }
        if new_col + p.col_span <= self.config.grid.columns {
            p.column = new_col;
        }
    }

    /// Adjusts a grid row's height percentage. Steals/gives from the largest other row.
    pub fn adjust_row_height(&mut self, row: u16, grow: bool) {
        let grid = &mut self.config.grid;
        let n = grid.rows as usize;
        if n < 2 {
            return;
        }

        // Ensure row_heights is materialized (equal distribution if None)
        let heights = grid.row_heights.get_or_insert_with(|| {
            let base = 100 / n as u16;
            let mut v = vec![base; n];
            v[n - 1] = 100 - base * (n as u16 - 1);
            v
        });

        let r = row as usize;
        if r >= heights.len() {
            return;
        }

        if grow {
            // Find the largest other row to steal from
            let donor = (0..n)
                .filter(|&i| i != r)
                .max_by_key(|&i| heights[i])
                .unwrap();
            if heights[donor] > MIN_ROW_HEIGHT_PCT {
                let step = RESIZE_STEP.min(heights[donor] - MIN_ROW_HEIGHT_PCT);
                heights[r] += step;
                heights[donor] -= step;
            }
        } else {
            if heights[r] <= MIN_ROW_HEIGHT_PCT {
                return;
            }
            // Give to the largest other row
            let receiver = (0..n)
                .filter(|&i| i != r)
                .max_by_key(|&i| heights[i])
                .unwrap();
            let step = RESIZE_STEP.min(heights[r] - MIN_ROW_HEIGHT_PCT);
            heights[r] -= step;
            heights[receiver] += step;
        }
    }

    /// Adjusts a grid column's width percentage. Steals/gives from the largest other column.
    pub fn adjust_col_width(&mut self, col: u16, grow: bool) {
        let grid = &mut self.config.grid;
        let n = grid.columns as usize;
        if n < 2 {
            return;
        }

        let widths = grid.column_widths.get_or_insert_with(|| {
            let base = 100 / n as u16;
            let mut v = vec![base; n];
            v[n - 1] = 100 - base * (n as u16 - 1);
            v
        });

        let c = col as usize;
        if c >= widths.len() {
            return;
        }

        if grow {
            let donor = (0..n)
                .filter(|&i| i != c)
                .max_by_key(|&i| widths[i])
                .unwrap();
            if widths[donor] > MIN_COL_WIDTH_PCT {
                let step = RESIZE_STEP.min(widths[donor] - MIN_COL_WIDTH_PCT);
                widths[c] += step;
                widths[donor] -= step;
            }
        } else {
            if widths[c] <= MIN_COL_WIDTH_PCT {
                return;
            }
            let receiver = (0..n)
                .filter(|&i| i != c)
                .max_by_key(|&i| widths[i])
                .unwrap();
            let step = RESIZE_STEP.min(widths[c] - MIN_COL_WIDTH_PCT);
            widths[c] -= step;
            widths[receiver] += step;
        }
    }

    /// Adjusts a component's row_span or col_span.
    fn adjust_span(&mut self, idx: usize, vertical: bool, grow: bool) {
        let p = &mut self.components[idx].placement;
        if vertical {
            if grow {
                if p.row + p.row_span < self.config.grid.rows {
                    p.row_span += 1;
                }
            } else if p.row_span > 1 {
                p.row_span -= 1;
            }
        } else if grow {
            if p.column + p.col_span < self.config.grid.columns {
                p.col_span += 1;
            }
        } else if p.col_span > 1 {
            p.col_span -= 1;
        }
    }

    pub fn remove_component(&mut self, idx: usize) {
        let id = self.components[idx].id.clone();
        self.components.remove(idx);
        self.runtime.remove(&id);

        let vis = self.visible_components();
        if self.focused_index >= vis.len() && !vis.is_empty() {
            self.focused_index = vis.len() - 1;
        }
    }

    pub fn add_component(&mut self, comp_type: ComponentType, style: Option<ClockStyle>) {
        let (row, col) =
            match find_empty_cell(&self.components, self.config.grid.rows, self.config.grid.columns)
            {
                Some(pos) => pos,
                None => {
                    // Expand grid by one row
                    self.config.grid.rows += 1;
                    self.config.grid.row_heights = None; // rebalance to equal
                    (self.config.grid.rows - 1, 0)
                }
            };

        let mut entry = ComponentEntry::default_for_type(comp_type, row, col);

        // Apply specific clock style if requested
        if let Some(clock_style) = style
            && let ComponentConfig::Clock(ref mut s) = entry.config {
                s.style = clock_style;
            }

        let rt = spawn_component_runtime(&entry);
        self.runtime.insert(entry.id.clone(), rt);
        self.components.push(entry);

        // Focus the new component
        let vis = self.visible_components();
        if let Some(pos) = vis
            .iter()
            .position(|&i| i == self.components.len() - 1)
        {
            self.focused_index = pos;
        }
    }

    pub fn toggle_component_visibility(&mut self, idx: usize) {
        self.components[idx].visible = !self.components[idx].visible;
    }

    /// Toggle time format for the focused component (if it's a clock).
    pub fn toggle_time_format(&mut self) {
        let Some(idx) = self.focused_component_idx() else {
            return;
        };
        if let ComponentConfig::Clock(ref mut s) = self.components[idx].config {
            s.time_format = if s.time_format == "24h" {
                "12h".to_string()
            } else {
                "24h".to_string()
            };
        }
    }

    /// Cycle font for the focused component (if it's a large clock).
    pub fn cycle_font_next(&mut self) {
        let Some(idx) = self.focused_component_idx() else {
            return;
        };
        let id = self.components[idx].id.clone();
        if let ComponentConfig::Clock(ref s) = self.components[idx].config
            && s.style == ClockStyle::Large
                && let Some(ComponentRuntime::Clock {
                    font_style, ..
                }) = self.runtime.get_mut(&id)
                {
                    *font_style = font_style.next();
                }
    }

    pub fn cycle_font_prev(&mut self) {
        let Some(idx) = self.focused_component_idx() else {
            return;
        };
        let id = self.components[idx].id.clone();
        if let ComponentConfig::Clock(ref s) = self.components[idx].config
            && s.style == ClockStyle::Large
                && let Some(ComponentRuntime::Clock {
                    font_style, ..
                }) = self.runtime.get_mut(&id)
                {
                    *font_style = font_style.prev();
                }
    }

    /// Returns the font name shown in the status bar (from first large clock, or first clock).
    pub fn active_font_name(&self) -> &str {
        for comp in &self.components {
            if let ComponentConfig::Clock(ref s) = comp.config
                && s.style == ClockStyle::Large
                    && let Some(ComponentRuntime::Clock { font_style, .. }) =
                        self.runtime.get(&comp.id)
                    {
                        return font_style.name();
                    }
        }
        "Standard"
    }

    /// Sync runtime state back to config and save.
    pub fn persist_state(&mut self) {
        // Sync font_style from runtime back into component settings
        for comp in &mut self.components {
            if let ComponentConfig::Clock(ref mut s) = comp.config
                && let Some(ComponentRuntime::Clock { font_style, .. }) =
                    self.runtime.get(&comp.id)
                {
                    s.font_style = font_style.name().to_string();
                }
        }

        self.config.sync_components(&self.components);
        if let Err(err) = self.config.save() {
            warn!("Failed to persist config on exit: {err}");
        }
    }
}

// ── Helpers ─────────────────────────────────────────────────────────

fn next_date_preset(current: &str) -> String {
    let presets = constants::DATE_FORMAT_PRESETS;
    let idx = presets.iter().position(|&p| p == current);
    let next_idx = match idx {
        Some(i) => (i + 1) % presets.len(),
        None => 0,
    };
    presets[next_idx].to_string()
}

/// Creates runtime state for a component, spawning background tasks as needed.
fn spawn_component_runtime(entry: &ComponentEntry) -> ComponentRuntime {
    match &entry.config {
        ComponentConfig::Clock(s) => ComponentRuntime::Clock {
            font_style: FontStyle::from_name(&s.font_style),
            area: Rect::default(),
        },
        ComponentConfig::Weather(s) => {
            let (tx, rx) = watch::channel(None);
            spawn_weather_task(tx, s);
            ComponentRuntime::Weather {
                data_rx: rx,
                area: Rect::default(),
            }
        }
        ComponentConfig::Calendar(_) => ComponentRuntime::Calendar {
            area: Rect::default(),
        },
        ComponentConfig::SystemStats(s) => {
            let (tx, rx) = watch::channel(system::read_system_stats());
            spawn_stats_task(tx, s.refresh_interval_seconds);
            ComponentRuntime::SystemStats {
                stats_rx: rx,
                area: Rect::default(),
            }
        }
    }
}

// ── Background tasks ────────────────────────────────────────────────

fn spawn_weather_task(
    tx: watch::Sender<Option<WeatherData>>,
    settings: &crate::component::WeatherSettings,
) {
    let lat = settings.latitude;
    let lon = settings.longitude;
    let unit = settings.temperature_unit.clone();
    let interval_mins = settings.refresh_interval_minutes;

    tokio::spawn(async move {
        loop {
            match weather_api::fetch_weather(lat, lon, &unit).await {
                Ok(data) => {
                    let _ = tx.send(Some(data));
                }
                Err(err) => {
                    error!("Weather fetch failed: {err}");
                }
            }
            tokio::time::sleep(std::time::Duration::from_secs(interval_mins * 60)).await;
        }
    });
}

pub fn spawn_ip_task(tx: watch::Sender<Option<String>>, config: &AppConfig) {
    let interval_mins = config.network.ip_refresh_interval_minutes;

    tokio::spawn(async move {
        loop {
            match ip::fetch_external_ip().await {
                Ok(ip_addr) => {
                    let _ = tx.send(Some(ip_addr));
                }
                Err(err) => {
                    error!("IP fetch failed: {err}");
                }
            }
            tokio::time::sleep(std::time::Duration::from_secs(interval_mins * 60)).await;
        }
    });
}

fn spawn_stats_task(tx: watch::Sender<SystemStats>, interval_secs: u64) {
    tokio::spawn(async move {
        loop {
            let stats = system::read_system_stats();
            let _ = tx.send(stats);
            tokio::time::sleep(std::time::Duration::from_secs(interval_secs)).await;
        }
    });
}
