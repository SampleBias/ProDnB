//! Maps PDB atoms to Strudel-style commands for Drum & Bass music.
//!
//! Dynamic mapping uses element pools, B-factor, occupancy, residue type, and chain index
//! for varied, deterministic patterns. See README "Dynamic Element Mapping" section.

use crate::protein::{Protein, AtomContext};
use crate::features::{FeatureExtractor, StructuralFingerprint};
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
    /// Per-step gain in Strudel mini-notation, e.g. "0.8 0.5 0.9 0.3"
    #[serde(default)]
    pub gain_pattern: Option<String>,
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
    /// Backbone angle variance → rhythmic swing amount (0.0 = straight, 1.0 = max)
    #[serde(default)]
    pub swing: f64,
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
        Some(DnBGenre::Trance) => match el.as_str() {
            "C" => "bd",   // four-on-the-floor foundation
            "N" => "cp",   // clap instead of snare (trance convention)
            "O" => "oh",   // open hats for drive
            "S" => "hh",   // closed hat texture
            "P" => "rim",  // accent
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

/// Build a distance-modulated rhythm seed: backbone geometry controls hit density.
/// Short inter-CA distances (helix) → subdivided hits, long distances (coil) → rests.
/// Returns (pattern, gain_pattern) where gain follows the B-factor contour.
fn build_distance_modulated_seed(
    protein: &Protein,
    fingerprint: &StructuralFingerprint,
    genre: Option<DnBGenre>,
    config: &MappingConfig,
    target_steps: usize,
) -> (String, String) {
    let contexts: Vec<_> = protein
        .all_atoms_with_context()
        .filter(|ctx| ctx.atom.element.to_uppercase() != "H")
        .collect();

    if contexts.is_empty() || fingerprint.distance_rhythm.is_empty() {
        return ("bd sd hh bd ~ sd".to_string(), "0.8".to_string());
    }

    let n_dist = fingerprint.distance_rhythm.len();
    let n_bfac = fingerprint.b_factor_contour.len();
    let n_ctx = contexts.len();

    let mut pattern_parts = Vec::new();
    let mut gain_parts = Vec::new();

    for i in 0..target_steps {
        let t = i as f64 / target_steps as f64;

        let dist_idx = ((t * n_dist as f64) as usize).min(n_dist.saturating_sub(1));
        let distance = fingerprint.distance_rhythm[dist_idx];

        let bfac_idx = ((t * n_bfac as f64) as usize).min(n_bfac.saturating_sub(1));
        let bfactor = fingerprint.b_factor_contour[bfac_idx];

        let ctx_idx = ((t * n_ctx as f64) as usize).min(n_ctx - 1);
        let sound = element_to_sound_dynamic(&contexts[ctx_idx], ctx_idx, genre, config);

        if distance < 0.15 {
            let next_idx = (ctx_idx + 1).min(n_ctx - 1);
            let sound2 = element_to_sound_dynamic(&contexts[next_idx], next_idx, genre, config);
            pattern_parts.push(format!("[{} {}]", sound, sound2));
        } else if distance > 0.7 {
            if (ctx_idx + i) % 3 == 0 {
                pattern_parts.push("~".to_string());
            } else {
                pattern_parts.push(sound.to_string());
            }
        } else {
            pattern_parts.push(sound.to_string());
        }

        let gain = 0.3 + bfactor * 0.7;
        gain_parts.push(format!("{:.2}", gain.clamp(0.15, 1.0)));
    }

    (pattern_parts.join(" "), gain_parts.join(" "))
}

/// Build a motif-derived drum pattern based on secondary structure composition.
/// Helix → driving 16th feel, Sheet → staccato sparse, Coil → syncopated broken.
fn build_motif_pattern(fingerprint: &StructuralFingerprint) -> String {
    const HELIX_PATTERNS: &[&str] = &[
        "[bd bd] sd [hh bd] sd",
        "bd [sd sd] hh [bd sd]",
        "[bd hh] sd [bd bd] hh",
    ];
    const SHEET_PATTERNS: &[&str] = &[
        "bd ~ sd ~ cp ~ sd ~",
        "bd ~ ~ sd ~ cp ~ ~",
        "cp ~ bd ~ sd ~ ~ bd",
    ];
    const COIL_PATTERNS: &[&str] = &[
        "~ bd [~ sd] hh ~ [cp ~] bd",
        "~ [bd ~] ~ sd [~ hh] cp ~",
        "bd ~ [sd cp] ~ ~ bd [~ hh]",
    ];

    let seed = (fingerprint.total_ca_atoms * 7) % 3;
    let summary = &fingerprint.motif_summary;

    let primary = if summary.helix_fraction > summary.sheet_fraction
        && summary.helix_fraction > summary.coil_fraction
    {
        HELIX_PATTERNS[seed]
    } else if summary.sheet_fraction > summary.coil_fraction {
        SHEET_PATTERNS[seed]
    } else {
        COIL_PATTERNS[seed]
    };

    primary.to_string()
}

/// Build a sparse accent layer from long-range 3D fold contacts.
/// Each contact position → accent hit; everything else → rest. Unique to each fold.
fn build_contact_accent_pattern(
    fingerprint: &StructuralFingerprint,
    steps: usize,
) -> Option<String> {
    if fingerprint.contact_accent_positions.is_empty() || fingerprint.total_ca_atoms == 0 {
        return None;
    }

    let mut grid = vec![false; steps];
    for &pos in &fingerprint.contact_accent_positions {
        let step = (pos * steps) / fingerprint.total_ca_atoms;
        if step < steps {
            grid[step] = true;
        }
    }

    let active = grid.iter().filter(|&&x| x).count();
    if active == 0 {
        return None;
    }
    // Thin out if too dense (> 75% filled)
    if active > steps * 3 / 4 {
        let mut keep = true;
        for g in grid.iter_mut() {
            if *g {
                if !keep {
                    *g = false;
                }
                keep = !keep;
            }
        }
    }

    let accent_sounds = ["cp", "rim", "cp", "oh"];
    let mut sound_idx = 0;

    let pattern: Vec<&str> = grid
        .iter()
        .map(|&hit| {
            if hit {
                let s = accent_sounds[sound_idx % accent_sounds.len()];
                sound_idx += 1;
                s
            } else {
                "~"
            }
        })
        .collect();

    Some(pattern.join(" "))
}

/// Build per-step gain pattern for hi-hats from the B-factor contour.
/// Rigid regions → louder (crisp), flexible regions → softer (flowing).
fn build_hat_gain_pattern(
    fingerprint: &StructuralFingerprint,
    hat_mult: u8,
    base_gain: f64,
) -> String {
    let n = hat_mult as usize;
    let n_bfac = fingerprint.b_factor_contour.len();
    if n_bfac == 0 || n == 0 {
        return format!("{:.2}", base_gain);
    }

    (0..n)
        .map(|i| {
            let idx = (i * n_bfac / n).min(n_bfac - 1);
            let bfactor = fingerprint.b_factor_contour[idx];
            let gain = (0.2 + (1.0 - bfactor) * 0.6) * base_gain;
            format!("{:.2}", gain.clamp(0.1, 1.0))
        })
        .collect::<Vec<_>>()
        .join(" ")
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
/// Uses structural fingerprint (3D geometry, B-factor contour, fold contacts, secondary
/// structure motifs) for a pronounced rhythmic fingerprint unique to each protein.
pub fn protein_to_primitives(
    protein: &Protein,
    bpm: u16,
    genre_params: Option<&GenreParams>,
) -> Result<MappedOutput> {
    let features = FeatureExtractor::extract(protein)?;
    let fingerprint = FeatureExtractor::structural_fingerprint(protein);
    let genre = genre_params.map(|g| g.genre);
    let config = MappingConfig::default();

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

    let genre_str = genre.map(|g| g.as_str().to_string());
    let key = genre_params.and_then(|g| g.key.clone());
    let octave = genre_params.and_then(|g| g.octave);
    let melodic = genre_params.map(|g| g.melodic).unwrap_or(false);

    // Distance-modulated rhythm seed + B-factor gain contour
    let (rhythm_seed, seed_gain_pattern) =
        build_distance_modulated_seed(protein, &fingerprint, genre, &config, 24);

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
            swing: 0.0,
        });
    }

    // B-factor variance → base euclidean density
    let (bfac_beats, _): (u8, u8) = if features.b_factor_variance > 50.0 {
        (5, 8)
    } else if features.b_factor_variance > 20.0 {
        (4, 8)
    } else {
        (3, 8)
    };

    // Motif composition adjusts euclidean grid: helix → regular, coil → polyrhythmic, sheet → sparse
    let motif_beat_adjust: i8 = if fingerprint.motif_summary.helix_fraction > 0.5 {
        -1
    } else if fingerprint.motif_summary.coil_fraction > 0.5 {
        1
    } else {
        0
    };
    let base_beats = ((bfac_beats as i8 + motif_beat_adjust).max(2) as u8).min(7);

    let base_segments: u8 = if fingerprint.motif_summary.coil_fraction > 0.5 {
        12
    } else if fingerprint.motif_summary.sheet_fraction > 0.5 {
        16
    } else {
        8
    };

    // Genre adjusts on top of structure
    let (beats, segments) = match genre {
        Some(DnBGenre::Jungle) => ((base_beats + 1).min(7), base_segments.max(8)),
        Some(DnBGenre::Liquid) => ((base_beats.saturating_sub(1)).max(2), base_segments),
        Some(DnBGenre::Trance) => (4, 4), // four-on-the-floor kick
        _ => (base_beats, base_segments),
    };

    // Avg occupancy for base gain
    let atoms: Vec<_> = protein.all_atoms().filter(|a| a.element.to_uppercase() != "H").collect();
    let avg_occupancy: f64 = atoms.iter()
        .map(|a| a.occupancy)
        .sum::<f64>() / atoms.len().max(1) as f64;
    let base_gain = (0.5 + avg_occupancy * 0.5).clamp(0.3, 1.0);

    let max_chain = chain_lengths.iter().copied().max().unwrap_or(8);
    let hat_mult = ((max_chain / 4).max(1).min(8)) as u8;

    let mut primitives = Vec::new();

    // Kick: euclidean with structure-aware grid
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
        gain_pattern: None,
    });

    // Snare/clap layer: structure-derived for DnB, offbeat clap for Trance
    let snare_pat = if matches!(genre, Some(DnBGenre::Trance)) {
        "~ cp ~ cp".to_string() // offbeat clap (trance convention)
    } else {
        build_motif_pattern(&fingerprint) // secondary-structure-derived
    };
    primitives.push(StrudelPrimitive {
        primitive_type: "drum".to_string(),
        pattern: Some(snare_pat),
        sound: None,
        beats: None,
        segments: None,
        gain: base_gain * 0.9,
        layer: Some("snare".to_string()),
        scale: None,
        octave: None,
        note_pattern: None,
        gain_pattern: None,
    });

    // Hi-hats / open hats: Trance uses open hats on offbeats, DnB uses closed hat 16ths
    let (hat_pattern, hat_gain_pat) = if matches!(genre, Some(DnBGenre::Trance)) {
        let pat = format!("[~ oh]*{}", (hat_mult / 2).max(2));
        let gpat = build_hat_gain_pattern(&fingerprint, (hat_mult / 2).max(2), base_gain * 0.5);
        (pat, gpat)
    } else {
        let gpat = build_hat_gain_pattern(&fingerprint, hat_mult, base_gain * 0.6);
        (format!("hh*{}", hat_mult), gpat)
    };
    primitives.push(StrudelPrimitive {
        primitive_type: "drum".to_string(),
        pattern: Some(hat_pattern),
        sound: None,
        beats: None,
        segments: None,
        gain: base_gain * 0.6,
        layer: Some("hats".to_string()),
        scale: None,
        octave: None,
        note_pattern: None,
        gain_pattern: Some(hat_gain_pat),
    });

    // Distance-modulated rhythm seed with B-factor velocity contour
    if element_counts.get("C").copied().unwrap_or(0) > 0 {
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
            gain_pattern: Some(seed_gain_pattern),
        });
    }

    // Contact accent layer: sparse percussion from the protein's 3D fold
    if let Some(contact_pat) = build_contact_accent_pattern(&fingerprint, 16) {
        primitives.push(StrudelPrimitive {
            primitive_type: "drum".to_string(),
            pattern: Some(contact_pat),
            sound: None,
            beats: None,
            segments: None,
            gain: base_gain * 0.7,
            layer: Some("contacts".to_string()),
            scale: None,
            octave: None,
            note_pattern: None,
            gain_pattern: None,
        });
    }

    // Melodic layer (Liquid, Dancefloor, Trance)
    if melodic && matches!(genre, Some(DnBGenre::Liquid) | Some(DnBGenre::Dancefloor) | Some(DnBGenre::Trance)) {
        let scale_key = key.as_deref().unwrap_or("C");
        let scale = if scale_key.contains(':') {
            scale_key.to_string()
        } else {
            let root = scale_key.trim_end_matches('m');
            format!("{}:minor", root)
        };
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
            gain_pattern: None,
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
        swing: fingerprint.swing,
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
            gain_pattern: None,
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
            gain_pattern: None,
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
            gain_pattern: None,
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

// ---------------------------------------------------------------------------
// Visual feedback: protein-structure-driven Strudel visualization
// ---------------------------------------------------------------------------

struct VisualPreset {
    function: &'static str,
    options: &'static str,
}

const KICK_VISUALS: &[VisualPreset] = &[
    VisualPreset { function: "_punchcard", options: "{ fillActive: 1, cycles: 4 }" },
    VisualPreset { function: "_spiral",    options: "{ steady: 0.95, thickness: 5, fade: 1 }" },
    VisualPreset { function: "_punchcard", options: "{ hideInactive: 1, fillActive: 1, smear: 1, cycles: 4 }" },
];

const SNARE_VISUALS: &[VisualPreset] = &[
    VisualPreset { function: "_pianoroll", options: "{ fillActive: 1, stroke: 1, cycles: 4 }" },
    VisualPreset { function: "_punchcard", options: "{ fillActive: 1, stroke: 1, cycles: 4 }" },
    VisualPreset { function: "_spiral",    options: "{ steady: 0.92, thickness: 3, colorizeInactive: 1 }" },
];

const HATS_VISUALS: &[VisualPreset] = &[
    VisualPreset { function: "_spiral",    options: "{ steady: 0.96, thickness: 3, fade: 1 }" },
    VisualPreset { function: "_spiral",    options: "{ steady: 0.90, thickness: 2, fade: 1, colorizeInactive: 1 }" },
    VisualPreset { function: "_punchcard", options: "{ hideInactive: 1, fillActive: 1, cycles: 4 }" },
];

const PERC_VISUALS: &[VisualPreset] = &[
    VisualPreset { function: "_punchcard", options: "{ fillActive: 1, hideInactive: 1, cycles: 4 }" },
    VisualPreset { function: "_pianoroll", options: "{ fillActive: 1, cycles: 6, stroke: 1 }" },
    VisualPreset { function: "_spiral",    options: "{ steady: 0.88, thickness: 4, colorizeInactive: 1, fade: 1 }" },
];

const CONTACT_VISUALS: &[VisualPreset] = &[
    VisualPreset { function: "_spiral",    options: "{ steady: 0.90, thickness: 4, colorizeInactive: 1 }" },
    VisualPreset { function: "_punchcard", options: "{ fillActive: 1, hideInactive: 1 }" },
    VisualPreset { function: "_spiral",    options: "{ steady: 0.85, thickness: 3, fade: 1 }" },
];

const MELODIC_VISUALS: &[VisualPreset] = &[
    VisualPreset { function: "_pianoroll", options: "{ labels: 1, fillActive: 1, autorange: 1, cycles: 4 }" },
    VisualPreset { function: "_pianoroll", options: "{ fillActive: 1, stroke: 1, autorange: 1, cycles: 4 }" },
    VisualPreset { function: "_spiral",    options: "{ steady: 0.94, thickness: 3, fade: 1 }" },
];

const GENERIC_VISUALS: &[VisualPreset] = &[
    VisualPreset { function: "_punchcard", options: "{ fillActive: 1 }" },
    VisualPreset { function: "_spiral",    options: "{ steady: 0.92, thickness: 3 }" },
    VisualPreset { function: "_pianoroll", options: "{ fillActive: 1, cycles: 4 }" },
];

const PALETTE_WARM:  &[&str] = &["#FF6B35", "#FF9F1C", "#FFCA28", "#E8751A", "coral",    "orange"];
const PALETTE_COOL:  &[&str] = &["cyan",    "#00B4D8", "#0096C7", "#48CAE4", "teal",    "#7491D2"];
const PALETTE_VIVID: &[&str] = &["magenta", "#FF006E", "#8338EC", "#FB5607", "hotpink", "#E040FB"];
const PALETTE_EARTH: &[&str] = &["#A7C957", "#6A994E", "#BC6C25", "#DDA15E", "#606C38", "#FEFAE0"];
const ALL_PALETTES:  &[&[&str]] = &[PALETTE_WARM, PALETTE_COOL, PALETTE_VIVID, PALETTE_EARTH];

/// Deterministic seed from protein structural data for visual selection.
fn compute_visual_seed(mapped: &MappedOutput) -> u64 {
    let mut seed: u64 = mapped.tempo as u64;
    for len in &mapped.chain_lengths {
        seed = seed.wrapping_mul(31).wrapping_add(*len as u64);
    }
    for (el, count) in &mapped.element_counts {
        for c in el.bytes() {
            seed = seed.wrapping_mul(37).wrapping_add(c as u64);
        }
        seed = seed.wrapping_mul(41).wrapping_add(*count as u64);
    }
    seed = seed.wrapping_add((mapped.swing * 1000.0) as u64);
    seed ^= seed >> 16;
    seed = seed.wrapping_mul(0x45d9f3b);
    seed ^= seed >> 16;
    seed
}

/// Select a color palette based on protein characteristics.
fn select_palette(mapped: &MappedOutput, seed: u64) -> &'static [&'static str] {
    let chain_count = mapped.chain_lengths.len();
    let total_elements: usize = mapped.element_counts.values().sum();

    if mapped.swing > 0.5 {
        PALETTE_VIVID
    } else if chain_count > 4 {
        PALETTE_WARM
    } else if total_elements > 3000 {
        PALETTE_COOL
    } else {
        ALL_PALETTES[(seed as usize) % ALL_PALETTES.len()]
    }
}

