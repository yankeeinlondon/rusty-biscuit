mod aggregate;
mod classify;
mod framework;
mod model;
mod registry;

pub use aggregate::{summarize_file_inventory, summarize_languages};
pub use classify::{
    project_package_inventory, scan_file_inventory, scan_file_inventory_with_exclusions,
};
pub use model::{
    ClassificationConfidence, ClassificationSource, FileAssociation, FileAssociationBreakdown,
    FileAssociationStats, FileClassification, FileInventory, FileScanScope, FileTypeDescriptor,
    FrameworkKind, FrameworkStats, LanguageSummary, ProgrammingLanguage, ProgrammingLanguageStats,
    ProgrammingLanguageType,
};
pub use registry::{is_command_runner_filename, lookup_exact_filename, lookup_extension};
