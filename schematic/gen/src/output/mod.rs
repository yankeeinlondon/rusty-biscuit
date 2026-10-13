//! Output assembly and file writing for generated code.
//!
//! This module handles the final phase of code generation: assembling all generated
//! pieces into a complete Rust file, validating the output, formatting it, and
//! writing it to disk atomically.
//!
//! ## Output Structure
//!
//! The generator produces per-API module files. Small modules are single files;
//! large modules (exceeding [`SPLIT_THRESHOLD_LINES`]) are split into directories:
//!
//! ```text
//! schema/src/
//! ├── lib.rs              # Module declarations, runtime ABI check, re-exports
//! ├── shared.rs           # Request-side helpers (cookies, FormFile, Paginated)
//! ├── anthropic.rs        # Small API module (single file)
//! ├── emqx/               # Large API module (split directory)
//! │   ├── mod.rs          #   API client structs and tags, re-exports, submodules
//! │   ├── requests.rs     #   Request structs and their EndpointSpec impls
//! │   └── responses.rs    #   Definitions re-exports
//! └── prelude.rs          # Common re-exports for consumers
//! ```
//!
//! ## Safety Guarantees
//!
//! - **Prepare, then publish**: [`prepare_rust_output`] validates the batch
//!   with [`validate_rust_client`], assembles, syntax-checks, and formats
//!   every file, and checks every target path before anything is written. The
//!   resulting [`MutationPlan`] is applied by [`MutationPlan::publish`], which
//!   reports completed and pending mutations if a write fails.
//! - **Syntax validation**: All generated code is parsed with `syn` before
//!   writing, which catches grammar/syntax errors only — it does not
//!   type-check, resolve names, or guarantee the code compiles.
//! - **Formatting**: Output is formatted with `prettyplease` for consistent style
//! - **Atomic writes**: Each file is written with temp file + rename; a plan
//!   as a whole is not a transaction.
//! - **Ownership**: A planned path must be absent or carry the generated-file
//!   header (or, for `Cargo.toml`, the manifest marker); deletion is limited to
//!   provably owned files and directories, and symbolic links are never
//!   followed. Hand-written files are never overwritten or deleted.
//! - **Partial generation**: [`Selection::Apis`] regenerates complete module
//!   groups and keeps every other group byte-for-byte; see
//!   [`prepare_rust_output`].

use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use std::path::{Path, PathBuf};

use indexmap::IndexMap;
use proc_macro2::TokenStream;
use schematic_define::RestApi;

use crate::cargo_gen::generate_cargo_toml;
use crate::diagnostics::Output;
use crate::errors::GeneratorError;
use crate::validation::{ensure_no_errors, validate_rust_client};

use inventory::{Occupant, Proof, list_entries, occupant};

pub mod assemble;
pub mod format;
pub(crate) mod inventory;
pub mod options;
pub mod publish;
pub mod write;
pub mod ws_modules;

pub use assemble::{
    SPLIT_THRESHOLD_LINES, SplitApiParts, assemble_api_code, assemble_api_module,
    assemble_api_module_with_options, assemble_combined_api_module, assemble_lib_rs,
    assemble_lib_rs_with_options, assemble_prelude, assemble_prelude_with_options,
    assemble_shared_module, assemble_split_api_module, assemble_split_combined_api_module,
    get_module_path, get_request_suffix,
};
pub use format::{GENERATED_FILE_HEADER, format_code, validate_code};
pub use options::OutputOptions;
pub use publish::{Mutation, MutationPlan, PublicationReport};
pub use write::write_atomic;

/// Generates and writes all API code to the output directory.
///
/// Convenience wrapper around [`generate_and_write_all`] for a single API.
///
/// ## Arguments
///
/// * `api` - The API definition to generate code for
/// * `output_dir` - Directory to write generated files to
/// * `dry_run` - If true, print code instead of writing files
///
/// ## Returns
///
/// The formatted API module code (useful for dry-run mode or testing).
///
/// ## Errors
///
/// As [`generate_and_write_all`].
pub fn generate_and_write(
    api: &RestApi,
    output_dir: &Path,
    dry_run: bool,
) -> Result<String, GeneratorError> {
    let apis = [api];
    generate_and_write_all(&apis, output_dir, dry_run)
}

/// Represents generated output for a single API module.
///
/// A module is either a single `.rs` file or a directory of submodules.
enum GeneratedModule {
    /// Single-file module: `{module_path}.rs`
    SingleFile { filename: String, content: String },
    /// Directory module: `{module_path}/mod.rs` + submodules
    Directory {
        module_path: String,
        mod_rs: String,
        requests_rs: String,
        responses_rs: String,
    },
}

impl GeneratedModule {
    fn main_source(&self) -> &str {
        match self {
            Self::SingleFile { content, .. } => content,
            Self::Directory { mod_rs, .. } => mod_rs,
        }
    }
}

/// Which module groups a Rust generation run regenerates.
#[derive(Debug, Clone, Copy)]
pub enum Selection<'a> {
    /// Every API of the registry. Obsolete generator-owned modules are
    /// removed.
    All,
    /// The complete module groups of these APIs, matched to the registry by
    /// name. Every other module group already in the output is kept
    /// byte-for-byte, and nothing outside the regenerated groups is deleted.
    Apis(&'a [&'a RestApi]),
}

/// One Rust generation run.
#[derive(Debug, Clone, Copy)]
pub struct RustOutputRequest<'a> {
    /// Every API the generator knows, in registry order. `lib.rs` and
    /// `prelude.rs` list modules in this order.
    pub registry: &'a [&'a RestApi],
    /// What this run regenerates.
    pub selection: Selection<'a>,
    /// The crate's `src` directory.
    pub output_dir: &'a Path,
    /// Where the generated `Cargo.toml` goes, when the run writes one.
    pub manifest_dir: Option<&'a Path>,
}

/// A Rust output assembled, formatted, and checked, with nothing written.
#[derive(Debug)]
pub struct RustOutputPlan {
    /// The writes and deletions that publish this output.
    pub plan: MutationPlan,
    /// Module groups this run regenerates, in file order.
    pub regenerated: Vec<String>,
    /// Module groups of the existing output kept as they are.
    pub retained: Vec<String>,
    first_module: String,
}

impl RustOutputPlan {
    /// The formatted source of the first regenerated module (its `mod.rs`
    /// when the module is split).
    pub fn first_module(&self) -> &str {
        &self.first_module
    }
}

