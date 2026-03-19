//! Schematic Code Generator
//!
//! Generates strongly-typed Rust client code from REST API definitions.

use std::collections::HashSet;
use std::fs;
use std::path::Path;
use std::process::ExitCode;

use clap::{Parser, Subcommand, ValueEnum};
use colored::Colorize;
use schematic_define::openapi::{ExportFormat, ExportOptions};
use schematic_definitions::anthropic::define_anthropic_api;
use schematic_definitions::apis_by_module;
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
use schematic_definitions::registry::get_registry;
use schematic_definitions::samsung_smart_tv::define_samsung_smart_tv_api;
use schematic_definitions::unfolded_circle::define_unfolded_circle_core_rest_api;
use schematic_gen::asyncapi_import::{self, AsyncImportOptions, WsRole as AsyncWsRole};
use schematic_gen::cargo_gen::write_cargo_toml;
use schematic_gen::errors::GeneratorError;
use schematic_gen::import_pipeline::{self, ImportOptions};
use schematic_gen::openapi_output::write_openapi;
use schematic_gen::output::{generate_and_write, generate_and_write_all};
use schematic_gen::postman_output::{write_postman, write_postman_grouped};
use schematic_gen::validate_api;

/// List of available API names for error messages.
const AVAILABLE_APIS: &str = "anthropic, bitbucket, openai, elevenlabs, eversolo, gitea, github, gitlab, huggingface, lmstudio, ollama-native, ollama-openai, emqx-basic, emqx-bearer, samsung-smart-tv, unfolded-circle-core-rest, all";

/// OpenAPI output format for CLI.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, ValueEnum)]
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

/// WebSocket role mapping mode for AsyncAPI import.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, ValueEnum)]
pub enum WsRoleArg {
    /// Treat AsyncAPI publish as client -> server.
    #[default]
    Client,
    /// Treat AsyncAPI publish as server -> client.
    Server,
    /// Map all messages as bidirectional.
    Both,
}

impl From<WsRoleArg> for AsyncWsRole {
    fn from(value: WsRoleArg) -> Self {
        match value {
            WsRoleArg::Client => AsyncWsRole::Client,
            WsRoleArg::Server => AsyncWsRole::Server,
            WsRoleArg::Both => AsyncWsRole::Both,
        }
    }
}

/// Schematic code generator - transforms API definitions into typed Rust clients
#[derive(Parser, Debug)]
#[command(name = "schematic-gen")]
#[command(author, version, about, long_about = None)]
struct Cli {
    #[command(subcommand)]
    command: Option<Commands>,

    // Legacy flat arguments for backwards compatibility
    /// API definition to generate code for (e.g., "openai")
    #[arg(short, long, global = true)]
    api: Option<String>,

    /// Output directory for generated code
    #[arg(short, long, default_value = "schematic/schema/src")]
    output: String,

    /// Print generated code without writing files
    #[arg(long)]
    dry_run: bool,

    /// Increase verbosity (-v, -vv, -vvv)
    #[arg(short, long, action = clap::ArgAction::Count, global = true)]
    verbose: u8,
}

#[derive(Subcommand, Debug)]
enum Commands {
    /// Generate Rust client code from an API definition
    Generate {
        /// API definition to generate code for (e.g., "openai")
        #[arg(short, long)]
        api: String,

        /// Output directory for generated code
        #[arg(short, long, default_value = "schematic/schema/src")]
        output: String,

        /// Print generated code without writing files
        #[arg(long)]
        dry_run: bool,

        /// Output directory for OpenAPI specification files
        ///
        /// When provided, exports the API definition to an OpenAPI 3.0.3 spec
        /// after generating Rust client code.
        #[arg(long, value_name = "DIR")]
        openapi_out: Option<String>,

        /// Output format for OpenAPI specification
        #[arg(long, value_enum, default_value = "json")]
        openapi_format: OpenApiFormat,

        /// Override version in OpenAPI specification
        #[arg(long, value_name = "VERSION")]
        openapi_version: Option<String>,

        /// Output directory for Postman collection files
        ///
        /// When provided, exports the API definition to a Postman v2.1.0 collection
        /// after generating Rust client code.
        #[arg(long, value_name = "DIR")]
        postman_out: Option<String>,

        /// Skip OpenAPI specification generation
        #[arg(long)]
        no_openapi: bool,

        /// Skip Postman collection generation
        #[arg(long)]
        no_postman: bool,
    },

