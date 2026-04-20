# Plan 3: Sink Hardening, Gemini Noise Suppression, OpenCode Stderr Filter

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.
>
> **Prerequisite:** Plan 1 (hook hang) and Plan 2 (Codex + Claude parser gap) should both have shipped before Task 4's manual verification. Tasks 1–3 can run in parallel with Plan 2.

**Goal:** Eliminate the last three categories of bad non-interactive UX identified in the 2026-04-14 captures: (a) `summarize_provider_payload` dumping raw JSON at 80 chars when a `ProviderExtension` has no recognizable top-level string key; (b) Gemini echoing the user's own prompt as a `message.non_assistant` status line; (c) OpenCode's default-mode TUI output (`✱ Glob …`, bare `$ cd …` blocks) leaking through stderr even when `--format json` is set. Each fix is narrow and isolated from Plans 1 and 2.

**Architecture:** Three changes, each self-contained. The sink fallback grows a conservative nested-text walker and a short hard-coded allowlist of kinds that render as `provider/kind` only (no `· payload` tail) when no human-readable summary can be produced; user-prompt echoes from Gemini stop emitting a `ProviderExtension` and are dropped silently at the parser; the wrap stderr reader adds a new noise-prefix set for OpenCode that matches its default-mode TUI formatter lines so they are suppressed when running `--format json`. Together they remove the "raw JSON on stderr" failure mode entirely while preserving the no-drop fidelity invariant — events still flow to the JSONL semantic log, they just stop visually polluting the status line.

**Tech Stack:** Rust, claudine `live_semantic_sink`, `gemini_semantic`, `wrap/exec.rs` stderr reader, claudine/biscuit-terminal rendering.

**Captured evidence (2026-04-14):**
- `claudine-output/gemini.err` shows the Gemini parser rendering is actually excellent for tool calls (`→ read_file · path` / `← read_file · success`), but the surrounding `message/non_assistant` events produce user-prompt echoes.
- `claudine-output/opencode.err` tail shows `✱ Glob "**/..." 2 matches` and `$ cd ... && git log ...` bare shell output interleaved with claudine status lines. Those are OpenCode's default formatter leaking through stderr because `opencode run --format json` still writes status to stderr.
- `claudine-output/codex.err` and earlier screenshots showed `codex/item.* · {"item":{...}}"` raw-JSON truncations. Plan 2 removes the events from ProviderExtension entirely, but Plan 3 adds a defense-in-depth fallback in case future wire-format drift routes events through the catch-all again.

---

## File Map

| File | Change | Purpose |
|------|--------|---------|
| `claudine/cli/src/commands/wrap/live_semantic_sink.rs` | Modify | Replace `summarize_provider_payload` raw-JSON fallback with a nested-text walker; add hard-coded "silent kinds" allowlist where the status line renders `provider/kind` with no tail; stop rendering `provider/kind` entirely for kinds in a "fully suppressed" set |
| `claudine/lib/src/stream/gemini_semantic.rs` | Modify | Stop emitting `ProviderExtension` for `message.non_assistant`; drop those events silently (fidelity preserved via `extra["semantic_event"]` on other events and via the raw JSONL log) |
| `claudine/cli/src/commands/wrap/exec.rs` | Modify | Extend OpenCode wrapper's stderr noise-prefix list with patterns matching OpenCode's default-mode TUI lines (`✱ `, `$ `, `> build · `, `████ `) so they're suppressed when running `--format json` |
| `claudine/cli/src/commands/wrap/live_semantic_sink.rs` | Modify (tests) | Property test: for every fixture in `lib/tests/fixtures/providers/`, asserting that no line rendered to stderr contains a raw JSON fragment (`{"` heuristic) |

---

## Preconditions

- [ ] **Step 0: Clean tree**

Run: `git status --short`
Expected: no pending modifications under `claudine/cli/src/commands/wrap/` or `claudine/lib/src/stream/gemini_semantic.rs`.

- [ ] **Step 0.1: Plan 2 fixtures available**

The tests in this plan use the `claudine/lib/tests/fixtures/providers/` directory created by Plan 2 Task 1. If Plan 2 hasn't shipped yet, create the directory and populate at minimum `codex.ndjson` and `claude.ndjson` from `claudine/agent-output/` first (the same copy step from Plan 2 Task 1). Also add `gemini.ndjson` and `opencode.ndjson`:

