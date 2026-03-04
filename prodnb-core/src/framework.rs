//! Framework file: GPU-preprocessed protein representation for Groq.
//!
//! The framework is a compact, structured summary of the protein that Groq
//! uses to organize and assemble appealing Drum & Bass Strudel code.
//! GPU preprocessing (future) will produce this; CPU fallback for now.

use crate::protein::Protein;
use crate::features::FeatureExtractor;
use anyhow::Result;
use serde::{Serialize, Deserialize};
use std::collections::HashMap;

/// Framework file: preprocessed protein data for LLM musical assembly.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProteinFramework {
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
}

impl ProteinFramework {
    /// Build framework from protein. CPU path; GPU will replace this later.
    pub fn from_protein(protein: &Protein) -> Result<Self> {
        let features = FeatureExtractor::extract(protein)?;

        let mut element_counts: HashMap<String, usize> = HashMap::new();
        for atom in protein.all_atoms() {
            let el = atom.element.to_uppercase();
            if el != "H" {
                *element_counts.entry(el).or_insert(0) += 1;
            }
        }

        let rhythm_seed = Self::build_rhythm_seed(protein);
        let chain_lengths: Vec<usize> = protein.chains.iter()
            .map(|c| c.residues.len())
            .collect();

        Ok(ProteinFramework {
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
        })
    }

    fn element_to_sound(element: &str) -> &'static str {
        match element.to_uppercase().as_str() {
            "C" => "bd",
            "N" => "sd",
            "O" => "hh",
            "S" => "cp",
            "P" => "rim",
            _ => "perc",
        }
    }

    fn build_rhythm_seed(protein: &Protein) -> String {
        let atoms: Vec<_> = protein.all_atoms()
            .filter(|a| a.element.to_uppercase() != "H")
            .collect();
        if atoms.is_empty() {
            return "bd sd hh".to_string();
        }
        let step = (atoms.len() / 24).max(1).min(4);
        atoms.iter()
            .step_by(step)
            .take(24)
            .map(|a| Self::element_to_sound(&a.element))
            .collect::<Vec<_>>()
            .join(" ")
    }

    /// Serialize to JSON for Groq.
    pub fn to_json(&self) -> Result<String> {
        serde_json::to_string_pretty(self).map_err(Into::into)
    }
}
