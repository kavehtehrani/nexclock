use ratatui::{
    layout::Rect,
    style::{Color, Style},
    text::{Line, Span},
    widgets::{Block, Paragraph},
    Frame,
};

/// Renders the bottom status bar with external IP and status info.
pub fn render(frame: &mut Frame, area: Rect, ip: &Option<String>) {
    let block = Block::bordered();
    let inner = block.inner(area);
    frame.render_widget(block, area);

    let ip_text = match ip {
        Some(ip) => format!("IP: {ip}"),
        None => "IP: Loading...".to_string(),
    };

    let line = Line::from(vec![
        Span::styled("  ", Style::default()),
        Span::styled(ip_text, Style::default().fg(Color::DarkGray)),
    ]);

    frame.render_widget(Paragraph::new(line), inner);
}
