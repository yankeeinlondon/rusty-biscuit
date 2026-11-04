//! The file factories and the one-argument input they take.
//!
//! A [`SchemaSource`] names a file as a raw path, a raw reference string, or
//! a parsed [`FileReference`]. On its own it is *raw*: the factory captures a
//! new request context once at entry. [`SchemaSource::bound`] attaches a
//! context the caller already captured, and the factory captures nothing.
//! [`SchemaSource::with_text`] supplies the file's current text (an editor's
//! unsaved buffer), so the factory resolves the path for its origin but never
//! reads the file.

use std::path::{Path, PathBuf};

use biscuit_file::{FileReference, FileReferenceKind, FileResolutionContext};

use super::layer::{Binding, Layer};
use super::{SimplifiedSchema, SimplifiedSchemaError, capture};
use crate::SchemaError;

/// The one argument of a file factory: which file, and optionally the
/// request context it resolves in and its current text.
///
/// Converts from `&Path`, `PathBuf`, `&str` and `String` (a file reference,
/// parsed at factory entry), and `FileReference`.
#[derive(Debug, Clone)]
pub struct SchemaSource {
    target: Target,
    context: Option<FileResolutionContext>,
    text: Option<String>,
}

#[derive(Debug, Clone)]
enum Target {
    Path(PathBuf),
    Raw(String),
    Reference(FileReference),
}

impl SchemaSource {
    /// Binds the source to a captured request context: relative paths,
    /// references, imports, and file values resolve in `context`, and the
    /// factory captures nothing.
    #[must_use]
    pub fn bound(mut self, context: FileResolutionContext) -> Self {
        self.context = Some(context);
        self
    }

    /// Supplies the file's text, which the factory uses instead of reading
    /// the file. The path still names the schema's origin and anchors its
    /// relative references.
    #[must_use]
    pub fn with_text(mut self, text: impl Into<String>) -> Self {
        self.text = Some(text.into());
        self
    }

    fn of(target: Target) -> Self {
        Self { target, context: None, text: None }
    }

    /// The context, captured now for a raw source.
    fn context(&mut self) -> Result<FileResolutionContext, SimplifiedSchemaError> {
        match self.context.take() {
            Some(context) => Ok(context),
            None => capture::request_context(),
        }
    }
}

impl From<&Path> for SchemaSource {
    fn from(path: &Path) -> Self {
        Self::of(Target::Path(path.to_path_buf()))
    }
}

impl From<PathBuf> for SchemaSource {
    fn from(path: PathBuf) -> Self {
        Self::of(Target::Path(path))
    }
}

impl From<&PathBuf> for SchemaSource {
    fn from(path: &PathBuf) -> Self {
        Self::of(Target::Path(path.clone()))
    }
}

impl From<&str> for SchemaSource {
    fn from(reference: &str) -> Self {
        Self::of(Target::Raw(reference.to_string()))
    }
}

impl From<String> for SchemaSource {
    fn from(reference: String) -> Self {
        Self::of(Target::Raw(reference))
    }
}

impl From<FileReference> for SchemaSource {
    fn from(reference: FileReference) -> Self {
        Self::of(Target::Reference(reference))
    }
}

impl From<&FileReference> for SchemaSource {
    fn from(reference: &FileReference) -> Self {
        Self::of(Target::Reference(reference.clone()))
    }
}

/// Which loader reads the file.
#[derive(Clone, Copy)]
enum Format {
    Markdown,
    Yaml,
    JsonSchema,
}

impl Format {
    /// By extension, for [`SimplifiedSchema::resolve`]: `.md`/`.markdown` is
    /// Markdown, `.json` raw JSON Schema, anything else a YAML schema
    /// document. This is the resolver's own rule for referenced files.
    fn of_path(path: &Path) -> Self {
        match path.extension().and_then(|extension| extension.to_str()).map(str::to_ascii_lowercase).as_deref() {
            Some("md" | "markdown") => Self::Markdown,
            Some("json") => Self::JsonSchema,
            _ => Self::Yaml,
        }
    }
}

impl SimplifiedSchema {
    /// The schema in a Markdown file's frontmatter `$schema`. The body is
    /// never read as schema.
    ///
    /// A raw source captures a new request context once, here; a bound one
    /// reuses its context. The file is read once (not at all when the source
    /// carries its text), and its relative references resolve from its
    /// directory.
    ///
    /// ## Errors
    ///
    /// [`SimplifiedSchemaError::MissingFile`],
    /// [`SimplifiedSchemaError::Read`], or
    /// [`SimplifiedSchemaError::Reference`] when the file cannot be found or
    /// read; [`SimplifiedSchemaError::MissingSchema`] when it has no
    /// frontmatter or no `$schema`; [`SimplifiedSchemaError::Frontmatter`]
    /// and [`SimplifiedSchemaError::Schema`] for malformed input.
    pub fn try_from_markdown(source: impl Into<SchemaSource>) -> Result<Self, SimplifiedSchemaError> {
        Self::load(source.into(), Some(Format::Markdown))
    }

