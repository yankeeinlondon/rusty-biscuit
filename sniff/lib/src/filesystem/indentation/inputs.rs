//! Reading the configuration files an indentation lookup needs.
//!
//! Each distinct path is probed, read, and parsed at most once per load,
//! whichever rows share it (`package.json`, `pyproject.toml`). Independent
//! files load in parallel; results are keyed by path, so the order work
//! finishes in never shows in a result.

use std::collections::{HashMap, HashSet};
use std::path::Path;

use rayon::prelude::*;

use super::catalog::{self, Dialect, EDITORCONFIG, ROWS};
use crate::filesystem::formatting::{EditorConfigScan, scan_editorconfig};
use crate::performance::{self, counters};

/// Which candidates a load reads.
#[derive(Debug, Clone, Copy)]
pub(super) enum Selection<'a> {
    /// EditorConfig plus the rows for one lowercase file type (none for
    /// `None`).
    Target(Option<&'a str>),
    /// Every supported file, for a reusable snapshot.
    All,
}

/// A format-neutral view of a parsed configuration value that keeps each
/// value's native type, so `"2"` and `2` stay different.
#[derive(Debug, Clone, PartialEq)]
pub(super) enum Node {
    Null,
    Bool(bool),
    Int(i128),
    Float(f64),
    String(String),
    List(Vec<Node>),
    /// String-keyed entries in document order. Duplicate keys were rejected
    /// by the loader; non-string keys are dropped, since no setting has one.
    Map(Vec<(String, Node)>),
    /// A YAML tagged value or a TOML date-time, described for diagnostics.
    Other(&'static str),
}

impl Node {
    pub(super) fn as_map(&self) -> Option<&[(String, Node)]> {
        match self {
            Node::Map(entries) => Some(entries),
            _ => None,
        }
    }

    pub(super) fn get(&self, key: &str) -> Option<&Node> {
        self.as_map()?.iter().find(|(name, _)| name == key).map(|(_, value)| value)
    }

