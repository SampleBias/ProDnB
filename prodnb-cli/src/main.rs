mod playback;

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
    /// Test LLM API (Groq) with a PDB file
    TestLlm {
        /// PDB or mmCIF file to send to LLM
        file: String,
    },
}

fn main() -> Result<()> {
    dotenvy::dotenv().ok();
    env_logger::init();

    // If single arg looks like a PDB/mmCIF file, treat as `tui <file>` for convenience
    let args: Vec<String> = std::env::args().collect();
    if args.len() == 2 {
        let a = &args[1];
        if !a.starts_with('-') && (a.ends_with(".pdb") || a.ends_with(".cif") || a.ends_with(".ent")) {
            return run_tui(Some(a.clone()));
        }
    }

    let cli = Cli::parse();

    match cli.command {
        Some(Commands::Tui { file }) => run_tui(file),
        Some(Commands::Render { file, output, style, seed }) => run_render(file, output, style, seed),
        Some(Commands::TestLlm { file }) => run_test_llm(file),
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
    let mut playback_driver: Option<playback::PlaybackDriver> = None;

    if let Some(path) = file_path.clone() {
        match load_protein(&path) {
            Ok(protein) => {
                if let Err(e) = app.load_protein(protein, path) {
                    eprintln!("Warning: Failed to extract features: {}", e);
                }
            }
            Err(e) => {
                eprintln!("Error loading file {}: {}", path, e);
            }
        }
    }

    if let Some(ref arr) = app.arrangement {
        match playback::PlaybackDriver::new(arr) {
            Ok(driver) => {
                app.set_audio_engine(driver.engine.clone());
                playback_driver = Some(driver);
            }
            Err(e) => {
                eprintln!("Audio not available (install a SoundFont): {}", e);
                eprintln!("  Arch: pacman -S soundfont-fluid");
                eprintln!("  Debian: apt install fluid-soundfont-gm");
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
            if app.needs_audio_restart {
                app.needs_audio_restart = false;
                if let Some(ref arr) = app.arrangement {
                    match playback::PlaybackDriver::new(arr) {
                        Ok(driver) => {
                            app.set_audio_engine(driver.engine.clone());
                            playback_driver = Some(driver);
                        }
                        Err(_) => {}
                    }
                }
            }
            if let Some(ref driver) = playback_driver {
                match app.playback_state {
                    prodnb_tui::PlaybackState::Playing => {
                        let _ = driver.play();
                        app.current_bar = driver.current_bar();
                    }
                    prodnb_tui::PlaybackState::Paused => {
                        let _ = driver.pause();
                    }
                    prodnb_tui::PlaybackState::Stopped => {
                        let _ = driver.stop();
                    }
                }
            }
            app.poll_llm_stream();
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
    app.load_protein(protein, file_path.clone())?;

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

fn run_test_llm(file: String) -> Result<()> {
    println!("Testing LLM API (Groq groq/compound)...");
    println!("Loading {}...", file);

    let pdb_content = std::fs::read_to_string(&file)
        .with_context(|| format!("Failed to read {}", file))?;

    println!("PDB size: {} bytes", pdb_content.len());
    println!("Calling Groq API...");

    match prodnb_tui::llm::reorganize_with_llm(&pdb_content) {
        Ok(code) => {
            println!("\n--- LLM response (Strudel code) ---\n");
            println!("{}", code);
            println!("\n--- end ---");
            println!("\nOK: LLM API working ({} chars)", code.len());
        }
        Err(e) => {
            eprintln!("Error: {}", e);
            std::process::exit(1);
        }
    }

    Ok(())
}
