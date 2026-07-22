//! Core types for YAML handling.

use std::path::{Path, PathBuf};
use thiserror::Error;
use tracing::instrument;

use super::analyze::{YamlAnalysis, YamlDiagnostic, YamlRepair, analyze_yaml};
use super::location::YamlLocation;

/// Source tracking for YAML content.
#[derive(Debug, Clone)]
pub enum YamlSource {
    /// Content loaded from a file path.
    Path(PathBuf),
    /// Content provided as text.
    Text(String),
    /// Content provided as bytes.
    Bytes(Vec<u8>),
}

/// Error type for YAML operations.
#[derive(Debug, Error)]
pub enum YamlError {
    /// IO error during file operations.
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    /// YAML parsing error.
    #[error("YAML parse error: {0}")]
    Parse(#[from] serde_yaml_ng::Error),

    /// JSON conversion error.
    #[error("JSON conversion error: {0}")]
    JsonConversion(String),

    /// TOML conversion error.
    #[error("TOML conversion error: {0}")]
    TomlConversion(String),

    /// Schema parsing error.
    #[error("Schema parse error: {0}")]
    SchemaParse(#[from] serde_json::Error),

    /// Schema validation error.
    #[error("Schema invalid: {0}")]
    SchemaInvalid(String),

    /// Schema feature not enabled.
    #[error("Schema support requires the 'schema' feature")]
    SchemaFeatureDisabled,

    /// TOML feature not enabled.
    #[cfg(not(feature = "toml"))]
    #[error("TOML support requires the 'toml' feature")]
    TomlFeatureDisabled,

    /// Cycle detected in YAML aliases.
    #[error("Cycle detected in YAML at path: {0}")]
    CycleDetected(String),

    /// Max depth exceeded.
    #[error("Max depth exceeded: {0}")]
    MaxDepthExceeded(usize),
}

impl YamlError {
    /// Returns the structured source location of a parse error.
    ///
    /// Projects the byte offset, line, and column carried by the underlying
    /// `serde_yaml_ng` error into a [`YamlLocation`]. Returns `None` for
    /// non-parse variants, which carry no source position.
    #[must_use]
    pub fn location(&self) -> Option<YamlLocation> {
        match self {
            Self::Parse(error) => error.location().map(YamlLocation::from),
            _ => None,
        }
    }
}

/// Policy for handling non-string map keys.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub enum NonStringKeyPolicy {
    /// Return an error on non-string keys.
    Error,
    /// Convert non-string keys to their string representation.
    #[default]
    Stringify,
    /// Drop entries with non-string keys.
    Drop,
}

/// Policy for handling non-finite float values (NaN, Infinity).
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub enum NonFiniteFloatPolicy {
    /// Return an error on non-finite floats.
    Error,
    /// Convert to null.
    #[default]
    Null,
    /// Convert to string representation.
    Stringify,
}

/// Policy for handling null values (for TOML conversion).
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub enum NullPolicy {
    /// Drop null values from the output.
    #[default]
    Drop,
    /// Convert null to empty string.
    Stringify,
    /// Return an error on null values.
    Error,
}

/// Policy for handling heterogeneous arrays (for TOML conversion).
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub enum HeteroArrayPolicy {
    /// Return an error on heterogeneous arrays.
    #[default]
    Error,
    /// Convert all elements to strings.
    Stringify,
}

/// Options for JSON conversion.
#[derive(Debug, Clone)]
pub struct JsonConversionOptions {
    /// Policy for non-string map keys.
    pub key_policy: NonStringKeyPolicy,
    /// Policy for non-finite floats.
    pub float_policy: NonFiniteFloatPolicy,
    /// Maximum recursion depth.
    pub max_depth: Option<usize>,
}

impl Default for JsonConversionOptions {
    fn default() -> Self {
        Self {
            key_policy: NonStringKeyPolicy::Stringify,
            float_policy: NonFiniteFloatPolicy::Null,
            max_depth: Some(100),
        }
    }
}

/// Options for TOML conversion.
#[derive(Debug, Clone)]
pub struct TomlConversionOptions {
    /// Policy for null values.
    pub null_policy: NullPolicy,
    /// Policy for heterogeneous arrays.
    pub array_policy: HeteroArrayPolicy,
    /// Maximum recursion depth.
    pub max_depth: Option<usize>,
}

impl Default for TomlConversionOptions {
    fn default() -> Self {
        Self {
            null_policy: NullPolicy::Drop,
            array_policy: HeteroArrayPolicy::Error,
            max_depth: Some(100),
        }
    }
}

/// Warning generated during conversion.
#[derive(Debug, Clone)]
pub enum ConversionWarning {
    /// A non-string key was encountered.
    NonStringKey {
        /// JSON path to the key.
        path: String,
    },
    /// A non-finite float was encountered.
    NonFiniteFloat {
        /// JSON path to the value.
        path: String,
    },
    /// A null value was dropped.
    NullDropped {
        /// JSON path to the value.
        path: String,
    },
    /// A heterogeneous array was coerced.
    HeteroArrayCoerced {
        /// JSON path to the array.
        path: String,
    },
}

/// Output from conversion operations, including warnings.
#[derive(Debug, Clone)]
pub struct ConversionOutput<T> {
    /// The converted value.
    pub value: T,
    /// Warnings generated during conversion.
    pub warnings: Vec<ConversionWarning>,
}

impl<T> ConversionOutput<T> {
    /// Create a new conversion output with no warnings.
    #[must_use]
    pub fn new(value: T) -> Self {
        Self {
            value,
            warnings: Vec::new(),
        }
    }

    /// Create a new conversion output with warnings.
    #[must_use]
    pub fn with_warnings(value: T, warnings: Vec<ConversionWarning>) -> Self {
        Self { value, warnings }
    }

