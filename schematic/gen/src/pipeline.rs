//! API resolution and pipeline orchestration.
//!
//! Contains the core logic for resolving API definitions and orchestrating
//! the generation pipeline, independent of CLI argument parsing.

use std::collections::HashSet;
use std::fs;
use std::path::Path;

use colored::Colorize;
use schematic_define::RestApi;
use schematic_define::openapi::{ExportFormat, ExportOptions};
use schematic_definitions::apis_by_module;
use schematic_definitions::registry::get_registries_for_module;

use crate::cargo_gen::write_cargo_toml;
use crate::errors::GeneratorError;
use crate::output::{generate_and_write, generate_and_write_all};

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

/// Runs validation on an API and prints colored results.
///
/// ## Returns
///
/// `true` if validation passed, `false` if it failed.
pub fn run_validation(api: &RestApi, verbose: u8) -> bool {
    use crate::validation::validate_api;

    if verbose > 0 {
        eprintln!(
            "{} Validating API: {} ({} endpoints)",
            "...".dimmed(),
            api.name,
            api.endpoints.len()
        );
    }

    match validate_api(api) {
        Ok(()) => {
            println!("{} Request suffix format", "  [PASS]".green().bold());
            println!(
                "{} No naming collisions detected",
                "  [PASS]".green().bold()
            );
            println!();
            println!(
                "{} All validation checks passed for '{}'",
                "[OK]".green().bold(),
                api.name
            );
            true
        }
        Err(err) => {
            match &err {
                GeneratorError::InvalidRequestSuffix { suffix, reason } => {
                    println!(
                        "{} Request suffix '{}': {}",
                        "  [FAIL]".red().bold(),
                        suffix,
                        reason
                    );
                }
                GeneratorError::NamingCollision {
                    endpoint_id,
                    body_type,
                    suggestion,
                } => {
                    println!("{} Request suffix format", "  [PASS]".green().bold());
                    println!(
                        "{} Naming collision in endpoint '{}'",
                        "  [FAIL]".red().bold(),
                        endpoint_id
                    );
                    println!(
                        "         Body type '{}' conflicts with generated request struct",
                        body_type.yellow()
                    );
                    println!(
                        "         {} Rename to '{}'",
                        "Suggestion:".cyan(),
                        suggestion.green()
                    );
                }
                _ => {
                    println!("{} {}", "  [FAIL]".red().bold(), err);
                }
            }
            println!();
            println!(
                "{} Validation failed for '{}'",
                "[ERROR]".red().bold(),
                api.name
            );
            false
        }
    }
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

/// Builds a `GeneratorError::ConfigError` describing missing JSON response
/// schemas for an API in a grouped OpenAPI export.
pub fn missing_schemas_error(
    module_name: &str,
    api_name: &str,
    missing: &[String],
) -> GeneratorError {
    let first_missing = missing.first().map(String::as_str).unwrap_or("MissingType");
    GeneratorError::ConfigError(format!(
        "OpenAPI registry incomplete for module \"{module}\" (API \"{api}\"): \
         missing schema(s) {missing:?}. \
         Add JsonSchema derive + register::<T>(\"{first}\") entries in \
         schematic-definitions, or skip with --no-openapi.",
        module = module_name,
        api = api_name,
        missing = missing,
        first = first_missing,
    ))
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

/// Runs the generate command for a single API.
pub fn run_generate(api_name: &str, opts: &GenerateOpts<'_>) -> Result<(), GeneratorError> {
    if api_name == "all" {
        return run_generate_all(opts);
    }

    let (openapi_out, postman_out) =
        resolve_export_defaults(opts.output, opts.openapi_out, opts.postman_out);

    let api = resolve_api(api_name)?;

    if opts.verbose > 0 {
        eprintln!("Generating code for API: {}", api_name);
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

    println!("{}", "Validating API definition...".dimmed());
    if !run_validation(&api, opts.verbose) {
        return Err(GeneratorError::ConfigError(
            "Validation failed. Fix the issues above before generating code.".to_string(),
        ));
    }
    println!();

    if opts.verbose > 1 {
        eprintln!("API: {} ({} endpoints)", api.name, api.endpoints.len());
        for endpoint in &api.endpoints {
            eprintln!("  - {} {} {}", endpoint.id, endpoint.method, endpoint.path);
        }
    }

    println!("{}", "Generating code...".dimmed());
    let output_dir = Path::new(opts.output);
    generate_and_write(&api, output_dir, opts.dry_run)?;

    let schema_dir = output_dir.parent().unwrap_or(Path::new("schematic/schema"));
    write_cargo_toml(schema_dir, opts.dry_run, None)?;

    if !opts.dry_run {
        println!(
            "{} Generated code to {}/lib.rs",
            "[OK]".green().bold(),
            opts.output
        );
        println!(
            "{} Generated {}/Cargo.toml",
            "[OK]".green().bold(),
            schema_dir.display()
        );
    } else {
        println!(
            "{} Dry run complete (no files written)",
            "[OK]".green().bold()
        );
    }

    if !opts.no_openapi
        && let Some(openapi_dir) = openapi_out.as_deref()
    {
        let module_name = crate::export::resolve_module_name(&api);
        let registry = get_registries_for_module(&module_name).ok_or_else(|| {
            GeneratorError::ConfigError(format!(
                "Missing schema registry for module \"{module_name}\". \
                 Add openapi_registry() to schematic-definitions or skip with --no-openapi."
            ))
        })?;

        registry
            .validate_completeness(&api)
            .map_err(|missing| missing_schemas_error(&module_name, &api.name, &missing))?;

        run_openapi_export_grouped(
            &module_name,
            &[&api],
            &registry,
            openapi_dir,
            opts.openapi_format,
            opts.openapi_version,
            opts.dry_run,
            opts.verbose,
        )?;
    }

    if !opts.no_postman
        && let Some(postman_dir) = postman_out.as_deref()
    {
        run_postman_export(&api, postman_dir, opts.dry_run, opts.verbose)?;
    }

    Ok(())
}

/// Exports an API definition to Postman collection format.
pub fn run_postman_export(
    api: &RestApi,
    postman_dir: &str,
    dry_run: bool,
    verbose: u8,
) -> Result<(), GeneratorError> {
    use crate::postman_output::write_postman;

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
    use crate::openapi_output::write_openapi_grouped;

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

/// Runs the generate command for all APIs at once.
pub fn run_generate_all(opts: &GenerateOpts<'_>) -> Result<(), GeneratorError> {
    let (openapi_out, postman_out) =
        resolve_export_defaults(opts.output, opts.openapi_out, opts.postman_out);

    let apis = resolve_all_apis();

    if opts.verbose > 0 {
        eprintln!("Generating code for all {} APIs", apis.len());
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

    println!("{}", "Validating all API definitions...".dimmed());
    let mut all_valid = true;
    for api in &apis {
        if !run_validation(api, opts.verbose) {
            all_valid = false;
        }
        println!();
    }

    if !all_valid {
        return Err(GeneratorError::ConfigError(
            "Validation failed. Fix the issues above before generating code.".to_string(),
        ));
    }

    if opts.verbose > 1 {
        for api in &apis {
            eprintln!("API: {} ({} endpoints)", api.name, api.endpoints.len());
            for endpoint in &api.endpoints {
                eprintln!("  - {} {} {}", endpoint.id, endpoint.method, endpoint.path);
            }
        }
    }

    println!("{}", "Generating code for all APIs...".dimmed());
    let output_dir = Path::new(opts.output);
    let api_refs: Vec<&RestApi> = apis.iter().collect();
    generate_and_write_all(&api_refs, output_dir, opts.dry_run)?;

    let schema_dir = output_dir.parent().unwrap_or(Path::new("schematic/schema"));
    write_cargo_toml(schema_dir, opts.dry_run, None)?;

    if !opts.dry_run {
        println!(
            "{} Generated code for {} APIs to {}",
            "[OK]".green().bold(),
            apis.len(),
            opts.output
        );
        println!(
            "{} Generated {}/Cargo.toml",
            "[OK]".green().bold(),
            schema_dir.display()
        );
    } else {
        println!(
            "{} Dry run complete (no files written)",
            "[OK]".green().bold()
        );
    }

    let mut openapi_files = HashSet::new();
    if !opts.no_openapi
        && let Some(openapi_dir) = openapi_out.as_deref()
    {
        let grouped = apis_by_module();

        for (module_name, module_apis) in grouped.iter() {
            let extension = match opts.openapi_format {
                OpenApiFormat::Json => "json",
                OpenApiFormat::Yaml => "yaml",
            };
            openapi_files.insert(format!("{}.{}", module_name, extension));

            let registry = get_registries_for_module(module_name).ok_or_else(|| {
                GeneratorError::ConfigError(format!(
                    "Missing schema registry for module \"{module_name}\". \
                     Add openapi_registry() to schematic-definitions or skip with --no-openapi."
                ))
            })?;

            for member in module_apis.iter() {
                registry.validate_completeness(member).map_err(|missing| {
                    missing_schemas_error(module_name, &member.name, &missing)
                })?;
            }

            let api_refs: Vec<&RestApi> = module_apis.iter().collect();

            run_openapi_export_grouped(
                module_name,
                &api_refs,
                &registry,
                openapi_dir,
                opts.openapi_format,
                opts.openapi_version,
                opts.dry_run,
                opts.verbose,
            )?;
        }

        if !opts.dry_run {
            cleanup_stale_artifacts(Path::new(openapi_dir), &openapi_files, opts.verbose)?;
        }
    }

    let mut postman_files = HashSet::new();
    if !opts.no_postman
        && let Some(postman_dir) = postman_out.as_deref()
    {
        let grouped = apis_by_module();

        for (module_name, module_apis) in grouped.iter() {
            if module_apis.len() == 1 {
                let module_name_resolved = crate::export::resolve_module_name(&module_apis[0]);
                postman_files.insert(format!("{}.postman_collection.json", module_name_resolved));

                run_postman_export(&module_apis[0], postman_dir, opts.dry_run, opts.verbose)?;
            } else {
                use crate::postman_output::write_postman_grouped;

                postman_files.insert(format!("{}.postman_collection.json", module_name));

                if opts.verbose > 0 {
                    println!(
                        "{} Exporting grouped Postman collection for module '{}' ({} APIs)...",
                        "...".dimmed(),
                        module_name,
                        module_apis.len()
                    );
                }

                let api_refs: Vec<&RestApi> = module_apis.iter().collect();
                let postman_path = Path::new(postman_dir);

                if !opts.dry_run && !postman_path.exists() {
                    std::fs::create_dir_all(postman_path).map_err(|e| {
                        GeneratorError::WriteError {
                            path: postman_dir.to_string(),
                            source: e,
                        }
                    })?;
                }

                if opts.dry_run {
                    println!(
                        "{} Would export grouped Postman collection to {}/{}.postman_collection.json",
                        "[OK]".green().bold(),
                        postman_dir,
                        module_name,
                    );
                } else {
                    let path = write_postman_grouped(&api_refs, module_name, postman_path, false)?;
                    println!(
                        "{} Exported grouped Postman collection to {}",
                        "[OK]".green().bold(),
                        path.display()
                    );
                }
            }
        }

        if !opts.dry_run {
            cleanup_stale_artifacts(Path::new(postman_dir), &postman_files, opts.verbose)?;
        }
    }

    Ok(())
}

/// Runs the validate command.
pub fn run_validate(api_name: &str, verbose: u8) -> Result<(), GeneratorError> {
    let api = resolve_api(api_name)?;

    if run_validation(&api, verbose) {
        Ok(())
    } else {
        Err(GeneratorError::ConfigError("Validation failed".to_string()))
    }
}
