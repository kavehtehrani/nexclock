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
    is_editing: bool,
    theme: &ResolvedTheme,
) {
    let block = Block::bordered();
    let inner = block.inner(area);
    frame.render_widget(block, area);

    let dim = Style::default().fg(theme.text);

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
                dim,
            ),
        ])
    } else {
        let ip_text = match ip {
            Some(ip) => format!("IP: {ip}"),
            None => "IP: Loading...".to_string(),
        };

        Line::from(vec![
            Span::styled("  ", Style::default()),
            Span::styled(ip_text, dim),
            Span::styled("  |  ", dim),
            Span::styled("Tab: navigate | Space: menu | e: edit | a: add | h: help", dim),
        ])
    };

    frame.render_widget(Paragraph::new(line), inner);
}
