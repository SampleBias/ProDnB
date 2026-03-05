//! Maps PDB atoms to Strudel-style commands for Drum & Bass music.
//!
//! Element mapping:
//! - C (Carbon)  → bd (bass drum)
//! - N (Nitrogen)→ sd (snare)
//! - O (Oxygen)  → hh (hi-hat)
//! - S (Sulfur)  → cp (clap)
//! - P (Phosphorus) → rim
//! - Other       → perc

use crate::protein::{Protein, Atom};
use crate::features::FeatureExtractor;
use crate::genre::{DnBGenre, GenreParams};
use anyhow::Result;
use serde::{Serialize, Deserialize};
use std::collections::HashMap;

/// A single Strudel primitive (drum pattern or euclidean rhythm).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StrudelPrimitive {
    #[serde(rename = "type")]
    pub primitive_type: String,
    pub pattern: Option<String>,
    pub sound: Option<String>,
    pub beats: Option<u8>,
    pub segments: Option<u8>,
    #[serde(default = "default_gain")]
    pub gain: f64,
    pub layer: Option<String>,
    /// Scale for melodic layers, e.g. "C:minor"
    #[serde(default)]
    pub scale: Option<String>,
    /// Octave for melodic content (2–5)
    #[serde(default)]
    pub octave: Option<u8>,
    /// Note pattern for melodic layers, e.g. "0 2 4 6"
    #[serde(default)]
    pub note_pattern: Option<String>,
}

fn default_gain() -> f64 {
    0.8
}

/// Output of deterministic PDB-to-Strudel mapping for LLM arrangement.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MappedOutput {
    pub tempo: u16,
    pub primitives: Vec<StrudelPrimitive>,
    pub rhythm_seed: String,
    pub chain_lengths: Vec<usize>,
    pub element_counts: HashMap<String, usize>,
    /// Genre for LLM arrangement context
    #[serde(default)]
    pub genre: Option<String>,
    /// Musical key, e.g. "C", "Am"
    #[serde(default)]
    pub key: Option<String>,
    /// Octave for melodic content (2–5)
    #[serde(default)]
    pub octave: Option<u8>,
    /// Include melodic layers
    #[serde(default)]
    pub melodic: bool,
}

/// Maps atom element to Strudel sound name (default mapping).
pub fn element_to_sound(element: &str) -> &'static str {
    element_to_sound_for_genre(element, None)
}

/// Genre-aware element-to-sound mapping.
/// - Liquid: softer hats, pad-like perc
/// - Jump Up: heavier bd, aggressive sd
/// - Neurofunk: metallic/industrial (rim, fx)
/// - Dancefloor: classic bd/sd/hh
/// - Jungle: break-style perc (cp, rim), amen density
pub fn element_to_sound_for_genre(element: &str, genre: Option<DnBGenre>) -> &'static str {
    let el = element.to_uppercase();
    if el == "H" {
        return "~";
    }
    match genre {
        Some(DnBGenre::Liquid) => match el.as_str() {
            "C" => "bd",
            "N" => "sd",
            "O" => "hh",  // softer in arrangement
            "S" => "cp",
            "P" => "perc",  // pad-like
            _ => "perc",
        },
        Some(DnBGenre::JumpUp) => match el.as_str() {
            "C" => "bd",
            "N" => "sd",
            "O" => "hh",
            "S" => "cp",
            "P" => "perc",  // wobble hints
            _ => "perc",
        },
        Some(DnBGenre::Neurofunk) => match el.as_str() {
            "C" => "bd",
            "N" => "sd",
            "O" => "hh",
            "S" => "rim",  // metallic
            "P" => "fx",   // industrial
            _ => "perc",
        },
        Some(DnBGenre::Dancefloor) => match el.as_str() {
            "C" => "bd",
            "N" => "sd",
            "O" => "hh",
            "S" => "cp",
            "P" => "rim",
            _ => "perc",
        },
        Some(DnBGenre::Jungle) => match el.as_str() {
            "C" => "bd",
            "N" => "sd",
            "O" => "hh",
            "S" => "cp",
            "P" => "rim",  // break-style
            _ => "perc",
        },
        None => match el.as_str() {
            "C" => "bd",
            "N" => "sd",
            "O" => "hh",
            "S" => "cp",
            "P" => "rim",
            _ => "perc",
        },
    }
}

