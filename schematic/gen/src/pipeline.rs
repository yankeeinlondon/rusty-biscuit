//! API resolution and pipeline orchestration.
//!
//! Contains the core logic for resolving API definitions and orchestrating
//! the generation pipeline, independent of CLI argument parsing.

use std::collections::HashSet;
use std::path::Path;

use colored::Colorize;
use indexmap::IndexMap;
use schematic_define::RestApi;
use schematic_define::openapi::{ExportFormat, ExportOptions};
use schematic_definitions::registry::{SchemaRegistry, get_registries_for_module};

use crate::diagnostics::{Output, ValidationDiagnostic, has_errors};
use crate::errors::GeneratorError;
use crate::export::openapi::{build_openapi_grouped, render_openapi_document};
use crate::export::postman::render_postman_module;
use crate::output::{
    MutationPlan, PublicationReport, RustOutputRequest, Selection, expand_selection,
    prepare_export_output, prepare_rust_output,
};
use crate::validation::{
    validate_openapi, validate_openapi_registry, validate_postman, validate_rust_client,
};

/// List of available API names for error messages.
const AVAILABLE_APIS: &str = "anthropic, artificial-analysis-data, artificial-analysis-critpt, bitbucket, openai, elevenlabs, eversolo, gitea, github, gitlab, huggingface, lmstudio, ollama-native, ollama-openai, emqx-basic, emqx-bearer, samsung-smart-tv, unfolded-circle-core-rest, all";

/// Returns the available API names string.
pub fn available_apis() -> &'static str {
    AVAILABLE_APIS
}

/// Resolves an API name to its definition.
pub fn resolve_api(name: &str) -> Result<RestApi, GeneratorError> {
    use schematic_definitions::anthropic::define_anthropic_api;
    use schematic_definitions::artificial_analysis::{
        define_artificial_analysis_critpt_api, define_artificial_analysis_data_api,
    };
    use schematic_definitions::bitbucket::define_bitbucket_api;
    use schematic_definitions::elevenlabs::define_elevenlabs_rest_api;
    use schematic_definitions::emqx::{define_emqx_basic_api, define_emqx_bearer_api};
    use schematic_definitions::eversolo::define_eversolo_api;
    use schematic_definitions::gitea::define_gitea_api;
    use schematic_definitions::github::define_github_api;
    use schematic_definitions::gitlab::define_gitlab_api;
    use schematic_definitions::huggingface::define_huggingface_hub_api;
    use schematic_definitions::lmstudio::define_lmstudio_api;
    use schematic_definitions::ollama::{define_ollama_native_api, define_ollama_openai_api};
    use schematic_definitions::openai::define_openai_api;
    use schematic_definitions::samsung_smart_tv::define_samsung_smart_tv_api;
    use schematic_definitions::unfolded_circle::define_unfolded_circle_core_rest_api;

    match name {
        "anthropic" => Ok(define_anthropic_api()),
        "artificial-analysis-data" => Ok(define_artificial_analysis_data_api()),
        "artificial-analysis-critpt" => Ok(define_artificial_analysis_critpt_api()),
        "bitbucket" => Ok(define_bitbucket_api()),
        "openai" => Ok(define_openai_api()),
        "elevenlabs" => Ok(define_elevenlabs_rest_api()),
        "gitea" => Ok(define_gitea_api()),
        "github" => Ok(define_github_api()),
        "gitlab" => Ok(define_gitlab_api()),
        "huggingface" => Ok(define_huggingface_hub_api()),
        "lmstudio" => Ok(define_lmstudio_api()),
        "ollama-native" => Ok(define_ollama_native_api()),
        "ollama-openai" => Ok(define_ollama_openai_api()),
        "emqx-basic" => Ok(define_emqx_basic_api()),
        "emqx-bearer" => Ok(define_emqx_bearer_api()),
        "eversolo" => Ok(define_eversolo_api()),
        "samsung-smart-tv" => Ok(define_samsung_smart_tv_api()),
        "unfolded-circle-core-rest" => Ok(define_unfolded_circle_core_rest_api()),
        "all" => Err(GeneratorError::ConfigError(
            "Use resolve_all_apis() for 'all'".to_string(),
        )),
        other => Err(GeneratorError::ConfigError(format!(
            "Unknown API: '{}'. Available APIs: {}",
            other, AVAILABLE_APIS
        ))),
    }
}

