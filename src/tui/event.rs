use crossterm::event::{self, Event, KeyCode, KeyEvent, KeyEventKind, KeyModifiers};
use std::time::Duration;

use super::app::{App, RollerFocus, Screen};

/// Poll for a crossterm event with the given timeout.
/// Returns None on timeout.
pub fn poll_event(timeout: Duration) -> std::io::Result<Option<Event>> {
    if event::poll(timeout)? {
        Ok(Some(event::read()?))
    } else {
        Ok(None)
    }
}

/// Handle a key event by dispatching to the current screen's handler.
pub fn handle_key(app: &mut App, key: KeyEvent) {
    // Only handle key press events (not release/repeat)
    if key.kind != KeyEventKind::Press {
        return;
    }

    // Global: Ctrl+C always quits
    if key.modifiers.contains(KeyModifiers::CONTROL) && key.code == KeyCode::Char('c') {
        app.should_quit = true;
        return;
    }

    match app.screen {
        Screen::Roller => match app.roller_focus {
            RollerFocus::Input => handle_roller_input(app, key),
            RollerFocus::Presets => handle_roller_presets(app, key),
        },
        Screen::History => handle_history(app, key),
        Screen::Help => handle_help(app, key),
    }
}

// ── Roller screen (input focused) ───────────────────────────────────────────

fn handle_roller_input(app: &mut App, key: KeyEvent) {
    match key.code {
        KeyCode::Enter => app.submit_roll(),
        KeyCode::Char(c) => {
            if key.modifiers.contains(KeyModifiers::CONTROL) {
                match c {
                    'a' => app.move_cursor_home(),
                    'e' => app.move_cursor_end(),
                    'u' => {
                        app.input.clear();
                        app.cursor_pos = 0;
                    }
                    _ => {}
                }
            } else {
                match c {
                    '[' => app.dist_move_target(-1),
                    ']' => app.dist_move_target(1),
                    _ => app.insert_char(c),
                }
            }
        }
        KeyCode::Backspace => app.delete_char_before(),
        KeyCode::Delete => app.delete_char_after(),
        KeyCode::Left => app.move_cursor_left(),
        KeyCode::Right => app.move_cursor_right(),
        KeyCode::Home => app.move_cursor_home(),
        KeyCode::End => app.move_cursor_end(),
        KeyCode::Up => app.history_up(),
        KeyCode::Down => app.history_down(),
        KeyCode::Tab => app.open_history(),
        KeyCode::Esc => {
            if app.error_msg.is_some() {
                app.error_msg = None;
            } else {
                app.should_quit = true;
            }
        }
        KeyCode::F(1) => app.toggle_help(),
        KeyCode::F(2) => app.open_presets(),
        _ => {}
    }
}

// ── Roller screen (presets focused) ─────────────────────────────────────────

fn handle_roller_presets(app: &mut App, key: KeyEvent) {
    match key.code {
        KeyCode::Up | KeyCode::Char('k') => app.preset_select_up(),
        KeyCode::Down | KeyCode::Char('j') => app.preset_select_down(),
        KeyCode::Enter => app.preset_roll_selected(),
        KeyCode::Char('d') => app.preset_delete_selected(),
        KeyCode::Esc => app.toggle_presets_focus(),
        KeyCode::F(2) => app.toggle_presets_focus(),
        KeyCode::Tab => {
            app.roller_focus = RollerFocus::Input;
            app.open_history();
        }
        KeyCode::F(1) => app.toggle_help(),
        KeyCode::Char('q') => {
            app.preset_confirm_delete = false;
            app.toggle_presets_focus();
        }
        _ => {
            app.preset_confirm_delete = false;
        }
    }
}

// ── History screen ──────────────────────────────────────────────────────────

fn handle_history(app: &mut App, key: KeyEvent) {
    match key.code {
        KeyCode::Tab | KeyCode::Esc => app.go_back(),
        KeyCode::PageUp => app.scroll_history_up(),
        KeyCode::PageDown => app.scroll_history_down(),
        KeyCode::Char('q') => app.go_back(),
        KeyCode::F(1) => app.toggle_help(),
        _ => {}
    }
}

// ── Help screen ─────────────────────────────────────────────────────────────

fn handle_help(app: &mut App, key: KeyEvent) {
    match key.code {
        KeyCode::Esc | KeyCode::F(1) | KeyCode::Char('q') => app.go_back(),
        _ => {}
    }
}
