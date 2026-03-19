pub mod calendar;
pub mod clock;
pub mod secondary_clock;

use ratatui::{
    layout::{Constraint, Layout},
    Frame,
};

use crate::app::App;
use crate::constants::STATUS_BAR_HEIGHT;

/// Root draw function: composes the full UI layout.
pub fn draw(frame: &mut Frame, app: &App) {
    let area = frame.area();
    let config = &app.config;

    // Main vertical split: clock | info panels | status bar
    let rows = Layout::vertical([
        Constraint::Percentage(config.layout.clock_height_percent),
        Constraint::Percentage(config.layout.info_height_percent),
        Constraint::Length(STATUS_BAR_HEIGHT),
    ])
    .split(area);

    // Clock panel
    clock::render(frame, rows[0], &config.clock);

    // Info panels: split into left and right columns
    let columns = Layout::horizontal([
        Constraint::Percentage(config.layout.left_column_percent),
        Constraint::Percentage(100 - config.layout.left_column_percent),
    ])
    .split(rows[1]);

    // Left column: secondary clock
    if config.secondary_clock.enabled {
        secondary_clock::render(frame, columns[0], &config.secondary_clock);
    }

    // Right column: Gregorian calendar
    if config.calendar.show_gregorian {
        calendar::render(frame, columns[1]);
    }

    // Status bar (rows[2]) will be added in Phase 3
}
