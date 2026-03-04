use prodnb_core::{Protein, ProteinFeatures, ArrangementPlan, DnBParameters, Style, CompositionEngine};
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
}

impl App {
    pub fn new() -> Self {
        App {
            state: AppState::Browsing,
            playback_state: PlaybackState::Stopped,
            protein: None,
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
        }
    }

    pub fn load_protein(&mut self, protein: Protein) -> Result<()> {
        let features = prodnb_core::FeatureExtractor::extract(&protein)?;

        let mut composer = CompositionEngine::new(self.seed);
        let params = composer.map_features_to_params(&features, &self.parameters);

        let arrangement = composer.compose(&features, &params)?;

        self.protein = Some(protein);
        self.features = Some(features);
        self.arrangement = Some(arrangement);
        self.parameters = params;

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
