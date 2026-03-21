use std::collections::HashMap;

use ratatui::layout::Rect;
use ratatui::style::Color;
use tokio::sync::watch;
use tracing::{error, warn};

use crate::component::{
    find_empty_cell, rects_overlap, ClockStyle, ComponentConfig, ComponentEntry, ComponentType,
    SecondaryCalendarEntry, TimezoneEntry,
};
use crate::config::{AppConfig, ThemeConfig};
use crate::constants;
use crate::data::calendar_api::{self, CalendarDateEntry};
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
    ColorMenu,
    StyleMenu,
    StyleColorPicker,
    CalendarSelectMenu,
    CalendarRemoveMenu,
    Help,
    TimezoneSearch,
    TimezoneRemoveMenu,
    TimezoneReorderMenu,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StyleProperty {
    Fg,
    Bg,
    BorderColor,
}

// ── Menu action ─────────────────────────────────────────────────────

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum MenuAction {
    ToggleTimeFormat,
    ToggleSeconds,
    ToggleBlink,
    CycleDateFormat,
    ChangeColors,
    OpenStyle,
    AddCalendar,
    RemoveCalendar,
    Remove,
    AddTimezone,
    RemoveTimezone,
    ReorderTimezones,
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

/// Converts any `Color` variant to an `(u8, u8, u8)` RGB tuple.
/// Named terminal colors are mapped to their standard xterm RGB values.
pub fn color_to_rgb(c: Color) -> (u8, u8, u8) {
    match c {
        Color::Rgb(r, g, b) => (r, g, b),
        Color::Black => (0, 0, 0),
        Color::Red => (205, 0, 0),
        Color::Green => (0, 205, 0),
        Color::Yellow => (205, 205, 0),
        Color::Blue => (0, 0, 238),
        Color::Magenta => (205, 0, 205),
        Color::Cyan => (0, 205, 205),
        Color::White => (255, 255, 255),
        Color::Gray => (128, 128, 128),
        Color::DarkGray => (85, 85, 85),
        Color::LightRed => (255, 85, 85),
        Color::LightGreen => (85, 255, 85),
        Color::LightYellow => (255, 255, 85),
        Color::LightBlue => (85, 85, 255),
        Color::LightMagenta => (255, 85, 255),
        Color::LightCyan => (85, 255, 255),
        _ => (255, 255, 255),
    }
}

// ── Component runtime ───────────────────────────────────────────────

/// Per-component runtime state (data receivers, rendered area, etc.).
pub enum ComponentRuntime {
    Clock {
        font_style: FontStyle,
        calendar_rx: Option<watch::Receiver<Vec<CalendarDateEntry>>>,
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
    WorldClock {
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
            Self::WorldClock { area } => *area,
        }
    }

    pub fn set_area(&mut self, new_area: Rect) {
        match self {
            Self::Clock { area, .. } => *area = new_area,
            Self::Weather { area, .. } => *area = new_area,
            Self::Calendar { area } => *area = new_area,
            Self::SystemStats { area, .. } => *area = new_area,
            Self::WorldClock { area } => *area = new_area,
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

    // Style editing state
    pub style_target: StyleProperty,

    // Calendar selection state
    pub cal_select_cursor: usize,
    pub cal_select_items: Vec<(&'static str, &'static str)>,

    // Timezone search state
    pub tz_search_query: String,
    pub tz_search_results: Vec<&'static str>,
    pub tz_search_cursor: usize,
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
            style_target: StyleProperty::Fg,
            cal_select_cursor: 0,
            cal_select_items: Vec::new(),
            tz_search_query: String::new(),
            tz_search_results: Vec::new(),
            tz_search_cursor: 0,
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
        match &comp.config {
            ComponentConfig::Clock(s) => {
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
                    items.push(ContextMenuItem {
                        label: "Add calendar".into(),
                        action: MenuAction::AddCalendar,
                    });
                    items.push(ContextMenuItem {
                        label: "Manage calendars".into(),
                        action: MenuAction::RemoveCalendar,
                    });
                }
                items.push(ContextMenuItem {
                    label: "Colors".into(),
                    action: MenuAction::ChangeColors,
                });
            }
            ComponentConfig::WorldClock(_) => {
                items.push(ContextMenuItem {
                    label: "Add timezone".into(),
                    action: MenuAction::AddTimezone,
                });
                items.push(ContextMenuItem {
                    label: "Remove timezone".into(),
                    action: MenuAction::RemoveTimezone,
                });
                items.push(ContextMenuItem {
                    label: "Reorder timezones".into(),
                    action: MenuAction::ReorderTimezones,
                });
                items.push(ContextMenuItem {
                    label: "Toggle 12h/24h".into(),
                    action: MenuAction::ToggleTimeFormat,
                });
                items.push(ContextMenuItem {
                    label: "Toggle seconds".into(),
                    action: MenuAction::ToggleSeconds,
                });
            }
            _ => {}
        }

        // Style (all types)
        items.push(ContextMenuItem {
            label: "Style".into(),
            action: MenuAction::OpenStyle,
        });

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
                self.toggle_time_format();
            }
            MenuAction::CycleDateFormat => {
                if let ComponentConfig::Clock(ref mut s) = self.components[idx].config {
                    s.date_format = next_date_preset(&s.date_format);
                }
            }
            MenuAction::ToggleSeconds => {
                match &mut self.components[idx].config {
                    ComponentConfig::Clock(s) => {
                        s.show_seconds = !s.show_seconds;
                    }
                    ComponentConfig::WorldClock(s) => {
                        s.show_seconds = !s.show_seconds;
                    }
                    _ => {}
                }
            }
            MenuAction::ToggleBlink => {
                if let ComponentConfig::Clock(ref mut s) = self.components[idx].config {
                    s.blink_separator = !s.blink_separator;
                }
            }
            MenuAction::ChangeColors => {
                self.menu_cursor = 0;
                self.ui_mode = UiMode::ColorMenu;
                return; // don't reset to Normal
            }
            MenuAction::OpenStyle => {
                self.menu_cursor = 0;
                self.ui_mode = UiMode::StyleMenu;
                return;
            }
            MenuAction::AddCalendar => {
                self.open_calendar_select();
                return;
            }
            MenuAction::RemoveCalendar => {
                self.menu_cursor = 0;
                self.ui_mode = UiMode::CalendarRemoveMenu;
                return;
            }
            MenuAction::Remove => {
                self.remove_component(idx);
            }
            MenuAction::AddTimezone => {
                self.tz_search_query.clear();
                self.tz_search_results.clear();
                self.tz_search_cursor = 0;
                self.tz_search_update();
                self.ui_mode = UiMode::TimezoneSearch;
                return;
            }
            MenuAction::RemoveTimezone => {
                self.menu_cursor = 0;
                self.ui_mode = UiMode::TimezoneRemoveMenu;
                return;
            }
            MenuAction::ReorderTimezones => {
                self.menu_cursor = 0;
                self.ui_mode = UiMode::TimezoneReorderMenu;
                return;
            }
        }
        self.ui_mode = UiMode::Normal;
    }

    pub fn move_component(&mut self, idx: usize, dr: i16, dc: i16) {
        let p = &self.components[idx].placement;
        let new_row = (p.row as i16 + dr).max(0) as u16;
        let new_col = (p.column as i16 + dc).max(0) as u16;
        let span_r = p.row_span;
        let span_c = p.col_span;
        let grid_rows = self.config.grid.rows;
        let grid_cols = self.config.grid.columns;

        // Bounds check
        if new_row + span_r > grid_rows || new_col + span_c > grid_cols {
            return;
        }

        // Build proposed position for the mover
        let mut proposed: Vec<(usize, u16, u16, u16, u16)> = Vec::new();
        proposed.push((idx, new_row, new_col, span_r, span_c));

        // Find ALL components that overlap with our target position
        // and compute their displaced positions
        for (ci, c) in self.components.iter().enumerate() {
            if ci == idx || !c.visible {
                continue;
            }
            let cp = &c.placement;
            if !rects_overlap((new_row, new_col, span_r, span_c), (cp.row, cp.column, cp.row_span, cp.col_span)) {
                continue;
            }
            let disp_row = (cp.row as i16 - dr).max(0) as u16;
            let disp_col = (cp.column as i16 - dc).max(0) as u16;

            // Bounds check for displaced component
            if disp_row + cp.row_span > grid_rows || disp_col + cp.col_span > grid_cols {
                return;
            }
            proposed.push((ci, disp_row, disp_col, cp.row_span, cp.col_span));
        }

        // Collect which components are being moved
        let affected: Vec<usize> = proposed.iter().map(|&(i, ..)| i).collect();

        // Check that no proposed position overlaps with another proposed position
        // or with any uninvolved component
        for (a, &(_, r1, c1, rs1, cs1)) in proposed.iter().enumerate() {
            let rect_a = (r1, c1, rs1, cs1);
            for &(_, r2, c2, rs2, cs2) in &proposed[a + 1..] {
                if rects_overlap(rect_a, (r2, c2, rs2, cs2)) {
                    return; // displaced components would overlap each other
                }
            }
            for (ci, c) in self.components.iter().enumerate() {
                if affected.contains(&ci) || !c.visible {
                    continue;
                }
                let cp = &c.placement;
                if rects_overlap(rect_a, (cp.row, cp.column, cp.row_span, cp.col_span)) {
                    return; // would overlap an uninvolved component
                }
            }
        }

        // All checks passed, apply moves
        for &(mi, new_r, new_c, _, _) in &proposed {
            self.components[mi].placement.row = new_r;
            self.components[mi].placement.column = new_c;
        }

        // Keep focus on the component we just moved
        let vis = self.visible_components();
        if let Some(new_fi) = vis.iter().position(|&ci| ci == idx) {
            self.focused_index = new_fi;
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

    pub fn remove_component(&mut self, idx: usize) {
        let id = self.components[idx].id.clone();
        self.components.remove(idx);
        self.runtime.remove(&id);

        self.compact_grid();

        let vis = self.visible_components();
        if self.focused_index >= vis.len() && !vis.is_empty() {
            self.focused_index = vis.len() - 1;
        }
    }

    /// Shrinks grid.rows and grid.columns to remove trailing empty rows/columns.
    fn compact_grid(&mut self) {
        let min_rows = self
            .components
            .iter()
            .map(|c| c.placement.row + c.placement.row_span)
            .max()
            .unwrap_or(1)
            .max(1);

        if min_rows < self.config.grid.rows {
            self.config.grid.rows = min_rows;
            self.config.grid.row_heights = None;
        }

        let min_cols = self
            .components
            .iter()
            .map(|c| c.placement.column + c.placement.col_span)
            .max()
            .unwrap_or(1)
            .max(1);

        if min_cols < self.config.grid.columns {
            self.config.grid.columns = min_cols;
            self.config.grid.column_widths = None;
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

    /// Apply a color preset to the focused clock component.
    pub fn apply_color_preset(&mut self, preset_index: usize) {
        let Some(idx) = self.focused_component_idx() else {
            return;
        };
        if let ComponentConfig::Clock(ref mut s) = self.components[idx].config
            && let Some(&(_, colors)) = constants::COLOR_PRESETS.get(preset_index)
        {
            s.colors = colors.iter().map(|&c| c.to_string()).collect();
        }
    }

    /// Apply a color from STYLE_COLOR_PRESETS to the focused component's style property.
    pub fn apply_style_color(&mut self, preset_index: usize) {
        let Some(idx) = self.focused_component_idx() else {
            return;
        };
        let Some(&(_, color_str)) = constants::STYLE_COLOR_PRESETS.get(preset_index) else {
            return;
        };
        let value = if color_str.is_empty() {
            None
        } else {
            Some(color_str.to_string())
        };
        match self.style_target {
            StyleProperty::Fg => self.components[idx].style.fg = value,
            StyleProperty::Bg => self.components[idx].style.bg = value,
            StyleProperty::BorderColor => self.components[idx].style.border_color = value,
        }
    }

    /// Reset all style overrides on the focused component.
    pub fn reset_component_style(&mut self) {
        let Some(idx) = self.focused_component_idx() else {
            return;
        };
        self.components[idx].style = crate::component::ComponentStyle::default();
    }

    /// Toggle time format for the focused component (clock or world clock).
    pub fn toggle_time_format(&mut self) {
        let Some(idx) = self.focused_component_idx() else {
            return;
        };
        match &mut self.components[idx].config {
            ComponentConfig::Clock(s) => {
                s.time_format = if s.time_format == "24h" {
                    "12h".to_string()
                } else {
                    "24h".to_string()
                };
            }
            ComponentConfig::WorldClock(s) => {
                s.time_format = if s.time_format == "24h" {
                    "12h".to_string()
                } else {
                    "24h".to_string()
                };
            }
            _ => {}
        }
    }

    /// Cycle font for the focused component (if it's a large clock).
    fn cycle_font(&mut self, direction: fn(FontStyle) -> FontStyle) {
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
                    *font_style = direction(*font_style);
                }
    }

    pub fn cycle_font_next(&mut self) {
        self.cycle_font(FontStyle::next);
    }

    pub fn cycle_font_prev(&mut self) {
        self.cycle_font(FontStyle::prev);
    }


    // ── Timezone search ──────────────────────────────────────────────

    /// Re-filters `TZ_VARIANTS` based on the current search query.
    pub fn tz_search_update(&mut self) {
        let query = self.tz_search_query.to_lowercase();
        self.tz_search_results = chrono_tz::TZ_VARIANTS
            .iter()
            .map(|tz| tz.name())
            .filter(|name| {
                if query.is_empty() {
                    return true;
                }
                name.to_lowercase().contains(&query)
            })
            .take(constants::TZ_SEARCH_MAX_RESULTS)
            .collect();
        // Clamp cursor
        if !self.tz_search_results.is_empty() {
            self.tz_search_cursor = self.tz_search_cursor.min(self.tz_search_results.len() - 1);
        } else {
            self.tz_search_cursor = 0;
        }
    }

    /// Adds the selected timezone from search results to the focused world clock.
    pub fn tz_search_select(&mut self) {
        let Some(tz_name) = self.tz_search_results.get(self.tz_search_cursor).copied() else {
            return;
        };
        let Some(idx) = self.focused_component_idx() else {
            return;
        };
        if let ComponentConfig::WorldClock(ref mut s) = self.components[idx].config {
            // Derive a label from the timezone name (last segment after '/')
            let label = tz_name.rsplit('/').next().unwrap_or(tz_name).replace('_', " ");
            s.timezones.push(TimezoneEntry {
                timezone: tz_name.to_string(),
                label: Some(label),
            });
        }
    }

    /// Removes a timezone at the given index from the focused world clock.
    pub fn remove_timezone(&mut self, tz_index: usize) {
        let Some(idx) = self.focused_component_idx() else {
            return;
        };
        if let ComponentConfig::WorldClock(s) = &mut self.components[idx].config
            && tz_index < s.timezones.len()
        {
            s.timezones.remove(tz_index);
        }
    }

    /// Swaps two adjacent timezones in the focused world clock.
    /// `direction` is -1 (swap up) or 1 (swap down).
    pub fn swap_timezone(&mut self, index: usize, direction: i32) {
        let Some(idx) = self.focused_component_idx() else {
            return;
        };
        if let ComponentConfig::WorldClock(s) = &mut self.components[idx].config {
            let neighbor = index as i32 + direction;
            if neighbor >= 0 && (neighbor as usize) < s.timezones.len() {
                s.timezones.swap(index, neighbor as usize);
            }
        }
    }

    /// Returns the timezone list of the focused world clock (for the remove menu).
    pub fn focused_world_clock_timezones(&self) -> &[TimezoneEntry] {
        if let Some(comp) = self.focused_component()
            && let ComponentConfig::WorldClock(s) = &comp.config
        {
            return &s.timezones;
        }
        &[]
    }

    // ── Secondary calendar management ─────────────────────────────

    /// Opens the calendar selection menu, showing only calendars not already added.
    pub fn open_calendar_select(&mut self) {
        let existing: Vec<String> = if let Some(comp) = self.focused_component()
            && let ComponentConfig::Clock(s) = &comp.config
        {
            s.secondary_calendars.iter().map(|c| c.calendar_id.clone()).collect()
        } else {
            Vec::new()
        };

        self.cal_select_items = constants::CALENDAR_SYSTEMS
            .iter()
            .filter(|(id, _)| !existing.contains(&id.to_string()))
            .copied()
            .collect();
        self.cal_select_cursor = 0;
        self.ui_mode = UiMode::CalendarSelectMenu;
    }

    /// Adds the selected calendar to the focused large clock and respawns the fetch task.
    pub fn calendar_select_confirm(&mut self) {
        let Some(&(cal_id, _)) = self.cal_select_items.get(self.cal_select_cursor) else {
            return;
        };
        let Some(idx) = self.focused_component_idx() else {
            return;
        };
        if let ComponentConfig::Clock(ref mut s) = self.components[idx].config {
            s.secondary_calendars.push(SecondaryCalendarEntry {
                calendar_id: cal_id.to_string(),
                use_native: false,
            });
        }
        self.respawn_calendar_task(idx);
    }

    /// Removes a secondary calendar at the given index from the focused clock.
    pub fn remove_secondary_calendar(&mut self, cal_index: usize) {
        let Some(idx) = self.focused_component_idx() else {
            return;
        };
        if let ComponentConfig::Clock(ref mut s) = self.components[idx].config
            && cal_index < s.secondary_calendars.len()
        {
            s.secondary_calendars.remove(cal_index);
        }
        self.respawn_calendar_task(idx);
    }

    /// Toggles `use_native` on a secondary calendar at the given index.
    pub fn toggle_calendar_native(&mut self, cal_index: usize) {
        let Some(idx) = self.focused_component_idx() else {
            return;
        };
        if let ComponentConfig::Clock(ref mut s) = self.components[idx].config
            && cal_index < s.secondary_calendars.len()
        {
            s.secondary_calendars[cal_index].use_native =
                !s.secondary_calendars[cal_index].use_native;
        }
    }

    /// Returns the secondary calendar list of the focused clock (for the remove menu).
    pub fn focused_clock_calendars(&self) -> &[SecondaryCalendarEntry] {
        if let Some(comp) = self.focused_component()
            && let ComponentConfig::Clock(s) = &comp.config
        {
            return &s.secondary_calendars;
        }
        &[]
    }

    /// Recreates the calendar background task for the component at `idx`.
    fn respawn_calendar_task(&mut self, idx: usize) {
        let entry = &self.components[idx];
        let ComponentConfig::Clock(s) = &entry.config else {
            return;
        };

        let calendar_rx = if s.secondary_calendars.is_empty() {
            None
        } else {
            let ids: Vec<String> = s.secondary_calendars.iter().map(|c| c.calendar_id.clone()).collect();
            let tz = s.timezone.clone().unwrap_or_else(|| crate::ui::clock::local_timezone_name());
            let (tx, rx) = watch::channel(Vec::new());
            spawn_calendar_task(tx, ids, tz);
            Some(rx)
        };

        if let Some(ComponentRuntime::Clock { calendar_rx: rx, .. }) =
            self.runtime.get_mut(&entry.id)
        {
            *rx = calendar_rx;
        }
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
        ComponentConfig::Clock(s) => {
            let calendar_rx = if s.secondary_calendars.is_empty() {
                None
            } else {
                let ids: Vec<String> = s.secondary_calendars.iter().map(|c| c.calendar_id.clone()).collect();
                let tz = s.timezone.clone().unwrap_or_else(crate::ui::clock::local_timezone_name);
                let (tx, rx) = watch::channel(Vec::new());
                spawn_calendar_task(tx, ids, tz);
                Some(rx)
            };
            ComponentRuntime::Clock {
                font_style: FontStyle::from_name(&s.font_style),
                calendar_rx,
                area: Rect::default(),
            }
        }
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
        ComponentConfig::WorldClock(_) => ComponentRuntime::WorldClock {
            area: Rect::default(),
        },
    }
}

// ── Background tasks ────────────────────────────────────────────────

fn spawn_calendar_task(
    tx: watch::Sender<Vec<CalendarDateEntry>>,
    calendar_ids: Vec<String>,
    timezone: String,
) {
    tokio::spawn(async move {
        loop {
            let dates = calendar_api::fetch_all_calendar_dates(&calendar_ids, &timezone).await;
            if tx.send(dates).is_err() {
                break; // receiver dropped, stop task
            }
            tokio::time::sleep(std::time::Duration::from_secs(
                constants::CALENDAR_REFRESH_SECONDS,
            ))
            .await;
        }
    });
}

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
