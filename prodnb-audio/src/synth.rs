use rustysynth::{SoundFont, Synthesizer};
use prodnb_midi::{MidiEvent, events::EventKind};
use std::sync::Arc;
use anyhow::{Result, Context};

const SAMPLE_RATE: i32 = 44100;
const BUFFER_SIZE: usize = 512;

pub struct AudioEngine {
    synthesizer: Synthesizer,
    left_buffer: Vec<f32>,
    right_buffer: Vec<f32>,
    output_buffer: Vec<f32>,
}

impl AudioEngine {
    pub fn new(soundfont_path: &str) -> Result<Self> {
        let mut file = std::fs::File::open(soundfont_path)?;
        let sound_font = Arc::new(SoundFont::new(&mut file)?);

        let mut settings = rustysynth::SynthesizerSettings::new(SAMPLE_RATE);
        settings.block_size = 64;

        let synthesizer = Synthesizer::new(&sound_font, &settings)
            .context("Failed to create synthesizer")?;

        Ok(AudioEngine {
            synthesizer,
            left_buffer: vec![0.0; BUFFER_SIZE],
            right_buffer: vec![0.0; BUFFER_SIZE],
            output_buffer: vec![0.0; BUFFER_SIZE * 2],
        })
    }

    pub fn render_block(&mut self) -> &[f32] {
        self.synthesizer.render(&mut self.left_buffer, &mut self.right_buffer);

        for i in 0..BUFFER_SIZE {
            self.output_buffer[i * 2] = self.left_buffer[i];
            self.output_buffer[i * 2 + 1] = self.right_buffer[i];
        }

        &self.output_buffer
    }

    pub fn process_midi_events(&mut self, events: &[MidiEvent], _sample_offset: u32) {
        for event in events {
            self.process_midi_event(event);
        }
    }

    fn process_midi_event(&mut self, event: &MidiEvent) {
        match &event.kind {
            EventKind::NoteOn(note) => {
                let _ = self.synthesizer.note_on(0, note.note as i32, note.velocity as i32);
            }
            EventKind::NoteOff(note) => {
                let _ = self.synthesizer.note_off(0, note.note as i32);
            }
            _ => {}
        }
    }

    pub fn reset(&mut self) {
        self.synthesizer.reset();
    }

    pub fn set_master_volume(&mut self, volume: f32) {
        self.synthesizer.set_master_volume(volume.max(0.0).min(1.0));
    }
}

pub struct SoundFontLoader;

impl SoundFontLoader {
    pub fn load_default() -> Result<Arc<SoundFont>> {
        let mut file = std::fs::File::open("assets/default.sf2")?;
        Ok(Arc::new(SoundFont::new(&mut file)?))
    }

    pub fn load_from_file(path: &str) -> Result<Arc<SoundFont>> {
        let mut file = std::fs::File::open(path)?;
        Ok(Arc::new(SoundFont::new(&mut file)?))
    }
}