/// Returns all available API definitions for batch generation.
pub fn resolve_all_apis() -> Vec<RestApi> {
    schematic_definitions::all_apis()
}

/// Resolves default export directories for OpenAPI and Postman.
///
/// If the output path ends with "schema/src" and no explicit output paths are provided,
/// defaults to sibling directories:
/// - OpenAPI: `<base>/openapi`
/// - Postman: `<base>/postman`
pub fn resolve_export_defaults(
    output: &str,
    openapi_out: Option<&str>,
    postman_out: Option<&str>,
) -> (Option<String>, Option<String>) {
    if openapi_out.is_some() || postman_out.is_some() {
        return (openapi_out.map(String::from), postman_out.map(String::from));
    }

    if output.ends_with("schema/src") {
        let base = output.strip_suffix("schema/src").unwrap_or("");
        let openapi_default = format!("{}openapi", base);
        let postman_default = format!("{}postman", base);
        (Some(openapi_default), Some(postman_default))
    } else {
        (None, None)
    }
}

/// OpenAPI output format for pipeline use.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub enum OpenApiFormat {
    /// JSON format (default).
    #[default]
    Json,
    /// YAML format.
    Yaml,
}

impl From<OpenApiFormat> for ExportFormat {
    fn from(format: OpenApiFormat) -> Self {
        match format {
            OpenApiFormat::Json => ExportFormat::Json,
            OpenApiFormat::Yaml => ExportFormat::Yaml,
        }
    }
}

/// Options for the generate command.
pub struct GenerateOpts<'a> {
    pub output: &'a str,
    pub dry_run: bool,
    pub verbose: u8,
    pub openapi_out: Option<&'a str>,
    pub openapi_format: OpenApiFormat,
    pub openapi_version: Option<&'a str>,
    pub postman_out: Option<&'a str>,
    pub no_openapi: bool,
    pub no_postman: bool,
}


/// Which part of the registry a batch regenerates.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum BatchScope {
    /// Every API. Each output root's obsolete generator-owned files are
    /// removed.
    Complete,
    /// These APIs, by name, expanded to their complete module groups. Every
    /// other module and artifact already in an output root is kept, and
    /// nothing outside the regenerated groups is deleted.
    Selection(Vec<String>),
}

/// The outcome of one requested output.
#[derive(Debug)]
pub struct OutputReport {
    /// Which output this is.
    pub output: Output,
    /// Everything its validator reported, warnings included.
    pub diagnostics: Vec<ValidationDiagnostic>,
    /// `Err` when validation, preparation, or publication failed.
    pub result: Result<(), GeneratorError>,
}

/// The outcome of every output a generate invocation requested.
#[derive(Debug, Default)]
pub struct GenerationReport {
    /// One entry per requested output, in the order they were attempted.
    pub outputs: Vec<OutputReport>,
}

impl GenerationReport {
    /// Outputs that were written (or would be, in a dry run).
    pub fn succeeded(&self) -> Vec<Output> {
        self.outputs
            .iter()
            .filter(|report| report.result.is_ok())
            .map(|report| report.output)
            .collect()
    }

    /// Outputs that failed validation, preparation, or publication.
    pub fn failed(&self) -> Vec<Output> {
        self.outputs
            .iter()
            .filter(|report| report.result.is_err())
            .map(|report| report.output)
            .collect()
    }

    /// The report for `output`, if it was requested.
    pub fn get(&self, output: Output) -> Option<&OutputReport> {
        self.outputs.iter().find(|report| report.output == output)
    }

    /// `Ok` when every requested output succeeded; warnings never fail.
    ///
    /// ## Errors
    ///
    /// [`GeneratorError::OutputsFailed`] naming the failed and the
    /// succeeded outputs.
    pub fn into_result(self) -> Result<(), GeneratorError> {
        let failed = self.failed();
        if failed.is_empty() {
            Ok(())
        } else {
            Err(GeneratorError::OutputsFailed {
                succeeded: self.succeeded(),
                failed,
            })
        }
    }
}

