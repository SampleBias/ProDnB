//! Framework file: GPU-preprocessed protein representation for Groq.
//!
//! The framework is a compact, structured summary of the protein that Groq
//! uses to organize and assemble appealing Drum & Bass Strudel code.
//! GPU preprocessing (future) will produce this; CPU fallback for now.

use crate::protein::Protein;
use crate::features::FeatureExtractor;
use crate::genre::GenreParams;
use crate::strudel::{protein_to_primitives, StrudelPrimitive};
use anyhow::Result;
use serde::{Serialize, Deserialize};
use std::collections::HashMap;

/// Framework file: preprocessed protein data for LLM musical assembly.
/// Includes deterministic Strudel primitives from PDB mapping.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProteinFramework {
    /// PDB ID (e.g. "1HGB") for representation key header
    #[serde(default)]
    pub pdb_id: Option<String>,

    /// Protein title/name (e.g. "Hemoglobin") for representation key header
    #[serde(default)]
    pub title: Option<String>,

    /// Element counts (C, N, O, S, P, etc.) → drum mapping hints
    pub element_counts: HashMap<String, usize>,

    /// Structural features (from feature extraction)
    pub chain_count: usize,
    pub residue_count: usize,
    pub total_atoms: usize,
    pub radius_of_gyration: f64,
    pub contact_density: f64,
    pub b_factor_variance: f64,

    /// Composition ratios (0–1)
    pub hydrophobic_ratio: f64,
    pub charged_ratio: f64,
    pub aromatic_ratio: f64,

    /// Rhythm seed: element sequence sampled along backbone (C→bd, N→sd, O→hh, S→cp, P→rim)
    /// e.g. "bd bd sd hh bd ~ sd" for Groq to use as structural hint
    pub rhythm_seed: String,

    /// Chain lengths for polyrhythmic layering
    pub chain_lengths: Vec<usize>,

    /// Deterministic Strudel primitives from PDB mapping (for LLM arrangement)
    pub primitives: Vec<StrudelPrimitive>,

    /// Tempo in BPM (default 174 for DnB)
    pub tempo: u16,

    /// DnB subgenre for arrangement context
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

impl ProteinFramework {
    /// Build framework from protein (no genre params). Uses default mapping.
    pub fn from_protein(protein: &Protein) -> Result<Self> {
        Self::from_protein_with_params(protein, 174, None)
    }

    /// Build framework from protein with genre and tonal params.
    /// Stage 1: deterministic mapping produces primitives; LLM arranges in stage 2.
    pub fn from_protein_with_params(
        protein: &Protein,
        bpm: u16,
        genre_params: Option<&GenreParams>,
    ) -> Result<Self> {
        let features = FeatureExtractor::extract(protein)?;
        let mapped = protein_to_primitives(protein, bpm, genre_params)?;

        let mut element_counts: HashMap<String, usize> = HashMap::new();
        for atom in protein.all_atoms() {
            let el = atom.element.to_uppercase();
            if el != "H" {
                *element_counts.entry(el).or_insert(0) += 1;
            }
        }

        let rhythm_seed = mapped.rhythm_seed;
        let chain_lengths = mapped.chain_lengths;

        Ok(ProteinFramework {
            pdb_id: protein.metadata.pdb_id.clone(),
            title: protein.metadata.title.clone(),
            element_counts,
            chain_count: features.chain_count,
            residue_count: features.residue_count,
            total_atoms: features.total_atoms,
            radius_of_gyration: features.radius_of_gyration,
            contact_density: features.contact_density,
            b_factor_variance: features.b_factor_variance,
            hydrophobic_ratio: features.hydrophobic_residue_ratio,
            charged_ratio: features.charged_residue_ratio,
            aromatic_ratio: features.aromatic_residue_ratio,
            rhythm_seed,
            chain_lengths,
            primitives: mapped.primitives,
            tempo: mapped.tempo,
            genre: mapped.genre,
            key: mapped.key,
            octave: mapped.octave,
            melodic: mapped.melodic,
        })
    }

    /// Serialize to JSON for Groq.
    pub fn to_json(&self) -> Result<String> {
        serde_json::to_string_pretty(self).map_err(Into::into)
    }
}
