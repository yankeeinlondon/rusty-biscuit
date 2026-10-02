//! `file(match(...))` globs: completion candidates everywhere, and a
//! constraint on each declaring arm of a root union.
//!
//! On a single schema `match` only suggests files: completion and choosers
//! walk it, and validation ignores it. In a root union, conversion emits each
//! declared glob as [`DARKMATTER_MATCH_KEYWORD`] on its arm's file fragment.
//! An existing file outside the glob then rules that arm out, in arm selection
//! and in the final verdict alike.
//!
//! A value is judged only when it names an existing file. A missing file is
//! the existence check's verdict (`file(eager)`), or a lazy output that does
//! not exist yet, and a partial a chooser is about to complete must not rule
//! an arm out before the user picks a file.
//!
//! Each pattern is a [`GlobReference`] pattern (`[!][prefix]glob`) with the
//! file-name view, judged from the value's `cwd`: the document's folder for a
//! frontmatter value, the launch directory for a caller-supplied one.
//! [`FileMatchGlobs`] is that one judgment, shared with Claudine's candidate
//! walk.

use std::path::{Path, PathBuf};

use biscuit_file::{FileResolutionContext, GlobReference, GlobReferenceError};
use jsonschema::{Keyword, ValidationError, paths::Location};
use serde_json::{Map, Value};

use super::format::resolve_file_reference_in_context;
use super::simplified::{Constraint, PropertyDef, SchemaArm, SimplifiedType, TypeExpr};

/// Keyword carrying the globs a root-union arm enforces on a file value.
pub const DARKMATTER_MATCH_KEYWORD: &str = "x-darkmatter-match";

/// The patterns of one `file(match(...))` constraint, as a [`GlobReference`]
/// with the file-name view.
///
/// A path is accepted when some positive pattern admits it and no `!`
/// pattern rejects it, each judged relative to the first of its own roots
/// that contains the path. A pattern whose glob after the prefix has no `/`
/// (`*.md`, `^*.md`, `!_*.md`) also matches a file's bare name at any depth.
#[derive(Debug, Clone)]
pub struct FileMatchGlobs {
    globs: GlobReference,
}

impl FileMatchGlobs {
    /// Parse `patterns`.
    ///
    /// ## Errors
    ///
    /// The [`GlobReferenceError`] of the first pattern that is not a valid
    /// glob reference (an invalid glob, `%`, a remote URL, a malformed
    /// prefix), or [`GlobReferenceError::NoPositivePattern`].
    pub fn new(patterns: &[String]) -> Result<Self, GlobReferenceError> {
        Ok(Self {
            globs: GlobReference::new(patterns)?.with_file_name_view(),
        })
    }

    /// Whether `path` is accepted, judged without a walk; it need not exist.
    /// A relative path is read from `ctx`'s `cwd`, which is also where bare
    /// and `./` patterns start.
    #[must_use]
    pub fn matches(&self, path: &Path, ctx: &FileResolutionContext) -> bool {
        self.globs.matches(path, ctx)
    }

    /// Whether a listing that reached the existing file `path` would offer it:
    /// [`matches`](Self::matches), less a file symlink whose target leaves
    /// the tree (see [`GlobReference::lists_file`]).
    #[must_use]
    pub fn lists_file(&self, path: &Path, ctx: &FileResolutionContext) -> bool {
        self.globs.lists_file(path, ctx)
    }

    /// The positive patterns' roots in precedence order, for a caller that
    /// walks them and judges each file with [`lists_file`](Self::lists_file).
    #[must_use]
    pub fn roots(&self, ctx: &FileResolutionContext) -> Vec<PathBuf> {
        self.globs.roots(ctx)
    }
}

/// Why `patterns` cannot be a `match()` constraint: the index of the
/// offending pattern (`None` when the list as a whole is at fault) and the
/// cause. `None` when every pattern is a valid glob reference.
pub(crate) fn definition_error(patterns: &[String]) -> Option<(Option<usize>, GlobReferenceError)> {
    for (index, pattern) in patterns.iter().enumerate() {
        match GlobReference::new([pattern]) {
            Ok(_) | Err(GlobReferenceError::NoPositivePattern) => {}
            Err(error) => return Some((Some(index), error)),
        }
    }
    FileMatchGlobs::new(patterns).err().map(|error| (None, error))
}