```bash
mkdir -p claudine/lib/tests/fixtures/providers
cp claudine/agent-output/gemini.out    claudine/lib/tests/fixtures/providers/gemini.ndjson
cp claudine/agent-output/opencode.out  claudine/lib/tests/fixtures/providers/opencode.ndjson
```

Commit as a separate prerequisite:

```bash
git add claudine/lib/tests/fixtures/providers/gemini.ndjson \
        claudine/lib/tests/fixtures/providers/opencode.ndjson
git commit -m "test(claudine): add Gemini + OpenCode wire-format fixtures"
```

Skip this step if Plan 2 already added all four providers' fixtures.

---

## Task 1: Gemini — drop `message.non_assistant` silently

**Files:**
- Modify: `claudine/lib/src/stream/gemini_semantic.rs`

- [ ] **Step 1: Write the failing test**

Append to `claudine/lib/src/stream/gemini_semantic.rs` tests module:

```rust
    #[test]
    fn gemini_non_assistant_message_emits_no_provider_extension() {
        use std::sync::{Arc, Mutex};
        use crate::stream::semantic::{SemanticEvent, SemanticEventSink};
        use crate::stream::parser::SemanticStreamParser;

        #[derive(Default)]
        struct Capture(Arc<Mutex<Vec<SemanticEvent>>>);
        impl SemanticEventSink for Capture {
            fn on_semantic_event(&mut self, event: SemanticEvent) {
                self.0.lock().unwrap().push(event);
            }
        }

        let events = Arc::new(Mutex::new(Vec::new()));
        let mut parser = GeminiSemanticStreamParser::new(Capture(events.clone()), None);

        // User-role message — Gemini emits these for the operator's own
        // prompt. They are always noise on stderr and should be dropped
        // silently here.
        parser.feed_line(
            r#"{"type":"message","content":"Hi how are you?","role":"user","timestamp":"2026-04-14T00:00:00Z"}"#,
        ).unwrap();

        let captured = events.lock().unwrap().clone();
        assert!(
            !captured.iter().any(|e| matches!(
                e,
                SemanticEvent::ProviderExtension { kind, .. } if kind == "message.non_assistant"
            )),
            "non-assistant messages must be dropped silently, got {captured:?}"
        );
    }

    #[test]
    fn gemini_assistant_message_still_routes_to_output_text() {
        use std::sync::{Arc, Mutex};
        use crate::stream::semantic::{SemanticEvent, SemanticEventSink};
        use crate::stream::parser::SemanticStreamParser;

        #[derive(Default)]
        struct Capture(Arc<Mutex<Vec<SemanticEvent>>>);
        impl SemanticEventSink for Capture {
            fn on_semantic_event(&mut self, event: SemanticEvent) {
                self.0.lock().unwrap().push(event);
            }
        }

        let events = Arc::new(Mutex::new(Vec::new()));
        let mut parser = GeminiSemanticStreamParser::new(Capture(events.clone()), None);

        parser.feed_line(
            r#"{"type":"message","content":"response text","role":"assistant"}"#,
        ).unwrap();

        let captured = events.lock().unwrap().clone();
        assert!(
            captured.iter().any(|e| matches!(e, SemanticEvent::OutputText { .. })),
            "assistant message must still route to OutputText"
        );
    }
```

- [ ] **Step 2: Run to verify failure**

Run: `cargo test -p claudine --lib stream::gemini_semantic::tests::gemini_non_assistant_message_emits_no_provider_extension`
Expected: FAIL — current code emits a `ProviderExtension` with `kind = "message.non_assistant"`.

- [ ] **Step 3: Drop non-assistant messages silently**

In `claudine/lib/src/stream/gemini_semantic.rs`, find `handle_message` (around line 94). Change:

```rust
fn handle_message(&mut self, msg: GeminiMessage, raw_kind: &str, raw: Value) {
    if msg.role.as_deref() != Some("assistant") {
        // Non-assistant messages (user, system) are preserved but do not
        // flow through OutputText.
        self.sink.on_semantic_event(SemanticEvent::ProviderExtension {
            provider: Provider::Gemini,
            kind: "message.non_assistant".into(),
            payload: raw,
        });
        return;
    }
    // ...
}
```

to:

