//! The re-export report: which names or modules a file forwards from
//! another module, and how completely the analysis could tell.
//!
//! Every key of every type here is always serialized (`null`, `[]` and
//! `false` included) and none has a default, so the cache reader can refuse
//! a damaged entry instead of reading a missing key as a negative.

use serde::{Deserialize, Deserializer, Serialize};

use super::TextSpan;

/// A key that must be present but may be `null`; unlike a plain `Option`
/// field, an absent key is an error rather than `None`.
fn nullable<'de, D, T>(deserializer: D) -> Result<Option<T>, D::Error>
where
    D: Deserializer<'de>,
    T: Deserialize<'de>,
{
    Option::<T>::deserialize(deserializer)
}

/// What a file forwards, with the coverage that says how far to trust an
/// empty list.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ReexportReport {
    pub coverage: ReexportCoverage,
    /// Source order (by `span.start_byte`), one entry per forwarding site.
    pub entries: Vec<ReexportEntry>,
}

impl ReexportReport {
    /// The report of a language with no forwarding construct.
    pub fn not_applicable() -> Self {
        Self {
            coverage: ReexportCoverage { state: ReexportCoverageState::NotApplicable, limits: Vec::new() },
            entries: Vec::new(),
        }
    }

    /// A report with no entries whose coverage is `limited` by `limits`.
    pub fn limited(limits: Vec<ReexportLimit>) -> Self {
        Self {
            coverage: ReexportCoverage { state: ReexportCoverageState::Limited, limits },
            entries: Vec::new(),
        }
    }
}

/// How completely the analysis covers the file's forwarding syntax.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ReexportCoverage {
    pub state: ReexportCoverageState,
    /// Ascending by (`span.start_byte`, `reason`). Empty exactly when `state`
    /// is not `Limited`.
    pub limits: Vec<ReexportLimit>,
}

/// `Complete` means complete under the language's documented contract, not a
/// claim about arbitrary runtime behavior.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ReexportCoverageState {
    Complete,
    Limited,
    /// Decided by language alone: the language has no forwarding construct.
    NotApplicable,
}

/// One region the analysis could not fully read.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ReexportLimit {
    pub reason: ReexportLimitReason,
    /// The region the limit covers; a whole-file limit spans the root node.
    pub span: TextSpan,
}

/// Why a region is limited. New reasons may be added; existing ones keep
/// their meaning.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ReexportLimitReason {
    /// An ERROR or MISSING node touches forwarding syntax.
    ParseError,
    /// Forwarding syntax the pinned grammar cannot represent.
    GrammarGap,
    /// An export list not built from literals.
    ComputedExportList,
    /// An export list extended from another module.
    CrossFileExportList,
    /// A forwarded name or interface table is reassigned.
    ReboundBinding,
    /// `require`, `module`, `exports` or `import` is shadowed.
    ShadowedGlobal,
    /// Runtime forwarding: `Object.assign`, spread, `module()`, a custom
    /// import routine.
    DynamicForwarding,
    /// A Rust macro body that may contain `pub use`.
    UnexpandedMacro,
}

/// One forwarding site.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ReexportEntry {
    pub form: ReexportForm,
    pub evidence: ReexportEvidence,
    /// The name callers import; `None` for `Wildcard` and `Module`.
    #[serde(deserialize_with = "nullable")]
    pub exported_name: Option<String>,
    /// The name in the source; `Some` exactly for `Named` and `ValueCopy`.
    #[serde(deserialize_with = "nullable")]
    pub original_name: Option<String>,
    /// The same-file binding the forward goes through, when one does.
    #[serde(deserialize_with = "nullable")]
    pub local_name: Option<String>,
    /// The module, package or path as written; `None` when the file does not
    /// state one (C# `TypeForwardedTo`).
    #[serde(deserialize_with = "nullable")]
    pub source: Option<String>,
    /// Names a wildcard excludes (Scala `x as _`); empty otherwise.
    pub excluded_names: Vec<String>,
    /// Qualified name of the enclosing module, namespace or object; `None`
    /// at file level.
    #[serde(deserialize_with = "nullable")]
    pub scope: Option<String>,
    /// Written visibility restriction (`crate`, `super`, `in crate::a`);
    /// `None` when unrestricted.
    #[serde(deserialize_with = "nullable")]
    pub restriction: Option<String>,
    pub type_only: bool,
    /// Under a build or runtime condition (Rust `cfg`, Swift `#if`, Python
    /// `if`/`try`, Go build constraint); never evaluated.
    pub conditional: bool,
    /// Ascending.
    pub caveats: Vec<ReexportCaveat>,
    /// `span` of the `ImportRecord` this entry forwards, when it forwards
    /// one.
    #[serde(deserialize_with = "nullable")]
    pub import_span: Option<TextSpan>,
    /// The forwarding specifier or clause.
    pub span: TextSpan,
    /// The whole forwarding statement.
    pub statement_span: TextSpan,
}