/// The globs a simplified root-union arm enforces on top-level `property`.
/// A single schema does not use this arm-level constraint.
#[must_use]
pub fn root_union_match_patterns<'a>(arm: &'a SchemaArm, property: &str) -> Option<&'a [String]> {
    let SchemaArm::Inline(shape) = arm else {
        return None;
    };
    shape.properties.get(property).and_then(file_match_patterns)
}

pub(crate) fn file_match_patterns(definition: &PropertyDef) -> Option<&[String]> {
    let PropertyDef::Single(atom) = definition else {
        return None;
    };
    if !matches!(atom.ty, TypeExpr::Primitive(SimplifiedType::File)) {
        return None;
    }
    atom.constraints.iter().find_map(|constraint| match constraint {
        Constraint::Match(patterns) if !patterns.is_empty() => Some(patterns.as_slice()),
        _ => None,
    })
}

/// Whether a caller's file value satisfies an arm's globs, resolved and
/// judged from the caller's own `origin`.
///
/// The same judgment the [`DARKMATTER_MATCH_KEYWORD`] validator makes: a value
/// that names no existing file, or still holds template or shell syntax, is
/// not judged. Patterns that are not valid glob references admit every value,
/// as they offer none; the schema definition check reports them.
#[must_use]
pub fn file_match_admits(value: &str, patterns: &[String], origin: &FileResolutionContext) -> bool {
    FileMatchGlobs::new(patterns).map_or(true, |globs| admits(&globs, value, None, None, origin))
}

/// Resolution matches the `darkmatter-file` format's: from `base_dir`, else
/// the context's `cwd`. That directory is also the value's `cwd` for the
/// globs.
fn admits(
    globs: &FileMatchGlobs,
    value: &str,
    base_dir: Option<&Path>,
    fallback: Option<&Path>,
    context: &FileResolutionContext,
) -> bool {
    if crate::markdown::literal_token::holds_pending_syntax(value) {
        return true;
    }
    let Ok(path) = resolve_file_reference_in_context(value, base_dir, fallback, context) else {
        return true;
    };
    let value_context = match base_dir {
        Some(base_dir) => {
            crate::markdown::compose::expression::resolve_ctx::document_file_context(base_dir, context)
        }
        None => context.clone(),
    };
    globs.matches(&path, &value_context)
}

type KeywordResult<'a> = Result<Box<dyn for<'i> Keyword<'i>>, ValidationError<'a>>;

/// Factory for [`DARKMATTER_MATCH_KEYWORD`], judging values in the
/// validator's context (see [`admits`]); a property a caller supplied is
/// judged in the context it was authored in. A structural validator judges
/// only an absolute path naming an existing file, by its full path
/// ([`GlobReference::matches_without_context`]), and admits every value whose
/// meaning needs a context. Root-union coercion relies on that to pick an
/// arm by glob.
pub(crate) fn match_keyword_factory(
    file_values: super::validate::FileValues,
) -> impl for<'a> Fn(&'a Map<String, Value>, &'a Value, Location) -> KeywordResult<'a>
+ Send
+ Sync
+ 'static {
    move |_parent, schema, location| {
        let patterns: Vec<String> = schema
            .as_array()
            .and_then(|items| items.iter().map(|item| item.as_str().map(String::from)).collect())
            .ok_or_else(|| {
                ValidationError::schema(format!(
                    "{DARKMATTER_MATCH_KEYWORD} must be an array of strings"
                ))
            })?;
        let globs = FileMatchGlobs::new(&patterns).map_err(|error| {
            ValidationError::schema(format!("{DARKMATTER_MATCH_KEYWORD} holds an invalid glob: {error}"))
        })?;
        Ok(Box::new(MatchKeyword {
            globs,
            patterns,
            judged_in: JudgedIn::for_keyword(&file_values, &location),
        }))
    }
}

