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
const MIN_SIZE_PCT: u16 = 10;

pub struct ContextMenuItem {
    pub label: String,
    pub action: MenuAction,
}

// ── Font style ──────────────────────────────────────────────────────

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FontStyle {
    Block,
    Slick,
    Tiny,
    Grid,
    Pallet,
    Shade,
    Chrome,
    Simple,
    SimpleBlock,
    Simple3d,
    Huge,
    Console,
}

impl FontStyle {
    const ALL: &[Self] = &[
        Self::Block,
        Self::Slick,
        Self::Tiny,
        Self::Grid,
        Self::Pallet,
        Self::Shade,
        Self::Chrome,
        Self::Simple,
        Self::SimpleBlock,
        Self::Simple3d,
        Self::Huge,
        Self::Console,
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
            Self::Block => "Block",
            Self::Slick => "Slick",
            Self::Tiny => "Tiny",
            Self::Grid => "Grid",
            Self::Pallet => "Pallet",
            Self::Shade => "Shade",
            Self::Chrome => "Chrome",
            Self::Simple => "Simple",
            Self::SimpleBlock => "SimpleBlock",
            Self::Simple3d => "Simple3D",
            Self::Huge => "Huge",
            Self::Console => "Console",
        }
    }

    pub fn from_name(name: &str) -> Self {
        // First try exact match against current names
        if let Some(&style) = Self::ALL
            .iter()
            .find(|s| s.name().eq_ignore_ascii_case(name))
        {
            return style;
        }

        // Backwards compat: map old figlet/toilet font names
        match name {
            "Standard" | "Mono12" => Self::Block,
            "Big" => Self::Huge,
            "Small" => Self::Tiny,
            "Slant" => Self::Slick,
            "SmBlock" => Self::SimpleBlock,
            "Future" => Self::Chrome,
            "Wideterm" => Self::Grid,
            "Mono9" => Self::Console,
            _ => Self::Block,
        }
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

    /// Down arrow: grow this row (shrink adjacent neighbor).
    /// Up arrow: shrink this row (grow adjacent neighbor).
    /// Neighbor = row below if it exists, else row above (so the last row is never stuck).
    pub fn adjust_row_height(&mut self, row: u16, grow: bool) {
        let n = self.config.grid.rows as usize;
        let r = row as usize;
        if n < 2 || r >= n {
            return;
        }
        let neighbor = if r + 1 < n { r + 1 } else { r - 1 };
        if grow {
            resize_between(&self.config.grid.rows, &mut self.config.grid.row_heights, r, neighbor);
        } else {
            resize_between(&self.config.grid.rows, &mut self.config.grid.row_heights, neighbor, r);
        }
    }

    /// Right arrow: grow this column. Left arrow: shrink this column.
    /// Neighbor = column to right if it exists, else column to left.
    pub fn adjust_col_width(&mut self, col: u16, grow: bool) {
        let n = self.config.grid.columns as usize;
        let c = col as usize;
        if n < 2 || c >= n {
            return;
        }
        let neighbor = if c + 1 < n { c + 1 } else { c - 1 };
        if grow {
            resize_between(&self.config.grid.columns, &mut self.config.grid.column_widths, c, neighbor);
        } else {
            resize_between(&self.config.grid.columns, &mut self.config.grid.column_widths, neighbor, c);
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
        "Block"
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

// ── Grid resize helpers (free functions for testability) ─────────

/// Grows `grower` and shrinks `shrinker` by one step in the given percentage vec.
/// Both indices must be in bounds. The shrinker won't go below the minimum.
fn resize_between(count: &u16, pcts: &mut Option<Vec<u16>>, grower: usize, shrinker: usize) {
    let n = *count as usize;
    if n < 2 || grower >= n || shrinker >= n {
        return;
    }

    let sizes = pcts.get_or_insert_with(|| {
        let base = 100 / n as u16;
        let mut v = vec![base; n];
        v[n - 1] = 100 - base * (n as u16 - 1);
        v
    });

    if grower == shrinker {
        return;
    }

    if sizes[shrinker] <= MIN_SIZE_PCT {
        return;
    }

    let step = RESIZE_STEP.min(sizes[shrinker] - MIN_SIZE_PCT);
    sizes[grower] += step;
    sizes[shrinker] -= step;
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::GridConfig;

    fn grid_3rows() -> GridConfig {
        GridConfig {
            rows: 3,
            columns: 2,
            row_heights: Some(vec![40, 30, 30]),
            column_widths: None,
        }
    }

    /// Simulates Down (grow=true) / Up (grow=false) on a given row.
    /// Uses the same logic as adjust_row_height.
    fn resize(g: &mut GridConfig, row: usize, grow: bool) {
        let n = g.rows as usize;
        if n < 2 || row >= n { return; }
        let neighbor = if row + 1 < n { row + 1 } else { row - 1 };
        if grow {
            resize_between(&g.rows, &mut g.row_heights, row, neighbor);
        } else {
            resize_between(&g.rows, &mut g.row_heights, neighbor, row);
        }
    }

    #[test]
    fn row0_down_grows_row0_shrinks_row1() {
        let mut g = grid_3rows();
        resize(&mut g, 0, true);
        assert_eq!(g.row_heights.unwrap(), vec![45, 25, 30]);
    }

    #[test]
    fn row0_up_shrinks_row0_grows_row1() {
        let mut g = grid_3rows();
        resize(&mut g, 0, false);
        assert_eq!(g.row_heights.unwrap(), vec![35, 35, 30]);
    }

    #[test]
    fn row1_down_grows_row1_shrinks_row2() {
        let mut g = grid_3rows();
        resize(&mut g, 1, true);
        assert_eq!(g.row_heights.unwrap(), vec![40, 35, 25]);
    }

    #[test]
    fn row1_up_shrinks_row1_grows_row2() {
        let mut g = grid_3rows();
        resize(&mut g, 1, false);
        assert_eq!(g.row_heights.unwrap(), vec![40, 25, 35]);
    }

    #[test]
    fn row2_down_grows_row2_shrinks_row1() {
        let mut g = grid_3rows();
        resize(&mut g, 2, true); // last row: fallback neighbor = row 1
        assert_eq!(g.row_heights.unwrap(), vec![40, 25, 35]);
    }

    #[test]
    fn row2_up_shrinks_row2_grows_row1() {
        let mut g = grid_3rows();
        resize(&mut g, 2, false); // last row: fallback neighbor = row 1
        assert_eq!(g.row_heights.unwrap(), vec![40, 35, 25]);
    }

    #[test]
    fn down_then_up_is_identity() {
        let mut g = grid_3rows();
        resize(&mut g, 0, true);
        resize(&mut g, 0, false);
        assert_eq!(g.row_heights.unwrap(), vec![40, 30, 30]);
    }

    #[test]
    fn row2_down_then_up_is_identity() {
        let mut g = grid_3rows();
        resize(&mut g, 2, true);
        resize(&mut g, 2, false);
        assert_eq!(g.row_heights.unwrap(), vec![40, 30, 30]);
    }

    #[test]
    fn no_row_changes_unrelated_rows() {
        let mut g = grid_3rows();
        for _ in 0..4 {
            resize(&mut g, 0, true);
        }
        let h = g.row_heights.unwrap();
        assert_eq!(h[2], 30, "row 2 untouched when resizing row 0");
        assert_eq!(h[0] + h[1] + h[2], 100);
    }

    #[test]
    fn respects_min() {
        let mut g = GridConfig {
            rows: 2,
            columns: 1,
            row_heights: Some(vec![85, 15]),
            column_widths: None,
        };
        resize(&mut g, 0, true);
        assert_eq!(g.row_heights.as_ref().unwrap(), &vec![90, 10]);
        resize(&mut g, 0, true);
        assert_eq!(g.row_heights.unwrap(), vec![90, 10], "stops at minimum");
    }

    #[test]
    fn col_last_grows_shrinks_left_neighbor() {
        let mut g = GridConfig {
            rows: 1,
            columns: 3,
            row_heights: None,
            column_widths: Some(vec![33, 34, 33]),
        };
        // Right (grow) on last col: fallback neighbor = col 1
        let n = g.columns as usize;
        let c = 2_usize;
        let neighbor = if c + 1 < n { c + 1 } else { c - 1 };
        resize_between(&g.columns, &mut g.column_widths, c, neighbor);
        assert_eq!(g.column_widths.unwrap(), vec![33, 29, 38]);
    }
}