    /// Returns true if no warnings were generated.
    #[must_use]
    pub fn is_clean(&self) -> bool {
        self.warnings.is_empty()
    }
}

/// YAML document with parsing, conversion, and validation capabilities.
#[derive(Debug, Clone)]
pub struct Yaml {
    source: YamlSource,
    value: serde_yaml_ng::Value,
    /// Source text read at construction, retained so diagnostics and repairs
    /// never observe a second, TOCTOU-raced version of the input. Populated
    /// by every parsing constructor; `None` only for [`Yaml::from_value`],
    /// which has no authored source.
    retained_source: Option<String>,
}

impl Yaml {
    /// Returns `true` if the input string is valid YAML.
    #[must_use]
    pub fn is_valid(input: impl AsRef<str>) -> bool {
        serde_yaml_ng::from_str::<serde_yaml_ng::Value>(input.as_ref()).is_ok()
    }

    /// Parse a YAML file from the given path.
    ///
    /// ## Errors
    ///
    /// Returns an error if the file cannot be read or contains invalid YAML.
    #[instrument(level = "debug", skip_all, fields(path = %path.as_ref().display()))]
    pub fn new(path: impl AsRef<Path>) -> Result<Self, YamlError> {
        let path = path.as_ref();
        let content = std::fs::read_to_string(path)?;
        let value: serde_yaml_ng::Value = serde_yaml_ng::from_str(&content)?;

        Ok(Self {
            source: YamlSource::Path(path.to_path_buf()),
            value,
            retained_source: Some(content),
        })
    }

    /// Parse YAML from a string.
    ///
    /// ## Errors
    ///
    /// Returns an error if the string contains invalid YAML.
    #[instrument(level = "trace", skip_all, fields(input_len = input.as_ref().len()))]
    #[allow(clippy::should_implement_trait)]
    pub fn from_str(input: impl AsRef<str>) -> Result<Self, YamlError> {
        let input = input.as_ref();
        let value: serde_yaml_ng::Value = serde_yaml_ng::from_str(input)?;

        Ok(Self {
            source: YamlSource::Text(input.to_string()),
            value,
            retained_source: Some(input.to_string()),
        })
    }

    /// Parse YAML from bytes.
    ///
    /// ## Errors
    ///
    /// Returns an error if the bytes contain invalid YAML.
    #[instrument(level = "trace", skip_all, fields(input_len = bytes.as_ref().len()))]
    pub fn from_bytes(bytes: impl AsRef<[u8]>) -> Result<Self, YamlError> {
        let bytes = bytes.as_ref();
        let value: serde_yaml_ng::Value = serde_yaml_ng::from_slice(bytes)?;
        // A successful parse implies valid UTF-8; on the unreachable error
        // arm, fall back to no retained source rather than a lossy copy that
        // would corrupt byte spans.
        let retained_source = std::str::from_utf8(bytes).ok().map(str::to_string);

        Ok(Self {
            source: YamlSource::Bytes(bytes.to_vec()),
            value,
            retained_source,
        })
    }

    /// Create from an existing parsed value.
    ///
    /// The value has no authored source, so [`Yaml::source_text`] returns
    /// `None` and span-based diagnostics are unavailable.
    #[must_use]
    pub fn from_value(value: serde_yaml_ng::Value) -> Self {
        Self {
            source: YamlSource::Text(String::new()),
            value,
            retained_source: None,
        }
    }

    /// Returns a reference to the parsed YAML value.
    #[must_use]
    pub fn value(&self) -> &serde_yaml_ng::Value {
        &self.value
    }

    /// Returns the source of this YAML document.
    #[must_use]
    pub fn source(&self) -> &YamlSource {
        &self.source
    }

    /// Returns the authored source text read at construction.
    ///
    /// This is the single read path for span-based diagnostics and repairs:
    /// for path-backed values it yields the text captured when the file was
    /// read (never a second filesystem read), and for [`Yaml::from_value`] it
    /// returns `None` because no authored source exists.
    #[must_use]
    pub fn source_text(&self) -> Option<&str> {
        self.retained_source.as_deref()
    }

    /// Analyzes the authored source with the source-first analyzer.
    ///
    /// This is the entry point to prefer: the returned [`YamlAnalysis`] is a
    /// single scan from which the diagnostics view
    /// ([`YamlAnalysis::diagnostics`]), the repairs view
    /// ([`YamlAnalysis::repairs`]), and the patched source
    /// ([`YamlAnalysis::apply`]) are all derived without re-scanning. Callers
    /// that want more than one of those views should retain the analysis
    /// rather than reach for [`Yaml::diagnose`] and
    /// [`Yaml::repair_candidates`], which scan once each.
    ///
    /// ## Examples
    ///
    /// ```rust
    /// use biscuit_file::Yaml;
    ///
    /// let yaml = Yaml::from_str("key: value  \n")?;
    /// if let Some(analysis) = yaml.analyze() {
    ///     let findings = analysis.diagnostics().len();
    ///     let offered = analysis.repairs().count();
    ///     let patched = analysis.apply();
    ///     assert_eq!(patched.source, "key: value\n");
    ///     assert!(findings > 0 && offered > 0);
    /// }
    /// # Ok::<(), biscuit_file::YamlError>(())
    /// ```
    ///
    /// ## Returns
    ///
    /// `None` for values built by [`Yaml::from_value`], which carry no
    /// authored source. An empty-source analysis is deliberately not
    /// synthesized: [`YamlAnalysis::apply`] would then report an empty
    /// document as the patched result, which a caller writing that source
    /// back to disk would act on destructively.
    #[must_use]
    pub fn analyze(&self) -> Option<YamlAnalysis> {
        self.source_text().map(analyze_yaml)
    }

    /// Diagnoses the authored source, discarding the rest of the analysis.
    ///
    /// Shorthand for the diagnostics view of [`Yaml::analyze`]; it performs
    /// its own scan, so pairing it with [`Yaml::repair_candidates`] analyzes
    /// the same source twice. Retain a single [`YamlAnalysis`] when both
    /// views are needed.
    ///
    /// Values constructed via [`Yaml::from_value`] have no authored source
    /// and return no diagnostics — value-level diagnostics without source
    /// spans are out of scope.
    #[must_use]
    pub fn diagnose(&self) -> Vec<YamlDiagnostic> {
        self.analyze()
            .map(|analysis| analysis.diagnostics().to_vec())
            .unwrap_or_default()
    }

