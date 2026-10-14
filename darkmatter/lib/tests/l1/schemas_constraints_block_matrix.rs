//! The input robustness matrix for block `$constraints` on reused and inline
//! objects (`2026-10-13-named-type-presence-and-local-names`, Phase 5).
//!
//! The fixture is the shipped `lifecycle-branch-bundle` named type from
//! `darkmatter/docs/schemas/claudine-types.yaml`, which declares
//! `$constraints: { min-keys: 1 }`; the matrix names its copy
//! `lifecycle-action`. Every cell makes one edit to that block,
//! then judges it at two sites:
//!
//! - **reused**: the type lives in a standalone schema file and a document
//!   reuses it as `lifecycle-action(…)@./types-N.yaml`, so the block merges
//!   with use-site postfix constraints;
//! - **inline**: the same object is a plain inline property of the
//!   document's own `$schema`.
//!
//! Each column starts from a base block whose field is load-bearing: a probe
//! document on the violating side of each bound, or an authored-versus-
//! `Completion` comparison for `generated`. A row's expectation is one of the
//! positive verdicts, the baseline verdicts (field omitted), the zero-bound
//! verdicts, or an error from the public load, resolution, or validation
//! path. Duplicate keys and trailing content are written into the source
//! text before anything decodes it, because a decoded map cannot show a lost
//! duplicate.
//!
//! The two tests walk the same table: one writes YAML block syntax, the other
//! writes the JSON form the same YAML reader accepts (a `.yaml` schema file
//! and frontmatter spelled as JSON). A `.json` schema file is a raw JSON
//! Schema, not a simplified schema, so it has no `$constraints` to read.

use std::fmt::Write as _;
use std::path::{Path, PathBuf};

use biscuit_file::FileResolutionContext;
use darkmatter::markdown::Markdown;
use darkmatter::markdown::schemas::{DarkmatterSchemas, SchemaPhase};
use serde_yaml_ng::Value as YamlValue;

use crate::request_support::context_at;

// ── source rendering ──────────────────────────────────────────────────────

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Format {
    Yaml,
    Json,
}

/// A source node. `Raw` text is written verbatim in both formats, so every
/// raw value is spelled as JSON (which YAML also reads) unless the cell is
/// deliberately malformed. Object entries are a list, so a repeated key is
/// written twice rather than collapsed.
#[derive(Clone, Debug)]
enum Node {
    Raw(String),
    Object(Vec<(String, Node)>),
}

fn raw(text: &str) -> Node {
    Node::Raw(text.to_string())
}

fn object(entries: &[(&str, &str)]) -> Node {
    Node::Object(entries.iter().map(|(key, value)| (key.to_string(), raw(value))).collect())
}

fn render_key(format: Format, key: &str) -> String {
    match format {
        Format::Json => serde_json::to_string(key).unwrap(),
        // `$constraints` and kebab-case keywords are plain YAML scalars, as
        // authors write them; a pattern key like `<string>` needs quotes.
        Format::Yaml if key.starts_with('<') => serde_json::to_string(key).unwrap(),
        Format::Yaml => key.to_string(),
    }
}

fn render_entries(format: Format, entries: &[(String, Node)], indent: usize, out: &mut String) {
    match format {
        Format::Yaml => {
            for (key, node) in entries {
                let pad = " ".repeat(indent);
                match node {
                    Node::Raw(text) => writeln!(out, "{pad}{}: {text}", render_key(format, key)).unwrap(),
                    Node::Object(children) if children.is_empty() => {
                        writeln!(out, "{pad}{}: {{}}", render_key(format, key)).unwrap()
                    }
                    Node::Object(children) => {
                        writeln!(out, "{pad}{}:", render_key(format, key)).unwrap();
                        render_entries(format, children, indent + 2, out);
                    }
                }
            }
        }
        Format::Json => out.push_str(&render_json_object(entries)),
    }
}

