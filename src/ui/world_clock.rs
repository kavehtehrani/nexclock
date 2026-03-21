use chrono::Utc;
use chrono_tz::Tz;
use ratatui::{
    layout::{Alignment, Rect},
    style::{Modifier, Style},
    text::{Line, Span},
    widgets::Paragraph,
    Frame,
};

use crate::app::{parse_color, ResolvedTheme};
use crate::component::{ComponentStyle, WorldClockSettings};
use crate::ui;

const LABEL_TIME_GAP: usize = 2;
const COLUMN_GAP: usize = 3;

pub fn render(
    frame: &mut Frame,
    area: Rect,
    settings: &WorldClockSettings,
    is_focused: bool,
    is_editing: bool,
    theme: &ResolvedTheme,
    comp_style: &ComponentStyle,
) {
    let block = ui::panel_block("World Clock", is_focused, is_editing, theme, comp_style);
    let inner = block.inner(area);
    frame.render_widget(block, area);

    if settings.timezones.is_empty() {
        let hint = Line::styled(
            "No timezones configured",
            Style::default().fg(theme.muted),
        );
        let centered = ui::centered_rect(inner, None, 1);
        frame.render_widget(
            Paragraph::new(hint).alignment(Alignment::Center),
            centered,
        );
        return;
    }

    let use_24h = settings.time_format == "24h";
    let fmt = match (use_24h, settings.show_seconds) {
        (true, true) => "%H:%M:%S",
        (true, false) => "%H:%M",
        (false, true) => "%I:%M:%S %p",
        (false, false) => "%I:%M %p",
    };

    let fg_color = comp_style.fg.as_deref().map(parse_color);
    let now = Utc::now();

    // Pre-compute all labels and time strings
    let entries: Vec<(&str, String)> = settings
        .timezones
        .iter()
        .map(|entry| {
            let label = entry.label.as_deref().unwrap_or(&entry.timezone);
            let time_str = if let Ok(tz) = entry.timezone.parse::<Tz>() {
                now.with_timezone(&tz).format(fmt).to_string()
            } else {
                "invalid tz".to_string()
            };
            (label, time_str)
        })
        .collect();

    let count = entries.len();
    let available_rows = inner.height as usize;
    let use_two_columns = count > available_rows && available_rows > 0;

    if use_two_columns {
        render_two_columns(frame, inner, &entries, fg_color, theme);
    } else {
        render_single_column(frame, inner, &entries, fg_color, theme);
    }
}

fn render_single_column(
    frame: &mut Frame,
    inner: Rect,
    entries: &[(&str, String)],
    fg_color: Option<ratatui::style::Color>,
    theme: &ResolvedTheme,
) {
    let max_label = entries.iter().map(|(l, _)| l.len()).max().unwrap_or(0);
    let max_time = entries.iter().map(|(_, t)| t.len()).max().unwrap_or(0);
    let content_width = max_label + LABEL_TIME_GAP + max_time;

    let lines: Vec<Line> = entries
        .iter()
        .take(inner.height as usize)
        .map(|(label, time_str)| make_row(label, max_label, time_str, fg_color, theme))
        .collect();

    let content_height = lines.len() as u16;
    let centered = ui::centered_rect(inner, Some(content_width as u16), content_height);
    frame.render_widget(
        Paragraph::new(lines).alignment(Alignment::Left),
        centered,
    );
}

fn render_two_columns(
    frame: &mut Frame,
    inner: Rect,
    entries: &[(&str, String)],
    fg_color: Option<ratatui::style::Color>,
    theme: &ResolvedTheme,
) {
    let rows = entries.len().div_ceil(2);
    let left = &entries[..rows];
    let right = &entries[rows..];

    let left_max_label = left.iter().map(|(l, _)| l.len()).max().unwrap_or(0);
    let left_max_time = left.iter().map(|(_, t)| t.len()).max().unwrap_or(0);
    let right_max_label = right.iter().map(|(l, _)| l.len()).max().unwrap_or(0);
    let right_max_time = right.iter().map(|(_, t)| t.len()).max().unwrap_or(0);

    let left_col_w = left_max_label + LABEL_TIME_GAP + left_max_time;
    let right_col_w = right_max_label + LABEL_TIME_GAP + right_max_time;
    let content_width = left_col_w + COLUMN_GAP + right_col_w;

    let mut lines: Vec<Line> = Vec::with_capacity(rows);
    for i in 0..rows {
        let (ll, lt) = &left[i];
        let mut spans = row_spans(ll, left_max_label, lt, fg_color, theme);

        if let Some((rl, rt)) = right.get(i) {
            spans.push(Span::raw(" ".repeat(COLUMN_GAP)));
            spans.extend(row_spans(rl, right_max_label, rt, fg_color, theme));
        }

        lines.push(Line::from(spans));
    }

    let content_height = lines.len() as u16;
    let centered = ui::centered_rect(inner, Some(content_width as u16), content_height);
    frame.render_widget(
        Paragraph::new(lines).alignment(Alignment::Left),
        centered,
    );
}

fn make_row<'a>(
    label: &str,
    label_width: usize,
    time_str: &str,
    fg_color: Option<ratatui::style::Color>,
    theme: &ResolvedTheme,
) -> Line<'a> {
    Line::from(row_spans(label, label_width, time_str, fg_color, theme))
}

fn row_spans(
    label: &str,
    label_width: usize,
    time_str: &str,
    fg_color: Option<ratatui::style::Color>,
    theme: &ResolvedTheme,
) -> Vec<Span<'static>> {
    vec![
        Span::styled(
            format!("{label:<w$}", w = label_width),
            Style::default().fg(theme.text),
        ),
        Span::raw(" ".repeat(LABEL_TIME_GAP)),
        Span::styled(
            time_str.to_string(),
            Style::default()
                .fg(fg_color.unwrap_or(theme.secondary))
                .add_modifier(Modifier::BOLD),
        ),
    ]
}