```rust
fn handle_message(&mut self, msg: GeminiMessage, raw_kind: &str, _raw: Value) {
    if msg.role.as_deref() != Some("assistant") {
        // Non-assistant messages are the operator's own prompt being
        // replayed back into the stream. They're always noise on stderr,
        // so drop silently. Fidelity is preserved via the raw JSONL log
        // path, which writes the full raw line independently of the
        // semantic event surface.
        return;
    }
    let Some(text) = msg.resolved_text() else {
        return;
    };
    self.assistant_text.push_str(&text);
    self.sink.on_semantic_event(SemanticEvent::OutputText {
        text: super::ensure_message_newline(text),
        extra: Value::Object(self.base_extra(raw_kind)),
    });
}
```

Note: the `raw: Value` parameter is kept as `_raw` rather than removed outright — the call site in `feed_line` passes it unconditionally and trimming the signature would ripple into the caller's match arms. Preserving the parameter keeps the patch surgical.

There is also a secondary branch in `gemini_semantic.rs` (around line 435) that matches on `SemanticEvent::ProviderExtension { ref kind, .. } if kind == "message.non_assistant"` — check whether that branch still has a useful effect after the parser stops emitting that kind. If it doesn't (e.g., it only existed to suppress rendering), remove the dead code in the same commit.

- [ ] **Step 4: Run both new tests**

Run: `cargo test -p claudine --lib stream::gemini_semantic::tests::gemini_non_assistant_message_emits_no_provider_extension stream::gemini_semantic::tests::gemini_assistant_message_still_routes_to_output_text`
Expected: both PASS.

- [ ] **Step 5: Run the full gemini_semantic test suite**

Run: `cargo test -p claudine --lib stream::gemini_semantic`
Expected: all tests pass including pre-existing ones.

- [ ] **Step 6: Commit**

```bash
git add claudine/lib/src/stream/gemini_semantic.rs
git commit -m "fix(claudine): drop Gemini non-assistant messages silently

Gemini emits message events for the operator's own prompt (role=user)
and for system messages. The parser used to emit those as
ProviderExtension events, which the live sink would then render as
status lines echoing the user's prompt back at them — always noise.
Drop silently at the parser. Fidelity is preserved via the raw JSONL
log path, which writes each raw line independently of the semantic
event surface."
```

---

## Task 2: Sink fallback hardening — no raw JSON in status lines

**Files:**
- Modify: `claudine/cli/src/commands/wrap/live_semantic_sink.rs`

- [ ] **Step 1: Write the failing tests**

Append to `claudine/cli/src/commands/wrap/live_semantic_sink.rs` tests module:

```rust
    #[test]
    fn provider_extension_with_only_nested_text_renders_summary_not_raw_json() {
        let lines = Arc::new(StdMutex::new(Vec::new()));
        let dispatched = Arc::new(StdMutex::new(Vec::new()));
        let mut sink = make_sink(lines.clone(), dispatched.clone());

        // Payload has no top-level message/status/name/path, but nested text.
        sink.on_semantic_event(SemanticEvent::ProviderExtension {
            provider: Provider::Codex,
            kind: "future.unknown".into(),
            payload: json!({
                "item": {
                    "content": { "parts": [ { "text": "meaningful text here" } ] }
                }
            }),
        });

        let rendered = lines.lock().unwrap().join("\n");
        assert!(
            rendered.contains("meaningful text here"),
            "expected nested text preview in stderr: {rendered}"
        );
        assert!(
            !rendered.contains(r#"{"item":"#),
            "raw JSON must not appear in stderr: {rendered}"
        );
    }

    #[test]
    fn provider_extension_unresolvable_drops_payload_tail_entirely() {
        let lines = Arc::new(StdMutex::new(Vec::new()));
        let dispatched = Arc::new(StdMutex::new(Vec::new()));
        let mut sink = make_sink(lines.clone(), dispatched.clone());

        sink.on_semantic_event(SemanticEvent::ProviderExtension {
            provider: Provider::Codex,
            kind: "opaque.event".into(),
            payload: json!({
                "some_numeric_field": 42,
                "another": [1, 2, 3]
            }),
        });

        let rendered = lines.lock().unwrap().join("\n");
        assert!(
            rendered.contains("codex/opaque.event"),
            "provider/kind label must still appear: {rendered}"
        );
        assert!(
            !rendered.contains(r#"{"some_numeric_field":"#) && !rendered.contains("42"),
            "raw payload must not appear when no human-readable summary is available: {rendered}"
        );
        // No `·` separator when there is no summary to append.
        assert!(
            !rendered.contains(" \u{00b7} {"),
            "must not render the summary separator followed by raw JSON: {rendered}"
        );
    }

    #[test]
    fn provider_extension_respects_silent_kind_allowlist() {
        let lines = Arc::new(StdMutex::new(Vec::new()));
        let dispatched = Arc::new(StdMutex::new(Vec::new()));
        let mut sink = make_sink(lines.clone(), dispatched.clone());

        // Kinds in the silent allowlist must produce NO stderr line at all
        // (they still get dispatched and logged, just not rendered).
        sink.on_semantic_event(SemanticEvent::ProviderExtension {
            provider: Provider::Claude,
            kind: "stream_event".into(), // per CLAUDE.md research: partial message deltas; always noisy
            payload: json!({ "delta": "chunk" }),
        });

        let rendered = lines.lock().unwrap().join("\n");
        assert!(
            !rendered.contains("claude/stream_event"),
            "silent-kind allowlist must suppress the status line entirely: {rendered}"
        );
    }
```

