pub mod calendar;
pub mod clock;
pub mod secondary_clock;
pub mod status_bar;
pub mod system_stats;
pub mod weather;

use ratatui::{
    layout::{Alignment, Constraint, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Clear, Paragraph},
    Frame,
};

use crate::app::{App, PanelId, UiMode};
use crate::config::Slot;
use crate::constants::{self, MIN_TERMINAL_HEIGHT, MIN_TERMINAL_WIDTH, STATUS_BAR_HEIGHT};

/// Returns a bordered block with the given title, highlighted if focused.
pub fn panel_block(title: &str, is_focused: bool) -> Block<'_> {
    let border_style = if is_focused {
        Style::default().fg(constants::FOCUS_BORDER_COLOR)
    } else {
        Style::default()
    };
    Block::bordered()
        .title(format!(" {title} "))
        .border_style(border_style)
}

/// Root draw function: composes the full UI layout.
pub fn draw(frame: &mut Frame, app: &mut App) {
    let area = frame.area();

    // Guard: if terminal is too small, show a message instead of panicking
    if area.width < MIN_TERMINAL_WIDTH || area.height < MIN_TERMINAL_HEIGHT {
        let msg = Line::styled(
            "Terminal too small",
            Style::default().fg(Color::Red),
        );
        frame.render_widget(
            Paragraph::new(msg).alignment(Alignment::Center),
            area,
        );
        return;
    }

    let layout = &app.config.layout;

    // Main vertical split: top row | bottom grid | status bar
    let rows = Layout::vertical([
        Constraint::Percentage(layout.top_height_percent),
        Constraint::Percentage(layout.bottom_height_percent),
        Constraint::Length(STATUS_BAR_HEIGHT),
    ])
    .split(area);

    // Bottom grid: left and right columns
    let columns = Layout::horizontal([
        Constraint::Percentage(layout.left_column_percent),
        Constraint::Percentage(100 - layout.left_column_percent),
    ])
    .split(rows[1]);

    // Left column split
    let left_panels = Layout::vertical([
        Constraint::Percentage(layout.left_split_percent),
        Constraint::Percentage(100 - layout.left_split_percent),
    ])
    .split(columns[0]);

    // Right column split
    let right_panels = Layout::vertical([
        Constraint::Percentage(layout.right_split_percent),
        Constraint::Percentage(100 - layout.right_split_percent),
    ])
    .split(columns[1]);

    // Map slots to computed areas
    let slot_areas: [(Slot, Rect); 5] = [
        (Slot::Top, rows[0]),
        (Slot::LeftTop, left_panels[0]),
        (Slot::LeftBottom, left_panels[1]),
        (Slot::RightTop, right_panels[0]),
        (Slot::RightBottom, right_panels[1]),
    ];

    // Render each slot and track where the Clock panel ends up
    for &(slot, area) in &slot_areas {
        let panel = render_panel(frame, area, app, slot);
        if panel == PanelId::Clock {
            app.clock_area = area;
        }
    }

    // Status bar
    status_bar::render(frame, rows[2], &app.external_ip(), app.font_style);

    // Overlays (drawn last, on top of everything)
    match app.ui_mode {
        UiMode::Help => render_help(frame, area),
        UiMode::ContextMenu => render_context_menu(frame, app, area),
        UiMode::VisibilityMenu => render_visibility_menu(frame, app, area),
        UiMode::Normal => {}
    }
}

/// Renders whichever panel is assigned to the given slot. Returns the PanelId rendered.
fn render_panel(frame: &mut Frame, area: Rect, app: &App, slot: Slot) -> PanelId {
    let panel = PanelId::from_name(app.config.layout.panel_at(slot));
    let focused = app.focused_panel == panel;

    if !app.is_panel_visible(panel) {
        return panel;
    }

    match panel {
        PanelId::Clock => {
            clock::render(
                frame,
                area,
                &app.config.clock,
                app.colon_visible(),
                app.font_style,
                focused,
            );
        }
        PanelId::SecondaryClock => {
            secondary_clock::render(frame, area, &app.config.secondary_clock, focused);
        }
        PanelId::Weather => {
            weather::render(frame, area, &app.weather(), focused);
        }
        PanelId::Calendar => {
            calendar::render(frame, area, focused);
        }
        PanelId::SystemStats => {
            system_stats::render(frame, area, &app.system_stats(), focused);
        }
    }

    panel
}

