#![feature(rustc_private)]
#![feature(box_patterns)]
#![feature(proc_macro_diagnostic)]
#![feature(proc_macro_span)]
#![feature(proc_macro_quote)]

#[cfg(feature = "with-rustc")]
extern crate rustc_driver;

pub mod analyzer;
pub mod rectifier;
pub mod solver;
pub mod validator;
pub mod mir_analyzer;

pub use analyzer::OverflowCandidate;
pub use rectifier::*;
pub use solver::{BufferSolver, BufferConstraint};
pub use validator::*;
pub use mir_analyzer::{BufferOverflow, MirAnalyzer};