    /// Returns every candidate repair attached to [`Yaml::diagnose`]
    /// findings, in stable source order.
    ///
    /// Shorthand for the repairs view of [`Yaml::analyze`]; see
    /// [`Yaml::diagnose`] for the duplicate-scan caveat when both views are
    /// wanted.
    ///
    /// Candidates are only ever attached when they have satisfied their
    /// safety proof; whether a candidate is eligible for automatic
    /// application is gated by the diagnostic's classification. Values
    /// constructed via [`Yaml::from_value`] return no candidates.
    #[must_use]
    pub fn repair_candidates(&self) -> Vec<YamlRepair> {
        self.analyze()
            .map(|analysis| analysis.repairs().cloned().collect())
            .unwrap_or_default()
    }

    /// Convert to a JSON value with default options.
    ///
    /// ## Errors
    ///
    /// Returns an error if conversion fails.
    #[instrument(level = "trace", skip(self), fields(source = ?self.source))]
    pub fn as_json(&self) -> Result<serde_json::Value, YamlError> {
        let output = self.as_json_with(JsonConversionOptions::default())?;
        Ok(output.value)
    }

    /// Convert to a JSON value with custom options.
    ///
    /// ## Errors
    ///
    /// Returns an error if conversion fails based on the configured policies.
    pub fn as_json_with(
        &self,
        options: JsonConversionOptions,
    ) -> Result<ConversionOutput<serde_json::Value>, YamlError> {
        let mut warnings = Vec::new();
        let value = yaml_to_json(&self.value, &options, &mut warnings, "", 0)?;
        Ok(ConversionOutput::with_warnings(value, warnings))
    }

    /// Convert to a TOML value with default options.
    ///
    /// ## Errors
    ///
    /// Returns an error if conversion fails.
    #[cfg(feature = "toml")]
    #[instrument(level = "trace", skip(self), fields(source = ?self.source))]
    pub fn as_toml(&self) -> Result<toml::Value, YamlError> {
        let output = self.as_toml_with(TomlConversionOptions::default())?;
        Ok(output.value)
    }

    /// Convert to a TOML value.
    #[cfg(not(feature = "toml"))]
    pub fn as_toml(&self) -> Result<(), YamlError> {
        Err(YamlError::TomlFeatureDisabled)
    }

    /// Convert to a TOML value with custom options.
    ///
    /// ## Errors
    ///
    /// Returns an error if conversion fails based on the configured policies.
    #[cfg(feature = "toml")]
    pub fn as_toml_with(
        &self,
        options: TomlConversionOptions,
    ) -> Result<ConversionOutput<toml::Value>, YamlError> {
        let mut warnings = Vec::new();
        let value = yaml_to_toml(&self.value, &options, &mut warnings, "", 0)?;
        Ok(ConversionOutput::with_warnings(value, warnings))
    }

