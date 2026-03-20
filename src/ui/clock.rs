use chrono::{Local, Utc};
use chrono_tz::Tz;
use ratatui::{
    layout::{Alignment, Constraint, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::Paragraph,
    Frame,
};

use crate::app::{color_to_rgb, parse_color, FontStyle, ResolvedTheme};
use crate::component::{ClockSettings, ClockStyle};
use crate::ui;

/// Resolves a component's color list, falling back to the given theme default.
fn resolve_colors(colors: &[String], fallback: Color) -> Vec<Color> {
    if colors.is_empty() {
        vec![fallback]
    } else {
        colors.iter().map(|s| parse_color(s)).collect()
    }
}

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
        tick_count.is_multiple_of(4)
    } else {
        true
    };

    let time_str = format_time(settings, colon_visible);
    let colors = resolve_colors(&settings.colors, theme.primary);
    render_figlet_clock(frame, chunks[0], &time_str, font_style, &colors);

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

    let time_color = resolve_colors(&settings.colors, theme.secondary)[0];

    let lines = vec![
        Line::from(Span::styled(
            time_str,
            Style::default()
                .fg(time_color)
                .add_modifier(Modifier::BOLD),
        )),
        Line::from(Span::styled(
            date_str,
            Style::default().fg(theme.muted),
        )),
    ];

    let paragraph = Paragraph::new(lines).alignment(Alignment::Center);
    let centered = ui::centered_rect(inner, None, 2);
    frame.render_widget(paragraph, centered);
}

// ── Time/date formatting ────────────────────────────────────────────

fn time_format_str(use_24h: bool, show_seconds: bool) -> &'static str {
    match (use_24h, show_seconds) {
        (true, true) => "%H:%M:%S",
        (true, false) => "%H:%M",
        (false, true) => "%I:%M:%S %p",
        (false, false) => "%I:%M %p",
    }
}

fn format_time(settings: &ClockSettings, colon_visible: bool) -> String {
    let use_24h = settings.time_format == "24h";
    let fmt = time_format_str(use_24h, settings.show_seconds);

    let time_str = if let Some(ref tz_str) = settings.timezone {
        if let Ok(tz) = tz_str.parse::<Tz>() {
            Utc::now().with_timezone(&tz).format(fmt).to_string()
        } else {
            "??:??".to_string()
        }
    } else {
        Local::now().format(fmt).to_string()
    };

    if !colon_visible {
        time_str.replace(':', " ")
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

// ── cfonts rendering ─────────────────────────────────────────────────

fn to_cfonts_font(style: FontStyle) -> cfonts::Fonts {
    match style {
        FontStyle::Block => cfonts::Fonts::FontBlock,
        FontStyle::Slick => cfonts::Fonts::FontSlick,
        FontStyle::Tiny => cfonts::Fonts::FontTiny,
        FontStyle::Grid => cfonts::Fonts::FontGrid,
        FontStyle::Pallet => cfonts::Fonts::FontPallet,
        FontStyle::Shade => cfonts::Fonts::FontShade,
        FontStyle::Chrome => cfonts::Fonts::FontChrome,
        FontStyle::Simple => cfonts::Fonts::FontSimple,
        FontStyle::SimpleBlock => cfonts::Fonts::FontSimpleBlock,
        FontStyle::Simple3d => cfonts::Fonts::FontSimple3d,
        FontStyle::Huge => cfonts::Fonts::FontHuge,
        FontStyle::Console => cfonts::Fonts::FontConsole,
    }
}

/// Interpolates across an arbitrary number of color stops.
/// Maps `index` (0..total-1) onto the gradient defined by `colors`.
pub fn lerp_color(colors: &[Color], index: usize, total: usize) -> Color {
    if colors.len() < 2 || total <= 1 {
        return colors[0];
    }

    let segments = colors.len() - 1;
    let t = index as f32 / (total - 1) as f32;
    let scaled = t * segments as f32;
    let seg = (scaled as usize).min(segments - 1);
    let local_t = scaled - seg as f32;

    let (r1, g1, b1) = color_to_rgb(colors[seg]);
    let (r2, g2, b2) = color_to_rgb(colors[seg + 1]);
    Color::Rgb(
        (r1 as f32 + (r2 as f32 - r1 as f32) * local_t) as u8,
        (g1 as f32 + (g2 as f32 - g1 as f32) * local_t) as u8,
        (b1 as f32 + (b2 as f32 - b1 as f32) * local_t) as u8,
    )
}

fn render_figlet_clock(
    frame: &mut Frame,
    area: Rect,
    time_str: &str,
    style: FontStyle,
    colors: &[Color],
) {
    // Suppress ANSI escape codes from cfonts so we get plain text
    // that we can style with ratatui theme colors.
    unsafe { std::env::set_var("NO_COLOR", "1") };
    let output = cfonts::render(cfonts::Options {
        text: String::from(time_str),
        font: to_cfonts_font(style),
        spaceless: true,
        max_length: area.width,
        ..cfonts::Options::default()
    });
    unsafe { std::env::remove_var("NO_COLOR") };

    let raw_lines: Vec<&str> = output.vec.iter().map(|s| s.as_str()).collect();

    // Strip trailing empty lines
    let trimmed = raw_lines
        .iter()
        .rev()
        .skip_while(|l| l.trim().is_empty())
        .count();
    let visible = &raw_lines[..trimmed];

    if visible.is_empty() {
        let line = Line::styled(time_str, Style::default().fg(colors[0]));
        frame.render_widget(
            Paragraph::new(line).alignment(Alignment::Center),
            area,
        );
        return;
    }

    let max_lines = area.height as usize;
    let total = visible.len().min(max_lines);
    let lines: Vec<Line> = visible
        .iter()
        .take(max_lines)
        .enumerate()
        .map(|(i, l)| Line::styled(l.to_string(), Style::default().fg(lerp_color(colors, i, total))))
        .collect();

    let content_height = lines.len() as u16;
    let centered = ui::centered_rect(area, None, content_height);
    frame.render_widget(
        Paragraph::new(lines).alignment(Alignment::Center),
        centered,
    );
}
