use crate::protein::Protein;
use anyhow::Result;
use serde::{Serialize, Deserialize};
use std::collections::HashSet;

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

    /// Extract a structural fingerprint from the protein's 3D geometry.
    /// Uses backbone CA atoms for distances, angles, motifs, and fold contacts.
    pub fn structural_fingerprint(protein: &Protein) -> StructuralFingerprint {
        let mut all_ca_bfactors: Vec<f64> = Vec::new();
        let mut all_distances: Vec<f64> = Vec::new();
        let mut all_motifs: Vec<BackboneMotif> = Vec::new();
        let mut all_ca_positions: Vec<(f64, f64, f64)> = Vec::new();

        for chain in &protein.chains {
            let cas: Vec<&crate::protein::Atom> = chain.residues.iter()
                .filter_map(|r| r.atoms.iter().find(|a| a.name == "CA"))
                .collect();

            if cas.is_empty() {
                continue;
            }

            for ca in &cas {
                all_ca_bfactors.push(ca.b_factor);
                all_ca_positions.push((ca.x, ca.y, ca.z));
            }

            for i in 1..cas.len() {
                all_distances.push(Self::atom_distance_raw(
                    cas[i - 1].x, cas[i - 1].y, cas[i - 1].z,
                    cas[i].x, cas[i].y, cas[i].z,
                ));
            }

            let mut motifs = vec![BackboneMotif::Coil; cas.len()];
            // Helix: CA_i to CA_i+3 ≈ 5.4Å
            for i in 0..cas.len().saturating_sub(3) {
                let d13 = Self::atom_distance_raw(
                    cas[i].x, cas[i].y, cas[i].z,
                    cas[i + 3].x, cas[i + 3].y, cas[i + 3].z,
                );
                if d13 < 6.0 {
                    for j in i..=(i + 3).min(cas.len() - 1) {
                        motifs[j] = BackboneMotif::Helix;
                    }
                }
            }
            // Sheet: CA_i to CA_i+2 ≈ 6.5–7.0Å (only if not already helix)
            for i in 0..cas.len().saturating_sub(2) {
                let d12 = Self::atom_distance_raw(
                    cas[i].x, cas[i].y, cas[i].z,
                    cas[i + 2].x, cas[i + 2].y, cas[i + 2].z,
                );
                if d12 > 6.0 && d12 < 7.5 {
                    for j in i..=(i + 2).min(cas.len() - 1) {
                        if motifs[j] == BackboneMotif::Coil {
                            motifs[j] = BackboneMotif::Sheet;
                        }
                    }
                }
            }
            all_motifs.extend(motifs);
        }

        let total_ca = all_ca_positions.len();
        if total_ca < 3 {
            return StructuralFingerprint::default();
        }

        // Normalize inter-CA distances: 3.8Å (helix) → 0.0, ~7.5Å (extended) → 1.0
        let distance_rhythm: Vec<f64> = all_distances.iter()
            .map(|d| ((d - 3.5) / 4.0).clamp(0.0, 1.0))
            .collect();

        // Normalize B-factors to 0–1 within this protein
        let max_b = all_ca_bfactors.iter().copied().fold(f64::MIN, f64::max);
        let min_b = all_ca_bfactors.iter().copied().fold(f64::MAX, f64::min);
        let b_range = (max_b - min_b).max(1.0);
        let b_factor_contour: Vec<f64> = all_ca_bfactors.iter()
            .map(|b| (b - min_b) / b_range)
            .collect();

        // Long-range contacts: close in 3D (<8Å), far in sequence (>20 residues apart)
        let mut contact_set = HashSet::new();
        let check_step = if total_ca > 1000 { 3 } else if total_ca > 500 { 2 } else { 1 };
        for i in (0..total_ca).step_by(check_step) {
            for j in ((i + 20)..total_ca).step_by(check_step) {
                let (x1, y1, z1) = all_ca_positions[i];
                let (x2, y2, z2) = all_ca_positions[j];
                let d = Self::atom_distance_raw(x1, y1, z1, x2, y2, z2);
                if d < 8.0 {
                    contact_set.insert(i);
                    contact_set.insert(j);
                }
            }
        }
        let mut contact_accent_positions: Vec<usize> = contact_set.into_iter().collect();
        contact_accent_positions.sort();

        // Backbone angle variance → swing (regular helix ≈ 0, variable loops → high)
        let mut all_angles: Vec<f64> = Vec::new();
        let mut pos_idx = 0;
        for chain in &protein.chains {
            let n_ca = chain.residues.iter()
                .filter(|r| r.atoms.iter().any(|a| a.name == "CA"))
                .count();
            if n_ca < 3 {
                pos_idx += n_ca;
                continue;
            }
            for i in (pos_idx + 1)..(pos_idx + n_ca - 1) {
                if i == 0 || i + 1 >= all_ca_positions.len() {
                    continue;
                }
                let (x0, y0, z0) = all_ca_positions[i - 1];
                let (x1, y1, z1) = all_ca_positions[i];
                let (x2, y2, z2) = all_ca_positions[i + 1];
                let v1 = (x0 - x1, y0 - y1, z0 - z1);
                let v2 = (x2 - x1, y2 - y1, z2 - z1);
                let dot = v1.0 * v2.0 + v1.1 * v2.1 + v1.2 * v2.2;
                let mag1 = (v1.0 * v1.0 + v1.1 * v1.1 + v1.2 * v1.2).sqrt();
                let mag2 = (v2.0 * v2.0 + v2.1 * v2.1 + v2.2 * v2.2).sqrt();
                if mag1 > 0.001 && mag2 > 0.001 {
                    let cos_a = (dot / (mag1 * mag2)).clamp(-1.0, 1.0);
                    all_angles.push(cos_a.acos());
                }
            }
            pos_idx += n_ca;
        }
        let swing = if all_angles.is_empty() {
            0.0
        } else {
            let avg = all_angles.iter().sum::<f64>() / all_angles.len() as f64;
            let var = all_angles.iter().map(|a| (a - avg).powi(2)).sum::<f64>() / all_angles.len() as f64;
            (var / 0.3).clamp(0.0, 1.0)
        };

        let residue_runs = Self::compute_residue_runs(protein);

        let n_motifs = all_motifs.len().max(1) as f64;
        let helix_count = all_motifs.iter().filter(|m| **m == BackboneMotif::Helix).count() as f64;
        let sheet_count = all_motifs.iter().filter(|m| **m == BackboneMotif::Sheet).count() as f64;
        let coil_count = all_motifs.iter().filter(|m| **m == BackboneMotif::Coil).count() as f64;

        StructuralFingerprint {
            distance_rhythm,
            b_factor_contour,
            backbone_motifs: all_motifs,
            contact_accent_positions,
            swing,
            residue_runs,
            motif_summary: MotifSummary {
                helix_fraction: helix_count / n_motifs,
                sheet_fraction: sheet_count / n_motifs,
                coil_fraction: coil_count / n_motifs,
            },
            total_ca_atoms: total_ca,
        }
    }

    fn atom_distance_raw(x1: f64, y1: f64, z1: f64, x2: f64, y2: f64, z2: f64) -> f64 {
        let dx = x1 - x2;
        let dy = y1 - y2;
        let dz = z1 - z2;
        (dx * dx + dy * dy + dz * dz).sqrt()
    }

    fn classify_residue(name: &str) -> ResidueClass {
        let n = name.to_uppercase();
        if ["PHE", "TYR", "TRP", "HIS"].contains(&n.as_str()) {
            ResidueClass::Aromatic
        } else if ["ARG", "LYS", "ASP", "GLU"].contains(&n.as_str()) {
            ResidueClass::Charged
        } else if ["ALA", "VAL", "LEU", "ILE", "MET", "PRO"].contains(&n.as_str()) {
            ResidueClass::Hydrophobic
        } else {
            ResidueClass::Polar
        }
    }

    fn compute_residue_runs(protein: &Protein) -> Vec<(ResidueClass, usize)> {
        let mut runs = Vec::new();
        let mut current_class: Option<ResidueClass> = None;
        let mut current_len = 0;

        for chain in &protein.chains {
            for residue in &chain.residues {
                let cls = Self::classify_residue(&residue.name);
                if current_class == Some(cls) {
                    current_len += 1;
                } else {
                    if let Some(c) = current_class {
                        runs.push((c, current_len));
                    }
                    current_class = Some(cls);
                    current_len = 1;
                }
            }
        }
        if let Some(c) = current_class {
            runs.push((c, current_len));
        }

        runs
    }
}