    /// Validate the YAML document.
    ///
    /// Returns Ok if the YAML is syntactically valid (already verified at parse time).
    pub fn validate(&self) -> Result<(), YamlError> {
        // Parsing already validates syntax
        Ok(())
    }
}

/// Convert YAML value to JSON value.
fn yaml_to_json(
    value: &serde_yaml_ng::Value,
    options: &JsonConversionOptions,
    warnings: &mut Vec<ConversionWarning>,
    path: &str,
    depth: usize,
) -> Result<serde_json::Value, YamlError> {
    if let Some(max) = options.max_depth
        && depth > max
    {
        return Err(YamlError::MaxDepthExceeded(max));
    }

    match value {
        serde_yaml_ng::Value::Null => Ok(serde_json::Value::Null),
        serde_yaml_ng::Value::Bool(b) => Ok(serde_json::Value::Bool(*b)),
        serde_yaml_ng::Value::Number(n) => {
            if let Some(i) = n.as_i64() {
                Ok(serde_json::Value::Number(i.into()))
            } else if let Some(u) = n.as_u64() {
                Ok(serde_json::Value::Number(u.into()))
            } else if let Some(f) = n.as_f64() {
                if f.is_finite() {
                    serde_json::Number::from_f64(f)
                        .map(serde_json::Value::Number)
                        .ok_or_else(|| {
                            YamlError::JsonConversion(format!("Invalid float at {path}"))
                        })
                } else {
                    match options.float_policy {
                        NonFiniteFloatPolicy::Error => Err(YamlError::JsonConversion(format!(
                            "Non-finite float at {path}"
                        ))),
                        NonFiniteFloatPolicy::Null => {
                            warnings.push(ConversionWarning::NonFiniteFloat {
                                path: path.to_string(),
                            });
                            Ok(serde_json::Value::Null)
                        }
                        NonFiniteFloatPolicy::Stringify => {
                            warnings.push(ConversionWarning::NonFiniteFloat {
                                path: path.to_string(),
                            });
                            Ok(serde_json::Value::String(f.to_string()))
                        }
                    }
                }
            } else {
                Err(YamlError::JsonConversion(format!(
                    "Invalid number at {path}"
                )))
            }
        }
        serde_yaml_ng::Value::String(s) => Ok(serde_json::Value::String(s.clone())),
        serde_yaml_ng::Value::Sequence(seq) => {
            let items: Result<Vec<_>, _> = seq
                .iter()
                .enumerate()
                .map(|(i, v)| {
                    let item_path = format!("{path}[{i}]");
                    yaml_to_json(v, options, warnings, &item_path, depth + 1)
                })
                .collect();
            Ok(serde_json::Value::Array(items?))
        }
        serde_yaml_ng::Value::Mapping(map) => {
            let mut obj = serde_json::Map::new();
            for (k, v) in map {
                let key = match k {
                    serde_yaml_ng::Value::String(s) => s.clone(),
                    other => match options.key_policy {
                        NonStringKeyPolicy::Error => {
                            return Err(YamlError::JsonConversion(format!(
                                "Non-string key at {path}: {:?}",
                                other
                            )));
                        }
                        NonStringKeyPolicy::Stringify => {
                            warnings.push(ConversionWarning::NonStringKey {
                                path: path.to_string(),
                            });
                            format!("{:?}", other)
                        }
                        NonStringKeyPolicy::Drop => {
                            warnings.push(ConversionWarning::NonStringKey {
                                path: path.to_string(),
                            });
                            continue;
                        }
                    },
                };
                let item_path = if path.is_empty() {
                    key.clone()
                } else {
                    format!("{path}.{key}")
                };
                let val = yaml_to_json(v, options, warnings, &item_path, depth + 1)?;
                obj.insert(key, val);
            }
            Ok(serde_json::Value::Object(obj))
        }
        serde_yaml_ng::Value::Tagged(tagged) => {
            // Ignore the tag and just convert the value
            yaml_to_json(&tagged.value, options, warnings, path, depth)
        }
    }
}

/// Convert YAML value to TOML value.
#[cfg(feature = "toml")]
fn yaml_to_toml(
    value: &serde_yaml_ng::Value,
    options: &TomlConversionOptions,
    warnings: &mut Vec<ConversionWarning>,
    path: &str,
    depth: usize,
) -> Result<toml::Value, YamlError> {
    if let Some(max) = options.max_depth
        && depth > max
    {
        return Err(YamlError::MaxDepthExceeded(max));
    }

    match value {
        serde_yaml_ng::Value::Null => match options.null_policy {
            NullPolicy::Error => Err(YamlError::TomlConversion(format!("Null value at {path}"))),
            NullPolicy::Drop => {
                warnings.push(ConversionWarning::NullDropped {
                    path: path.to_string(),
                });
                // Return empty string as placeholder - caller should filter
                Ok(toml::Value::String(String::new()))
            }
            NullPolicy::Stringify => {
                warnings.push(ConversionWarning::NullDropped {
                    path: path.to_string(),
                });
                Ok(toml::Value::String("null".to_string()))
            }
        },
        serde_yaml_ng::Value::Bool(b) => Ok(toml::Value::Boolean(*b)),
        serde_yaml_ng::Value::Number(n) => {
            if let Some(i) = n.as_i64() {
                Ok(toml::Value::Integer(i))
            } else if let Some(f) = n.as_f64() {
                if f.is_finite() {
                    Ok(toml::Value::Float(f))
                } else {
                    Err(YamlError::TomlConversion(format!(
                        "Non-finite float at {path}"
                    )))
                }
            } else {
                Err(YamlError::TomlConversion(format!(
                    "Invalid number at {path}"
                )))
            }
        }
        serde_yaml_ng::Value::String(s) => Ok(toml::Value::String(s.clone())),
        serde_yaml_ng::Value::Sequence(seq) => {
            if seq.is_empty() {
                return Ok(toml::Value::Array(vec![]));
            }

            // Check for heterogeneous array
            let first_type = yaml_type_name(seq.first().unwrap());
            let is_homogeneous = seq.iter().all(|v| yaml_type_name(v) == first_type);

            if !is_homogeneous {
                match options.array_policy {
                    HeteroArrayPolicy::Error => {
                        return Err(YamlError::TomlConversion(format!(
                            "Heterogeneous array at {path}"
                        )));
                    }
                    HeteroArrayPolicy::Stringify => {
                        warnings.push(ConversionWarning::HeteroArrayCoerced {
                            path: path.to_string(),
                        });
                        let items: Vec<toml::Value> = seq
                            .iter()
                            .map(|v| toml::Value::String(format!("{:?}", v)))
                            .collect();
                        return Ok(toml::Value::Array(items));
                    }
                }
            }

            let items: Result<Vec<_>, _> = seq
                .iter()
                .enumerate()
                .map(|(i, v)| {
                    let item_path = format!("{path}[{i}]");
                    yaml_to_toml(v, options, warnings, &item_path, depth + 1)
                })
                .collect();
            Ok(toml::Value::Array(items?))
        }
        serde_yaml_ng::Value::Mapping(map) => {
            let mut table = toml::map::Map::new();
            for (k, v) in map {
                let key = match k {
                    serde_yaml_ng::Value::String(s) => s.clone(),
                    _ => {
                        return Err(YamlError::TomlConversion(format!(
                            "Non-string key at {path}"
                        )));
                    }
                };
                let item_path = if path.is_empty() {
                    key.clone()
                } else {
                    format!("{path}.{key}")
                };

                // Skip null values if policy is Drop
                if matches!(v, serde_yaml_ng::Value::Null)
                    && options.null_policy == NullPolicy::Drop
                {
                    warnings.push(ConversionWarning::NullDropped {
                        path: item_path.clone(),
                    });
                    continue;
                }

                let val = yaml_to_toml(v, options, warnings, &item_path, depth + 1)?;
                table.insert(key, val);
            }
            Ok(toml::Value::Table(table))
        }
        serde_yaml_ng::Value::Tagged(tagged) => {
            yaml_to_toml(&tagged.value, options, warnings, path, depth)
        }
    }
}

/// Get a type name for YAML value for homogeneity checking.
#[cfg(feature = "toml")]
fn yaml_type_name(value: &serde_yaml_ng::Value) -> &'static str {
    match value {
        serde_yaml_ng::Value::Null => "null",
        serde_yaml_ng::Value::Bool(_) => "bool",
        serde_yaml_ng::Value::Number(_) => "number",
        serde_yaml_ng::Value::String(_) => "string",
        serde_yaml_ng::Value::Sequence(_) => "array",
        serde_yaml_ng::Value::Mapping(_) => "object",
        serde_yaml_ng::Value::Tagged(_) => "tagged",
    }
}

// TryFrom implementations

impl TryFrom<&str> for Yaml {
    type Error = YamlError;

