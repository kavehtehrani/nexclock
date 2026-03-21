use crossterm::event::{
    self, Event as CrosstermEvent, KeyCode, KeyEvent, KeyEventKind, KeyModifiers, MouseButton,
    MouseEventKind,
};
use std::time::Duration;

use crate::app::{App, StyleProperty, UiMode};
use crate::ui;

pub fn handle_events(app: &mut App, tick_rate: Duration) -> std::io::Result<()> {
    if event::poll(tick_rate)? {
        match event::read()? {
            CrosstermEvent::Key(key) if key.kind == KeyEventKind::Press => {
                handle_key(app, key);
            }
            CrosstermEvent::Mouse(mouse) => {
                if mouse.kind == MouseEventKind::Down(MouseButton::Left) {
                    handle_mouse_click(app, mouse.column, mouse.row);
                }
            }
            _ => {}
        }
    }
    app.tick();
    Ok(())
}

fn handle_key(app: &mut App, key: KeyEvent) {
    match app.ui_mode {
        UiMode::Normal => handle_normal_key(app, key),
        UiMode::EditMode => handle_edit_mode_key(app, key),
        UiMode::ContextMenu => handle_context_menu_key(app, key.code),
        UiMode::VisibilityMenu => handle_visibility_menu_key(app, key.code),
        UiMode::AddComponentMenu => handle_add_menu_key(app, key.code),
        UiMode::ColorMenu => handle_color_menu_key(app, key.code),
        UiMode::StyleMenu => handle_style_menu_key(app, key.code),
        UiMode::StyleColorPicker => handle_style_color_picker_key(app, key.code),
        UiMode::CalendarSelectMenu => handle_cal_select_key(app, key.code),
        UiMode::CalendarRemoveMenu => handle_cal_remove_key(app, key.code),
        UiMode::TimezoneSearch => handle_tz_search_key(app, key.code),
        UiMode::TimezoneRemoveMenu => handle_tz_remove_menu_key(app, key.code),
        UiMode::TimezoneReorderMenu => handle_tz_reorder_key(app, key),
        UiMode::Help => {
            app.ui_mode = UiMode::Normal;
        }
    }
}

fn handle_normal_key(app: &mut App, key: KeyEvent) {
    match key.code {
        KeyCode::Char('q') | KeyCode::Char('Q') => app.quit(),
        KeyCode::Esc => app.quit(),
        KeyCode::Char('h') | KeyCode::Char('H') | KeyCode::Char('?') => {
            app.ui_mode = UiMode::Help;
        }
        KeyCode::Char('f') => app.cycle_font_next(),
        KeyCode::Char('F') => app.cycle_font_prev(),
        KeyCode::Char('t') | KeyCode::Char('T') => app.toggle_time_format(),
        KeyCode::Tab => app.focus_next(),
        KeyCode::BackTab => app.focus_prev(),
        KeyCode::Up => app.focus_direction(-1, 0),
        KeyCode::Down => app.focus_direction(1, 0),
        KeyCode::Left => app.focus_direction(0, -1),
        KeyCode::Right => app.focus_direction(0, 1),
        KeyCode::Char(' ') | KeyCode::Enter => app.open_context_menu(),
        KeyCode::Char('e') | KeyCode::Char('E') => {
            if app.focused_component().is_some() {
                app.ui_mode = UiMode::EditMode;
            }
        }
        KeyCode::Char('v') | KeyCode::Char('V') => {
            app.menu_cursor = 0;
            app.ui_mode = UiMode::VisibilityMenu;
        }
        KeyCode::Char('a') | KeyCode::Char('A') => {
            app.menu_cursor = 0;
            app.ui_mode = UiMode::AddComponentMenu;
        }
        _ => {}
    }
}