fn layer_visual_presets(layer: Option<&str>) -> &'static [VisualPreset] {
    match layer {
        Some("kick")     => KICK_VISUALS,
        Some("snare")    => SNARE_VISUALS,
        Some("hats")     => HATS_VISUALS,
        Some("perc")     => PERC_VISUALS,
        Some("contacts") => CONTACT_VISUALS,
        Some("melodic")  => MELODIC_VISUALS,
        _                => GENERIC_VISUALS,
    }
}

/// Build the `.color("...")._{visual}({...})` suffix for a layer.
fn build_visual_suffix(
    layer: Option<&str>,
    palette: &[&str],
    seed: u64,
    layer_idx: usize,
) -> String {
    let presets = layer_visual_presets(layer);
    let preset_idx = ((seed.wrapping_add(layer_idx as u64 * 7)) as usize) % presets.len();
    let preset = &presets[preset_idx];

    let color1 = palette[layer_idx % palette.len()];
    let color2 = palette[(layer_idx + 1) % palette.len()];

    let use_dual = matches!(layer, Some("hats") | Some("perc"));
    let color_str = if use_dual {
        format!("{} {}", color1, color2)
    } else {
        color1.to_string()
    };

    format!(
        r#".color("{}").{}({})"#,
        color_str, preset.function, preset.options
    )
}

