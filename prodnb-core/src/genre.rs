//! DnB subgenre model and parameters for genre-aware mapping.

use serde::{Deserialize, Serialize};

/// Drum & Bass subgenres with distinct sound characteristics.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum DnBGenre {
    /// Soulful, melodic - pads, soft hats, melodic bass
    Liquid,
    /// High-energy, wobble - aggressive basses, punchy
    JumpUp,
    /// Dark, techy - industrial, metallic, reese
    Neurofunk,
    /// Anthemic, mainstream - big kicks, catchy
    Dancefloor,
    /// Breakbeat-heavy roots - amen breaks, ragga
    Jungle,
}

impl DnBGenre {
    pub fn from_str(s: &str) -> Option<Self> {
        match s.to_lowercase().replace('-', "_").as_str() {
            "liquid" => Some(DnBGenre::Liquid),
            "jump_up" | "jumpup" => Some(DnBGenre::JumpUp),
            "neurofunk" => Some(DnBGenre::Neurofunk),
            "dancefloor" => Some(DnBGenre::Dancefloor),
            "jungle" => Some(DnBGenre::Jungle),
            _ => None,
        }
    }

    pub fn as_str(&self) -> &'static str {
        match self {
            DnBGenre::Liquid => "liquid",
            DnBGenre::JumpUp => "jump_up",
            DnBGenre::Neurofunk => "neurofunk",
            DnBGenre::Dancefloor => "dancefloor",
            DnBGenre::Jungle => "jungle",
        }
    }

    pub fn display_name(&self) -> &'static str {
        match self {
            DnBGenre::Liquid => "Liquid",
            DnBGenre::JumpUp => "Jump Up",
            DnBGenre::Neurofunk => "Neurofunk",
            DnBGenre::Dancefloor => "Dancefloor",
            DnBGenre::Jungle => "Jungle",
        }
    }
}

/// Genre and tonal parameters for mapping and LLM arrangement.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GenreParams {
    pub genre: DnBGenre,
    /// Musical key, e.g. "C", "Am", "F#m"
    #[serde(default)]
    pub key: Option<String>,
    /// Octave range 2–5 for melodic content
    #[serde(default)]
    pub octave: Option<u8>,
    /// Include note/scale layers (melodic content)
    #[serde(default = "default_melodic")]
    pub melodic: bool,
}

fn default_melodic() -> bool {
    false
}

impl GenreParams {
    pub fn new(genre: DnBGenre) -> Self {
        Self {
            genre,
            key: None,
            octave: None,
            melodic: false,
        }
    }

    pub fn with_key(mut self, key: impl Into<String>) -> Self {
        self.key = Some(key.into());
        self
    }

    pub fn with_octave(mut self, octave: u8) -> Self {
        self.octave = Some(octave.clamp(2, 5));
        self
    }

    pub fn with_melodic(mut self, melodic: bool) -> Self {
        self.melodic = melodic;
        self
    }
}