/// In edit mode: arrows resize, Shift+arrows move, Esc exits.
fn handle_edit_mode_key(app: &mut App, key: KeyEvent) {
    let Some(idx) = app.focused_component_idx() else {
        app.ui_mode = UiMode::Normal;
        return;
    };

    let shifted = key.modifiers.contains(KeyModifiers::SHIFT);

    match key.code {
        KeyCode::Esc | KeyCode::Char('e') | KeyCode::Char('E') => {
            app.ui_mode = UiMode::Normal;
        }
        KeyCode::Up => {
            if shifted {
                app.move_component(idx, -1, 0);
            } else {
                let p = &app.components[idx].placement;
                // Skip if component spans all rows (resize would be invisible)
                if p.row_span < app.config.grid.rows {
                    app.adjust_row_height(p.row, false);
                }
            }
        }
        KeyCode::Down => {
            if shifted {
                app.move_component(idx, 1, 0);
            } else {
                let p = &app.components[idx].placement;
                if p.row_span < app.config.grid.rows {
                    app.adjust_row_height(p.row, true);
                }
            }
        }
        KeyCode::Left => {
            if shifted {
                app.move_component(idx, 0, -1);
            } else {
                let p = &app.components[idx].placement;
                // Skip if component spans all columns (resize would be invisible)
                if p.col_span < app.config.grid.columns {
                    app.adjust_col_width(p.column, false);
                }
            }
        }
        KeyCode::Right => {
            if shifted {
                app.move_component(idx, 0, 1);
            } else {
                let p = &app.components[idx].placement;
                if p.col_span < app.config.grid.columns {
                    app.adjust_col_width(p.column, true);
                }
            }
        }
        KeyCode::Char('q') | KeyCode::Char('Q') => {
            app.ui_mode = UiMode::Normal;
        }
        _ => {}
    }
}

fn handle_context_menu_key(app: &mut App, code: KeyCode) {
    match code {
        KeyCode::Up => {
            if app.menu_cursor > 0 {
                app.menu_cursor -= 1;
            }
        }
        KeyCode::Down => {
            let max = app.context_menu_items.len().saturating_sub(1);
            if app.menu_cursor < max {
                app.menu_cursor += 1;
            }
        }
        KeyCode::Enter => {
            if let Some(item) = app.context_menu_items.get(app.menu_cursor) {
                let action = item.action.clone();
                app.execute_menu_action(action);
            }
        }
        KeyCode::Esc | KeyCode::Char(' ') => {
            app.ui_mode = UiMode::Normal;
        }
        _ => {}
    }
}

fn handle_visibility_menu_key(app: &mut App, code: KeyCode) {
    let count = app.components.len();
    match code {
        KeyCode::Up => {
            if app.menu_cursor > 0 {
                app.menu_cursor -= 1;
            }
        }
        KeyCode::Down => {
            if app.menu_cursor < count.saturating_sub(1) {
                app.menu_cursor += 1;
            }
        }
        KeyCode::Char(' ') | KeyCode::Enter => {
            if app.menu_cursor < count {
                app.toggle_component_visibility(app.menu_cursor);
            }
        }
        KeyCode::Esc | KeyCode::Char('v') | KeyCode::Char('V') => {
            app.ui_mode = UiMode::Normal;
        }
        _ => {}
    }
}

fn handle_add_menu_key(app: &mut App, code: KeyCode) {
    let options = ui::add_menu_options();
    match code {
        KeyCode::Up => {
            if app.menu_cursor > 0 {
                app.menu_cursor -= 1;
            }
        }
        KeyCode::Down => {
            if app.menu_cursor < options.len().saturating_sub(1) {
                app.menu_cursor += 1;
            }
        }
        KeyCode::Enter => {
            if let Some(&(_, comp_type, clock_style)) = options.get(app.menu_cursor) {
                app.add_component(comp_type, clock_style);
                app.ui_mode = UiMode::Normal;
            }
        }
        KeyCode::Esc | KeyCode::Char('a') | KeyCode::Char('A') => {
            app.ui_mode = UiMode::Normal;
        }
        _ => {}
    }
}

