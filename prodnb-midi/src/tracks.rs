use crate::events::MidiEvent;
use crate::events::NoteEvent;
use crate::events::ControlEvent;
use crate::events::EventKind;
use serde::{Serialize, Deserialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum StemType {
    Drums,
    Bass,
    Pad,
    Lead,
    Percussion,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MidiTrack {
    pub stem_type: StemType,
    pub channel: u8,
    pub events: Vec<MidiEvent>,
}

impl MidiTrack {
    pub fn new(stem_type: StemType) -> Self {
        let channel = match stem_type {
            StemType::Drums => 9,
            StemType::Bass => 0,
            StemType::Pad => 1,
            StemType::Lead => 2,
            StemType::Percussion => 3,
        };

        MidiTrack {
            stem_type,
            channel,
            events: Vec::new(),
        }
    }

    pub fn add_note(&mut self, start_ticks: u32, note: u8, velocity: u8, duration_ticks: u32) {
        let note_on = MidiEvent::new(
            start_ticks,
            EventKind::NoteOn(NoteEvent::new(note, velocity))
        );

        let note_off = MidiEvent::new(
            start_ticks + duration_ticks,
            EventKind::NoteOff(NoteEvent::new(note, velocity))
        );

        self.events.push(note_on);
        self.events.push(note_off);
    }

    pub fn add_controller(&mut self, ticks: u32, controller: u8, value: u8) {
        let event = MidiEvent::new(
            ticks,
            EventKind::ControlChange(ControlEvent::new(controller, value))
        );
        self.events.push(event);
    }

    pub fn sort_events(&mut self) {
        self.events.sort_by_key(|e| e.start_ticks());
    }
}
