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
//! The glob is compared the way candidate discovery walks it: in portable
//! (`/`-separated) spelling, relative to the launch directory, so a file a
//! chooser offers is a file the arm accepts. [`FileMatchGlobs`] is that one
//! comparison, shared with Claudine's candidate walk.

use std::path::{Component, Path, PathBuf};

use biscuit_file::FileResolutionContext;
use globset::{Glob, GlobSet, GlobSetBuilder};
use jsonschema::{Keyword, ValidationError, paths::Location};
use serde_json::{Map, Value};

use super::format::resolve_file_reference_in_context;
use super::simplified::{Constraint, PropertyDef, SchemaArm, SimplifiedType, TypeExpr};

/// Keyword carrying the globs a root-union arm enforces on a file value.
pub const DARKMATTER_MATCH_KEYWORD: &str = "x-darkmatter-match";

/// Compiled positive and negative globs of one `file(match(...))` constraint.
///
/// A path is accepted when at least one positive pattern matches and no
/// negative (`!`-prefixed) pattern does; with only negative patterns, every
/// path a negative pattern does not reject is accepted. A pattern without a
/// `/` that does not start with `**` also matches at any depth, so `*.md`
/// accepts `docs/api.md`.
pub struct FileMatchGlobs {
    positive: Option<GlobSet>,
    negative: GlobSet,
}

impl FileMatchGlobs {
    /// Compile `patterns`; `None` when any pattern is not a valid glob.
    #[must_use]
    pub fn compile(patterns: &[String]) -> Option<Self> {
        let mut positive = GlobSetBuilder::new();
        let mut negative = GlobSetBuilder::new();
        let mut has_positive = false;
        for raw in patterns {
            let (target, is_negative) = match raw.strip_prefix('!') {
                Some(stripped) => (stripped, true),
                None => (raw.as_str(), false),
            };
            let builder = if is_negative {
                &mut negative
            } else {
                has_positive = true;
                &mut positive
            };
            builder.add(Glob::new(target).ok()?);
            if !target.contains('/') && !target.starts_with("**") {
                builder.add(Glob::new(&format!("**/{target}")).ok()?);
            }
        }
        Some(Self {
            positive: if has_positive {
                Some(positive.build().ok()?)
            } else {
                None
            },
            negative: negative.build().ok()?,
        })
    }

    /// Whether a `/`-separated relative path, or its file name, is accepted.
    /// A negative match on either view rejects the path.
    #[must_use]
    pub fn is_match(&self, rel_path: &str, file_name: &str) -> bool {
        if self.negative.is_match(rel_path) || self.negative.is_match(file_name) {
            return false;
        }
        match &self.positive {
            Some(set) => set.is_match(rel_path) || set.is_match(file_name),
            None => true,
        }
    }

    /// Whether the existing file at `path` is accepted, judged relative to the
    /// first of `anchors` that contains it.
    ///
    /// A file under none of them is judged by its full path without the root,
    /// so a `**/`-led glob can still accept it while an anchored one cannot.
    fn admits_path(&self, path: &Path, anchors: &[PathBuf]) -> bool {
        let file_name = path.file_name().and_then(|name| name.to_str()).unwrap_or_default();
        let judge = |relative: &Path| self.is_match(&portable(relative), file_name);
        let containing = |path: &Path, anchors: &[PathBuf]| {
            anchors
                .iter()
                .find_map(|anchor| path.strip_prefix(anchor).ok().map(Path::to_path_buf))
        };
        if let Some(relative) = containing(path, anchors) {
            return judge(&relative);
        }
        // Symlinked spellings (macOS `/var` vs `/private/var`) of one location.
        let canonical = biscuit_file::canonicalize_simplified(path).ok();
        if let Some(canonical) = &canonical {
            let canonical_anchors: Vec<PathBuf> = anchors
                .iter()
                .filter_map(|anchor| biscuit_file::canonicalize_simplified(anchor).ok())
                .collect();
            if let Some(relative) = containing(canonical, &canonical_anchors) {
                return judge(&relative);
            }
        }
        judge(canonical.as_deref().unwrap_or(path))
    }
}

