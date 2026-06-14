//! Schema-aware completion for composition `name=value` setters.
//!
//! Phase 4 of the `2026-05-15-schemas` feature. When the cursor sits in a
//! setter slot of `claudine compose`, `claudine inline-compose`, or
//! `claudine sequence` AND a positional prompt-file argument is already
//! committed, this module consults the prompt's `$schema` declaration via
//! Darkmatter to produce:
//!
//! - **Property names** (before `=`) — required properties first, then
//!   optional, in declaration order. Already-supplied names are filtered out.
//! - **Property values** (after `=`) — enum members for `enum` properties,
//!   filesystem paths constrained by `match(...)` globs for `file` properties.
//!
//! All entry points are pure functions over schema state plus filesystem
//! reads — no shell execution, no provider launches. When the schema cannot
//! be resolved or the file_arg cannot be loaded, every function returns an
//! empty `Vec`, signalling to the caller that the shell's native fallback
//! should take over.

use std::collections::HashSet;
use std::path::{Path, PathBuf};

use biscuit_file::YamlValue;
use globset::{Glob, GlobSet, GlobSetBuilder};

use darkmatter::markdown::Markdown;
use darkmatter::markdown::compose::ComposeSource;
use darkmatter::markdown::schemas::{
    CompletionKind, Constraint, DarkmatterSchemas, EffectiveSchema, PropertyAtom, PropertyDef,
    SchemaShape, SimplifiedSchema, completion as dm_completion,
};

use super::fuzzy;
use super::scopes::{self, ScopeContext};

/// Resolve `file_arg` to a loaded [`EffectiveSchema`].
///
/// Returns `None` when:
/// - the file cannot be located on disk,
/// - the file is not valid Markdown,
/// - the document has no `$schema`,
/// - the schema cannot be parsed or compiled.
///
/// All failure modes are silent: completion is best-effort and falls back to
/// shell-native behavior on any error.
pub(crate) fn load_effective_schema(file_arg: &str, ctx: &ScopeContext) -> Option<EffectiveSchema> {
    let path = resolve_prompt_path(file_arg, ctx)?;
    let text = std::fs::read_to_string(&path).ok()?;
    let markdown: Markdown =
        Markdown::from(text).with_source(ComposeSource::infer_from_path(&path));
    DarkmatterSchemas::new().effective_for(&markdown).ok()?
}

/// Resolve a setter-token file_arg to an absolute path on disk.
///
/// Tries the following interpretations in order:
/// 1. As-is when absolute.
/// 2. Relative to cwd.
/// 3. Relative to the effective repo root (when one is detected).
///
/// `@`-prefixed magic paths are not expanded — the file_arg here is the
/// value the user has already committed in argv, which the shell completion
/// engine would have rewritten to a concrete path on selection.
fn resolve_prompt_path(file_arg: &str, ctx: &ScopeContext) -> Option<PathBuf> {
    if file_arg.is_empty() {
        return None;
    }
    let trimmed = file_arg.trim_matches(['"', '\'']);
    let raw = Path::new(trimmed);
    if raw.is_absolute() && raw.is_file() {
        return Some(raw.to_path_buf());
    }
    let from_cwd = ctx.cwd.join(raw);
    if from_cwd.is_file() {
        return Some(from_cwd);
    }
    if let Some(root) = scopes::effective_repo_root(ctx) {
        let from_repo = root.join(raw);
        if from_repo.is_file() {
            return Some(from_repo);
        }
    }
    None
}

