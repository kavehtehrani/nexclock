use ratatui::{
    layout::{Alignment, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Paragraph},
    Frame,
};

use crate::data::weather_api::WeatherData;

/// Renders the weather display panel.
pub fn render(frame: &mut Frame, area: Rect, weather: &Option<WeatherData>) {
    let block = Block::bordered().title(" Weather ");
    let inner = block.inner(area);
    frame.render_widget(block, area);

    let lines = match weather {
        Some(data) => {
            vec![
                Line::from(Span::styled(
                    format!("{:.1}°{}", data.temperature, data.unit),
                    Style::default()
                        .fg(Color::Yellow)
                        .add_modifier(Modifier::BOLD),
                )),
                Line::from(Span::styled(
                    &data.description,
                    Style::default().fg(Color::White),
                )),
            ]
        }
        None => {
            vec![Line::from(Span::styled(
                "Loading...",
                Style::default().fg(Color::DarkGray),
            ))]
        }
    };

    let paragraph = Paragraph::new(lines).alignment(Alignment::Center);

    // Vertically center
    let y_offset = inner.height.saturating_sub(2) / 2;
    let centered = Rect {
        x: inner.x,
        y: inner.y + y_offset,
        width: inner.width,
        height: inner.height.saturating_sub(y_offset),
    };

    frame.render_widget(paragraph, centered);
}
