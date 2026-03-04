pub const TICKS_PER_BEAT: u16 = 480;
pub const TICKS_PER_16TH: u16 = TICKS_PER_BEAT / 4;

#[derive(Debug, Clone)]
pub struct DrumPattern {
    pub kick: [bool; 16],
    pub snare: [bool; 16],
    pub hihat: [bool; 16],
    pub perc: [bool; 16],
}

impl DrumPattern {
    pub fn basic_dnb() -> Self {
        DrumPattern {
            kick: [true, false, false, false,  true, false, false, false,
                   false, false, false, false, false, false, false, false],
            snare: [false, false, false, false, true, false, false, false,
                    false, false, false, false, true, false, false, false],
            hihat: [true, false, true, false, true, false, true, false,
                    true, false, true, false, true, false, true, false],
            perc: [false; 16],
        }
    }

    pub fn liquid() -> Self {
        let mut pattern = Self::basic_dnb();
        pattern.perc = [false, true, false, false, false, true, false, true,
                        false, true, false, false, false, true, false, true];
        pattern
    }

    pub fn jungle() -> Self {
        let mut pattern = Self::basic_dnb();
        pattern.kick[1] = true;
        pattern.kick[3] = true;
        pattern.kick[13] = true;
        pattern.kick[15] = true;
        pattern
    }

    pub fn neuro() -> Self {
        let mut pattern = Self::basic_dnb();
        pattern.kick[1] = true;
        pattern.kick[6] = true;
        pattern.kick[11] = true;
        pattern.kick[13] = true;
        pattern.snare[5] = true;
        pattern.snare[10] = true;
        pattern
    }
}

#[derive(Debug, Clone)]
pub struct BassPattern {
    pub notes: Vec<u8>,
    pub rhythm: Vec<bool>,
}

impl BassPattern {
    pub fn basic() -> Self {
        BassPattern {
            notes: vec![36, 36, 39, 36],
            rhythm: vec![true, false, true, false, true, false, true, false,
                         true, false, true, false, true, false, true, false],
        }
    }

    pub fn sub_only() -> Self {
        BassPattern {
            notes: vec![36, 36, 36, 36],
            rhythm: vec![true, false, false, false, true, false, false, false,
                         true, false, false, false, true, false, false, false],
        }
    }

    pub fn rolling() -> Self {
        BassPattern {
            notes: vec![36, 39, 41, 36],
            rhythm: vec![true, false, true, false, true, false, true, false,
                         true, false, true, false, true, false, true, false],
        }
    }
}