/// Property name candidates for the cursor partial.
///
/// Returns names in priority order: required properties first (in declaration
/// order), then optional properties (in declaration order). `supplied` is the
/// set of property names already passed on the command line — those are
/// filtered out so the user is never offered to re-set the same key.
///
/// `declared_order` is the property-name sequence as authored in the prompt's
/// raw frontmatter YAML (or an empty slice when it could not be recovered).
/// When populated, candidates within each required/optional group are sorted
/// to match that authored order; names absent from the list keep their
/// `IndexMap` iteration position appended at the end of the group. This
/// breaks the underlying [`serde_json::Map`] alphabetisation that
/// Darkmatter's [`EffectiveSchema`] inherits when it stores nested
/// frontmatter values.
///
/// Each candidate is rendered with a trailing `=` so accepting it leaves the
/// cursor positioned to start typing the value.
pub(crate) fn property_names(
    effective: &EffectiveSchema,
    partial: &str,
    supplied: &HashSet<String>,
    declared_order: &[String],
) -> Vec<String> {
    let Some(shape) = single_shape(effective) else {
        return Vec::new();
    };

    let mut required: Vec<(usize, String)> = Vec::new();
    let mut optional: Vec<(usize, String)> = Vec::new();
    for (iter_idx, (name, def)) in shape.properties.iter().enumerate() {
        if supplied.contains(name) {
            continue;
        }
        if !name_matches(name, partial) {
            continue;
        }
        let rank = declaration_rank(declared_order, name, iter_idx);
        if is_required(def) {
            required.push((rank, format!("{name}=")));
        } else {
            optional.push((rank, format!("{name}=")));
        }
    }

    required.sort_by_key(|(rank, _)| *rank);
    optional.sort_by_key(|(rank, _)| *rank);

    required
        .into_iter()
        .chain(optional)
        .map(|(_, candidate)| candidate)
        .collect()
}

/// Sort key for a property: its index in the authored `declared_order` when
/// present, otherwise an offset past the end of that list (preserving the
/// underlying [`IndexMap`] iteration position so the relative order of
/// unknown names stays deterministic).
fn declaration_rank(declared_order: &[String], name: &str, iter_idx: usize) -> usize {
    declared_order
        .iter()
        .position(|n| n == name)
        .unwrap_or(declared_order.len() + iter_idx)
}

/// Authored property-name order for `file_arg`'s `$schema` declaration.
///
/// Re-parses the prompt's raw frontmatter YAML with `serde_yaml_ng` (whose
/// `Mapping` preserves insertion order) so the original authored sequence of
/// property names is recoverable. This is necessary because Darkmatter's
/// [`EffectiveSchema`] stores nested frontmatter values as
/// `serde_json::Value`, whose `Map` is alphabetised
/// ([`serde_json::Map`] is a [`BTreeMap`] unless the `preserve_order`
/// feature is enabled) — the declaration order is lost before
/// `parse_yaml_schema` ever sees it.
///
/// Handles three `$schema` shapes:
///
/// - **Inline mapping** → returns the mapping's keys in authored order.
/// - **String file reference** → loads the referenced file (YAML or JSON) and
///   returns the property-names from its `$schema` mapping (or its root
///   `properties` object for raw JSON Schema files).
/// - **Sequence (root union)** → returns an empty `Vec`; root unions have no
///   single property set so completion already declines them.
///
/// Returns an empty `Vec` on any failure (file missing, frontmatter not
/// parseable, no `$schema`, unsupported shape). Callers treat the empty
/// case as "no authored ordering available" and fall back to whatever order
/// the [`SchemaShape::properties`] map provides.
///
/// [`BTreeMap`]: std::collections::BTreeMap
pub(crate) fn declared_property_order(file_arg: &str, ctx: &ScopeContext) -> Vec<String> {
    let Some(path) = resolve_prompt_path(file_arg, ctx) else {
        return Vec::new();
    };
    let Ok(text) = std::fs::read_to_string(&path) else {
        return Vec::new();
    };
    let Some(yaml) = extract_frontmatter_yaml(&text) else {
        return Vec::new();
    };
    let Ok(root) = biscuit_file::serde_yaml_ng::from_str::<YamlValue>(yaml) else {
        return Vec::new();
    };
    let Some(schema_value) = root
        .as_mapping()
        .and_then(|m| m.get(YamlValue::String("$schema".into())))
    else {
        return Vec::new();
    };
    let base_dir = path
        .parent()
        .map(Path::to_path_buf)
        .unwrap_or_else(|| PathBuf::from("."));
    schema_keys_in_order(schema_value, &base_dir)
}

/// Splits the leading `---`-delimited frontmatter block out of a Markdown
/// document and returns its YAML body. Returns `None` when no frontmatter
/// block is present or the block is unterminated.
fn extract_frontmatter_yaml(text: &str) -> Option<&str> {
    let body = text.strip_prefix("---\n")?;
    let end = body
        .find("\n---\n")
        .or_else(|| body.strip_suffix("\n---").map(|trimmed| trimmed.len()))?;
    Some(&body[..end])
}