    /// A standalone schema YAML file (root `$schema`, or `kind: schema` with
    /// `types`), or a raw JSON Schema written in YAML.
    ///
    /// ## Errors
    ///
    /// As [`Self::try_from_markdown`]; a YAML document with no schema-bearing
    /// structure is [`SimplifiedSchemaError::MissingSchema`].
    pub fn try_from_yaml(source: impl Into<SchemaSource>) -> Result<Self, SimplifiedSchemaError> {
        Self::load(source.into(), Some(Format::Yaml))
    }

    /// A raw JSON Schema file, through the resolver's JSON loader (strict
    /// JSON, admitted numbers, an object at the root).
    ///
    /// ## Errors
    ///
    /// As [`Self::try_from_markdown`]; JSON that is not an object schema is
    /// [`SimplifiedSchemaError::Schema`].
    pub fn try_from_json_schema(source: impl Into<SchemaSource>) -> Result<Self, SimplifiedSchemaError> {
        Self::load(source.into(), Some(Format::JsonSchema))
    }

    /// The schema a local reference names, read by its extension (see
    /// [`Self::try_from_markdown`], [`Self::try_from_yaml`],
    /// [`Self::try_from_json_schema`]).
    ///
    /// Asynchronous for callers that resolve schemas inside async code; the
    /// context is captured before the first suspension point. Only local
    /// references and supplied sources load: an `http(s)://` reference is
    /// [`SchemaError::RemoteUnsupported`], as everywhere in schema loading.
    ///
    /// ## Errors
    ///
    /// As the factory the extension selects.
    pub async fn resolve(uri: impl Into<SchemaSource>) -> Result<Self, SimplifiedSchemaError> {
        Self::load(uri.into(), None)
    }

    /// Captures (raw) or reuses (bound) the context, resolves the target,
    /// takes the text once, and builds the schema from that one buffer.
    fn load(mut source: SchemaSource, format: Option<Format>) -> Result<Self, SimplifiedSchemaError> {
        // A raw reference is parsed, and a remote one refused, before
        // anything is captured or read.
        if let Target::Raw(raw) = &source.target {
            let reference = FileReference::new(raw.trim())
                .map_err(|error| SimplifiedSchemaError::Reference { reference: raw.clone(), source: error })?;
            source.target = Target::Reference(reference);
        }
        if let Target::Reference(reference) = &source.target
            && reference.class().kind == FileReferenceKind::Url
        {
            return Err(SchemaError::RemoteUnsupported { reference: reference.raw().to_string() }.into());
        }
        let context = source.context()?;
        let path = resolve_target(&source.target, &context, source.text.is_some())?;
        let text = match source.text.take() {
            Some(text) => text,
            None => read(&path)?,
        };
        let binding = Binding {
            base_dir: path.parent().map_or_else(|| context.cwd().to_path_buf(), Path::to_path_buf),
            context,
        };
        let layer = match format.unwrap_or_else(|| Format::of_path(&path)) {
            Format::Markdown => Layer::from_markdown(&text, Some(path), Some(binding))?,
            Format::Yaml => Layer::from_yaml_document(&text, Some(path), Some(binding))?,
            Format::JsonSchema => Layer::from_json_schema_bytes(text.as_bytes(), path, Some(binding))?,
        };
        Ok(Self::from_layer(layer))
    }
}

/// The file `target` names in `context`. A path is taken as written (joined
/// to the context's directory when relative); a reference resolves through
/// the context's candidate order. With supplied text the file need not
/// exist, so a reference is resolved only lexically there.
fn resolve_target(
    target: &Target,
    context: &FileResolutionContext,
    has_text: bool,
) -> Result<PathBuf, SimplifiedSchemaError> {
    match target {
        Target::Path(path) => Ok(if path.is_absolute() { path.clone() } else { context.cwd().join(path) }),
        Target::Raw(_) => unreachable!("`load` parses a raw reference at entry"),
        Target::Reference(reference) => match reference.resolve_in_context(context) {
            Ok(Some(path)) => Ok(path),
            Ok(None) if has_text => Ok(context.cwd().join(reference.raw())),
            Ok(None) => Err(SimplifiedSchemaError::MissingFile { path: PathBuf::from(reference.raw()) }),
            Err(source) => Err(SimplifiedSchemaError::Reference { reference: reference.raw().to_string(), source }),
        },
    }
}

/// The file's text, read once.
fn read(path: &Path) -> Result<String, SimplifiedSchemaError> {
    #[cfg(test)]
    READS.with(|count| count.set(count.get() + 1));
    std::fs::read_to_string(path).map_err(|source| match source.kind() {
        std::io::ErrorKind::NotFound => SimplifiedSchemaError::MissingFile { path: path.to_path_buf() },
        _ => SimplifiedSchemaError::Read { path: path.to_path_buf(), source },
    })
}

#[cfg(test)]
thread_local! {
    /// Schema-file reads made on this thread, for the read-once tests.
    static READS: std::cell::Cell<usize> = const { std::cell::Cell::new(0) };
}

/// Schema-file reads made on this thread so far.
#[cfg(test)]
pub(super) fn reads() -> usize {
    READS.with(std::cell::Cell::get)
}