/// Generates every API of `apis` into `output_dir`: full generation.
///
/// The output holds `lib.rs`, `shared.rs`, `prelude.rs`, the WebSocket
/// modules, and one module per module group: `{module}.rs`, or a directory
/// with `mod.rs`, `requests.rs`, and `responses.rs` once the
/// module exceeds [`SPLIT_THRESHOLD_LINES`]. Generator-owned modules no API
/// produces are removed. See [`prepare_rust_output`] for every check that
/// runs before the first write.
///
/// ## Returns
///
/// The formatted code for the first API module.
///
/// ## Errors
///
/// As [`prepare_rust_output`], plus [`GeneratorError::PublicationFailed`]
/// when a write or deletion fails.
pub fn generate_and_write_all(
    apis: &[&RestApi],
    output_dir: &Path,
    dry_run: bool,
) -> Result<String, GeneratorError> {
    let prepared = prepare_rust_output(&RustOutputRequest {
        registry: apis,
        selection: Selection::All,
        output_dir,
        manifest_dir: None,
    })?;
    if dry_run {
        prepared.plan.print_dry_run(true);
    } else {
        prepared.plan.publish()?;
    }
    Ok(prepared.first_module)
}

/// Prepares one Rust generation run without touching the filesystem.
///
/// The selection expands to complete module groups: selecting one member of
/// a shared module regenerates every member. A partial run
/// ([`Selection::Apis`]) reads the existing output's inventory, keeps every
/// other module group it holds, and rebuilds `lib.rs` and `prelude.rs` from
/// the retained and regenerated groups. Every file is validated, assembled,
/// syntax-checked, and formatted here. Every planned write must land on an
/// absent path or on a file the generator provably wrote; obsolete paths are
/// deleted only when owned, and only within the regenerated groups unless the
/// run is full. Dry runs print this same plan.
///
/// ## Errors
///
/// - [`GeneratorError::ConfigError`] when a selected API is not in the
///   registry
/// - [`GeneratorError::Validation`] when [`validate_rust_client`] reports an
///   error for the regenerated APIs
/// - [`GeneratorError::UnsafeOutput`] when a partial run cannot establish
///   the existing inventory, or a planned path holds content the generator
///   did not write
/// - [`GeneratorError::RequiresFullGeneration`] when a partial run would
///   change the shared runtime under retained modules
/// - [`GeneratorError::CodeGenError`] when generated code does not parse
pub fn prepare_rust_output(
    request: &RustOutputRequest<'_>,
) -> Result<RustOutputPlan, GeneratorError> {
    let root = request.output_dir;
    let groups = module_groups(request.registry);
    let selected = selected_groups(request.registry, request.selection)?;
    let regenerated_apis = expand_selection(request.registry, request.selection)?;
    ensure_valid_rust_batch(&regenerated_apis)?;

    let inventory = match request.selection {
        Selection::All => inventory::RustInventory::default(),
        Selection::Apis(_) => inventory::read_rust_inventory(root, &groups, &selected)?,
    };
    let retained = inventory.retained;
    let exported: BTreeSet<&str> = selected.iter().chain(&retained).map(String::as_str).collect();
    let crate_apis: Vec<&RestApi> = request
        .registry
        .iter()
        .copied()
        .filter(|api| exported.contains(get_module_path(api).as_str()))
        .collect();

    let lib = render(&assemble_lib_rs(&crate_apis))?;
    let shared = render(&assemble_shared_module())?;
    let prelude = render(&assemble_prelude(&crate_apis))?;
    let generated = assemble_modules(&regenerated_apis)?;
    let (ws_shared_filename, ws_shared) = ws_modules::generate_ws_shared_module()?;
    let ws_definition_modules = ws_modules::generate_ws_definition_modules()?;

    inventory::shared_runtime_compatibility(
        root,
        &[("shared.rs", &shared), (&ws_shared_filename, &ws_shared)],
        &retained,
        &inventory.abi,
    )?;

    let mut planner = Planner::new(root);
    for module in &generated {
        planner.module(module);
    }
    planner.file("shared.rs", shared);
    planner.file(&ws_shared_filename, ws_shared);
    for (filename, content) in ws_definition_modules {
        planner.file(&filename, content);
    }
    if let Some(manifest_dir) = request.manifest_dir {
        planner.write(
            manifest_dir.join("Cargo.toml"),
            generate_cargo_toml(None),
            Proof::Manifest,
        );
    }
    planner.file("prelude.rs", prelude);
    planner.file("lib.rs", lib);
    if matches!(request.selection, Selection::All) {
        planner.remove_obsolete_modules();
    }

    Ok(RustOutputPlan {
        plan: planner.finish()?,
        regenerated: generated.iter().map(|m| m.module_name().to_string()).collect(),
        retained,
        first_module: generated
            .first()
            .map(|m| m.main_source().to_string())
            .unwrap_or_default(),
    })
}

/// The registry grouped by Rust module path, in registry order.
fn module_groups<'a>(registry: &[&'a RestApi]) -> IndexMap<String, Vec<&'a RestApi>> {
    let mut groups: IndexMap<String, Vec<&RestApi>> = IndexMap::new();
    for api in registry {
        groups.entry(get_module_path(api)).or_default().push(api);
    }
    groups
}

/// The registry APIs a Rust run regenerates: every member of every module
/// group `selection` touches, in registry order.
///
/// ## Errors
///
/// [`GeneratorError::ConfigError`] when a selected API is not in the registry.
pub fn expand_selection<'a>(
    registry: &[&'a RestApi],
    selection: Selection<'_>,
) -> Result<Vec<&'a RestApi>, GeneratorError> {
    let selected = selected_groups(registry, selection)?;
    Ok(registry
        .iter()
        .copied()
        .filter(|api| selected.contains(&get_module_path(api)))
        .collect())
}

/// The module groups `selection` regenerates, each API resolved through the
/// registry by name.
fn selected_groups(
    registry: &[&RestApi],
    selection: Selection<'_>,
) -> Result<BTreeSet<String>, GeneratorError> {
    let Selection::Apis(apis) = selection else {
        return Ok(registry.iter().map(|api| get_module_path(api)).collect());
    };
    apis.iter()
        .map(|api| {
            registry
                .iter()
                .find(|known| known.name == api.name)
                .map(|known| get_module_path(known))
                .ok_or_else(|| {
                    GeneratorError::ConfigError(format!(
                        "API `{}` is not in the registry, so its module group is unknown",
                        api.name
                    ))
                })
        })
        .collect()
}

fn render(tokens: &TokenStream) -> Result<String, GeneratorError> {
    Ok(format_code(&validate_code(tokens)?))
}

/// Assembles one module per module group of `apis`, in module-path order.
fn assemble_modules(apis: &[&RestApi]) -> Result<Vec<GeneratedModule>, GeneratorError> {
    let mut module_groups: BTreeMap<String, Vec<&RestApi>> = BTreeMap::new();
    for api in apis {
        module_groups.entry(get_module_path(api)).or_default().push(api);
    }

    let mut generated = Vec::new();
    for (module_path, group) in &module_groups {
        let tokens = if group.len() == 1 {
            assemble_api_module(group[0])
        } else {
            assemble_combined_api_module(group)
        };
        let formatted = render(&tokens)?;
        if formatted.lines().count() > SPLIT_THRESHOLD_LINES {
            let parts = if group.len() == 1 {
                assemble_split_api_module(group[0], &OutputOptions::default())
            } else {
                assemble_split_combined_api_module(group)
            };
            generated.push(GeneratedModule::Directory {
                module_path: module_path.clone(),
                mod_rs: render(&parts.mod_rs)?,
                requests_rs: render(&parts.requests_rs)?,
                responses_rs: render(&parts.responses_rs)?,
            });
        } else {
            generated.push(GeneratedModule::SingleFile {
                filename: format!("{module_path}.rs"),
                content: formatted,
            });
        }
    }
    Ok(generated)
}

