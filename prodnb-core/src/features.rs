use crate::protein::Protein;
use anyhow::Result;
use serde::{Serialize, Deserialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProteinFeatures {
    pub chain_count: usize,
    pub residue_count: usize,
    pub total_atoms: usize,

    pub radius_of_gyration: f64,
    pub contact_density: f64,
    pub avg_b_factor: f64,
    pub b_factor_variance: f64,

    pub avg_chain_length: f64,
    pub chain_length_variance: f64,

    pub hydrophobic_residue_ratio: f64,
    pub charged_residue_ratio: f64,
    pub aromatic_residue_ratio: f64,
}

pub struct FeatureExtractor;

impl FeatureExtractor {
    pub fn extract(protein: &Protein) -> Result<ProteinFeatures> {
        let chain_count = protein.chain_count();
        let residue_count = protein.residue_count();
        let total_atoms = protein.all_atoms().count();

        if total_atoms == 0 {
            anyhow::bail!("Protein has no atoms");
        }

        let ca_atoms: Vec<_> = protein.ca_atoms().collect();

        let (radius_of_gyration, center_of_mass) = if ca_atoms.is_empty() {
            (0.0, [0.0, 0.0, 0.0])
        } else {
            let mut cx = 0.0;
            let mut cy = 0.0;
            let mut cz = 0.0;
            for atom in &ca_atoms {
                cx += atom.x;
                cy += atom.y;
                cz += atom.z;
            }
            let n = ca_atoms.len() as f64;
            cx /= n;
            cy /= n;
            cz /= n;

            let mut rg_sq = 0.0;
            for atom in &ca_atoms {
                let dx = atom.x - cx;
                let dy = atom.y - cy;
                let dz = atom.z - cz;
                rg_sq += dx * dx + dy * dy + dz * dz;
            }
            (rg_sq.sqrt(), [cx, cy, cz])
        };

        let contact_density = Self::calculate_contact_density(&ca_atoms, center_of_mass);

        let b_factors: Vec<f64> = protein.all_atoms().map(|a| a.b_factor).collect();
        let avg_b_factor = if b_factors.is_empty() {
            0.0
        } else {
            b_factors.iter().sum::<f64>() / b_factors.len() as f64
        };
        let b_factor_variance = Self::variance(&b_factors, avg_b_factor);

        let chain_lengths: Vec<f64> = protein.chains.iter()
            .map(|c| c.residues.len() as f64)
            .collect();
        let avg_chain_length = if chain_lengths.is_empty() {
            0.0
        } else {
            chain_lengths.iter().sum::<f64>() / chain_lengths.len() as f64
        };
        let chain_length_variance = Self::variance(&chain_lengths, avg_chain_length);

        let (hydrophobic_ratio, charged_ratio, aromatic_ratio) = if residue_count > 0 {
            Self::calculate_composition(protein)
        } else {
            (0.0, 0.0, 0.0)
        };

        Ok(ProteinFeatures {
            chain_count,
            residue_count,
            total_atoms,
            radius_of_gyration,
            contact_density,
            avg_b_factor,
            b_factor_variance,
            avg_chain_length,
            chain_length_variance,
            hydrophobic_residue_ratio: hydrophobic_ratio,
            charged_residue_ratio: charged_ratio,
            aromatic_residue_ratio: aromatic_ratio,
        })
    }

    fn calculate_contact_density(atoms: &[&crate::protein::Atom], center: [f64; 3]) -> f64 {
        if atoms.is_empty() {
            return 0.0;
        }

        let contact_threshold = 8.0;
        let mut contact_count = 0;

        for (i, a1) in atoms.iter().enumerate() {
            for a2 in atoms.iter().skip(i + 1) {
                let dx = a1.x - a2.x;
                let dy = a1.y - a2.y;
                let dz = a1.z - a2.z;
                let dist = (dx * dx + dy * dy + dz * dz).sqrt();

                if dist < contact_threshold {
                    contact_count += 1;
                }
            }
        }

        contact_count as f64 / atoms.len() as f64
    }

    fn variance(values: &[f64], mean: f64) -> f64 {
        if values.is_empty() {
            return 0.0;
        }

        let sum_sq: f64 = values.iter()
            .map(|v| (v - mean).powi(2))
            .sum();

        sum_sq / values.len() as f64
    }

    fn calculate_composition(protein: &Protein) -> (f64, f64, f64) {
        const HYDROPHOBIC: &[&str] = &["ALA", "VAL", "LEU", "ILE", "MET", "PHE", "TRP", "PRO"];
        const CHARGED: &[&str] = &["ARG", "LYS", "ASP", "GLU"];
        const AROMATIC: &[&str] = &["PHE", "TYR", "TRP", "HIS"];

        let mut hydrophobic = 0;
        let mut charged = 0;
        let mut aromatic = 0;
        let mut total = 0;

        for chain in &protein.chains {
            for residue in &chain.residues {
                let name = residue.name.to_uppercase();
                total += 1;

                if HYDROPHOBIC.contains(&name.as_str()) {
                    hydrophobic += 1;
                }
                if CHARGED.contains(&name.as_str()) {
                    charged += 1;
                }
                if AROMATIC.contains(&name.as_str()) {
                    aromatic += 1;
                }
            }
        }

        let total = total as f64;
        (
            hydrophobic as f64 / total,
            charged as f64 / total,
            aromatic as f64 / total,
        )
    }
}