/// Where a [`MatchKeyword`] judges its values.
enum JudgedIn {
    /// No context: only an existing absolute path is judged.
    Syntax,
    /// Resolved like the `darkmatter-file` format, from `base_dir` (else the
    /// context's `cwd`), and judged from that directory.
    Resolved {
        base_dir: Option<PathBuf>,
        fallback: Option<PathBuf>,
        context: Box<FileResolutionContext>,
    },
}

impl JudgedIn {
    /// A caller-supplied property is judged in the context it was authored
    /// in; its value is already projected from there.
    fn for_keyword(file_values: &super::validate::FileValues, location: &Location) -> Self {
        match file_values {
            super::validate::FileValues::Syntax => Self::Syntax,
            super::validate::FileValues::Resolved { base_dir, fallback, context, callers } => {
                let origin = top_level_property(location)
                    .map_or(super::validate::ValueOrigin::Document, |property| callers.origin_of(&property));
                match origin {
                    super::validate::ValueOrigin::Caller(origin) => Self::Resolved {
                        base_dir: None,
                        fallback: None,
                        context: Box::new(origin.clone()),
                    },
                    super::validate::ValueOrigin::Document => Self::Resolved {
                        base_dir: base_dir.clone(),
                        fallback: fallback.clone(),
                        context: context.clone(),
                    },
                }
            }
        }
    }
}

/// The top-level property a keyword's schema location belongs to: the
/// segment after the location's first `properties`. `match` is emitted only
/// on top-level root-union properties (see [`root_union_match_patterns`]).
fn top_level_property(location: &Location) -> Option<String> {
    let pointer = location.as_str();
    let mut segments = pointer.split('/').skip(1);
    segments.by_ref().find(|segment| *segment == "properties")?;
    segments
        .next()
        .map(|segment| segment.replace("~1", "/").replace("~0", "~"))
}

struct MatchKeyword {
    globs: FileMatchGlobs,
    patterns: Vec<String>,
    judged_in: JudgedIn,
}

impl MatchKeyword {
    fn check(&self, value: &str) -> bool {
        match &self.judged_in {
            JudgedIn::Syntax => super::format::context_free_path(value)
                .filter(|path| path.exists())
                .is_none_or(|path| self.globs.globs.matches_without_context(&path)),
            JudgedIn::Resolved { base_dir, fallback, context } => {
                admits(&self.globs, value, base_dir.as_deref(), fallback.as_deref(), context)
            }
        }
    }
}

