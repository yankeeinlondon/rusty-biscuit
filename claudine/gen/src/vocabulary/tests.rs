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
