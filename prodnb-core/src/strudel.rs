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
}

/// Maps atom element to Strudel sound name.
pub fn element_to_sound(element: &str) -> &'static str {
    match element.to_uppercase().as_str() {
        "C" => "bd",
        "N" => "sd",
        "O" => "hh",
        "S" => "cp",
        "P" => "rim",
        "H" => "~",  // rest/silence for hydrogen (too many)
        _ => "perc",
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
        r#"// ProDnB Strudel code (from PDB)
// {} atoms → {} sounds

setcps({})

d1 $ stack [
  sound "{}"
  sound "hh*{}"
  sound "bd*4"
]
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

    for (i, chain) in protein.chains.iter().enumerate() {
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
        let d = format!("d{}", (i % 4) + 1);
        parts.push(format!(r#"  {} $ sound "{}""#, d, pattern));
    }

    if parts.is_empty() {
        return default_strudel_code(bpm);
    }

    format!(
        r#"// ProDnB layered Strudel (chains → d1,d2,d3,d4)
setcps({})

{}
"#,
        cps,
        parts.join("\n")
    )
}

/// Deterministic mapping: PDB protein → structured Strudel primitives JSON.
/// B-factor variance → euclidean (beats,segments); occupancy → gain; chain length → density.
pub fn protein_to_primitives(protein: &Protein, bpm: u16) -> Result<MappedOutput> {
    let features = FeatureExtractor::extract(protein)?;

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

    // B-factor variance → euclidean rhythm: high variance → (5,8), low → (3,4)
    let (beats, segments) = if features.b_factor_variance > 50.0 {
        (5, 8)
    } else if features.b_factor_variance > 20.0 {
        (4, 8)
    } else {
        (3, 8)
    };

    // Build rhythm seed from atoms
    let atoms: Vec<_> = protein.all_atoms()
        .filter(|a| a.element.to_uppercase() != "H")
        .collect();
    let step = (atoms.len() / 24).max(1).min(4);
    let rhythm_seed: String = atoms.iter()
        .step_by(step)
        .take(24)
        .map(|a| element_to_sound(&a.element).to_string())
        .collect::<Vec<_>>()
        .join(" ");

    if rhythm_seed.is_empty() {
        return Ok(MappedOutput {
            tempo: bpm,
            primitives: default_primitives(bpm),
            rhythm_seed: "bd sd hh bd ~ sd".to_string(),
            chain_lengths: vec![4],
            element_counts,
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
        });
    }

    Ok(MappedOutput {
        tempo: bpm,
        primitives,
        rhythm_seed,
        chain_lengths,
        element_counts,
    })
}

fn default_primitives(bpm: u16) -> Vec<StrudelPrimitive> {
    vec![
        StrudelPrimitive {
            primitive_type: "euclidean".to_string(),
            pattern: None,
            sound: Some("bd".to_string()),
            beats: Some(5),
            segments: Some(8),
            gain: 0.9,
            layer: Some("kick".to_string()),
        },
        StrudelPrimitive {
            primitive_type: "drum".to_string(),
            pattern: Some("sd ~ ~ sd".to_string()),
            sound: None,
            beats: None,
            segments: None,
            gain: 0.85,
            layer: Some("snare".to_string()),
        },
        StrudelPrimitive {
            primitive_type: "drum".to_string(),
            pattern: Some("hh*8".to_string()),
            sound: None,
            beats: None,
            segments: None,
            gain: 0.6,
            layer: Some("hats".to_string()),
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

/// Assemble Strudel code from primitives + optional slider values, no LLM.
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

        let pattern = if p.primitive_type == "euclidean" {
            if let (Some(sound), Some(beats), Some(segments)) = (p.sound.as_ref(), p.beats, p.segments) {
                let eucl = format!("{}({},{})", sound, beats, segments);
                format!(r#"s("{}").gain({})"#, eucl, gain)
            } else {
                continue;
            }
        } else if let Some(ref pat) = p.pattern {
            format!(r#"s("{}").gain({})"#, pat, gain)
        } else {
            continue;
        };
        parts.push(pattern);
    }

    let stack_body = parts.join(",\n  ");
    format!(
        r#"// ProDnB assembled from primitives
setcps({})

d1 $ stack [
  {}
]
"#,
        cps
            .max(0.1),
        stack_body
    )
}

/// Returns default Strudel code when no protein is loaded.
pub fn default_strudel_code(bpm: u16) -> String {
    let cps = bpm as f64 / 60.0 / 4.0;
    format!(
        r#"// ProDnB default (no protein loaded)
setcps({})

d1 $ stack [
  sound "bd*4"
  sound "sd ~ ~ sd"
  sound "hh*8"
]
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