impl<'i> Keyword<'i> for MatchKeyword {
    fn validate(&self, instance: &'i Value) -> Result<(), ValidationError<'i>> {
        match instance {
            Value::String(value) if !self.check(value) => Err(ValidationError::custom(format!(
                "`{value}` is outside this schema arm's `match({})`",
                self.patterns.join(", ")
            ))),
            _ => Ok(()),
        }
    }

    fn is_valid(&self, instance: &'i Value) -> bool {
        match instance {
            Value::String(value) => self.check(value),
            _ => true,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::markdown::schemas::simplified::parse_yaml_schema;

    fn union(yaml: &str) -> Vec<SchemaArm> {
        let value: serde_yaml_ng::Value = serde_yaml_ng::from_str(yaml).unwrap();
        match parse_yaml_schema(&value).unwrap() {
            crate::markdown::schemas::SimplifiedSchema::Union(arms) => arms,
            other => panic!("expected a union, got {other:?}"),
        }
    }

    #[test]
    fn every_declared_root_union_glob_constrains_its_arm() {
        let arms = union(
            "- {kind: 'literal(feature)', spec: 'file(eager;match(**/features/**/spec.md))', plan: 'file(match(**/*plan*.md))', solo: 'file(match(*.md))'}\n\
             - {kind: 'literal(fix)', spec: 'file(eager;match(**/fixes/**/spec.md))', plan: 'file(match(**/*plan*.md))'}\n",
        );
        assert_eq!(
            root_union_match_patterns(&arms[0], "spec"),
            Some(&["**/features/**/spec.md".to_string()][..])
        );
        assert_eq!(
            root_union_match_patterns(&arms[1], "spec"),
            Some(&["**/fixes/**/spec.md".to_string()][..])
        );
        assert_eq!(root_union_match_patterns(&arms[0], "plan"), Some(&["**/*plan*.md".to_string()][..]));
        assert_eq!(root_union_match_patterns(&arms[0], "solo"), Some(&["*.md".to_string()][..]));
        assert_eq!(root_union_match_patterns(&arms[1], "solo"), None);
        assert_eq!(root_union_match_patterns(&arms[0], "kind"), None);
    }

    #[test]
    fn conversion_emits_every_root_union_glob() {
        let value: serde_yaml_ng::Value = serde_yaml_ng::from_str(
            "- {spec: 'file(eager;match(**/features/**/spec.md))', plan: 'file(match(*.md))[]'}\n\
             - {spec: 'file(eager;match(**/fixes/**/spec.md))', plan: 'file(match(*.md))[]'}\n",
        )
        .unwrap();
        let json =
            crate::markdown::schemas::to_json_schema(&parse_yaml_schema(&value).unwrap()).unwrap();
        let spec = &json["anyOf"][1]["properties"]["spec"];
        let file_arm = spec["anyOf"]
            .as_array()
            .and_then(|arms| arms.iter().find(|arm| arm.get("format").is_some()))
            .unwrap_or(spec);
        assert_eq!(file_arm[DARKMATTER_MATCH_KEYWORD], serde_json::json!(["**/fixes/**/spec.md"]));
        for arm in &json["anyOf"].as_array().unwrap()[..2] {
            let plan = &arm["properties"]["plan"];
            assert!(
                plan.to_string().contains("\"x-darkmatter-match\":[\"*.md\"]"),
                "the identical array glob must constrain each arm: {plan}"
            );
        }
        let single: serde_yaml_ng::Value =
            serde_yaml_ng::from_str("{spec: 'file(eager;match(**/fixes/**/spec.md))'}").unwrap();
        let single =
            crate::markdown::schemas::to_json_schema(&parse_yaml_schema(&single).unwrap()).unwrap();
        assert!(!single.to_string().contains(DARKMATTER_MATCH_KEYWORD));
    }

    #[test]
    fn a_glob_against_an_unglobbed_declaration_constrains_its_arm() {
        let arms = union(
            "- {spec: 'file(match(**/fixes/**/spec.md))'}\n\
             - {spec: 'string'}\n",
        );
        assert!(root_union_match_patterns(&arms[0], "spec").is_some());
        assert_eq!(root_union_match_patterns(&arms[1], "spec"), None);
    }

    #[test]
    fn globs_judge_a_path_relative_to_the_value_cwd() {
        let globs = FileMatchGlobs::new(&["fixes/**/spec.md".to_string()]).unwrap();
        let launch = tempfile::tempdir().unwrap();
        let context = FileResolutionContext::new(launch.path());
        assert!(globs.matches(Path::new("fixes/x/spec.md"), &context));
        assert!(globs.matches(&launch.path().join("fixes").join("x").join("spec.md"), &context));
        assert!(!globs.matches(Path::new("features/x/spec.md"), &context));
    }

    #[test]
    fn a_missing_file_is_never_judged() {
        let dir = tempfile::tempdir().unwrap();
        let origin = FileResolutionContext::new(dir.path());
        let patterns = ["**/fixes/**/spec.md".to_string()];
        assert!(file_match_admits("partial", &patterns, &origin));
        std::fs::create_dir_all(dir.path().join("features/x")).unwrap();
        std::fs::write(dir.path().join("features/x/spec.md"), "").unwrap();
        assert!(!file_match_admits("features/x/spec.md", &patterns, &origin));
        std::fs::create_dir_all(dir.path().join("fixes/x")).unwrap();
        std::fs::write(dir.path().join("fixes/x/spec.md"), "").unwrap();
        assert!(file_match_admits("fixes/x/spec.md", &patterns, &origin));
    }
}