/// Local backbone geometry classification.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum BackboneMotif {
    Helix,
    Sheet,
    Coil,
}

/// Residue chemical class for phrase-level rhythm structure.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ResidueClass {
    Hydrophobic,
    Charged,
    Aromatic,
    Polar,
}

/// Secondary structure composition summary.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MotifSummary {
    pub helix_fraction: f64,
    pub sheet_fraction: f64,
    pub coil_fraction: f64,
}

/// 3D-geometry-derived rhythmic fingerprint unique to each protein fold.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StructuralFingerprint {
    /// Normalized inter-CA distances (0.0 = tight helix, 1.0 = fully extended)
    pub distance_rhythm: Vec<f64>,
    /// Per-residue normalized B-factor (0.0 = rigid, 1.0 = most flexible in this protein)
    pub b_factor_contour: Vec<f64>,
    /// Per-residue backbone motif classification
    pub backbone_motifs: Vec<BackboneMotif>,
    /// Residue indices with long-range 3D contacts (sequence gap >20, distance <8Å)
    pub contact_accent_positions: Vec<usize>,
    /// Backbone angle variance normalized 0–1 (0 = regular helix, 1 = highly variable)
    pub swing: f64,
    /// Consecutive runs of same residue class
    pub residue_runs: Vec<(ResidueClass, usize)>,
    pub motif_summary: MotifSummary,
    pub total_ca_atoms: usize,
}

impl Default for StructuralFingerprint {
    fn default() -> Self {
        Self {
            distance_rhythm: Vec::new(),
            b_factor_contour: Vec::new(),
            backbone_motifs: Vec::new(),
            contact_accent_positions: Vec::new(),
            swing: 0.0,
            residue_runs: Vec::new(),
            motif_summary: MotifSummary {
                helix_fraction: 0.0,
                sheet_fraction: 0.0,
                coil_fraction: 1.0,
            },
            total_ca_atoms: 0,
        }
    }
}