    /// Validate an API definition without generating code
    Validate {
        /// API definition to validate (e.g., "openai")
        #[arg(short, long)]
        api: String,
    },

    /// Import an OpenAPI specification and generate Rust client code
    Import {
        /// Path to OpenAPI spec file (JSON or YAML)
        #[arg(short, long)]
        input: String,

        /// Override API name (derived from spec title by default)
        #[arg(long, value_name = "NAME")]
        api_name: Option<String>,

        /// Override module path for generated code
        #[arg(long, value_name = "PATH")]
        module_path: Option<String>,

        /// Output directory for generated code
        #[arg(short, long)]
        output: String,

        /// Print generated code without writing files
        #[arg(long)]
        dry_run: bool,

        /// Fail on any warning-level diagnostic
        #[arg(long)]
        strict: bool,
    },

    /// Import an AsyncAPI specification and emit websocket import artifacts
    ImportAsyncapi {
        /// Path to AsyncAPI spec file (JSON or YAML)
        #[arg(short, long)]
        input: String,

        /// Override API name (derived from spec title by default)
        #[arg(long, value_name = "NAME")]
        api_name: Option<String>,

        /// Override module path for generated code
        #[arg(long, value_name = "PATH")]
        module_path: Option<String>,

        /// Role mapping for publish/subscribe semantics
        #[arg(long, value_enum, default_value = "client")]
        ws_role: WsRoleArg,

        /// Output directory for import artifacts
        #[arg(short, long)]
        output: String,

        /// Print import summary without writing files
        #[arg(long)]
        dry_run: bool,

        /// Fail on any warning-level diagnostic
        #[arg(long)]
        strict: bool,
    },
}

