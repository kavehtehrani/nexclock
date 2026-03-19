use tokio::sync::watch;

use crate::config::AppConfig;
use crate::data::{ip, weather_api};
use crate::data::weather_api::WeatherData;

/// Core application state.
pub struct App {
    pub running: bool,
    pub tick_count: u64,
    pub config: AppConfig,
    pub weather_rx: watch::Receiver<Option<WeatherData>>,
    pub ip_rx: watch::Receiver<Option<String>>,
}

impl App {
    pub fn new(
        config: AppConfig,
        weather_rx: watch::Receiver<Option<WeatherData>>,
        ip_rx: watch::Receiver<Option<String>>,
    ) -> Self {
        Self {
            running: true,
            tick_count: 0,
            config,
            weather_rx,
            ip_rx,
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
}

/// Spawns the background weather fetch loop.
pub fn spawn_weather_task(
    tx: watch::Sender<Option<WeatherData>>,
    config: &AppConfig,
) {
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
pub fn spawn_ip_task(
    tx: watch::Sender<Option<String>>,
    config: &AppConfig,
) {
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