impl GeneratedModule {
    fn module_name(&self) -> &str {
        match self {
            Self::SingleFile { filename, .. } => filename.strip_suffix(".rs").unwrap_or(filename),
            Self::Directory { module_path, .. } => module_path,
        }
    }
}

/// Fails with [`GeneratorError::Validation`] when the batch cannot produce a
/// valid Rust client, before anything is assembled or written.
fn ensure_valid_rust_batch(apis: &[&RestApi]) -> Result<(), GeneratorError> {
    ensure_no_errors(Output::Rust, validate_rust_client(apis))
}

/// Remedy shown when a planned path holds content the generator did not write.
const CONFLICT_REMEDY: &str = "Move or remove the listed paths, or generate into an \
     isolated output directory.";

/// Collects one Rust output root's mutations and checks each target's
/// ownership as it is planned.
struct Planner<'a> {
    root: &'a Path,
    writes: Vec<Mutation>,
    deletions: Vec<Mutation>,
    problems: Vec<String>,
    /// Top-level names this run writes.
    planned: BTreeSet<String>,
}

impl<'a> Planner<'a> {
    fn new(root: &'a Path) -> Self {
        let mut problems = Vec::new();
        if let Ok(metadata) = fs::metadata(root)
            && !metadata.is_dir()
        {
            problems.push(format!("{} is not a directory", root.display()));
        }
        Self {
            root,
            writes: Vec::new(),
            deletions: Vec::new(),
            problems,
            planned: BTreeSet::new(),
        }
    }

    /// Plans a write to `path`, which must be absent or provably owned.
    fn write(&mut self, path: PathBuf, content: String, proof: Proof) {
        match occupant(&path, proof) {
            Occupant::Absent | Occupant::OwnedFile => {}
            Occupant::OwnedDir | Occupant::Foreign { is_dir: true, .. } => self
                .problems
                .push(format!("{} is a directory where a file is planned", path.display())),
            Occupant::Foreign { reason, .. } => {
                self.problems.push(format!("{} {reason}", path.display()));
            }
        }
        self.writes.push(Mutation::Write { path, content });
    }

    /// Plans a top-level `.rs` file.
    fn file(&mut self, filename: &str, content: String) {
        self.planned.insert(filename.to_string());
        self.write(self.root.join(filename), content, Proof::RustSource);
    }

    fn module(&mut self, module: &GeneratedModule) {
        match module {
            GeneratedModule::SingleFile { filename, content } => {
                self.file(filename, content.clone());
                self.replace(self.root.join(module.module_name()));
            }
            GeneratedModule::Directory {
                module_path,
                mod_rs,
                requests_rs,
                responses_rs,
            } => {
                self.planned.insert(module_path.clone());
                let dir = self.root.join(module_path);
                let files = [
                    ("mod.rs", mod_rs),
                    ("requests.rs", requests_rs),
                    ("responses.rs", responses_rs),
                ];
                let is_real_dir = fs::symlink_metadata(&dir).is_ok_and(|m| m.is_dir());
                if !is_real_dir && dir.symlink_metadata().is_ok() {
                    self.problems.push(format!(
                        "{} is not a directory where module `{module_path}` is planned",
                        dir.display()
                    ));
                }
                for (name, content) in files {
                    self.write(dir.join(name), content.clone(), Proof::RustSource);
                }
                if is_real_dir {
                    self.remove_stale_in(&dir, &files.map(|(name, _)| name));
                }
                self.replace(self.root.join(format!("{module_path}.rs")));
            }
        }
    }

    /// Deletes the other layout of a regenerated module once its new layout
    /// is written. Content the generator did not write is a conflict.
    fn replace(&mut self, obsolete: PathBuf) {
        match occupant(&obsolete, Proof::RustSource) {
            Occupant::Absent => {}
            Occupant::OwnedFile => self.delete(Mutation::RemoveFile { path: obsolete }),
            Occupant::OwnedDir => self.delete(Mutation::RemoveDir { path: obsolete }),
            Occupant::Foreign { reason, .. } => self.problems.push(format!(
                "{} must be replaced by the module's new layout, but it {reason}; it is left intact",
                obsolete.display()
            )),
        }
    }

    /// Deletes owned files and directories inside a regenerated directory
    /// module that this run does not write.
    fn remove_stale_in(&mut self, dir: &Path, keep: &[&str]) {
        match list_entries(dir) {
            Ok(entries) => {
                for name in entries.unwrap_or_default() {
                    if !keep.contains(&name.as_str()) {
                        self.remove_if_owned(dir.join(name));
                    }
                }
            }
            Err(problem) => self.problems.push(problem),
        }
    }

    /// Full generation: deletes owned top-level modules no API produces.
    fn remove_obsolete_modules(&mut self) {
        match list_entries(self.root) {
            Ok(entries) => {
                for name in entries.unwrap_or_default() {
                    if !self.planned.contains(&name) {
                        self.remove_if_owned(self.root.join(name));
                    }
                }
            }
            Err(problem) => self.problems.push(problem),
        }
    }

    fn remove_if_owned(&mut self, path: PathBuf) {
        match occupant(&path, Proof::RustSource) {
            Occupant::OwnedFile => self.delete(Mutation::RemoveFile { path }),
            Occupant::OwnedDir => self.delete(Mutation::RemoveDir { path }),
            Occupant::Absent | Occupant::Foreign { .. } => {}
        }
    }

    /// Plans a deletion once, however many rules find the same path.
    fn delete(&mut self, deletion: Mutation) {
        if !self.deletions.iter().any(|planned| planned.path() == deletion.path()) {
            self.deletions.push(deletion);
        }
    }

    fn finish(self) -> Result<MutationPlan, GeneratorError> {
        if self.problems.is_empty() {
            Ok(MutationPlan::new(self.root, self.writes, self.deletions))
        } else {
            Err(GeneratorError::UnsafeOutput {
                root: self.root.display().to_string(),
                problems: self.problems,
                remedy: CONFLICT_REMEDY.to_string(),
            })
        }
    }
}

