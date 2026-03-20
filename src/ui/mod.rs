pub mod calendar;
pub mod clock;
pub mod grid;
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

use crate::app::{App, ComponentRuntime, FontStyle, ResolvedTheme, UiMode};
use crate::component::{ClockStyle, ComponentConfig, ComponentType};
use crate::constants::{MIN_TERMINAL_HEIGHT, MIN_TERMINAL_WIDTH, STATUS_BAR_HEIGHT};
use crate::data::weather_api::WeatherData;

/// Returns a bordered block with the given title, highlighted if focused.
/// In edit mode the border uses a distinct style to indicate the active editing state.
pub fn panel_block<'a>(
    title: &str,
    is_focused: bool,
    is_editing: bool,
    theme: &ResolvedTheme,
) -> Block<'a> {
    let border_style = if is_editing {
        Style::default()
            .fg(theme.secondary)
            .add_modifier(Modifier::BOLD)
    } else if is_focused {
        Style::default().fg(theme.focus)
    } else {
        Style::default()
    };

    if is_editing {
        Block::bordered()
            .title(format!(" {title} [EDIT] "))
            .border_style(border_style)
    } else {
        Block::bordered()
            .title(format!(" {title} "))
            .border_style(border_style)
    }
}

/// Root draw function: composes the full UI layout.
pub fn draw(frame: &mut Frame, app: &mut App) {
    let area = frame.area();

    if area.width < MIN_TERMINAL_WIDTH || area.height < MIN_TERMINAL_HEIGHT {
        let msg = Line::styled(
            "Terminal too small",
            Style::default().fg(app.theme.error),
        );
        frame.render_widget(
            Paragraph::new(msg).alignment(Alignment::Center),
            area,
        );
        return;
    }

    // Split into content area + status bar
    let rows = Layout::vertical([
        Constraint::Min(1),
        Constraint::Length(STATUS_BAR_HEIGHT),
    ])
    .split(area);

    let content_area = rows[0];
    let status_area = rows[1];

    // Compute grid cells
    let cells = grid::compute_grid(content_area, &app.config.grid);

    // Get visible component indices and which one is focused
    let visible = app.visible_components();
    let focused_comp_idx = visible.get(app.focused_index).copied();

    let is_edit_mode = app.ui_mode == UiMode::EditMode;

    // Render each visible component
    for &ci in &visible {
        let entry = &app.components[ci];
        let is_focused = Some(ci) == focused_comp_idx;
        let is_editing = is_focused && is_edit_mode;

        let Some(cell_rect) = grid::merged_rect(&cells, &entry.placement) else {
            continue;
        };

        // Store the rendered area in runtime
        if let Some(rt) = app.runtime.get_mut(&entry.id) {
            rt.set_area(cell_rect);
        }

        render_component(frame, cell_rect, app, ci, is_focused, is_editing);
    }

    // Status bar
    let ip = app.external_ip();
    let font_name = app.active_font_name();
    status_bar::render(frame, status_area, &ip, font_name, is_edit_mode, &app.theme);

    // Overlays
    match app.ui_mode {
        UiMode::Help => render_help(frame, area, &app.theme),
        UiMode::ContextMenu => render_context_menu(frame, app, area),
        UiMode::VisibilityMenu => render_visibility_menu(frame, app, area),
        UiMode::AddComponentMenu => render_add_menu(frame, app, area),
        UiMode::Normal | UiMode::EditMode => {}
    }
}

/// Dispatches rendering to the appropriate component renderer.
fn render_component(frame: &mut Frame, area: Rect, app: &App, idx: usize, is_focused: bool, is_editing: bool) {
    let entry = &app.components[idx];
    let theme = &app.theme;

    match &entry.config {
        ComponentConfig::Clock(settings) => {
            let font_style = if let Some(ComponentRuntime::Clock { font_style, .. }) =
                app.runtime.get(&entry.id)
            {
                *font_style
            } else {
                FontStyle::Block
            };

            clock::render(
                frame,
                area,
                settings,
                app.tick_count,
                font_style,
                is_focused,
                is_editing,
                theme,
            );
        }
        ComponentConfig::Weather(_) => {
            let data: Option<WeatherData> =
                if let Some(ComponentRuntime::Weather { data_rx, .. }) =
                    app.runtime.get(&entry.id)
                {
                    data_rx.borrow().clone()
                } else {
                    None
                };
            weather::render(frame, area, &data, is_focused, is_editing, theme);
        }
        ComponentConfig::Calendar(_) => {
            calendar::render(frame, area, is_focused, is_editing, theme);
        }
        ComponentConfig::SystemStats(_) => {
            let stats = if let Some(ComponentRuntime::SystemStats { stats_rx, .. }) =
                app.runtime.get(&entry.id)
            {
                stats_rx.borrow().clone()
            } else {
                crate::data::system::read_system_stats()
            };
            system_stats::render(frame, area, &stats, is_focused, is_editing, theme);
        }
    }
}

// ── Overlays ────────────────────────────────────────────────────────

