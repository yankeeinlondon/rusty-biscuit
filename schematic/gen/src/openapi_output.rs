//! OpenAPI export file writing.
//!
//! This module provides utilities for writing OpenAPI specifications to files
//! after export from schematic API definitions.

use std::fs;
use std::path::Path;

use schematic_define::openapi::{
    export, serialize, ExportFormat, ExportOptions, SchemaRegistryLike,
};
use schematic_define::RestApi;

use crate::errors::GeneratorError;

/// Writes an OpenAPI specification file to the specified directory.
///
/// This function exports the API definition to OpenAPI 3.0.3 format and writes
/// it to a file in the specified directory. The file is named after the API
/// (lowercased) with the appropriate extension based on the format.
///
/// ## Parameters
///
/// - `api` - The REST API definition to export
/// - `registry` - Schema registry containing JSON schemas for response types
/// - `options` - Export configuration options (format, version, extensions)
/// - `dir` - Output directory for the generated file
///
/// ## Returns
///
/// The path to the written file.
///
/// ## Errors
///
/// Returns `GeneratorError::WriteError` if file writing fails.
/// Returns `GeneratorError::ConfigError` if the directory doesn't exist or export fails.
///
/// ## Examples
///
/// ```ignore
/// use std::path::Path;
/// use schematic_define::openapi::ExportOptions;
/// use schematic_definitions::openai::{define_openai_api, openapi_registry};
/// use schematic_gen::openapi_output::write_openapi;
///
/// let api = define_openai_api();
/// let registry = openapi_registry();
/// let options = ExportOptions::new().with_version("1.0.0");
///
/// let path = write_openapi(&api, &registry, &options, Path::new("./openapi"))?;
/// assert_eq!(path.file_name().unwrap(), "openai.json");
/// ```
pub fn write_openapi<R: SchemaRegistryLike>(
    api: &RestApi,
    registry: &R,
    options: &ExportOptions,
    dir: &Path,
) -> Result<std::path::PathBuf, GeneratorError> {
    // Validate directory exists
    if !dir.exists() {
        return Err(GeneratorError::OutputDirNotFound(dir.display().to_string()));
    }

    // Export to OpenAPI document
    let doc = export(api, registry, options)
        .map_err(|e| GeneratorError::ConfigError(format!("OpenAPI export failed: {}", e)))?;

    // Serialize to string
    let content = serialize(&doc, options.format)
        .map_err(|e| GeneratorError::ConfigError(format!("OpenAPI serialization failed: {}", e)))?;

    // Determine file extension
    let extension = match options.format {
        ExportFormat::Json => "json",
        ExportFormat::Yaml => "yaml",
    };

    // Build file path: {api_name_lowercase}.{extension}
    let file_name = format!("{}.{}", api.name.to_lowercase(), extension);
    let file_path = dir.join(&file_name);

    // Write the file
    fs::write(&file_path, content).map_err(|source| GeneratorError::WriteError {
        path: file_path.display().to_string(),
        source,
    })?;

    Ok(file_path)
}

#[cfg(test)]
mod tests {
    use super::*;
    use biscuit_file::serde_yaml_ng;
    use schematic_define::openapi::ExportFormat;
    use schematic_definitions::openai::{define_openai_api, openapi_registry};
    use tempfile::TempDir;

    // =============================================
    // write_openapi() tests
    // =============================================

    #[test]
    fn write_openapi_creates_json_file() {
        let api = define_openai_api();
        let registry = openapi_registry();
        let options = ExportOptions::new().with_version("1.0.0");
        let temp_dir = TempDir::new().unwrap();

        let result = write_openapi(&api, &registry, &options, temp_dir.path());
        assert!(result.is_ok());

        let path = result.unwrap();
        assert!(path.exists());
        assert_eq!(path.file_name().unwrap().to_str().unwrap(), "openai.json");
    }

    #[test]
    fn write_openapi_creates_yaml_file() {
        let api = define_openai_api();
        let registry = openapi_registry();
        let options = ExportOptions::new()
            .with_version("1.0.0")
            .with_format(ExportFormat::Yaml);
        let temp_dir = TempDir::new().unwrap();

        let result = write_openapi(&api, &registry, &options, temp_dir.path());
        assert!(result.is_ok());

        let path = result.unwrap();
        assert!(path.exists());
        assert_eq!(path.file_name().unwrap().to_str().unwrap(), "openai.yaml");
    }

