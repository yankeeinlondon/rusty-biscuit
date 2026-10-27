//! `hook.<path>` reads the requested member off the event; only bare `hook`
//! builds the whole payload object.

use super::*;

/// One megabyte of text, the size at which a whole-payload copy dominated.
fn large_text() -> String {
    "x".repeat(1024 * 1024)
}

/// The sample event with a large `tool_response` beside small members, an
/// array, and explicit `null` values.
fn meta_with_large_response() -> EventMeta {
    let mut meta = sample_meta();
    meta.tool_response = Some(json!({
        "status": "ok",
        "items": [1, 2, 3],
        "nothing": null,
        "body": large_text(),
    }));
    meta.extra.insert("explicit_null".to_string(), Value::Null);
    meta
}

/// Paths compared against the whole-object projection: scalars, nested
/// members, arrays, absent and `null` values, the environment groups and
/// their omitted fields, malformed segments, and reserved or unknown roots.
const PATHS: &[&str] = &[
    "provider",
    "event",
    "timestamp",
    "session_id",
    "cwd",
    "tool_name",
    "tool_name.length",
    "error",
    "prompt",
    "tool_input",
    "tool_input.command",
    "tool_input.options.cwd",
    "tool_input.options.missing",
    "tool_response",
    "tool_response.status",
    "tool_response.items",
    "tool_response.items.0",
    "tool_response.nothing",
    "tool_response.body",
    "tool_response.status.deeper",
    "extra",
    "extra.status",
    "extra.tags",
    "extra.nested.inner.depth",
    "extra.nested.missing",
    "extra.explicit_null",
    "extra.missing",
    "os",
    "os.name",
    "os.kernel",
    "os.name.deeper",
    "hardware",
    "hardware.cores",
    "git",
    "git.branch",
    "git.is_dirty",
    "git.user_email",
    "git.repo_root",
    "project",
    "project.language",
    "project.monorepo_tool",
    "project.monorepo_standard",
    "env",
    "env.PATH",
    "env.HOME",
    "ctx",
    "ctx.today",
    "seq",
    "loop",
    "fm",
    "doc",
    "status",
    "attempt",
    "missing",
    "",
    "tool_response.",
    "extra.",
    ".",
];

fn assert_matches_projection(meta: &EventMeta) {
    let lookup = EventMetaExpressionLookup::new(meta);
    let whole = lookup.get("hook").expect("bare hook resolves");
    for path in PATHS {
        assert_eq!(
            lookup.get(&format!("hook.{path}")),
            nested_pointer(&whole, path),
            "`hook.{path}` differs from the whole payload's value at `{path}`"
        );
    }
}

#[test]
fn hook_leaf_reads_equal_the_whole_payload_projection() {
    assert_matches_projection(&meta_with_large_response());
}

#[test]
fn hook_leaf_reads_equal_the_projection_when_optional_members_are_missing() {
    let mut meta = sample_meta();
    meta.tool_input = Some(Value::Null);
    meta.tool_response = None;
    meta.tool_name = None;
    meta.extra.clear();
    meta.env.git = None;
    meta.env.repo = None;
    meta.env.primary_language = None;

    assert_matches_projection(&meta);
    let lookup = EventMetaExpressionLookup::new(&meta);
    assert_eq!(lookup.get("hook.tool_input"), Some(Value::Null));
    assert_eq!(lookup.get("hook.tool_response"), None);
    assert_eq!(lookup.get("hook.git"), None);
    assert_eq!(lookup.get("hook.project"), None);
    assert_eq!(lookup.get("hook.extra"), Some(json!({})));
}

