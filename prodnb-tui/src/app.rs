use prodnb_core::{Protein, ProteinFeatures, ArrangementPlan, DnBParameters, Style, CompositionEngine, protein_to_strudel, protein_to_strudel_layered, default_strudel_code};
use prodnb_audio::AudioEngine;
use std::sync::Arc;
use std::time::{Duration, Instant};
use anyhow::Result;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PlaybackState {
    Stopped,
    Playing,
    Paused,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AppState {
    Browsing,
    Playing,
    Exporting,
}

pub struct App {
    pub state: AppState,
    pub playback_state: PlaybackState,

    pub protein: Option<Protein>,
    pub loaded_path: Option<String>,
    pub features: Option<ProteinFeatures>,
    pub arrangement: Option<ArrangementPlan>,
    pub parameters: DnBParameters,
    pub seed: u64,

    pub audio_engine: Option<Arc<spin::Mutex<AudioEngine>>>,

    pub current_bar: u16,
    pub current_section: String,
    pub last_frame_time: Instant,
    pub fps: f64,

    pub should_quit: bool,

    /// Strudel-like code editor: always visible, multi-line
    pub editor_lines: Vec<String>,
    pub editor_cursor_row: usize,
    pub editor_cursor_col: usize,
    pub editor_output: String,  // Last eval result or feedback

    /// Set when arrangement changes (e.g. load, reseed); CLI should recreate playback
    pub needs_audio_restart: bool,

    /// Help overlay shown when user presses /
    pub show_help_overlay: bool,
}

impl App {
    pub fn new() -> Self {
        App {
            state: AppState::Browsing,
            playback_state: PlaybackState::Stopped,
            protein: None,
            loaded_path: None,
            features: None,
            arrangement: None,
            parameters: DnBParameters::default(),
            seed: 42,
            audio_engine: None,
            current_bar: 0,
            current_section: String::new(),
            last_frame_time: Instant::now(),
            fps: 0.0,
            should_quit: false,
            editor_lines: vec![
                "style liquid".into(),
                "bpm 174".into(),
                "intensity 0.5".into(),
                "// Ctrl+Enter eval | Ctrl+. stop | / help".into(),
            ],
            editor_cursor_row: 0,
            editor_cursor_col: 0,
            editor_output: String::new(),
            needs_audio_restart: false,
            show_help_overlay: false,
        }
    }

    pub fn editor_current_line(&self) -> &str {
        self.editor_lines
            .get(self.editor_cursor_row)
            .map(|s| s.as_str())
            .unwrap_or("")
    }

    pub fn editor_insert_char(&mut self, c: char) {
        if self.editor_lines.is_empty() {
            self.editor_lines.push(String::new());
        }
        let row = self.editor_cursor_row.min(self.editor_lines.len().saturating_sub(1));
        let line = &mut self.editor_lines[row];
        let col = self.editor_cursor_col.min(line.len());
        line.insert(col, c);
        self.editor_cursor_col = col + 1;
    }

    pub fn editor_backspace(&mut self) {
        if self.editor_lines.is_empty() {
            return;
        }
        let row = self.editor_cursor_row.min(self.editor_lines.len().saturating_sub(1));
        let line = &mut self.editor_lines[row];
        if self.editor_cursor_col > 0 {
            let col = self.editor_cursor_col - 1;
            line.remove(col);
            self.editor_cursor_col = col;
        } else if row > 0 {
            let prev_len = self.editor_lines[row - 1].len();
            let prev = self.editor_lines.remove(row);
            self.editor_lines[row - 1].push_str(&prev);
            self.editor_cursor_row = row - 1;
            self.editor_cursor_col = prev_len;
        }
    }

    pub fn editor_delete(&mut self) {
        if self.editor_lines.is_empty() {
            return;
        }
        let row = self.editor_cursor_row.min(self.editor_lines.len().saturating_sub(1));
        if self.editor_cursor_col < self.editor_lines[row].len() {
            self.editor_lines[row].remove(self.editor_cursor_col);
        } else if row + 1 < self.editor_lines.len() {
            let next = self.editor_lines.remove(row + 1);
            self.editor_lines[row].push_str(&next);
        }
    }

    pub fn editor_newline(&mut self) {
        let row = self.editor_cursor_row.min(self.editor_lines.len().saturating_sub(1));
        let line = &mut self.editor_lines[row];
        let col = self.editor_cursor_col.min(line.len());
        let rest: String = line.drain(col..).collect();
        self.editor_lines.insert(row + 1, rest);
        self.editor_cursor_row = row + 1;
        self.editor_cursor_col = 0;
    }

    pub fn editor_move_left(&mut self) {
        if self.editor_cursor_col > 0 {
            self.editor_cursor_col -= 1;
        } else if self.editor_cursor_row > 0 {
            self.editor_cursor_row -= 1;
            self.editor_cursor_col = self.editor_lines[self.editor_cursor_row].len();
        }
    }

    pub fn editor_move_right(&mut self) {
        let row = self.editor_cursor_row.min(self.editor_lines.len().saturating_sub(1));
        let line_len = self.editor_lines.get(row).map(|s| s.len()).unwrap_or(0);
        if self.editor_cursor_col < line_len {
            self.editor_cursor_col += 1;
        } else if self.editor_cursor_row + 1 < self.editor_lines.len() {
            self.editor_cursor_row += 1;
            self.editor_cursor_col = 0;
        }
    }

    pub fn editor_move_up(&mut self) {
        if self.editor_cursor_row > 0 {
            self.editor_cursor_row -= 1;
            self.editor_cursor_col = self.editor_cursor_col
                .min(self.editor_lines[self.editor_cursor_row].len());
        }
    }

    pub fn editor_move_down(&mut self) {
        if self.editor_cursor_row + 1 < self.editor_lines.len() {
            self.editor_cursor_row += 1;
            let line_len = self.editor_lines[self.editor_cursor_row].len();
            self.editor_cursor_col = self.editor_cursor_col.min(line_len);
        }
    }

    pub fn editor_eval_current_line(&mut self) {
        let line = self.editor_current_line().trim().to_string();
        if line.is_empty() || line.starts_with("//") {
            return;
        }
        self.editor_output = self.eval_command(&line);
    }

    pub fn editor_eval_all(&mut self) {
        let mut out = Vec::new();
        for line in &self.editor_lines.clone() {
            let line = line.trim();
            if line.is_empty() || line.starts_with("//") {
                continue;
            }
            let result = self.eval_command(line);
            if !result.is_empty() {
                out.push(result);
            }
        }
        self.editor_output = out.join(" | ");
    }

    pub fn set_intensity(&mut self, value: f32) {
        self.parameters.intensity = value.clamp(0.0, 1.0);
    }

    pub fn set_complexity(&mut self, value: f32) {
        self.parameters.complexity = value.clamp(0.0, 1.0);
    }

    pub fn set_bpm(&mut self, bpm: u16) {
        self.parameters.bpm = bpm.clamp(60, 240);
        if let Some(ref mut arr) = self.arrangement {
            arr.bpm = self.parameters.bpm;
        }
    }

    /// Evaluate a REPL command and apply it. Returns feedback message.
    pub fn eval_command(&mut self, cmd: &str) -> String {
        let cmd = cmd.trim();
        if cmd.is_empty() {
            return String::new();
        }

        // Strudel compatibility: setcps(x) → set our BPM
        if cmd.trim().starts_with("setcps(") {
            if let Some(inner) = cmd.trim().strip_prefix("setcps(").and_then(|s| s.strip_suffix(')')) {
                if let Ok(cps) = inner.trim().parse::<f64>() {
                    let bpm = (cps * 60.0 * 4.0).round() as u16;
                    self.set_bpm(bpm.clamp(60, 240));
                    return format!("BPM set to {} (from setcps)", bpm);
                }
            }
        }

        let parts: Vec<&str> = cmd.split_whitespace().collect();
        let first = parts.first().map(|s| s.to_lowercase()).unwrap_or_default();
        match first.as_str() {
            "style" => {
                let v = parts.get(1).map(|s| s.to_lowercase()).unwrap_or_default();
                match v.as_str() {
                    "1" | "liquid" => {
                        self.set_style(Style::Liquid);
                        "Style: Liquid".into()
                    }
                    "2" | "jungle" => {
                        self.set_style(Style::Jungle);
                        "Style: Jungle".into()
                    }
                    "3" | "neuro" => {
                        self.set_style(Style::Neuro);
                        "Style: Neuro".into()
                    }
                    _ => "Usage: style 1|2|3 or liquid|jungle|neuro".into(),
                }
            }
            "intensity" => {
                let v = parts.get(1).unwrap_or(&"");
                if let Ok(f) = v.parse::<f32>() {
                    self.set_intensity(f);
                    format!("Intensity: {:.2}", self.parameters.intensity)
                } else {
                    "Usage: intensity 0.0-1.0".into()
                }
            }
            "complexity" => {
                let v = parts.get(1).unwrap_or(&"");
                if let Ok(f) = v.parse::<f32>() {
                    self.set_complexity(f);
                    format!("Complexity: {:.2}", self.parameters.complexity)
                } else {
                    "Usage: complexity 0.0-1.0".into()
                }
            }
            "bpm" => {
                let v = parts.get(1).unwrap_or(&"");
                if let Ok(b) = v.parse::<u16>() {
                    self.set_bpm(b);
                    if let Some(ref mut arr) = self.arrangement {
                        arr.bpm = self.parameters.bpm;
                    }
                    format!("BPM: {}", self.parameters.bpm)
                } else {
                    "Usage: bpm 60-240".into()
                }
            }
            "reseed" => {
                self.reseed();
                format!("Reseeded: {}", self.seed)
            }
            "help" => {
                "Commands: load, style, bpm, intensity, complexity, reseed, strudel, code, layer, llm".into()
            }
            "strudel" | "code" => {
                let code = if let Some(ref p) = self.protein {
                    protein_to_strudel(p, self.parameters.bpm)
                } else {
                    default_strudel_code(self.parameters.bpm)
                };
                // Insert generated code into editor for user to run
                for line in code.lines() {
                    self.editor_lines.push(line.to_string());
                }
                format!("Inserted {} lines (Ctrl+Enter to eval)", code.lines().count())
            }
            "layer" => {
                if let Some(ref p) = self.protein {
                    let code = protein_to_strudel_layered(p, self.parameters.bpm);
                    for line in code.lines() {
                        self.editor_lines.push(line.to_string());
                    }
                    format!("Inserted {} lines (layered by chain)", code.lines().count())
                } else {
                    "Load a PDB first: load path/to/file.pdb".into()
                }
            }
            "llm" | "reorganize" => {
                if let Some(ref p) = self.loaded_path {
                    match std::fs::read_to_string(p) {
                        Ok(pdb_content) => match crate::llm::reorganize_with_llm(&pdb_content) {
                            Ok(code) => {
                                for line in code.lines() {
                                    self.editor_lines.push(line.to_string());
                                }
                                format!("LLM inserted {} lines", code.lines().count())
                            }
                            Err(e) => format!("LLM error: {}", e),
                        },
                        Err(e) => format!("Read failed: {}", e),
                    }
                } else {
                    "Load a PDB first: load path/to/file.pdb".into()
                }
            }
            "load" => {
                let path = parts.get(1..).map(|p| p.join(" ")).unwrap_or_default().trim().to_string();
                if path.is_empty() {
                    "Usage: load <path/to/file.pdb>".into()
                } else {
                    match Protein::load_from_file(&path) {
                        Ok(protein) => {
                            if let Err(e) = self.load_protein(protein, path.clone()) {
                                format!("Load failed: {}", e)
                            } else {
                                format!("Loaded {}", path)
                            }
                        }
                        Err(e) => format!("Load failed: {}", e),
                    }
                }
            }
            _ => format!("Unknown: '{}'. Type 'help' for commands.", first),
        }
    }

    pub fn load_protein(&mut self, protein: Protein, path: String) -> Result<()> {
        let features = prodnb_core::FeatureExtractor::extract(&protein)?;

        let mut composer = CompositionEngine::new(self.seed);
        let params = composer.map_features_to_params(&features, &self.parameters);

        let arrangement = composer.compose(&features, &params)?;

        self.protein = Some(protein);
        self.loaded_path = Some(path);
        self.features = Some(features);
        self.arrangement = Some(arrangement);
        self.parameters = params;
        self.needs_audio_restart = true;

        Ok(())
    }

    pub fn set_audio_engine(&mut self, engine: Arc<spin::Mutex<AudioEngine>>) {
        self.audio_engine = Some(engine);
    }

    pub fn toggle_playback(&mut self) {
        match self.playback_state {
            PlaybackState::Stopped => {
                if self.arrangement.is_some() {
                    self.playback_state = PlaybackState::Playing;
                    self.state = AppState::Playing;
                }
            }
            PlaybackState::Playing => {
                self.playback_state = PlaybackState::Paused;
            }
            PlaybackState::Paused => {
                self.playback_state = PlaybackState::Playing;
            }
        }
    }

    pub fn stop_playback(&mut self) {
        self.playback_state = PlaybackState::Stopped;
        self.current_bar = 0;
        self.state = AppState::Browsing;
        if let Some(engine) = &self.audio_engine {
            let mut engine = engine.lock();
            engine.reset();
        }
    }

    pub fn seek(&mut self, bars: i16) {
        if self.arrangement.is_none() {
            return;
        }

        let total_bars: u16 = self.arrangement.as_ref().unwrap().sections.iter()
            .map(|s| s.bars)
            .sum();

        let new_bar = (self.current_bar as i16 + bars).max(0).min(total_bars as i16 - 1) as u16;
        self.current_bar = new_bar;

        self.update_current_section();
    }

    pub fn set_style(&mut self, style: Style) {
        self.parameters.style = style;
    }

    pub fn adjust_intensity(&mut self, delta: f32) {
        self.parameters.intensity = (self.parameters.intensity + delta).max(0.0).min(1.0);
    }

    pub fn adjust_complexity(&mut self, delta: f32) {
        self.parameters.complexity = (self.parameters.complexity + delta).max(0.0).min(1.0);
    }

    pub fn reseed(&mut self) {
        use rand::Rng;
        self.seed = rand::thread_rng().gen();

        if let (Some(protein), Some(features)) = (&self.protein, &self.features) {
            let mut composer = CompositionEngine::new(self.seed);
            let params = composer.map_features_to_params(features, &self.parameters);
            let arrangement = composer.compose(features, &params).ok();

            self.parameters = params;
            self.arrangement = arrangement;
            self.needs_audio_restart = true;
        }
    }

    pub fn export(&mut self, format: ExportFormat) {
        self.state = AppState::Exporting;
    }

    pub fn update(&mut self) {
        let now = Instant::now();
        let delta = now.duration_since(self.last_frame_time);
        self.last_frame_time = now;

        self.fps = 1.0 / delta.as_secs_f64();

        if self.playback_state == PlaybackState::Playing {
            self.update_playback();
        }
    }

    fn update_playback(&mut self) {
        if self.arrangement.is_none() {
            return;
        }

        let bpm = self.arrangement.as_ref().unwrap().bpm;
        let bars_per_second = bpm as f64 / 60.0;

        self.current_bar += 1;
        self.update_current_section();
    }

    fn update_current_section(&mut self) {
        if self.arrangement.is_none() {
            return;
        }

        for section in &self.arrangement.as_ref().unwrap().sections {
            if self.current_bar >= section.start_bar
                && self.current_bar < section.start_bar + section.bars
            {
                self.current_section = format!("{:?}", section.section);
                break;
            }
        }
    }

    pub fn scope_samples(&self) -> Vec<f32> {
        Vec::new()
    }
}

impl Default for App {
    fn default() -> Self {
        Self::new()
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ExportFormat {
    Midi,
    Wav,
    Both,
}