fn handle_color_menu_key(app: &mut App, code: KeyCode) {
    let count = crate::constants::COLOR_PRESETS.len();
    let rows = ui::color_menu_rows();
    let in_right = app.menu_cursor >= rows;

    match code {
        KeyCode::Up => {
            if in_right {
                if app.menu_cursor > rows {
                    app.menu_cursor -= 1;
                }
            } else if app.menu_cursor > 0 {
                app.menu_cursor -= 1;
            }
        }
        KeyCode::Down => {
            let next = app.menu_cursor + 1;
            if in_right {
                if next < count {
                    app.menu_cursor = next;
                }
            } else if next < rows {
                app.menu_cursor = next;
            }
        }
        KeyCode::Left => {
            if in_right {
                app.menu_cursor -= rows;
            }
        }
        KeyCode::Right => {
            let target = app.menu_cursor + rows;
            if !in_right && target < count {
                app.menu_cursor = target;
            }
        }
        KeyCode::Enter => {
            app.apply_color_preset(app.menu_cursor);
            app.ui_mode = UiMode::Normal;
        }
        KeyCode::Esc => {
            app.ui_mode = UiMode::Normal;
        }
        _ => {}
    }
}

fn handle_style_menu_key(app: &mut App, code: KeyCode) {
    const ITEM_COUNT: usize = 4; // Text Color, Background, Border Color, Reset All
    match code {
        KeyCode::Up => {
            if app.menu_cursor > 0 {
                app.menu_cursor -= 1;
            }
        }
        KeyCode::Down => {
            if app.menu_cursor < ITEM_COUNT - 1 {
                app.menu_cursor += 1;
            }
        }
        KeyCode::Enter => match app.menu_cursor {
            0 => {
                app.style_target = StyleProperty::Fg;
                app.menu_cursor = 0;
                app.ui_mode = UiMode::StyleColorPicker;
            }
            1 => {
                app.style_target = StyleProperty::Bg;
                app.menu_cursor = 0;
                app.ui_mode = UiMode::StyleColorPicker;
            }
            2 => {
                app.style_target = StyleProperty::BorderColor;
                app.menu_cursor = 0;
                app.ui_mode = UiMode::StyleColorPicker;
            }
            3 => {
                app.reset_component_style();
                app.ui_mode = UiMode::Normal;
            }
            _ => {}
        },
        KeyCode::Esc => {
            app.ui_mode = UiMode::Normal;
        }
        _ => {}
    }
}

fn handle_style_color_picker_key(app: &mut App, code: KeyCode) {
    let count = crate::constants::STYLE_COLOR_PRESETS.len();
    let rows = ui::style_color_picker_rows();
    let in_right = app.menu_cursor >= rows;

    match code {
        KeyCode::Up => {
            if in_right {
                if app.menu_cursor > rows {
                    app.menu_cursor -= 1;
                }
            } else if app.menu_cursor > 0 {
                app.menu_cursor -= 1;
            }
        }
        KeyCode::Down => {
            let next = app.menu_cursor + 1;
            if in_right {
                if next < count {
                    app.menu_cursor = next;
                }
            } else if next < rows {
                app.menu_cursor = next;
            }
        }
        KeyCode::Left => {
            if in_right {
                app.menu_cursor -= rows;
            }
        }
        KeyCode::Right => {
            let target = app.menu_cursor + rows;
            if !in_right && target < count {
                app.menu_cursor = target;
            }
        }
        KeyCode::Enter => {
            app.apply_style_color(app.menu_cursor);
            app.ui_mode = UiMode::Normal;
        }
        KeyCode::Esc => {
            app.ui_mode = UiMode::Normal;
        }
        _ => {}
    }
}

fn handle_cal_select_key(app: &mut App, code: KeyCode) {
    let count = app.cal_select_items.len();
    match code {
        KeyCode::Up => {
            if app.cal_select_cursor > 0 {
                app.cal_select_cursor -= 1;
            }
        }
        KeyCode::Down => {
            if app.cal_select_cursor < count.saturating_sub(1) {
                app.cal_select_cursor += 1;
            }
        }
        KeyCode::Enter => {
            if !app.cal_select_items.is_empty() {
                app.calendar_select_confirm();
            }
            app.ui_mode = UiMode::Normal;
        }
        KeyCode::Esc => {
            app.ui_mode = UiMode::Normal;
        }
        _ => {}
    }
}