fn render_json_object(entries: &[(String, Node)]) -> String {
    let members: Vec<String> = entries
        .iter()
        .map(|(key, node)| {
            let value = match node {
                Node::Raw(text) => text.clone(),
                Node::Object(children) => render_json_object(children),
            };
            format!("{}: {value}", render_key(Format::Json, key))
        })
        .collect();
    format!("{{{}}}", members.join(", "))
}

/// A whole top-level mapping: block YAML, or one JSON object line.
fn render_document_mapping(format: Format, entries: &[(String, Node)]) -> String {
    let mut out = String::new();
    render_entries(format, entries, 0, &mut out);
    if format == Format::Json {
        out.push('\n');
    }
    out
}

// ── the shipped fixture ───────────────────────────────────────────────────

/// The shipped `lifecycle-branch-bundle` members other than `$constraints`, each
/// as JSON text, after checking the shipped block is still `{ min-keys: 1 }`.
fn shipped_lifecycle_action(shipped: &str) -> Vec<(String, Node)> {
    let value: YamlValue = serde_yaml_ng::from_str(shipped).expect("shipped claudine-types.yaml parses");
    let definition = value["$schema"]["lifecycle-branch-bundle"]
        .as_mapping()
        .expect("claudine-types.yaml defines `lifecycle-branch-bundle` as an object");
    let block: YamlValue = serde_yaml_ng::from_str("min-keys: 1").unwrap();
    assert_eq!(
        definition.get("$constraints"),
        Some(&block),
        "the shipped fixture's block changed; re-derive this matrix's base columns"
    );
    definition
        .iter()
        .filter(|(key, _)| key.as_str() != Some("$constraints"))
        .map(|(key, member)| {
            let key = key.as_str().expect("string member names").to_string();
            let json = serde_json::to_value(member).expect("member converts to JSON");
            (key, Node::Raw(serde_json::to_string(&json).unwrap()))
        })
        .collect()
}

// ── the matrix ────────────────────────────────────────────────────────────

