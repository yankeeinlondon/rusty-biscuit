//! Error-vocabulary loading stage: the ordered per-provider
//! error-classification tables that drive
//! `lib/src/stream/providers/common.rs::classify_error_by_keywords`.
//!
//! This is the typed input model plus the source-declaration loader for the
//! standalone stream-layer vocabulary artifact (spec D1–D3). It is
//! deliberately NOT a `ProviderInfo` field: the general mapping registry is
//! exhaustive over serialized `ProviderInfo` fields, and this vocabulary is
//! not one of them. It therefore owns its own facts-to-research collision
//! guard here, mirroring the delete-on-graduate guarantee the registry gives
//! every other catalog field.
//!
//! ## Source lifecycle
//!
//! The graduated source is the `agent-errors` research topic
//! ([`RESEARCH_TOPIC`]). Its provenance-bearing objects are projected into the
//! lean runtime shape below; the collision guard fails while any facts file
//! still carries the retired [`FACTS_KEY`].
//!
//! ## Order is the behavior contract
//!
//! Bucket *sequence order* encodes each provider's precedence quirks (the
//! "late ApiRemote" second pass, Gemini's Configuration-first kind branch),
//! and repeated `kind` across buckets is legal and meaningful. Parsing walks
//! the YAML sequences into `Vec`s so both order and repeated kinds survive
//! unchanged.

use std::collections::BTreeSet;
use std::path::{Path, PathBuf};

use serde::Deserialize;
use serde_json::Value;

use crate::agent_errors_check::ResearchVocabulary;
use crate::emit::{pascal, provider_variant, render_slice};
use crate::errors::GenError;
use crate::generate::{CheckOutcome, diff_lines, provider_slugs};
use crate::inputs::{self, ProviderInputs};

/// The facts-file key carrying a provider's ordered error vocabulary. Owned by
/// this loader, so the general mapping-registry collision gate skips it.
pub const FACTS_KEY: &str = "error_vocabulary";

/// The research topic that graduates the vocabulary in Phase C.
pub const RESEARCH_TOPIC: &str = "agent-errors";

/// Which input layer supplies a provider's error vocabulary.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum VocabularySource {
    /// Phase A: the seeded facts file (`docs/providers/facts/<slug>.yaml`).
    Facts,
    /// Phase C: the graduated `agent-errors/` research frontmatter.
    Research,
}

impl VocabularySource {
    /// Wire label used in collision diagnostics.
    pub fn wire(self) -> &'static str {
        match self {
            VocabularySource::Facts => "facts",
            VocabularySource::Research => "research",
        }
    }
}

/// The graduated source-of-truth. The collision guard rejects any retired
/// facts entry that remains after research graduation.
pub const DECLARED_SOURCE: VocabularySource = VocabularySource::Research;

/// One provider's ordered runtime error-classification vocabulary.
///
/// Absent `kind_buckets`/`code_buckets` deserialize as empty vectors —
/// message-only classifiers (Pi, Antigravity) omit the kind branch, and only
/// Kimi carries numeric `code_buckets`.
#[derive(Debug, Clone, Default, PartialEq, Eq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ErrorVocabulary {
    /// Buckets checked against a structured error-kind discriminator.
    #[serde(default)]
    pub kind_buckets: Vec<KeywordBucket>,
    /// Buckets checked against the free-form error message.
    #[serde(default)]
    pub msg_buckets: Vec<KeywordBucket>,
    /// Numeric wire-code → kind mappings (Kimi JSON-RPC codes).
    #[serde(default)]
    pub code_buckets: Vec<CodeBucket>,
}

/// One ordered keyword bucket: a semantic kind plus its ordered needle list.
///
/// `kind` is the snake_case `SemanticErrorKind` name; the generator (Phase A2)
/// validates membership and lowercase needles. Repeated `kind` across buckets
/// is legal (the "late ApiRemote" second pass).
#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct KeywordBucket {
    pub kind: String,
    pub needles: Vec<String>,
}

