use ratatui::{
    layout::{Alignment, Rect},
    style::{Modifier, Style},
    text::{Line, Span},
    widgets::Paragraph,
    Frame,
};

use crate::app::ResolvedTheme;
use crate::data::weather_api::WeatherData;
use crate::ui;

pub fn render(
    frame: &mut Frame,
    area: Rect,
    weather: &Option<WeatherData>,
    is_focused: bool,
    is_editing: bool,
    theme: &ResolvedTheme,
) {
    let block = ui::panel_block("Weather", is_focused, is_editing, theme);
    let inner = block.inner(area);
    frame.render_widget(block, area);

    let lines = match weather {
        Some(data) => {
            let mut lines = vec![
                Line::from(Span::styled(
                    data.description.clone(),
                    Style::default()
                        .fg(theme.secondary)
                        .add_modifier(Modifier::BOLD),
                )),
            ];

            let mut detail_spans = vec![Span::styled(
                format!("{:.1}°{}", data.temperature, data.unit),
                Style::default().fg(theme.primary),
            )];

            if let Some(humidity) = data.humidity {
                detail_spans.push(Span::raw("  "));
                detail_spans.push(Span::styled(
                    format!("{humidity}% humidity"),
                    Style::default().fg(theme.text),
                ));
            }

            if let Some(precip) = data.precipitation_probability {
                detail_spans.push(Span::raw("  "));
                detail_spans.push(Span::styled(
                    format!("{precip}% rain"),
                    Style::default().fg(theme.info),
                ));
            }

            lines.push(Line::from(detail_spans));

            lines
        }
        None => {
            vec![Line::from(Span::styled(
                "Loading...",
                Style::default().fg(theme.muted),
            ))]
        }
    };

    let content_height = lines.len() as u16;
    let paragraph = Paragraph::new(lines).alignment(Alignment::Center);

    let y_offset = inner.height.saturating_sub(content_height) / 2;
    let centered = Rect {
        x: inner.x,
        y: inner.y + y_offset,
        width: inner.width,
        height: inner.height.saturating_sub(y_offset),
    };

    frame.render_widget(paragraph, centered);
}