/// Runs the generate command for one API's module group, or for every API
/// with `"all"`.
pub fn run_generate(api_name: &str, opts: &GenerateOpts<'_>) -> Result<(), GeneratorError> {
    let scope = if api_name == "all" {
        BatchScope::Complete
    } else {
        BatchScope::Selection(vec![resolve_api(api_name)?.name])
    };
    generate_batch(&resolve_all_apis(), &scope, opts, &get_registries_for_module).into_result()
}

/// Generates every requested output for `scope` of the registry `apis`.
///
/// `apis` is the registry: every API the output roots may hold, in the order
/// `lib.rs` and `prelude.rs` list them. A selection expands to complete
/// module groups separately for each output (Rust module paths for the
/// client, export module names for OpenAPI and Postman). Each output is
/// validated over its whole batch, prepared in memory, and published only
/// when its own validation and ownership checks pass, so one output's
/// failure never stops another. Diagnostics print to stderr as they are
/// found, and the report names which outputs succeeded and which failed.
/// `registry_for` supplies the OpenAPI schema registry of a module.
pub fn generate_batch(
    apis: &[RestApi],
    scope: &BatchScope,
    opts: &GenerateOpts<'_>,
    registry_for: &dyn Fn(&str) -> Option<SchemaRegistry>,
) -> GenerationReport {
    let (openapi_out, postman_out) =
        resolve_export_defaults(opts.output, opts.openapi_out, opts.postman_out);
    let registry: Vec<&RestApi> = apis.iter().collect();
    let selected: Result<Vec<&RestApi>, String> = match scope {
        BatchScope::Complete => Ok(registry.clone()),
        BatchScope::Selection(names) => names
            .iter()
            .map(|name| {
                registry
                    .iter()
                    .copied()
                    .find(|api| &api.name == name)
                    .ok_or_else(|| format!("API `{name}` is not in the registry"))
            })
            .collect(),
    };
    let complete = *scope == BatchScope::Complete;

    if opts.verbose > 0 {
        match &selected {
            Ok(selected) => eprintln!("Generating code for {} API(s)", selected.len()),
            Err(_) => eprintln!("Generating code for an unknown API"),
        }
        eprintln!("Output directory: {}", opts.output);
        if let Some(ref dir) = openapi_out {
            eprintln!("OpenAPI output: {}", dir);
        }
        if let Some(ref dir) = postman_out {
            eprintln!("Postman output: {}", dir);
        }
        if opts.dry_run {
            eprintln!("Dry run mode - no files will be written");
        }
    }
    if opts.verbose > 1
        && let Ok(selected) = &selected
    {
        for api in selected {
            eprintln!("API: {} ({} endpoints)", api.name, api.endpoints.len());
            for endpoint in &api.endpoints {
                eprintln!("  - {} {} {}", endpoint.id, endpoint.method, endpoint.path);
            }
        }
    }

    let mut requested = vec![Output::Rust];
    if !opts.no_openapi && openapi_out.is_some() {
        requested.push(Output::OpenApi);
    }
    if !opts.no_postman && postman_out.is_some() {
        requested.push(Output::Postman);
    }

    let mut report = GenerationReport::default();
    match &selected {
        Err(message) => {
            for output in requested {
                let message = message.clone();
                report.outputs.push(publish(output, Vec::new(), || {
                    Err(GeneratorError::ConfigError(message))
                }));
            }
        }
        Ok(selected) => {
            let rust_selection = if complete {
                Selection::All
            } else {
                Selection::Apis(selected)
            };
            report.outputs.push(publish_rust(&registry, rust_selection, opts));
            let exports = export_batch(&registry, selected, complete);
            if let Some(dir) = openapi_out.as_deref().filter(|_| !opts.no_openapi) {
                report
                    .outputs
                    .push(publish_openapi(&exports, complete, dir, opts, registry_for));
            }
            if let Some(dir) = postman_out.as_deref().filter(|_| !opts.no_postman) {
                report
                    .outputs
                    .push(publish_postman(&exports, complete, dir, opts));
            }
        }
    }

    println!();
    for output in &report.outputs {
        match &output.result {
            Ok(()) => println!("{} {} output", "[OK]".green().bold(), output.output),
            Err(_) => println!("{} {} output", "[FAILED]".red().bold(), output.output),
        }
    }
    report
}