/// Plans the publication of one export root (OpenAPI or Postman).
///
/// `files` are the artifacts to write, by file name. Each must land on an
/// absent path or a regular file; a directory or symbolic link there is a
/// conflict. Export artifacts carry no header, so a regular file whose name
/// `is_artifact` accepts counts as generator-owned. A `complete` run deletes
/// such files that no module produced; a partial run deletes nothing.
///
/// ## Errors
///
/// [`GeneratorError::UnsafeOutput`] listing every conflict.
pub(crate) fn prepare_export_output(
    root: &Path,
    files: Vec<(String, String)>,
    complete: bool,
    is_artifact: impl Fn(&str) -> bool,
) -> Result<MutationPlan, GeneratorError> {
    let mut planner = Planner::new(root);
    for (name, content) in files {
        planner.planned.insert(name.clone());
        planner.write(root.join(name), content, Proof::Artifact);
    }
    if complete {
        match list_entries(root) {
            Ok(entries) => {
                for name in entries.unwrap_or_default() {
                    let path = root.join(&name);
                    if !planner.planned.contains(&name)
                        && is_artifact(&name)
                        && occupant(&path, Proof::Artifact) == Occupant::OwnedFile
                    {
                        planner.delete(Mutation::RemoveFile { path });
                    }
                }
            }
            Err(problem) => planner.problems.push(problem),
        }
    }
    planner.finish()
}

/// Generates and writes standalone API code (for imported OpenAPI specs).
///
/// Unlike `generate_and_write`, this function:
/// - Does NOT include `pub use schematic_definitions::...` imports
/// - Uses `pub use crate::types::*` to reference locally generated types
/// - Adds `pub mod types;` to lib.rs if types were generated
///
/// The same ownership rules as [`prepare_rust_output`] apply: a planned path
/// holding content the generator did not write fails before any write.
///
/// ## Arguments
///
/// * `api` - The API definition to generate code for
/// * `output_dir` - Directory to write generated files to
/// * `dry_run` - If true, print code instead of writing files
/// * `has_types` - If true, include `pub mod types;` in lib.rs
///
/// ## Returns
///
/// The formatted API module code.
///
/// ## Errors
///
/// Returns an error if:
/// - [`validate_rust_client`] reports an error ([`GeneratorError::Validation`])
/// - Code generation produces invalid Rust
/// - A planned path is not owned ([`GeneratorError::UnsafeOutput`])
/// - File writing fails ([`GeneratorError::PublicationFailed`])
pub fn generate_and_write_standalone(
    api: &RestApi,
    output_dir: &Path,
    dry_run: bool,
    has_types: bool,
) -> Result<String, GeneratorError> {
    let apis = [api];
    ensure_valid_rust_batch(&apis)?;
    let options = OutputOptions {
        standalone: true,
        include_types_module: has_types,
    };

    let lib = render(&assemble_lib_rs_with_options(&apis, &options))?;
    let shared = render(&assemble_shared_module())?;
    let prelude = render(&assemble_prelude_with_options(&apis, false))?;
    let api_formatted = render(&assemble_api_module_with_options(api, &options))?;

    // The import path writes the API as a single `{module}.rs` file; an owned
    // directory-form `{module}/` left by a prior run is removed after it, so
    // `foo.rs` never stays beside `foo/`, which Rust rejects as ambiguous.
    let mut planner = Planner::new(output_dir);
    planner.module(&GeneratedModule::SingleFile {
        filename: format!("{}.rs", get_module_path(api)),
        content: api_formatted.clone(),
    });
    planner.file("shared.rs", shared);
    planner.file("prelude.rs", prelude);
    planner.file("lib.rs", lib);
    let plan = planner.finish()?;
    if dry_run {
        plan.print_dry_run(true);
    } else {
        plan.publish()?;
    }

    Ok(api_formatted)
}

#[cfg(test)]
mod tests {
    use super::*;
    use schematic_define::{
        ApiRequest, ApiResponse, AuthStrategy, CrossOrigin, Endpoint, RestMethod,
    };
    use std::fs;
    use tempfile::TempDir;

    fn make_simple_api() -> RestApi {
        RestApi {
            name: "TestApi".to_string(),
            description: "Test API".to_string(),
            base_url: "https://api.test.com/v1".to_string(),
            docs_url: None,
            auth: AuthStrategy::None,
            auth_policy: None,
            env_auth: vec![],
            env_username: None,
            env_mapping: None,
            headers: vec![],
            endpoints: vec![Endpoint {
                id: "ListItems".to_string(),
                method: RestMethod::Get,
                path: "/items".to_string(),
                description: "List all items".to_string(),
                request: None,
                response: ApiResponse::json_type("ListItemsResponse"),
                headers: vec![],
                params: None,
                oauth_scopes: None,
            }],
            module_path: None,
            request_suffix: None,
            version: None,
            cross_origin: CrossOrigin::default(),
            request_id_header: None,
            secret_names: vec![],
        }
    }

    fn make_complex_api() -> RestApi {
        RestApi {
            name: "OpenAI".to_string(),
            description: "OpenAI REST API".to_string(),
            base_url: "https://api.openai.com/v1".to_string(),
            docs_url: Some("https://platform.openai.com/docs".to_string()),
            auth: AuthStrategy::BearerToken { header: None },
            auth_policy: None,
            env_auth: vec!["OPENAI_API_KEY".to_string()],
            env_username: None,
            env_mapping: None,
            headers: vec![],
            endpoints: vec![
                Endpoint {
                    id: "ListModels".to_string(),
                    method: RestMethod::Get,
                    path: "/models".to_string(),
                    description: "Lists available models".to_string(),
                    request: None,
                    response: ApiResponse::json_type("ListModelsResponse"),
                    headers: vec![],
                    params: None,
                    oauth_scopes: None,
                },
                Endpoint {
                    id: "RetrieveModel".to_string(),
                    method: RestMethod::Get,
                    path: "/models/{model}".to_string(),
                    description: "Retrieves a model".to_string(),
                    request: None,
                    response: ApiResponse::json_type("Model"),
                    headers: vec![],
                    params: None,
                    oauth_scopes: None,
                },
                Endpoint {
                    id: "CreateCompletion".to_string(),
                    method: RestMethod::Post,
                    path: "/completions".to_string(),
                    description: "Creates a completion".to_string(),
                    request: Some(ApiRequest::json_type("CreateCompletionBody")),
                    response: ApiResponse::json_type("Completion"),
                    headers: vec![],
                    params: None,
                    oauth_scopes: None,
                },
            ],
            module_path: None,
            request_suffix: None,
            version: None,
            cross_origin: CrossOrigin::default(),
            request_id_header: None,
            secret_names: vec![],
        }
    }

    #[test]
    fn assemble_api_code_produces_valid_tokenstream() {
        let api = make_simple_api();
        let tokens = assemble_api_code(&api);
        assert!(!tokens.is_empty());
    }

    #[test]
    fn assemble_api_code_includes_all_components() {
        let api = make_complex_api();
        let tokens = assemble_api_code(&api);
        let code = tokens.to_string();

        assert!(code.contains("EndpointSpec for ListModelsRequest"));
        assert!(code.contains("EndpointSpec for RetrieveModelRequest"));
        assert!(code.contains("EndpointSpec for CreateCompletionRequest"));
        assert!(code.contains("struct OpenAI"));
        assert!(code.contains("enum OpenAIApi"));
        assert!(!code.contains("OpenAIClient"));
        // Sending is the runtime client's job; the module has no send path.
        assert!(!code.contains("OpenAIRequest"));
        assert!(!code.contains("SchematicError"));
        assert!(!code.contains("async fn request"));
    }

