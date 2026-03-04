//! ProDnB Web Server Library
//!
//! Provides web interface for converting PDB files to Strudel code.

pub mod handlers;
pub mod templates;

pub use handlers::{upload_pdb, generate_strudel, health_check};
