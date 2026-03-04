use crate::features::ProteinFeatures;
use crate::style::Style;
use crate::rng::DeterministicRng;
use serde::{Serialize, Deserialize};
use anyhow::Result;

const DEFAULT_BPM: u16 = 174;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum Section {
    Intro,
    Build,
    Drop1,
    Break,
    Drop2,
    Outro,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DnBParameters {
    pub bpm: u16,
    pub intensity: f32,      // 0.0 - 1.0
    pub complexity: f32,    // 0.0 - 1.0
    pub style: Style,
    pub bass_movement: f32,  // Filter envelope depth
    pub drum_chaos: f32,     // Fill probability and microtiming
    pub layer_count: u8,     // Number of percussion/bass layers
    pub distortion: f32,     // Saturation amount
}

impl Default for DnBParameters {
    fn default() -> Self {
        DnBParameters {
            bpm: DEFAULT_BPM,
            intensity: 0.5,
            complexity: 0.5,
            style: Style::Liquid,
            bass_movement: 0.5,
            drum_chaos: 0.5,
            layer_count: 2,
            distortion: 0.0,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ArrangementPlan {
    pub bpm: u16,
    pub sections: Vec<SectionConfig>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SectionConfig {
    pub section: Section,
    pub start_bar: u16,
    pub bars: u16,
}

#[derive(Debug, Clone)]
pub struct CompositionEngine {
    rng: DeterministicRng,
}

impl CompositionEngine {
    pub fn new(seed: u64) -> Self {
        CompositionEngine {
            rng: DeterministicRng::new(seed),
        }
    }

    pub fn compose(&mut self, features: &ProteinFeatures, params: &DnBParameters) -> Result<ArrangementPlan> {
        let plan = ArrangementPlan {
            bpm: params.bpm,
            sections: self.generate_sections(params),
        };
        Ok(plan)
    }

    pub fn map_features_to_params(&mut self, features: &ProteinFeatures, base_params: &DnBParameters) -> DnBParameters {
        let mut params = base_params.clone();

        params.layer_count = (features.chain_count as u8).max(1).min(4);

        params.complexity = normalize_feature(features.contact_density, 0.0, 5.0);

        params.drum_chaos = normalize_feature(features.b_factor_variance, 0.0, 100.0);

        params.bass_movement = normalize_feature(features.radius_of_gyration, 0.0, 50.0);

        params.distortion = match params.style {
            Style::Neuro => features.charged_residue_ratio as f32 * 0.8,
            Style::Jungle => features.hydrophobic_residue_ratio as f32 * 0.4,
            Style::Liquid => features.hydrophobic_residue_ratio as f32 * 0.2,
        };

        params
    }

    pub fn generate_sections(&mut self, params: &DnBParameters) -> Vec<SectionConfig> {
        let mut sections = Vec::new();
        let mut bar = 0;

        sections.push(SectionConfig {
            section: Section::Intro,
            start_bar: bar,
            bars: 16,
        });
        bar += 16;

        sections.push(SectionConfig {
            section: Section::Build,
            start_bar: bar,
            bars: 16,
        });
        bar += 16;

        sections.push(SectionConfig {
            section: Section::Drop1,
            start_bar: bar,
            bars: 32,
        });
        bar += 32;

        sections.push(SectionConfig {
            section: Section::Break,
            start_bar: bar,
            bars: 16,
        });
        bar += 16;

        sections.push(SectionConfig {
            section: Section::Drop2,
            start_bar: bar,
            bars: 32,
        });
        bar += 32;

        sections.push(SectionConfig {
            section: Section::Outro,
            start_bar: bar,
            bars: 16,
        });

        sections
    }
}

fn normalize_feature(value: f64, min: f64, max: f64) -> f32 {
    if max <= min {
        return 0.5;
    }
    let normalized = (value - min) / (max - min);
    normalized as f32
}
