use tokio::sync::watch;
use tracing::{error, warn};

use crate::config::AppConfig;
use crate::constants;
use crate::data::system::{self, SystemStats};
use crate::data::weather_api::WeatherData;
use crate::data::{ip, weather_api};

/// Identifies each focusable panel.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PanelId {
    Clock,
    SecondaryClock,
    Weather,
    Calendar,
    SystemStats,
}

impl PanelId {
    pub const ALL: &[Self] = &[
        Self::Clock,
        Self::SecondaryClock,
        Self::Weather,
        Self::Calendar,
        Self::SystemStats,
    ];

    pub fn label(self) -> &'static str {
        match self {
            Self::Clock => "Clock",
            Self::SecondaryClock => "Secondary Clock",
            Self::Weather => "Weather",
            Self::Calendar => "Calendar",
            Self::SystemStats => "System Stats",
        }
    }
}

/// Controls which event-handling mode is active.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum UiMode {
    Normal,
    ContextMenu,
    VisibilityMenu,
    Help,
}

/// An action that can be triggered from the context menu.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MenuAction {
    Hide(PanelId),
    ToggleTimeFormat,
    ToggleSeconds,
    ToggleBlink,
    GrowVertical,
    ShrinkVertical,
    WidenLeft,
    NarrowLeft,
}

/// A single entry in the context menu.
pub struct ContextMenuItem {
    pub label: String,
    pub action: MenuAction,
}

/// Available font styles (FIGlet outline + Toilet filled/block).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FontStyle {
    // FIGlet (outline/line-drawn)
    Standard,
    Big,
    Small,
    Slant,
    // Toilet (filled/block)
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

/// Core application state.
pub struct App {
    pub running: bool,
    pub tick_count: u64,
    pub config: AppConfig,
    pub weather_rx: watch::Receiver<Option<WeatherData>>,
    pub ip_rx: watch::Receiver<Option<String>>,
    pub stats_rx: watch::Receiver<SystemStats>,
    pub font_style: FontStyle,
    pub clock_area: ratatui::layout::Rect,

    // Focus / interaction state
    pub focused_panel: PanelId,
    pub ui_mode: UiMode,
    pub context_menu_items: Vec<ContextMenuItem>,
    pub menu_cursor: usize,
}

