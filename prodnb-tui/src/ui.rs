use ratatui::{
    layout::{Constraint, Direction, Layout, Rect},
    style::Modifier,
    text::{Line, Span},
    widgets::{Block, Borders, Clear, Paragraph, Wrap},
    Frame,
};
use crate::app::App;
use crate::widgets::{Oscilloscope, Spectrum, Vectorscope};

/// Strudel-like dark theme colors
mod theme {
    use ratatui::style::Color;
    pub const BG: Color = Color::Rgb(18, 18, 24);
    pub const FG: Color = Color::Rgb(200, 200, 210);
    pub const ACCENT: Color = Color::Rgb(129, 161, 193);   // Strudel cyan/blue
    pub const STRING: Color = Color::Rgb(152, 195, 121);  // Green
    pub const COMMENT: Color = Color::Rgb(127, 132, 156); // Muted
    pub const BORDER: Color = Color::Rgb(60, 62, 78);
}

pub fn draw_ui(f: &mut Frame, app: &App) {
    let size = f.area();

    // Layout: control bar | stream output (always) | editor | viz | footer
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .margin(0)
        .constraints([
            Constraint::Length(3),
            Constraint::Length(10),
            Constraint::Length(6),
            Constraint::Min(8),
            Constraint::Length(4),
        ])
        .split(size);

    draw_control_bar(f, app, chunks[0]);
    draw_stream_output(f, app, chunks[1]);
    draw_editor(f, app, chunks[2]);
    draw_visualizations(f, app, chunks[3]);
    draw_footer(f, app, chunks[4]);

    if app.show_help_overlay {
        draw_help_overlay(f, size);
    }
}

fn draw_control_bar(f: &mut Frame, app: &App, area: Rect) {
    let path_style = if app.focus_path_input {
        ratatui::style::Style::default().fg(theme::ACCENT)
    } else {
        ratatui::style::Style::default().fg(theme::FG)
    };
    let placeholder = "path/to/file.pdb";
    let path_display = if app.pdb_path_input.is_empty() {
        Span::styled(placeholder, ratatui::style::Style::default().fg(theme::COMMENT))
    } else {
        Span::styled(&app.pdb_path_input, path_style)
    };
    let cursor = if app.focus_path_input {
        Span::styled("▌", ratatui::style::Style::default().fg(theme::ACCENT))
    } else {
        Span::raw("")
    };

    let load_btn = if app.load_in_progress {
        Span::styled(" [Loading…] ", ratatui::style::Style::default().fg(ratatui::style::Color::Yellow))
    } else if app.focus_path_input {
        Span::styled("[Load]", ratatui::style::Style::default().fg(theme::STRING))
    } else {
        Span::styled(" Load ", ratatui::style::Style::default().fg(theme::COMMENT))
    };
    let submit_btn = Span::styled(" [Submit] ", ratatui::style::Style::default().fg(theme::ACCENT));
    let start_btn = Span::styled(" [▶ Start] ", ratatui::style::Style::default().fg(ratatui::style::Color::Green));

    let line = Line::from(vec![
        Span::styled("PDB: ", ratatui::style::Style::default().fg(theme::COMMENT)),
        path_display,
        cursor,
        Span::raw("  "),
        load_btn,
        Span::styled(" Enter ", ratatui::style::Style::default().fg(theme::COMMENT)),
        submit_btn,
        Span::styled(" Ctrl+S ", ratatui::style::Style::default().fg(theme::COMMENT)),
        start_btn,
        Span::styled(" Space ", ratatui::style::Style::default().fg(theme::COMMENT)),
    ]);

    let block = Block::default()
        .borders(Borders::ALL)
        .title(" ProDnB ")
        .title_style(ratatui::style::Style::default().fg(theme::ACCENT))
        .border_style(ratatui::style::Style::default().fg(theme::BORDER))
        .style(ratatui::style::Style::default().bg(theme::BG));

    let inner = block.inner(area);
    f.render_widget(block, area);
    f.render_widget(Paragraph::new(line).wrap(Wrap { trim: true }), inner);
}