    /// Interpret the string as a file path and load the YAML file.
    fn try_from(path: &str) -> Result<Self, Self::Error> {
        Self::new(path)
    }
}

impl TryFrom<String> for Yaml {
    type Error = YamlError;

    /// Interpret the string as a file path and load the YAML file.
    fn try_from(path: String) -> Result<Self, Self::Error> {
        Self::new(path)
    }
}

impl TryFrom<PathBuf> for Yaml {
    type Error = YamlError;

    fn try_from(path: PathBuf) -> Result<Self, Self::Error> {
        Self::new(path)
    }
}

impl TryFrom<&Path> for Yaml {
    type Error = YamlError;

    fn try_from(path: &Path) -> Result<Self, Self::Error> {
        Self::new(path)
    }
}

impl TryFrom<Vec<u8>> for Yaml {
    type Error = YamlError;

    fn try_from(bytes: Vec<u8>) -> Result<Self, Self::Error> {
        Self::from_bytes(bytes)
    }
}

impl From<serde_yaml_ng::Value> for Yaml {
    fn from(value: serde_yaml_ng::Value) -> Self {
        Self::from_value(value)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;

    #[test]
    fn test_is_valid_accepts_valid_yaml() {
        assert!(Yaml::is_valid("name: test"));
        assert!(Yaml::is_valid("items:\n  - one\n  - two"));
        assert!(Yaml::is_valid("key: 42\nenabled: true"));
    }

    #[test]
    fn test_is_valid_rejects_invalid_yaml() {
        assert!(!Yaml::is_valid(":\n  - :\n  -: :\n:"));
        assert!(!Yaml::is_valid("key: [unclosed"));
    }

    #[test]
    fn test_parse_simple_yaml() {
        let yaml = Yaml::from_str(
            r#"
            name: test
            version: "0.1.0"
        "#,
        )
        .unwrap();

        assert!(yaml.value().is_mapping());
    }

    #[test]
    fn test_invalid_yaml_error() {
        let result = Yaml::from_str("key: [invalid");
        assert!(result.is_err());
    }

    #[test]
    fn test_as_json() {
        let yaml = Yaml::from_str(
            r#"
            name: test
            count: 42
            enabled: true
        "#,
        )
        .unwrap();

        let json = yaml.as_json().unwrap();
        assert!(json.is_object());
        assert_eq!(json["name"], "test");
        assert_eq!(json["count"], 42);
        assert_eq!(json["enabled"], true);
    }

    #[cfg(feature = "toml")]
    #[test]
    fn test_as_toml() {
        let yaml = Yaml::from_str(
            r#"
            name: test
            count: 42
        "#,
        )
        .unwrap();

        let toml_val = yaml.as_toml().unwrap();
        assert!(toml_val.is_table());
    }

    #[test]
    fn test_conversion_output() {
        let output = ConversionOutput::new(42);
        assert!(output.is_clean());
        assert_eq!(output.value, 42);

        let output_with_warnings = ConversionOutput::with_warnings(
            42,
            vec![ConversionWarning::NullDropped {
                path: "$.foo".to_string(),
            }],
        );
        assert!(!output_with_warnings.is_clean());
    }

    // ===== NonStringKeyPolicy Tests =====

    #[test]
    fn test_non_string_key_policy_error() {
        // YAML allows integer keys
        let yaml = Yaml::from_str(
            r#"
            123: value_for_int_key
            name: test
        "#,
        )
        .unwrap();

        let options = JsonConversionOptions {
            key_policy: NonStringKeyPolicy::Error,
            ..Default::default()
        };

        let result = yaml.as_json_with(options);
        assert!(result.is_err());
        let err = result.unwrap_err();
        assert!(matches!(err, YamlError::JsonConversion(_)));
    }

    #[test]
    fn test_non_string_key_policy_stringify() {
        // YAML allows integer keys
        let yaml = Yaml::from_str(
            r#"
            123: value_for_int_key
            name: test
        "#,
        )
        .unwrap();

        let options = JsonConversionOptions {
            key_policy: NonStringKeyPolicy::Stringify,
            ..Default::default()
        };

        let result = yaml.as_json_with(options).unwrap();
        assert!(!result.is_clean());
        assert_eq!(result.warnings.len(), 1);
        assert!(matches!(
            &result.warnings[0],
            ConversionWarning::NonStringKey { .. }
        ));
        // The key should be stringified
        assert!(result.value.is_object());
    }

    #[test]
    fn test_non_string_key_policy_drop() {
        // YAML allows integer keys
        let yaml = Yaml::from_str(
            r#"
            123: value_for_int_key
            name: test
        "#,
        )
        .unwrap();

        let options = JsonConversionOptions {
            key_policy: NonStringKeyPolicy::Drop,
            ..Default::default()
        };

        let result = yaml.as_json_with(options).unwrap();
        assert!(!result.is_clean());
        assert_eq!(result.warnings.len(), 1);
        // The integer key should be dropped
        let obj = result.value.as_object().unwrap();
        assert!(obj.contains_key("name"));
        // The stringified key should NOT be present (dropped)
        assert!(!obj.values().any(|v| v == "value_for_int_key"));
    }

    // ===== NonFiniteFloatPolicy Tests =====

    #[test]
    fn test_non_finite_float_policy_error_nan() {
        let yaml = Yaml::from_str(
            r#"
            value: .nan
        "#,
        )
        .unwrap();

        let options = JsonConversionOptions {
            float_policy: NonFiniteFloatPolicy::Error,
            ..Default::default()
        };

        let result = yaml.as_json_with(options);
        assert!(result.is_err());
        let err = result.unwrap_err();
        assert!(matches!(err, YamlError::JsonConversion(_)));
    }

    #[test]
    fn test_non_finite_float_policy_error_infinity() {
        let yaml = Yaml::from_str(
            r#"
            value: .inf
        "#,
        )
        .unwrap();

        let options = JsonConversionOptions {
            float_policy: NonFiniteFloatPolicy::Error,
            ..Default::default()
        };

        let result = yaml.as_json_with(options);
        assert!(result.is_err());
    }

    #[test]
    fn test_non_finite_float_policy_null_nan() {
        let yaml = Yaml::from_str(
            r#"
            value: .nan
        "#,
        )
        .unwrap();

        let options = JsonConversionOptions {
            float_policy: NonFiniteFloatPolicy::Null,
            ..Default::default()
        };

        let result = yaml.as_json_with(options).unwrap();
        assert!(!result.is_clean());
        assert_eq!(result.warnings.len(), 1);
        assert!(matches!(
            &result.warnings[0],
            ConversionWarning::NonFiniteFloat { .. }
        ));
        assert!(result.value["value"].is_null());
    }

    #[test]
    fn test_non_finite_float_policy_stringify_nan() {
        let yaml = Yaml::from_str(
            r#"
            value: .nan
        "#,
        )
        .unwrap();

        let options = JsonConversionOptions {
            float_policy: NonFiniteFloatPolicy::Stringify,
            ..Default::default()
        };

        let result = yaml.as_json_with(options).unwrap();
        assert!(!result.is_clean());
        assert!(result.value["value"].is_string());
        // NaN stringified
        assert!(result.value["value"].as_str().unwrap().contains("NaN"));
    }

    #[test]
    fn test_non_finite_float_policy_stringify_infinity() {
        let yaml = Yaml::from_str(
            r#"
            value: .inf
        "#,
        )
        .unwrap();

        let options = JsonConversionOptions {
            float_policy: NonFiniteFloatPolicy::Stringify,
            ..Default::default()
        };

        let result = yaml.as_json_with(options).unwrap();
        assert!(!result.is_clean());
        assert!(result.value["value"].is_string());
        // Infinity stringified
        assert!(result.value["value"].as_str().unwrap().contains("inf"));
    }

    #[test]
    fn test_non_finite_float_policy_negative_infinity() {
        let yaml = Yaml::from_str(
            r#"
            value: -.inf
        "#,
        )
        .unwrap();

        let options = JsonConversionOptions {
            float_policy: NonFiniteFloatPolicy::Stringify,
            ..Default::default()
        };

        let result = yaml.as_json_with(options).unwrap();
        assert!(!result.is_clean());
        assert!(result.value["value"].is_string());
    }

    // ===== Max Depth (Cycle Protection) Tests =====

    #[test]
    fn test_max_depth_exceeded_json() {
        // Create deeply nested YAML
        let mut nested = "value: test".to_string();
        for _ in 0..10 {
            nested = format!("level:\n  {}", nested.replace('\n', "\n  "));
        }

        let yaml = Yaml::from_str(&nested).unwrap();

        let options = JsonConversionOptions {
            max_depth: Some(5),
            ..Default::default()
        };

        let result = yaml.as_json_with(options);
        assert!(result.is_err());
        let err = result.unwrap_err();
        assert!(matches!(err, YamlError::MaxDepthExceeded(5)));
    }

    #[test]
    fn test_max_depth_exceeded_toml() {
        // Create deeply nested YAML
        let mut nested = "value: test".to_string();
        for _ in 0..10 {
            nested = format!("level:\n  {}", nested.replace('\n', "\n  "));
        }

        let yaml = Yaml::from_str(&nested).unwrap();

        #[cfg(feature = "toml")]
        {
            let options = TomlConversionOptions {
                max_depth: Some(5),
                ..Default::default()
            };

            let result = yaml.as_toml_with(options);
            assert!(result.is_err());
            let err = result.unwrap_err();
            assert!(matches!(err, YamlError::MaxDepthExceeded(5)));
        }
    }

    #[test]
    fn test_max_depth_within_limit() {
        // Nested but within limit
        let yaml = Yaml::from_str(
            r#"
            level1:
              level2:
                level3:
                  value: test
        "#,
        )
        .unwrap();

        let options = JsonConversionOptions {
            max_depth: Some(10),
            ..Default::default()
        };

        let result = yaml.as_json_with(options);
        assert!(result.is_ok());
    }

    #[test]
    fn test_max_depth_none_allows_deep_nesting() {
        // Create deeply nested YAML
        let mut nested = "value: test".to_string();
        for _ in 0..50 {
            nested = format!("level:\n  {}", nested.replace('\n', "\n  "));
        }

        let yaml = Yaml::from_str(&nested).unwrap();

        let options = JsonConversionOptions {
            max_depth: None,
            ..Default::default()
        };

        let result = yaml.as_json_with(options);
        assert!(result.is_ok());
    }

    // ===== TOML NullPolicy Tests =====

    #[cfg(feature = "toml")]
    #[test]
    fn test_null_policy_drop() {
        let yaml = Yaml::from_str(
            r#"
            name: test
            value: null
        "#,
        )
        .unwrap();

        let options = TomlConversionOptions {
            null_policy: NullPolicy::Drop,
            ..Default::default()
        };

        let result = yaml.as_toml_with(options).unwrap();
        assert!(!result.is_clean());
        assert!(
            result
                .warnings
                .iter()
                .any(|w| matches!(w, ConversionWarning::NullDropped { .. }))
        );
        let table = result.value.as_table().unwrap();
        assert!(table.contains_key("name"));
        assert!(!table.contains_key("value"));
    }

    #[cfg(feature = "toml")]
    #[test]
    fn test_null_policy_stringify() {
        let yaml = Yaml::from_str(
            r#"
            name: test
            value: null
        "#,
        )
        .unwrap();

        let options = TomlConversionOptions {
            null_policy: NullPolicy::Stringify,
            ..Default::default()
        };

        let result = yaml.as_toml_with(options).unwrap();
        let table = result.value.as_table().unwrap();
        assert!(table.contains_key("value"));
        assert_eq!(table["value"].as_str(), Some("null"));
    }

    #[cfg(feature = "toml")]
    #[test]
    fn test_null_policy_error() {
        let yaml = Yaml::from_str(
            r#"
            name: test
            value: null
        "#,
        )
        .unwrap();

        let options = TomlConversionOptions {
            null_policy: NullPolicy::Error,
            ..Default::default()
        };

        let result = yaml.as_toml_with(options);
        assert!(result.is_err());
        let err = result.unwrap_err();
        assert!(matches!(err, YamlError::TomlConversion(_)));
    }

    // ===== TOML HeteroArrayPolicy Tests =====

    #[cfg(feature = "toml")]
    #[test]
    fn test_hetero_array_policy_error() {
        let yaml = Yaml::from_str(
            r#"
            items:
              - 1
              - "two"
              - 3
        "#,
        )
        .unwrap();

        let options = TomlConversionOptions {
            array_policy: HeteroArrayPolicy::Error,
            ..Default::default()
        };

        let result = yaml.as_toml_with(options);
        assert!(result.is_err());
        let err = result.unwrap_err();
        assert!(matches!(err, YamlError::TomlConversion(_)));
    }

    #[cfg(feature = "toml")]
    #[test]
    fn test_hetero_array_policy_stringify() {
        let yaml = Yaml::from_str(
            r#"
            items:
              - 1
              - "two"
              - 3
        "#,
        )
        .unwrap();

        let options = TomlConversionOptions {
            array_policy: HeteroArrayPolicy::Stringify,
            ..Default::default()
        };

        let result = yaml.as_toml_with(options).unwrap();
        assert!(!result.is_clean());
        assert!(
            result
                .warnings
                .iter()
                .any(|w| matches!(w, ConversionWarning::HeteroArrayCoerced { .. }))
        );
        let arr = result.value["items"].as_array().unwrap();
        // All elements should be strings now
        assert!(arr.iter().all(|v| v.is_str()));
    }

    #[cfg(feature = "toml")]
    #[test]
    fn test_homogeneous_array_passes() {
        let yaml = Yaml::from_str(
            r#"
            items:
              - 1
              - 2
              - 3
        "#,
        )
        .unwrap();

        let options = TomlConversionOptions {
            array_policy: HeteroArrayPolicy::Error,
            ..Default::default()
        };

        let result = yaml.as_toml_with(options);
        assert!(result.is_ok());
        let output = result.unwrap();
        let arr = output.value["items"].as_array().unwrap();
        assert_eq!(arr.len(), 3);
    }

    // ===== TryFrom Implementation Tests =====

    #[test]
    fn test_try_from_bytes() {
        let bytes = b"name: test\ncount: 42";
        let yaml = Yaml::try_from(bytes.to_vec()).unwrap();
        assert!(yaml.value().is_mapping());
        let json = yaml.as_json().unwrap();
        assert_eq!(json["name"], "test");
        assert_eq!(json["count"], 42);
    }

    #[test]
    fn test_try_from_path_not_found() {
        let result = Yaml::try_from("/nonexistent/path/to/file.yaml");
        assert!(result.is_err());
        let err = result.unwrap_err();
        assert!(matches!(err, YamlError::Io(_)));
    }

    #[test]
    fn test_try_from_pathbuf() {
        // Create a temp file
        let dir = std::env::temp_dir();
        let path = dir.join("biscuit_yaml_test.yaml");
        let mut file = std::fs::File::create(&path).unwrap();
        writeln!(file, "name: from_pathbuf").unwrap();
        drop(file);

        let yaml = Yaml::try_from(path.clone()).unwrap();
        let json = yaml.as_json().unwrap();
        assert_eq!(json["name"], "from_pathbuf");

        // Cleanup
        std::fs::remove_file(path).ok();
    }

    #[test]
    fn test_try_from_path_ref() {
        // Create a temp file
        let dir = std::env::temp_dir();
        let path = dir.join("biscuit_yaml_test2.yaml");
        let mut file = std::fs::File::create(&path).unwrap();
        writeln!(file, "name: from_path_ref").unwrap();
        drop(file);

        let yaml = Yaml::try_from(path.as_path()).unwrap();
        let json = yaml.as_json().unwrap();
        assert_eq!(json["name"], "from_path_ref");

        // Cleanup
        std::fs::remove_file(path).ok();
    }

    #[test]
    fn test_from_serde_value() {
        use serde_yaml_ng::Value;
        let mut map = serde_yaml_ng::Mapping::new();
        map.insert(
            Value::String("key".to_string()),
            Value::String("value".to_string()),
        );
        let value = Value::Mapping(map);

        let yaml = Yaml::from(value);
        let json = yaml.as_json().unwrap();
        assert_eq!(json["key"], "value");
    }

    // ===== Source Tracking Tests =====

    #[test]
    fn test_source_from_str() {
        let yaml = Yaml::from_str("name: test").unwrap();
        assert!(matches!(yaml.source(), YamlSource::Text(_)));
    }

    #[test]
    fn test_source_from_bytes() {
        let yaml = Yaml::from_bytes(b"name: test").unwrap();
        assert!(matches!(yaml.source(), YamlSource::Bytes(_)));
    }

    #[test]
    fn test_source_from_value() {
        let value = serde_yaml_ng::Value::Null;
        let yaml = Yaml::from_value(value);
        // from_value uses Text source with empty string
        assert!(matches!(yaml.source(), YamlSource::Text(_)));
    }

    // ===== Arrays in JSON conversion =====

    #[test]
    fn test_array_conversion_json() {
        let yaml = Yaml::from_str(
            r#"
            items:
              - one
              - two
              - three
        "#,
        )
        .unwrap();

        let json = yaml.as_json().unwrap();
        let arr = json["items"].as_array().unwrap();
        assert_eq!(arr.len(), 3);
        assert_eq!(arr[0], "one");
        assert_eq!(arr[1], "two");
        assert_eq!(arr[2], "three");
    }

    #[test]
    fn test_nested_arrays_json() {
        let yaml = Yaml::from_str(
            r#"
            matrix:
              - [1, 2, 3]
              - [4, 5, 6]
        "#,
        )
        .unwrap();

        let json = yaml.as_json().unwrap();
        let matrix = json["matrix"].as_array().unwrap();
        assert_eq!(matrix.len(), 2);
        assert_eq!(matrix[0][0], 1);
        assert_eq!(matrix[1][2], 6);
    }

    // ===== Tagged Values Tests =====

    #[test]
    fn test_tagged_value_json() {
        // YAML tags should be ignored during JSON conversion
        let yaml = Yaml::from_str(
            r#"
            value: !custom_tag "tagged_value"
        "#,
        )
        .unwrap();

        let json = yaml.as_json().unwrap();
        assert_eq!(json["value"], "tagged_value");
    }

    // ===== Complex Nested Structures =====

    #[test]
    fn test_complex_nested_structure() {
        let yaml = Yaml::from_str(
            r#"
            database:
              host: localhost
              port: 5432
              credentials:
                username: admin
                password: secret
              pools:
                - name: primary
                  size: 10
                - name: replica
                  size: 5
        "#,
        )
        .unwrap();

        let json = yaml.as_json().unwrap();
        assert_eq!(json["database"]["host"], "localhost");
        assert_eq!(json["database"]["port"], 5432);
        assert_eq!(json["database"]["credentials"]["username"], "admin");
        assert_eq!(json["database"]["pools"][0]["name"], "primary");
        assert_eq!(json["database"]["pools"][1]["size"], 5);
    }

    // ===== Empty Values Tests =====

    #[test]
    fn test_empty_mapping() {
        let yaml = Yaml::from_str("{}").unwrap();
        let json = yaml.as_json().unwrap();
        assert!(json.is_object());
        assert_eq!(json.as_object().unwrap().len(), 0);
    }

    #[test]
    fn test_empty_sequence() {
        let yaml = Yaml::from_str("items: []").unwrap();
        let json = yaml.as_json().unwrap();
        assert!(json["items"].is_array());
        assert_eq!(json["items"].as_array().unwrap().len(), 0);
    }

    #[cfg(feature = "toml")]
    #[test]
    fn test_empty_array_toml() {
        let yaml = Yaml::from_str("items: []").unwrap();
        let toml_val = yaml.as_toml().unwrap();
        assert!(toml_val["items"].is_array());
        assert_eq!(toml_val["items"].as_array().unwrap().len(), 0);
    }

    // ===== Boolean and Number Edge Cases =====

    #[test]
    fn test_yaml_boolean_variants() {
        // Note: serde_yaml_ng uses YAML 1.2 by default which only treats
        // true/false as booleans, not yes/no/on/off (those are YAML 1.1)
        let yaml = Yaml::from_str(
            r#"
            true_bool: true
            false_bool: false
            True_bool: True
            False_bool: False
        "#,
        )
        .unwrap();

        let json = yaml.as_json().unwrap();
        assert_eq!(json["true_bool"], true);
        assert_eq!(json["false_bool"], false);
        assert_eq!(json["True_bool"], true);
        assert_eq!(json["False_bool"], false);
    }

    #[test]
    #[allow(clippy::approx_constant)]
    fn test_yaml_number_formats() {
        let yaml = Yaml::from_str(
            r#"
            int: 42
            negative: -17
            float: 3.14159
            scientific: 1.23e4
            hex: 0xFF
            octal: 0o77
        "#,
        )
        .unwrap();

        let json = yaml.as_json().unwrap();
        assert_eq!(json["int"], 42);
        assert_eq!(json["negative"], -17);
        assert!((json["float"].as_f64().unwrap() - 3.14159).abs() < 0.0001);
        assert_eq!(json["scientific"], 12300.0);
        assert_eq!(json["hex"], 255);
        assert_eq!(json["octal"], 63);
    }

    // ===== Policy Default Tests =====

    #[test]
    fn test_json_conversion_options_default() {
        let opts = JsonConversionOptions::default();
        assert_eq!(opts.key_policy, NonStringKeyPolicy::Stringify);
        assert_eq!(opts.float_policy, NonFiniteFloatPolicy::Null);
        assert_eq!(opts.max_depth, Some(100));
    }

    #[test]
    fn test_toml_conversion_options_default() {
        let opts = TomlConversionOptions::default();
        assert_eq!(opts.null_policy, NullPolicy::Drop);
        assert_eq!(opts.array_policy, HeteroArrayPolicy::Error);
        assert_eq!(opts.max_depth, Some(100));
    }

    // ===== Validate Method Test =====

    #[test]
    fn test_validate_returns_ok() {
        let yaml = Yaml::from_str("name: test").unwrap();
        assert!(yaml.validate().is_ok());
    }

    // ===== Multiple Warnings Test =====

    #[test]
    fn test_multiple_warnings_collected() {
        let yaml = Yaml::from_str(
            r#"
            123: first_int_key
            456: second_int_key
            789: third_int_key
        "#,
        )
        .unwrap();

        let options = JsonConversionOptions {
            key_policy: NonStringKeyPolicy::Stringify,
            ..Default::default()
        };

        let result = yaml.as_json_with(options).unwrap();
        // Should have 3 warnings for 3 non-string keys
        assert_eq!(result.warnings.len(), 3);
        assert!(
            result
                .warnings
                .iter()
                .all(|w| matches!(w, ConversionWarning::NonStringKey { .. }))
        );
    }

    #[cfg(feature = "toml")]
    #[test]
    fn test_multiple_null_warnings() {
        let yaml = Yaml::from_str(
            r#"
            a: null
            b: null
            c: value
        "#,
        )
        .unwrap();

        let options = TomlConversionOptions {
            null_policy: NullPolicy::Drop,
            ..Default::default()
        };

        let result = yaml.as_toml_with(options).unwrap();
        assert_eq!(result.warnings.len(), 2);
        assert!(
            result
                .warnings
                .iter()
                .all(|w| matches!(w, ConversionWarning::NullDropped { .. }))
        );
    }
}