impl App {
    pub fn new(
        config: AppConfig,
        weather_rx: watch::Receiver<Option<WeatherData>>,
        ip_rx: watch::Receiver<Option<String>>,
        stats_rx: watch::Receiver<SystemStats>,
    ) -> Self {
        let font_style = FontStyle::from_name(&config.appearance.font_style);
        Self {
            running: true,
            tick_count: 0,
            config,
            weather_rx,
            ip_rx,
            stats_rx,
            font_style,
            clock_area: ratatui::layout::Rect::default(),
            focused_panel: PanelId::Clock,
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

    pub fn weather(&self) -> Option<WeatherData> {
        self.weather_rx.borrow().clone()
    }

    pub fn external_ip(&self) -> Option<String> {
        self.ip_rx.borrow().clone()
    }

    pub fn system_stats(&self) -> SystemStats {
        self.stats_rx.borrow().clone()
    }

    /// Returns true when the colon should be visible (for blinking effect).
    pub fn colon_visible(&self) -> bool {
        if !self.config.clock.blink_separator {
            return true;
        }
        self.tick_count.is_multiple_of(2)
    }

    pub fn cycle_font_next(&mut self) {
        self.font_style = self.font_style.next();
    }

    pub fn cycle_font_prev(&mut self) {
        self.font_style = self.font_style.prev();
    }

    pub fn toggle_time_format(&mut self) {
        if self.config.clock.time_format == "24h" {
            self.config.clock.time_format = "12h".to_string();
        } else {
            self.config.clock.time_format = "24h".to_string();
        }
    }

    // ── Focus navigation ────────────────────────────────────────────

    /// Returns the list of currently visible panels.
    pub fn visible_panels(&self) -> Vec<PanelId> {
        PanelId::ALL
            .iter()
            .copied()
            .filter(|p| self.is_panel_visible(*p))
            .collect()
    }

    pub fn is_panel_visible(&self, panel: PanelId) -> bool {
        match panel {
            PanelId::Clock => true, // clock is always visible
            PanelId::SecondaryClock => self.config.secondary_clock.enabled,
            PanelId::Weather => self.config.weather.enabled,
            PanelId::Calendar => self.config.calendar.show_gregorian,
            PanelId::SystemStats => self.config.system_stats.enabled,
        }
    }

    /// Move focus to the next visible panel.
    pub fn focus_next(&mut self) {
        let visible = self.visible_panels();
        if visible.is_empty() {
            return;
        }
        let cur = visible.iter().position(|&p| p == self.focused_panel);
        let next_idx = match cur {
            Some(i) => (i + 1) % visible.len(),
            None => 0,
        };
        self.focused_panel = visible[next_idx];
    }

    /// Move focus to the previous visible panel.
    pub fn focus_prev(&mut self) {
        let visible = self.visible_panels();
        if visible.is_empty() {
            return;
        }
        let cur = visible.iter().position(|&p| p == self.focused_panel);
        let prev_idx = match cur {
            Some(i) => (i + visible.len() - 1) % visible.len(),
            None => 0,
        };
        self.focused_panel = visible[prev_idx];
    }

    // ── Context menu ────────────────────────────────────────────────

    /// Populates the context menu for the currently focused panel and enters ContextMenu mode.
    pub fn open_context_menu(&mut self) {
        let mut items = Vec::new();
        let panel = self.focused_panel;

        match panel {
            PanelId::Clock => {
                items.push(ContextMenuItem {
                    label: "Toggle 12h/24h".into(),
                    action: MenuAction::ToggleTimeFormat,
                });
                items.push(ContextMenuItem {
                    label: "Toggle seconds".into(),
                    action: MenuAction::ToggleSeconds,
                });
                items.push(ContextMenuItem {
                    label: "Toggle blink".into(),
                    action: MenuAction::ToggleBlink,
                });
                items.push(ContextMenuItem {
                    label: "Grow".into(),
                    action: MenuAction::GrowVertical,
                });
                items.push(ContextMenuItem {
                    label: "Shrink".into(),
                    action: MenuAction::ShrinkVertical,
                });
            }
            PanelId::SecondaryClock | PanelId::Weather => {
                items.push(ContextMenuItem {
                    label: "Hide this panel".into(),
                    action: MenuAction::Hide(panel),
                });
                items.push(ContextMenuItem {
                    label: "Widen column".into(),
                    action: MenuAction::WidenLeft,
                });
                items.push(ContextMenuItem {
                    label: "Narrow column".into(),
                    action: MenuAction::NarrowLeft,
                });
            }
            PanelId::Calendar | PanelId::SystemStats => {
                items.push(ContextMenuItem {
                    label: "Hide this panel".into(),
                    action: MenuAction::Hide(panel),
                });
                // Widen right = narrow left, and vice versa
                items.push(ContextMenuItem {
                    label: "Widen column".into(),
                    action: MenuAction::NarrowLeft,
                });
                items.push(ContextMenuItem {
                    label: "Narrow column".into(),
                    action: MenuAction::WidenLeft,
                });
            }
        }

        self.context_menu_items = items;
        self.menu_cursor = 0;
        self.ui_mode = UiMode::ContextMenu;
    }

    /// Execute the selected context menu action and close the menu.
    pub fn execute_menu_action(&mut self, action: MenuAction) {
        match action {
            MenuAction::Hide(panel) => {
                self.toggle_panel_visibility(panel);
                // Move focus away from the now-hidden panel
                if self.focused_panel == panel {
                    self.focus_next();
                }
            }
            MenuAction::ToggleTimeFormat => self.toggle_time_format(),
            MenuAction::ToggleSeconds => {
                self.config.clock.show_seconds = !self.config.clock.show_seconds;
            }
            MenuAction::ToggleBlink => {
                self.config.clock.blink_separator = !self.config.clock.blink_separator;
            }
            MenuAction::GrowVertical => {
                let cur = self.config.layout.clock_height_percent;
                let new = (cur + constants::RESIZE_STEP_PERCENT).min(constants::MAX_PANEL_PERCENT);
                let delta = new - cur;
                self.config.layout.clock_height_percent = new;
                self.config.layout.info_height_percent =
                    self.config.layout.info_height_percent.saturating_sub(delta);
            }
            MenuAction::ShrinkVertical => {
                let cur = self.config.layout.clock_height_percent;
                let new = cur
                    .saturating_sub(constants::RESIZE_STEP_PERCENT)
                    .max(constants::MIN_PANEL_PERCENT);
                let delta = cur - new;
                self.config.layout.clock_height_percent = new;
                self.config.layout.info_height_percent =
                    (self.config.layout.info_height_percent + delta).min(constants::MAX_PANEL_PERCENT);
            }
            MenuAction::WidenLeft => {
                let cur = self.config.layout.left_column_percent;
                self.config.layout.left_column_percent =
                    (cur + constants::RESIZE_STEP_PERCENT).min(constants::MAX_PANEL_PERCENT);
            }
            MenuAction::NarrowLeft => {
                let cur = self.config.layout.left_column_percent;
                self.config.layout.left_column_percent = cur
                    .saturating_sub(constants::RESIZE_STEP_PERCENT)
                    .max(constants::MIN_PANEL_PERCENT);
            }
        }
        self.ui_mode = UiMode::Normal;
    }

    /// Toggle visibility of a given panel.
    pub fn toggle_panel_visibility(&mut self, panel: PanelId) {
        match panel {
            PanelId::Clock => {} // clock cannot be hidden
            PanelId::SecondaryClock => {
                self.config.secondary_clock.enabled = !self.config.secondary_clock.enabled;
            }
            PanelId::Weather => {
                self.config.weather.enabled = !self.config.weather.enabled;
            }
            PanelId::Calendar => {
                self.config.calendar.show_gregorian = !self.config.calendar.show_gregorian;
            }
            PanelId::SystemStats => {
                self.config.system_stats.enabled = !self.config.system_stats.enabled;
            }
        }
    }

    /// Sync runtime state back to config and save to disk.
    pub fn persist_state(&mut self) {
        self.config.appearance.font_style = self.font_style.name().to_string();
        if let Err(err) = self.config.save() {
            warn!("Failed to persist config on exit: {err}");
        }
    }
}

/// Spawns the background weather fetch loop.
pub fn spawn_weather_task(tx: watch::Sender<Option<WeatherData>>, config: &AppConfig) {
    let lat = config.weather.latitude;
    let lon = config.weather.longitude;
    let unit = config.weather.temperature_unit.clone();
    let interval_mins = config.weather.refresh_interval_minutes;

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

/// Spawns the background IP fetch loop.
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

/// Spawns the background system stats refresh loop.
pub fn spawn_stats_task(tx: watch::Sender<SystemStats>, config: &AppConfig) {
    let interval_secs = config.system_stats.refresh_interval_seconds;

    tokio::spawn(async move {
        loop {
            let stats = system::read_system_stats();
            let _ = tx.send(stats);
            tokio::time::sleep(std::time::Duration::from_secs(interval_secs)).await;
        }
    });
}
