use tokio::sync::watch;

use crate::config::AppConfig;
use crate::data::system::{self, SystemStats};
use crate::data::weather_api::WeatherData;
use crate::data::{ip, weather_api};

/// Core application state.
pub struct App {
    pub running: bool,
    pub tick_count: u64,
    pub config: AppConfig,
    pub weather_rx: watch::Receiver<Option<WeatherData>>,
    pub ip_rx: watch::Receiver<Option<String>>,
    pub stats_rx: watch::Receiver<SystemStats>,
}

impl App {
    pub fn new(
        config: AppConfig,
        weather_rx: watch::Receiver<Option<WeatherData>>,
        ip_rx: watch::Receiver<Option<String>>,
        stats_rx: watch::Receiver<SystemStats>,
    ) -> Self {
        Self {
            running: true,
            tick_count: 0,
            config,
            weather_rx,
            ip_rx,
            stats_rx,
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
        // Blink every other tick (with default 250ms tick, that's 500ms on/off)
        self.tick_count.is_multiple_of(2)
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
                    eprintln!("Weather fetch error: {err}");
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
                    eprintln!("IP fetch error: {err}");
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
