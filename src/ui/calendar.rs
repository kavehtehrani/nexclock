use chrono::{Datelike, Local, NaiveDate};
use ratatui::{
    layout::{Alignment, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::Paragraph,
    Frame,
};

use crate::app::{parse_color, ResolvedTheme};
use crate::component::ComponentStyle;
use crate::ui;

const DAYS_HEADER: &str = "Mo Tu We Th Fr Sa Su";

pub fn render(frame: &mut Frame, area: Rect, is_focused: bool, is_editing: bool, theme: &ResolvedTheme, comp_style: &ComponentStyle) {
    let today = Local::now().date_naive();
    let year = today.year();
    let month = today.month();

    let title = format!("{} {}", month_name(month), year);
    let block = ui::panel_block(&title, is_focused, is_editing, theme, comp_style);
    let inner = block.inner(area);
    frame.render_widget(block, area);

    let fg_color = comp_style
        .fg
        .as_deref()
        .map(parse_color);

    let mut lines: Vec<Line> = Vec::new();

    lines.push(Line::from(Span::styled(
        DAYS_HEADER,
        Style::default()
            .fg(fg_color.unwrap_or(theme.primary))
            .add_modifier(Modifier::BOLD),
    )));

    let first = NaiveDate::from_ymd_opt(year, month, 1).unwrap();
    let start_weekday = first.weekday().num_days_from_monday() as usize;
    let days_in_month = days_in_month(year, month);

    let mut day = 1u32;
    let mut week_row = 0;

    while day <= days_in_month {
        let mut spans: Vec<Span> = Vec::new();

        for col in 0..7 {
            if col > 0 {
                spans.push(Span::raw(" "));
            }

            if (week_row == 0 && col < start_weekday) || day > days_in_month {
                spans.push(Span::raw("  "));
            } else {
                let is_today = day == today.day();
                let day_str = format!("{day:>2}");

                let style = if is_today {
                    Style::default()
                        .fg(Color::Black)
                        .bg(fg_color.unwrap_or(theme.primary))
                        .add_modifier(Modifier::BOLD)
                } else {
                    Style::default().fg(theme.text)
                };

                spans.push(Span::styled(day_str, style));
                day += 1;
            }
        }

        lines.push(Line::from(spans));
        week_row += 1;
    }

    let content_height = lines.len() as u16;
    let content_width = DAYS_HEADER.len() as u16;
    let paragraph = Paragraph::new(lines).alignment(Alignment::Left);
    let centered = ui::centered_rect(inner, Some(content_width), content_height);
    frame.render_widget(paragraph, centered);
}

fn month_name(month: u32) -> &'static str {
    match month {
        1 => "January",
        2 => "February",
        3 => "March",
        4 => "April",
        5 => "May",
        6 => "June",
        7 => "July",
        8 => "August",
        9 => "September",
        10 => "October",
        11 => "November",
        12 => "December",
        _ => "Unknown",
    }
}

fn days_in_month(year: i32, month: u32) -> u32 {
    if month == 12 {
        NaiveDate::from_ymd_opt(year + 1, 1, 1)
    } else {
        NaiveDate::from_ymd_opt(year, month + 1, 1)
    }
    .unwrap()
    .pred_opt()
    .unwrap()
    .day()
}