/// Assemble Strudel code from primitives with protein-driven visual feedback.
/// Each layer gets a deterministic color + inline visualization derived from
/// the protein's structural signature.
pub fn assemble_strudel(mapped: &MappedOutput, sliders: Option<&SliderValues>) -> String {
    let cps = mapped.tempo as f64 / 60.0 / 4.0;
    let seed = compute_visual_seed(mapped);
    let palette = select_palette(mapped, seed);

    let mut parts = Vec::new();
    for (idx, p) in mapped.primitives.iter().enumerate() {
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

        let gain_expr = if let Some(ref gp) = p.gain_pattern {
            format!(r#""{}""#, gp)
        } else {
            format!("slider({:.2}, 0, 1)", gain)
        };

        let base_pattern = if p.primitive_type == "euclidean" {
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

        let visual = build_visual_suffix(p.layer.as_deref(), palette, seed, idx);
        parts.push(format!("{}\n    {}", base_pattern, visual));
    }

    let stack_body = parts.join(",\n  ");
    let swing_comment = if mapped.swing > 0.1 {
        format!("\n// Structural swing: {:.2} (from backbone angle variance)", mapped.swing)
    } else {
        String::new()
    };

    format!(
        r#"// ProDnB assembled from primitives (Strudel JS mode)
// Visual feedback: colors + visuals derived from protein 3D structure.{}
setcps({})

stack(
  {}
)
"#,
        swing_comment,
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
  s("bd*4").color("cyan")._punchcard({{ fillActive: 1, cycles: 4 }}),
  s("sd ~ ~ sd").color("magenta")._pianoroll({{ fillActive: 1, stroke: 1, cycles: 4 }}),
  s("hh*8").color("white")._spiral({{ steady: 0.96, thickness: 3, fade: 1 }})
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