- [ ] **Step 2: Run to verify failures**

Run: `cargo test -p claudine-cli commands::wrap::live_semantic_sink::tests::provider_extension_with_only_nested_text_renders_summary_not_raw_json commands::wrap::live_semantic_sink::tests::provider_extension_unresolvable_drops_payload_tail_entirely commands::wrap::live_semantic_sink::tests::provider_extension_respects_silent_kind_allowlist`
Expected: all FAIL with the current `summarize_provider_payload` + `render_event` behavior.

- [ ] **Step 3: Rewrite `summarize_provider_payload`**

In `claudine/cli/src/commands/wrap/live_semantic_sink.rs`, replace `summarize_provider_payload`:

```rust
/// Produce a terse one-line human summary of a ProviderExtension payload.
///
/// Returns `None` when no summary can be derived from known nested shapes —
/// callers must render `provider/kind` WITHOUT a trailing ` · <payload>` in
/// that case rather than falling back to raw JSON. This is a deliberate UX
/// trade-off: a bare `provider/kind` is less informative but still readable,
/// whereas a truncated raw JSON blob is actively harmful.
fn summarize_provider_payload(payload: &Value) -> Option<String> {
    // Known text locations, in descending specificity.
    let known_paths: &[&[&str]] = &[
        &["message"],
        &["status"],
        &["name"],
        &["path"],
        &["text"],
        &["content"],
        &["error", "message"],
        &["error_message"],
        &["title"],
        &["description"],
    ];

    if let Some(obj) = payload.as_object() {
        for path in known_paths {
            let mut cursor: &Value = payload;
            let mut resolved: Option<&str> = None;
            for segment in path.iter() {
                if let Some(next) = cursor.get(*segment) {
                    cursor = next;
                    if let Some(s) = cursor.as_str() {
                        resolved = Some(s);
                    }
                } else {
                    resolved = None;
                    break;
                }
            }
            if let Some(s) = resolved.filter(|s| !s.is_empty()) {
                return Some(truncate(s, 80));
            }
        }

        // Nested content arrays: message.content[*].text, item.content.parts[*].text
        for nested_array_path in [
            &["message", "content"][..],
            &["item", "content", "parts"][..],
            &["content", "parts"][..],
            &["parts"][..],
        ] {
            let mut cursor: &Value = payload;
            for seg in nested_array_path.iter() {
                match cursor.get(*seg) {
                    Some(next) => cursor = next,
                    None => { cursor = &Value::Null; break; }
                }
            }
            if let Some(array) = cursor.as_array() {
                for elem in array {
                    if let Some(text) = elem.get("text").and_then(Value::as_str).filter(|s| !s.is_empty()) {
                        return Some(truncate(text, 80));
                    }
                    if let Some(text) = elem.get("content").and_then(Value::as_str).filter(|s| !s.is_empty()) {
                        return Some(truncate(text, 80));
                    }
                }
            }
        }

        // Top-level string values that aren't a known path. Use the first
        // non-empty string value as a last resort.
        for (_, v) in obj.iter() {
            if let Some(s) = v.as_str().filter(|s| !s.is_empty()) {
                return Some(truncate(s, 80));
            }
        }
    }

    // No raw-JSON fallback. Returning None signals to the caller to render
    // provider/kind without a trailing payload tail.
    None
}
```