/// Prints `diagnostics` to stderr, one line each.
fn print_diagnostics(diagnostics: &[ValidationDiagnostic]) {
    for diagnostic in diagnostics {
        let label = if diagnostic.is_error() {
            "[ERROR]".red().bold()
        } else {
            "[WARN]".yellow().bold()
        };
        eprintln!("{label} {diagnostic}");
    }
}

/// Validates one output's batch, then prepares and publishes it with
/// `write` if nothing blocks.
fn publish(
    output: Output,
    diagnostics: Vec<ValidationDiagnostic>,
    write: impl FnOnce() -> Result<(), GeneratorError>,
) -> OutputReport {
    print_diagnostics(&diagnostics);
    let result = if has_errors(&diagnostics) {
        Err(GeneratorError::Validation {
            output,
            diagnostics: diagnostics.clone(),
        })
    } else {
        write()
    };
    if let Err(err) = &result {
        eprintln!("{} {}", "[ERROR]".red().bold(), err);
    }
    OutputReport {
        output,
        diagnostics,
        result,
    }
}

/// Generates the Rust client crate, including its `Cargo.toml`.
fn publish_rust(
    registry: &[&RestApi],
    selection: Selection<'_>,
    opts: &GenerateOpts<'_>,
) -> OutputReport {
    println!("{}", "Validating the Rust client batch...".dimmed());
    let batch = match expand_selection(registry, selection) {
        Ok(batch) => batch,
        Err(err) => return publish(Output::Rust, Vec::new(), || Err(err)),
    };
    publish(Output::Rust, validate_rust_client(&batch), || {
        println!("{}", "Generating code...".dimmed());
        let output_dir = Path::new(opts.output);
        let manifest_dir = output_dir.parent().unwrap_or(Path::new("schematic/schema"));
        let prepared = prepare_rust_output(&RustOutputRequest {
            registry,
            selection,
            output_dir,
            manifest_dir: Some(manifest_dir),
        })?;
        let retained = if prepared.retained.is_empty() {
            String::new()
        } else {
            format!("; kept {}", prepared.retained.join(", "))
        };
        if opts.dry_run {
            prepared.plan.print_dry_run(true);
            println!(
                "{} Dry run complete (no files written): would regenerate {}{retained}",
                "[OK]".green().bold(),
                prepared.regenerated.join(", ")
            );
            return Ok(());
        }
        let published = prepared.plan.publish()?;
        print_applied(&published, opts.verbose);
        println!(
            "{} Generated {} under {} ({} written, {} deleted{retained})",
            "[OK]".green().bold(),
            prepared.regenerated.join(", "),
            opts.output,
            published.written(),
            published.deleted(),
        );
        Ok(())
    })
}

fn print_applied(report: &PublicationReport, verbose: u8) {
    if verbose > 0 {
        for mutation in &report.applied {
            println!("{} {}", "...".dimmed(), mutation);
        }
    }
}

/// The registry APIs whose export module a selected API shares, so a
/// partial run rewrites complete export modules. A complete run exports all.
fn export_batch<'a>(
    registry: &[&'a RestApi],
    selected: &[&RestApi],
    complete: bool,
) -> Vec<&'a RestApi> {
    if complete {
        return registry.to_vec();
    }
    let modules: HashSet<String> = selected
        .iter()
        .map(|api| crate::export::resolve_module_name(api))
        .collect();
    registry
        .iter()
        .copied()
        .filter(|api| modules.contains(&crate::export::resolve_module_name(api)))
        .collect()
}

/// Groups APIs by their export module name, keeping batch order.
fn export_groups<'a>(apis: &[&'a RestApi]) -> IndexMap<String, Vec<&'a RestApi>> {
    let mut groups: IndexMap<String, Vec<&RestApi>> = IndexMap::new();
    for api in apis {
        groups
            .entry(crate::export::resolve_module_name(api))
            .or_default()
            .push(api);
    }
    groups
}

