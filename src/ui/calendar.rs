use chrono::{Datelike, Local, NaiveDate};
use ratatui::{
    layout::{Alignment, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Paragraph},
    Frame,
};

const DAYS_HEADER: &str = "Mo Tu We Th Fr Sa Su";

/// Renders a Gregorian calendar panel showing the current month with today highlighted.
pub fn render(frame: &mut Frame, area: Rect) {
    let today = Local::now().date_naive();
    let year = today.year();
    let month = today.month();

    let block = Block::bordered().title(format!(
        " {} {} ",
        month_name(month),
        year
    ));
    let inner = block.inner(area);
    frame.render_widget(block, area);

    let mut lines: Vec<Line> = Vec::new();

    // Day-of-week header
    lines.push(Line::from(Span::styled(
        DAYS_HEADER,
        Style::default()
            .fg(Color::Cyan)
            .add_modifier(Modifier::BOLD),
    )));

    // First day of the month
    let first = NaiveDate::from_ymd_opt(year, month, 1).unwrap();
    // Monday = 0 for our grid
    let start_weekday = first.weekday().num_days_from_monday() as usize;

    // Number of days in this month
    let days_in_month = days_in_month(year, month);

    let mut day = 1u32;
    let mut week_row = 0;

    while day <= days_in_month {
        let mut spans: Vec<Span> = Vec::new();

        for col in 0..7 {
            if (week_row == 0 && col < start_weekday) || day > days_in_month {
                spans.push(Span::raw("   "));
            } else {
                let is_today = day == today.day();
                let day_str = format!("{day:>2}");

                let style = if is_today {
                    Style::default()
                        .fg(Color::Black)
                        .bg(Color::Cyan)
                        .add_modifier(Modifier::BOLD)
                } else {
                    Style::default().fg(Color::White)
                };

                spans.push(Span::styled(day_str, style));
                day += 1;

                // Add space separator unless last column
                if col < 6 && day <= days_in_month + 1 {
                    spans.push(Span::raw(" "));
                    continue;
                }
            }

            // Add space separator unless last column
            if col < 6 {
                spans.push(Span::raw(" "));
            }
        }

        lines.push(Line::from(spans));
        week_row += 1;
    }

    let paragraph = Paragraph::new(lines).alignment(Alignment::Left);
    frame.render_widget(paragraph, inner);
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
    // Get the first day of the next month, then subtract one day
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
