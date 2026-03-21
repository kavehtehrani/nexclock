use ratatui::{
    layout::{Alignment, Rect},
    style::{Modifier, Style},
    text::{Line, Span},
    widgets::Paragraph,
    Frame,
};

use crate::app::{parse_color, ResolvedTheme};
use crate::component::ComponentStyle;
use crate::data::weather_api::WeatherData;
use crate::ui;

pub fn render(
    frame: &mut Frame,
    area: Rect,
    weather: &Option<WeatherData>,
    is_focused: bool,
    is_editing: bool,
    theme: &ResolvedTheme,
    comp_style: &ComponentStyle,
) {
    let block = ui::panel_block("Weather", is_focused, is_editing, theme, comp_style);
    let inner = block.inner(area);
    frame.render_widget(block, area);

    let fg_color = comp_style
        .fg
        .as_deref()
        .map(parse_color);

    let lines = match weather {
        Some(data) => {
            let mut lines = vec![
                Line::from(Span::styled(
                    data.description.clone(),
                    Style::default()
                        .fg(fg_color.unwrap_or(theme.secondary))
                        .add_modifier(Modifier::BOLD),
                )),
            ];

            let mut detail_spans = vec![Span::styled(
                format!("{:.1}°{}", data.temperature, data.unit),
                Style::default().fg(fg_color.unwrap_or(theme.primary)),
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
    let centered = ui::centered_rect(inner, None, content_height);
    frame.render_widget(paragraph, centered);
}