    /// How diagnostics describe the value's type.
    pub(super) fn describe(&self) -> String {
        match self {
            Node::Null => "null".to_string(),
            Node::Bool(value) => format!("the boolean `{value}`"),
            Node::Int(value) => format!("the integer `{value}`"),
            Node::Float(value) => format!("the number `{value}`"),
            Node::String(value) => format!("the string `{value:?}`"),
            Node::List(_) => "a list".to_string(),
            Node::Map(_) => "a mapping".to_string(),
            Node::Other(kind) => (*kind).to_string(),
        }
    }
}

/// One candidate path after loading.
#[derive(Debug)]
pub(super) enum Loaded {
    Absent,
    /// The file exists but could not be read; the reason, ready to report.
    Unreadable(String),
    /// The file was read but is not valid in its dialect.
    Unparseable(String),
    EditorConfig(EditorConfigScan),
    Document(Node),
    /// Every non-empty document of a YAML stream.
    Documents(Vec<Node>),
}

#[derive(Debug, Default)]
pub(crate) struct Inputs {
    loaded: HashMap<&'static str, Loaded>,
    executables: HashSet<&'static str>,
}

#[derive(Debug, Clone, Copy)]
enum Job {
    EditorConfig,
    Parse(Dialect),
    /// An executable configuration: its existence is all Sniff reports.
    Probe,
}

impl Inputs {
    pub(super) fn load(root: &Path, selection: Selection<'_>) -> Self {
        let jobs = jobs(selection);
        let collector = performance::current_collector();
        let results: Vec<(&'static str, Job, Loaded)> = jobs
            .par_iter()
            .map(|(path, job)| {
                let _worker = performance::pooled_worker(collector.as_ref());
                (*path, *job, load_one(&root.join(path), *job))
            })
            .collect();
        let mut inputs = Inputs::default();
        for (path, job, loaded) in results {
            match job {
                Job::Probe => {
                    if !matches!(loaded, Loaded::Absent) {
                        inputs.executables.insert(path);
                    }
                }
                Job::EditorConfig | Job::Parse(_) => {
                    inputs.loaded.insert(path, loaded);
                }
            }
        }
        inputs
    }

    pub(super) fn get(&self, path: &str) -> &Loaded {
        self.loaded.get(path).unwrap_or(&Loaded::Absent)
    }

    pub(super) fn editorconfig(&self) -> Option<&EditorConfigScan> {
        match self.get(EDITORCONFIG) {
            Loaded::EditorConfig(scan) => Some(scan),
            _ => None,
        }
    }

    /// Why an existing file could not be used, if it could not.
    pub(super) fn failure(&self, path: &str) -> Option<&str> {
        match self.get(path) {
            Loaded::Unreadable(reason) | Loaded::Unparseable(reason) => Some(reason),
            _ => None,
        }
    }

    pub(super) fn executable_exists(&self, path: &str) -> bool {
        self.executables.contains(path)
    }
}

fn jobs(selection: Selection<'_>) -> Vec<(&'static str, Job)> {
    let mut jobs: Vec<(&'static str, Job)> = vec![(EDITORCONFIG, Job::EditorConfig)];
    let rows: Vec<_> = match selection {
        Selection::All => ROWS.iter().collect(),
        Selection::Target(None) => Vec::new(),
        Selection::Target(Some(file_type)) => ROWS.iter().filter(|row| row.applies_to(file_type)).collect(),
    };
    for row in rows {
        for candidate in row.candidates {
            if !jobs.iter().any(|(path, _)| *path == candidate.path) {
                jobs.push((candidate.path, Job::Parse(candidate.dialect)));
            }
        }
        for executable in row.executable {
            jobs.push((executable, Job::Probe));
        }
    }
    debug_assert!(
        matches!(selection, Selection::Target(_))
            || catalog::all_candidate_paths().iter().all(|(path, _)| jobs.iter().any(|(job, _)| job == path))
    );
    jobs
}

fn load_one(path: &Path, job: Job) -> Loaded {
    performance::increment_counter(counters::FS_METADATA_PROBES, 1);
    match std::fs::metadata(path) {
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Loaded::Absent,
        Err(error) => return Loaded::Unreadable(format!("could not be read: {error}")),
        Ok(metadata) if metadata.is_dir() => return Loaded::Unreadable("is a directory, not a file".to_string()),
        Ok(_) => {}
    }
    let Job::Parse(dialect) = job else {
        if matches!(job, Job::Probe) {
            return Loaded::Document(Node::Null);
        }
        return match read(path) {
            Ok(text) => {
                performance::increment_counter(counters::INDENT_CONFIG_PARSES, 1);
                Loaded::EditorConfig(scan_editorconfig(&text))
            }
            Err(reason) => Loaded::Unreadable(reason),
        };
    };
    let text = match read(path) {
        Ok(text) => text,
        Err(reason) => return Loaded::Unreadable(reason),
    };
    performance::increment_counter(counters::INDENT_CONFIG_PARSES, 1);
    match parse(&text, dialect) {
        Ok(loaded) => loaded,
        Err(reason) => Loaded::Unparseable(reason),
    }
}

fn read(path: &Path) -> Result<String, String> {
    performance::increment_counter(counters::FS_FILE_OPENS, 1);
    let bytes = std::fs::read(path).map_err(|error| format!("could not be read: {error}"))?;
    performance::increment_counter(counters::FS_BYTES_READ, bytes.len() as u64);
    let text = String::from_utf8(bytes).map_err(|_| "could not be read: it is not UTF-8 text".to_string())?;
    Ok(match text.strip_prefix('\u{feff}') {
        Some(rest) => rest.to_string(),
        None => text,
    })
}

/// Parses `text` in `dialect`. Exposed to tests so each loader's document
/// shapes are proved without a file.
pub(super) fn parse(text: &str, dialect: Dialect) -> Result<Loaded, String> {
    match dialect {
        Dialect::Json => parse_json(text).map(Loaded::Document),
        Dialect::Json5 => parse_json5(text).map(Loaded::Document),
        Dialect::Toml => parse_toml(text).map(Loaded::Document),
        Dialect::Yaml => parse_yaml(text).map(Loaded::Document),
        Dialect::YamlDocuments => parse_yaml_documents(text).map(Loaded::Documents),
        Dialect::JsonOrYaml => {
            if text.trim_start().starts_with(['{', '[']) {
                parse_json(text).map(Loaded::Document)
            } else {
                parse_yaml(text).map(Loaded::Document)
            }
        }
    }
}

/// Strict JSON. Grammar is checked without building a value, then duplicate
/// keys are rejected before `serde_json` would let the last one win.
fn parse_json(text: &str) -> Result<Node, String> {
    serde_json::from_str::<serde::de::IgnoredAny>(text).map_err(|error| format!("is not valid JSON: {error}"))?;
    reject_duplicate_keys(text)?;
    let value: serde_json::Value =
        serde_json::from_str(text).map_err(|error| format!("is not valid JSON: {error}"))?;
    Ok(from_json(&value))
}

fn parse_json5(text: &str) -> Result<Node, String> {
    if !biscuit_file::json5_is_well_formed(text) {
        let detail = biscuit_file::Json5::from_str(text).err().map(|error| error.to_string()).unwrap_or_default();
        return Err(format!("is not valid JSON5: {detail}"));
    }
    reject_duplicate_keys(text)?;
    let parsed = biscuit_file::Json5::from_str(text).map_err(|error| format!("is not valid JSON5: {error}"))?;
    Ok(from_json(parsed.value()))
}

fn reject_duplicate_keys(text: &str) -> Result<(), String> {
    let scan = biscuit_file::json5_scan(text).map_err(|error| format!("could not be scanned: {error}"))?;
    match scan.duplicate_key {
        Some(duplicate) => {
            let (line, column) = line_column(text, duplicate.offset);
            Err(format!("repeats the key `{}` at line {line} column {column}", duplicate.key))
        }
        None => Ok(()),
    }
}

fn line_column(text: &str, offset: usize) -> (usize, usize) {
    let before = &text[..offset.min(text.len())];
    let line = before.matches('\n').count() + 1;
    let column = before.rsplit('\n').next().map_or(0, |tail| tail.chars().count()) + 1;
    (line, column)
}

fn parse_toml(text: &str) -> Result<Node, String> {
    let table: biscuit_file::toml_crate::Table =
        biscuit_file::toml_crate::from_str(text).map_err(|error| format!("is not valid TOML: {}", error.to_string().trim_end()))?;
    Ok(from_toml(&biscuit_file::toml_crate::Value::Table(table)))
}

/// One YAML document. An empty file (or one holding only comments) is an
/// empty mapping: it states no settings.
fn parse_yaml(text: &str) -> Result<Node, String> {
    let value: biscuit_file::YamlValue = biscuit_file::serde_yaml_ng::from_str(text)
        .map_err(|error| format!("is not valid YAML: {error}"))?;
    Ok(match value {
        biscuit_file::YamlValue::Null if is_blank_yaml(text) => Node::Map(Vec::new()),
        value => from_yaml(&value),
    })
}

fn is_blank_yaml(text: &str) -> bool {
    text.lines().all(|line| {
        let line = line.trim();
        line.is_empty() || line.starts_with('#') || line == "---" || line == "..."
    })
}

fn parse_yaml_documents(text: &str) -> Result<Vec<Node>, String> {
    use serde::Deserialize;

    let mut documents = Vec::new();
    for document in biscuit_file::serde_yaml_ng::Deserializer::from_str(text) {
        let value = biscuit_file::YamlValue::deserialize(document).map_err(|error| format!("is not valid YAML: {error}"))?;
        if !matches!(value, biscuit_file::YamlValue::Null) {
            documents.push(from_yaml(&value));
        }
    }
    Ok(documents)
}

fn from_json(value: &serde_json::Value) -> Node {
    match value {
        serde_json::Value::Null => Node::Null,
        serde_json::Value::Bool(value) => Node::Bool(*value),
        serde_json::Value::Number(number) => match (number.as_i64(), number.as_u64()) {
            (Some(value), _) => Node::Int(i128::from(value)),
            (None, Some(value)) => Node::Int(i128::from(value)),
            _ => Node::Float(number.as_f64().unwrap_or(f64::NAN)),
        },
        serde_json::Value::String(value) => Node::String(value.clone()),
        serde_json::Value::Array(items) => Node::List(items.iter().map(from_json).collect()),
        serde_json::Value::Object(entries) => {
            Node::Map(entries.iter().map(|(key, value)| (key.clone(), from_json(value))).collect())
        }
    }
}

fn from_toml(value: &biscuit_file::toml_crate::Value) -> Node {
    use biscuit_file::toml_crate::Value;
    match value {
        Value::String(value) => Node::String(value.clone()),
        Value::Integer(value) => Node::Int(i128::from(*value)),
        Value::Float(value) => Node::Float(*value),
        Value::Boolean(value) => Node::Bool(*value),
        Value::Datetime(_) => Node::Other("a date-time"),
        Value::Array(items) => Node::List(items.iter().map(from_toml).collect()),
        Value::Table(entries) => Node::Map(entries.iter().map(|(key, value)| (key.clone(), from_toml(value))).collect()),
    }
}

fn from_yaml(value: &biscuit_file::YamlValue) -> Node {
    use biscuit_file::YamlValue as Value;
    match value {
        Value::Null => Node::Null,
        Value::Bool(value) => Node::Bool(*value),
        Value::Number(number) => match (number.as_i64(), number.as_u64()) {
            (Some(value), _) => Node::Int(i128::from(value)),
            (None, Some(value)) => Node::Int(i128::from(value)),
            _ => Node::Float(number.as_f64().unwrap_or(f64::NAN)),
        },
        Value::String(value) => Node::String(value.clone()),
        Value::Sequence(items) => Node::List(items.iter().map(from_yaml).collect()),
        Value::Mapping(entries) => Node::Map(
            entries
                .iter()
                .filter_map(|(key, value)| key.as_str().map(|key| (key.to_string(), from_yaml(value))))
                .collect(),
        ),
        Value::Tagged(_) => Node::Other("a tagged value"),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn document(text: &str, dialect: Dialect) -> Result<Node, String> {
        match parse(text, dialect)? {
            Loaded::Document(node) => Ok(node),
            other => panic!("expected one document, got {other:?}"),
        }
    }

    #[test]
    fn strict_json_rejects_comments_trailing_commas_duplicates_and_trailing_content() {
        assert!(document(r#"{"a": 1}"#, Dialect::Json).is_ok());
        assert!(document("{\"a\": 1, // c\n}", Dialect::Json).unwrap_err().contains("not valid JSON"));
        assert!(document(r#"{"a": 1,}"#, Dialect::Json).unwrap_err().contains("not valid JSON"));
        let duplicate = document("{\n  \"a\": 1,\n  \"a\": 2\n}", Dialect::Json).unwrap_err();
        assert_eq!(duplicate, "repeats the key `a` at line 3 column 3");
        assert!(document(r#"{"a": 1} trailing"#, Dialect::Json).unwrap_err().contains("not valid JSON"));
    }

    #[test]
    fn json5_accepts_comments_and_trailing_commas_and_rejects_duplicates_and_garbage() {
        let node = document("// lead\n{a: 1, /* x */ b: [2,],}", Dialect::Json5).unwrap();
        assert_eq!(node.get("a"), Some(&Node::Int(1)));
        assert_eq!(node.get("b"), Some(&Node::List(vec![Node::Int(2)])));
        assert!(document("{a: 1, a: 2}", Dialect::Json5).unwrap_err().starts_with("repeats the key `a`"));
        assert!(document("{a: 1} }", Dialect::Json5).unwrap_err().contains("not valid JSON5"));
    }

    #[test]
    fn toml_rejects_duplicates_and_trailing_content_and_keeps_types() {
        let node = document("a = 2\nb = \"2\"\n", Dialect::Toml).unwrap();
        assert_eq!(node.get("a"), Some(&Node::Int(2)));
        assert_eq!(node.get("b"), Some(&Node::String("2".to_string())));
        assert!(document("a = 1\na = 2\n", Dialect::Toml).unwrap_err().contains("not valid TOML"));
        assert!(document("a = 1 garbage\n", Dialect::Toml).unwrap_err().contains("not valid TOML"));
    }

    #[test]
    fn yaml_reads_implicit_null_and_rejects_duplicates_second_documents_and_garbage() {
        let node = document("a:\nb: 2\n", Dialect::Yaml).unwrap();
        assert_eq!(node.get("a"), Some(&Node::Null));
        assert_eq!(node.get("b"), Some(&Node::Int(2)));
        assert!(document("a: 1\na: 2\n", Dialect::Yaml).unwrap_err().contains("not valid YAML"));
        assert!(document("a: 1\n---\nb: 2\n", Dialect::Yaml).unwrap_err().contains("not valid YAML"));
        assert!(document("a: 1\nb: [\n", Dialect::Yaml).unwrap_err().contains("not valid YAML"));
        assert_eq!(document("# only a comment\n", Dialect::Yaml).unwrap(), Node::Map(Vec::new()));
        assert_eq!(document("~\n", Dialect::Yaml).unwrap(), Node::Null);
    }

    #[test]
    fn a_yaml_stream_keeps_every_document_in_order() {
        let Loaded::Documents(documents) =
            parse("---\nIndentWidth: 4\n---\nLanguage: Cpp\nIndentWidth: 8\n...\n", Dialect::YamlDocuments).unwrap()
        else {
            panic!("expected documents");
        };
        assert_eq!(documents.len(), 2);
        assert_eq!(documents[1].get("Language"), Some(&Node::String("Cpp".to_string())));
        assert!(parse("a: 1\na: 2\n", Dialect::YamlDocuments).is_err());
    }

    #[test]
    fn an_extensionless_file_is_json_when_it_opens_a_collection_and_yaml_otherwise() {
        assert_eq!(document("  {\"useTabs\": true}", Dialect::JsonOrYaml).unwrap().get("useTabs"), Some(&Node::Bool(true)));
        assert_eq!(document("useTabs: true\n", Dialect::JsonOrYaml).unwrap().get("useTabs"), Some(&Node::Bool(true)));
        // A JSON-looking file is held to strict JSON, not read as YAML flow.
        assert!(document("{useTabs: true}", Dialect::JsonOrYaml).unwrap_err().contains("not valid JSON"));
    }
}
