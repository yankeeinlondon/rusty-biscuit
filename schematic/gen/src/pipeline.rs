//! API resolution and pipeline orchestration.
//!
//! Contains the core logic for resolving API definitions and orchestrating
//! the generation pipeline, independent of CLI argument parsing.

use std::collections::HashSet;
use std::fs;
use std::path::Path;

use colored::Colorize;
use indexmap::IndexMap;
use schematic_define::RestApi;
use schematic_define::openapi::{ExportFormat, ExportOptions};
use schematic_definitions::registry::{SchemaRegistry, get_registries_for_module};

use crate::cargo_gen::write_cargo_toml;
use crate::diagnostics::{Output, ValidationDiagnostic, has_errors};
use crate::errors::GeneratorError;
use crate::output::generate_and_write_all;
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

    vec![
        define_anthropic_api(),
        define_artificial_analysis_data_api(),
        define_artificial_analysis_critpt_api(),
        define_bitbucket_api(),
        define_openai_api(),
        define_elevenlabs_rest_api(),
        define_gitea_api(),
        define_github_api(),
        define_gitlab_api(),
        define_huggingface_hub_api(),
        define_lmstudio_api(),
        define_ollama_native_api(),
        define_ollama_openai_api(),
        define_emqx_basic_api(),
        define_emqx_bearer_api(),
        define_eversolo_api(),
        define_samsung_smart_tv_api(),
        define_unfolded_circle_core_rest_api(),
    ]
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

/// Cleans up stale artifacts from an output directory.
///
/// Removes any files in the directory that are not in the expected files set.
pub fn cleanup_stale_artifacts(
    dir: &Path,
    expected_files: &HashSet<String>,
    verbose: u8,
) -> Result<(), GeneratorError> {
    if !dir.exists() {
        return Ok(());
    }

    for entry in fs::read_dir(dir).map_err(|e| GeneratorError::WriteError {
        path: dir.display().to_string(),
        source: e,
    })? {
        let entry = entry.map_err(|e| GeneratorError::WriteError {
            path: dir.display().to_string(),
            source: e,
        })?;

        let file_name = entry.file_name().to_string_lossy().to_string();

        if !expected_files.contains(&file_name) {
            if verbose > 0 {
                println!(
                    "{} Removing stale artifact: {}",
                    "[CLEAN]".yellow().bold(),
                    entry.path().display()
                );
            }
            fs::remove_file(entry.path()).map_err(|e| GeneratorError::WriteError {
                path: entry.path().display().to_string(),
                source: e,
            })?;
        }
    }

    Ok(())
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


/// Which part of the registry a batch is, deciding export cleanup.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BatchScope {
    /// Selected APIs only: existing export files are left alone.
    Selection,
    /// Every API: export files that no API produces are removed.
    Complete,
}

/// The outcome of one requested output.
#[derive(Debug)]
pub struct OutputReport {
    /// Which output this is.
    pub output: Output,
    /// Everything its validator reported, warnings included.
    pub diagnostics: Vec<ValidationDiagnostic>,
    /// `Err` when validation blocked the output or writing it failed.
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

