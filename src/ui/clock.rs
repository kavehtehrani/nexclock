use chrono::Local;
use ratatui::{
    layout::{Alignment, Constraint, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Paragraph},
    Frame,
};
use tui_big_text::{BigText, PixelSize};

use crate::config::ClockConfig;
use crate::constants::{PIXEL_SIZE_FULL_MIN_HEIGHT, PIXEL_SIZE_HALF_MIN_HEIGHT};

const BLINK_REPLACEMENT: char = ' ';

/// Selects the best pixel size for the available area height.
fn select_pixel_size(available_height: u16) -> PixelSize {
    if available_height >= PIXEL_SIZE_FULL_MIN_HEIGHT {
        PixelSize::Full
    } else if available_height >= PIXEL_SIZE_HALF_MIN_HEIGHT {
        PixelSize::HalfHeight
    } else {
        PixelSize::Quadrant
    }
}

/// Formats the current time string based on config, with optional colon blinking.
fn format_time(config: &ClockConfig, colon_visible: bool) -> String {
    let now = Local::now();
    let use_24h = config.time_format == "24h";
    let time_str = match (use_24h, config.show_seconds) {
        (true, true) => now.format("%H:%M:%S").to_string(),
        (true, false) => now.format("%H:%M").to_string(),
        (false, true) => now.format("%I:%M:%S %p").to_string(),
        (false, false) => now.format("%I:%M %p").to_string(),
    };

    if !colon_visible {
        time_str.replace(':', &BLINK_REPLACEMENT.to_string())
    } else {
        time_str
    }
}

/// Returns the local timezone name (e.g. "America/Vancouver").
/// Reads the /etc/localtime symlink (most reliable), then TZ env var, then UTC offset.
fn local_timezone_name() -> String {
    // Try /etc/localtime symlink (what the system actually uses)
    if let Ok(target) = std::fs::read_link("/etc/localtime") {
        let path = target.to_string_lossy();
        if let Some(tz) = path.strip_prefix("/usr/share/zoneinfo/") {
            return tz.to_string();
        }
    }

    // Try TZ env var
    if let Ok(tz) = std::env::var("TZ")
        && !tz.is_empty()
    {
        return tz;
    }

    // Fallback: UTC offset
    Local::now().format("%Z").to_string()
}

/// Renders the main clock widget into the given area.
pub fn render(frame: &mut Frame, area: Rect, config: &ClockConfig, colon_visible: bool) {
    let tz_name = local_timezone_name();
    let block = Block::bordered().title(format!(" {tz_name} "));
    let inner = block.inner(area);
    frame.render_widget(block, area);

    // Reserve 1 row at the bottom for the date line
    let chunks = Layout::vertical([
        Constraint::Min(1),
        Constraint::Length(1),
    ])
    .split(inner);

    let pixel_size = select_pixel_size(chunks[0].height);
    let time_str = format_time(config, colon_visible);

    let big_text = BigText::builder()
        .pixel_size(pixel_size)
        .style(Style::default().fg(Color::Cyan))
        .lines(vec![time_str.into()])
        .alignment(Alignment::Center)
        .build();

    frame.render_widget(big_text, chunks[0]);

    // Date line below the clock
    let now = Local::now();
    let date_str = now.format("%A, %B %d, %Y").to_string();
    let date_line = Line::from(Span::styled(
        date_str,
        Style::default()
            .fg(Color::DarkGray)
            .add_modifier(Modifier::ITALIC),
    ));
    frame.render_widget(
        Paragraph::new(date_line).alignment(Alignment::Center),
        chunks[1],
    );
}