    #[test]
    fn assemble_api_code_includes_imports() {
        let api = make_simple_api();
        let tokens = assemble_api_code(&api);
        let code = tokens.to_string();
        assert!(code.contains("serde"));
    }

    #[test]
    fn assemble_api_code_has_no_unnecessary_lint_allows() {
        let api = make_simple_api();
        let tokens = assemble_api_code(&api);
        let code = tokens.to_string();
        assert!(!code.contains("dead_code"));
        assert!(!code.contains("unused_imports"));
    }

    #[test]
    fn validate_code_accepts_valid_code() {
        let api = make_simple_api();
        let tokens = assemble_api_code(&api);
        let result = validate_code(&tokens);
        assert!(result.is_ok());
    }

    #[test]
    fn validate_code_accepts_complex_api() {
        let api = make_complex_api();
        let tokens = assemble_api_code(&api);
        let result = validate_code(&tokens);
        assert!(result.is_ok());
    }

    #[test]
    fn validate_code_rejects_invalid_code() {
        use quote::quote;
        let invalid_tokens = quote! {
            let x =
        };
        let result = validate_code(&invalid_tokens);
        assert!(result.is_err());
        match result {
            Err(GeneratorError::CodeGenError(_)) => {}
            Err(other) => panic!("Unexpected error type: {:?}", other),
            Ok(_) => panic!("Expected error but got success"),
        }
    }

    #[test]
    fn format_code_produces_string() {
        let api = make_simple_api();
        let tokens = assemble_api_code(&api);
        let file = validate_code(&tokens).unwrap();
        let formatted = format_code(&file);
        assert!(!formatted.is_empty());
    }

    #[test]
    fn format_code_produces_readable_output() {
        let api = make_simple_api();
        let tokens = assemble_api_code(&api);
        let file = validate_code(&tokens).unwrap();
        let formatted = format_code(&file);
        assert!(formatted.contains('\n'));
        assert!(formatted.contains("///") || formatted.contains("//!"));
    }

    #[test]
    fn format_code_preserves_structure() {
        let api = make_complex_api();
        let tokens = assemble_api_code(&api);
        let file = validate_code(&tokens).unwrap();
        let formatted = format_code(&file);
        assert!(formatted.contains("use serde::{Deserialize, Serialize};"));
        assert!(formatted.contains("pub struct OpenAI(schematic_runtime::Client<OpenAIApi>);"));
        assert!(formatted.contains("pub enum OpenAIApi {}"));
        assert!(formatted.contains("impl schematic_runtime::EndpointSpec for ListModelsRequest {"));
    }

    #[test]
    fn write_atomic_creates_file() {
        let temp_dir = TempDir::new().unwrap();
        let file_path = temp_dir.path().join("test.rs");
        let content = "// Test content";
        let result = write_atomic(&file_path, content);
        assert!(result.is_ok());
        assert!(file_path.exists());
        let read_content = fs::read_to_string(&file_path).unwrap();
        assert_eq!(read_content, content);
    }

    #[test]
    fn write_atomic_creates_parent_directories() {
        let temp_dir = TempDir::new().unwrap();
        let file_path = temp_dir.path().join("nested/deep/test.rs");
        let content = "// Nested content";
        let result = write_atomic(&file_path, content);
        assert!(result.is_ok());
        assert!(file_path.exists());
    }

    #[test]
    fn write_atomic_overwrites_existing_file() {
        let temp_dir = TempDir::new().unwrap();
        let file_path = temp_dir.path().join("existing.rs");
        fs::write(&file_path, "// Old content").unwrap();
        let new_content = "// New content";
        let result = write_atomic(&file_path, new_content);
        assert!(result.is_ok());
        let read_content = fs::read_to_string(&file_path).unwrap();
        assert_eq!(read_content, new_content);
    }

    #[test]
    fn write_atomic_no_temp_file_left_behind() {
        let temp_dir = TempDir::new().unwrap();
        let file_path = temp_dir.path().join("clean.rs");
        write_atomic(&file_path, "// Content").unwrap();
        let temp_path = file_path.with_extension("tmp");
        assert!(!temp_path.exists());
    }

    #[test]
    fn generate_and_write_dry_run_returns_code() {
        let api = make_simple_api();
        let temp_dir = TempDir::new().unwrap();
        let result = generate_and_write(&api, temp_dir.path(), true);
        assert!(result.is_ok());
        let code = result.unwrap();
        assert!(code.contains("pub struct TestApi(schematic_runtime::Client<TestApiApi>);"));
    }

    #[test]
    fn generate_and_write_dry_run_no_file_created() {
        let api = make_simple_api();
        let temp_dir = TempDir::new().unwrap();
        generate_and_write(&api, temp_dir.path(), true).unwrap();
        let output_path = temp_dir.path().join("lib.rs");
        assert!(!output_path.exists());
    }

    #[test]
    fn generate_and_write_creates_lib_rs() {
        let api = make_simple_api();
        let temp_dir = TempDir::new().unwrap();
        let result = generate_and_write(&api, temp_dir.path(), false);
        assert!(result.is_ok());
        let output_path = temp_dir.path().join("lib.rs");
        assert!(output_path.exists());
    }

    #[test]
    fn generate_and_write_file_contains_formatted_code() {
        let api = make_complex_api();
        let temp_dir = TempDir::new().unwrap();
        generate_and_write(&api, temp_dir.path(), false).unwrap();
        let api_module_path = temp_dir.path().join("openai.rs");
        let content = fs::read_to_string(api_module_path).unwrap();
        assert!(content.contains("    "));
        assert!(content.contains("pub struct OpenAI(schematic_runtime::Client<OpenAIApi>);"));
        assert!(content.contains("pub enum OpenAIApi {}"));
        assert!(!content.contains("OpenAIClient"));
        assert!(!content.contains("crate::shared::"));
        let shared_path = temp_dir.path().join("shared.rs");
        let shared_content = fs::read_to_string(shared_path).unwrap();
        assert!(shared_content.contains("pub fn combine_cookie_headers"));
        assert!(!shared_content.contains("pub type RequestParts"));
        assert!(!shared_content.contains("pub enum SchematicError"));
        let lib_path = temp_dir.path().join("lib.rs");
        let lib_content = fs::read_to_string(lib_path).unwrap();
        assert!(lib_content.contains("pub use schematic_runtime;"));
        assert!(lib_content.contains("pub mod shared;"));
        assert!(lib_content.contains("pub mod openai;"));
        assert!(lib_content.contains("pub mod prelude;"));
    }

    #[test]
    fn generate_and_write_returns_same_as_file_content() {
        let api = make_simple_api();
        let temp_dir = TempDir::new().unwrap();
        let returned = generate_and_write(&api, temp_dir.path(), false).unwrap();
        let api_module_path = temp_dir.path().join("test.rs");
        let file_content = fs::read_to_string(api_module_path).unwrap();
        assert_eq!(returned, file_content);
    }

