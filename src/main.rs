mod app;
mod config;
mod constants;
mod data;
mod error;
mod event;
mod ui;

use std::io;

use crossterm::{
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use ratatui::{backend::CrosstermBackend, Terminal};
use tokio::sync::watch;

use app::App;
use config::AppConfig;

#[tokio::main]
async fn main() -> io::Result<()> {
    // Load config before entering raw mode so warnings print normally
    let config = AppConfig::load();

    // Set up a panic hook that restores the terminal before printing the panic.
    let original_hook = std::panic::take_hook();
    std::panic::set_hook(Box::new(move |panic_info| {
        let _ = restore_terminal();
        original_hook(panic_info);
    }));

    // Initialize terminal
    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    // Set up watch channels for async data
    let (weather_tx, weather_rx) = watch::channel(None);
    let (ip_tx, ip_rx) = watch::channel(None);

    // Spawn background tasks
    if config.weather.enabled {
        app::spawn_weather_task(weather_tx, &config);
    }
    app::spawn_ip_task(ip_tx, &config);

    // Run the app
    let result = run_app(&mut terminal, config, weather_rx, ip_rx);

    // Restore terminal
    restore_terminal()?;

    if let Err(err) = result {
        eprintln!("Error: {err}");
        std::process::exit(1);
    }

    Ok(())
}

fn run_app(
    terminal: &mut Terminal<CrosstermBackend<io::Stdout>>,
    config: AppConfig,
    weather_rx: watch::Receiver<Option<data::weather_api::WeatherData>>,
    ip_rx: watch::Receiver<Option<String>>,
) -> io::Result<()> {
    let mut app = App::new(config, weather_rx, ip_rx);
    let tick_rate = app.config.tick_rate();

    while app.running {
        terminal.draw(|frame| ui::draw(frame, &app))?;
        event::handle_events(&mut app, tick_rate)?;
    }

    Ok(())
}

fn restore_terminal() -> io::Result<()> {
    disable_raw_mode()?;
    execute!(io::stdout(), LeaveAlternateScreen)?;
    Ok(())
}