/// Validates and publishes one OpenAPI document per module.
fn publish_openapi(
    apis: &[&RestApi],
    complete: bool,
    openapi_dir: &str,
    opts: &GenerateOpts<'_>,
    registry_for: &dyn Fn(&str) -> Option<SchemaRegistry>,
) -> OutputReport {
    let groups = export_groups(apis);
    let mut diagnostics = validate_openapi(apis);
    let mut registries = Vec::with_capacity(groups.len());
    for (module_name, members) in &groups {
        let registry = registry_for(module_name);
        diagnostics.extend(validate_openapi_registry(
            module_name,
            members,
            registry.as_ref(),
        ));
        registries.push(registry);
    }

    publish(Output::OpenApi, diagnostics, || {
        let extension = match opts.openapi_format {
            OpenApiFormat::Json => "json",
            OpenApiFormat::Yaml => "yaml",
        };
        // Build every module's document before writing any, so a merge or
        // operationId failure in one module leaves the whole root untouched.
        let mut files = Vec::with_capacity(groups.len());
        for ((module_name, members), registry) in groups.iter().zip(registries) {
            // Validation proved every module has a registry.
            let Some(registry) = registry else {
                continue;
            };
            let options = openapi_export_options(members, opts.openapi_format, opts.openapi_version);
            let document = build_openapi_grouped(members, module_name, &registry, &options)?;
            files.push((
                format!("{module_name}.{extension}"),
                render_openapi_document(&document, opts.openapi_format.into())?,
            ));
        }
        let plan = prepare_export_output(Path::new(openapi_dir), files, complete, |name| {
            name.ends_with(".json") || name.ends_with(".yaml")
        })?;
        publish_export(&plan, opts)
    })
}

/// Validates and publishes one Postman collection per module.
fn publish_postman(
    apis: &[&RestApi],
    complete: bool,
    postman_dir: &str,
    opts: &GenerateOpts<'_>,
) -> OutputReport {
    publish(Output::Postman, validate_postman(apis), || {
        let mut files = Vec::new();
        for (module_name, members) in export_groups(apis) {
            files.push((
                format!("{module_name}.postman_collection.json"),
                render_postman_module(&members, &module_name)?,
            ));
        }
        let plan = prepare_export_output(Path::new(postman_dir), files, complete, |name| {
            name.ends_with(".postman_collection.json")
        })?;
        publish_export(&plan, opts)
    })
}

/// Publishes an export root's plan, or prints it in a dry run.
fn publish_export(plan: &MutationPlan, opts: &GenerateOpts<'_>) -> Result<(), GeneratorError> {
    if opts.dry_run {
        for mutation in plan.mutations() {
            println!("{} Would {}", "[OK]".green().bold(), mutation);
        }
        return Ok(());
    }
    let published = plan.publish()?;
    print_applied(&published, opts.verbose);
    println!(
        "{} Published {} ({} written, {} deleted)",
        "[OK]".green().bold(),
        plan.root().display(),
        published.written(),
        published.deleted(),
    );
    Ok(())
}

/// Export options for a module document: the CLI version override, else the
/// first member's version, else `0.1.0`.
fn openapi_export_options(
    apis: &[&RestApi],
    format: OpenApiFormat,
    version_override: Option<&str>,
) -> ExportOptions {
    let version = version_override
        .or_else(|| apis.first().and_then(|api| api.version.as_deref()))
        .unwrap_or("0.1.0");
    ExportOptions::new()
        .with_version(version)
        .with_format(format.into())
}

/// Runs the validate command: every output's validator over one API.
///
/// Prints each diagnostic to stderr and one pass/fail line per output.
pub fn run_validate(api_name: &str, verbose: u8) -> Result<(), GeneratorError> {
    let api = resolve_api(api_name)?;
    let apis = [&api];
    if verbose > 0 {
        eprintln!(
            "{} Validating API: {} ({} endpoints)",
            "...".dimmed(),
            api.name,
            api.endpoints.len()
        );
    }

    let module_name = crate::export::resolve_module_name(&api);
    let mut openapi = validate_openapi(&apis);
    openapi.extend(validate_openapi_registry(
        &module_name,
        &apis,
        get_registries_for_module(&module_name).as_ref(),
    ));

    let mut report = GenerationReport::default();
    for (output, diagnostics) in [
        (Output::Rust, validate_rust_client(&apis)),
        (Output::OpenApi, openapi),
        (Output::Postman, validate_postman(&apis)),
    ] {
        report.outputs.push(publish(output, diagnostics, || Ok(())));
    }

    for output in &report.outputs {
        match &output.result {
            Ok(()) => println!("{} {}", "  [PASS]".green().bold(), output.output),
            Err(_) => println!("{} {}", "  [FAIL]".red().bold(), output.output),
        }
    }
    report.into_result()
}
