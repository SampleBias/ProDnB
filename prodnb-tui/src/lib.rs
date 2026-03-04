pub mod app;
pub mod ui;
pub mod widgets;
pub mod input;
pub mod llm;

pub use app::{App, AppState, PlaybackState};
pub use ui::draw_ui;
pub use widgets::{Oscilloscope, Spectrum, Vectorscope};
pub use input::{InputHandler, InputAction};
