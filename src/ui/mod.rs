pub mod calendar;
pub mod clock;
pub mod secondary_clock;
pub mod status_bar;
pub mod system_stats;
pub mod weather;

use ratatui::{
    layout::{Alignment, Constraint, Layout},
    style::{Color, Style},
    text::Line,
    widgets::Paragraph,
    Frame,
};

use crate::app::App;
use crate::constants::{MIN_TERMINAL_HEIGHT, MIN_TERMINAL_WIDTH, STATUS_BAR_HEIGHT};

/// Root draw function: composes the full UI layout.
pub fn draw(frame: &mut Frame, app: &App) {
    let area = frame.area();

    // Guard: if terminal is too small, show a message instead of panicking
    if area.width < MIN_TERMINAL_WIDTH || area.height < MIN_TERMINAL_HEIGHT {
        let msg = Line::styled(
            "Terminal too small",
            Style::default().fg(Color::Red),
        );
        frame.render_widget(
            Paragraph::new(msg).alignment(Alignment::Center),
            area,
        );
        return;
    }

    let config = &app.config;

    // Main vertical split: clock | info panels | status bar
    let rows = Layout::vertical([
        Constraint::Percentage(config.layout.clock_height_percent),
        Constraint::Percentage(config.layout.info_height_percent),
        Constraint::Length(STATUS_BAR_HEIGHT),
    ])
    .split(area);

    // Clock panel
    clock::render(frame, rows[0], &config.clock, app.colon_visible());

    // Info panels: split into left and right columns
    let columns = Layout::horizontal([
        Constraint::Percentage(config.layout.left_column_percent),
        Constraint::Percentage(100 - config.layout.left_column_percent),
    ])
    .split(rows[1]);

    // Left column: secondary clock (top) + weather (bottom)
    let left_panels = Layout::vertical([
        Constraint::Percentage(50),
        Constraint::Percentage(50),
    ])
    .split(columns[0]);

    if config.secondary_clock.enabled {
        secondary_clock::render(frame, left_panels[0], &config.secondary_clock);
    }

    if config.weather.enabled {
        weather::render(frame, left_panels[1], &app.weather());
    }

    // Right column: calendar (top) + system stats (bottom)
    let right_panels = Layout::vertical([
        Constraint::Percentage(60),
        Constraint::Percentage(40),
    ])
    .split(columns[1]);

    if config.calendar.show_gregorian {
        calendar::render(frame, right_panels[0]);
    }

    if config.system_stats.enabled {
        system_stats::render(frame, right_panels[1], &app.system_stats());
    }

    // Status bar
    status_bar::render(frame, rows[2], &app.external_ip());
}
