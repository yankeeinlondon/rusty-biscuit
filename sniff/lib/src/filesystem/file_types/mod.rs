mod aggregate;
mod classify;
mod framework;
mod model;
mod registry;

pub(crate) use aggregate::{
    FrameworkAccumulator, LanguageAccumulator, accumulate_language_classification,
    build_association_breakdown, build_language_summary,
};
pub use aggregate::{summarize_file_inventory, summarize_languages};
pub(crate) use classify::{MAX_FILES, classify_file, should_skip_directory_name};
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
