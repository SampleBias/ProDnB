use anyhow::{Result, Context};
use serde::{Serialize, Deserialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Residue {
    pub serial: u32,
    pub name: String,
    pub chain_id: String,
    pub sequence_number: u32,
    pub insertion_code: String,
    pub atoms: Vec<Atom>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Atom {
    pub serial: u32,
    pub name: String,
    pub element: String,
    pub x: f64,
    pub y: f64,
    pub z: f64,
    pub b_factor: f64,
    pub occupancy: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Chain {
    pub id: String,
    pub residues: Vec<Residue>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Protein {
    pub chains: Vec<Chain>,
    pub metadata: ProteinMetadata,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProteinMetadata {
    pub filename: Option<String>,
    pub pdb_id: Option<String>,
    pub title: Option<String>,
    pub total_atoms: usize,
    pub total_residues: usize,
}

impl Protein {
    pub fn load_from_file(path: &str) -> Result<Self> {
        let content = std::fs::read_to_string(path)
            .with_context(|| format!("Failed to read PDB file: {}", path))?;

        Self::parse_pdb(&content, Some(path.to_string()))
    }

    pub fn parse_pdb(content: &str, filename: Option<String>) -> Result<Self> {
        let mut chains: std::collections::HashMap<String, Vec<Residue>> = std::collections::HashMap::new();
        let mut current_residue: Option<Residue> = None;
        let mut pdb_id = None;
        let mut title = None;

        for line in content.lines() {
            if line.starts_with("HEADER") {
                if line.len() >= 66 {
                    pdb_id = Some(line[62..66].trim().to_string());
                }
            } else if line.starts_with("TITLE") {
                let title_text = line[10..].trim().to_string();
                title = Some(title_text);
            } else if line.starts_with("ATOM") || line.starts_with("HETATM") {
                let atom = Self::parse_atom_line(line)?;
                let chain_id = atom.chain_id.clone();
                let res_key = format!("{}-{}{}", atom.residue_name, atom.residue_seq, atom.insertion_code);

                if current_residue.is_none() ||
                    current_residue.as_ref().unwrap().name != atom.residue_name ||
                    current_residue.as_ref().unwrap().sequence_number != atom.residue_seq ||
                    current_residue.as_ref().unwrap().chain_id != atom.chain_id
                {
                    if let Some(res) = current_residue.take() {
                        chains.entry(res.chain_id.clone()).or_insert_with(Vec::new).push(res);
                    }
                    current_residue = Some(Residue {
                        serial: atom.residue_seq,
                        name: atom.residue_name.clone(),
                        chain_id: chain_id.clone(),
                        sequence_number: atom.residue_seq,
                        insertion_code: atom.insertion_code.clone(),
                        atoms: Vec::new(),
                    });
                }

                if let Some(res) = current_residue.as_mut() {
                    res.atoms.push(Atom {
                        serial: atom.serial as u32,
                        name: atom.name.clone(),
                        element: atom.element,
                        x: atom.x,
                        y: atom.y,
                        z: atom.z,
                        b_factor: atom.b_factor,
                        occupancy: atom.occupancy,
                    });
                }
            }
        }

        if let Some(res) = current_residue {
            chains.entry(res.chain_id.clone()).or_insert_with(Vec::new).push(res);
        }

        let chain_list: Vec<Chain> = chains.into_iter()
            .map(|(id, mut residues)| {
                residues.sort_by_key(|r| r.sequence_number);
                Chain { id, residues }
            })
            .collect();

        let total_residues = chain_list.iter().map(|c| c.residues.len()).sum();
        let total_atoms = chain_list.iter()
            .flat_map(|c| c.residues.iter())
            .map(|r| r.atoms.len())
            .sum();

        Ok(Protein {
            chains: chain_list,
            metadata: ProteinMetadata {
                filename,
                pdb_id,
                title,
                total_atoms,
                total_residues,
            },
        })
    }

    fn parse_atom_line(line: &str) -> Result<AtomLineData> {
        if line.len() < 54 {
            anyhow::bail!("Line too short: {}", line);
        }

        let serial = line[6..11].trim().parse::<usize>()?;
        let name = line[12..16].trim().to_string();
        let residue_name = line[17..20].trim().to_string();
        let chain_id = line[21..22].to_string();
        let residue_seq = line[22..26].trim().parse::<u32>()?;
        let insertion_code = line[26..27].trim().to_string();
        let x = line[30..38].trim().parse::<f64>()?;
        let y = line[38..46].trim().parse::<f64>()?;
        let z = line[46..54].trim().parse::<f64>()?;
        let occupancy = if line.len() >= 60 { line[54..60].trim().parse::<f64>().unwrap_or(1.0) } else { 1.0 };
        let b_factor = if line.len() >= 66 { line[60..66].trim().parse::<f64>().unwrap_or(0.0) } else { 0.0 };

        let element = if line.len() >= 78 {
            line[76..78].trim().to_string()
        } else {
            name.chars().next().map(|c| c.to_string()).unwrap_or_else(|| "X".to_string())
        };

        Ok(AtomLineData {
            serial,
            name,
            residue_name,
            chain_id,
            residue_seq,
            insertion_code,
            x,
            y,
            z,
            occupancy,
            b_factor,
            element,
        })
    }

    pub fn chain_count(&self) -> usize {
        self.chains.len()
    }

    pub fn residue_count(&self) -> usize {
        self.chains.iter().map(|c| c.residues.len()).sum()
    }

    pub fn atom_count(&self) -> usize {
        self.metadata.total_atoms
    }

    pub fn all_atoms(&self) -> impl Iterator<Item = &Atom> {
        self.chains.iter()
            .flat_map(|chain| chain.residues.iter())
            .flat_map(|residue| residue.atoms.iter())
    }

    /// Iterate atoms with residue and chain context for dynamic mapping.
    pub fn all_atoms_with_context(&self) -> impl Iterator<Item = AtomContext<'_>> {
        self.chains.iter().enumerate().flat_map(|(chain_idx, chain)| {
            chain.residues.iter().flat_map(move |residue| {
                residue.atoms.iter().map(move |atom| AtomContext {
                    atom,
                    residue_name: &residue.name,
                    chain_id: &residue.chain_id,
                    chain_index: chain_idx,
                    residue_seq: residue.sequence_number,
                })
            })
        })
    }

    pub fn ca_atoms(&self) -> impl Iterator<Item = &Atom> {
        self.all_atoms()
            .filter(|atom| atom.name == "CA")
    }
}

/// Atom with residue and chain context for dynamic element-to-sound mapping.
#[derive(Debug, Clone)]
pub struct AtomContext<'a> {
    pub atom: &'a Atom,
    pub residue_name: &'a str,
    pub chain_id: &'a str,
    pub chain_index: usize,
    pub residue_seq: u32,
}

struct AtomLineData {
    serial: usize,
    name: String,
    residue_name: String,
    chain_id: String,
    residue_seq: u32,
    insertion_code: String,
    x: f64,
    y: f64,
    z: f64,
    occupancy: f64,
    b_factor: f64,
    element: String,
}
