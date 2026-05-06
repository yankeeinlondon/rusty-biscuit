//! Output generation options.

/// Options for controlling code generation output.
#[derive(Debug, Clone, Default)]
pub struct OutputOptions {
    /// If true, don't include `pub use schematic_definitions::...` in generated modules.
    ///
    /// This is used for imported APIs where types are generated locally in `types.rs`
    /// instead of being imported from `schematic-definitions`.
    pub standalone: bool,
    /// If true, include a `pub mod types;` declaration in lib.rs.
    ///
    /// This is used when types are generated from imported OpenAPI specs.
    pub include_types_module: bool,
}
