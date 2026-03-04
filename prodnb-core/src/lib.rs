pub mod protein;
pub mod features;
pub mod composition;
pub mod rng;
pub mod style;
pub mod strudel;
pub mod framework;

pub use protein::{Protein, Chain, Residue, Atom};
pub use features::{ProteinFeatures, FeatureExtractor};
pub use composition::{CompositionEngine, ArrangementPlan, DnBParameters};
pub use strudel::{element_to_sound, protein_to_strudel, protein_to_strudel_layered, default_strudel_code};
pub use framework::ProteinFramework;
pub use rng::DeterministicRng;
pub use style::{Style, StyleConfig};
