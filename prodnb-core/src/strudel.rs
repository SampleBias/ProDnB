//! Maps PDB atoms to Strudel-style commands for Drum & Bass music.
//!
//! Dynamic mapping uses element pools, B-factor, occupancy, residue type, and chain index
//! for varied, deterministic patterns. See README "Dynamic Element Mapping" section.

use crate::protein::{Protein, AtomContext};
use crate::features::FeatureExtractor;
use crate::genre::{DnBGenre, GenreParams};
use anyhow::Result;
use serde::{Serialize, Deserialize};
use std::collections::HashMap;

/// Configurable thresholds for dynamic mapping (Phase 3).
#[derive(Debug, Clone)]
pub struct MappingConfig {
    /// B-factor above which "flex" variant may be used (default 40)
    pub b_factor_flex_threshold: f64,
    /// Occupancy below which rest may be used (default 0.25)
    pub occupancy_rest_threshold: f64,
    /// Deterministic seed for rest decision (0-2: 0=never, 1=1/3, 2=2/3 when below threshold)
    pub occupancy_rest_mod: u32,
}

impl Default for MappingConfig {
    fn default() -> Self {
        Self {
            b_factor_flex_threshold: 40.0,
            occupancy_rest_threshold: 0.25,
            occupancy_rest_mod: 1,
        }
    }
}

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

/// Base-sound pools for dynamic variation (Phase 1). Genre-aware base is rotated within pool.
fn sound_pool(base: &str) -> &'static [&'static str] {
    match base {
        "bd" => &["bd", "bd", "perc"],
        "sd" => &["sd", "sd", "cp"],
        "hh" => &["hh", "hh", "oh"],
        "cp" => &["cp", "cp", "rim"],
        "rim" => &["rim", "perc", "fx"],
        "perc" | "fx" => &["perc"],
        _ => &["perc"],
    }
}

/// Flex variants: high B-factor atoms may substitute (structural flexibility → variation).
const FLEX_VARIANTS: &[(&str, &str)] = &[
    ("bd", "perc"),
    ("sd", "cp"),
    ("hh", "oh"),
    ("cp", "rim"),
    ("rim", "perc"),
];

/// Residue-type bias for Phase 2: hydrophobic → low, charged → sharp, aromatic → distinct.
const HYDROPHOBIC_RESIDUES: &[&str] = &["ALA", "VAL", "LEU", "ILE", "MET", "PHE", "TRP", "PRO"];
const CHARGED_RESIDUES: &[&str] = &["ARG", "LYS", "ASP", "GLU"];
const AROMATIC_RESIDUES: &[&str] = &["PHE", "TYR", "TRP", "HIS"];

/// Maps atom element to Strudel sound name (default mapping).
pub fn element_to_sound(element: &str) -> &'static str {
    element_to_sound_for_genre(element, None)
}

/// Dynamic element-to-sound mapping (Phases 1–2). Uses element pools, B-factor, occupancy,
/// residue type, and chain index for deterministic variation.
pub fn element_to_sound_dynamic(
    ctx: &AtomContext,
    atom_index: usize,
    genre: Option<DnBGenre>,
    config: &MappingConfig,
) -> &'static str {
    let el = ctx.atom.element.to_uppercase();
    if el == "H" {
        return "~";
    }

    let seed = (atom_index as u64)
        .wrapping_add(ctx.atom.serial as u64)
        .wrapping_add(ctx.residue_seq as u64);

    // Occupancy-based rest (Phase 1): very low occupancy → deterministic rest
    if ctx.atom.occupancy < config.occupancy_rest_threshold {
        let occ_seed = (ctx.atom.serial as usize)
            .wrapping_add(ctx.residue_seq as usize)
            .wrapping_add(ctx.chain_index);
        if (occ_seed % 3) < config.occupancy_rest_mod as usize {
            return "~";
        }
    }

    // Base sound from genre-aware mapping
    let base = element_to_sound_for_genre(&ctx.atom.element, genre);

    // Element pool: rotate by atom_index for variation (Phase 1)
    let pool = sound_pool(base);
    let mut sound = pool[atom_index % pool.len()];

    // B-factor substitution (Phase 1): high flexibility → flex variant sometimes
    let use_flex = ctx.atom.b_factor > config.b_factor_flex_threshold && (seed % 4) == 0;
    if use_flex {
        sound = FLEX_VARIANTS
            .iter()
            .find(|(from, _)| *from == sound)
            .map(|(_, to)| *to)
            .unwrap_or(sound);
    }

    // Residue-type bias (Phase 2): shift toward distinct sounds for charged/aromatic
    let residue_upper = ctx.residue_name.to_uppercase();
    let residue_bias = if CHARGED_RESIDUES.iter().any(|r| residue_upper == *r) {
        Some("sd")
    } else if AROMATIC_RESIDUES.iter().any(|r| residue_upper == *r) {
        Some("cp")
    } else if HYDROPHOBIC_RESIDUES.iter().any(|r| residue_upper == *r) {
        Some("bd")
    } else {
        None
    };
    if let Some(bias) = residue_bias {
        if (seed % 5) == 0 && matches!(el.as_str(), "C" | "N" | "O") {
            return bias;
        }
    }

    // Chain-index rotation (Phase 2): different chains emphasize different families
    let chain_shift = ctx.chain_index % 3;
    if chain_shift == 1 && (seed % 7) < 2 && sound == "bd" {
        return "perc";
    }
    if chain_shift == 2 && (seed % 7) < 2 && sound == "hh" {
        return "oh";
    }

    sound
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