impl ReexportEntry {
    /// Whether this entry marks the import at `span` as forwarded: only
    /// dedicated syntax or a declared interface does; `exposed` never.
    pub fn forwards_import_at(&self, span: &TextSpan) -> bool {
        self.evidence != ReexportEvidence::Exposed && self.import_span.as_ref() == Some(span)
    }
}

/// What a forwarding site does.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ReexportForm {
    /// Forwards one name.
    Named,
    /// Forwards every public name of `source` without listing them.
    Wildcard,
    /// Exposes a whole module under `exported_name`.
    Namespace,
    /// Makes `source` the file's whole interface.
    Module,
    /// A new binding initialized from an imported value; identity is not
    /// preserved.
    ValueCopy,
}

/// What kind of fact backs a forwarding entry.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ReexportEvidence {
    /// Dedicated forwarding syntax.
    Syntax,
    /// A convention the language's tooling treats as an explicit interface
    /// declaration.
    Declared,
    /// Runtime exposure with no declaration.
    Exposed,
}

/// A qualification on an entry; ascending in `ReexportEntry::caveats`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ReexportCaveat {
    /// Go `var` copy.
    MutableBinding,
    /// C# `TypeForwardedTo`: the target assembly is not in the file.
    SourceUnresolved,
    /// C++ using-directive: names are found only when not shadowed.
    FallbackLookup,
    /// Swift `@_exported`.
    UnstableFeature,
    /// Go import matched by its last path segment.
    InferredPackageName,
    /// Scala `given` selector.
    GivensOnly,
}

#[cfg(test)]
mod tests {
    use super::*;

    fn entry_json() -> serde_json::Value {
        serde_json::json!({
            "form": "named", "evidence": "syntax", "exported_name": "read",
            "original_name": "parse", "local_name": null, "source": "./parser",
            "excluded_names": [], "scope": null, "restriction": null,
            "type_only": false, "conditional": false, "caveats": [],
            "import_span": null,
            "span": TextSpan::default(), "statement_span": TextSpan::default(),
        })
    }

    #[test]
    fn an_entry_round_trips_with_every_key_written() {
        let entry: ReexportEntry = serde_json::from_value(entry_json()).unwrap();
        assert_eq!(entry.exported_name.as_deref(), Some("read"));
        assert_eq!(serde_json::to_value(&entry).unwrap(), entry_json());
    }

    #[test]
    fn a_null_name_is_read_but_an_absent_key_is_refused() {
        let mut absent = entry_json();
        absent.as_object_mut().unwrap().remove("local_name");
        assert!(serde_json::from_value::<ReexportEntry>(absent).is_err());

        let mut null = entry_json();
        null["exported_name"] = serde_json::Value::Null;
        assert_eq!(serde_json::from_value::<ReexportEntry>(null).unwrap().exported_name, None);
    }

    #[test]
    fn unknown_keys_and_variants_are_refused() {
        let mut extra = entry_json();
        extra["extra"] = serde_json::json!(1);
        assert!(serde_json::from_value::<ReexportEntry>(extra).is_err());

        let mut variant = entry_json();
        variant["form"] = serde_json::json!("barrel");
        assert!(serde_json::from_value::<ReexportEntry>(variant).is_err());
    }

    #[test]
    fn the_constructors_pin_the_coverage_shapes() {
        let not_applicable = serde_json::to_value(ReexportReport::not_applicable()).unwrap();
        assert_eq!(
            not_applicable,
            serde_json::json!({ "coverage": { "state": "not_applicable", "limits": [] }, "entries": [] })
        );
        let limited = ReexportReport::limited(vec![ReexportLimit {
            reason: ReexportLimitReason::ComputedExportList,
            span: TextSpan::default(),
        }]);
        assert_eq!(limited.coverage.state, ReexportCoverageState::Limited);
        assert!(limited.entries.is_empty());
    }

    #[test]
    fn an_exposed_entry_never_forwards_an_import() {
        let span = TextSpan { start_byte: 3, end_byte: 9, ..TextSpan::default() };
        let mut entry: ReexportEntry = serde_json::from_value(entry_json()).unwrap();
        entry.import_span = Some(span.clone());
        assert!(entry.forwards_import_at(&span));
        entry.evidence = ReexportEvidence::Declared;
        assert!(entry.forwards_import_at(&span));
        entry.evidence = ReexportEvidence::Exposed;
        assert!(!entry.forwards_import_at(&span));
        entry.evidence = ReexportEvidence::Syntax;
        assert!(!entry.forwards_import_at(&TextSpan::default()));
    }
}
