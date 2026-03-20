use crossterm::event::{
    self, Event as CrosstermEvent, KeyCode, KeyEvent, KeyEventKind, KeyModifiers, MouseButton,
    MouseEventKind,
};
use std::time::Duration;

use crate::app::{App, UiMode};
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
