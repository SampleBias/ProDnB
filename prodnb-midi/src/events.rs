use serde::{Serialize, Deserialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MidiEvent {
    pub start_ticks: u32,
    pub kind: EventKind,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum EventKind {
    NoteOn(NoteEvent),
    NoteOff(NoteEvent),
    ControlChange(ControlEvent),
    ProgramChange(ProgramChangeEvent),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NoteEvent {
    pub note: u8,
    pub velocity: u8,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ControlEvent {
    pub controller: u8,
    pub value: u8,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProgramChangeEvent {
    pub program: u8,
}

impl MidiEvent {
    pub fn new(start_ticks: u32, kind: EventKind) -> Self {
        MidiEvent {
            start_ticks,
            kind,
        }
    }

    pub fn start_ticks(&self) -> u32 {
        self.start_ticks
    }
}

impl From<(u32, EventKind)> for MidiEvent {
    fn from((start_ticks, kind): (u32, EventKind)) -> Self {
        MidiEvent { start_ticks, kind }
    }
}

impl NoteEvent {
    pub fn new(note: u8, velocity: u8) -> Self {
        NoteEvent {
            note: note.min(127),
            velocity: velocity.min(127),
        }
    }
}

impl ControlEvent {
    pub fn new(controller: u8, value: u8) -> Self {
        ControlEvent {
            controller: controller.min(127),
            value: value.min(127),
        }
    }
}

impl ProgramChangeEvent {
    pub fn new(program: u8) -> Self {
        ProgramChangeEvent {
            program: program.min(127),
        }
    }
}