/// Returns the property-name sequence for a `$schema` YAML value, following
/// file references as needed. Returns an empty `Vec` for shapes that have
/// no single ordered property set (root unions, unsupported types).
fn schema_keys_in_order(value: &YamlValue, base_dir: &Path) -> Vec<String> {
    match value {
        YamlValue::Mapping(map) => map
            .keys()
            .filter_map(|k| k.as_str().map(str::to_string))
            .collect(),
        YamlValue::String(reference) => referenced_schema_keys(reference, base_dir),
        _ => Vec::new(),
    }
}

/// Loads a `$schema` file reference (YAML or JSON), returning the authored
/// property-name order. Mirrors Darkmatter's disambiguation rule: YAML files
/// whose root holds a `$schema:` mapping are SimplifiedSchema documents
/// (the order comes from that nested mapping); everything else is a raw JSON
/// Schema (the order comes from its top-level `properties` object).
fn referenced_schema_keys(reference: &str, base_dir: &Path) -> Vec<String> {
    let candidate = base_dir.join(reference);
    let Ok(text) = std::fs::read_to_string(&candidate) else {
        return Vec::new();
    };
    let lower = candidate
        .extension()
        .and_then(|ext| ext.to_str())
        .map(str::to_ascii_lowercase);
    if lower.as_deref() == Some("json") {
        return json_schema_property_keys(&text);
    }
    let Ok(root) = biscuit_file::serde_yaml_ng::from_str::<YamlValue>(&text) else {
        return Vec::new();
    };
    if let YamlValue::Mapping(map) = &root
        && let Some(inner) = map.get(YamlValue::String("$schema".into()))
        && let YamlValue::Mapping(inner_map) = inner
    {
        return inner_map
            .keys()
            .filter_map(|k| k.as_str().map(str::to_string))
            .collect();
    }
    yaml_root_property_keys(&root)
}

/// Property-name order for a raw JSON Schema file (whose `properties` object
/// is the source of truth).
fn json_schema_property_keys(text: &str) -> Vec<String> {
    let Ok(value) = serde_json::from_str::<serde_json::Value>(text) else {
        return Vec::new();
    };
    value
        .get("properties")
        .and_then(serde_json::Value::as_object)
        .map(|obj| obj.keys().cloned().collect())
        .unwrap_or_default()
}

/// Property-name order for a YAML JSON-Schema-shaped root (its `properties`
/// mapping, when present).
fn yaml_root_property_keys(value: &YamlValue) -> Vec<String> {
    let YamlValue::Mapping(root) = value else {
        return Vec::new();
    };
    let Some(props) = root.get(YamlValue::String("properties".into())) else {
        return Vec::new();
    };
    let YamlValue::Mapping(map) = props else {
        return Vec::new();
    };
    map.keys()
        .filter_map(|k| k.as_str().map(str::to_string))
        .collect()
}

/// Value candidates for `property=<partial>` when the property is in the
/// schema's completable set.
///
/// - `enum` → enum members matching `value_partial` (prefix-insensitive when
///   `value_partial` is non-empty; all members for an empty partial).
/// - `file` → filesystem paths matching the property's `match(...)` globs.
///   Walks the filesystem starting from the effective repo root (or cwd when
///   no repo). Empty `match` patterns return no candidates — fall back to
///   shell-native file completion.
///
/// Each candidate is rendered as the **full** `name='value'` token so the
/// shell can replace the entire setter under the cursor (matches the
/// existing `setter_value` contract).
///
/// Returns an empty `Vec` for completion shapes that have no value
/// candidates (string, number, url/email/date hints) — those defer to the
/// caller's fallback completer.
pub(crate) fn property_value(
    effective: &EffectiveSchema,
    property: &str,
    value_partial: &str,
    ctx: &ScopeContext,
) -> Vec<String> {
    let Some(suggestion) = dm_completion::for_property(effective, property) else {
        return Vec::new();
    };
    match &suggestion.kind {
        CompletionKind::Enum { members } => enum_candidates(property, members, value_partial),
        CompletionKind::File { patterns } => {
            file_candidates(property, patterns, value_partial, ctx)
        }
        // Hints don't produce concrete candidates — the caller falls back
        // to the existing `@`-gated path or shell-native completion.
        CompletionKind::Hint { .. } => Vec::new(),
    }
}