#[test]
fn hook_env_stays_absent_and_a_missing_member_reads_no_other_root() {
    let key = "CLAUDINE_HOOK_LEAF_READ_ENV_VAR";
    // SAFETY: the variable is unique to this test and removed before it ends.
    unsafe {
        std::env::set_var(key, "present");
    }
    let mut meta = sample_meta();
    // Same-named values reachable unprefixed or under another root.
    meta.extra.insert("branch".to_string(), json!("from-extra"));
    meta.tool_input = Some(json!({"exit_code": 9}));
    let lookup = EventMetaExpressionLookup::new(&meta);

    assert_eq!(lookup.get(&format!("env.{key}")), Some(json!("present")));
    assert_eq!(lookup.get(&format!("hook.env.{key}")), None);
    assert_eq!(lookup.get("hook.env"), None);
    // `status` and `branch` resolve under `extra` and `git`; `command` under
    // `tool_input`. None of them is a top-level member of `hook`.
    assert_eq!(lookup.get("extra.status"), Some(json!("success")));
    assert_eq!(lookup.get("hook.status"), None);
    assert_eq!(lookup.get("hook.branch"), None);
    assert_eq!(lookup.get("hook.command"), None);
    assert_eq!(lookup.get("hook.tool_response.command"), None);
    assert_eq!(
        parse_and_evaluate("hook.status", ParseMode::Interpolation, &lookup),
        Value::Null
    );

    unsafe {
        std::env::remove_var(key);
    }
}

#[test]
fn hook_leaf_reads_see_each_event_current_values() {
    let mut meta = sample_meta();
    meta.tool_response = Some(json!({"status": "first"}));
    let first = parse_and_evaluate(
        "hook.tool_response.status",
        ParseMode::Interpolation,
        &EventMetaExpressionLookup::new(&meta),
    );

    meta.tool_name = Some("Edit".to_string());
    meta.tool_response = Some(json!({"status": "second"}));
    let lookup = EventMetaExpressionLookup::new(&meta);

    assert_eq!(first, json!("first"));
    assert_eq!(
        parse_and_evaluate(
            "hook.tool_response.status",
            ParseMode::Interpolation,
            &lookup
        ),
        json!("second")
    );
    assert_eq!(lookup.get("hook.tool_name"), Some(json!("Edit")));
    assert_eq!(
        lookup
            .get("hook")
            .and_then(|hook| hook.get("tool_name").cloned()),
        Some(json!("Edit"))
    );
}

#[cfg(feature = "work-counters")]
mod work_counts {
    use super::*;
    use crate::work_counters::{Work, snapshot};

    fn materializations(read: impl FnOnce()) -> u64 {
        let before = snapshot();
        read();
        snapshot().since(&before).get(Work::HookPayloadMaterialize)
    }

    #[test]
    fn hook_leaf_reads_never_build_the_payload_beside_a_large_response() {
        let meta = meta_with_large_response();
        let lookup = EventMetaExpressionLookup::new(&meta);

        for path in [
            "hook.tool_name",
            "hook.tool_response.status",
            "hook.extra.status",
            "hook.git.branch",
            "hook.os.name",
            "hook.missing",
        ] {
            assert_eq!(materializations(|| drop(lookup.get(path))), 0, "`{path}`");
        }
        // A large requested leaf is copied, but the payload still is not built.
        let mut body = None;
        assert_eq!(
            materializations(|| body = lookup.get("hook.tool_response.body")),
            0
        );
        assert_eq!(body, Some(Value::String(large_text())));

        let rendered = crate::dispatch::template::interpolate(
            "{{ hook.tool_name }} {{ hook.tool_response.status }}",
            &meta,
        );
        assert_eq!(rendered, "Bash ok");
        assert_eq!(
            materializations(|| {
                crate::dispatch::template::interpolate(
                    "{{ hook.tool_name }} {{ hook.tool_response.status }}",
                    &meta,
                );
            }),
            0
        );
    }

    #[test]
    fn a_whole_hook_read_builds_the_payload_once() {
        let meta = meta_with_large_response();
        let lookup = EventMetaExpressionLookup::new(&meta);

        let mut whole = None;
        assert_eq!(materializations(|| whole = lookup.get("hook")), 1);
        assert!(whole.is_some_and(|hook| hook.get("tool_response").is_some()));
    }
}
