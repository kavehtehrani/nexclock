use chrono::Utc;
use chrono_tz::Tz;
use ratatui::{
    layout::{Alignment, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::Paragraph,
    Frame,
};

use crate::config::SecondaryClockConfig;
use crate::ui;

/// Renders the secondary timezone clock panel.
pub fn render(frame: &mut Frame, area: Rect, config: &SecondaryClockConfig, is_focused: bool) {
    let block = ui::panel_block(&config.label, is_focused);
    let inner = block.inner(area);
    frame.render_widget(block, area);

    let tz: Tz = match config.timezone.parse() {
        Ok(tz) => tz,
        Err(_) => {
            let err_line = Line::from("Invalid timezone").style(Style::default().fg(Color::Red));
            frame.render_widget(Paragraph::new(err_line).alignment(Alignment::Center), inner);
            return;
        }
    };

    let now = Utc::now().with_timezone(&tz);
    let time_str = now.format("%H:%M:%S").to_string();
    let date_str = now.format(&config.date_format).to_string();

    let lines = vec![
        Line::from(Span::styled(
            time_str,
            Style::default()
                .fg(Color::Yellow)
                .add_modifier(Modifier::BOLD),
        )),
        Line::from(Span::styled(
            date_str,
            Style::default().fg(Color::DarkGray),
        )),
    ];

    let paragraph = Paragraph::new(lines).alignment(Alignment::Center);

    // Vertically center within inner area
    let y_offset = inner.height.saturating_sub(2) / 2;
    let centered = Rect {
        x: inner.x,
        y: inner.y + y_offset,
        width: inner.width,
        height: inner.height.saturating_sub(y_offset),
    };

    frame.render_widget(paragraph, centered);
}
