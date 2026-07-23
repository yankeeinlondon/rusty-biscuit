use std::collections::BTreeMap;
use std::fs;
use std::path::PathBuf;

use super::canonical::{MappingFidelity, PolicyWarning};
use crate::config::atomic::atomic_write;
use crate::error::{ClaudineError, Result};
use crate::provider::Provider;

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

    /// Applies the persistent edits in this plan.
    ///
    /// One-shot plans are descriptive only and are not executed here.
    pub fn apply(&self) -> Result<AppliedMutationReport> {
        if !self.supported {
            let path = self
                .persistent_plan
                .as_ref()
                .and_then(|plan| plan.edits.first())
                .map(|edit| edit.path.clone())
                .unwrap_or_default();
            return Err(ClaudineError::PolicyApplyFailed {
                path,
                message: "mutation plan is marked unsupported".to_owned(),
                source: None,
            });
        }

        let Some(persistent) = &self.persistent_plan else {
            return Ok(AppliedMutationReport {
                provider: self.provider,
                edits_applied: 0,
                warnings: self.warnings.clone(),
            });
        };

        for edit in &persistent.edits {
            if let Some(parent) = edit.path.parent() {
                fs::create_dir_all(parent)?;
            }
            atomic_write(&edit.path, edit.after_preview.as_bytes()).map_err(|error| {
                ClaudineError::PolicyApplyFailed {
                    path: edit.path.clone(),
                    message: error.to_string(),
                    source: Some(error),
                }
            })?;
        }

        Ok(AppliedMutationReport {
            provider: self.provider,
            edits_applied: persistent.edits.len(),
            warnings: self.warnings.clone(),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn apply_writes_edit_contents() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("settings.json");

        let plan = PolicyMutationPlan {
            provider: Provider::Claude,
            persistent_plan: Some(PersistentMutationPlan {
                edits: vec![ConfigEditPlan {
                    source_id: "user".to_owned(),
                    path: path.clone(),
                    description: "write file".to_owned(),
                    before_preview: None,
                    after_preview: "{\"ok\":true}\n".to_owned(),
                }],
                fidelity: MappingFidelity::Exact,
            }),
            one_shot_plan: None,
            warnings: Vec::new(),
            supported: true,
        };

        let report = plan.apply().unwrap();

        assert_eq!(report.edits_applied, 1);
        assert_eq!(fs::read_to_string(path).unwrap(), "{\"ok\":true}\n");
    }

    #[test]
    fn apply_uses_atomic_writer_cleanup() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("nested/settings.json");

        let plan = PolicyMutationPlan {
            provider: Provider::Claude,
            persistent_plan: Some(PersistentMutationPlan {
                edits: vec![ConfigEditPlan {
                    source_id: "user".to_owned(),
                    path: path.clone(),
                    description: "write file".to_owned(),
                    before_preview: None,
                    after_preview: "{\"ok\":true}\n".to_owned(),
                }],
                fidelity: MappingFidelity::Exact,
            }),
            one_shot_plan: None,
            warnings: Vec::new(),
            supported: true,
        };

        plan.apply().unwrap();

        assert!(!path.with_extension("tmp").exists());
    }
}