/// The `/`-joined normal components of `path`, whatever the host separator.
fn portable(path: &Path) -> String {
    path.components()
        .filter_map(|component| match component {
            Component::Normal(part) => Some(part.to_string_lossy()),
            _ => None,
        })
        .collect::<Vec<_>>()
        .join("/")
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

/// Whether a caller's file value satisfies an arm's globs, resolved
/// from the caller's own `origin`.
///
/// The same judgment the [`DARKMATTER_MATCH_KEYWORD`] validator makes: a value
/// that names no existing file, or still holds template or shell syntax, is
/// not judged. Unparseable globs admit every value, as they offer none.
#[must_use]
pub fn file_match_admits(value: &str, patterns: &[String], origin: &FileResolutionContext) -> bool {
    FileMatchGlobs::compile(patterns).is_none_or(|globs| {
        admits(&globs, value, Some(origin.base_dir()), None, Some(origin))
    })
}

/// Resolution matches the `darkmatter-file` format's: from `base_dir` when
/// the validator has a document anchor, otherwise from the process directory,
/// which then also anchors the glob.
fn admits(
    globs: &FileMatchGlobs,
    value: &str,
    base_dir: Option<&Path>,
    fallback: Option<&Path>,
    context: Option<&FileResolutionContext>,
) -> bool {
    if crate::markdown::literal_token::holds_pending_syntax(value) {
        return true;
    }
    let Ok(path) = resolve_file_reference_in_context(value, base_dir, fallback, context) else {
        return true;
    };
    let ambient = std::env::current_dir().ok();
    let Some(base_dir) = base_dir.or(ambient.as_deref()) else {
        return true;
    };
    let mut anchors: Vec<PathBuf> = Vec::new();
    let mut push = |anchor: &Path| {
        if !anchors.iter().any(|known| known == anchor) {
            anchors.push(anchor.to_path_buf());
        }
    };
    if let Some(fallback) = fallback {
        push(fallback);
    }
    if let Some(context) = context {
        push(context.launch_magic_scope().request_dir());
    }
    push(base_dir);
    if let Some(root) = context.and_then(FileResolutionContext::repository_root) {
        push(root);
    }
    globs.admits_path(&path, &anchors)
}

type KeywordResult<'a> = Result<Box<dyn for<'i> Keyword<'i>>, ValidationError<'a>>;

/// Factory for [`DARKMATTER_MATCH_KEYWORD`], judging values from the
/// validator's anchors (see [`admits`]).
pub(crate) fn match_keyword_factory(
    base_dir: Option<PathBuf>,
    fallback: Option<PathBuf>,
    context: Option<FileResolutionContext>,
) -> impl for<'a> Fn(&'a Map<String, Value>, &'a Value, Location) -> KeywordResult<'a>
+ Send
+ Sync
+ 'static {
    move |_parent, schema, _location| {
        let patterns: Vec<String> = schema
            .as_array()
            .and_then(|items| items.iter().map(|item| item.as_str().map(String::from)).collect())
            .ok_or_else(|| {
                ValidationError::schema(format!(
                    "{DARKMATTER_MATCH_KEYWORD} must be an array of strings"
                ))
            })?;
        let globs = FileMatchGlobs::compile(&patterns).ok_or_else(|| {
            ValidationError::schema(format!("{DARKMATTER_MATCH_KEYWORD} holds an invalid glob"))
        })?;
        Ok(Box::new(MatchKeyword {
            globs,
            patterns,
            base_dir: base_dir.clone(),
            fallback: fallback.clone(),
            context: context.clone(),
        }))
    }
}

struct MatchKeyword {
    globs: FileMatchGlobs,
    patterns: Vec<String>,
    base_dir: Option<PathBuf>,
    fallback: Option<PathBuf>,
    context: Option<FileResolutionContext>,
}

impl MatchKeyword {
    fn check(&self, value: &str) -> bool {
        admits(
            &self.globs,
            value,
            self.base_dir.as_deref(),
            self.fallback.as_deref(),
            self.context.as_ref(),
        )
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
    fn globs_compare_in_portable_spelling_relative_to_the_first_containing_anchor() {
        let globs = FileMatchGlobs::compile(&["fixes/**/spec.md".to_string()]).unwrap();
        let launch = PathBuf::from("/work/repo");
        let path = launch.join("fixes").join("x").join("spec.md");
        assert!(globs.admits_path(&path, std::slice::from_ref(&launch)));
        let features = launch.join("features").join("x").join("spec.md");
        assert!(!globs.admits_path(&features, &[launch]));
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
