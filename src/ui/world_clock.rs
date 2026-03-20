use chrono::Utc;
use chrono_tz::Tz;
use ratatui::{
    layout::Rect,
    style::{Modifier, Style},
    text::{Line, Span},
    widgets::Paragraph,
    Frame,
};

use crate::app::ResolvedTheme;
use crate::component::WorldClockSettings;
use crate::ui;

pub fn render(
    frame: &mut Frame,
    area: Rect,
    settings: &WorldClockSettings,
    is_focused: bool,
    is_editing: bool,
    theme: &ResolvedTheme,
) {
    let block = ui::panel_block("World Clock", is_focused, is_editing, theme);
    let inner = block.inner(area);
    frame.render_widget(block, area);

    if settings.timezones.is_empty() {
        let hint = Line::styled(
            "No timezones configured",
            Style::default().fg(theme.muted),
        );
        frame.render_widget(Paragraph::new(hint), inner);
        return;
    }

    let use_24h = settings.time_format == "24h";
    let fmt = match (use_24h, settings.show_seconds) {
        (true, true) => "%H:%M:%S",
        (true, false) => "%H:%M",
        (false, true) => "%I:%M:%S %p",
        (false, false) => "%I:%M %p",
    };

    let now = Utc::now();
    let inner_width = inner.width as usize;

    let lines: Vec<Line> = settings
        .timezones
        .iter()
        .take(inner.height as usize)
        .map(|entry| {
            let label = entry
                .label
                .as_deref()
                .unwrap_or(&entry.timezone);

            let time_str = if let Ok(tz) = entry.timezone.parse::<Tz>() {
                now.with_timezone(&tz).format(fmt).to_string()
            } else {
                "invalid tz".to_string()
            };

            let time_len = time_str.len();
            // Pad label to fill available width, leaving room for time
            let label_width = inner_width.saturating_sub(time_len + 1);
            let truncated_label = if label.len() > label_width {
                &label[..label_width]
            } else {
                label
            };

            Line::from(vec![
                Span::styled(
                    format!("{truncated_label:<w$}", w = label_width),
                    Style::default().fg(theme.muted),
                ),
                Span::raw(" "),
                Span::styled(
                    time_str,
                    Style::default()
                        .fg(theme.secondary)
                        .add_modifier(Modifier::BOLD),
                ),
            ])
        })
        .collect();

    frame.render_widget(Paragraph::new(lines), inner);
}
