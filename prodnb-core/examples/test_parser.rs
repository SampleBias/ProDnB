use prodnb_core::Protein;

fn main() -> anyhow::Result<()> {
    let pdb_path = "test_protein.pdb";
    println!("Loading PDB file: {}", pdb_path);

    let protein = Protein::load_from_file(pdb_path)?;
    println!("\nProtein loaded successfully!");
    println!("  Filename: {:?}", protein.metadata.filename);
    println!("  Total atoms: {}", protein.metadata.total_atoms);
    println!("  Total residues: {}", protein.metadata.total_residues);
    println!("  Chains: {}", protein.chains.len());

    for chain in &protein.chains {
        println!("\nChain {}: {} residues", chain.id, chain.residues.len());
        for (i, residue) in chain.residues.iter().take(3).enumerate() {
            println!("  Residue {}: {} ({})", i + 1, residue.name, residue.sequence_number);
        }
        if chain.residues.len() > 3 {
            println!("  ... and {} more residues", chain.residues.len() - 3);
        }
    }

    println!("\nExtracting features...");
    let features = prodnb_core::FeatureExtractor::extract(&protein)?;
    println!("Features extracted successfully!");
    println!("  Chain count: {}", features.chain_count);
    println!("  Residue count: {}", features.residue_count);
    println!("  Total atoms: {}", features.total_atoms);
    println!("  Radius of gyration: {:.2} Å", features.radius_of_gyration);
    println!("  Contact density: {:.4}", features.contact_density);
    println!("  Avg B-factor: {:.2}", features.avg_b_factor);
    println!("  B-factor variance: {:.2}", features.b_factor_variance);

    println!("\nResidue composition:");
    println!("  Hydrophobic ratio: {:.2}", features.hydrophobic_residue_ratio);
    println!("  Charged ratio: {:.2}", features.charged_residue_ratio);
    println!("  Aromatic ratio: {:.2}", features.aromatic_residue_ratio);

    println!("\nTesting composition engine...");
    let params = prodnb_core::DnBParameters::default();
    let mut composer = prodnb_core::CompositionEngine::new(42);
    let arrangement = composer.compose(&features, &params)?;
    println!("Arrangement created successfully!");
    println!("  BPM: {}", arrangement.bpm);
    println!("  Sections: {}", arrangement.sections.len());

    for section in &arrangement.sections {
        println!("    {:?}: bars {}-{}", section.section, section.start_bar, section.start_bar + section.bars);
    }

    Ok(())
}