    #[test]
    fn write_openapi_json_produces_valid_json() {
        let api = define_openai_api();
        let registry = openapi_registry();
        let options = ExportOptions::new().with_version("1.0.0");
        let temp_dir = TempDir::new().unwrap();

        let path = write_openapi(&api, &registry, &options, temp_dir.path()).unwrap();
        let content = fs::read_to_string(&path).unwrap();

        // Should be parseable as JSON
        let parsed: Result<serde_json::Value, _> = serde_json::from_str(&content);
        assert!(parsed.is_ok(), "Should produce valid JSON");

        let doc = parsed.unwrap();
        assert_eq!(doc["openapi"], "3.0.3");
        assert_eq!(doc["info"]["title"], "OpenAI");
    }

    #[test]
    fn write_openapi_yaml_produces_valid_yaml() {
        let api = define_openai_api();
        let registry = openapi_registry();
        let options = ExportOptions::new()
            .with_version("1.0.0")
            .with_format(ExportFormat::Yaml);
        let temp_dir = TempDir::new().unwrap();

        let path = write_openapi(&api, &registry, &options, temp_dir.path()).unwrap();
        let content = fs::read_to_string(&path).unwrap();

        // Should be parseable as YAML
        let parsed: Result<serde_yaml_ng::Value, _> = serde_yaml_ng::from_str(&content);
        assert!(parsed.is_ok(), "Should produce valid YAML");
    }

    #[test]
    fn write_openapi_file_name_lowercase() {
        let api = define_openai_api();
        let registry = openapi_registry();
        let options = ExportOptions::new();
        let temp_dir = TempDir::new().unwrap();

        let path = write_openapi(&api, &registry, &options, temp_dir.path()).unwrap();

        // API name is "OpenAI" but file should be "openai.json"
        assert_eq!(path.file_name().unwrap().to_str().unwrap(), "openai.json");
    }

    #[test]
    fn write_openapi_error_on_missing_directory() {
        let api = define_openai_api();
        let registry = openapi_registry();
        let options = ExportOptions::new();

        let result = write_openapi(&api, &registry, &options, Path::new("/nonexistent/path"));
        assert!(result.is_err());

        match result {
            Err(GeneratorError::OutputDirNotFound(path)) => {
                assert!(path.contains("nonexistent"));
            }
            _ => panic!("Expected OutputDirNotFound error"),
        }
    }

    #[test]
    fn write_openapi_respects_version_option() {
        let api = define_openai_api();
        let registry = openapi_registry();
        let options = ExportOptions::new().with_version("2.5.0");
        let temp_dir = TempDir::new().unwrap();

        let path = write_openapi(&api, &registry, &options, temp_dir.path()).unwrap();
        let content = fs::read_to_string(&path).unwrap();
        let doc: serde_json::Value = serde_json::from_str(&content).unwrap();

        assert_eq!(doc["info"]["version"], "2.5.0");
    }

    #[test]
    fn write_openapi_includes_schemas() {
        let api = define_openai_api();
        let registry = openapi_registry();
        let options = ExportOptions::new();
        let temp_dir = TempDir::new().unwrap();

        let path = write_openapi(&api, &registry, &options, temp_dir.path()).unwrap();
        let content = fs::read_to_string(&path).unwrap();
        let doc: serde_json::Value = serde_json::from_str(&content).unwrap();

        // Verify schemas are included
        assert!(doc["components"]["schemas"]["Model"].is_object());
        assert!(doc["components"]["schemas"]["ListModelsResponse"].is_object());
    }

    #[test]
    fn write_openapi_can_skip_extensions() {
        let api = define_openai_api();
        let registry = openapi_registry();
        let options = ExportOptions::new().skip_extensions();
        let temp_dir = TempDir::new().unwrap();

        let path = write_openapi(&api, &registry, &options, temp_dir.path()).unwrap();
        let content = fs::read_to_string(&path).unwrap();
        let doc: serde_json::Value = serde_json::from_str(&content).unwrap();

        // x-schematic should not be present
        assert!(doc.get("x-schematic").is_none());
    }
}
