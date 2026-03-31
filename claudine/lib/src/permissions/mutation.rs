use std::collections::BTreeMap;
use std::path::PathBuf;

use crate::events::Provider;

use super::canonical::{MappingFidelity, PolicyWarning};

/// Structured mutation plan produced by a backend.
///
/// Contains optional persistent and one-shot plans, with warnings about
/// fidelity or unsupported operations.
#[derive(Debug, Clone)]
pub struct PolicyMutationPlan {
    /// Provider this plan was generated for.
    pub provider: Provider,
    /// Persistent config edit plan, if the change can be written to files.
    pub persistent_plan: Option<PersistentMutationPlan>,
    /// One-shot CLI arg plan, if the change can be expressed as launch flags.
    pub one_shot_plan: Option<OneShotMutationPlan>,
    /// Warnings about fidelity, unsupported operations, or broadened scope.
    pub warnings: Vec<PolicyWarning>,
    /// Whether the requested mutation is supported by this backend.
    pub supported: bool,
}

/// Plan for persistent config file edits.
#[derive(Debug, Clone)]
pub struct PersistentMutationPlan {
    /// Individual file edit plans.
    pub edits: Vec<ConfigEditPlan>,
    /// Overall fidelity of the persistent mutation.
    pub fidelity: MappingFidelity,
}

/// Plan for one-shot CLI argument overrides.
#[derive(Debug, Clone)]
pub struct OneShotMutationPlan {
    /// CLI arguments that express this change.
    pub argv: Vec<String>,
    /// Environment variables that express this change.
    pub env: BTreeMap<String, String>,
    /// Fidelity of the one-shot mapping.
    pub fidelity: MappingFidelity,
}

/// Plan for editing a single config file.
#[derive(Debug, Clone)]
pub struct ConfigEditPlan {
    /// Source ID of the config being edited.
    pub source_id: String,
    /// Path to the config file.
    pub path: PathBuf,
    /// Human-readable description of the edit.
    pub description: String,
    /// Preview of the file content before the edit, if available.
    pub before_preview: Option<String>,
    /// Preview of the file content after the edit.
    pub after_preview: String,
}

/// Report produced after applying a mutation plan.
#[derive(Debug, Clone)]
pub struct AppliedMutationReport {
    /// Provider the mutation was applied for.
    pub provider: Provider,
    /// Number of config edits applied.
    pub edits_applied: usize,
    /// Warnings encountered during application.
    pub warnings: Vec<PolicyWarning>,
}

impl PolicyMutationPlan {
    /// Creates an unsupported mutation plan with a warning.
    pub fn unsupported(provider: Provider, reason: impl Into<String>) -> Self {
        Self {
            provider,
            persistent_plan: None,
            one_shot_plan: None,
            warnings: vec![PolicyWarning {
                code: "unsupported_mutation".to_owned(),
                message: reason.into(),
                source_id: None,
            }],
            supported: false,
        }
    }
}
