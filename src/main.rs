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
use tracing::info;

use app::App;
use config::AppConfig;
use data::system;

#[tokio::main]
async fn main() -> io::Result<()> {
    // Load config before anything else (logging not yet available, so warnings go to stderr)
    let config = AppConfig::load();

    // Set up file logging
    let _guard = init_logging();

    info!("nexclock starting");

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
    let (stats_tx, stats_rx) = watch::channel(system::read_system_stats());

    // Spawn background tasks
    if config.weather.enabled {
        app::spawn_weather_task(weather_tx, &config);
    }
    app::spawn_ip_task(ip_tx, &config);
    if config.system_stats.enabled {
        app::spawn_stats_task(stats_tx, &config);
    }

    // Spawn signal handler
    let mut app = App::new(config, weather_rx, ip_rx, stats_rx);
    let tick_rate = app.config.tick_rate();

    // Run the main loop with signal handling
    let result = tokio::select! {
        res = async { run_app(&mut terminal, &mut app, tick_rate) } => res,
        _ = tokio::signal::ctrl_c() => {
            info!("Received Ctrl+C, shutting down");
            Ok(())
        }
    };

    // Restore terminal
    restore_terminal()?;

    info!("nexclock exiting");

    if let Err(err) = result {
        eprintln!("Error: {err}");
        std::process::exit(1);
    }

    Ok(())
}

fn run_app(
    terminal: &mut Terminal<CrosstermBackend<io::Stdout>>,
    app: &mut App,
    tick_rate: std::time::Duration,
) -> io::Result<()> {
    while app.running {
        terminal.draw(|frame| ui::draw(frame, app))?;
        event::handle_events(app, tick_rate)?;
    }

    Ok(())
}

/// Initializes tracing with file logging to ~/.local/share/nexclock/nexclock.log.
/// Returns a guard that must be held for the lifetime of the program.
fn init_logging() -> Option<tracing_appender::non_blocking::WorkerGuard> {
    let data_dir = AppConfig::data_dir()?;
    std::fs::create_dir_all(&data_dir).ok()?;

    let file_appender = tracing_appender::rolling::daily(&data_dir, "nexclock.log");
    let (non_blocking, guard) = tracing_appender::non_blocking(file_appender);

    tracing_subscriber::fmt()
        .with_writer(non_blocking)
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| tracing_subscriber::EnvFilter::new("info")),
        )
        .with_ansi(false)
        .init();

    Some(guard)
}

fn restore_terminal() -> io::Result<()> {
    disable_raw_mode()?;
    execute!(io::stdout(), LeaveAlternateScreen)?;
    Ok(())
}
