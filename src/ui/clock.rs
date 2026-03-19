use chrono::Local;
use figlet_rs::{FIGlet, Toilet};
use ratatui::{
    layout::{Alignment, Constraint, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::Paragraph,
    Frame,
};

use crate::app::FontStyle;
use crate::config::ClockConfig;
use crate::ui;

const BLINK_REPLACEMENT: char = ' ';

/// Formats the current time string based on config, with optional colon blinking.
fn format_time(config: &ClockConfig, colon_visible: bool) -> String {
    let now = Local::now();
    let use_24h = config.time_format == "24h";
    let time_str = match (use_24h, config.show_seconds) {
        (true, true) => now.format("%H:%M:%S").to_string(),
        (true, false) => now.format("%H:%M").to_string(),
        (false, true) => now.format("%I:%M:%S %p").to_string(),
        (false, false) => now.format("%I:%M %p").to_string(),
    };

    if !colon_visible {
        time_str.replace(':', &BLINK_REPLACEMENT.to_string())
    } else {
        time_str
    }
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

/// Font can be either FIGlet or Toilet, both produce FIGure via convert().
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

/// Loads a font by style.
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

/// Renders the main clock widget into the given area.
/// Returns the clock area rect for mouse click detection.
pub fn render(
    frame: &mut Frame,
    area: Rect,
    config: &ClockConfig,
    colon_visible: bool,
    font_style: FontStyle,
    is_focused: bool,
) -> Rect {
    let tz_name = local_timezone_name();
    let block = ui::panel_block(&tz_name, is_focused);
    let inner = block.inner(area);
    frame.render_widget(block, area);

    // Reserve 1 row at the bottom for the date line
    let chunks = Layout::vertical([
        Constraint::Min(1),
        Constraint::Length(1),
    ])
    .split(inner);

    let time_str = format_time(config, colon_visible);
    render_figlet_clock(frame, chunks[0], &time_str, font_style);

    // Date line below the clock
    let now = Local::now();
    let date_str = now.format(&config.date_format).to_string();
    let date_line = Line::from(Span::styled(
        date_str,
        Style::default()
            .fg(Color::DarkGray)
            .add_modifier(Modifier::ITALIC),
    ));
    frame.render_widget(
        Paragraph::new(date_line).alignment(Alignment::Center),
        chunks[1],
    );

    area
}

/// Renders the time string as FIGlet ASCII art, centered in the area.
fn render_figlet_clock(frame: &mut Frame, area: Rect, time_str: &str, style: FontStyle) {
    let Some(font) = load_font(style) else {
        render_plain_fallback(frame, area, time_str);
        return;
    };

    let Some(art) = font.convert(time_str) else {
        render_plain_fallback(frame, area, time_str);
        return;
    };

    let lines: Vec<Line> = art
        .lines()
        .map(|l: &str| Line::styled(l.to_string(), Style::default().fg(Color::Cyan)))
        .collect();

    let content_height = lines.len() as u16;
    let y_offset = area.height.saturating_sub(content_height) / 2;

    let centered = Rect {
        x: area.x,
        y: area.y + y_offset,
        width: area.width,
        height: area.height.saturating_sub(y_offset),
    };

    frame.render_widget(
        Paragraph::new(lines).alignment(Alignment::Center),
        centered,
    );
}

fn render_plain_fallback(frame: &mut Frame, area: Rect, time_str: &str) {
    let line = Line::styled(time_str, Style::default().fg(Color::Cyan));
    frame.render_widget(
        Paragraph::new(line).alignment(Alignment::Center),
        area,
    );
}