/// Returns the format hint string for a property when the schema marks it as
/// a hint-only completable type (url, email, date, datetime, time).
///
/// Used by the engine to surface a one-line description through the shell
/// when the completion protocol supports it. The current `__complete`
/// stdout protocol emits one candidate per line with no description channel,
/// so the engine does not yet route through this — exposed for future
/// integration with description-bearing completion protocols.
#[allow(dead_code)]
pub(crate) fn property_value_hint(
    effective: &EffectiveSchema,
    property: &str,
) -> Option<&'static str> {
    let suggestion = dm_completion::for_property(effective, property)?;
    match suggestion.kind {
        CompletionKind::Hint { format } => Some(format),
        _ => None,
    }
}

#[cfg(test)]
fn ordered_completable_suggestions(
    effective: &EffectiveSchema,
) -> Vec<darkmatter::markdown::schemas::CompletionSuggestion> {
    dm_completion::completable_properties(effective)
        .into_iter()
        .filter_map(|name| dm_completion::for_property(effective, &name))
        .collect()
}

fn name_matches(name: &str, partial: &str) -> bool {
    if partial.is_empty() {
        return true;
    }
    // Use fuzzy subsequence matching to match the rest of the engine's
    // partial-classification semantics. Short partials behave as starting
    // substrings under `fuzzy_match` (which lowercases both sides).
    fuzzy::fuzzy_match(name, partial)
}

fn single_shape(effective: &EffectiveSchema) -> Option<&SchemaShape> {
    match effective.simplified.as_ref()? {
        SimplifiedSchema::Single(shape) => Some(shape),
        SimplifiedSchema::Union(_) => None,
    }
}

fn is_required(def: &PropertyDef) -> bool {
    let atoms: Vec<&PropertyAtom> = match def {
        PropertyDef::Single(atom) => vec![atom],
        PropertyDef::Union(items) => items.iter().collect(),
    };
    atoms.iter().any(|atom| {
        atom.constraints
            .iter()
            .any(|c| matches!(c, Constraint::Required))
    })
}

fn enum_candidates(property: &str, members: &[String], value_partial: &str) -> Vec<String> {
    let trimmed = value_partial.trim_matches(['"', '\'']);
    members
        .iter()
        .filter(|m| {
            trimmed.is_empty()
                || m.to_ascii_lowercase()
                    .starts_with(&trimmed.to_ascii_lowercase())
        })
        .map(|m| format!("{property}='{m}'"))
        .collect()
}

fn file_candidates(
    property: &str,
    patterns: &[String],
    value_partial: &str,
    ctx: &ScopeContext,
) -> Vec<String> {
    // Empty `match()` means "any file" — without a pattern the result set
    // would be unbounded, so we return nothing and let the existing
    // `@`-gated path or shell-native file completion handle it.
    if patterns.is_empty() {
        return Vec::new();
    }
    let Some(matcher) = MatchGlobs::compile(patterns) else {
        return Vec::new();
    };
    let base: PathBuf = scopes::effective_repo_root(ctx)
        .map(Path::to_path_buf)
        .unwrap_or_else(|| ctx.cwd.clone());
    let trimmed = value_partial.trim_matches(['"', '\'']);
    let active = trimmed.trim_start_matches('@');

    let mut out: Vec<String> = Vec::new();
    let mut seen: HashSet<String> = HashSet::new();
    let walker = ignore::WalkBuilder::new(&base)
        .hidden(true)
        .git_ignore(true)
        .require_git(false)
        .build();
    for entry in walker.flatten() {
        let path = entry.path();
        if !path.is_file() {
            continue;
        }
        let Ok(rel) = path.strip_prefix(&base) else {
            continue;
        };
        let Some(rel_str) = rel.to_str() else {
            continue;
        };
        let file_name = path.file_name().and_then(|n| n.to_str()).unwrap_or(rel_str);
        if !matcher.is_match(rel_str, file_name) {
            continue;
        }
        // The active partial is fuzzy-matched against the basename — users
        // typically narrow file completion by typing characters from the
        // file name, not the leading directory path. Path-qualified globs
        // are already enforced above by the GlobSet check against `rel_str`.
        if !active.is_empty() && !fuzzy::fuzzy_match(file_name, active) {
            continue;
        }
        let rendered = format!("{property}='{rel_str}'");
        if seen.insert(rendered.clone()) {
            out.push(rendered);
        }
    }
    out.sort();
    out
}

