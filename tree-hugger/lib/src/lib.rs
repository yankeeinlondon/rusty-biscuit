pub mod adapter;
pub mod analysis;
pub mod builtins;
pub mod cache;
pub mod corpus;
pub mod dead_code;
pub mod error;
pub mod file;
pub mod god_files;
pub mod ignore_directives;
pub mod package;
pub mod queries;
pub mod resolver;
pub mod rule_registry;
pub mod scanner;
pub mod shared;

// Re-export query subsystem for consumers
pub use queries::{
    compatibility::{CompatibilityRegistry, find_hook_predicates, find_unsupported_predicates},
    drift::{DriftItem, DriftReport, check_all_drift, check_query_drift, summarize_drift},
    inventory::{CaptureEntry, LanguageInventory, NvimQuerySuite, QueryInventory, suite_for_kind},
    provenance::{QueryProvenance, TranslationStatus, all_query_provenance, query_provenance},
};

pub use builtins::is_builtin;
pub use dead_code::{find_dead_code_after, is_terminal_statement};
pub use error::TreeHuggerError;
pub use file::tree_file::TreeFile;
pub use ignore_directives::IgnoreDirectives;
pub use package::tree_package::{
    TreePackage, TreePackageConfig, find_git_root, find_package_root, has_package_manifest,
};
pub use rule_registry::{RuleRegistry, RuleSelector};
pub use shared::*;
