//! Library target of the `unchained-ai-gen` crate.
//!
//! Exists so the model-catalog artifact logic is testable and reusable by the
//! crate's binaries without becoming a new workspace member; the JSON artifact
//! it produces (`unchained-ai/artifacts/models-catalog.json`) is the only
//! cross-area interface (`claudine/features/2026-07-02-provider-metadata/
//! design/model-catalog-boundary.md`).

pub mod catalog;