/// Compiled positive + negative globset pair for a Darkmatter
/// `file(match(...))` constraint.
///
/// Mirrors Darkmatter's `x-darkmatter-match` keyword semantics: a path is
/// accepted iff (a) at least one positive pattern matches AND (b) no
/// negative pattern matches. When the constraint contains only negative
/// patterns, every non-rejected path is accepted (`positive: None`).
///
/// Each pattern is added to the globset twice — once with its raw shape
/// (which may contain path separators like `src/**/*.rs`) and once as a
/// `**/`-anchored variant so filename-only patterns like `*.png` continue
/// to match files in subdirectories. This matches Darkmatter's
/// multi-candidate validator: `*.md` accepts `docs/api.md` because the
/// resolved filename `api.md` is one of the views the validator tests.
struct MatchGlobs {
    positive: Option<GlobSet>,
    negative: GlobSet,
}

impl MatchGlobs {
    fn compile(patterns: &[String]) -> Option<Self> {
        let mut positive = GlobSetBuilder::new();
        let mut negative = GlobSetBuilder::new();
        let mut has_positive = false;
        let mut has_negative = false;
        for raw in patterns {
            let (target, is_negative) = match raw.strip_prefix('!') {
                Some(stripped) => (stripped, true),
                None => (raw.as_str(), false),
            };
            let primary = Glob::new(target).ok()?;
            // Anchor filename-only patterns so they match anywhere in the
            // tree. Skip when the pattern already contains a path separator
            // or is itself a recursive prefix.
            let secondary = if target.contains('/') || target.starts_with("**") {
                None
            } else {
                Glob::new(&format!("**/{target}")).ok()
            };
            if is_negative {
                negative.add(primary);
                if let Some(g) = secondary {
                    negative.add(g);
                }
                has_negative = true;
            } else {
                positive.add(primary);
                if let Some(g) = secondary {
                    positive.add(g);
                }
                has_positive = true;
            }
        }
        let positive = if has_positive {
            Some(positive.build().ok()?)
        } else {
            None
        };
        let negative = if has_negative {
            negative.build().ok()?
        } else {
            GlobSetBuilder::new().build().ok()?
        };
        Some(Self { positive, negative })
    }

    /// Returns true when the relative path is accepted by the constraint.
    ///
    /// Both `rel_path` (e.g. `src/lib.rs`) and `file_name` (e.g. `lib.rs`)
    /// are tested so that patterns can target either view. Negation wins:
    /// a negative match against any view rejects the path even if a
    /// positive pattern would accept it.
    fn is_match(&self, rel_path: &str, file_name: &str) -> bool {
        if self.negative.is_match(rel_path) || self.negative.is_match(file_name) {
            return false;
        }
        match &self.positive {
            Some(set) => set.is_match(rel_path) || set.is_match(file_name),
            None => true,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::TempDir;

    fn write(path: &Path, content: &str) {
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent).unwrap();
        }
        fs::write(path, content).unwrap();
    }

    fn seed_repo(root: &Path) {
        fs::create_dir_all(root.join(".git")).unwrap();
    }

    fn effective_from_doc(doc: &str) -> EffectiveSchema {
        let md: Markdown = doc.into();
        DarkmatterSchemas::new()
            .effective_for(&md)
            .unwrap()
            .expect("effective schema")
    }

