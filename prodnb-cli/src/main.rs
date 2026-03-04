use clap::{Parser, Subcommand};
use anyhow::{Result, Context};
use prodnb_core::Protein;
use prodnb_tui::{App, draw_ui, InputHandler};
use ratatui::{
    backend::CrosstermBackend,
    Terminal,
};
use crossterm::{
    event::{self, Event, DisableMouseCapture, EnableMouseCapture},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use std::{
    io,
    time::{Duration, Instant},
};

#[derive(Parser)]
#[command(name = "prodnb")]
#[command(about = "Turn protein structures into Drum & Bass tracks", long_about = None)]
struct Cli {
    #[command(subcommand)]
    command: Option<Commands>,
}

#[derive(Subcommand)]
enum Commands {
    /// Launch interactive TUI
    Tui {
        /// PDB or mmCIF file to load
        file: Option<String>,
    },
    /// Render to WAV (headless)
    Render {
        /// PDB or mmCIF file to process
        file: String,
        /// Output WAV file path
        #[arg(short, long)]
        output: Option<String>,
        /// Style (1=Liquid, 2=Jungle, 3=Neuro)
        #[arg(short, long, default_value_t = 1)]
        style: u8,
        /// Seed for reproducible output
        #[arg(short, long)]
        seed: Option<u64>,
    },
}

fn main() -> Result<()> {
    env_logger::init();

    let cli = Cli::parse();

    match cli.command {
        Some(Commands::Tui { file }) => run_tui(file),
        Some(Commands::Render { file, output, style, seed }) => run_render(file, output, style, seed),
        None => run_tui(None),
    }
}

fn run_tui(file_path: Option<String>) -> Result<()> {
    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen, EnableMouseCapture)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    let mut app = App::new();

    if let Some(path) = file_path {
        match load_protein(&path) {
            Ok(protein) => {
                if let Err(e) = app.load_protein(protein) {
                    eprintln!("Warning: Failed to extract features: {}", e);
                }
            }
            Err(e) => {
                eprintln!("Error loading file {}: {}", path, e);
            }
        }
    }

    let mut last_tick = Instant::now();
    let tick_rate = Duration::from_millis(1000 / 60);

    loop {
        terminal.draw(|f| draw_ui(f, &app))?;

        let timeout = tick_rate
            .checked_sub(last_tick.elapsed())
            .unwrap_or_else(|| Duration::from_secs(0));

        if crossterm::event::poll(timeout)? {
            if let Event::Key(key) = event::read()? {
                InputHandler::handle_event(Event::Key(key), &mut app);

                if app.should_quit {
                    break;
                }
            }
        }

        if last_tick.elapsed() >= tick_rate {
            app.update();
            last_tick = Instant::now();
        }
    }

    disable_raw_mode()?;
    execute!(
        terminal.backend_mut(),
        LeaveAlternateScreen,
        DisableMouseCapture
    )?;
    terminal.show_cursor()?;

    Ok(())
}

fn run_render(file_path: String, output_path: Option<String>, style_num: u8, seed: Option<u64>) -> Result<()> {
    println!("Loading protein from {}...", file_path);

    let protein = load_protein(&file_path)?;
    println!("Loaded protein: {} chains, {} residues",
             protein.chain_count(),
             protein.residue_count());

    let mut app = App::new();
    app.load_protein(protein)?;

    let seed = seed.unwrap_or(42);
    app.seed = seed;

    let style = prodnb_core::Style::from_int(style_num);
    app.set_style(style);

    let output = output_path.unwrap_or_else(|| {
        format!("{}_track.wav", file_path.trim_end_matches(".pdb").trim_end_matches(".cif"))
    });

    println!("Rendering track to {}...", output);
    println!("Style: {:?}", style);
    println!("Seed: {}", seed);

    if let Some(engine) = app.audio_engine {
        let render_config = prodnb_audio::RenderConfig::default();
        if let Some(arrangement) = &app.arrangement {
            let mut engine_guard = engine.lock();
            prodnb_audio::WavRenderer::render_to_file(
                &mut engine_guard,
                arrangement,
                &output,
                render_config,
            )?;
        }
    }

    println!("Done! Track saved to {}", output);
    Ok(())
}

fn load_protein(path: &str) -> Result<Protein> {
    Protein::load_from_file(path)
        .with_context(|| format!("Failed to load protein from {}", path))
}
