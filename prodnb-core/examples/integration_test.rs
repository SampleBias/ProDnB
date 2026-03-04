use prodnb_core::{Protein, CompositionEngine, DnBParameters};
use prodnb_midi::MidiBuilder;

fn main() -> anyhow::Result<()> {
    println!("=== ProDnB End-to-End Test ===\n");

    println!("Step 1: Loading PDB file...");
    let protein = Protein::load_from_file("test_protein.pdb")?;
    println!("  Loaded {} atoms across {} chains",
        protein.metadata.total_atoms,
        protein.chains.len());

    println!("\nStep 2: Extracting features...");
    let features = prodnb_core::FeatureExtractor::extract(&protein)?;
    println!("  Chain count: {}", features.chain_count);
    println!("  Residue count: {}", features.residue_count);
    println!("  Radius of gyration: {:.2} Å", features.radius_of_gyration);
    println!("  Contact density: {:.4}", features.contact_density);

    println!("\nStep 3: Creating composition...");
    let params = DnBParameters::default();
    let mut composer = CompositionEngine::new(42);
    let mapped_params = composer.map_features_to_params(&features, &params);
    let arrangement = composer.compose(&features, &mapped_params)?;
    println!("  BPM: {}", arrangement.bpm);
    println!("  Sections: {}", arrangement.sections.len());
    println!("  Style: {:?}", mapped_params.style);

    println!("\nStep 4: Generating MIDI events...");
    let mut midi_builder = MidiBuilder::new();
    midi_builder.build_from_composition(&arrangement, &mapped_params)?;
    println!("  Tracks generated: {}", midi_builder.tracks().len());

    for track in midi_builder.tracks() {
        println!("    {:?}: {} events (channel {})",
            track.stem_type,
            track.events.len(),
            track.channel);
    }

    println!("\n=== Test Complete ===");
    println!("All systems operational!");

    Ok(())
}