/// One numeric wire-code → kind mapping (Kimi JSON-RPC, spec D5).
#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CodeBucket {
    pub code: i64,
    pub kind: String,
}

/// Loads one provider's error vocabulary from its already-loaded inputs,
/// enforcing the delete-on-graduate collision guard.
///
/// ## Returns
///
/// `Ok(None)` when neither the declared nor the other layer carries the
/// vocabulary (the missing-source state — a provider without a stream parser).
///
/// ## Errors
///
/// - [`GenError::VocabularyCollision`] when both the facts and research layers
///   carry the vocabulary (graduation-in-progress must delete one).
/// - [`GenError::VocabularyInvalid`] when the declared layer's value does not
///   match the [`ErrorVocabulary`] shape.
pub fn load_error_vocabulary(inputs: &ProviderInputs) -> Result<Option<ErrorVocabulary>, GenError> {
    let facts = inputs.facts.get(FACTS_KEY);
    let research = inputs.research.get(RESEARCH_TOPIC);
    match resolve_source(&inputs.slug, facts, research, DECLARED_SOURCE)? {
        None => Ok(None),
        Some(value) => Ok(Some(parse_declared_vocabulary(
            &inputs.slug,
            value,
            DECLARED_SOURCE,
        )?)),
    }
}

/// Selects the single active vocabulary value, enforcing that the non-declared
/// layer is empty (the delete-on-graduate guard). Returns the declared layer's
/// value, or `None` when the declared layer is also empty (missing-source).
fn resolve_source<'a>(
    slug: &str,
    facts: Option<&'a Value>,
    research: Option<&'a Value>,
    declared: VocabularySource,
) -> Result<Option<&'a Value>, GenError> {
    let (primary, secondary) = match declared {
        VocabularySource::Facts => (facts, research),
        VocabularySource::Research => (research, facts),
    };
    if secondary.is_some() {
        return Err(GenError::VocabularyCollision {
            slug: slug.to_string(),
            declared: declared.wire(),
            offending: match declared {
                VocabularySource::Facts => VocabularySource::Research.wire(),
                VocabularySource::Research => VocabularySource::Facts.wire(),
            },
        });
    }
    Ok(primary)
}

/// Parses one `error_vocabulary` value into the typed model, preserving YAML
/// sequence order and repeated-kind buckets.
fn parse_error_vocabulary(slug: &str, value: &Value) -> Result<ErrorVocabulary, GenError> {
    serde_json::from_value(value.clone()).map_err(|err| GenError::VocabularyInvalid {
        slug: slug.to_string(),
        message: err.to_string(),
    })
}

/// Parses the selected source and drops research-only provenance while
/// preserving branch, bucket, and item order exactly.
fn parse_declared_vocabulary(
    slug: &str,
    value: &Value,
    source: VocabularySource,
) -> Result<ErrorVocabulary, GenError> {
    match source {
        VocabularySource::Facts => parse_error_vocabulary(slug, value),
        VocabularySource::Research => {
            let research: ResearchVocabulary = serde_json::from_value(value.clone()).map_err(
                |err| GenError::VocabularyInvalid {
                    slug: slug.to_string(),
                    message: err.to_string(),
                },
            )?;
            Ok(project_research_vocabulary(research))
        }
    }
}

fn project_research_vocabulary(research: ResearchVocabulary) -> ErrorVocabulary {
    let project_buckets = |buckets: Vec<crate::agent_errors_check::ResearchBucket>| {
        buckets
            .into_iter()
            .map(|bucket| KeywordBucket {
                kind: bucket.kind,
                needles: bucket.needles.into_iter().map(|needle| needle.text).collect(),
            })
            .collect()
    };
    ErrorVocabulary {
        kind_buckets: project_buckets(research.kind_buckets),
        msg_buckets: project_buckets(research.msg_buckets),
        code_buckets: research
            .code_buckets
            .into_iter()
            .flat_map(|bucket| {
                let kind = bucket.kind;
                bucket.codes.into_iter().map(move |code| CodeBucket {
                    code: code.code,
                    kind: kind.clone(),
                })
            })
            .collect(),
    }
}