/// Builds a Strudel pattern from protein atoms.
/// Samples atoms along the sequence and maps elements to sounds.
pub fn protein_to_strudel(protein: &Protein, bpm: u16) -> String {
    let atoms: Vec<&Atom> = protein.all_atoms().collect();
    if atoms.is_empty() {
        return default_strudel_code(bpm);
    }

    // Sample atoms (every Nth to avoid huge patterns); skip H
    let step = (atoms.len() / 32).max(1).min(8);
    let sampled: Vec<&str> = atoms
        .iter()
        .step_by(step)
        .filter(|a| a.element.to_uppercase() != "H")
        .take(32)
        .map(|a| element_to_sound(&a.element))
        .collect();

    if sampled.is_empty() {
        return default_strudel_code(bpm);
    }

    // Build pattern string: "bd sd hh bd ..."
    let pattern = sampled.join(" ");
    let bars = (sampled.len() / 4).max(1);

    format!(
        r#"// ProDnB Strudel code (from PDB) - Strudel JS mode
// {} atoms → {} sounds

setcps({})

stack(
  s("{}"),
  s("hh*{}"),
  s("bd*4")
)
"#,
        atoms.len(),
        sampled.len(),
        bpm as f64 / 60.0 / 4.0,
        pattern,
        bars,
    )
}