    #[test]
    fn property_names_required_first_then_optional() {
        let effective = effective_from_doc(concat!(
            "---\n",
            "$schema:\n",
            "  title: 'string(required)'\n",
            "  status: 'enum(draft, published; required)'\n",
            "  description: string\n",
            "  count: number\n",
            "---\nbody\n",
        ));
        let got = property_names(&effective, "", &HashSet::new(), &[]);
        // Without an authored-order hint the fall-back is required-first
        // then optional, in `IndexMap` iteration order. The actual order
        // within each group can vary because Darkmatter stores nested
        // frontmatter values as `serde_json::Value` (alphabetised), so
        // the only contract we can assert here is the group boundary.
        let pos = |needle: &str| got.iter().position(|c| c == needle).unwrap();
        assert!(pos("status=") < pos("description="));
        assert!(pos("status=") < pos("count="));
        assert!(pos("title=") < pos("description="));
        assert!(pos("title=") < pos("count="));
        assert_eq!(got.len(), 4);
    }

    #[test]
    fn property_names_respects_declared_order_within_groups() {
        let effective = effective_from_doc(concat!(
            "---\n",
            "$schema:\n",
            "  title: 'string(required)'\n",
            "  status: 'enum(draft, published; required)'\n",
            "  description: string\n",
            "  count: number\n",
            "---\nbody\n",
        ));
        let declared_order = vec![
            "title".to_string(),
            "status".to_string(),
            "description".to_string(),
            "count".to_string(),
        ];
        let got = property_names(&effective, "", &HashSet::new(), &declared_order);
        assert_eq!(
            got,
            vec![
                "title=".to_string(),
                "status=".to_string(),
                "description=".to_string(),
                "count=".to_string(),
            ],
            "required group must preserve `title` before `status`, optional \
             group must preserve `description` before `count`",
        );

        // Reversing the authored order must reverse the within-group output.
        let reversed = vec![
            "count".to_string(),
            "description".to_string(),
            "status".to_string(),
            "title".to_string(),
        ];
        let got = property_names(&effective, "", &HashSet::new(), &reversed);
        assert_eq!(
            got,
            vec![
                "status=".to_string(),
                "title=".to_string(),
                "count=".to_string(),
                "description=".to_string(),
            ],
        );
    }

    #[test]
    fn property_names_filters_supplied() {
        let effective = effective_from_doc(concat!(
            "---\n",
            "$schema:\n",
            "  title: 'string(required)'\n",
            "  description: string\n",
            "---\nbody\n",
        ));
        let mut supplied = HashSet::new();
        supplied.insert("title".to_string());
        let got = property_names(&effective, "", &supplied, &[]);
        assert_eq!(got, vec!["description="]);
    }

    #[test]
    fn property_names_fuzzy_matches_partial() {
        let effective = effective_from_doc(concat!(
            "---\n",
            "$schema:\n",
            "  title: 'string(required)'\n",
            "  description: string\n",
            "---\nbody\n",
        ));
        let got = property_names(&effective, "des", &HashSet::new(), &[]);
        assert_eq!(got, vec!["description="]);
    }

    #[test]
    fn declared_property_order_returns_authored_keys_for_inline_schema() {
        let tmp = TempDir::new().unwrap();
        seed_repo(tmp.path());
        write(
            &tmp.path().join("prompt.md"),
            concat!(
                "---\n",
                "$schema:\n",
                "  title: 'string(required)'\n",
                "  status: 'enum(draft, published; required)'\n",
                "  description: string\n",
                "  count: number\n",
                "---\nbody\n",
            ),
        );
        let ctx = ScopeContext::discover_from(tmp.path());
        let order = declared_property_order("prompt.md", &ctx);
        assert_eq!(
            order,
            vec![
                "title".to_string(),
                "status".to_string(),
                "description".to_string(),
                "count".to_string(),
            ],
        );
    }

    #[test]
    fn declared_property_order_returns_empty_for_root_union_schema() {
        let tmp = TempDir::new().unwrap();
        seed_repo(tmp.path());
        write(
            &tmp.path().join("p.md"),
            concat!(
                "---\n",
                "$schema:\n",
                "  - title: 'string(required)'\n",
                "  - name: 'string(required)'\n",
                "---\nbody\n",
            ),
        );
        let ctx = ScopeContext::discover_from(tmp.path());
        let order = declared_property_order("p.md", &ctx);
        assert!(
            order.is_empty(),
            "root unions have no single ordered property set: {order:?}",
        );
    }

