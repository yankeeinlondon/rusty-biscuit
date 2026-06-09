//! God-file detection: identify oversized source files that are strong
//! candidates for refactoring.
//!
//! The entry point is [`GodFiles`], which provides lazy candidate
//! discovery and structured analysis.

pub mod constants;
pub mod types;

mod analysis;

pub use analysis::GodFiles;
pub use types::{
    ContainerCallout, GodAnalysis, KindHistogram, MemberSummary, RefactorHint, RiskBand,
    SymbolBlock,
};
