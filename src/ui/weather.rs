use ratatui::{
    layout::{Alignment, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::Paragraph,
    Frame,
};

use crate::data::weather_api::WeatherData;
use crate::ui;

/// Renders the weather display panel.
pub fn render(frame: &mut Frame, area: Rect, weather: &Option<WeatherData>, is_focused: bool) {
    let block = ui::panel_block("Weather", is_focused);
    let inner = block.inner(area);
    frame.render_widget(block, area);

    let lines = match weather {
        Some(data) => {
            let mut lines = vec![
                Line::from(Span::styled(
                    format!("{:.1}°{} - {}", data.temperature, data.unit, data.description),
                    Style::default()
                        .fg(Color::Yellow)
                        .add_modifier(Modifier::BOLD),
                )),
            ];

            // Humidity + precipitation probability on a second line
            let mut detail_spans = Vec::new();

            if let Some(humidity) = data.humidity {
                detail_spans.push(Span::styled(
                    format!("Humidity: {humidity}%"),
                    Style::default().fg(Color::Cyan),
                ));
            }

            if let Some(precip) = data.precipitation_probability {
                if !detail_spans.is_empty() {
                    detail_spans.push(Span::raw("  "));
                }
                detail_spans.push(Span::styled(
                    format!("Rain: {precip}%"),
                    Style::default().fg(Color::Blue),
                ));
            }

            if !detail_spans.is_empty() {
                lines.push(Line::from(detail_spans));
            }

            lines
        }
        None => {
            vec![Line::from(Span::styled(
                "Loading...",
                Style::default().fg(Color::DarkGray),
            ))]
        }
    };

    let content_height = lines.len() as u16;
    let paragraph = Paragraph::new(lines).alignment(Alignment::Center);

    // Vertically center
    let y_offset = inner.height.saturating_sub(content_height) / 2;
    let centered = Rect {
        x: inner.x,
        y: inner.y + y_offset,
        width: inner.width,
        height: inner.height.saturating_sub(y_offset),
    };

    frame.render_widget(paragraph, centered);
}
