use crossterm::event::{
    self, Event as CrosstermEvent, KeyCode, KeyEventKind, MouseButton, MouseEventKind,
};
use std::time::Duration;

use crate::app::{App, PanelId, UiMode};

/// Polls for terminal events with the given timeout.
pub fn handle_events(app: &mut App, tick_rate: Duration) -> std::io::Result<()> {
    if event::poll(tick_rate)? {
        match event::read()? {
            CrosstermEvent::Key(key) if key.kind == KeyEventKind::Press => {
                handle_key(app, key.code);
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

fn handle_key(app: &mut App, code: KeyCode) {
    match app.ui_mode {
        UiMode::Normal => handle_normal_key(app, code),
        UiMode::ContextMenu => handle_context_menu_key(app, code),
        UiMode::VisibilityMenu => handle_visibility_menu_key(app, code),
        UiMode::Help => {
            // Any key dismisses help
            app.ui_mode = UiMode::Normal;
        }
    }
}

fn handle_normal_key(app: &mut App, code: KeyCode) {
    match code {
        KeyCode::Char('q') | KeyCode::Char('Q') => app.quit(),
        KeyCode::Esc => app.quit(),
        KeyCode::Char('h') | KeyCode::Char('H') | KeyCode::Char('?') => {
            app.ui_mode = UiMode::Help;
        }
        KeyCode::Char('f') | KeyCode::Right => app.cycle_font_next(),
        KeyCode::Char('F') | KeyCode::Left => app.cycle_font_prev(),
        KeyCode::Char('t') | KeyCode::Char('T') => app.toggle_time_format(),
        KeyCode::Tab => app.focus_next(),
        KeyCode::BackTab => app.focus_prev(),
        KeyCode::Char(' ') | KeyCode::Enter => app.open_context_menu(),
        KeyCode::Char('v') | KeyCode::Char('V') => {
            app.menu_cursor = 0;
            app.ui_mode = UiMode::VisibilityMenu;
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
                let action = item.action;
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
    let panel_count = PanelId::ALL.len();
    match code {
        KeyCode::Up => {
            if app.menu_cursor > 0 {
                app.menu_cursor -= 1;
            }
        }
        KeyCode::Down => {
            if app.menu_cursor < panel_count.saturating_sub(1) {
                app.menu_cursor += 1;
            }
        }
        KeyCode::Char(' ') | KeyCode::Enter => {
            if let Some(&panel) = PanelId::ALL.get(app.menu_cursor) {
                app.toggle_panel_visibility(panel);
            }
        }
        KeyCode::Esc | KeyCode::Char('v') | KeyCode::Char('V') => {
            app.ui_mode = UiMode::Normal;
        }
        _ => {}
    }
}

fn handle_mouse_click(app: &mut App, col: u16, row: u16) {
    // Clicking anywhere while a menu/overlay is open dismisses it
    if app.ui_mode != UiMode::Normal {
        app.ui_mode = UiMode::Normal;
        return;
    }

    // Click on clock area toggles time format
    let area = app.clock_area;
    if col >= area.x
        && col < area.x + area.width
        && row >= area.y
        && row < area.y + area.height
    {
        app.toggle_time_format();
    }
}