// ---------------------------------------------------------------------------
// Generation stage: research → lib/src/stream/providers/vocabulary.rs
// ---------------------------------------------------------------------------

/// Wired providers with no structured stream parser. They emit an explicit
/// empty vocabulary table and must NOT carry an `error_vocabulary` in facts.
/// Goose is the only one.
const PARSERLESS_SLUGS: &[&str] = &["goose"];

/// The five `SemanticErrorKind` wire members a bucket may name. Kept in sync
/// by hand: the generator cannot depend on the `claudine` lib (bootstrap
/// rule), so an added variant means adding it here (mirroring how [`emit`]
/// hardcodes every other enum's wire forms).
const VALID_KINDS: &[&str] = &[
    "configuration",
    "agent_native",
    "api_remote",
    "interrupted",
    "unknown",
];

/// The committed generated-vocabulary path under an area root.
pub fn vocabulary_path(area: &Path) -> PathBuf {
    area.join("lib/src/stream/providers/vocabulary.rs")
}

/// Builds the deterministic `vocabulary.rs` text from every wired provider's
/// schema-validated `agent-errors` research document.
///
/// ## Errors
///
/// Fails loudly when a parser-backed provider is missing its vocabulary, a
/// parserless provider carries one, or any bucket violates the hygiene rules
/// ([`GenError::VocabularyGenInvalid`]); the loader's collision/parse errors
/// propagate unchanged.
pub fn build_vocabulary(area: &Path) -> Result<String, GenError> {
    let slugs = provider_slugs();
    let mut tables = Vec::with_capacity(slugs.len());
    for slug in slugs {
        let inputs = inputs::load(area, slug, &[RESEARCH_TOPIC])?;
        let loaded = load_error_vocabulary(&inputs)?;
        let parserless = PARSERLESS_SLUGS.contains(&slug);
        let vocab = match (loaded, parserless) {
            // Providers without a stream parser emit an explicit empty table.
            (None, true) => ErrorVocabulary::default(),
            (Some(vocab), true) if vocab != ErrorVocabulary::default() => {
                return Err(gen_invalid(
                    slug,
                    "provider has no structured stream parser but carries an \
                     executable `agent-errors` vocabulary — keep its researched \
                     findings non-executable until a parser lands",
                ));
            }
            (Some(vocab), true) => vocab,
            (None, false) => {
                return Err(gen_invalid(
                    slug,
                    "parser-backed provider is missing its required ordered vocabulary \
                     in `agent-errors` research — research it before generating",
                ));
            }
            (Some(vocab), false) => {
                validate_vocabulary(slug, &vocab)?;
                vocab
            }
        };
        tables.push((slug, provider_variant(slug)?, vocab));
    }
    Ok(emit_file(&tables))
}

/// Byte-compares [`build_vocabulary`] output against the committed file — the
/// code path shared by the CLI `check` subcommand and the drift test.
pub fn check_vocabulary(area: &Path) -> Result<CheckOutcome, GenError> {
    let generated = build_vocabulary(area)?;
    let path = vocabulary_path(area);
    if !path.is_file() {
        return Ok(CheckOutcome::MissingCommitted { path });
    }
    let committed = std::fs::read_to_string(&path).map_err(|source| GenError::Io {
        path: path.clone(),
        source,
    })?;
    if committed == generated {
        return Ok(CheckOutcome::Clean);
    }
    Ok(CheckOutcome::Drift {
        details: diff_lines(&committed, &generated),
    })
}

fn gen_invalid(slug: &str, message: impl Into<String>) -> GenError {
    GenError::VocabularyGenInvalid {
        slug: slug.to_string(),
        message: message.into(),
    }
}