- [ ] **Step 4: Update `provider_extension_description` to honor the new `None` semantics**

Current:

```rust
fn provider_extension_description(provider: Provider, kind: &str, payload: &Value) -> String {
    let summary = summarize_provider_payload(payload);
    match summary {
        Some(s) => format!("{}/{kind} \u{00b7} {s}", provider_short(provider)),
        None => format!("{}/{kind}", provider_short(provider)),
    }
}
```

The shape is already correct for the new `None` path — no change needed. Verify during review.

- [ ] **Step 5: Add a silent-kind allowlist in `render_event`**

In `claudine/cli/src/commands/wrap/live_semantic_sink.rs`, find the `ProviderExtension` arm in `render_event`. Add a suppression list:

```rust
/// Kinds that are known to be high-volume or entirely redundant on stderr.
/// Listed here explicitly rather than relying on summary heuristics so the
/// suppression is visible, reviewable, and reversible.
const SILENT_PROVIDER_EXTENSION_KINDS: &[(Provider, &str)] = &[
    // Claude: partial assistant token deltas — redundant with OutputText.
    (Provider::Claude, "stream_event"),
    // Claude: hook lifecycle is already surfaced via Info events in Plan 2.
    (Provider::Claude, "hook_started"),
    (Provider::Claude, "hook_response"),
    (Provider::Claude, "hook_progress"),
    // Add new entries as real traffic reveals noise categories.
];

fn is_silent_extension_kind(provider: Provider, kind: &str) -> bool {
    SILENT_PROVIDER_EXTENSION_KINDS
        .iter()
        .any(|(p, k)| *p == provider && *k == kind)
}
```

In `render_event`'s `ProviderExtension` arm:

```rust
            SemanticEvent::ProviderExtension {
                provider,
                kind,
                payload,
            } => {
                if is_silent_extension_kind(*provider, kind) {
                    // Suppress stderr rendering; the event still flows
                    // through to dispatch and logging.
                    return;
                }
                self.render_status(
                    StatusState::Info,
                    Self::provider_extension_description(*provider, kind, payload),
                );
            }
```

Place `is_silent_extension_kind` as an associated `fn` on `LiveSemanticSink` or as a free `fn` in the same module, whichever matches the existing style (mirror `provider_short` and `summarize_provider_payload` placement).

- [ ] **Step 6: Run the three new tests**

Run: `cargo test -p claudine-cli commands::wrap::live_semantic_sink::tests::provider_extension_with_only_nested_text_renders_summary_not_raw_json commands::wrap::live_semantic_sink::tests::provider_extension_unresolvable_drops_payload_tail_entirely commands::wrap::live_semantic_sink::tests::provider_extension_respects_silent_kind_allowlist`
Expected: all PASS.

- [ ] **Step 7: Run full wrap sink suite**

Run: `cargo test -p claudine-cli commands::wrap::live_semantic_sink`
Expected: all tests pass.

- [ ] **Step 8: Property test — no raw JSON on stderr for any captured fixture**

Append to the same tests module:

```rust
    #[test]
    fn no_captured_fixture_ever_renders_raw_json_on_stderr() {
        use std::path::Path;

        let fixtures_dir = Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("..")
            .join("lib")
            .join("tests")
            .join("fixtures")
            .join("providers");

        assert!(fixtures_dir.exists(), "fixtures dir must exist: {fixtures_dir:?}");

        for provider_slug in &["claude", "codex", "gemini", "opencode"] {
            let fixture = fixtures_dir.join(format!("{provider_slug}.ndjson"));
            if !fixture.exists() {
                continue; // optional fixtures
            }
            let provider = match *provider_slug {
                "claude" => Provider::Claude,
                "codex" => Provider::Codex,
                "gemini" => Provider::Gemini,
                "opencode" => Provider::OpenCode,
                _ => unreachable!(),
            };
            let fixture_lines: Vec<String> = std::fs::read_to_string(&fixture)
                .expect("read fixture")
                .lines()
                .map(String::from)
                .collect();

            let lines_ref: Vec<&str> = fixture_lines.iter().map(String::as_str).collect();
            let stderr_lines = super::super::super::live_semantic_sink::tests::golden_stderr::replay_to_stderr(
                provider,
                &lines_ref,
                None,
            );

            for line in &stderr_lines {
                assert!(
                    !line.contains(r#"{"#) || !line.contains(r#"":"#),
                    "provider={provider_slug}: stderr line contains raw JSON: {line:?}"
                );
            }
        }
    }
```