    #[test]
    fn generate_and_write_creates_nested_output_dir() {
        let api = make_simple_api();
        let temp_dir = TempDir::new().unwrap();
        let nested_dir = temp_dir.path().join("src/generated");
        let result = generate_and_write(&api, &nested_dir, false);
        assert!(result.is_ok());
        let output_path = nested_dir.join("lib.rs");
        assert!(output_path.exists());
    }

    #[test]
    fn full_pipeline_with_all_auth_strategies() {
        let test_cases: Vec<(AuthStrategy, Vec<String>, Option<String>)> = vec![
            (AuthStrategy::None, vec![], None),
            (
                AuthStrategy::BearerToken { header: None },
                vec!["TOKEN".to_string()],
                None,
            ),
            (
                AuthStrategy::ApiKey {
                    header: "X-API-Key".to_string(),
                    value_prefix: None,
                },
                vec!["KEY".to_string()],
                None,
            ),
            (
                AuthStrategy::Basic,
                vec!["PASS".to_string()],
                Some("USER".to_string()),
            ),
        ];

        for (auth, env_auth, env_username) in test_cases {
            let api = RestApi {
                name: "TestApi".to_string(),
                description: "Test".to_string(),
                base_url: "https://test.com".to_string(),
                docs_url: None,
                auth: auth.clone(),
                auth_policy: None,
                env_auth,
                env_username,
                env_mapping: None,
                headers: vec![],
                endpoints: vec![Endpoint {
                    id: "Test".to_string(),
                    method: RestMethod::Get,
                    path: "/test".to_string(),
                    description: "Test endpoint".to_string(),
                    request: None,
                    response: ApiResponse::json_type("TestResponse"),
                    headers: vec![],
                    params: None,
                    oauth_scopes: None,
                }],
                module_path: None,
                request_suffix: None,
                version: None,
                cross_origin: CrossOrigin::default(),
                request_id_header: None,
                secret_names: vec![],
            };

            let temp_dir = TempDir::new().unwrap();
            let result = generate_and_write(&api, temp_dir.path(), false);
            assert!(result.is_ok(), "Failed for auth strategy: {:?}", auth);
        }
    }

    #[test]
    fn full_pipeline_with_all_http_methods() {
        let methods = [
            RestMethod::Get,
            RestMethod::Post,
            RestMethod::Put,
            RestMethod::Patch,
            RestMethod::Delete,
            RestMethod::Head,
            RestMethod::Options,
        ];

        let endpoints: Vec<Endpoint> = methods
            .iter()
            .enumerate()
            .map(|(i, method)| Endpoint {
                id: format!("Endpoint{}", i),
                method: *method,
                path: format!("/path{}", i),
                description: format!("{:?} endpoint", method),
                request: None,
                response: ApiResponse::json_type("Response"),
                headers: vec![],
                params: None,
                oauth_scopes: None,
            })
            .collect();

        let api = RestApi {
            name: "AllMethods".to_string(),
            description: "API with all HTTP methods".to_string(),
            base_url: "https://test.com".to_string(),
            docs_url: None,
            auth: AuthStrategy::None,
            auth_policy: None,
            env_auth: vec![],
            env_username: None,
            env_mapping: None,
            headers: vec![],
            endpoints,
            module_path: None,
            request_suffix: None,
            version: None,
            cross_origin: CrossOrigin::default(),
            request_id_header: None,
            secret_names: vec![],
        };

        let temp_dir = TempDir::new().unwrap();
        let result = generate_and_write(&api, temp_dir.path(), false);
        assert!(result.is_ok());

        let content = result.unwrap();
        for method in methods {
            let method_str = format!("{:?}", method).to_uppercase();
            assert!(
                content.contains(&format!(
                    "const METHOD: schematic_runtime::Method = schematic_runtime::Method::{method_str};"
                )),
                "Missing method: {}",
                method_str
            );
        }
    }

    #[test]
    fn full_pipeline_empty_api_produces_valid_code() {
        let api = RestApi {
            name: "EmptyApi".to_string(),
            description: "API with no endpoints".to_string(),
            base_url: "https://empty.com".to_string(),
            docs_url: None,
            auth: AuthStrategy::None,
            auth_policy: None,
            env_auth: vec![],
            env_username: None,
            env_mapping: None,
            headers: vec![],
            endpoints: vec![],
            module_path: None,
            request_suffix: None,
            version: None,
            cross_origin: CrossOrigin::default(),
            request_id_header: None,
            secret_names: vec![],
        };

        let temp_dir = TempDir::new().unwrap();
        let result = generate_and_write(&api, temp_dir.path(), false);
        assert!(result.is_ok());
    }

    #[test]
    fn generated_code_has_module_documentation() {
        let api = make_simple_api();
        let temp_dir = TempDir::new().unwrap();
        let code = generate_and_write(&api, temp_dir.path(), true).unwrap();
        assert!(code.contains("//!"));
        assert!(code.contains("Generated API client"));
        assert!(code.starts_with("// This code was automatically generated"));
        assert!(code.contains("Do not edit manually"));
    }

    fn make_shared_module_apis() -> (RestApi, RestApi) {
        let api_a = RestApi {
            name: "FooNative".to_string(),
            description: "Foo native API".to_string(),
            base_url: "http://localhost:8080".to_string(),
            docs_url: None,
            auth: AuthStrategy::None,
            auth_policy: None,
            env_auth: vec![],
            env_username: None,
            env_mapping: None,
            headers: vec![],
            endpoints: vec![Endpoint {
                id: "ListItems".to_string(),
                method: RestMethod::Get,
                path: "/api/items".to_string(),
                description: "List items".to_string(),
                request: None,
                response: ApiResponse::json_type("ListItemsResponse"),
                headers: vec![],
                params: None,
                oauth_scopes: None,
            }],
            module_path: Some("foo".to_string()),
            request_suffix: Some("NativeRequest".to_string()),
            version: None,
            cross_origin: CrossOrigin::default(),
            request_id_header: None,
            secret_names: vec![],
        };

        let api_b = RestApi {
            name: "FooCompat".to_string(),
            description: "Foo compat API".to_string(),
            base_url: "http://localhost:8080".to_string(),
            docs_url: None,
            auth: AuthStrategy::None,
            auth_policy: None,
            env_auth: vec![],
            env_username: None,
            env_mapping: None,
            headers: vec![],
            endpoints: vec![Endpoint {
                id: "ListItems".to_string(),
                method: RestMethod::Get,
                path: "/v1/items".to_string(),
                description: "List items (compat)".to_string(),
                request: None,
                response: ApiResponse::json_type("CompatListItemsResponse"),
                headers: vec![],
                params: None,
                oauth_scopes: None,
            }],
            module_path: Some("foo".to_string()),
            request_suffix: Some("CompatRequest".to_string()),
            version: None,
            cross_origin: CrossOrigin::default(),
            request_id_header: None,
            secret_names: vec![],
        };

        (api_a, api_b)
    }

