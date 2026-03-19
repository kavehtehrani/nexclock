use ratatui::{
    layout::Rect,
    style::Style,
    text::{Line, Span},
    widgets::{Block, Paragraph},
    Frame,
};

use crate::app::ResolvedTheme;

pub fn render(
    frame: &mut Frame,
    area: Rect,
    ip: &Option<String>,
    font_name: &str,
    is_editing: bool,
    theme: &ResolvedTheme,
) {
    let block = Block::bordered();
    let inner = block.inner(area);
    frame.render_widget(block, area);

    let line = if is_editing {
        let edit_style = Style::default()
            .fg(theme.secondary)
            .add_modifier(ratatui::style::Modifier::BOLD);
        Line::from(vec![
            Span::styled("  ", Style::default()),
            Span::styled("EDIT", edit_style),
            Span::styled("  ", Style::default()),
            Span::styled(
                "Arrows: resize | Shift+Arrows: move | Esc: exit edit",
                Style::default().fg(theme.muted),
            ),
        ])
    } else {
        let ip_text = match ip {
            Some(ip) => format!("IP: {ip}"),
            None => "IP: Loading...".to_string(),
        };

        let muted = Style::default().fg(theme.muted);

        Line::from(vec![
            Span::styled("  ", Style::default()),
            Span::styled(ip_text, muted),
            Span::styled("  |  ", muted),
            Span::styled(format!("Font: {font_name}"), muted),
            Span::styled("  |  ", muted),
            Span::styled("Tab: navigate | Space: menu | e: edit | a: add | h: help", muted),
        ])
    };

    frame.render_widget(Paragraph::new(line), inner);
}
