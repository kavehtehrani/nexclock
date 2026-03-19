use chrono::Local;
use ratatui::{
    layout::{Alignment, Rect},
    style::{Color, Style},
    widgets::Block,
    Frame,
};
use tui_big_text::{BigText, PixelSize};

use crate::config::ClockConfig;
use crate::constants::{PIXEL_SIZE_FULL_MIN_HEIGHT, PIXEL_SIZE_HALF_MIN_HEIGHT};

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

/// Formats the current time string based on config.
fn format_time(config: &ClockConfig) -> String {
    let now = Local::now();
    let use_24h = config.time_format == "24h";
    match (use_24h, config.show_seconds) {
        (true, true) => now.format("%H:%M:%S").to_string(),
        (true, false) => now.format("%H:%M").to_string(),
        (false, true) => now.format("%I:%M:%S %p").to_string(),
        (false, false) => now.format("%I:%M %p").to_string(),
    }
}

/// Renders the main clock widget into the given area.
pub fn render(frame: &mut Frame, area: Rect, config: &ClockConfig) {
    let block = Block::bordered().title(" Clock ");
    let inner = block.inner(area);
    frame.render_widget(block, area);

    let pixel_size = select_pixel_size(inner.height);
    let time_str = format_time(config);

    let big_text = BigText::builder()
        .pixel_size(pixel_size)
        .style(Style::default().fg(Color::Cyan))
        .lines(vec![time_str.into()])
        .alignment(Alignment::Center)
        .build();

    frame.render_widget(big_text, inner);
}