/// Builds a Strudel pattern from protein atoms (uses dynamic mapping).
pub fn protein_to_strudel(protein: &Protein, bpm: u16) -> String {
    let config = MappingConfig::default();
    let contexts: Vec<_> = protein
        .all_atoms_with_context()
        .filter(|ctx| ctx.atom.element.to_uppercase() != "H")
        .collect();
    if contexts.is_empty() {
        return default_strudel_code(bpm);
    }

    let step = (contexts.len() / 32).max(1).min(8);
    let sampled: Vec<String> = contexts
        .iter()
        .enumerate()
        .step_by(step)
        .take(32)
        .map(|(i, ctx)| element_to_sound_dynamic(ctx, i, None, &config).to_string())
        .collect();

    if sampled.is_empty() {
        return default_strudel_code(bpm);
    }

    let pattern = sampled.join(" ");
    let bars = (sampled.len() / 4).max(1);
    let total_atoms: usize = protein.all_atoms().count();

    format!(
        r#"// ProDnB Strudel code (from PDB) - Strudel JS mode
// {} atoms → {} sounds (dynamic mapping)

setcps({})

stack(
  s("{}"),
  s("hh*{}"),
  s("bd*4")
)
"#,
        total_atoms,
        sampled.len(),
        bpm as f64 / 60.0 / 4.0,
        pattern,
        bars,
    )
}

/// Alternative: per-chain patterns for polyrhythmic structure (uses dynamic mapping).
pub fn protein_to_strudel_layered(protein: &Protein, bpm: u16) -> String {
    let cps = bpm as f64 / 60.0 / 4.0;
    let config = MappingConfig::default();

    let mut chain_sounds: Vec<Vec<String>> = Vec::new();
    for (chain_idx, chain) in protein.chains.iter().enumerate() {
        let mut sounds = Vec::new();
        let mut atom_idx = 0;
        for residue in &chain.residues {
            for atom in &residue.atoms {
                if atom.element.to_uppercase() == "H" {
                    continue;
                }
                let ctx = AtomContext {
                    atom,
                    residue_name: &residue.name,
                    chain_id: &chain.id,
                    chain_index: chain_idx,
                    residue_seq: residue.sequence_number,
                };
                let s = element_to_sound_dynamic(&ctx, atom_idx, None, &config);
                sounds.push(s.to_string());
                atom_idx += 1;
                if sounds.len() >= 16 {
                    break;
                }
            }
            if sounds.len() >= 16 {
                break;
            }
        }
        if !sounds.is_empty() {
            chain_sounds.push(sounds);
        }
    }

    let parts: Vec<String> = chain_sounds
        .iter()
        .map(|sounds| format!(r#"  s("{}")"#, sounds.join(" ")))
        .collect();

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

    // Build rhythm seed from atoms (dynamic mapping: pools, B-factor, occupancy, residue, chain)
    let config = MappingConfig::default();
    let contexts: Vec<_> = protein
        .all_atoms_with_context()
        .filter(|ctx| ctx.atom.element.to_uppercase() != "H")
        .collect();
    let step = (contexts.len() / 24).max(1).min(4);
    let rhythm_seed: String = contexts
        .iter()
        .enumerate()
        .step_by(step)
        .take(24)
        .map(|(i, ctx)| {
            element_to_sound_dynamic(ctx, i, genre, &config).to_string()
        })
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
    let atoms: Vec<_> = protein.all_atoms().filter(|a| a.element.to_uppercase() != "H").collect();
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
            format!(r#"n("{}").scale("{}").s("triangle").gain({})"#, note_pat, scale, gain_expr)
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
    use crate::protein::Atom;

    #[test]
    fn test_element_mapping() {
        assert_eq!(element_to_sound("C"), "bd");
        assert_eq!(element_to_sound("N"), "sd");
        assert_eq!(element_to_sound("O"), "hh");
        assert_eq!(element_to_sound("S"), "cp");
    }

    #[test]
    fn test_dynamic_mapping_deterministic() {
        let atom = Atom {
            serial: 1,
            name: "CA".to_string(),
            element: "C".to_string(),
            x: 0.0, y: 0.0, z: 0.0,
            b_factor: 30.0,
            occupancy: 1.0,
        };
        let ctx = AtomContext {
            atom: &atom,
            residue_name: "ALA",
            chain_id: "A",
            chain_index: 0,
            residue_seq: 1,
        };
        let config = MappingConfig::default();
        let s = element_to_sound_dynamic(&ctx, 0, None, &config);
        assert!(matches!(s, "bd" | "perc"));
    }
}