    /// Outputs that failed validation or writing.
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

/// Runs the generate command for one API, or for every API with `"all"`.
pub fn run_generate(api_name: &str, opts: &GenerateOpts<'_>) -> Result<(), GeneratorError> {
    let (apis, scope) = if api_name == "all" {
        (resolve_all_apis(), BatchScope::Complete)
    } else {
        (vec![resolve_api(api_name)?], BatchScope::Selection)
    };
    generate_batch(&apis, scope, opts, &get_registries_for_module).into_result()
}

/// Generates every requested output for a batch of APIs.
///
/// The Rust client, OpenAPI documents, and Postman collections are each
/// validated over the whole batch and written only when their own validator
/// reports no error, so one output's failure never stops another. Every
/// diagnostic is printed to stderr as it is found, and the report names
/// which outputs succeeded and which failed. `registry_for` supplies the
/// OpenAPI schema registry of a module.
pub fn generate_batch(
    apis: &[RestApi],
    scope: BatchScope,
    opts: &GenerateOpts<'_>,
    registry_for: &dyn Fn(&str) -> Option<SchemaRegistry>,
) -> GenerationReport {
    let (openapi_out, postman_out) =
        resolve_export_defaults(opts.output, opts.openapi_out, opts.postman_out);
    let api_refs: Vec<&RestApi> = apis.iter().collect();

    if opts.verbose > 0 {
        eprintln!("Generating code for {} API(s)", apis.len());
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
    if opts.verbose > 1 {
        for api in apis {
            eprintln!("API: {} ({} endpoints)", api.name, api.endpoints.len());
            for endpoint in &api.endpoints {
                eprintln!("  - {} {} {}", endpoint.id, endpoint.method, endpoint.path);
            }
        }
    }

    let mut report = GenerationReport::default();
    report.outputs.push(publish_rust(&api_refs, opts));
    if !opts.no_openapi
        && let Some(dir) = openapi_out.as_deref()
    {
        report
            .outputs
            .push(publish_openapi(&api_refs, scope, dir, opts, registry_for));
    }
    if !opts.no_postman
        && let Some(dir) = postman_out.as_deref()
    {
        report
            .outputs
            .push(publish_postman(&api_refs, scope, dir, opts));
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

/// Validates one output's batch, then writes it with `write` if nothing blocks.
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

/// Generates and writes the Rust client crate.
fn publish_rust(apis: &[&RestApi], opts: &GenerateOpts<'_>) -> OutputReport {
    println!("{}", "Validating the Rust client batch...".dimmed());
    publish(Output::Rust, validate_rust_client(apis), || {
        println!("{}", "Generating code...".dimmed());
        let output_dir = Path::new(opts.output);
        generate_and_write_all(apis, output_dir, opts.dry_run)?;

        let schema_dir = output_dir.parent().unwrap_or(Path::new("schematic/schema"));
        write_cargo_toml(schema_dir, opts.dry_run, None)?;

        if opts.dry_run {
            println!(
                "{} Dry run complete (no files written)",
                "[OK]".green().bold()
            );
        } else {
            println!(
                "{} Generated code for {} API(s) to {}",
                "[OK]".green().bold(),
                apis.len(),
                opts.output
            );
            println!(
                "{} Generated {}/Cargo.toml",
                "[OK]".green().bold(),
                schema_dir.display()
            );
        }
        Ok(())
    })
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

/// Validates and writes one OpenAPI document per module.
fn publish_openapi(
    apis: &[&RestApi],
    scope: BatchScope,
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
        let mut written = HashSet::new();
        for ((module_name, members), registry) in groups.iter().zip(registries) {
            // Validation proved every module has a registry.
            let Some(registry) = registry else {
                continue;
            };
            written.insert(format!("{module_name}.{extension}"));
            run_openapi_export_grouped(
                module_name,
                members,
                &registry,
                openapi_dir,
                opts.openapi_format,
                opts.openapi_version,
                opts.dry_run,
                opts.verbose,
            )?;
        }
        if scope == BatchScope::Complete && !opts.dry_run {
            cleanup_stale_artifacts(Path::new(openapi_dir), &written, opts.verbose)?;
        }
        Ok(())
    })
}

/// Validates and writes one Postman collection per module.
fn publish_postman(
    apis: &[&RestApi],
    scope: BatchScope,
    postman_dir: &str,
    opts: &GenerateOpts<'_>,
) -> OutputReport {
    publish(Output::Postman, validate_postman(apis), || {
        let mut written = HashSet::new();
        for (module_name, members) in export_groups(apis) {
            written.insert(format!("{module_name}.postman_collection.json"));
            if let [api] = members.as_slice() {
                run_postman_export(api, postman_dir, opts.dry_run, opts.verbose)?;
            } else {
                run_postman_export_grouped(
                    &module_name,
                    &members,
                    postman_dir,
                    opts.dry_run,
                    opts.verbose,
                )?;
            }
        }
        if scope == BatchScope::Complete && !opts.dry_run {
            cleanup_stale_artifacts(Path::new(postman_dir), &written, opts.verbose)?;
        }
        Ok(())
    })
}

/// Exports the APIs of one module to a single Postman collection.
fn run_postman_export_grouped(
    module_name: &str,
    apis: &[&RestApi],
    postman_dir: &str,
    dry_run: bool,
    verbose: u8,
) -> Result<(), GeneratorError> {
    use crate::export::postman::write_postman_grouped;

    if verbose > 0 {
        println!(
            "{} Exporting grouped Postman collection for module '{}' ({} APIs)...",
            "...".dimmed(),
            module_name,
            apis.len()
        );
    }

    let postman_path = Path::new(postman_dir);
    if dry_run {
        println!(
            "{} Would export grouped Postman collection to {}/{}.postman_collection.json",
            "[OK]".green().bold(),
            postman_dir,
            module_name,
        );
        return Ok(());
    }

    if !postman_path.exists() {
        std::fs::create_dir_all(postman_path).map_err(|e| GeneratorError::WriteError {
            path: postman_dir.to_string(),
            source: e,
        })?;
    }
    let path = write_postman_grouped(apis, module_name, postman_path, false)?;
    println!(
        "{} Exported grouped Postman collection to {}",
        "[OK]".green().bold(),
        path.display()
    );
    Ok(())
}

/// Exports an API definition to Postman collection format.
pub fn run_postman_export(
    api: &RestApi,
    postman_dir: &str,
    dry_run: bool,
    verbose: u8,
) -> Result<(), GeneratorError> {
    use crate::export::postman::write_postman;

    if verbose > 0 {
        println!("{}", "Exporting Postman collection...".dimmed());
    }

    let postman_path = Path::new(postman_dir);

    if !dry_run && !postman_path.exists() {
        std::fs::create_dir_all(postman_path).map_err(|e| GeneratorError::WriteError {
            path: postman_dir.to_string(),
            source: e,
        })?;
    }

    if dry_run {
        let module_name = crate::export::resolve_module_name(api);
        println!(
            "{} Would export Postman collection to {}/{}.postman_collection.json",
            "[OK]".green().bold(),
            postman_dir,
            module_name,
        );
    } else {
        let path = write_postman(api, postman_path, false)?;
        println!(
            "{} Exported Postman collection to {}",
            "[OK]".green().bold(),
            path.display()
        );
    }

    Ok(())
}

/// Exports a module-grouped OpenAPI document.
#[allow(clippy::too_many_arguments)]
pub fn run_openapi_export_grouped(
    module_name: &str,
    apis: &[&RestApi],
    registry: &schematic_definitions::registry::SchemaRegistry,
    openapi_dir: &str,
    format: OpenApiFormat,
    version_override: Option<&str>,
    dry_run: bool,
    verbose: u8,
) -> Result<(), GeneratorError> {
    use crate::export::openapi::write_openapi_grouped;

    if verbose > 0 {
        println!(
            "{} Exporting OpenAPI specification for module '{}' ({} APIs)...",
            "...".dimmed(),
            module_name,
            apis.len(),
        );
    }

    let openapi_path = Path::new(openapi_dir);

    if !dry_run && !openapi_path.exists() {
        std::fs::create_dir_all(openapi_path).map_err(|e| GeneratorError::WriteError {
            path: openapi_dir.to_string(),
            source: e,
        })?;
    }

    let version = version_override
        .or_else(|| apis.first().and_then(|api| api.version.as_deref()))
        .unwrap_or("0.1.0");

    let options = ExportOptions::new()
        .with_version(version)
        .with_format(format.into());

    if dry_run {
        let extension = match format {
            OpenApiFormat::Json => "json",
            OpenApiFormat::Yaml => "yaml",
        };
        println!(
            "{} Would export OpenAPI spec to {}/{}.{}",
            "[OK]".green().bold(),
            openapi_dir,
            module_name,
            extension,
        );
    } else {
        let path = write_openapi_grouped(apis, module_name, registry, &options, openapi_path)?;
        println!(
            "{} Exported OpenAPI spec to {}",
            "[OK]".green().bold(),
            path.display()
        );
    }

    Ok(())
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
