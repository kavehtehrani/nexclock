use ratatui::{
    layout::{Alignment, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::Paragraph,
    Frame,
};

use crate::data::system::SystemStats;
use crate::ui;

/// Renders the system stats panel.
pub fn render(frame: &mut Frame, area: Rect, stats: &SystemStats, is_focused: bool) {
    let block = ui::panel_block("System", is_focused);
    let inner = block.inner(area);
    frame.render_widget(block, area);

    let na = "N/A".to_string();

    let temp_str = stats
        .cpu_temp
        .map(|t| format!("{t:.1}°C"))
        .unwrap_or_else(|| na.clone());

    let mem_str = match (stats.memory_used_mb, stats.memory_total_mb) {
        (Some(used), Some(total)) => format!("{used} / {total} MB"),
        _ => na.clone(),
    };

    let uptime_str = stats.uptime.as_deref().unwrap_or("N/A");

    let lines = vec![
        Line::from(vec![
            Span::styled(
                "CPU: ",
                Style::default()
                    .fg(Color::DarkGray)
                    .add_modifier(Modifier::BOLD),
            ),
            Span::styled(temp_str, Style::default().fg(Color::Yellow)),
        ]),
        Line::from(vec![
            Span::styled(
                "Mem: ",
                Style::default()
                    .fg(Color::DarkGray)
                    .add_modifier(Modifier::BOLD),
            ),
            Span::styled(mem_str, Style::default().fg(Color::Green)),
        ]),
        Line::from(vec![
            Span::styled(
                "Up:  ",
                Style::default()
                    .fg(Color::DarkGray)
                    .add_modifier(Modifier::BOLD),
            ),
            Span::styled(uptime_str, Style::default().fg(Color::Cyan)),
        ]),
    ];

    let paragraph = Paragraph::new(lines).alignment(Alignment::Left);

    // Vertically center within inner area
    let content_height = 3u16;
    let y_offset = inner.height.saturating_sub(content_height) / 2;
    let centered = Rect {
        x: inner.x + 1,
        y: inner.y + y_offset,
        width: inner.width.saturating_sub(1),
        height: inner.height.saturating_sub(y_offset),
    };

    frame.render_widget(paragraph, centered);
}