fn draw_stream_output(f: &mut Frame, app: &App, area: Rect) {
    let title = if app.llm_streaming() {
        " ⟳ LLM (Groq Compound) "
    } else if app.llm_last_output.is_some() {
        " Output "
    } else {
        " Output (Ctrl+S to Submit) "
    };
    let block = Block::default()
        .borders(Borders::ALL)
        .title(title)
        .title_style(ratatui::style::Style::default().fg(ratatui::style::Color::Cyan))
        .border_style(ratatui::style::Style::default().fg(theme::BORDER))
        .style(ratatui::style::Style::default().bg(theme::BG));

    let inner = block.inner(area);
    f.render_widget(block, area);

    let text: Vec<Line> = if let Some(ref err) = app.llm_stream_error {
        vec![Line::from(Span::styled(err, ratatui::style::Style::default().fg(ratatui::style::Color::Red)))]
    } else if app.llm_streaming() {
        if app.llm_stream_buffer.is_empty() {
            vec![Line::from(Span::styled("Waiting for Groq Compound...", ratatui::style::Style::default().fg(theme::COMMENT)))]
        } else {
            app.llm_stream_buffer
                .lines()
                .map(|l| Line::from(Span::styled(l, ratatui::style::Style::default().fg(theme::STRING))))
                .collect()
        }
    } else if let Some(ref out) = app.llm_last_output {
        out.lines()
            .map(|l| Line::from(Span::styled(l, ratatui::style::Style::default().fg(theme::STRING))))
            .collect()
    } else {
        vec![
            Line::from(Span::styled("1. Type PDB path above, Enter to Load", ratatui::style::Style::default().fg(theme::COMMENT))),
            Line::from(Span::styled("2. Ctrl+S to Submit to Groq Compound", ratatui::style::Style::default().fg(theme::COMMENT))),
            Line::from(Span::styled("3. Strudel code appears here", ratatui::style::Style::default().fg(theme::COMMENT))),
            Line::from(Span::styled("4. Space to Start music", ratatui::style::Style::default().fg(theme::COMMENT))),
        ]
    };

    let line_count = text.len();
    let para = Paragraph::new(text)
        .scroll((line_count.saturating_sub(inner.height as usize) as u16, 0))
        .wrap(Wrap { trim: false });
    f.render_widget(para, inner);
}

fn draw_help_overlay(f: &mut Frame, area: Rect) {
    let lines = vec![
        Line::from(vec![
            Span::styled(" ProDnB Key Bindings ", ratatui::style::Style::default().fg(ratatui::style::Color::Cyan).add_modifier(Modifier::BOLD)),
        ]),
        Line::from(""),
        Line::from(vec![
            Span::styled(" / ", ratatui::style::Style::default().fg(theme::ACCENT)),
            Span::styled("Show this help", ratatui::style::Style::default().fg(theme::FG)),
        ]),
        Line::from(vec![
            Span::styled(" Ctrl+Enter ", ratatui::style::Style::default().fg(theme::ACCENT)),
            Span::styled("Eval current line", ratatui::style::Style::default().fg(theme::FG)),
        ]),
        Line::from(vec![
            Span::styled(" Ctrl+E ", ratatui::style::Style::default().fg(theme::ACCENT)),
            Span::styled("Eval all lines", ratatui::style::Style::default().fg(theme::FG)),
        ]),
        Line::from(vec![
            Span::styled(" Ctrl+S ", ratatui::style::Style::default().fg(theme::ACCENT)),
            Span::styled("Submit to LLM", ratatui::style::Style::default().fg(theme::FG)),
        ]),
        Line::from(vec![
            Span::styled(" Space ", ratatui::style::Style::default().fg(theme::ACCENT)),
            Span::styled("Start / Play", ratatui::style::Style::default().fg(theme::FG)),
        ]),
        Line::from(vec![
            Span::styled(" Tab ", ratatui::style::Style::default().fg(theme::ACCENT)),
            Span::styled("Switch path ↔ editor", ratatui::style::Style::default().fg(theme::FG)),
        ]),
        Line::from(vec![
            Span::styled(" Ctrl+. ", ratatui::style::Style::default().fg(theme::ACCENT)),
            Span::styled("Stop playback", ratatui::style::Style::default().fg(theme::FG)),
        ]),
        Line::from(vec![
            Span::styled(" Ctrl+L ", ratatui::style::Style::default().fg(theme::ACCENT)),
            Span::styled("Clear output", ratatui::style::Style::default().fg(theme::FG)),
        ]),
        Line::from(vec![
            Span::styled(" Ctrl+Q ", ratatui::style::Style::default().fg(theme::ACCENT)),
            Span::styled("Quit", ratatui::style::Style::default().fg(theme::FG)),
        ]),
        Line::from(""),
        Line::from(vec![
            Span::styled(" Commands: ", ratatui::style::Style::default().fg(theme::ACCENT)),
        ]),
        Line::from("  Type path, Enter=Load  Ctrl+S=Submit"),
        Line::from("  style 1|2|3  liquid|jungle|neuro"),
        Line::from("  bpm 174      Set tempo"),
        Line::from("  intensity 0.5  complexity 0.5"),
        Line::from("  reseed       New random seed"),
        Line::from("  code/strudel Generate Strudel from PDB"),
        Line::from("  llm / Ctrl+S Submit PDB to Groq"),
        Line::from(""),
        Line::from(vec![
            Span::styled(" Esc or / to close ", ratatui::style::Style::default().fg(theme::COMMENT)),
        ]),
    ];

    let help = Paragraph::new(lines)
        .block(
            Block::default()
                .borders(Borders::ALL)
                .border_style(ratatui::style::Style::default().fg(theme::BORDER))
                .style(ratatui::style::Style::default().bg(theme::BG)),
        )
        .style(ratatui::style::Style::default().fg(theme::FG));

    let width = 42u16;
    let height = 22u16;
    let popup = Rect {
        x: area.x + (area.width.saturating_sub(width)) / 2,
        y: area.y + (area.height.saturating_sub(height)) / 2,
        width,
        height,
    };

    // Clear area for overlay
    f.render_widget(Clear, area);

    f.render_widget(help, popup);
}