    #[test]
    fn declared_property_order_follows_yaml_file_reference() {
        let tmp = TempDir::new().unwrap();
        seed_repo(tmp.path());
        write(
            &tmp.path().join("schema.yaml"),
            "$schema:\n  zeta: 'string(required)'\n  alpha: number\n",
        );
        write(
            &tmp.path().join("p.md"),
            "---\n$schema: ./schema.yaml\n---\nbody\n",
        );
        let ctx = ScopeContext::discover_from(tmp.path());
        let order = declared_property_order("p.md", &ctx);
        assert_eq!(order, vec!["zeta".to_string(), "alpha".to_string()]);
    }

    #[test]
    fn declared_property_order_returns_empty_when_no_frontmatter() {
        let tmp = TempDir::new().unwrap();
        seed_repo(tmp.path());
        write(&tmp.path().join("p.md"), "no frontmatter here\n");
        let ctx = ScopeContext::discover_from(tmp.path());
        assert!(declared_property_order("p.md", &ctx).is_empty());
    }

    #[test]
    fn property_value_returns_enum_members() {
        let effective = effective_from_doc(concat!(
            "---\n",
            "$schema:\n",
            "  status: 'enum(draft, published, archived; required)'\n",
            "---\nbody\n",
        ));
        let tmp = TempDir::new().unwrap();
        seed_repo(tmp.path());
        let ctx = ScopeContext::discover_from(tmp.path());
        let got = property_value(&effective, "status", "", &ctx);
        assert!(got.contains(&"status='draft'".to_string()));
        assert!(got.contains(&"status='published'".to_string()));
        assert!(got.contains(&"status='archived'".to_string()));
    }

    #[test]
    fn property_value_filters_enum_by_partial() {
        let effective = effective_from_doc(concat!(
            "---\n",
            "$schema:\n",
            "  status: 'enum(draft, published, archived; required)'\n",
            "---\nbody\n",
        ));
        let tmp = TempDir::new().unwrap();
        seed_repo(tmp.path());
        let ctx = ScopeContext::discover_from(tmp.path());
        let got = property_value(&effective, "status", "pub", &ctx);
        assert_eq!(got, vec!["status='published'".to_string()]);
    }

    #[test]
    fn property_value_returns_files_for_match_pattern() {
        let effective = effective_from_doc(concat!(
            "---\n",
            "$schema:\n",
            "  cover: \"file(match('*.png'))\"\n",
            "---\nbody\n",
        ));
        let tmp = TempDir::new().unwrap();
        seed_repo(tmp.path());
        write(&tmp.path().join("assets").join("cover.png"), "");
        write(&tmp.path().join("assets").join("other.jpg"), "");
        let ctx = ScopeContext::discover_from(tmp.path());
        let got = property_value(&effective, "cover", "", &ctx);
        assert!(
            got.iter().any(|c| c.ends_with("cover.png'")),
            "expected cover.png in candidates: {got:?}"
        );
        assert!(
            !got.iter().any(|c| c.contains("other.jpg")),
            "non-matching extension must be filtered: {got:?}"
        );
    }

    #[test]
    fn property_value_returns_empty_for_string_property() {
        let effective = effective_from_doc(concat!(
            "---\n",
            "$schema:\n",
            "  title: 'string(required)'\n",
            "---\nbody\n",
        ));
        let tmp = TempDir::new().unwrap();
        seed_repo(tmp.path());
        let ctx = ScopeContext::discover_from(tmp.path());
        let got = property_value(&effective, "title", "", &ctx);
        assert!(got.is_empty());
    }

    #[test]
    fn property_value_returns_empty_for_file_without_match() {
        let effective = effective_from_doc(concat!(
            "---\n",
            "$schema:\n",
            "  cover: file\n",
            "---\nbody\n",
        ));
        let tmp = TempDir::new().unwrap();
        seed_repo(tmp.path());
        write(&tmp.path().join("a.txt"), "");
        let ctx = ScopeContext::discover_from(tmp.path());
        let got = property_value(&effective, "cover", "", &ctx);
        assert!(got.is_empty(), "no `match` => no candidates: {got:?}");
    }

    #[test]
    fn property_value_hint_returns_format_for_url() {
        let effective = effective_from_doc(concat!(
            "---\n",
            "$schema:\n",
            "  homepage: url\n",
            "---\nbody\n",
        ));
        let hint = property_value_hint(&effective, "homepage");
        assert!(hint.unwrap_or("").contains("URL"));
    }

