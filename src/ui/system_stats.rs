use ratatui::{
    layout::{Alignment, Rect},
    style::{Modifier, Style},
    text::{Line, Span},
    widgets::Paragraph,
    Frame,
};

use crate::app::ResolvedTheme;
use crate::data::system::SystemStats;
use crate::ui;

pub fn render(
    frame: &mut Frame,
    area: Rect,
    stats: &SystemStats,
    is_focused: bool,
    is_editing: bool,
    theme: &ResolvedTheme,
) {
    let block = ui::panel_block("System", is_focused, is_editing, theme);
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
                    .fg(theme.muted)
                    .add_modifier(Modifier::BOLD),
            ),
            Span::styled(temp_str, Style::default().fg(theme.secondary)),
        ]),
        Line::from(vec![
            Span::styled(
                "Mem: ",
                Style::default()
                    .fg(theme.muted)
                    .add_modifier(Modifier::BOLD),
            ),
            Span::styled(mem_str, Style::default().fg(theme.tertiary)),
        ]),
        Line::from(vec![
            Span::styled(
                "Up:  ",
                Style::default()
                    .fg(theme.muted)
                    .add_modifier(Modifier::BOLD),
            ),
            Span::styled(uptime_str, Style::default().fg(theme.primary)),
        ]),
    ];

    let paragraph = Paragraph::new(lines).alignment(Alignment::Left);

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
