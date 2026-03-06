pub mod protein;
pub mod features;
pub mod composition;
pub mod rng;
pub mod style;
pub mod genre;
pub mod strudel;
pub mod framework;

pub use protein::{Protein, Chain, Residue, Atom};
pub use features::{ProteinFeatures, FeatureExtractor, StructuralFingerprint, BackboneMotif, ResidueClass, MotifSummary};
pub use composition::{CompositionEngine, ArrangementPlan, DnBParameters};
pub use strudel::{
    element_to_sound, element_to_sound_dynamic, protein_to_strudel, protein_to_strudel_layered,
    default_strudel_code, protein_to_primitives, assemble_strudel, infer_genre_from_structure,
    MappedOutput, StrudelPrimitive, SliderValues, MappingConfig,
};
pub use protein::AtomContext;
pub use framework::ProteinFramework;
pub use genre::{DnBGenre, GenreParams};
pub use rng::DeterministicRng;
pub use style::{Style, StyleConfig};