/// Resolves an API name to its definition.
fn resolve_api(name: &str) -> Result<schematic_define::RestApi, GeneratorError> {
    match name {
        "anthropic" => Ok(define_anthropic_api()),
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
fn resolve_all_apis() -> Vec<schematic_define::RestApi> {
    vec![
        define_anthropic_api(),
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
fn run_validation(api: &schematic_define::RestApi, verbose: u8) -> bool {
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

/// Options for the generate command.
struct GenerateOpts<'a> {
    output: &'a str,
    dry_run: bool,
    verbose: u8,
    openapi_out: Option<&'a str>,
    openapi_format: OpenApiFormat,
    openapi_version: Option<&'a str>,
    postman_out: Option<&'a str>,
    no_openapi: bool,
    no_postman: bool,
}

/// Resolves default export directories for OpenAPI and Postman.
///
/// If the output path ends with "schema/src" and no explicit output paths are provided,
/// defaults to sibling directories:
/// - OpenAPI: `<base>/openapi`
/// - Postman: `<base>/postman`
///
/// where `<base>` is the grandparent directory of the output path.
///
/// ## Examples
///
/// ```text
/// resolve_export_defaults("schematic/schema/src", None, None)
///   -> (Some("schematic/openapi"), Some("schematic/postman"))
///
/// resolve_export_defaults("other/path", None, None)
///   -> (None, None)
///
/// resolve_export_defaults("schema/src", Some("custom"), None)
///   -> (Some("custom"), None)  // explicit values preserved
/// ```
fn resolve_export_defaults(
    output: &str,
    openapi_out: Option<&str>,
    postman_out: Option<&str>,
) -> (Option<String>, Option<String>) {
    // If explicit values provided, use them
    if openapi_out.is_some() || postman_out.is_some() {
        return (openapi_out.map(String::from), postman_out.map(String::from));
    }

    // Auto-detect: if output ends with "schema/src", default to sibling dirs
    if output.ends_with("schema/src") {
        let base = output.strip_suffix("schema/src").unwrap_or("");
        let openapi_default = format!("{}openapi", base);
        let postman_default = format!("{}postman", base);
        (Some(openapi_default), Some(postman_default))
    } else {
        (None, None)
    }
}

/// Runs the generate command.
fn run_generate(api_name: &str, opts: &GenerateOpts<'_>) -> Result<(), GeneratorError> {
    if api_name == "all" {
        return run_generate_all(opts);
    }

    // Resolve export defaults before processing
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

    // Run validation first
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

    // Generate Cargo.toml in the parent directory of src/
    // The output_dir points to src/, so we need to get its parent for Cargo.toml
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

    // Export OpenAPI spec if requested (use resolved default if available)
    if !opts.no_openapi
        && let Some(openapi_dir) = openapi_out.as_deref()
    {
        run_openapi_export(
            api_name,
            &api,
            openapi_dir,
            opts.openapi_format,
            opts.openapi_version,
            opts.dry_run,
            opts.verbose,
        )?;
    }

    // Export Postman collection if requested (use resolved default if available)
    if !opts.no_postman
        && let Some(postman_dir) = postman_out.as_deref()
    {
        run_postman_export(&api, postman_dir, opts.dry_run, opts.verbose)?;
    }

    Ok(())
}

/// Exports an API definition to Postman collection format.
fn run_postman_export(
    api: &schematic_define::RestApi,
    postman_dir: &str,
    dry_run: bool,
    verbose: u8,
) -> Result<(), GeneratorError> {
    if verbose > 0 {
        println!("{}", "Exporting Postman collection...".dimmed());
    }

    let postman_path = Path::new(postman_dir);

    // Create the directory if it doesn't exist
    if !dry_run && !postman_path.exists() {
        std::fs::create_dir_all(postman_path).map_err(|e| GeneratorError::WriteError {
            path: postman_dir.to_string(),
            source: e,
        })?;
    }

    if dry_run {
        let module_name = schematic_gen::export::resolve_module_name(api);
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

/// Exports an API definition to OpenAPI format.
fn run_openapi_export(
    api_name: &str,
    api: &schematic_define::RestApi,
    openapi_dir: &str,
    format: OpenApiFormat,
    version_override: Option<&str>,
    dry_run: bool,
    verbose: u8,
) -> Result<(), GeneratorError> {
    // Get the schema registry for this API (strict mode: fail if missing)
    let registry = get_registry(api_name).ok_or_else(|| {
        GeneratorError::ConfigError(format!(
            "Missing schema registry for API \"{}\" (module: {})\n  \
             → Add `openapi_registry()` to schematic-definitions/src/{}/mod.rs\n  \
             → Or skip with --no-openapi",
            api.name,
            api_name,
            api_name.replace('-', "_")
        ))
    })?;

    if verbose > 0 {
        println!("{}", "Exporting OpenAPI specification...".dimmed());
    }

    let openapi_path = Path::new(openapi_dir);

    // Create the directory if it doesn't exist
    if !dry_run && !openapi_path.exists() {
        std::fs::create_dir_all(openapi_path).map_err(|e| GeneratorError::WriteError {
            path: openapi_dir.to_string(),
            source: e,
        })?;
    }

    // Version resolution: CLI override > RestApi.version > fallback "0.1.0"
    let version = version_override
        .or(api.version.as_deref())
        .unwrap_or("0.1.0");

    let options = ExportOptions::new()
        .with_version(version)
        .with_format(format.into());

    if dry_run {
        println!(
            "{} Would export OpenAPI spec to {}/{}.{}",
            "[OK]".green().bold(),
            openapi_dir,
            api.name.to_lowercase(),
            match format {
                OpenApiFormat::Json => "json",
                OpenApiFormat::Yaml => "yaml",
            }
        );
    } else {
        let path = write_openapi(api, &registry, &options, openapi_path)?;
        println!(
            "{} Exported OpenAPI spec to {}",
            "[OK]".green().bold(),
            path.display()
        );
    }

    Ok(())
}

/// Runs the generate command for all APIs at once.
fn run_generate_all(opts: &GenerateOpts<'_>) -> Result<(), GeneratorError> {
    // Resolve export defaults before processing
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

    // Run validation on all APIs first
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
    let api_refs: Vec<&schematic_define::RestApi> = apis.iter().collect();
    generate_and_write_all(&api_refs, output_dir, opts.dry_run)?;

    // Generate Cargo.toml in the parent directory of src/
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

    // Export OpenAPI specs if requested (use resolved default if available)
    let mut openapi_files = HashSet::new();
    if !opts.no_openapi
        && let Some(openapi_dir) = openapi_out.as_deref()
    {
        // Map API structs to names for registry lookup
        let api_names = [
            ("Anthropic", "anthropic"),
            ("OpenAI", "openai"),
            ("ElevenLabs", "elevenlabs"),
            ("Gitea", "gitea"),
            ("GitHub", "github"),
            ("GitLab", "gitlab"),
            ("HuggingFaceHub", "huggingface"),
            ("LmStudio", "lmstudio"),
            ("OllamaNative", "ollama-native"),
            ("OllamaOpenAI", "ollama-openai"),
            ("EmqxBasic", "emqx-basic"),
            ("EmqxBearer", "emqx-bearer"),
            ("Eversolo", "eversolo"),
            ("SamsungSmartTv", "samsung-smart-tv"),
            ("UnfoldedCircleCoreRest", "unfolded-circle-core-rest"),
        ];

        for api in &apis {
            let api_name_lower = api.name.to_lowercase();
            let api_name = api_names
                .iter()
                .find(|(name, _)| *name == api.name)
                .map(|(_, key)| *key)
                .unwrap_or(&api_name_lower);

            // Track expected filename
            let extension = match opts.openapi_format {
                OpenApiFormat::Json => "json",
                OpenApiFormat::Yaml => "yaml",
            };
            openapi_files.insert(format!("{}.{}", api.name.to_lowercase(), extension));

            run_openapi_export(
                api_name,
                api,
                openapi_dir,
                opts.openapi_format,
                opts.openapi_version,
                opts.dry_run,
                opts.verbose,
            )?;
        }

        // Clean up stale artifacts
        if !opts.dry_run {
            cleanup_stale_artifacts(Path::new(openapi_dir), &openapi_files, opts.verbose)?;
        }
    }

    // Export Postman collections if requested (use resolved default if available)
    let mut postman_files = HashSet::new();
    if !opts.no_postman
        && let Some(postman_dir) = postman_out.as_deref()
    {
        let grouped = apis_by_module();

        for (module_name, module_apis) in grouped.iter() {
            if module_apis.len() == 1 {
                // Single API in module: use regular export
                // Track expected filename
                let module_name_resolved =
                    schematic_gen::export::resolve_module_name(&module_apis[0]);
                postman_files.insert(format!("{}.postman_collection.json", module_name_resolved));

                run_postman_export(&module_apis[0], postman_dir, opts.dry_run, opts.verbose)?;
            } else {
                // Multiple APIs in module: use grouped export
                // Track expected filename
                postman_files.insert(format!("{}.postman_collection.json", module_name));

                if opts.verbose > 0 {
                    println!(
                        "{} Exporting grouped Postman collection for module '{}' ({} APIs)...",
                        "...".dimmed(),
                        module_name,
                        module_apis.len()
                    );
                }

                let api_refs: Vec<&schematic_define::RestApi> = module_apis.iter().collect();
                let postman_path = Path::new(postman_dir);

                // Create the directory if it doesn't exist
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

        // Clean up stale artifacts
        if !opts.dry_run {
            cleanup_stale_artifacts(Path::new(postman_dir), &postman_files, opts.verbose)?;
        }
    }

    Ok(())
}

/// Runs the AsyncAPI import command.
struct ImportAsyncapiCommandArgs<'a> {
    input: &'a str,
    api_name: Option<&'a str>,
    module_path: Option<&'a str>,
    ws_role: WsRoleArg,
    output: &'a str,
    dry_run: bool,
    strict: bool,
}

/// Runs the AsyncAPI import command.
fn run_import_asyncapi_command(
    args: ImportAsyncapiCommandArgs<'_>,
    verbose: u8,
) -> Result<(), GeneratorError> {
    if verbose > 0 {
        eprintln!("Importing AsyncAPI spec: {}", args.input);
        eprintln!("Output directory: {}", args.output);
        if args.dry_run {
            eprintln!("Dry run mode - no files will be written");
        }
    }

    println!("{}", "Importing AsyncAPI specification...".dimmed());

    let options = AsyncImportOptions {
        input: args.input.to_string(),
        api_name: args.api_name.map(String::from),
        module_path: args.module_path.map(String::from),
        ws_role: args.ws_role.into(),
        output: args.output.to_string(),
        dry_run: args.dry_run,
        strict: args.strict,
        verbose: verbose > 0,
    };

    let result = asyncapi_import::run_import_asyncapi(&options)?;
    let warn_count = result.diagnostics.iter().filter(|d| d.is_warn()).count();
    let error_count = result.diagnostics.iter().filter(|d| d.is_error()).count();

    if !args.dry_run {
        println!(
            "{} Imported '{}' (AsyncAPI): {} endpoints, {} messages, {} models",
            "[OK]".green().bold(),
            result.api_name,
            result.endpoint_count,
            result.message_count,
            result.model_count,
        );
    } else {
        println!(
            "{} Dry run complete for '{}' (AsyncAPI): {} endpoints, {} messages, {} models",
            "[OK]".green().bold(),
            result.api_name,
            result.endpoint_count,
            result.message_count,
            result.model_count,
        );
    }

    if warn_count > 0 || error_count > 0 {
        println!(
            "     {} warnings, {} errors in diagnostics",
            warn_count.to_string().yellow(),
            error_count.to_string().red(),
        );
    }

    Ok(())
}

/// Runs the import command.
fn run_import_command(
    input: &str,
    api_name: Option<&str>,
    module_path: Option<&str>,
    output: &str,
    dry_run: bool,
    strict: bool,
    verbose: u8,
) -> Result<(), GeneratorError> {
    if verbose > 0 {
        eprintln!("Importing OpenAPI spec: {}", input);
        eprintln!("Output directory: {}", output);
        if dry_run {
            eprintln!("Dry run mode - no files will be written");
        }
    }

    println!("{}", "Importing OpenAPI specification...".dimmed());

    let options = ImportOptions {
        input: input.to_string(),
        api_name: api_name.map(String::from),
        module_path: module_path.map(String::from),
        output: output.to_string(),
        dry_run,
        strict,
        verbose: verbose > 0,
    };

    let result = import_pipeline::run_import(&options)?;

    // Print summary
    let warn_count = result
        .diagnostics
        .iter()
        .filter(|d| {
            matches!(
                d.severity,
                schematic_define::openapi::import::DiagnosticSeverity::Warn
            )
        })
        .count();
    let error_count = result
        .diagnostics
        .iter()
        .filter(|d| {
            matches!(
                d.severity,
                schematic_define::openapi::import::DiagnosticSeverity::Error
            )
        })
        .count();

    if !dry_run {
        println!(
            "{} Imported '{}': {} endpoints, {} models",
            "[OK]".green().bold(),
            result.api_name,
            result.endpoint_count,
            result.model_count,
        );
    } else {
        println!(
            "{} Dry run complete for '{}': {} endpoints, {} models",
            "[OK]".green().bold(),
            result.api_name,
            result.endpoint_count,
            result.model_count,
        );
    }

    if warn_count > 0 || error_count > 0 {
        println!(
            "     {} warnings, {} errors in diagnostics",
            warn_count.to_string().yellow(),
            error_count.to_string().red(),
        );
    }

    Ok(())
}

/// Cleans up stale artifacts from an output directory.
///
/// Removes any files in the directory that are not in the expected files set.
///
/// ## Parameters
///
/// - `dir` - The directory to clean
/// - `expected_files` - Set of filenames that should be kept
/// - `verbose` - Verbosity level for logging
///
/// ## Errors
///
/// Returns `GeneratorError::WriteError` if directory reading or file removal fails.
fn cleanup_stale_artifacts(
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

/// Runs the validate command.
fn run_validate(api_name: &str, verbose: u8) -> Result<(), GeneratorError> {
    let api = resolve_api(api_name)?;

    if run_validation(&api, verbose) {
        Ok(())
    } else {
        Err(GeneratorError::ConfigError("Validation failed".to_string()))
    }
}

fn main() -> ExitCode {
    let cli = Cli::parse();

    let result = match cli.command {
        // Explicit subcommand: generate
        Some(Commands::Generate {
            api,
            output,
            dry_run,
            openapi_out,
            openapi_format,
            openapi_version,
            postman_out,
            no_openapi,
            no_postman,
        }) => run_generate(
            &api,
            &GenerateOpts {
                output: &output,
                dry_run,
                verbose: cli.verbose,
                openapi_out: openapi_out.as_deref(),
                openapi_format,
                openapi_version: openapi_version.as_deref(),
                postman_out: postman_out.as_deref(),
                no_openapi,
                no_postman,
            },
        ),
        // Explicit subcommand: validate
        Some(Commands::Validate { api }) => run_validate(&api, cli.verbose),
        // Explicit subcommand: import
        Some(Commands::Import {
            input,
            api_name,
            module_path,
            output,
            dry_run,
            strict,
        }) => run_import_command(
            &input,
            api_name.as_deref(),
            module_path.as_deref(),
            &output,
            dry_run,
            strict,
            cli.verbose,
        ),
        // Explicit subcommand: import-asyncapi
        Some(Commands::ImportAsyncapi {
            input,
            api_name,
            module_path,
            ws_role,
            output,
            dry_run,
            strict,
        }) => run_import_asyncapi_command(
            ImportAsyncapiCommandArgs {
                input: &input,
                api_name: api_name.as_deref(),
                module_path: module_path.as_deref(),
                ws_role,
                output: &output,
                dry_run,
                strict,
            },
            cli.verbose,
        ),
        // No subcommand: backwards-compatible mode (acts like generate)
        None => {
            if let Some(api_name) = cli.api {
                run_generate(
                    &api_name,
                    &GenerateOpts {
                        output: &cli.output,
                        dry_run: cli.dry_run,
                        verbose: cli.verbose,
                        openapi_out: None,
                        openapi_format: OpenApiFormat::default(),
                        openapi_version: None,
                        postman_out: None,
                        no_openapi: false,
                        no_postman: false,
                    },
                )
            } else {
                eprintln!(
                    "{} Missing required argument: --api <NAME>",
                    "[ERROR]".red().bold()
                );
                eprintln!();
                eprintln!("Usage:");
                eprintln!("  schematic-gen --api <NAME> [OPTIONS]");
                eprintln!("  schematic-gen generate --api <NAME> [OPTIONS]");
                eprintln!("  schematic-gen validate --api <NAME>");
                eprintln!();
                eprintln!("Available APIs: {}", AVAILABLE_APIS);
                return ExitCode::from(2);
            }
        }
    };

    match result {
        Ok(()) => ExitCode::SUCCESS,
        Err(err) => {
            eprintln!("{} {}", "[ERROR]".red().bold(), err);
            ExitCode::FAILURE
        }
    }
}