    #[test]
    fn assemble_lib_rs_deduplicates_modules() {
        let (api_a, api_b) = make_shared_module_apis();
        let apis: Vec<&RestApi> = vec![&api_a, &api_b];
        let tokens = assemble_lib_rs(&apis);
        let code = tokens.to_string();
        let count = code.matches("pub mod foo").count();
        assert_eq!(
            count, 1,
            "Expected exactly 1 'pub mod foo', found {}",
            count
        );
    }

    #[test]
    fn assemble_combined_module_includes_both_apis() {
        let (api_a, api_b) = make_shared_module_apis();
        let apis: Vec<&RestApi> = vec![&api_a, &api_b];
        let tokens = assemble_combined_api_module(&apis);
        let file = validate_code(&tokens).unwrap();
        let code = format_code(&file);
        assert!(code.contains("pub struct FooNative("), "Missing FooNative client");
        assert!(code.contains("pub struct FooCompat("), "Missing FooCompat client");
        // Each API's requests name their own marker, which scopes them to its client.
        assert!(
            code.contains("type Api = FooNativeApi;"),
            "Missing FooNative request scope"
        );
        assert!(
            code.contains("type Api = FooCompatApi;"),
            "Missing FooCompat request scope"
        );
        let import_count = code
            .matches("pub use schematic_definitions::foo::*")
            .count();
        assert_eq!(
            import_count, 1,
            "Expected exactly 1 definitions import, found {}",
            import_count
        );
        let serde_count = code
            .matches("use serde::{Deserialize, Serialize};")
            .count();
        assert_eq!(
            serde_count, 1,
            "Expected exactly 1 serde import, found {}",
            serde_count
        );
    }

    #[test]
    fn generate_and_write_all_groups_shared_modules() {
        let (api_a, api_b) = make_shared_module_apis();
        let standalone = make_simple_api();
        let apis: Vec<&RestApi> = vec![&standalone, &api_a, &api_b];
        let temp_dir = TempDir::new().unwrap();
        let result = generate_and_write_all(&apis, temp_dir.path(), false);
        assert!(result.is_ok());
        assert!(temp_dir.path().join("foo.rs").exists(), "Missing foo.rs");
        assert!(
            !temp_dir.path().join("foonative.rs").exists(),
            "Unexpected foonative.rs"
        );
        assert!(
            !temp_dir.path().join("foocompat.rs").exists(),
            "Unexpected foocompat.rs"
        );
        let foo_content = fs::read_to_string(temp_dir.path().join("foo.rs")).unwrap();
        assert!(foo_content.contains("pub struct FooNative("));
        assert!(foo_content.contains("pub struct FooCompat("));
        assert!(temp_dir.path().join("test.rs").exists(), "Missing test.rs");
    }

    #[test]
    fn generate_and_write_all_cleans_stale_files() {
        let api = make_simple_api();
        let apis: Vec<&RestApi> = vec![&api];
        let temp_dir = TempDir::new().unwrap();
        // Stale files from a prior generation carry the generated header, so
        // they are owned and get removed.
        let stale = format!("{GENERATED_FILE_HEADER}\n\n// stale");
        fs::write(temp_dir.path().join("ollamaopenai.rs"), &stale).unwrap();
        fs::write(temp_dir.path().join("old_api.rs"), &stale).unwrap();
        fs::write(temp_dir.path().join("Cargo.toml"), "# keep").unwrap();
        generate_and_write_all(&apis, temp_dir.path(), false).unwrap();
        assert!(
            !temp_dir.path().join("ollamaopenai.rs").exists(),
            "ollamaopenai.rs should be deleted"
        );
        assert!(
            !temp_dir.path().join("old_api.rs").exists(),
            "old_api.rs should be deleted"
        );
        assert!(
            temp_dir.path().join("Cargo.toml").exists(),
            "Cargo.toml should be preserved"
        );
        assert!(temp_dir.path().join("lib.rs").exists());
        assert!(temp_dir.path().join("shared.rs").exists());
        assert!(temp_dir.path().join("prelude.rs").exists());
        assert!(temp_dir.path().join("test.rs").exists());
    }

    #[test]
    fn cleanup_preserves_hand_written_files_and_foreign_dirs() {
        let api = make_simple_api();
        let apis: Vec<&RestApi> = vec![&api];
        let temp_dir = TempDir::new().unwrap();

        // A hand-written module WITHOUT the generated header must survive.
        let hand_written = temp_dir.path().join("handwritten.rs");
        fs::write(&hand_written, "pub fn keep_me() {}\n").unwrap();

        // An unrelated subdirectory (e.g. a binary crate) must survive intact.
        let bin_dir = temp_dir.path().join("bin");
        fs::create_dir_all(&bin_dir).unwrap();
        let bin_main = bin_dir.join("main.rs");
        fs::write(&bin_main, "fn main() {}\n").unwrap();

        // A directory that mixes generator output with a hand-written file must
        // survive: it is not provably generator-owned.
        let mixed_dir = temp_dir.path().join("mixed");
        fs::create_dir_all(&mixed_dir).unwrap();
        fs::write(
            mixed_dir.join("generated.rs"),
            format!("{GENERATED_FILE_HEADER}\n\n// gen"),
        )
        .unwrap();
        let mixed_keep = mixed_dir.join("keep.rs");
        fs::write(&mixed_keep, "pub fn keep() {}\n").unwrap();

        generate_and_write_all(&apis, temp_dir.path(), false).unwrap();

        assert!(hand_written.exists(), "hand-written .rs must survive");
        assert!(bin_dir.is_dir(), "unrelated bin/ dir must survive");
        assert!(bin_main.exists(), "bin/main.rs must survive");
        assert!(
            mixed_dir.is_dir() && mixed_keep.exists(),
            "a dir containing hand-written content must survive"
        );
    }

    #[test]
    fn standalone_import_removes_conflicting_owned_directory() {
        let api = make_simple_api();
        let temp_dir = TempDir::new().unwrap();
        let module = get_module_path(&api);

        // Simulate a prior directory-form module owned by the generator.
        let dir = temp_dir.path().join(&module);
        fs::create_dir_all(&dir).unwrap();
        fs::write(
            dir.join("mod.rs"),
            format!("{GENERATED_FILE_HEADER}\n\n// old"),
        )
        .unwrap();

        generate_and_write_standalone(&api, temp_dir.path(), false, false).unwrap();

        assert!(
            !dir.is_dir(),
            "prior directory-form module must be removed to avoid foo.rs/foo ambiguity"
        );
        assert!(
            temp_dir.path().join(format!("{module}.rs")).exists(),
            "single-file module should be written"
        );
    }

