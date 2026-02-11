//! OpenAPI import pipeline.
//!
//! Wires together the OpenAPI import (parse + transform) with model code
//! generation and client code generation.

use std::path::Path;

use colored::Colorize;
use schematic_define::openapi::import::{DiagnosticSeverity, OpenApiDiagnostic};
use schematic_define::openapi::{OpenApiImport, OpenApiSource};

use crate::errors::GeneratorError;
use crate::model_gen;
use crate::output::generate_and_write_standalone;

/// Options for the import pipeline.
#[derive(Debug)]
pub struct ImportOptions {
    /// Path to the OpenAPI spec file.
    pub input: String,
    /// Override API name.
    pub api_name: Option<String>,
    /// Override module path.
    pub module_path: Option<String>,
    /// Output directory for generated code.
    pub output: String,
    /// Print generated code without writing files.
    pub dry_run: bool,
    /// Fail on any warning-level diagnostic.
    pub strict: bool,
    /// Print all diagnostics to stderr.
    pub verbose: bool,
}

/// Result of a successful import pipeline run.
#[derive(Debug)]
pub struct ImportResult {
    /// API name that was imported.
    pub api_name: String,
    /// Number of endpoints imported.
    pub endpoint_count: usize,
    /// Number of model types imported.
    pub model_count: usize,
    /// Diagnostics generated during import.
    pub diagnostics: Vec<OpenApiDiagnostic>,
}

/// Runs the full import pipeline: parse → import → generate models → generate client.
///
/// ## Errors
///
/// Returns `GeneratorError` if:
/// - The input file cannot be read
/// - The OpenAPI spec fails to parse
/// - Strict mode is enabled and warnings were generated
/// - Code generation fails
pub fn run_import(options: &ImportOptions) -> Result<ImportResult, GeneratorError> {
    // Read the input file
    let source = OpenApiSource::path(&options.input);

    // Configure the import builder
    let mut builder = OpenApiImport::new(source);

    if let Some(ref name) = options.api_name {
        builder = builder.api_name(name);
    }

    if let Some(ref path) = options.module_path {
        builder = builder.module_path(path);
    }

    if options.strict {
        builder = builder.strict();
    }

    builder = builder.prefer_json();

    // Run the import
    let import_result = builder
        .build()
        .map_err(|e| GeneratorError::ParseError(format!("OpenAPI import failed: {}", e)))?;

    // Print diagnostics if verbose
    if options.verbose {
        print_diagnostics(&import_result.diagnostics);
    }

    let api = import_result.api;
    let models = import_result.models;
    let diagnostics = import_result.diagnostics;

    let api_name = api.name.clone();
    let endpoint_count = api.endpoints.len();
    let model_count = models.types.len();

    let output_dir = Path::new(&options.output);

    // Validate model names
    model_gen::validate_model_names(&models)?;

    // Generate model types
    let has_types = !models.types.is_empty();
    if has_types {
        model_gen::generate_models(&models, output_dir, options.dry_run)?;
    }

    // Generate client code using standalone mode (no schematic_definitions imports)
    generate_and_write_standalone(&api, output_dir, options.dry_run, has_types)?;

    Ok(ImportResult {
        api_name,
        endpoint_count,
        model_count,
        diagnostics,
    })
}

/// Prints diagnostics to stderr with colored output.
fn print_diagnostics(diagnostics: &[OpenApiDiagnostic]) {
    for diag in diagnostics {
        let prefix = match diag.severity {
            DiagnosticSeverity::Info => format!("{}", "[INFO]".dimmed()),
            DiagnosticSeverity::Warn => format!("{}", "[WARN]".yellow().bold()),
            DiagnosticSeverity::Error => format!("{}", "[ERROR]".red().bold()),
        };
        eprintln!("{} {} - {}", prefix, diag.location, diag.message);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn import_options_debug() {
        let opts = ImportOptions {
            input: "test.yaml".to_string(),
            api_name: Some("TestApi".to_string()),
            module_path: None,
            output: "./out".to_string(),
            dry_run: true,
            strict: false,
            verbose: false,
        };
        assert!(format!("{:?}", opts).contains("TestApi"));
    }

    #[test]
    fn run_import_from_petstore_yaml() {
        let yaml = r#"
openapi: "3.0.0"
info:
  title: Petstore
  version: "1.0.0"
paths:
  /pets:
    get:
      operationId: listPets
      summary: List all pets
      responses:
        '200':
          description: A list of pets
          content:
            application/json:
              schema:
                type: array
                items:
                  $ref: '#/components/schemas/Pet'
  /pets/{petId}:
    get:
      operationId: showPetById
      summary: Info for a specific pet
      parameters:
        - name: petId
          in: path
          required: true
          schema:
            type: string
      responses:
        '200':
          description: Expected response to a valid request
          content:
            application/json:
              schema:
                $ref: '#/components/schemas/Pet'
components:
  schemas:
    Pet:
      type: object
      required:
        - id
        - name
      properties:
        id:
          type: integer
          format: int64
        name:
          type: string
        tag:
          type: string
"#;

        // Write to a temp file
        let temp_dir = tempfile::TempDir::new().unwrap();
        let spec_path = temp_dir.path().join("petstore.yaml");
        std::fs::write(&spec_path, yaml).unwrap();

        let output_dir = temp_dir.path().join("output");
        std::fs::create_dir_all(&output_dir).unwrap();

        let opts = ImportOptions {
            input: spec_path.to_string_lossy().to_string(),
            api_name: Some("Petstore".to_string()),
            module_path: None,
            output: output_dir.to_string_lossy().to_string(),
            dry_run: true,
            strict: false,
            verbose: false,
        };

        let result = run_import(&opts);
        assert!(result.is_ok(), "Import failed: {:?}", result.err());

        let import_result = result.unwrap();
        assert_eq!(import_result.api_name, "Petstore");
        assert_eq!(import_result.endpoint_count, 2);
        assert!(import_result.model_count > 0);
    }

    #[test]
    fn run_import_nonexistent_file_returns_error() {
        let opts = ImportOptions {
            input: "/nonexistent/path/spec.yaml".to_string(),
            api_name: None,
            module_path: None,
            output: "/tmp/out".to_string(),
            dry_run: true,
            strict: false,
            verbose: false,
        };

        let result = run_import(&opts);
        assert!(result.is_err());
    }
}
