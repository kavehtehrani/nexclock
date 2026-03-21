pub mod calendar;
pub mod clock;
pub mod grid;
pub mod status_bar;
pub mod system_stats;
pub mod weather;
pub mod world_clock;

use ratatui::{
    layout::{Alignment, Constraint, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Clear, Paragraph},
    Frame,
};

use crate::app::{parse_color, App, ComponentRuntime, FontStyle, ResolvedTheme, StyleProperty, UiMode};
use crate::component::{ClockStyle, ComponentConfig, ComponentStyle, ComponentType};
use crate::constants::{self, MIN_TERMINAL_HEIGHT, MIN_TERMINAL_WIDTH, STATUS_BAR_HEIGHT};
use crate::data::weather_api::WeatherData;

/// Returns a vertically centered (and optionally horizontally centered) sub-rect.
/// When `width` is `None`, the full container width is used (vertical-only centering).
/// When `Some(w)`, both axes are centered.
pub fn centered_rect(container: Rect, width: Option<u16>, height: u16) -> Rect {
    let y_offset = container.height.saturating_sub(height) / 2;
    let (x, w) = if let Some(content_width) = width {
        let x_offset = container.width.saturating_sub(content_width) / 2;
        (container.x + x_offset, content_width.min(container.width))
    } else {
        (container.x, container.width)
    };
    Rect {
        x,
        y: container.y + y_offset,
        width: w,
        height: container.height.saturating_sub(y_offset),
    }
}

/// Returns a bordered block with the given title, highlighted if focused.
/// In edit mode the border uses a distinct style to indicate the active editing state.
/// Per-component style overrides are applied for bg and border color.
pub fn panel_block<'a>(
    title: &str,
    is_focused: bool,
    is_editing: bool,
    theme: &ResolvedTheme,
    comp_style: &ComponentStyle,
) -> Block<'a> {
    let border_style = if is_editing {
        Style::default()
            .fg(theme.secondary)
            .add_modifier(Modifier::BOLD)
    } else if is_focused {
        Style::default().fg(theme.focus)
    } else if let Some(ref bc) = comp_style.border_color {
        Style::default().fg(parse_color(bc))
    } else {
        Style::default()
    };

    let mut block_style = Style::default();
    if let Some(ref bg) = comp_style.bg {
        block_style = block_style.bg(parse_color(bg));
    }

    if is_editing {
        Block::bordered()
            .title(format!(" {title} [EDIT] "))
            .border_style(border_style)
            .style(block_style)
    } else {
        Block::bordered()
            .title(format!(" {title} "))
            .border_style(border_style)
            .style(block_style)
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
    status_bar::render(frame, status_area, &ip, is_edit_mode, &app.theme);

    // Overlays
    match app.ui_mode {
        UiMode::Help => render_help(frame, area, &app.theme),
        UiMode::ContextMenu => render_context_menu(frame, app, area),
        UiMode::VisibilityMenu => render_visibility_menu(frame, app, area),
        UiMode::AddComponentMenu => render_add_menu(frame, app, area),
        UiMode::ColorMenu => render_color_menu(frame, app, area),
        UiMode::StyleMenu => render_style_menu(frame, app, area),
        UiMode::StyleColorPicker => render_style_color_picker(frame, app, area),
        UiMode::CalendarSelectMenu => render_cal_select_menu(frame, app, area),
        UiMode::CalendarRemoveMenu => render_cal_remove_menu(frame, app, area),
        UiMode::TimezoneSearch => render_tz_search(frame, app, area),
        UiMode::TimezoneRemoveMenu => render_tz_remove_menu(frame, app, area),
        UiMode::TimezoneReorderMenu => render_tz_reorder_menu(frame, app, area),
        UiMode::Normal | UiMode::EditMode => {}
    }
}