/// Renders a centered help overlay with keyboard shortcuts.
fn render_help(frame: &mut Frame, area: Rect) {
    let help_lines = vec![
        Line::from(Span::styled(
            " Keyboard Shortcuts ",
            Style::default()
                .fg(Color::Yellow)
                .add_modifier(Modifier::BOLD),
        )),
        Line::from(""),
        shortcut_line("Tab", "Next panel"),
        shortcut_line("Shift+Tab", "Previous panel"),
        shortcut_line("Space", "Panel context menu"),
        shortcut_line("v", "Visibility menu"),
        shortcut_line("h / ?", "Toggle this help"),
        shortcut_line("q / Esc", "Quit"),
        shortcut_line("t", "Toggle 12h / 24h"),
        shortcut_line("f / Right", "Next font style"),
        shortcut_line("F / Left", "Previous font style"),
        Line::from(""),
        Line::from(Span::styled(
            " Mouse ",
            Style::default()
                .fg(Color::Yellow)
                .add_modifier(Modifier::BOLD),
        )),
        Line::from(""),
        shortcut_line("Click clock", "Toggle 12h / 24h"),
        Line::from(""),
        Line::from(Span::styled(
            " Press any key to close ",
            Style::default().fg(Color::DarkGray),
        )),
    ];

    render_popup(frame, area, " Help ", &help_lines, 40);
}

/// Renders the context menu popup anchored near the center.
fn render_context_menu(frame: &mut Frame, app: &App, area: Rect) {
    let cursor = app.menu_cursor;
    let items = &app.context_menu_items;

    let mut lines: Vec<Line> = Vec::with_capacity(items.len());
    for (i, item) in items.iter().enumerate() {
        let style = if i == cursor {
            Style::default()
                .fg(Color::Black)
                .bg(Color::Cyan)
                .add_modifier(Modifier::BOLD)
        } else {
            Style::default().fg(Color::White)
        };
        lines.push(Line::styled(format!(" {} ", item.label), style));
    }

    let title = format!("{} ", app.focused_panel.label());
    render_popup(frame, area, &title, &lines, 35);
}

/// Renders the visibility menu as a centered overlay.
fn render_visibility_menu(frame: &mut Frame, app: &App, area: Rect) {
    let cursor = app.menu_cursor;

    let mut lines: Vec<Line> = Vec::with_capacity(PanelId::ALL.len());
    for (i, &panel) in PanelId::ALL.iter().enumerate() {
        let checked = if app.is_panel_visible(panel) {
            "x"
        } else {
            " "
        };
        let label = format!(" [{checked}] {} ", panel.label());
        let style = if i == cursor {
            Style::default()
                .fg(Color::Black)
                .bg(Color::Cyan)
                .add_modifier(Modifier::BOLD)
        } else {
            Style::default().fg(Color::White)
        };
        lines.push(Line::styled(label, style));
    }

    lines.push(Line::from(""));
    lines.push(Line::from(Span::styled(
        " Esc to close ",
        Style::default().fg(Color::DarkGray),
    )));

    render_popup(frame, area, " Panels ", &lines, 30);
}

/// Renders a centered popup with a title, content lines, and a given width.
fn render_popup(frame: &mut Frame, area: Rect, title: &str, lines: &[Line], width: u16) {
    let popup_width = width.min(area.width);
    let popup_height = (lines.len() as u16 + 2).min(area.height); // +2 for border

    let x = area.x + area.width.saturating_sub(popup_width) / 2;
    let y = area.y + area.height.saturating_sub(popup_height) / 2;

    let popup_area = Rect {
        x,
        y,
        width: popup_width,
        height: popup_height,
    };

    let block = Block::bordered()
        .title(format!(" {title} "))
        .style(Style::default().bg(Color::Black));

    frame.render_widget(Clear, popup_area);
    frame.render_widget(
        Paragraph::new(lines.to_vec())
            .block(block)
            .alignment(Alignment::Center),
        popup_area,
    );
}

fn shortcut_line<'a>(key: &'a str, desc: &'a str) -> Line<'a> {
    Line::from(vec![
        Span::styled(
            format!("{key:>14}"),
            Style::default()
                .fg(Color::Cyan)
                .add_modifier(Modifier::BOLD),
        ),
        Span::raw("  "),
        Span::styled(desc, Style::default().fg(Color::White)),
    ])
}
