use crossterm::event::{Event, KeyCode, KeyEvent, KeyModifiers};
use crate::app::{App, ExportFormat};
use prodnb_core::Style;

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
        match key.code {
            KeyCode::Char('q') | KeyCode::Char('Q') => {
                app.should_quit = true;
            }
            KeyCode::Char(' ') => {
                app.toggle_playback();
            }
            KeyCode::Char('s') | KeyCode::Char('S') => {
                app.stop_playback();
            }
            KeyCode::Left => {
                app.seek(-4);
            }
            KeyCode::Right => {
                app.seek(4);
            }
            KeyCode::Char('1') => {
                app.set_style(Style::Liquid);
            }
            KeyCode::Char('2') => {
                app.set_style(Style::Jungle);
            }
            KeyCode::Char('3') => {
                app.set_style(Style::Neuro);
            }
            KeyCode::Up => {
                app.adjust_intensity(0.1);
            }
            KeyCode::Down => {
                app.adjust_intensity(-0.1);
            }
            KeyCode::Char(']') => {
                app.adjust_complexity(0.1);
            }
            KeyCode::Char('[') => {
                app.adjust_complexity(-0.1);
            }
            KeyCode::Char('e') | KeyCode::Char('E') => {
                app.export(ExportFormat::Midi);
            }
            KeyCode::Char('r') | KeyCode::Char('R') => {
                app.reseed();
            }
            KeyCode::Esc => {
                app.stop_playback();
            }
            _ => {}
        }
    }
}