/// Dispatches rendering to the appropriate component renderer.
fn render_component(frame: &mut Frame, area: Rect, app: &App, idx: usize, is_focused: bool, is_editing: bool) {
    let entry = &app.components[idx];
    let theme = &app.theme;
    let comp_style = &entry.style;

    match &entry.config {
        ComponentConfig::Clock(settings) => {
            let (font_style, secondary_dates) =
                if let Some(ComponentRuntime::Clock { font_style, calendar_rx, .. }) =
                    app.runtime.get(&entry.id)
                {
                    let dates = calendar_rx
                        .as_ref()
                        .map(|rx| rx.borrow().clone())
                        .unwrap_or_default();
                    (*font_style, dates)
                } else {
                    (FontStyle::Block, Vec::new())
                };

            clock::render(
                frame,
                area,
                settings,
                app.tick_count,
                font_style,
                &secondary_dates,
                is_focused,
                is_editing,
                theme,
                comp_style,
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
            weather::render(frame, area, &data, is_focused, is_editing, theme, comp_style);
        }
        ComponentConfig::Calendar(_) => {
            calendar::render(frame, area, is_focused, is_editing, theme, comp_style);
        }
        ComponentConfig::SystemStats(_) => {
            let stats = if let Some(ComponentRuntime::SystemStats { stats_rx, .. }) =
                app.runtime.get(&entry.id)
            {
                stats_rx.borrow().clone()
            } else {
                crate::data::system::read_system_stats()
            };
            system_stats::render(frame, area, &stats, is_focused, is_editing, theme, comp_style);
        }
        ComponentConfig::WorldClock(settings) => {
            world_clock::render(frame, area, settings, is_focused, is_editing, theme, comp_style);
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

    render_popup(frame, area, " Help ", &help_lines, constants::HELP_POPUP_WIDTH);
}

fn render_context_menu(frame: &mut Frame, app: &App, area: Rect) {
    let cursor = app.menu_cursor;
    let items = &app.context_menu_items;
    let theme = &app.theme;

    let popup_width = constants::CONTEXT_MENU_WIDTH;
    let inner_w = (popup_width - 2) as usize;

    let mut lines: Vec<Line> = Vec::with_capacity(items.len());
    for (i, item) in items.iter().enumerate() {
        lines.push(styled_menu_line(&item.label, i, cursor, inner_w, theme));
    }

    let title = app
        .focused_component()
        .map(|c| c.config.component_type().label().to_string())
        .unwrap_or_else(|| "Menu".to_string());
    render_popup(frame, area, &title, &lines, popup_width);
}

fn render_visibility_menu(frame: &mut Frame, app: &App, area: Rect) {
    let cursor = app.menu_cursor;
    let theme = &app.theme;

    let popup_width = constants::VISIBILITY_MENU_WIDTH;
    let inner_w = (popup_width - 2) as usize;

    let mut lines: Vec<Line> = Vec::with_capacity(app.components.len());
    for (i, comp) in app.components.iter().enumerate() {
        let checked = if comp.visible { "x" } else { " " };
        let type_label = comp.config.component_type().label();
        let label = format!("[{checked}] {} ({type_label})", comp.id);
        lines.push(styled_menu_line(&label, i, cursor, inner_w, theme));
    }

    lines.push(Line::from(""));
    lines.push(Line::from(Span::styled(
        " Esc to close ",
        Style::default().fg(theme.muted),
    )));

    render_popup(frame, area, " Components ", &lines, popup_width);
}

fn render_add_menu(frame: &mut Frame, app: &App, area: Rect) {
    let cursor = app.menu_cursor;
    let theme = &app.theme;

    let popup_width = constants::ADD_MENU_WIDTH;
    let inner_w = (popup_width - 2) as usize;

    let options = add_menu_options();
    let mut lines: Vec<Line> = Vec::with_capacity(options.len());
    for (i, (label, _, _)) in options.iter().enumerate() {
        lines.push(styled_menu_line(label, i, cursor, inner_w, theme));
    }

    lines.push(Line::from(""));
    lines.push(Line::from(Span::styled(
        " Esc to cancel ",
        Style::default().fg(theme.muted),
    )));

    render_popup(frame, area, " Add Component ", &lines, popup_width);
}

/// Number of rows in the two-column color menu layout.
pub fn color_menu_rows() -> usize {
    constants::COLOR_PRESETS.len().div_ceil(2)
}

fn render_color_menu(frame: &mut Frame, app: &App, area: Rect) {
    let cursor = app.menu_cursor;
    let theme = &app.theme;
    let presets = constants::COLOR_PRESETS;
    let count = presets.len();
    let rows = color_menu_rows();

    let bar_width = constants::COLOR_BAR_WIDTH;
    // layout: " " + indicator + bar + "   " + indicator + bar + " "
    let inner_w = 1 + 1 + bar_width + 3 + 1 + bar_width + 1;
    let popup_width = (inner_w + 2) as u16;

    let in_right = cursor >= rows;
    let cursor_row = if in_right { cursor - rows } else { cursor };

    let indicator_style = Style::default()
        .fg(theme.focus)
        .add_modifier(Modifier::BOLD);

    let mut lines: Vec<Line> = Vec::with_capacity(rows + 2);
    for row in 0..rows {
        let left_idx = row;
        let right_idx = rows + row;

        let mut spans = Vec::new();

        // Left column
        let left_selected = !in_right && row == cursor_row;
        let arrow = constants::INDICATOR_ARROW;
        spans.push(Span::styled(
            if left_selected { format!(" {arrow}") } else { "  ".to_string() },
            indicator_style,
        ));
        append_gradient_bar(&mut spans, presets[left_idx].1, bar_width, theme);

        // Gap between columns
        spans.push(Span::raw("   "));

        // Right column
        if right_idx < count {
            let right_selected = in_right && row == cursor_row;
            spans.push(Span::styled(
                if right_selected { arrow.to_string() } else { " ".to_string() },
                indicator_style,
            ));
            append_gradient_bar(&mut spans, presets[right_idx].1, bar_width, theme);
            spans.push(Span::raw(" "));
        } else {
            spans.push(Span::raw(" ".repeat(1 + bar_width + 1)));
        }

        lines.push(Line::from(spans));
    }

    lines.push(Line::from(""));
    lines.push(Line::from(Span::styled(
        " Esc to cancel ",
        Style::default().fg(theme.muted),
    )));

    render_popup(frame, area, " Colors ", &lines, popup_width);
}

/// Number of rows in the two-column style color picker layout.
pub fn style_color_picker_rows() -> usize {
    constants::STYLE_COLOR_PRESETS.len().div_ceil(2)
}

fn render_style_menu(frame: &mut Frame, app: &App, area: Rect) {
    let cursor = app.menu_cursor;
    let theme = &app.theme;
    let popup_width = constants::STYLE_MENU_WIDTH;
    let inner_w = (popup_width - 2) as usize;

    let options = ["Text Color", "Background", "Border Color", "Reset All"];
    let mut lines: Vec<Line> = Vec::with_capacity(options.len());
    for (i, label) in options.iter().enumerate() {
        lines.push(styled_menu_line(label, i, cursor, inner_w, theme));
    }

    lines.push(Line::from(""));
    lines.push(Line::from(Span::styled(
        " Esc to cancel ",
        Style::default().fg(theme.muted),
    )));

    render_popup(frame, area, "Style", &lines, popup_width);
}

fn render_style_color_picker(frame: &mut Frame, app: &App, area: Rect) {
    let cursor = app.menu_cursor;
    let theme = &app.theme;
    let presets = constants::STYLE_COLOR_PRESETS;
    let count = presets.len();
    let rows = style_color_picker_rows();

    let label_width: usize = 14;
    // layout: " " + indicator(2) + swatch(2) + " " + label + "   " + indicator(1) + swatch(2) + " " + label + " "
    let inner_w = 2 + 2 + 1 + label_width + 3 + 1 + 2 + 1 + label_width + 1;
    let popup_width = (inner_w + 2) as u16;

    let in_right = cursor >= rows;
    let cursor_row = if in_right { cursor - rows } else { cursor };

    let indicator_style = Style::default()
        .fg(theme.focus)
        .add_modifier(Modifier::BOLD);

    let title = match app.style_target {
        StyleProperty::Fg => "Text Color",
        StyleProperty::Bg => "Background",
        StyleProperty::BorderColor => "Border Color",
    };

    let mut lines: Vec<Line> = Vec::with_capacity(rows + 2);
    for row in 0..rows {
        let left_idx = row;
        let right_idx = rows + row;

        let mut spans = Vec::new();

        // Left column
        let left_selected = !in_right && row == cursor_row;
        let arrow = constants::INDICATOR_ARROW;
        spans.push(Span::styled(
            if left_selected { format!(" {arrow}") } else { "  ".to_string() },
            indicator_style,
        ));
        append_color_swatch(&mut spans, presets[left_idx].1, theme);
        spans.push(Span::raw(" "));
        spans.push(Span::styled(
            format!("{:<w$}", presets[left_idx].0, w = label_width),
            if left_selected {
                Style::default().fg(theme.focus).add_modifier(Modifier::BOLD)
            } else {
                Style::default().fg(theme.text)
            },
        ));

        // Gap between columns
        spans.push(Span::raw("   "));

        // Right column
        if right_idx < count {
            let right_selected = in_right && row == cursor_row;
            spans.push(Span::styled(
                if right_selected { arrow.to_string() } else { " ".to_string() },
                indicator_style,
            ));
            append_color_swatch(&mut spans, presets[right_idx].1, theme);
            spans.push(Span::raw(" "));
            spans.push(Span::styled(
                format!("{:<w$}", presets[right_idx].0, w = label_width),
                if right_selected {
                    Style::default().fg(theme.focus).add_modifier(Modifier::BOLD)
                } else {
                    Style::default().fg(theme.text)
                },
            ));
            spans.push(Span::raw(" "));
        }

        lines.push(Line::from(spans));
    }

    lines.push(Line::from(""));
    lines.push(Line::from(Span::styled(
        " Esc to cancel ",
        Style::default().fg(theme.muted),
    )));

    render_popup(frame, area, title, &lines, popup_width);
}

/// Appends a 2-char color swatch block for a single color string.
fn append_color_swatch(spans: &mut Vec<Span<'static>>, color_str: &str, theme: &ResolvedTheme) {
    let color = if color_str.is_empty() {
        theme.text
    } else {
        parse_color(color_str)
    };
    let block = constants::GRADIENT_BLOCK;
    spans.push(Span::styled(
        format!("{block}{block}"),
        Style::default().fg(color),
    ));
}

fn append_gradient_bar(spans: &mut Vec<Span<'static>>, colors: &[&str], width: usize, theme: &ResolvedTheme) {
    let resolved: Vec<Color> = if colors.is_empty() {
        vec![theme.primary]
    } else {
        colors.iter().map(|&c| parse_color(c)).collect()
    };

    for i in 0..width {
        let color = clock::lerp_color(&resolved, i, width);
        spans.push(Span::styled(constants::GRADIENT_BLOCK, Style::default().fg(color)));
    }
}

/// Returns the list of add-menu options: (label, ComponentType, Option<ClockStyle>).
pub fn add_menu_options() -> Vec<(&'static str, ComponentType, Option<ClockStyle>)> {
    vec![
        ("Clock (Large)", ComponentType::Clock, Some(ClockStyle::Large)),
        ("Clock (Compact)", ComponentType::Clock, Some(ClockStyle::Compact)),
        ("World Clock", ComponentType::WorldClock, None),
        ("Weather", ComponentType::Weather, None),
        ("Calendar", ComponentType::Calendar, None),
        ("System Stats", ComponentType::SystemStats, None),
    ]
}

fn styled_menu_line(
    label: &str,
    index: usize,
    cursor: usize,
    width: usize,
    theme: &ResolvedTheme,
) -> Line<'static> {
    let style = if index == cursor {
        Style::default()
            .fg(Color::Black)
            .bg(theme.primary)
            .add_modifier(Modifier::BOLD)
    } else {
        Style::default().fg(theme.text)
    };
    Line::styled(format!(" {label:<w$}", w = width - 1), style)
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

fn render_cal_select_menu(frame: &mut Frame, app: &App, area: Rect) {
    let cursor = app.cal_select_cursor;
    let theme = &app.theme;
    let popup_width = constants::CALENDAR_SELECT_MENU_WIDTH;
    let inner_w = (popup_width - 2) as usize;

    let items = &app.cal_select_items;
    let mut lines: Vec<Line> = Vec::with_capacity(items.len());

    if items.is_empty() {
        lines.push(Line::styled(
            " All calendars already added",
            Style::default().fg(theme.muted),
        ));
    } else {
        for (i, &(_, label)) in items.iter().enumerate() {
            lines.push(styled_menu_line(label, i, cursor, inner_w, theme));
        }
    }

    lines.push(Line::from(""));
    lines.push(Line::from(Span::styled(
        " Esc to cancel ",
        Style::default().fg(theme.muted),
    )));

    render_popup(frame, area, "Add Calendar", &lines, popup_width);
}

fn render_cal_remove_menu(frame: &mut Frame, app: &App, area: Rect) {
    let cursor = app.menu_cursor;
    let theme = &app.theme;
    let popup_width = constants::CALENDAR_REMOVE_MENU_WIDTH;
    let inner_w = (popup_width - 2) as usize;

    let calendars = app.focused_clock_calendars();

    let mut lines: Vec<Line> = Vec::new();

    if calendars.is_empty() {
        lines.push(Line::styled(
            " No calendars configured",
            Style::default().fg(theme.muted),
        ));
    } else {
        for (i, entry) in calendars.iter().enumerate() {
            let name = constants::CALENDAR_SYSTEMS
                .iter()
                .find(|(id, _)| *id == entry.calendar_id)
                .map(|(_, name)| *name)
                .unwrap_or(&entry.calendar_id);
            let native_tag = if entry.use_native { " [native]" } else { "" };
            let label = format!("{name}{native_tag}");
            lines.push(styled_menu_line(&label, i, cursor, inner_w, theme));
        }
    }

    lines.push(Line::from(""));
    lines.push(Line::from(Span::styled(
        " n: native  d: remove  Esc: close ",
        Style::default().fg(theme.muted),
    )));

    render_popup(frame, area, "Calendars", &lines, popup_width);
}

fn render_tz_search(frame: &mut Frame, app: &App, area: Rect) {
    let theme = &app.theme;
    let popup_width = constants::TZ_SEARCH_WIDTH;
    let inner_w = (popup_width - 2) as usize;

    let mut lines: Vec<Line> = Vec::new();

    // Search input line
    let query_display = format!(" > {}_", app.tz_search_query);
    lines.push(Line::styled(
        query_display,
        Style::default().fg(theme.text).add_modifier(Modifier::BOLD),
    ));
    lines.push(Line::from(""));

    // Filtered results
    for (i, tz_name) in app.tz_search_results.iter().enumerate() {
        lines.push(styled_menu_line(tz_name, i, app.tz_search_cursor, inner_w, theme));
    }

    if app.tz_search_results.is_empty() && !app.tz_search_query.is_empty() {
        lines.push(Line::styled(
            " No matches",
            Style::default().fg(theme.muted),
        ));
    }

    lines.push(Line::from(""));
    lines.push(Line::from(Span::styled(
        " Esc to cancel ",
        Style::default().fg(theme.muted),
    )));

    render_popup(frame, area, "Add Timezone", &lines, popup_width);
}

fn render_tz_remove_menu(frame: &mut Frame, app: &App, area: Rect) {
    let theme = &app.theme;
    let popup_width = constants::TZ_REMOVE_MENU_WIDTH;
    let inner_w = (popup_width - 2) as usize;

    let timezones = app.focused_world_clock_timezones();

    let mut lines: Vec<Line> = Vec::new();

    if timezones.is_empty() {
        lines.push(Line::styled(
            " No timezones to remove",
            Style::default().fg(theme.muted),
        ));
    } else {
        for (i, entry) in timezones.iter().enumerate() {
            let label = entry.label.as_deref().unwrap_or(&entry.timezone);
            lines.push(styled_menu_line(label, i, app.menu_cursor, inner_w, theme));
        }
    }

    lines.push(Line::from(""));
    lines.push(Line::from(Span::styled(
        " Esc to cancel ",
        Style::default().fg(theme.muted),
    )));

    render_popup(frame, area, "Remove Timezone", &lines, popup_width);
}

fn render_tz_reorder_menu(frame: &mut Frame, app: &App, area: Rect) {
    let theme = &app.theme;
    let popup_width = constants::TZ_REORDER_MENU_WIDTH;
    let inner_w = (popup_width - 2) as usize;

    let timezones = app.focused_world_clock_timezones();

    let mut lines: Vec<Line> = Vec::new();

    if timezones.is_empty() {
        lines.push(Line::styled(
            " No timezones to reorder",
            Style::default().fg(theme.muted),
        ));
    } else {
        for (i, entry) in timezones.iter().enumerate() {
            let label = entry.label.as_deref().unwrap_or(&entry.timezone);
            lines.push(styled_menu_line(label, i, app.menu_cursor, inner_w, theme));
        }
    }

    lines.push(Line::from(""));
    lines.push(Line::from(Span::styled(
        " Shift+\u{2191}/\u{2193} to move, Esc to close ",
        Style::default().fg(theme.muted),
    )));

    render_popup(frame, area, "Reorder Timezones", &lines, popup_width);
}

fn shortcut_line<'a>(key: &'a str, desc: &'a str, theme: &ResolvedTheme) -> Line<'a> {
    let w = constants::SHORTCUT_KEY_WIDTH;
    Line::from(vec![
        Span::styled(
            format!("{key:>w$}"),
            Style::default()
                .fg(theme.primary)
                .add_modifier(Modifier::BOLD),
        ),
        Span::raw("  "),
        Span::styled(desc, Style::default().fg(theme.text)),
    ])
}