NOTE: the test reaches into `super::super::super::live_semantic_sink::tests::golden_stderr::replay_to_stderr`. Adjust the path and visibility of `replay_to_stderr` as needed — promote it from private to `pub(crate)` (inside the test module) if the cross-module reach fails. If promoting visibility is awkward, duplicate the replay helper into this test — 15 lines of trivial code — rather than fighting the module system.

Run: `cargo test -p claudine-cli no_captured_fixture_ever_renders_raw_json_on_stderr`
Expected: PASS. If it fails, inspect which fixture line produced the raw JSON — that either reveals a `ProviderExtension` kind that should be in the silent allowlist, or a summary path `summarize_provider_payload` should walk. Patch iteratively.

- [ ] **Step 9: Commit**

```bash
git add claudine/cli/src/commands/wrap/live_semantic_sink.rs
git commit -m "feat(claudine): harden ProviderExtension sink against raw-JSON leaks

Replaces summarize_provider_payload's JSON-serialization fallback with
a nested-text walker (message.content[*].text, item.content.parts[*].text,
error.message, etc.). When no readable summary can be produced, the
status line now renders 'provider/kind' with no tail rather than a
truncated raw JSON blob.

Adds a hard-coded silent-kind allowlist (Claude stream_event, hook_*)
that suppresses the status line entirely for high-volume or redundant
extension kinds — events still flow through dispatch and the JSONL log.

Property test guards every captured fixture against future regressions."
```

---

## Task 3: OpenCode — suppress default-mode TUI leakage in stderr

**Files:**
- Modify: `claudine/cli/src/commands/wrap/exec.rs` (OpenCode stderr noise-prefixes set) — and likely a wrap-mod `opencode.rs` or `mod.rs` call site that supplies the list.

- [ ] **Step 1: Locate the OpenCode wrapper's stderr noise-prefix configuration**

Grep for the OpenCode-specific call site that passes `stderr_noise_prefixes` into `run_child_stream_semantic` or `run_child`. Likely file: `claudine/cli/src/commands/wrap/mod.rs` near the OpenCode provider dispatch match arm. Read the surrounding code to identify the existing noise-prefix set.

- [ ] **Step 2: Write the failing test**

Append to wherever the OpenCode noise-filter logic is (file TBD from Step 1). If it's in `mod.rs`, add a targeted unit test. If it's computed inline in a large function, extract a small helper first:

```rust
/// The default-mode TUI formatter lines that OpenCode keeps emitting to
/// stderr even when `--format json` is set. Suppressed when wrapping
/// OpenCode so the JSON NDJSON stream on stdout is the only visible
/// output surface.
pub(crate) fn opencode_default_tui_noise_prefixes() -> &'static [&'static str] {
    &[
        "\u{2731} ",   // ✱  — bullet used for Glob/Grep/Read status lines
        "$ ",           // bare shell command echo lines
        "> build ",     // session banner
        "\u{2588}\u{2588}\u{2588}\u{2588} ", // ████  — subheader marker
    ]
}

#[cfg(test)]
mod opencode_noise_tests {
    use super::*;

    #[test]
    fn opencode_noise_prefixes_cover_captured_symptoms() {
        let noise = opencode_default_tui_noise_prefixes();

        // Representative lines taken verbatim from
        // claudine/claudine-output/opencode.err (2026-04-14 capture).
        let symptoms = [
            r#"✱ Glob "**/claudine/**/improved-sequences/**" 2 matches"#,
            r#"$ cd /tmp && git log --all --oneline"#,
            r#"> build · MiniMax-M2.7-highspeed"#,
            r#"████ Subprocess hygiene"#,
        ];

        for line in symptoms {
            assert!(
                noise.iter().any(|p| line.starts_with(p)),
                "noise prefixes must match representative line: {line}"
            );
        }
    }
}
```

- [ ] **Step 3: Run to verify**

Run: `cargo test -p claudine-cli opencode_noise_prefixes_cover_captured_symptoms`
Expected: FAIL because the helper doesn't exist yet.

