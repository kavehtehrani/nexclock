use crate::config::AppConfig;

/// Core application state.
pub struct App {
    pub running: bool,
    pub tick_count: u64,
    pub config: AppConfig,
}

impl App {
    pub fn new(config: AppConfig) -> Self {
        Self {
            running: true,
            tick_count: 0,
            config,
        }
    }

    pub fn tick(&mut self) {
        self.tick_count = self.tick_count.wrapping_add(1);
    }

    pub fn quit(&mut self) {
        self.running = false;
    }
}
