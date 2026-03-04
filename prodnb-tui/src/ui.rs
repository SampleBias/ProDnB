use ratatui::{
    layout::{Constraint, Direction, Layout, Rect},
    text::{Line, Span},
    widgets::{Block, Borders, Paragraph},
    Frame,
};
use crate::app::App;
use crate::widgets::{Oscilloscope, Spectrum, Vectorscope};

pub fn draw_ui(f: &mut Frame, app: &App) {
    let size = f.size();

    let header_height = 3;
    let footer_height = 3;

    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .margin(0)
        .constraints([
            Constraint::Length(header_height),
            Constraint::Min(0),
            Constraint::Length(footer_height),
        ])
        .split(size);

    draw_header(f, app, chunks[0]);

    let body_chunks = Layout::default()
        .direction(Direction::Vertical)
        .margin(1)
        .constraints([
            Constraint::Percentage(50),
            Constraint::Percentage(50),
        ])
        .split(chunks[1]);

    let oscilloscope_chunk = body_chunks[0];
    let bottom_chunks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Percentage(50),
            Constraint::Percentage(50),
        ])
        .split(body_chunks[1]);

    draw_oscilloscope(f, app, oscilloscope_chunk);
    draw_spectrum(f, app, bottom_chunks[0]);
    draw_vectorscope(f, app, bottom_chunks[1]);

    draw_footer(f, app, chunks[2]);
}

fn draw_header(f: &mut Frame, app: &App, area: Rect) {
    let title = match &app.protein {
        Some(p) => p.metadata.filename.clone().unwrap_or_else(|| "Unknown".to_string()),
        None => "No file loaded".to_string(),
    };

    let info = if let Some(f) = &app.features {
        format!(
            "Chains: {} | Residues: {} | BPM: {} | Style: {:?} | Seed: {}",
            f.chain_count,
            f.residue_count,
            app.parameters.bpm,
            app.parameters.style,
            app.seed
        )
    } else {
        "Load a PDB file to begin".to_string()
    };

    let text = vec![
        Line::from(vec![
            Span::styled(title, ratatui::style::Style::default()
                .fg(ratatui::style::Color::Cyan)
                .add_modifier(ratatui::style::Modifier::BOLD)),
        ]),
        Line::from(vec![
            Span::styled(info, ratatui::style::Style::default()
                .fg(ratatui::style::Color::Gray)),
        ]),
    ];

    let header = Paragraph::new(text)
        .block(Block::default()
            .borders(Borders::ALL)
            .border_style(ratatui::style::Style::default()
                .fg(ratatui::style::Color::Blue)));

    f.render_widget(header, area);
}

fn draw_oscilloscope(f: &mut Frame, app: &App, area: Rect) {
    let samples = app.scope_samples();
    let oscilloscope = Oscilloscope::new(&samples);
    f.render_widget(oscilloscope, area);
}

fn draw_spectrum(f: &mut Frame, app: &App, area: Rect) {
    let samples = app.scope_samples();
    let spectrum = Spectrum::new(&samples);
    f.render_widget(spectrum, area);
}

fn draw_vectorscope(f: &mut Frame, app: &App, area: Rect) {
    let samples = app.scope_samples();
    let vectorscope = Vectorscope::new(&samples);
    f.render_widget(vectorscope, area);
}

fn draw_footer(f: &mut Frame, app: &App, area: Rect) {
    let status = format!(
        "{:?} | Bar: {} | Section: {} | FPS: {:.1}",
        app.playback_state,
        app.current_bar,
        app.current_section,
        app.fps
    );

    let hints = "[Space] Play/Pause | [S] Stop | [←/→] Seek | [1/2/3] Style | [↑/↓] Intensity | [/] Complexity | [E] Export | [R] Reseed | [Q] Quit";

    let text = vec![
        Line::from(vec![
            Span::styled(status, ratatui::style::Style::default()
                .fg(ratatui::style::Color::Yellow)),
        ]),
        Line::from(vec![
            Span::styled(hints, ratatui::style::Style::default()
                .fg(ratatui::style::Color::DarkGray)),
        ]),
    ];

    let footer = Paragraph::new(text)
        .block(Block::default()
            .borders(Borders::ALL)
            .border_style(ratatui::style::Style::default()
                .fg(ratatui::style::Color::Blue)));

    f.render_widget(footer, area);
}