/// Every generate-time hygiene gate: a required non-empty message vocabulary,
/// known semantic kinds, non-empty buckets, lowercase/non-whitespace needles,
/// and (for Kimi) i32-fitting, non-duplicate numeric codes.
fn validate_vocabulary(slug: &str, vocab: &ErrorVocabulary) -> Result<(), GenError> {
    if vocab.msg_buckets.is_empty() {
        return Err(gen_invalid(
            slug,
            "parser-backed provider requires a non-empty `msg_buckets` vocabulary",
        ));
    }
    validate_keyword_branch(slug, "kind_buckets", &vocab.kind_buckets)?;
    validate_keyword_branch(slug, "msg_buckets", &vocab.msg_buckets)?;

    let mut seen = BTreeSet::new();
    for bucket in &vocab.code_buckets {
        if !VALID_KINDS.contains(&bucket.kind.as_str()) {
            return Err(gen_invalid(
                slug,
                format!(
                    "code_buckets: unknown semantic kind `{}` for code {}",
                    bucket.kind, bucket.code
                ),
            ));
        }
        if i32::try_from(bucket.code).is_err() {
            return Err(gen_invalid(
                slug,
                format!("code_buckets: code {} does not fit in i32", bucket.code),
            ));
        }
        if !seen.insert(bucket.code) {
            return Err(gen_invalid(
                slug,
                format!("code_buckets: duplicate numeric code {}", bucket.code),
            ));
        }
    }
    Ok(())
}

fn validate_keyword_branch(
    slug: &str,
    branch: &str,
    buckets: &[KeywordBucket],
) -> Result<(), GenError> {
    for (index, bucket) in buckets.iter().enumerate() {
        if !VALID_KINDS.contains(&bucket.kind.as_str()) {
            return Err(gen_invalid(
                slug,
                format!("{branch}[{index}]: unknown semantic kind `{}`", bucket.kind),
            ));
        }
        if bucket.needles.is_empty() {
            return Err(gen_invalid(
                slug,
                format!("{branch}[{index}] ({}): bucket has no needles", bucket.kind),
            ));
        }
        for needle in &bucket.needles {
            if needle.trim().is_empty() {
                return Err(gen_invalid(
                    slug,
                    format!(
                        "{branch}[{index}] ({}): empty or whitespace-only needle",
                        bucket.kind
                    ),
                ));
            }
            if needle != &needle.to_ascii_lowercase() {
                return Err(gen_invalid(
                    slug,
                    format!(
                        "{branch}[{index}] ({}): needle `{needle}` must be lowercase",
                        bucket.kind
                    ),
                ));
            }
        }
    }
    Ok(())
}

fn emit_file(tables: &[(&str, &str, ErrorVocabulary)]) -> String {
    let mut out = String::from("// GENERATED by claudine-gen — DO NOT EDIT BY HAND.\n//\n// Inputs:\n");
    for (slug, _, _) in tables {
        if !PARSERLESS_SLUGS.contains(slug) {
            out.push_str(&format!(
                "//   docs/research/agent-errors/{slug}.md (ordered vocabulary projection)\n"
            ));
        }
    }
    out.push_str(
        "// Regenerate with `cargo run -p claudine-gen -- generate`; drift-check with\n\
         // `cargo run -p claudine-gen -- check` (the same code path as the drift test).\n\
         \n\
         //! Generated per-provider error-classification vocabulary tables.\n\
         //!\n\
         //! One [`ErrorKeywords`] table per wired provider plus the exhaustive\n\
         //! [`error_keywords`] accessor. Providers with no structured stream parser\n\
         //! (Goose) emit an explicitly empty table. Every parser's `classify_error*`\n\
         //! consults [`error_keywords`] for its runtime provider identity.\n\
         \n\
         use super::common::ErrorKeywords;\n\
         use super::semantic::SemanticErrorKind;\n\
         use crate::provider_id::Provider;\n\
         \n",
    );
    for (slug, _, vocab) in tables {
        out.push_str(&emit_table(slug, vocab));
        out.push('\n');
    }
    out.push_str(
        "/// Exhaustive `Provider` → error-classification vocabulary accessor.\n\
         pub(crate) fn error_keywords(provider: Provider) -> &'static ErrorKeywords {\n\
         \x20   match provider {\n",
    );
    for (slug, variant, _) in tables {
        out.push_str(&format!(
            "        Provider::{variant} => &{}_VOCABULARY,\n",
            slug.to_uppercase()
        ));
    }
    out.push_str("    }\n}\n");
    out
}