    #[test]
    fn load_effective_schema_resolves_cwd_relative_path() {
        let tmp = TempDir::new().unwrap();
        seed_repo(tmp.path());
        let doc_path = tmp.path().join("prompt.md");
        write(
            &doc_path,
            "---\n$schema:\n  title: 'string(required)'\n---\nbody\n",
        );
        let ctx = ScopeContext::discover_from(tmp.path());
        let effective = load_effective_schema("prompt.md", &ctx).expect("schema loads");
        let suggestions = ordered_completable_suggestions(&effective);
        // `title` is `string` so it's NOT a completable type.
        assert!(suggestions.is_empty(), "string is not completable");
    }

    #[test]
    fn load_effective_schema_returns_none_for_missing_file() {
        let tmp = TempDir::new().unwrap();
        seed_repo(tmp.path());
        let ctx = ScopeContext::discover_from(tmp.path());
        assert!(load_effective_schema("does-not-exist.md", &ctx).is_none());
    }

    #[test]
    fn load_effective_schema_returns_none_when_no_schema() {
        let tmp = TempDir::new().unwrap();
        seed_repo(tmp.path());
        write(&tmp.path().join("p.md"), "---\ntitle: hi\n---\n");
        let ctx = ScopeContext::discover_from(tmp.path());
        assert!(load_effective_schema("p.md", &ctx).is_none());
    }

    #[test]
    fn load_effective_schema_strips_surrounding_quotes() {
        let tmp = TempDir::new().unwrap();
        seed_repo(tmp.path());
        write(
            &tmp.path().join("p.md"),
            "---\n$schema:\n  status: 'enum(a, b)'\n---\n",
        );
        let ctx = ScopeContext::discover_from(tmp.path());
        assert!(load_effective_schema("'p.md'", &ctx).is_some());
        assert!(load_effective_schema("\"p.md\"", &ctx).is_some());
    }

    #[test]
    fn match_globs_basename_pattern_matches_anywhere_in_tree() {
        let matcher = MatchGlobs::compile(&["*.png".to_string()]).unwrap();
        assert!(matcher.is_match("cover.png", "cover.png"));
        assert!(matcher.is_match("assets/cover.png", "cover.png"));
        assert!(matcher.is_match("a/b/c/cover.png", "cover.png"));
        assert!(!matcher.is_match("cover.jpg", "cover.jpg"));
    }

    #[test]
    fn match_globs_honors_negation_against_basename() {
        let matcher = MatchGlobs::compile(&["*.md".to_string(), "!_*.md".to_string()]).unwrap();
        assert!(matcher.is_match("plan.md", "plan.md"));
        assert!(matcher.is_match("docs/plan.md", "plan.md"));
        assert!(!matcher.is_match("_draft.md", "_draft.md"));
        assert!(!matcher.is_match("docs/_draft.md", "_draft.md"));
        assert!(!matcher.is_match("notes.txt", "notes.txt"));
    }

    #[test]
    fn match_globs_path_qualified_glob_matches_relative_path() {
        let matcher = MatchGlobs::compile(&["src/**/*.rs".to_string()]).unwrap();
        assert!(matcher.is_match("src/lib.rs", "lib.rs"));
        assert!(matcher.is_match("src/inner/mod.rs", "mod.rs"));
        // Files outside `src/` must NOT match a path-qualified pattern,
        // even when the basename would match `*.rs`.
        assert!(!matcher.is_match("tests/integration.rs", "integration.rs"));
        assert!(!matcher.is_match("benches/perf.rs", "perf.rs"));
    }

    #[test]
    fn match_globs_path_qualified_negation_filters_subset() {
        let matcher =
            MatchGlobs::compile(&["src/**/*.rs".to_string(), "!src/**/test_*.rs".to_string()])
                .unwrap();
        assert!(matcher.is_match("src/lib.rs", "lib.rs"));
        assert!(matcher.is_match("src/inner/mod.rs", "mod.rs"));
        assert!(!matcher.is_match("src/test_helpers.rs", "test_helpers.rs"));
        assert!(!matcher.is_match("src/inner/test_util.rs", "test_util.rs"));
    }
}
