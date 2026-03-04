//! Audio playback driver: soundfont resolution, MIDI scheduling, and stream management.

use anyhow::{Context, Result};
use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};
use prodnb_audio::AudioEngine;
use prodnb_core::ArrangementPlan;
use prodnb_midi::{MidiBuilder, MidiEvent};
use std::path::PathBuf;
use std::sync::atomic::{AtomicU64, AtomicUsize, Ordering};
use std::sync::Arc;

const SAMPLE_RATE: u32 = 44100;
const TICKS_PER_BEAT: u32 = 480;

/// Resolve path to a SoundFont file. Tries common locations.
pub fn resolve_soundfont() -> Result<PathBuf> {
    let candidates = [
        "assets/default.sf2",
        "assets/FluidR3_GM.sf2",
        "/usr/share/sounds/sf2/FluidR3_GM.sf2",
        "/usr/share/soundfonts/FluidR3_GM.sf2",
        "/usr/share/sounds/sf2/default.sf2",
    ];

    if let Ok(path) = std::env::var("PRODNB_SOUNDFONT") {
        let p = PathBuf::from(&path);
        if p.exists() {
            return Ok(p);
        }
    }

    for candidate in candidates {
        let p = PathBuf::from(candidate);
        if p.exists() {
            return Ok(p);
        }
    }

    anyhow::bail!(
        "No SoundFont found. Set PRODNB_SOUNDFONT or place FluidR3_GM.sf2 in assets/.\n\
         Install on Arch: sudo pacman -S soundfont-fluid\n\
         Install on Debian: sudo apt install fluid-soundfont-gm"
    )
}

/// Flatten MIDI tracks to (sample_offset, event) sorted by sample.
fn flatten_midi_to_samples(arrangement: &ArrangementPlan) -> Vec<(u64, MidiEvent)> {
    let mut builder = MidiBuilder::new();
    let params = prodnb_core::DnBParameters {
        bpm: arrangement.bpm,
        ..Default::default()
    };
    if builder.build_from_composition(arrangement, &params).is_err() {
        return Vec::new();
    }

    let samples_per_tick =
        (SAMPLE_RATE as f64 * 60.0) / (arrangement.bpm as f64 * TICKS_PER_BEAT as f64);

    let mut events: Vec<(u64, MidiEvent)> = Vec::new();
    for track in builder.tracks() {
        for ev in &track.events {
            let sample = (ev.start_ticks as f64 * samples_per_tick) as u64;
            events.push((sample, ev.clone()));
        }
    }
    events.sort_by_key(|(s, _)| *s);
    events
}

/// Shared state for audio callback (sample position for UI sync).
#[derive(Default)]
pub struct PlaybackState {
    pub samples_rendered: AtomicU64,
    pub next_event_idx: AtomicUsize,
}

impl PlaybackState {
    pub fn current_bar(&self, bpm: u16) -> u16 {
        let samples = self.samples_rendered.load(Ordering::Relaxed);
        let samples_per_bar = (SAMPLE_RATE as f64 * 60.0 * 4.0) / bpm as f64;
        (samples as f64 / samples_per_bar) as u16
    }

    pub fn reset(&self) {
        self.samples_rendered.store(0, Ordering::Relaxed);
        self.next_event_idx.store(0, Ordering::Relaxed);
    }
}

/// Playback driver: holds engine, stream, and shared state.
pub struct PlaybackDriver {
    pub engine: Arc<spin::Mutex<AudioEngine>>,
    pub stream: cpal::Stream,
    pub state: Arc<PlaybackState>,
    pub events: Arc<Vec<(u64, MidiEvent)>>,
    pub bpm: u16,
    is_playing: std::cell::Cell<bool>,
}

impl PlaybackDriver {
    pub fn new(arrangement: &ArrangementPlan) -> Result<Self> {
        let soundfont = resolve_soundfont()?;
        let engine = AudioEngine::new(soundfont.to_str().unwrap())
            .context("Failed to create audio engine")?;

        let events = flatten_midi_to_samples(arrangement);
        let events = Arc::new(events);
        let state = Arc::new(PlaybackState::default());
        let engine = Arc::new(spin::Mutex::new(engine));

        let engine_cb = engine.clone();
        let events_cb = events.clone();
        let state_cb = state.clone();
        let bpm = arrangement.bpm;

        let device = cpal::default_host()
            .default_output_device()
            .context("No default audio device (check ALSA/PulseAudio)")?;
        let stream_config = cpal::StreamConfig {
            channels: 2,
            sample_rate: cpal::SampleRate(SAMPLE_RATE),
            buffer_size: cpal::BufferSize::Default,
        };

        let stream = device.build_output_stream(
            &stream_config,
            move |data: &mut [f32], _: &cpal::OutputCallbackInfo| {
                let mut eng = engine_cb.lock();
                let samples_this_block = data.len() / 2;
                let base = state_cb.samples_rendered.load(Ordering::Relaxed);

                let mut idx = state_cb.next_event_idx.load(Ordering::Relaxed);
                for i in 0..samples_this_block {
                    let sample_offset = base + i as u64;
                    while idx < events_cb.len() {
                        let (ev_sample, ev) = &events_cb[idx];
                        if *ev_sample > sample_offset {
                            break;
                        }
                        eng.process_midi_events(&[ev.clone()], 0);
                        idx += 1;
                    }
                }
                state_cb.next_event_idx.store(idx, Ordering::Relaxed);

                let out = eng.render_block();
                let len = data.len().min(out.len());
                data[..len].copy_from_slice(&out[..len]);
                if len < data.len() {
                    data[len..].fill(0.0);
                }

                state_cb
                    .samples_rendered
                    .fetch_add(samples_this_block as u64, Ordering::Relaxed);
            },
            |e| eprintln!("Audio error: {}", e),
            None,
        )?;

        Ok(PlaybackDriver {
            engine,
            stream,
            state,
            events,
            bpm,
            is_playing: std::cell::Cell::new(false),
        })
    }

    pub fn play(&self) -> Result<()> {
        if !self.is_playing.get() {
            self.stream.play().context("Failed to start audio")?;
            self.is_playing.set(true);
        }
        Ok(())
    }

    pub fn pause(&self) -> Result<()> {
        if self.is_playing.get() {
            self.stream.pause().context("Failed to pause audio")?;
            self.is_playing.set(false);
        }
        Ok(())
    }

    pub fn stop(&self) -> Result<()> {
        if self.is_playing.get() {
            self.stream.pause().context("Failed to pause audio")?;
            self.is_playing.set(false);
        }
        self.state.reset();
        self.engine.lock().reset();
        Ok(())
    }

    pub fn current_bar(&self) -> u16 {
        self.state.current_bar(self.bpm)
    }
}