    #[test]
    fn standalone_import_preserves_hand_written_conflicting_directory() {
        let api = make_simple_api();
        let temp_dir = TempDir::new().unwrap();
        let module = get_module_path(&api);

        // A conflicting directory that is NOT generator-owned is a conflict:
        // the run refuses before writing anything and leaves it in place.
        let dir = temp_dir.path().join(&module);
        fs::create_dir_all(&dir).unwrap();
        let keep = dir.join("keep.rs");
        fs::write(&keep, "pub fn keep() {}\n").unwrap();

        let err = generate_and_write_standalone(&api, temp_dir.path(), false, false).unwrap_err();

        assert!(matches!(err, GeneratorError::UnsafeOutput { .. }), "{err:?}");
        assert!(keep.exists(), "hand-written directory content must survive");
        assert!(
            !temp_dir.path().join(format!("{module}.rs")).exists(),
            "nothing is written when a conflict is found"
        );
    }

    #[test]
    fn prelude_exports_all_apis_in_shared_module() {
        let (api_a, api_b) = make_shared_module_apis();
        let apis: Vec<&RestApi> = vec![&api_a, &api_b];
        let tokens = assemble_prelude(&apis);
        let file = validate_code(&tokens).unwrap();
        let code = format_code(&file);
        assert!(
            code.contains("pub use crate::foo::FooNative;"),
            "Missing FooNative in prelude:\n{code}"
        );
        assert!(
            code.contains("pub use crate::foo::FooCompat;"),
            "Missing FooCompat in prelude:\n{code}"
        );
        assert!(
            !code.contains("FooNativeRequest") && !code.contains("FooCompatRequest"),
            "Request types are not re-exported by the prelude:\n{code}"
        );
    }

    fn make_large_api() -> RestApi {
        let endpoints: Vec<Endpoint> = (0..500)
            .map(|i| Endpoint {
                id: format!("Endpoint{}", i),
                method: if i % 5 == 0 {
                    RestMethod::Post
                } else {
                    RestMethod::Get
                },
                path: if i % 3 == 0 {
                    format!("/endpoint/{}/{{param}}", i)
                } else {
                    format!("/endpoint/{}", i)
                },
                description: format!("Endpoint {} description with enough text to add some length to the generated documentation comments", i),
                request: if i % 5 == 0 {
                    Some(ApiRequest::json_type(format!("Endpoint{}Body", i)))
                } else {
                    None
                },
                response: ApiResponse::json_type("TestResponse"),
                headers: vec![],
                params: None,
                oauth_scopes: None,
            })
            .collect();

        RestApi {
            name: "LargeApi".to_string(),
            description: "A large API for testing split output".to_string(),
            base_url: "https://api.large.com/v1".to_string(),
            docs_url: Some("https://docs.large.com".to_string()),
            auth: AuthStrategy::BearerToken { header: None },
            auth_policy: None,
            env_auth: vec!["LARGE_API_KEY".to_string()],
            env_username: None,
            env_mapping: None,
            headers: vec![],
            endpoints,
            module_path: None,
            request_suffix: None,
            version: None,
            cross_origin: CrossOrigin::default(),
            request_id_header: None,
            secret_names: vec![],
        }
    }

    #[test]
    fn large_api_produces_split_directory() {
        let api = make_large_api();
        let temp_dir = TempDir::new().unwrap();
        generate_and_write(&api, temp_dir.path(), false).unwrap();

        let single_file = temp_dir.path().join("large.rs");
        let dir = temp_dir.path().join("large");

        if single_file.exists() && !dir.is_dir() {
            let content = fs::read_to_string(&single_file).unwrap();
            let line_count = content.lines().count();
            panic!(
                "Generated as single file ({} lines, threshold is {}). \
                 Increase endpoint count in make_large_api().",
                line_count, SPLIT_THRESHOLD_LINES
            );
        }

        assert!(dir.is_dir(), "large should be a directory");
        assert!(dir.join("mod.rs").exists(), "large/mod.rs should exist");
        assert!(
            dir.join("requests.rs").exists(),
            "large/requests.rs should exist"
        );
        assert!(
            dir.join("responses.rs").exists(),
            "large/responses.rs should exist"
        );
        assert!(
            !dir.join("client.rs").exists(),
            "large/client.rs should not exist: the runtime client sends"
        );

        assert!(
            !temp_dir.path().join("large.rs").exists(),
            "large.rs should NOT exist (directory takes precedence)"
        );

        let mod_rs = fs::read_to_string(dir.join("mod.rs")).unwrap();
        assert!(
            mod_rs.contains("pub struct LargeApi(schematic_runtime::Client<LargeApiApi>);"),
            "mod.rs should contain the API client struct"
        );
        assert!(
            mod_rs.contains("impl LargeApi {"),
            "mod.rs should contain the API's constructors"
        );
        assert!(
            mod_rs.contains("pub mod requests;"),
            "mod.rs should declare requests submodule"
        );
        assert!(
            mod_rs.contains("pub use requests::*;"),
            "mod.rs should re-export requests"
        );

        let requests = fs::read_to_string(dir.join("requests.rs")).unwrap();
        assert!(
            requests.contains("pub struct Endpoint0Request"),
            "requests.rs should contain request structs"
        );
        assert!(
            requests.contains("type Api = LargeApiApi;"),
            "requests.rs should scope its requests to the API tag"
        );
    }

    #[test]
    fn split_module_public_api_is_accessible() {
        let api = make_large_api();
        let temp_dir = TempDir::new().unwrap();
        generate_and_write(&api, temp_dir.path(), false).unwrap();

        let dir = temp_dir.path().join("large");

        let mod_rs = fs::read_to_string(dir.join("mod.rs")).unwrap();
        assert!(
            !mod_rs.contains("mod client;"),
            "mod.rs should not declare a client submodule"
        );
        assert!(
            mod_rs.contains("pub use requests::*;"),
            "mod.rs should re-export request items"
        );
        assert!(
            mod_rs.contains("pub use responses::*;"),
            "mod.rs should re-export response items"
        );

        let requests_rs = fs::read_to_string(dir.join("requests.rs")).unwrap();
        assert!(
            requests_rs.contains("use super::responses::*;"),
            "requests.rs should import the response types it names"
        );
        assert!(
            requests_rs.contains("use super::LargeApiApi;"),
            "requests.rs should import the API tag its requests name"
        );

        let responses = fs::read_to_string(dir.join("responses.rs")).unwrap();
        assert!(
            responses.contains("pub use"),
            "responses.rs should have pub use for definitions"
        );
    }

    #[test]
    fn split_to_single_file_transition_cleans_up() {
        let large_api = make_large_api();
        let mut small_api = make_simple_api();
        small_api.name = "LargeApi".to_string();
        let temp_dir = TempDir::new().unwrap();

        generate_and_write(&large_api, temp_dir.path(), false).unwrap();
        assert!(temp_dir.path().join("large").is_dir());

        generate_and_write(&small_api, temp_dir.path(), false).unwrap();
        assert!(
            temp_dir.path().join("large.rs").exists(),
            "Should now be a single file"
        );
        assert!(
            !temp_dir.path().join("large").is_dir(),
            "Directory should be cleaned up"
        );
    }
}