/// One edit to a column's base block.
#[derive(Clone, Debug)]
enum Edit {
    Unchanged,
    /// Removes the column's key (or the whole `$constraints` member).
    Remove,
    /// Replaces the column's value (or the whole container) with raw text.
    Set(&'static str),
    /// Writes the column's key (or the `$constraints` member) twice.
    Duplicate,
    /// Replaces the container with these entries.
    Members(&'static [(&'static str, &'static str)]),
}

#[derive(Clone, Copy, Debug)]
enum Expect {
    Positive,
    Baseline,
    Zero,
    Error,
}

/// One probe document: frontmatter data members (JSON text values) and the
/// phase to judge it at (`None` is authored validation). The three booleans
/// are the expected validity under the positive, baseline, and zero-bound
/// verdicts.
struct Probe {
    data: &'static [(&'static str, &'static str)],
    phase: Option<SchemaPhase>,
    positive: bool,
    baseline: bool,
    zero: bool,
}

const fn probe(data: &'static [(&'static str, &'static str)], positive: bool, baseline: bool) -> Probe {
    Probe { data, phase: None, positive, baseline, zero: baseline }
}

struct Column {
    name: &'static str,
    /// `None` for the container column.
    key: Option<&'static str>,
    base: &'static [(&'static str, &'static str)],
    /// The document's `$schema` members for the reused site; `TYPES` is
    /// replaced with the cell's schema file name.
    reuse_schema: &'static [(&'static str, &'static str)],
    reuse_probes: Vec<Probe>,
    inline_probes: Vec<Probe>,
    rows: Vec<(&'static str, Edit, Expect)>,
}

const EMPTY: &[(&str, &str)] = &[("action", "{}")];
const ONE: &[(&str, &str)] = &[("action", "{\"a\": 1}")];
const TWO: &[(&str, &str)] = &[("action", "{\"a\": 1, \"b\": 2}")];
const THREE: &[(&str, &str)] = &[("action", "{\"a\": 1, \"b\": 2, \"c\": 3}")];
const FOUR: &[(&str, &str)] = &[("action", "{\"a\": 1, \"b\": 2, \"c\": 3, \"d\": 4}")];
const NOTHING: &[(&str, &str)] = &[];
const PINNED_ONLY: &[(&str, &str)] = &[("pinned", "{\"a\": 1}")];
const BOTH: &[(&str, &str)] = &[("action", "{\"a\": 1}"), ("pinned", "{\"a\": 1}")];

fn numeric_rows() -> Vec<(&'static str, Edit, Expect)> {
    vec![
        ("positive control", Edit::Unchanged, Expect::Positive),
        ("absent", Edit::Remove, Expect::Baseline),
        ("explicit null", Edit::Set("null"), Expect::Error),
        ("wrong type: string", Edit::Set("\"1\""), Expect::Error),
        ("wrong type: fraction", Edit::Set("1.5"), Expect::Error),
        ("wrong type: negative", Edit::Set("-1"), Expect::Error),
        ("wrong type: boolean", Edit::Set("true"), Expect::Error),
        ("wrong type: object", Edit::Set("{\"n\": 1}"), Expect::Error),
        ("one invalid element", Edit::Set("[1, \"x\"]"), Expect::Error),
        ("all invalid elements", Edit::Set("[\"x\", \"y\"]"), Expect::Error),
        ("all invalid elements: a list of one number", Edit::Set("[1]"), Expect::Error),
        ("empty string", Edit::Set("\"\""), Expect::Error),
        ("empty list", Edit::Set("[]"), Expect::Error),
        ("empty object", Edit::Set("{}"), Expect::Error),
        ("zero", Edit::Set("0"), Expect::Zero),
        ("duplicate key", Edit::Duplicate, Expect::Error),
        ("trailing content", Edit::Set("1 ]"), Expect::Error),
        ("invalid content: two values", Edit::Set("1, 2"), Expect::Error),
    ]
}

fn flag_rows() -> Vec<(&'static str, Edit, Expect)> {
    vec![
        ("positive control", Edit::Unchanged, Expect::Positive),
        ("absent", Edit::Remove, Expect::Baseline),
        ("explicit null", Edit::Set("null"), Expect::Error),
        ("wrong type: string", Edit::Set("\"true\""), Expect::Error),
        ("wrong type: number", Edit::Set("1"), Expect::Error),
        ("wrong type: object", Edit::Set("{\"v\": true}"), Expect::Error),
        ("one invalid element", Edit::Set("[true, \"x\"]"), Expect::Error),
        ("all invalid elements", Edit::Set("[\"x\"]"), Expect::Error),
        ("all invalid elements: a list of `true`", Edit::Set("[true]"), Expect::Error),
        ("empty string", Edit::Set("\"\""), Expect::Error),
        ("empty list", Edit::Set("[]"), Expect::Error),
        ("empty object", Edit::Set("{}"), Expect::Error),
        ("false", Edit::Set("false"), Expect::Error),
        ("duplicate key", Edit::Duplicate, Expect::Error),
        ("trailing content", Edit::Set("true ]"), Expect::Error),
        ("invalid content: two values", Edit::Set("true, false"), Expect::Error),
    ]
}

fn container_rows() -> Vec<(&'static str, Edit, Expect)> {
    vec![
        ("positive control", Edit::Unchanged, Expect::Positive),
        ("absent", Edit::Remove, Expect::Baseline),
        ("explicit null", Edit::Set("null"), Expect::Error),
        ("wrong type: number", Edit::Set("1"), Expect::Error),
        ("wrong type: string", Edit::Set("\"min-keys\""), Expect::Error),
        ("wrong type: list", Edit::Set("[\"min-keys\"]"), Expect::Error),
        ("one invalid member: unknown key", Edit::Members(&[("min-keys", "1"), ("bogus", "1")]), Expect::Error),
        ("one invalid member: bad value", Edit::Members(&[("min-keys", "1"), ("max-keys", "\"x\"")]), Expect::Error),
        ("all invalid members: unknown key", Edit::Members(&[("bogus", "1")]), Expect::Error),
        ("all invalid members: bad value", Edit::Members(&[("min-keys", "\"x\"")]), Expect::Error),
        ("empty object", Edit::Set("{}"), Expect::Baseline),
        ("empty list", Edit::Set("[]"), Expect::Error),
        ("duplicate `$constraints`", Edit::Duplicate, Expect::Error),
        ("trailing content", Edit::Set("{\"min-keys\": 1} ]"), Expect::Error),
        ("invalid content: unbalanced brace", Edit::Set("{\"min-keys\": 1}}"), Expect::Error),
    ]
}

fn columns() -> Vec<Column> {
    // The shipped lower bound, reused under a use-site upper bound that a
    // four-key object violates whatever the block says.
    let lower_bound_reuse: &[(&str, &str)] = &[("action", "lifecycle-action(max-keys(3))@./TYPES")];
    let lower_bound_reuse_probes = || {
        vec![probe(EMPTY, false, true), probe(ONE, true, true), probe(FOUR, false, false)]
    };
    let lower_bound_inline_probes = || vec![probe(EMPTY, false, true), probe(ONE, true, true)];
    let with_zero = |mut probe: Probe, zero: bool| {
        probe.zero = zero;
        probe
    };
    vec![
        Column {
            name: "$constraints",
            key: None,
            base: &[("min-keys", "1")],
            reuse_schema: lower_bound_reuse,
            reuse_probes: lower_bound_reuse_probes(),
            inline_probes: lower_bound_inline_probes(),
            rows: container_rows(),
        },
        Column {
            name: "min-keys",
            key: Some("min-keys"),
            base: &[("min-keys", "1")],
            reuse_schema: lower_bound_reuse,
            reuse_probes: lower_bound_reuse_probes(),
            inline_probes: lower_bound_inline_probes(),
            rows: numeric_rows(),
        },
        // The shipped block with its one member swapped for an upper bound,
        // so `max-keys: 0` is not also a contradiction with `min-keys: 1`.
        Column {
            name: "max-keys",
            key: Some("max-keys"),
            base: &[("max-keys", "2")],
            reuse_schema: lower_bound_reuse,
            reuse_probes: vec![
                with_zero(probe(EMPTY, true, true), true),
                with_zero(probe(TWO, true, true), false),
                with_zero(probe(THREE, false, true), false),
                with_zero(probe(FOUR, false, false), false),
            ],
            inline_probes: vec![
                with_zero(probe(EMPTY, true, true), true),
                with_zero(probe(TWO, true, true), false),
                with_zero(probe(THREE, false, true), false),
            ],
            rows: numeric_rows(),
        },
        // `pinned` carries a use-site `(required)`, so an empty document is
        // invalid with or without the block flag.
        Column {
            name: "required",
            key: Some("required"),
            base: &[("min-keys", "1"), ("required", "true")],
            reuse_schema: &[
                ("action", "lifecycle-action@./TYPES"),
                ("pinned", "lifecycle-action(required)@./TYPES"),
            ],
            reuse_probes: vec![probe(PINNED_ONLY, false, true), probe(NOTHING, false, false), probe(BOTH, true, true)],
            inline_probes: vec![probe(NOTHING, false, true), probe(ONE, true, true)],
            rows: flag_rows(),
        },
        // Block `required` stays, so dropping `generated` makes absence fail
        // authored validation; `pinned` carries a use-site `(generated)`
        // that keeps deferring its presence either way.
        Column {
            name: "generated",
            key: Some("generated"),
            base: &[("min-keys", "1"), ("required", "true"), ("generated", "true")],
            reuse_schema: &[
                ("action", "lifecycle-action@./TYPES"),
                ("pinned", "lifecycle-action(generated)@./TYPES"),
            ],
            reuse_probes: vec![
                probe(PINNED_ONLY, true, false),
                Probe { phase: Some(SchemaPhase::Completion), ..probe(PINNED_ONLY, false, false) },
                probe(ONE, true, true),
                Probe { phase: Some(SchemaPhase::Completion), ..probe(ONE, false, false) },
                probe(BOTH, true, true),
            ],
            inline_probes: vec![
                probe(NOTHING, true, false),
                Probe { phase: Some(SchemaPhase::Completion), ..probe(NOTHING, false, false) },
                probe(ONE, true, true),
            ],
            rows: flag_rows(),
        },
    ]
}

/// The edited `$constraints` member(s) of the object, or none when removed.
fn constraints_members(column: &Column, edit: &Edit) -> Vec<(String, Node)> {
    let base = || object(column.base);
    let one = |node: Node| vec![("$constraints".to_string(), node)];
    let Some(key) = column.key else {
        return match edit {
            Edit::Unchanged => one(base()),
            Edit::Remove => Vec::new(),
            Edit::Set(text) => one(raw(text)),
            Edit::Duplicate => [one(base()), one(base())].concat(),
            Edit::Members(members) => one(object(members)),
        };
    };
    let mut members: Vec<(String, Node)> = Vec::new();
    for (name, value) in column.base {
        if *name != key {
            members.push((name.to_string(), raw(value)));
            continue;
        }
        match edit {
            Edit::Unchanged => members.push((name.to_string(), raw(value))),
            Edit::Remove => {}
            Edit::Set(text) => members.push((name.to_string(), raw(text))),
            Edit::Duplicate => {
                members.push((name.to_string(), raw(value)));
                members.push((name.to_string(), raw(value)));
            }
            Edit::Members(_) => unreachable!("member edits apply to the container column"),
        }
    }
    one(Node::Object(members))
}

#[derive(Clone, Copy, Debug)]
enum Site {
    Reused,
    Inline,
}

/// The observed result of one cell at one site.
#[derive(Debug, PartialEq, Eq)]
enum Observed {
    Verdicts(Vec<bool>),
    /// The message of the first probe's error. Every probe failed.
    Error(String),
    /// Some probes failed to load or resolve and some did not.
    Mixed(String),
}

struct Matrix {
    format: Format,
    root: PathBuf,
    context: FileResolutionContext,
    action: Vec<(String, Node)>,
    files: usize,
}

impl Matrix {
    fn object(&self, column: &Column, edit: &Edit) -> Node {
        let mut entries = self.action.clone();
        entries.extend(constraints_members(column, edit));
        Node::Object(entries)
    }

    /// Writes this cell's sources under fresh names and judges every probe.
    fn observe(&mut self, column: &Column, edit: &Edit, site: Site) -> Observed {
        self.files += 1;
        let types_name = format!("types-{}.yaml", self.files);
        let object = self.object(column, edit);
        let (schema, probes): (Node, &[Probe]) = match site {
            Site::Reused => {
                let types = vec![(
                    "$schema".to_string(),
                    Node::Object(vec![("lifecycle-action".to_string(), object)]),
                )];
                std::fs::write(self.root.join(&types_name), render_document_mapping(self.format, &types))
                    .expect("write schema file");
                let members = column
                    .reuse_schema
                    .iter()
                    .map(|(name, expression)| {
                        let expression = expression.replace("TYPES", &types_name);
                        (name.to_string(), Node::Raw(serde_json::to_string(&expression).unwrap()))
                    })
                    .collect();
                (Node::Object(members), &column.reuse_probes)
            }
            Site::Inline => (Node::Object(vec![("action".to_string(), object)]), &column.inline_probes),
        };
        let mut verdicts = Vec::new();
        let mut errors = Vec::new();
        for (index, probe) in probes.iter().enumerate() {
            let mut frontmatter = vec![("$schema".to_string(), schema.clone())];
            frontmatter.extend(probe.data.iter().map(|(key, value)| (key.to_string(), raw(value))));
            let text = format!("---\n{}---\nbody\n", render_document_mapping(self.format, &frontmatter));
            let path = self.root.join(format!("doc-{}-{index}.md", self.files));
            std::fs::write(&path, &text).expect("write document");
            match self.judge(&path, probe.phase) {
                Ok(valid) => verdicts.push(valid),
                Err(message) => errors.push(message),
            }
        }
        match (verdicts.is_empty(), errors.is_empty()) {
            (_, true) => Observed::Verdicts(verdicts),
            (true, false) => Observed::Error(errors.swap_remove(0)),
            (false, false) => Observed::Mixed(errors.swap_remove(0)),
        }
    }

    /// Loads the document through the fallible public loader and validates
    /// it. A load or schema error is returned as its full display chain.
    fn judge(&self, path: &Path, phase: Option<SchemaPhase>) -> Result<bool, String> {
        let markdown = Markdown::try_from(path).map_err(|error| format!("load: {error}"))?;
        let api = DarkmatterSchemas::new(self.context.clone());
        let report = match phase {
            Some(phase) => api.validate_for_phase(&markdown, phase),
            None => api.validate(&markdown),
        };
        report.map(|report| report.valid).map_err(|error| {
            let mut message = format!("schema: {error}");
            let mut source = std::error::Error::source(&error);
            while let Some(cause) = source {
                write!(message, " <- {cause}").unwrap();
                source = cause.source();
            }
            message
        })
    }
}

fn walk_matrix(format: Format, shipped: &str) {
    let dir = tempfile::tempdir().expect("temp dir");
    let root = biscuit_file::canonicalize_simplified(dir.path()).expect("canonical temp dir");
    let mut matrix = Matrix {
        format,
        context: context_at(&root),
        root,
        action: shipped_lifecycle_action(shipped),
        files: 0,
    };
    let mut failures = Vec::new();
    let mut table = String::new();
    for column in columns() {
        for (row, edit, expect) in &column.rows {
            for site in [Site::Reused, Site::Inline] {
                let probes = match site {
                    Site::Reused => &column.reuse_probes,
                    Site::Inline => &column.inline_probes,
                };
                let observed = matrix.observe(&column, edit, site);
                let label = format!("{format:?} / {} / {row} / {site:?}", column.name);
                writeln!(table, "{label}: {observed:?}").unwrap();
                let verdicts = |pick: fn(&Probe) -> bool| Observed::Verdicts(probes.iter().map(pick).collect());
                // An error names where it was authored: the cell's schema file
                // for a reused type, the property (or, for a frontmatter syntax
                // error, the document) for an inline one. An error from a
                // decoded value also names the `$constraints` block.
                let located = |message: &str| {
                    let n = matrix.files;
                    let at_site = match site {
                        Site::Reused => message.contains(&format!("types-{n}.yaml")),
                        Site::Inline => {
                            message.contains("property `action`") || message.contains(&format!("doc-{n}-"))
                        }
                    };
                    let syntax = message.contains("not valid YAML")
                        || message.starts_with("load: Failed to parse frontmatter");
                    at_site && (syntax || message.contains("`$constraints"))
                };
                let ok = match expect {
                    Expect::Positive => observed == verdicts(|probe| probe.positive),
                    Expect::Baseline => observed == verdicts(|probe| probe.baseline),
                    Expect::Zero => observed == verdicts(|probe| probe.zero),
                    Expect::Error => matches!(&observed, Observed::Error(message) if located(message)),
                };
                if !ok {
                    failures.push(format!("{label}: expected {expect:?}, observed {observed:?}"));
                }
            }
        }
    }
    eprintln!("{table}");
    assert!(failures.is_empty(), "{} cell(s) failed:\n{}", failures.len(), failures.join("\n"));
}

#[test]
fn constraints_block_matrix_yaml() {
    walk_matrix(Format::Yaml, include_str!("../../../docs/schemas/claudine-types.yaml"));
}

#[test]
fn constraints_block_matrix_json_form() {
    walk_matrix(Format::Json, include_str!("../../../docs/schemas/claudine-types.yaml"));
}
