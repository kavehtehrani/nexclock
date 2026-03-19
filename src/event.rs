use crossterm::event::{self, Event as CrosstermEvent, KeyCode, KeyEventKind};
use std::time::Duration;

use crate::app::App;

/// Polls for terminal events with the given timeout.
/// Returns `true` if the app should continue running.
pub fn handle_events(app: &mut App, tick_rate: Duration) -> std::io::Result<()> {
    if event::poll(tick_rate)?
        && let CrosstermEvent::Key(key) = event::read()?
        && key.kind == KeyEventKind::Press
    {
        match key.code {
            KeyCode::Char('q') | KeyCode::Char('Q') => app.quit(),
            KeyCode::Esc => app.quit(),
            _ => {}
        }
    }
    app.tick();
    Ok(())
}
