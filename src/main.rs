mod app;
mod component;
mod config;
mod constants;
mod data;
mod defaults;
mod error;
mod event;
mod ui;

use std::io;

use crossterm::{
    event::{DisableMouseCapture, EnableMouseCapture},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use ratatui::{backend::CrosstermBackend, Terminal};
use tokio::sync::watch;
use tracing::info;

use app::App;
use config::AppConfig;

#[tokio::main]
async fn main() -> io::Result<()> {
    let config = AppConfig::load();

    let _guard = init_logging();

    info!("nexclock starting");

    let original_hook = std::panic::take_hook();
    std::panic::set_hook(Box::new(move |panic_info| {
        let _ = restore_terminal();
        original_hook(panic_info);
    }));

    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen, EnableMouseCapture)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    // IP is global (not per-component), so we set it up here
    let (ip_tx, ip_rx) = watch::channel(None);
    app::spawn_ip_task(ip_tx, &config);

    // App::new() handles spawning per-component background tasks
    let mut app = App::new(config, ip_rx);
    let tick_rate = app.config.tick_rate();

    let result = tokio::select! {
        res = async { run_app(&mut terminal, &mut app, tick_rate) } => res,
        _ = tokio::signal::ctrl_c() => {
            info!("Received Ctrl+C, shutting down");
            Ok(())
        }
    };

    app.persist_state();

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
    execute!(io::stdout(), LeaveAlternateScreen, DisableMouseCapture)?;
    Ok(())
}