- [ ] **Step 4: Implement the helper and wire it into the OpenCode wrapper**

Add `opencode_default_tui_noise_prefixes` as defined in Step 2. Then, at the call site that constructs the OpenCode wrap's `ChildIoOptions` / passes `stderr_noise_prefixes` into `run_child_stream_semantic`, concatenate the new prefixes with any existing OpenCode-specific prefixes:

```rust
let opencode_noise: Vec<&str> = existing_noise_prefixes
    .iter()
    .copied()
    .chain(opencode_default_tui_noise_prefixes().iter().copied())
    .collect();
// ... pass opencode_noise into run_child_stream_semantic(...)
```

- [ ] **Step 5: Run the test**

Run: `cargo test -p claudine-cli opencode_noise_prefixes_cover_captured_symptoms`
Expected: PASS.

- [ ] **Step 6: Full wrap test suite**

Run: `cargo test -p claudine-cli commands::wrap`
Expected: all tests pass.

- [ ] **Step 7: Commit**

```bash
git add claudine/cli/src/commands/wrap/mod.rs   # adjust path to match Step 1 finding
git commit -m "fix(claudine): suppress OpenCode default-mode TUI noise on stderr

OpenCode keeps emitting its default formatter lines (✱ Glob, bare
\$ shell echoes, > build banner, ████ subheader markers) to stderr
even when 'opencode run --format json' is set. Add a provider-specific
noise-prefix set to the wrap stderr reader so these lines are
suppressed when wrapping OpenCode, leaving the NDJSON on stdout as
the only visible output surface."
```

---

## Task 4: Regression sweep + post-plan capture

- [ ] **Step 1: Full workspace test run**

Run: `cargo test -p claudine -p claudine-cli`
Expected: all tests pass.

- [ ] **Step 2: Clippy delta check**

Run: `cargo clippy -p claudine -p claudine-cli --all-targets 2>&1 | grep -cE "^error:"`
Zero net increase against baseline.

- [ ] **Step 3: Post-plan capture for all four providers**

Only after Plans 1 and 2 are merged. Re-run the same prompt from the 2026-04-14 capture:

```bash
mkdir -p claudine/claudine-output/post-plan-3/
for p in claude codex gemini opencode; do
    claudine $p -p "find other versions of claudine/features/_unscheduled/improved-sequences/spec.md" -y \
        2>claudine/claudine-output/post-plan-3/$p.err \
        >claudine/claudine-output/post-plan-3/$p.out
done
```

Then grep for each pre-existing symptom; all must return nothing:

```bash
grep -rnF '{"message":{"content":[{"content":'  claudine/claudine-output/post-plan-3/ || true  # claude user-echo gone
grep -rnF 'gemini/message.non_assistant'        claudine/claudine-output/post-plan-3/ || true  # gemini user-echo gone
grep -rnE 'codex/item\.(started|completed|agent_message) \u00b7 \{' claudine/claudine-output/post-plan-3/ || true  # codex raw-JSON gone
grep -rnE '^\s*(\u2731|\$|████)' claudine/claudine-output/post-plan-3/opencode.err || true  # opencode TUI noise gone
```

Each returning no matches is the success signal.

- [ ] **Step 4: Commit the post-plan capture**

```bash
git add claudine/claudine-output/post-plan-3/
git commit -m "chore(claudine): post-plan-3 captures confirm clean UX"
```

---

## Out of Scope

- Kimi and Qwen protocol gap closure. They have minimal real-world traffic and no representative capture in the 2026-04-14 corpus. Revisit after Plan 3 ships if a user report surfaces.
- OpenCode `reasoning` / `step_start` / `step_finish` rendering changes beyond noise suppression. Currently these route to `Info` events and are fine; if they become noisy, add them to the silent-kind allowlist in Task 2's allowlist rather than changing the parser.
- Promoting `ProviderExtension` kinds to typed `SemanticEvent` variants. The tech-design graduation path exists; it's case-by-case work that doesn't belong here.
- Structured badge upgrades beyond the billing/rate-limit fix in Plan 2. Badge UX (position, color, count) is its own visual-design concern.
- A full semantic-event writer rewrite. The JSONL log path continues to be the no-drop fidelity surface; Plan 3 only changes what's RENDERED, not what's LOGGED.
