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
