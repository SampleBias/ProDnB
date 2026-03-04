use crossterm::event::{Event, KeyCode, KeyEvent, KeyModifiers};
use crate::app::App;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum InputAction {
    TogglePlay,
    Stop,
    SeekLeft,
    SeekRight,
    StyleLiquid,
    StyleJungle,
    StyleNeuro,
    IntensityUp,
    IntensityDown,
    ComplexityUp,
    ComplexityDown,
    Export,
    Reseed,
    Quit,
}

pub struct InputHandler;

impl InputHandler {
    pub fn handle_event(event: Event, app: &mut App) {
        match event {
            Event::Key(key) => Self::handle_key_event(key, app),
            Event::Resize(_, _) => {}
            Event::Mouse(_) => {}
            Event::FocusGained => {}
            Event::FocusLost => {}
            Event::Paste(_) => {}
        }
    }

    fn handle_key_event(key: KeyEvent, app: &mut App) {
        let ctrl = key.modifiers.contains(KeyModifiers::CONTROL);

        // Global shortcuts (work even in editor)
        if ctrl && key.code == KeyCode::Char('q') {
            app.should_quit = true;
            return;
        }
        if ctrl && key.code == KeyCode::Char('.') {
            app.stop_playback();
            return;
        }
        if ctrl && key.code == KeyCode::Char('l') {
            app.editor_output.clear();
            return;
        }

        // Ctrl+Enter: evaluate (Strudel convention)
        if ctrl && key.code == KeyCode::Enter {
            app.editor_eval_current_line();
            return;
        }
        if ctrl && key.code == KeyCode::Char('e') {
            app.editor_eval_all();
            return;
        }

        // Space: Start / play/pause
        if key.code == KeyCode::Char(' ') && key.modifiers.is_empty() {
            app.toggle_playback();
            return;
        }

        // Ctrl+S: Submit to LLM
        if ctrl && key.code == KeyCode::Char('s') {
            app.editor_output = app.submit_to_llm();
            return;
        }

        // Tab: switch between path input and editor
        if key.code == KeyCode::Tab {
            app.focus_path_input = !app.focus_path_input;
            return;
        }

        // Path input mode: type path, Enter = Load
        if app.focus_path_input {
            match key.code {
                KeyCode::Enter => {
                    app.editor_output = app.load_from_path_input();
                }
                KeyCode::Backspace => {
                    app.pdb_path_input.pop();
                }
                KeyCode::Char(c) => {
                    app.pdb_path_input.push(c);
                }
                KeyCode::Left => {
                    // Could add cursor in path, skip for now
                }
                KeyCode::Right => {}
                _ => {}
            }
            return;
        }

        // / or Escape: toggle help overlay
        if app.show_help_overlay {
            if key.code == KeyCode::Char('/') || key.code == KeyCode::Esc {
                app.show_help_overlay = false;
            }
            return;
        }
        if key.code == KeyCode::Char('/') && key.modifiers.is_empty() {
            app.show_help_overlay = true;
            return;
        }

        // Editor navigation and input
        match key.code {
            KeyCode::Up => app.editor_move_up(),
            KeyCode::Down => app.editor_move_down(),
            KeyCode::Left => app.editor_move_left(),
            KeyCode::Right => app.editor_move_right(),
            KeyCode::Enter => app.editor_newline(),
            KeyCode::Backspace => app.editor_backspace(),
            KeyCode::Delete => app.editor_delete(),
            KeyCode::Char(c) => app.editor_insert_char(c),
            _ => {}
        }
    }
}