fn handle_cal_remove_key(app: &mut App, code: KeyCode) {
    let count = app.focused_clock_calendars().len();
    match code {
        KeyCode::Up => {
            if app.menu_cursor > 0 {
                app.menu_cursor -= 1;
            }
        }
        KeyCode::Down => {
            if app.menu_cursor < count.saturating_sub(1) {
                app.menu_cursor += 1;
            }
        }
        KeyCode::Char('n') | KeyCode::Char('N') => {
            if app.menu_cursor < count {
                app.toggle_calendar_native(app.menu_cursor);
            }
        }
        KeyCode::Char('d') | KeyCode::Char('D') | KeyCode::Delete | KeyCode::Backspace => {
            if app.menu_cursor < count {
                app.remove_secondary_calendar(app.menu_cursor);
                let new_count = app.focused_clock_calendars().len();
                if new_count == 0 {
                    app.ui_mode = UiMode::Normal;
                } else if app.menu_cursor >= new_count {
                    app.menu_cursor = new_count - 1;
                }
            }
        }
        KeyCode::Esc => {
            app.ui_mode = UiMode::Normal;
        }
        _ => {}
    }
}

fn handle_tz_search_key(app: &mut App, code: KeyCode) {
    match code {
        KeyCode::Char(c) => {
            app.tz_search_query.push(c);
            app.tz_search_update();
        }
        KeyCode::Backspace => {
            app.tz_search_query.pop();
            app.tz_search_update();
        }
        KeyCode::Up => {
            if app.tz_search_cursor > 0 {
                app.tz_search_cursor -= 1;
            }
        }
        KeyCode::Down => {
            let max = app.tz_search_results.len().saturating_sub(1);
            if app.tz_search_cursor < max {
                app.tz_search_cursor += 1;
            }
        }
        KeyCode::Enter => {
            app.tz_search_select();
            app.ui_mode = UiMode::Normal;
        }
        KeyCode::Esc => {
            app.ui_mode = UiMode::Normal;
        }
        _ => {}
    }
}

fn handle_tz_remove_menu_key(app: &mut App, code: KeyCode) {
    let count = app.focused_world_clock_timezones().len();
    match code {
        KeyCode::Up => {
            if app.menu_cursor > 0 {
                app.menu_cursor -= 1;
            }
        }
        KeyCode::Down => {
            if app.menu_cursor < count.saturating_sub(1) {
                app.menu_cursor += 1;
            }
        }
        KeyCode::Enter => {
            if app.menu_cursor < count {
                app.remove_timezone(app.menu_cursor);
                // Clamp cursor after removal
                let new_count = app.focused_world_clock_timezones().len();
                if new_count == 0 {
                    app.ui_mode = UiMode::Normal;
                } else if app.menu_cursor >= new_count {
                    app.menu_cursor = new_count - 1;
                }
            }
        }
        KeyCode::Esc => {
            app.ui_mode = UiMode::Normal;
        }
        _ => {}
    }
}

fn handle_tz_reorder_key(app: &mut App, key: KeyEvent) {
    let count = app.focused_world_clock_timezones().len();
    let shifted = key.modifiers.contains(KeyModifiers::SHIFT);

    match key.code {
        KeyCode::Up => {
            if shifted {
                if app.menu_cursor > 0 {
                    app.swap_timezone(app.menu_cursor, -1);
                    app.menu_cursor -= 1;
                }
            } else if app.menu_cursor > 0 {
                app.menu_cursor -= 1;
            }
        }
        KeyCode::Down => {
            if shifted {
                if app.menu_cursor + 1 < count {
                    app.swap_timezone(app.menu_cursor, 1);
                    app.menu_cursor += 1;
                }
            } else if app.menu_cursor < count.saturating_sub(1) {
                app.menu_cursor += 1;
            }
        }
        KeyCode::Esc => {
            app.ui_mode = UiMode::Normal;
        }
        _ => {}
    }
}

fn handle_mouse_click(app: &mut App, col: u16, row: u16) {
    if app.ui_mode != UiMode::Normal {
        app.ui_mode = UiMode::Normal;
        return;
    }

    let visible = app.visible_components();
    for (vi, &ci) in visible.iter().enumerate() {
        let id = &app.components[ci].id;
        if let Some(rt) = app.runtime.get(id) {
            let area = rt.area();
            if col >= area.x
                && col < area.x + area.width
                && row >= area.y
                && row < area.y + area.height
            {
                app.focused_index = vi;
                return;
            }
        }
    }
}
