use anyhow::Result;
use prodnb_core::{DnBParameters, ArrangementPlan, composition::SectionConfig};
use patterns::TICKS_PER_BEAT;
use patterns::TICKS_PER_16TH;
use events::EventKind;

pub mod tracks;
pub mod patterns;
pub mod events;
pub mod strudel;

pub use tracks::{MidiTrack, StemType};
pub use patterns::{DrumPattern, BassPattern};
pub use events::{MidiEvent, NoteEvent, ControlEvent};
pub use strudel::{strudel_to_midi, strudel_to_playback_events};

pub struct MidiBuilder {
    tracks: Vec<MidiTrack>,
}

impl MidiBuilder {
    pub fn new() -> Self {
        MidiBuilder {
            tracks: Vec::new(),
        }
    }

    pub fn build_from_composition(&mut self, arrangement: &ArrangementPlan, params: &DnBParameters) -> Result<()> {
        self.generate_drum_track(arrangement, params)?;
        self.generate_bass_track(arrangement, params)?;
        self.generate_pad_track(arrangement, params)?;
        Ok(())
    }

    fn generate_drum_track(&mut self, arrangement: &ArrangementPlan, params: &DnBParameters) -> Result<()> {
        let mut track = MidiTrack::new(StemType::Drums);

        let drum_pattern = match params.style {
            prodnb_core::Style::Liquid => patterns::DrumPattern::liquid(),
            prodnb_core::Style::Jungle => patterns::DrumPattern::jungle(),
            prodnb_core::Style::Neuro => patterns::DrumPattern::neuro(),
        };

        for section in &arrangement.sections {
            self.add_drum_section(&mut track, &section, &drum_pattern, params)?;
        }

        track.sort_events();
        self.tracks.push(track);
        Ok(())
    }

    fn add_drum_section(&mut self, track: &mut MidiTrack, section: &SectionConfig, pattern: &patterns::DrumPattern, params: &DnBParameters) -> Result<()> {
        let section_start_ticks = section.start_bar as u32 * 4 * TICKS_PER_BEAT as u32;
        let velocity = 100;

        for bar in 0..section.bars {
            let bar_start_ticks = section_start_ticks + bar as u32 * 4 * TICKS_PER_BEAT as u32;

            for step in 0..16 {
                let step_ticks = bar_start_ticks + step as u32 * TICKS_PER_16TH as u32;

                if pattern.kick[step] {
                    track.add_note(step_ticks, 36, velocity, TICKS_PER_16TH as u32);
                }

                if pattern.snare[step] {
                    track.add_note(step_ticks, 38, velocity, TICKS_PER_16TH as u32);
                }

                if pattern.hihat[step] {
                    track.add_note(step_ticks, 42, 80, TICKS_PER_16TH as u32 / 2);
                }

                if pattern.perc[step] {
                    track.add_note(step_ticks, 45, 90, TICKS_PER_16TH as u32 / 2);
                }
            }
        }

        Ok(())
    }

    fn generate_bass_track(&mut self, arrangement: &ArrangementPlan, params: &DnBParameters) -> Result<()> {
        let mut track = MidiTrack::new(StemType::Bass);

        let bass_pattern = patterns::BassPattern::basic();
        let velocity = 110;

        for section in &arrangement.sections {
            self.add_bass_section(&mut track, &section, &bass_pattern, params, velocity)?;
        }

        track.sort_events();
        self.tracks.push(track);
        Ok(())
    }

    fn add_bass_section(&mut self, track: &mut MidiTrack, section: &SectionConfig, pattern: &patterns::BassPattern, params: &DnBParameters, velocity: u8) -> Result<()> {
        let section_start_ticks = section.start_bar as u32 * 4 * TICKS_PER_BEAT as u32;

        for bar in 0..section.bars {
            let bar_start_ticks = section_start_ticks + bar as u32 * 4 * TICKS_PER_BEAT as u32;

            for (i, &should_play) in pattern.rhythm.iter().enumerate() {
                if should_play {
                    let step_ticks = bar_start_ticks + i as u32 * TICKS_PER_16TH as u32;
                    let note_idx = i % pattern.notes.len();
                    track.add_note(step_ticks, pattern.notes[note_idx], velocity, 2 * TICKS_PER_16TH as u32);
                }
            }
        }

        Ok(())
    }

    fn generate_pad_track(&mut self, arrangement: &ArrangementPlan, params: &DnBParameters) -> Result<()> {
        let mut track = MidiTrack::new(StemType::Pad);

        let pad_notes = vec![48, 52, 55, 60];
        let velocity = 70;
        let duration = 4 * TICKS_PER_BEAT as u32;

        for section in &arrangement.sections {
            let section_start_ticks = section.start_bar as u32 * 4 * TICKS_PER_BEAT as u32;

            for bar in 0..section.bars {
                let bar_ticks = section_start_ticks + bar as u32 * 4 * TICKS_PER_BEAT as u32;
                let note_idx = (bar as usize) % pad_notes.len();

                track.add_note(bar_ticks, pad_notes[note_idx], velocity, duration);
            }
        }

        track.sort_events();
        self.tracks.push(track);
        Ok(())
    }

    pub fn tracks(&self) -> &[MidiTrack] {
        &self.tracks
    }
}

impl Default for MidiBuilder {
    fn default() -> Self {
        Self::new()
    }
}