fn emit_table(slug: &str, vocab: &ErrorVocabulary) -> String {
    let prefix = slug.to_uppercase();
    let mut out = format!(
        "pub(crate) static {prefix}_VOCABULARY: ErrorKeywords = ErrorKeywords {{\n"
    );
    out.push_str(&format!(
        "    kind_buckets: {},\n",
        emit_keyword_buckets(&vocab.kind_buckets)
    ));
    out.push_str(&format!(
        "    msg_buckets: {},\n",
        emit_keyword_buckets(&vocab.msg_buckets)
    ));
    out.push_str(&format!(
        "    code_buckets: {},\n",
        emit_code_buckets(&vocab.code_buckets)
    ));
    out.push_str("};\n");
    out
}

/// Renders a `&[(SemanticErrorKind, &[&str])]` literal at struct-field indent
/// (level 1). Needle lists stay inline; the outer bucket list wraps one row
/// per line when the inline form runs long.
fn emit_keyword_buckets(buckets: &[KeywordBucket]) -> String {
    let elements: Vec<String> = buckets
        .iter()
        .map(|bucket| {
            let needles: Vec<String> =
                bucket.needles.iter().map(|n| format!("{n:?}")).collect();
            format!(
                "(SemanticErrorKind::{}, &[{}])",
                pascal(&bucket.kind),
                needles.join(", ")
            )
        })
        .collect();
    render_slice(&elements, 1)
}

