use chrono::{Local, Utc};
use chrono_tz::Tz;
use figlet_rs::{FIGlet, Toilet};
use ratatui::{
    layout::{Alignment, Constraint, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::Paragraph,
    Frame,
};

use crate::app::{FontStyle, ResolvedTheme};
use crate::component::{ClockSettings, ClockStyle};
use crate::ui;

const BLINK_REPLACEMENT: char = ' ';

/// Renders a clock component (either large FIGlet or compact text style).
#[allow(clippy::too_many_arguments)]
pub fn render(
    frame: &mut Frame,
    area: Rect,
    settings: &ClockSettings,
    tick_count: u64,
    font_style: FontStyle,
    is_focused: bool,
    is_editing: bool,
    theme: &ResolvedTheme,
) {
    match settings.style {
        ClockStyle::Large => render_large(frame, area, settings, tick_count, font_style, is_focused, is_editing, theme),
        ClockStyle::Compact => render_compact(frame, area, settings, is_focused, is_editing, theme),
    }
}

/// Returns the title for the clock panel.
fn clock_title(settings: &ClockSettings) -> String {
    if let Some(ref label) = settings.label {
        return label.clone();
    }
    if let Some(ref tz_str) = settings.timezone {
        return tz_str.clone();
    }
    local_timezone_name()
}

// ── Large (FIGlet) style ────────────────────────────────────────────

#[allow(clippy::too_many_arguments)]
fn render_large(
    frame: &mut Frame,
    area: Rect,
    settings: &ClockSettings,
    tick_count: u64,
    font_style: FontStyle,
    is_focused: bool,
    is_editing: bool,
    theme: &ResolvedTheme,
) {
    let title = clock_title(settings);
    let block = ui::panel_block(&title, is_focused, is_editing, theme);
    let inner = block.inner(area);
    frame.render_widget(block, area);

    let chunks = Layout::vertical([Constraint::Min(1), Constraint::Length(1)]).split(inner);

    let colon_visible = if settings.blink_separator {
        tick_count.is_multiple_of(2)
    } else {
        true
    };

    let time_str = format_time(settings, colon_visible);
    render_figlet_clock(frame, chunks[0], &time_str, font_style, theme.primary);

    let date_str = format_date(settings);
    let date_line = Line::from(Span::styled(
        date_str,
        Style::default()
            .fg(theme.muted)
            .add_modifier(Modifier::ITALIC),
    ));
    frame.render_widget(
        Paragraph::new(date_line).alignment(Alignment::Center),
        chunks[1],
    );
}

// ── Compact style ───────────────────────────────────────────────────

fn render_compact(
    frame: &mut Frame,
    area: Rect,
    settings: &ClockSettings,
    is_focused: bool,
    is_editing: bool,
    theme: &ResolvedTheme,
) {
    let title = clock_title(settings);
    let block = ui::panel_block(&title, is_focused, is_editing, theme);
    let inner = block.inner(area);
    frame.render_widget(block, area);

    // Validate timezone if specified
    if let Some(ref tz_str) = settings.timezone
        && tz_str.parse::<Tz>().is_err() {
            let err_line =
                Line::from("Invalid timezone").style(Style::default().fg(theme.error));
            frame.render_widget(
                Paragraph::new(err_line).alignment(Alignment::Center),
                inner,
            );
            return;
        }

    let time_str = format_time(settings, true);
    let date_str = format_date(settings);

    let lines = vec![
        Line::from(Span::styled(
            time_str,
            Style::default()
                .fg(theme.secondary)
                .add_modifier(Modifier::BOLD),
        )),
        Line::from(Span::styled(
            date_str,
            Style::default().fg(theme.muted),
        )),
    ];

    let paragraph = Paragraph::new(lines).alignment(Alignment::Center);

    let y_offset = inner.height.saturating_sub(2) / 2;
    let centered = Rect {
        x: inner.x,
        y: inner.y + y_offset,
        width: inner.width,
        height: inner.height.saturating_sub(y_offset),
    };

    frame.render_widget(paragraph, centered);
}

// ── Time/date formatting ────────────────────────────────────────────

fn format_time(settings: &ClockSettings, colon_visible: bool) -> String {
    let use_24h = settings.time_format == "24h";

    let time_str = if let Some(ref tz_str) = settings.timezone {
        if let Ok(tz) = tz_str.parse::<Tz>() {
            let now = Utc::now().with_timezone(&tz);
            match (use_24h, settings.show_seconds) {
                (true, true) => now.format("%H:%M:%S").to_string(),
                (true, false) => now.format("%H:%M").to_string(),
                (false, true) => now.format("%I:%M:%S %p").to_string(),
                (false, false) => now.format("%I:%M %p").to_string(),
            }
        } else {
            "??:??".to_string()
        }
    } else {
        let now = Local::now();
        match (use_24h, settings.show_seconds) {
            (true, true) => now.format("%H:%M:%S").to_string(),
            (true, false) => now.format("%H:%M").to_string(),
            (false, true) => now.format("%I:%M:%S %p").to_string(),
            (false, false) => now.format("%I:%M %p").to_string(),
        }
    };

    if !colon_visible {
        time_str.replace(':', &BLINK_REPLACEMENT.to_string())
    } else {
        time_str
    }
}

fn format_date(settings: &ClockSettings) -> String {
    if let Some(ref tz_str) = settings.timezone
        && let Ok(tz) = tz_str.parse::<Tz>() {
            return Utc::now()
                .with_timezone(&tz)
                .format(&settings.date_format)
                .to_string();
        }
    Local::now().format(&settings.date_format).to_string()
}

/// Returns the local timezone name (e.g. "America/Vancouver").
fn local_timezone_name() -> String {
    if let Ok(target) = std::fs::read_link("/etc/localtime") {
        let path = target.to_string_lossy();
        if let Some(tz) = path.strip_prefix("/usr/share/zoneinfo/") {
            return tz.to_string();
        }
    }

    if let Ok(tz) = std::env::var("TZ")
        && !tz.is_empty()
    {
        return tz;
    }

    Local::now().format("%Z").to_string()
}

// ── FIGlet rendering ────────────────────────────────────────────────

enum Font {
    Figlet(FIGlet),
    Toilet(Toilet),
}

impl Font {
    fn convert(&self, text: &str) -> Option<String> {
        match self {
            Self::Figlet(f) => f.convert(text).map(|fig| fig.to_string()),
            Self::Toilet(t) => t.convert(text).map(|fig| fig.to_string()),
        }
    }
}

fn load_font(style: FontStyle) -> Option<Font> {
    match style {
        FontStyle::Standard => FIGlet::standard().ok().map(Font::Figlet),
        FontStyle::Big => FIGlet::big().ok().map(Font::Figlet),
        FontStyle::Small => FIGlet::small().ok().map(Font::Figlet),
        FontStyle::Slant => FIGlet::slant().ok().map(Font::Figlet),
        FontStyle::SmBlock => Toilet::smblock().ok().map(Font::Toilet),
        FontStyle::Mono12 => Toilet::mono12().ok().map(Font::Toilet),
        FontStyle::Future => Toilet::future().ok().map(Font::Toilet),
        FontStyle::Wideterm => Toilet::wideterm().ok().map(Font::Toilet),
        FontStyle::Mono9 => Toilet::mono9().ok().map(Font::Toilet),
    }
}

fn render_figlet_clock(
    frame: &mut Frame,
    area: Rect,
    time_str: &str,
    style: FontStyle,
    color: Color,
) {
    let Some(font) = load_font(style) else {
        render_plain_fallback(frame, area, time_str, color);
        return;
    };

    let Some(art) = font.convert(time_str) else {
        render_plain_fallback(frame, area, time_str, color);
        return;
    };

    // Clamp to available height to prevent overflow into adjacent panels
    let max_lines = area.height as usize;
    let lines: Vec<Line> = art
        .lines()
        .filter(|l| !l.is_empty() || art.lines().count() <= max_lines)
        .take(max_lines)
        .map(|l: &str| Line::styled(l.to_string(), Style::default().fg(color)))
        .collect();

    // Fallback to plain text if FIGlet produced no visible lines
    if lines.is_empty() {
        render_plain_fallback(frame, area, time_str, color);
        return;
    }

    let content_height = lines.len() as u16;
    let y_offset = area.height.saturating_sub(content_height) / 2;

    let centered = Rect {
        x: area.x,
        y: area.y + y_offset,
        width: area.width,
        height: content_height.min(area.height.saturating_sub(y_offset)),
    };

    frame.render_widget(
        Paragraph::new(lines).alignment(Alignment::Center),
        centered,
    );
}

fn render_plain_fallback(frame: &mut Frame, area: Rect, time_str: &str, color: Color) {
    let line = Line::styled(time_str, Style::default().fg(color));
    frame.render_widget(
        Paragraph::new(line).alignment(Alignment::Center),
        area,
    );
}