fn draw_editor(f: &mut Frame, app: &App, area: Rect) {
    let title = match &app.protein {
        Some(p) => format!(" Code ({}) ", p.metadata.filename.as_deref().unwrap_or("")),
        None => " Code ".to_string(),
    };
    let block = Block::default()
        .borders(Borders::ALL)
        .title(title)
        .border_style(ratatui::style::Style::default().fg(theme::BORDER))
        .title_style(ratatui::style::Style::default().fg(theme::ACCENT))
        .style(ratatui::style::Style::default().bg(theme::BG));

    let inner = block.inner(area);
    f.render_widget(block, area);

    let mut lines: Vec<Line> = Vec::new();
    for (i, line) in app.editor_lines.iter().enumerate() {
        let is_cursor_row = i == app.editor_cursor_row;
        let styled: Vec<Span> = if line.trim_start().starts_with("//") {
            vec![Span::styled(line, ratatui::style::Style::default().fg(theme::COMMENT))]
        } else {
            vec![Span::styled(line, ratatui::style::Style::default().fg(theme::FG))]
        };
        let mut line_spans = styled;
        if is_cursor_row {
            let mut col = app.editor_cursor_col.min(line.len());
            while col > 0 && !line.is_char_boundary(col) {
                col -= 1;
            }
            let (before, after) = line.split_at(col);
            line_spans = vec![
                Span::styled(before, ratatui::style::Style::default().fg(theme::FG)),
                Span::styled("▌", ratatui::style::Style::default().fg(theme::ACCENT)),
                Span::styled(after, ratatui::style::Style::default().fg(theme::FG)),
            ];
        }
        lines.push(Line::from(line_spans));
    }

    let editor = Paragraph::new(lines)
        .style(ratatui::style::Style::default().bg(theme::BG).fg(theme::FG));

    f.render_widget(editor, inner);

    if !app.editor_output.is_empty() {
        let output_area = Rect {
            y: inner.y + inner.height.saturating_sub(1),
            height: 1,
            ..inner
        };
        if output_area.height > 0 {
            let output = Paragraph::new(Line::from(Span::styled(
                &app.editor_output,
                ratatui::style::Style::default().fg(theme::STRING),
            )));
            f.render_widget(output, output_area);
        }
    }
}

fn draw_visualizations(f: &mut Frame, app: &App, area: Rect) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Percentage(50),
            Constraint::Percentage(50),
        ])
        .split(area);

    let bottom = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Percentage(50), Constraint::Percentage(50)])
        .split(chunks[1]);

    let samples = app.scope_samples();
    f.render_widget(Oscilloscope::new(&samples), chunks[0]);
    f.render_widget(Spectrum::new(&samples), bottom[0]);
    f.render_widget(Vectorscope::new(&samples), bottom[1]);
}

fn draw_footer(f: &mut Frame, app: &App, area: Rect) {
    let status = format!(
        "{:?} | Bar: {} | {} | {:.0} FPS",
        app.playback_state,
        app.current_bar,
        app.current_section,
        app.fps
    );

    let hints = "PDB: Enter Load • Ctrl+S Submit • Space Start • Tab path↔editor • / help • Ctrl+Q Quit";

    let block = Block::default()
        .borders(Borders::ALL)
        .border_style(ratatui::style::Style::default().fg(theme::BORDER))
        .style(ratatui::style::Style::default().bg(theme::BG));

    let inner = block.inner(area);
    f.render_widget(block, area);

    let text = vec![
        Line::from(Span::styled(status, ratatui::style::Style::default().fg(theme::ACCENT))),
        Line::from(Span::styled(hints, ratatui::style::Style::default().fg(theme::COMMENT))),
    ];

    let footer = Paragraph::new(text).wrap(Wrap { trim: true });
    f.render_widget(footer, inner);
}