/// Renders a `&[(i32, SemanticErrorKind)]` literal for the numeric code
/// branch (empty for every provider except Kimi).
fn emit_code_buckets(buckets: &[CodeBucket]) -> String {
    let elements: Vec<String> = buckets
        .iter()
        .map(|bucket| {
            format!("({}, SemanticErrorKind::{})", bucket.code, pascal(&bucket.kind))
        })
        .collect();
    render_slice(&elements, 1)
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn facts_vocab() -> Value {
        json!({
            "kind_buckets": [
                { "kind": "api_remote", "needles": ["rate", "quota", "billing"] },
                { "kind": "configuration", "needles": ["auth", "config"] },
                { "kind": "interrupted", "needles": ["interrupt", "cancel", "abort"] },
                { "kind": "api_remote", "needles": ["api", "upstream", "server"] }
            ],
            "msg_buckets": [
                { "kind": "api_remote", "needles": ["rate limit", "quota"] }
            ]
        })
    }

    fn research_vocab() -> Value {
        json!({
            "kind_buckets": [{
                "kind": "api_remote",
                "needles": [
                    { "text": "rate", "evidence": "seed" },
                    { "text": "overloaded", "evidence": "documented", "source": "https://example.com/errors" }
                ]
            }],
            "msg_buckets": [{
                "kind": "configuration",
                "needles": [{ "text": "api key", "evidence": "seed" }]
            }],
            "code_buckets": [{
                "kind": "agent_native",
                "codes": [
                    { "code": -32700, "name": "PARSE_ERROR", "evidence": "seed" },
                    { "code": -32600, "evidence": "source_code", "source": "https://example.com/source" }
                ]
            }],
            "gaps": [{ "area": "capacity", "notes": "Exact phrasing remains unknown." }]
        })
    }

    #[test]
    fn parse_preserves_sequence_order_of_buckets_and_needles() {
        let vocab = parse_error_vocabulary("codex", &facts_vocab()).unwrap();
        // Bucket order is the behavior contract — assert it verbatim.
        let kinds: Vec<&str> = vocab.kind_buckets.iter().map(|b| b.kind.as_str()).collect();
        assert_eq!(kinds, ["api_remote", "configuration", "interrupted", "api_remote"]);
        assert_eq!(vocab.kind_buckets[0].needles, ["rate", "quota", "billing"]);
        assert_eq!(vocab.kind_buckets[3].needles, ["api", "upstream", "server"]);
    }

    #[test]
    fn parse_preserves_repeated_kind_buckets() {
        // The "late ApiRemote" second pass is a distinct fourth bucket, not
        // merged into the first api_remote bucket.
        let vocab = parse_error_vocabulary("codex", &facts_vocab()).unwrap();
        let api_remote_buckets: Vec<_> = vocab
            .kind_buckets
            .iter()
            .filter(|b| b.kind == "api_remote")
            .collect();
        assert_eq!(api_remote_buckets.len(), 2);
        assert_ne!(api_remote_buckets[0].needles, api_remote_buckets[1].needles);
    }

    #[test]
    fn parse_omitted_branches_default_to_empty() {
        let value = json!({
            "msg_buckets": [
                { "kind": "configuration", "needles": ["sign in", "401"] }
            ]
        });
        let vocab = parse_error_vocabulary("antigravity", &value).unwrap();
        assert!(vocab.kind_buckets.is_empty());
        assert!(vocab.code_buckets.is_empty());
        assert_eq!(vocab.msg_buckets.len(), 1);
    }

    #[test]
    fn parse_preserves_code_bucket_order_and_negative_codes() {
        let value = json!({
            "msg_buckets": [
                { "kind": "api_remote", "needles": ["rate limit"] }
            ],
            "code_buckets": [
                { "code": -32004, "kind": "configuration" },
                { "code": -32005, "kind": "api_remote" },
                { "code": -32700, "kind": "agent_native" }
            ]
        });
        let vocab = parse_error_vocabulary("kimi", &value).unwrap();
        let codes: Vec<i64> = vocab.code_buckets.iter().map(|c| c.code).collect();
        assert_eq!(codes, [-32004, -32005, -32700]);
        assert_eq!(vocab.code_buckets[0].kind, "configuration");
        assert_eq!(vocab.code_buckets[2].kind, "agent_native");
    }

    #[test]
    fn research_projection_drops_provenance_and_preserves_order() {
        let vocab = parse_declared_vocabulary(
            "codex",
            &research_vocab(),
            VocabularySource::Research,
        )
        .unwrap();
        assert_eq!(vocab.kind_buckets[0].kind, "api_remote");
        assert_eq!(vocab.kind_buckets[0].needles, ["rate", "overloaded"]);
        assert_eq!(vocab.msg_buckets[0].needles, ["api key"]);
        assert_eq!(
            vocab.code_buckets,
            [
                CodeBucket { code: -32700, kind: "agent_native".into() },
                CodeBucket { code: -32600, kind: "agent_native".into() },
            ]
        );
    }

    #[test]
    fn parse_rejects_unknown_top_level_key() {
        let value = json!({ "msg_buckets": [], "typo_buckets": [] });
        assert!(parse_error_vocabulary("codex", &value).is_err());
    }

    #[test]
    fn resolve_facts_only_returns_the_facts_value() {
        let facts = facts_vocab();
        let got = resolve_source("codex", Some(&facts), None, VocabularySource::Facts).unwrap();
        assert_eq!(got, Some(&facts));
    }

    #[test]
    fn resolve_research_only_returns_the_research_value() {
        let research = facts_vocab();
        let got =
            resolve_source("codex", None, Some(&research), VocabularySource::Research).unwrap();
        assert_eq!(got, Some(&research));
    }

    #[test]
    fn resolve_missing_source_is_none() {
        let got = resolve_source("goose", None, None, VocabularySource::Facts).unwrap();
        assert!(got.is_none());
    }

    #[test]
    fn resolve_facts_plus_research_collides() {
        let facts = facts_vocab();
        let research = facts_vocab();
        let err = resolve_source("codex", Some(&facts), Some(&research), VocabularySource::Facts)
            .unwrap_err();
        assert!(matches!(err, GenError::VocabularyCollision { .. }));
    }

    #[test]
    fn resolve_graduated_but_facts_not_deleted_collides() {
        // Phase C posture: declared source is Research, but a stale facts entry
        // remains — the delete-on-graduate guard must fire.
        let facts = facts_vocab();
        let research = facts_vocab();
        let err =
            resolve_source("codex", Some(&facts), Some(&research), VocabularySource::Research)
                .unwrap_err();
        match err {
            GenError::VocabularyCollision {
                declared,
                offending,
                ..
            } => {
                assert_eq!(declared, "research");
                assert_eq!(offending, "facts");
            }
            other => panic!("expected VocabularyCollision, got {other:?}"),
        }
    }

    #[test]
    fn graduated_loader_rejects_a_stale_facts_entry() {
        let inputs = ProviderInputs {
            slug: "codex".into(),
            roster: json!({}),
            facts: [(FACTS_KEY.into(), facts_vocab())].into_iter().collect(),
            research: [(RESEARCH_TOPIC.into(), research_vocab())]
                .into_iter()
                .collect(),
            sidecars: Default::default(),
            overrides: Default::default(),
        };
        let err = load_error_vocabulary(&inputs).unwrap_err();
        assert!(matches!(
            err,
            GenError::VocabularyCollision {
                declared,
                offending,
                ..
            } if declared == "research" && offending == "facts"
        ));
    }

    // -- generation-stage validation + emission ----------------------------

    fn bucket(kind: &str, needles: &[&str]) -> KeywordBucket {
        KeywordBucket {
            kind: kind.to_string(),
            needles: needles.iter().map(|n| n.to_string()).collect(),
        }
    }

    /// A minimal valid message-only vocabulary (the parser-backed baseline).
    fn valid_vocab() -> ErrorVocabulary {
        ErrorVocabulary {
            kind_buckets: vec![bucket("api_remote", &["rate"])],
            msg_buckets: vec![bucket("configuration", &["api key"])],
            code_buckets: vec![],
        }
    }

    fn gen_message(result: Result<(), GenError>) -> String {
        match result.unwrap_err() {
            GenError::VocabularyGenInvalid { message, .. } => message,
            other => panic!("expected VocabularyGenInvalid, got {other:?}"),
        }
    }

    #[test]
    fn validate_accepts_a_clean_vocabulary() {
        assert!(validate_vocabulary("codex", &valid_vocab()).is_ok());
    }

    #[test]
    fn validate_rejects_missing_message_vocabulary() {
        let mut vocab = valid_vocab();
        vocab.msg_buckets.clear();
        let message = gen_message(validate_vocabulary("codex", &vocab));
        assert!(message.contains("msg_buckets"), "{message}");
    }

    #[test]
    fn validate_rejects_unknown_semantic_kind() {
        let mut vocab = valid_vocab();
        vocab.kind_buckets = vec![bucket("teleporting", &["zap"])];
        let message = gen_message(validate_vocabulary("codex", &vocab));
        assert!(message.contains("unknown semantic kind"), "{message}");
        assert!(message.contains("kind_buckets[0]"), "{message}");
    }

    #[test]
    fn validate_rejects_uppercase_needle() {
        let mut vocab = valid_vocab();
        vocab.msg_buckets = vec![bucket("api_remote", &["Rate Limit"])];
        let message = gen_message(validate_vocabulary("codex", &vocab));
        assert!(message.contains("must be lowercase"), "{message}");
        assert!(message.contains("Rate Limit"), "{message}");
    }

    #[test]
    fn validate_rejects_empty_or_whitespace_needle() {
        let mut vocab = valid_vocab();
        vocab.msg_buckets = vec![bucket("api_remote", &["   "])];
        let message = gen_message(validate_vocabulary("codex", &vocab));
        assert!(message.contains("whitespace"), "{message}");
    }

    #[test]
    fn validate_rejects_empty_bucket() {
        let mut vocab = valid_vocab();
        vocab.kind_buckets = vec![bucket("api_remote", &[])];
        let message = gen_message(validate_vocabulary("codex", &vocab));
        assert!(message.contains("no needles"), "{message}");
    }

    #[test]
    fn validate_rejects_duplicate_numeric_code() {
        let mut vocab = valid_vocab();
        vocab.code_buckets = vec![
            CodeBucket { code: -32004, kind: "configuration".into() },
            CodeBucket { code: -32004, kind: "api_remote".into() },
        ];
        let message = gen_message(validate_vocabulary("kimi", &vocab));
        assert!(message.contains("duplicate numeric code -32004"), "{message}");
    }

    #[test]
    fn validate_rejects_out_of_range_code() {
        let mut vocab = valid_vocab();
        vocab.code_buckets =
            vec![CodeBucket { code: i64::from(i32::MAX) + 1, kind: "agent_native".into() }];
        let message = gen_message(validate_vocabulary("kimi", &vocab));
        assert!(message.contains("does not fit in i32"), "{message}");
    }

    #[test]
    fn validate_rejects_unknown_kind_in_code_bucket() {
        let mut vocab = valid_vocab();
        vocab.code_buckets =
            vec![CodeBucket { code: -32000, kind: "teleporting".into() }];
        let message = gen_message(validate_vocabulary("kimi", &vocab));
        assert!(message.contains("unknown semantic kind"), "{message}");
    }

    #[test]
    fn emit_preserves_repeated_kind_and_bucket_order() {
        let vocab = ErrorVocabulary {
            kind_buckets: vec![
                bucket("api_remote", &["rate", "quota"]),
                bucket("configuration", &["auth"]),
                bucket("api_remote", &["api", "server"]),
            ],
            msg_buckets: vec![bucket("configuration", &["api key"])],
            code_buckets: vec![],
        };
        let table = emit_table("codex", &vocab);
        // Both api_remote rows survive as distinct, ordered lines (the "late
        // ApiRemote" second pass is not merged into the first).
        let api_rows: Vec<&str> = table
            .lines()
            .filter(|line| line.contains("SemanticErrorKind::ApiRemote"))
            .collect();
        assert_eq!(api_rows.len(), 2, "{table}");
        assert!(api_rows[0].contains("\"rate\", \"quota\""), "{table}");
        assert!(api_rows[1].contains("\"api\", \"server\""), "{table}");
    }

    #[test]
    fn emit_code_buckets_render_signed_and_ordered() {
        let vocab = ErrorVocabulary {
            kind_buckets: vec![],
            msg_buckets: vec![bucket("api_remote", &["rate"])],
            code_buckets: vec![
                CodeBucket { code: -32004, kind: "configuration".into() },
                CodeBucket { code: -32005, kind: "api_remote".into() },
            ],
        };
        let table = emit_table("kimi", &vocab);
        let idx_first = table.find("(-32004, SemanticErrorKind::Configuration)");
        let idx_second = table.find("(-32005, SemanticErrorKind::ApiRemote)");
        assert!(idx_first.is_some() && idx_second.is_some(), "{table}");
        assert!(idx_first < idx_second, "code order must be preserved: {table}");
    }

    #[test]
    fn emit_file_ends_in_exactly_one_trailing_newline() {
        let tables = vec![
            ("codex", "Codex", valid_vocab()),
            ("goose", "Goose", ErrorVocabulary::default()),
        ];
        let file = emit_file(&tables);
        assert!(file.ends_with('\n'), "must end with a newline");
        assert!(!file.ends_with("\n\n"), "must not end with a blank line");
        // Providers emit in the given order, and the accessor is exhaustive
        // over them.
        let codex_at = file.find("CODEX_VOCABULARY").expect("codex table");
        let goose_at = file.find("GOOSE_VOCABULARY").expect("goose table");
        assert!(codex_at < goose_at, "table order must follow the input order");
        assert!(file.contains("Provider::Codex => &CODEX_VOCABULARY,"));
        assert!(file.contains("Provider::Goose => &GOOSE_VOCABULARY,"));
        // Goose is research-only and therefore carries no executable input line.
        assert!(file.contains(
            "docs/research/agent-errors/codex.md (ordered vocabulary projection)"
        ));
        assert!(!file.contains("docs/research/agent-errors/goose.md"));
    }
}