/// Alternative: per-chain patterns for polyrhythmic structure.
pub fn protein_to_strudel_layered(protein: &Protein, bpm: u16) -> String {
    let mut parts = Vec::new();
    let cps = bpm as f64 / 60.0 / 4.0;

    for chain in protein.chains.iter() {
        let sounds: Vec<&str> = chain
            .residues
            .iter()
            .flat_map(|r| r.atoms.iter())
            .filter(|a| a.element.to_uppercase() != "H")
            .take(16)
            .map(|a| element_to_sound(&a.element))
            .collect();

        if sounds.is_empty() {
            continue;
        }
        let pattern = sounds.join(" ");
        parts.push(format!(r#"  s("{}")"#, pattern));
    }

    if parts.is_empty() {
        return default_strudel_code(bpm);
    }

    format!(
        r#"// ProDnB layered Strudel (chains → stack) - Strudel JS mode
setcps({})

stack(
{}
)
"#,
        cps,
        parts.join(",\n")
    )
}

/// Deterministic mapping: PDB protein → structured Strudel primitives JSON.
/// B-factor variance → euclidean (beats,segments); occupancy → gain; chain length → density.
/// Genre params adjust element-to-sound mapping, euclidean density, and optional melodic layers.
pub fn protein_to_primitives(
    protein: &Protein,
    bpm: u16,
    genre_params: Option<&GenreParams>,
) -> Result<MappedOutput> {
    let features = FeatureExtractor::extract(protein)?;
    let genre = genre_params.map(|g| g.genre);

    let mut element_counts: HashMap<String, usize> = HashMap::new();
    for atom in protein.all_atoms() {
        let el = atom.element.to_uppercase();
        if el != "H" {
            *element_counts.entry(el).or_insert(0) += 1;
        }
    }

    let chain_lengths: Vec<usize> = protein.chains.iter()
        .map(|c| c.residues.len())
        .collect();

    // B-factor variance → euclidean rhythm; genre adjusts density (Jungle denser, Liquid sparser)
    let (base_beats, base_segments): (u8, u8) = if features.b_factor_variance > 50.0 {
        (5, 8)
    } else if features.b_factor_variance > 20.0 {
        (4, 8)
    } else {
        (3, 8)
    };
    let (beats, segments) = match genre {
        Some(DnBGenre::Jungle) => ((base_beats + 1).min(7), base_segments.max(8)),
        Some(DnBGenre::Liquid) => ((base_beats.saturating_sub(1)).max(2), base_segments),
        _ => (base_beats, base_segments),
    };

    // Build rhythm seed from atoms (genre-aware element mapping)
    let atoms: Vec<_> = protein.all_atoms()
        .filter(|a| a.element.to_uppercase() != "H")
        .collect();
    let step = (atoms.len() / 24).max(1).min(4);
    let rhythm_seed: String = atoms.iter()
        .step_by(step)
        .take(24)
        .map(|a| element_to_sound_for_genre(&a.element, genre).to_string())
        .collect::<Vec<_>>()
        .join(" ");

    let genre_str = genre.map(|g| g.as_str().to_string());
    let key = genre_params.and_then(|g| g.key.clone());
    let octave = genre_params.and_then(|g| g.octave);
    let melodic = genre_params.map(|g| g.melodic).unwrap_or(false);

    if rhythm_seed.is_empty() {
        return Ok(MappedOutput {
            tempo: bpm,
            primitives: default_primitives(bpm, genre),
            rhythm_seed: "bd sd hh bd ~ sd".to_string(),
            chain_lengths: vec![4],
            element_counts,
            genre: genre_str,
            key,
            octave,
            melodic,
        });
    }

    // Avg occupancy for gain (0.1–1.0)
    let avg_occupancy: f64 = atoms.iter()
        .map(|a| a.occupancy)
        .sum::<f64>() / atoms.len().max(1) as f64;
    let base_gain = (0.5 + avg_occupancy * 0.5).clamp(0.3, 1.0);

    // Chain length → speed multiplier for hi-hat density
    let max_chain = chain_lengths.iter().copied().max().unwrap_or(8);
    let hat_mult = ((max_chain / 4).max(1).min(8)) as u8;

    let mut primitives = Vec::new();

    // Kick: bd with euclidean
    primitives.push(StrudelPrimitive {
        primitive_type: "euclidean".to_string(),
        pattern: None,
        sound: Some("bd".to_string()),
        beats: Some(beats),
        segments: Some(segments),
        gain: base_gain * 0.95,
        layer: Some("kick".to_string()),
        scale: None,
        octave: None,
        note_pattern: None,
    });

    // Snare: classic DnB on 2 and 4
    primitives.push(StrudelPrimitive {
        primitive_type: "drum".to_string(),
        pattern: Some("sd ~ ~ sd".to_string()),
        sound: None,
        beats: None,
        segments: None,
        gain: base_gain * 0.9,
        layer: Some("snare".to_string()),
        scale: None,
        octave: None,
        note_pattern: None,
    });

    // Hi-hats: 16ths from chain length
    primitives.push(StrudelPrimitive {
        primitive_type: "drum".to_string(),
        pattern: Some(format!("hh*{}", hat_mult)),
        sound: None,
        beats: None,
        segments: None,
        gain: base_gain * 0.6,
        layer: Some("hats".to_string()),
        scale: None,
        octave: None,
        note_pattern: None,
    });

    // Optional: rhythm from protein as additional layer
    if !rhythm_seed.is_empty() && element_counts.get("C").copied().unwrap_or(0) > 0 {
        primitives.push(StrudelPrimitive {
            primitive_type: "drum".to_string(),
            pattern: Some(rhythm_seed.clone()),
            sound: None,
            beats: None,
            segments: None,
            gain: base_gain * 0.5,
            layer: Some("perc".to_string()),
            scale: None,
            octave: None,
            note_pattern: None,
        });
    }

    // Optional: melodic layer (Liquid, Dancefloor)
    if melodic && matches!(genre, Some(DnBGenre::Liquid) | Some(DnBGenre::Dancefloor)) {
        let scale_key = key.as_deref().unwrap_or("C");
        // "Am" -> "A:minor", "C" -> "C:minor"
        let root = scale_key.trim_end_matches('m');
        let scale = format!("{}:minor", root);
        let oct = octave.unwrap_or(3);
        primitives.push(StrudelPrimitive {
            primitive_type: "melodic".to_string(),
            pattern: None,
            sound: None,
            beats: None,
            segments: None,
            gain: base_gain * 0.4,
            layer: Some("melodic".to_string()),
            scale: Some(scale),
            octave: Some(oct),
            note_pattern: Some("0 2 4 6 4 2".to_string()),
        });
    }

    Ok(MappedOutput {
        tempo: bpm,
        primitives,
        rhythm_seed,
        chain_lengths,
        element_counts,
        genre: genre_str,
        key,
        octave,
        melodic,
    })
}

fn default_primitives(_bpm: u16, _genre: Option<DnBGenre>) -> Vec<StrudelPrimitive> {
    vec![
        StrudelPrimitive {
            primitive_type: "euclidean".to_string(),
            pattern: None,
            sound: Some("bd".to_string()),
            beats: Some(5),
            segments: Some(8),
            gain: 0.9,
            layer: Some("kick".to_string()),
            scale: None,
            octave: None,
            note_pattern: None,
        },
        StrudelPrimitive {
            primitive_type: "drum".to_string(),
            pattern: Some("sd ~ ~ sd".to_string()),
            sound: None,
            beats: None,
            segments: None,
            gain: 0.85,
            layer: Some("snare".to_string()),
            scale: None,
            octave: None,
            note_pattern: None,
        },
        StrudelPrimitive {
            primitive_type: "drum".to_string(),
            pattern: Some("hh*8".to_string()),
            sound: None,
            beats: None,
            segments: None,
            gain: 0.6,
            layer: Some("hats".to_string()),
            scale: None,
            octave: None,
            note_pattern: None,
        },
    ]
}

/// Slider values for intensity control (kick, snare, hats, energy).
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct SliderValues {
    pub kick: Option<f64>,
    pub snare: Option<f64>,
    pub hats: Option<f64>,
    pub energy: Option<f64>,
}

/// Assemble Strudel code from primitives. Intensity controls use slider() for Strudel.cc UI.
/// Piano roll patterns are in the stack output.
pub fn assemble_strudel(mapped: &MappedOutput, sliders: Option<&SliderValues>) -> String {
    let cps = mapped.tempo as f64 / 60.0 / 4.0;

    let mut parts = Vec::new();
    for p in &mapped.primitives {
        let gain = if let Some(s) = sliders {
            let base = match p.layer.as_deref() {
                Some("kick") => s.kick.unwrap_or(p.gain),
                Some("snare") => s.snare.unwrap_or(p.gain),
                Some("hats") => s.hats.unwrap_or(p.gain),
                _ => p.gain,
            };
            s.energy.map(|e| base * e).unwrap_or(base)
        } else {
            p.gain
        };
        // slider(value, min, max) - Strudel.cc adds interactive sliders in the REPL
        let gain_expr = format!("slider({:.2}, 0, 1)", gain);

        let pattern = if p.primitive_type == "euclidean" {
            if let (Some(sound), Some(beats), Some(segments)) = (p.sound.as_ref(), p.beats, p.segments) {
                let eucl = format!("{}({},{})", sound, beats, segments);
                format!(r#"s("{}").gain({})"#, eucl, gain_expr)
            } else {
                continue;
            }
        } else if p.primitive_type == "melodic" {
            let scale = p.scale.as_deref().unwrap_or("C:minor");
            let note_pat = p.note_pattern.as_deref().unwrap_or("0 2 4 6");
            format!(r#"n("{}").scale("{}").s("sawtooth").gain({})"#, note_pat, scale, gain_expr)
        } else if let Some(ref pat) = p.pattern {
            format!(r#"s("{}").gain({})"#, pat, gain_expr)
        } else {
            continue;
        };
        parts.push(pattern);
    }

    // JS mode: stack(p1, p2, ...) variadic - no d1 $, no array (Tidal syntax breaks in Strudel default REPL)
    let stack_body = parts.join(",\n  ");
    format!(
        r#"// ProDnB assembled from primitives (Strudel JS mode)
// Intensity: slider() adds controls in Strudel.cc UI.
setcps({})

stack(
  {}
)
"#,
        cps.max(0.1),
        stack_body
    )
}

/// Returns default Strudel code when no protein is loaded.
pub fn default_strudel_code(bpm: u16) -> String {
    let cps = bpm as f64 / 60.0 / 4.0;
    format!(
        r#"// ProDnB default (no protein loaded) - Strudel JS mode
setcps({})

stack(
  s("bd*4"),
  s("sd ~ ~ sd"),
  s("hh*8")
)
"#,
        cps
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_element_mapping() {
        assert_eq!(element_to_sound("C"), "bd");
        assert_eq!(element_to_sound("N"), "sd");
        assert_eq!(element_to_sound("O"), "hh");
        assert_eq!(element_to_sound("S"), "cp");
    }
}