fn render_help(frame: &mut Frame, area: Rect, theme: &ResolvedTheme) {
    let help_lines = vec![
        Line::from(Span::styled(
            " Keyboard Shortcuts ",
            Style::default()
                .fg(theme.secondary)
                .add_modifier(Modifier::BOLD),
        )),
        Line::from(""),
        shortcut_line("Tab", "Next panel", theme),
        shortcut_line("Shift+Tab", "Previous panel", theme),
        shortcut_line("Arrow keys", "Spatial navigation", theme),
        shortcut_line("Space", "Panel context menu", theme),
        shortcut_line("e", "Edit mode (resize/move)", theme),
        shortcut_line("a", "Add component", theme),
        shortcut_line("v", "Visibility menu", theme),
        shortcut_line("h / ?", "Toggle this help", theme),
        shortcut_line("q / Esc", "Quit", theme),
        shortcut_line("t", "Toggle 12h / 24h", theme),
        shortcut_line("f", "Next font style", theme),
        shortcut_line("F", "Previous font style", theme),
        Line::from(""),
        Line::from(Span::styled(
            " Edit Mode ",
            Style::default()
                .fg(theme.secondary)
                .add_modifier(Modifier::BOLD),
        )),
        shortcut_line("Arrows", "Resize panel", theme),
        shortcut_line("Shift+Arrows", "Move panel", theme),
        shortcut_line("Esc / e", "Exit edit mode", theme),
        Line::from(""),
        Line::from(Span::styled(
            " Press any key to close ",
            Style::default().fg(theme.muted),
        )),
    ];

    render_popup(frame, area, " Help ", &help_lines, 42);
}

fn render_context_menu(frame: &mut Frame, app: &App, area: Rect) {
    let cursor = app.menu_cursor;
    let items = &app.context_menu_items;
    let theme = &app.theme;

    let mut lines: Vec<Line> = Vec::with_capacity(items.len());
    for (i, item) in items.iter().enumerate() {
        let style = if i == cursor {
            Style::default()
                .fg(Color::Black)
                .bg(theme.primary)
                .add_modifier(Modifier::BOLD)
        } else {
            Style::default().fg(theme.text)
        };
        lines.push(Line::styled(format!(" {} ", item.label), style));
    }

    let title = app
        .focused_component()
        .map(|c| c.config.component_type().label().to_string())
        .unwrap_or_else(|| "Menu".to_string());
    render_popup(frame, area, &title, &lines, 35);
}

fn render_visibility_menu(frame: &mut Frame, app: &App, area: Rect) {
    let cursor = app.menu_cursor;
    let theme = &app.theme;

    let mut lines: Vec<Line> = Vec::with_capacity(app.components.len());
    for (i, comp) in app.components.iter().enumerate() {
        let checked = if comp.visible { "x" } else { " " };
        let type_label = comp.config.component_type().label();
        let label = format!(" [{checked}] {} ({}) ", comp.id, type_label);
        let style = if i == cursor {
            Style::default()
                .fg(Color::Black)
                .bg(theme.primary)
                .add_modifier(Modifier::BOLD)
        } else {
            Style::default().fg(theme.text)
        };
        lines.push(Line::styled(label, style));
    }

    lines.push(Line::from(""));
    lines.push(Line::from(Span::styled(
        " Esc to close ",
        Style::default().fg(theme.muted),
    )));

    render_popup(frame, area, " Components ", &lines, 40);
}

fn render_add_menu(frame: &mut Frame, app: &App, area: Rect) {
    let cursor = app.menu_cursor;
    let theme = &app.theme;

    let options = add_menu_options();
    let mut lines: Vec<Line> = Vec::with_capacity(options.len());
    for (i, (label, _, _)) in options.iter().enumerate() {
        let style = if i == cursor {
            Style::default()
                .fg(Color::Black)
                .bg(theme.primary)
                .add_modifier(Modifier::BOLD)
        } else {
            Style::default().fg(theme.text)
        };
        lines.push(Line::styled(format!(" {label} "), style));
    }

    lines.push(Line::from(""));
    lines.push(Line::from(Span::styled(
        " Esc to cancel ",
        Style::default().fg(theme.muted),
    )));

    render_popup(frame, area, " Add Component ", &lines, 30);
}

/// Returns the list of add-menu options: (label, ComponentType, Option<ClockStyle>).
pub fn add_menu_options() -> Vec<(&'static str, ComponentType, Option<ClockStyle>)> {
    vec![
        ("Clock (Large)", ComponentType::Clock, Some(ClockStyle::Large)),
        ("Clock (Compact)", ComponentType::Clock, Some(ClockStyle::Compact)),
        ("Weather", ComponentType::Weather, None),
        ("Calendar", ComponentType::Calendar, None),
        ("System Stats", ComponentType::SystemStats, None),
    ]
}

fn render_popup(frame: &mut Frame, area: Rect, title: &str, lines: &[Line], width: u16) {
    let popup_width = width.min(area.width);
    let popup_height = (lines.len() as u16 + 2).min(area.height);

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

fn shortcut_line<'a>(key: &'a str, desc: &'a str, theme: &ResolvedTheme) -> Line<'a> {
    Line::from(vec![
        Span::styled(
            format!("{key:>14}"),
            Style::default()
                .fg(theme.primary)
                .add_modifier(Modifier::BOLD),
        ),
        Span::raw("  "),
        Span::styled(desc, Style::default().fg(theme.text)),
    ])
}
