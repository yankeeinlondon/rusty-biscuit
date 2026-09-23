---
created: 2026-07-17
description: Per-site classification of the 64 `error-propagation-followup` entries in `transport-allow.toml` against spec §D8 categories, with recommended mechanisms and a batch plan.
status: draft
---

# Burn-Down Triage: the 64 `error-propagation-followup` Exceptions

Review 1 Finding 1 rates it Critical that
`cli/tests/error_guards/transport-allow.toml` freezes 64 known lossy sites under
a `error-propagation-followup` tag. This document classifies each one against
spec §D8's three categories and proposes the work to close them.

**The review's premise is confirmed.** The allowlist's blanket §D10 rationale
does not survive contact with the sources. §D10 defers *routing and retry-policy
changes*; 45 of these 64 sites are plain typed-source preservation, which is
exactly what §D1 and §D8 mandate. Exactly **one** site is a genuine §D10
deferral.

The allowlist's second stock reason — "the enclosing signature returns
`Result<_, String>`, so there is no typed slot" — is also largely unfounded. Of
the eight entries carrying it, only one names a real trait seam
(`ShellRunner::run`), and that trait is Claudine's own, with **one** production
implementor. The rest are private or free functions whose `String` error type is
an unforced internal choice.

## Summary

### By category

| Category | Count | Disposition |
|---|---|---|
| **1 — typed provenance defect** | **46** | Fix. (45 in scope; 1 is the §D10 deferral below.) |
| **2 — genuinely unstructured external text** | **8** | Keep; re-tag `retained` with a narrow reason. |
| **3 — presentation-only, post-render-boundary** | **10** | Keep; re-tag `retained` with a narrow reason. |
| Total | **64** | |

So **18 of 64 (28%) are mis-tagged** — they are permanent exceptions wearing a
burn-down tag — and **46 (72%) are real defects**, one of which legitimately
defers.

### Category-1 sites by shape

| Shape | Category-1 count |
|---|---|
| `format_context` | 19 |
| `error_text_field` | 16 |
| `to_string_collapse` | 7 |
| `formatted_report` | 4 |
| Total | 46 |

### Category-1 sites by proposed mechanism

Each site is assigned its one primary mechanism.

| Mechanism | Count | Typical effort |
|---|---|---|
| Retype a function/trait error slot (`String`/`Report` → concrete) | 14 | moderate–invasive |
| New typed variant on an existing enum (`ClaudineError` / `CompositionError` / `HarnessError`) | 10 | moderate |
| New heterogeneous cause enum (`MarkdownLoadCause` pattern) + `#[source]` | 9 | moderate |
| `Report::wrap_err` in place of `eyre!("…{e}")` | 5 | trivial |
| Reuse an existing `#[from]` variant (`JsonParse`, `LaunchContextDetection`) | 4 | trivial |
| Typed `#[source]` on an existing variant, prose field kept | 4 | trivial |
| Total | 46 | |

### The retention rule makes most of this cheap

`source_scan.rs::retains_typed` clears a finding when the binding survives as a
bare path anywhere in the constructing expression. So

```rust
ClaudineError::PolicyNativeParse { source_id, message: error.to_string() }
```

becomes, with **no change to `Display`, `code()`, `detail()`, exit status, or
rendered text**:

```rust
ClaudineError::PolicyNativeParse {
    source_id,
    message: error.to_string(),
    source: PolicyParseCause::from(error),
}
```

The scanner's own module doc ratifies this: `Foo { message: e.to_string(),
source: e }` "stringifies `e` *and* keeps it. The chain is intact, so it is not
a defect."

**17 of the 46 close this way** — the bottom three mechanism buckets above
(4 + 9 + 4). Each is additive, leaves every observable surface untouched, and is
individually reviewable. This is the single most important fact for scoping the
burn-down: it is not 46 redesigns. The genuinely hard work is the 14 retypes,
and half of those are concentrated in two subsystems (messaging and the
lifecycle executor).

## Findings by file

Effort is `trivial` (call-site or one field), `moderate` (variant/enum change
plus its call sites), or `invasive` (signature or contract change). "Risk" flags
§D10 exposure.

### `lib/src/error.rs` and the permissions providers

`lib/src/permissions/` owns **no** error enum; every site funnels into
`ClaudineError`. No new enum is needed — `ClaudineError` already demonstrates
both patterns (`ProtectRuleParse { pattern, source: regex::Error }`,
`LaunchContextDetection(#[source] Box<sniff::SniffError>)`).

| File | Symbol | Concrete type in hand | Cat | Mechanism | Effort | Risk |
|---|---|---|---|---|---|---|
| `permissions/mutation.rs` | `PolicyMutationPlan::apply` | `ClaudineError` (always `::Io`) from `atomic_write` | 1 | Narrow `atomic_write` to `io::Error`; add `#[source] source: std::io::Error` to `PolicyApplyFailed` | moderate | none |
| `permissions/providers/claude.rs` | `ClaudePolicyBackend::load_native_layers` | `serde_json::Error` | 1 | New `PolicyParseCause` enum + `#[source]` on `PolicyNativeParse` | trivial | none |
| `permissions/providers/claude.rs` | `ClaudePolicyBackend::parse_cli_overrides` | `serde_json::Error` | 1 | `#[source]` on `PolicyCliParse` | trivial | none |
| `permissions/providers/codex.rs` | `CodexPolicyBackend::load_native_layers` | `toml_edit::TomlError` | 1 | `PolicyParseCause` + `#[source]` | trivial | none |
| `permissions/providers/gemini.rs` | `GeminiPolicyBackend::load_native_layers` | `serde_json::Error` (:205) **and** `toml_edit::TomlError` (:216) | 1 | `PolicyParseCause` + `#[source]` | trivial | none |
| `permissions/providers/goose.rs` | `GoosePolicyBackend::load_native_layers` | `serde_yaml_ng::Error` | 1 | `PolicyParseCause` + `#[source]` | trivial | none |
| `permissions/providers/kimi.rs` | `KimiPolicyBackend::load_native_layers` | `toml::de::Error` (**not** `toml_edit::TomlError`) | 1 | `PolicyParseCause` + `#[source]` | trivial | none |
| `permissions/providers/opencode.rs` | `OpenCodePolicyBackend::load_native_layers` | `serde_json::Error` | 1 | `PolicyParseCause` + `#[source]` | trivial | none |
| `permissions/providers/qwen.rs` | `QwenPolicyBackend::load_native_layers` | `serde_json::Error` | 1 | `PolicyParseCause` + `#[source]` | trivial | none |

**Four distinct parse-error types** feed the single `PolicyNativeParse {
message: String }`: `serde_json::Error`, `toml_edit::TomlError`,
`toml::de::Error`, `serde_yaml_ng::Error`. `ClaudineError`'s existing
`JsonParse`/`TomlParse` cover only two, so a `PolicyParseCause` cause enum —
modeled exactly on the in-repo `MarkdownLoadCause` / `SequenceLoadCause` — is
the right host.

**`atomic_write` is the keystone.** Every fallible operation in
`lib/src/config/atomic.rs` is an `io::Error` (`create_dir_all`,
`NamedTempFile::new_in`, `write_all`, `sync_all`, `persist`), so its
`Result<()>` (= `ClaudineError`) is wider than reality. Narrowing it to
`io::Error` is nearly free: its ~39 call sites across 15 files use `?` into
`Result<_, ClaudineError>`, which keeps compiling via `#[from] std::io::Error`.
This narrowing also **retires the one `error-propagation-followup` entry in
`boxed-diagnostic-allow.toml`** (`CompositionError::AtomicWriteFailed` boxes
`ClaudineError` and is invisible to `as_diagnostic`); with an `io::Error` source
the box disappears entirely. Land it first.

### `lib/src/actions`, `lib/src/dispatch`, `lib/src/runaway`

| File | Symbol | Concrete type in hand | Cat | Mechanism | Effort | Risk |
|---|---|---|---|---|---|---|
| `actions/bash_executor.rs` | `validate_js_ts` | `std::io::Error` (:97); `which::Error` (:124, discarded by `\|_\|`) | 1 | New `ClaudineError::ScriptUnreadable { command, #[source] source: io::Error }` | trivial | **flag** — see below |
| `dispatch/loader.rs` | `compile_canonical_mapper` | `regex::Error` | 1 | New variant shaped like the existing `ProtectRuleParse { pattern, #[source] source: regex::Error }` | trivial | **flag** — see below |
| `dispatch/loader.rs` | `parse_json5_to_value` | `biscuit_file::Json5Error` | 1 | `#[from] Json5Error` variant or `#[source]` | trivial | none |
| `runaway/config.rs` | `extract_frontmatter_exit_expressions` | `serde_json::Error` | 1 | `ClaudineError::JsonParse` (already exists) or `#[source]` | trivial | none |
| `cli/src/commands/wrap/runaway_guard.rs` | `extract_frontmatter_guard_settings` | `serde_json::Error` | 1 | Same as above | trivial | none |

Two **behavior flags**, both cases where the *current* code misclassifies and
the obvious fix silently changes `err.code`:

- `validate_js_ts` flattens into `ConfigValidation` → `config.invalid`. Routing
  the `io::Error` to `ClaudineError::Io` instead would reclassify a
  permission-denied script as `io.permission_denied`, because
  `ClaudineError::code()` branches on `e.kind()`. That is a *better*
  classification but it is an `err.code` change. **Recommendation:** keep
  `config.invalid` via a dedicated variant; propose the reclassification
  separately.
- `compile_canonical_mapper` flattens into `TemplateError` → **`internal.bug`**,
  so a user's malformed regex is currently reported as a Claudine bug.
  `RegexError` would map it to `config.invalid`. Same treatment: preserve the
  source now, propose the code correction separately.

Note `runaway/config.rs` and `runaway_guard.rs` are byte-identical shapes; both
`ConfigValidation` and `JsonParse` already map to `config.invalid`, so those two
are genuinely zero-risk.

### `lib/src/harness`

`HarnessError` has **no `#[source]` and no `#[from]` on any variant** — there is
no cause chain in this enum at all. These sites cannot retain anything until the
enum gains one; this batch must precede its dependents.

`PathResolutionFailed` is the model already in the file: it carries a typed
`PathResolutionFailure` discriminant plus `raw`/`source_path`/`resolved`, and
derives its prose at `Display` time via `path_resolution_detail`. (§D8 names it
as a `{ detail: String }` anchor; that is stale — it was already fixed.)

| File | Symbol | Concrete type in hand | Cat | Mechanism | Effort | Risk |
|---|---|---|---|---|---|---|
| `harness/audit.rs` | `collect_auditable_commands` | `darkmatter::…::ShellExpansionError` | 1 | `#[source] source: Box<ShellExpansionError>` on `ShellAuditParseError`, mirroring `CompositionError::ShellExpansionFailed` | moderate | none |
| `harness/audit.rs` | `audit_shell_commands` | `HarnessError` | **3** | Re-tag `retained` | — | — |
| `harness/shell.rs` | `execute_approved_command` | `which::Error` (:375, discarded); `io::Error` (:387, :432); `tokio::…::Elapsed` (:436, discarded) | 1 | Typed `failure: ShellExecFailure` discriminant + `#[source]`, mirroring `PathResolutionFailed` | moderate | none |

**`audit_shell_commands` — Category 3, retain.** Proposed reason: *"The sink is
`ShellAuditOutcome.message`, a pre-rendered prose field carrying terminal markup
(`prose_escape`d, `<red-500>` spans). It is a partially-rendered view, not a
data record; the typed variants that matter are already destructured by the
preceding arms."*

Two **`Diagnostic::detail` mislabels** surface here and should be fixed in the
same batch: `ShellCommandExecutionFailed` and `ShellAuditParseError` both
project `json!({ "command": detail })`, where `detail` is a spawn/wait/PATH/parse
message and emphatically not a command.

### `lib/src/composition` — error surface and edges

`CompositionError` has 93 variants; 10 carry `#[source]`, **none** uses
`#[from]`. `role()` is `Semantic` for all but the two `Transparent` wrappers.

| File | Symbol | Concrete type in hand | Cat | Mechanism | Effort | Risk |
|---|---|---|---|---|---|---|
| `composition/preflight.rs` | `resolve_shell_approvals` | **`HarnessError`** | 1 | New variant carrying `#[source] source: HarnessError` — exact precedent: `InvalidFileReference` | moderate | **flag**: corrects a lost identity (§D10-permitted) |
| `composition/preflight.rs` | `resolve_shell_command_expr` | `darkmatter::markdown::MarkdownError` | 1 | Add `#[source] source: MarkdownError` to `LifecycleShellResolution`; keep `message` | trivial | none |
| `composition/preflight.rs` | `resolve_lifecycle_shell_commands` | `CtxMergeError` | 1 | New `PreFlightStateBuildFailed { #[source] source: CtxMergeError }` | moderate | none |
| `composition/resolve.rs` | `validate_file_permissions` | `std::io::Error` | 1 | Struct-ify `InsufficientFilePermissions` with `path` + `#[source] source: io::Error` | moderate | none |
| `composition/closure.rs` | `rewrite_inline_document` | `darkmatter::markdown::MarkdownError` | 1 | Return `Result<String, MarkdownError>`; caller wraps in a new `InlineRewriteFailed(#[source] MarkdownError)` | moderate | none |

`resolve_shell_approvals` is the highest-value site in this group: every
`HarnessError` variant other than the two destructured above lands in
`PreFlightFailed(String)` — the prose catch-all the architecture doc explicitly
says must not claim a code. §D10 lists "correction of a diagnostic identity that
was previously lost" as an *intended* change, so this is in scope.

Do **not** try to give `PreFlightFailed` a code. Per `error-architecture.md`, the
fix for a prose error that deserves a code is to *type it*, not to code the prose.

### `lib/src/composition/lifecycle`

| File | Symbol | Concrete type in hand | Cat | Mechanism | Effort | Risk |
|---|---|---|---|---|---|---|
| `lifecycle/executor.rs` | `SystemShellRunner::run` | `std::io::Error` | 1 | Retype `ShellRunner::run` → `Result<i32, ShellRunError>` (new enum, `#[from] io::Error`) | invasive | none (all 3 impls in-repo) |
| `lifecycle/executor.rs` | `StackExecutionContext::run_shell_action` | `String` today — becomes `ShellRunError` once the above lands | 1 (derived) | Carry the typed value into `ActionFailure` | moderate | none |
| `lifecycle/executor.rs` | `StackExecutionContext::eval_expr` | `darkmatter::…::ExpressionError` | 1 | Retype the private method → `Result<Value, ExpressionError>` | moderate | none |
| `lifecycle/executor.rs` | `StackExecutionContext::resolve_string_value` | `darkmatter::markdown::MarkdownError` | 1 | Retype the private method | moderate | none |
| `lifecycle/executor.rs` | `StackExecutionContext::dispatch_side_effect` | `darkmatter::effects::EffectError` | 1 | Typed slot on `ActionFailure` → `LifecycleErrorInfo` | invasive | **flag** — couples to Review Finding 2 |
| `lifecycle/parse.rs` | `parse_lifecycle_stack_item` | `darkmatter::…::ParseError` (`parse_condition`) | 1 | New `LifecycleWhenExpressionInvalid { …, #[source] source: ParseError }` | moderate | none |
| `lifecycle/parse.rs` | `parse_long_form_action_object` | `String` today — becomes `ParseError` once `action_value_to_expr` is retyped | 1 (derived) | Follows `action_shape.rs` | moderate | none |
| `lifecycle/action_shape.rs` | `action_value_to_expr` | `darkmatter::…::ParseError` | 1 | `Result<Expr, ActionExprError>` cause enum (`Parse(#[from] ParseError)` + a prose arm for the genuinely-prose `Err` arms) | moderate | none |

**`ShellRunner` is not "its own workstream".** It is `pub trait ShellRunner { fn
run(&self, command: &str) -> Result<i32, String>; }` — Claudine's own trait, one
production implementor (`SystemShellRunner`), one in-crate mock, and one
out-of-crate implementor in `lib/tests/agent_errors_fleet.rs`. It is held as
`&dyn ShellRunner` in four places, and a concrete error type keeps it
object-safe. Retyping it is a contained, mechanical change.

The two `(derived)` rows are important for sequencing: **they are Category 2 as
the code stands today** (the value in hand really is already a `String`), but
only because a sibling site in the same batch destroyed the type one frame
earlier. They must not be re-tagged `retained` — they close automatically once
their upstream lands, and the guard will prove it.

`dispatch_side_effect` is the one entangled with **Review Finding 2**:
`LifecycleErrorInfo::from_action_failure` hard-codes `cause: None` with the
comment "No typed error, so no chain to project a cause from." Giving it a typed
slot is the same work as making `LifecycleErrorInfo` consume a
`DiagnosticSnapshot`. Sequence these together or the second will churn the first.

### `lib/src/composition/looping` and the loop/sequence result records

| File | Symbol | Concrete type in hand | Cat | Mechanism | Effort | Risk |
|---|---|---|---|---|---|---|
| `looping/actions.rs` | `render_string_with_lookup` | `ParseError` (:218); `ExpressionError` (:224) | 1 | Cause enum + `#[source]` on `InvalidAction` | moderate | none |
| `looping/expression.rs` | `evaluate_condition` | `ParseError` (:159); `ExpressionError` (:162) | 1 | New `LoopExpressionInvalid { kind, source_str, #[source] source }` — `LoopInvalid(String)` is a bare newtype | moderate | none |
| `looping/config.rs` | `resolve_loop_config` | **none** | **2** | Re-tag `retained` | — | — |
| `cli/src/commands/compose/prep.rs` | `build_and_run_loop` | **`color_eyre::eyre::Report`** | 1 | See below — needs a design decision | invasive | none (`reason` is not matched on) |
| `cli/src/commands/wrap/sequence/iterate.rs` | `run_sequence_steps` | **`color_eyre::eyre::Report`** | 1 | Same shape as above | invasive | none (fail-fast keys on counts, not text) |

**`resolve_loop_config` — Category 2, retain.** Proposed reason: *"The value is
already a `String`: `OnRateLimit::parse` returns `Result<Self, String>` and
constructs its message inline from an unrecognized literal. No typed error is
ever created, so none is dropped."* The scanner's other hits in this function
interpolate `json_type_name(...)`, a `&'static str` type label.

**The two `Report` sites need a ratified decision before implementation.**
`eyre::Report` does **not** implement `std::error::Error`, so it cannot be a
`#[source]` field. `.to_string()` on it prints only the outermost frame and
discards every `wrap_err` context accumulated across the whole composition
pipeline — by volume, the largest loss in the burn-down. Two candidate
mechanisms, neither free:

1. **Type the producer.** Give `execute_composition_attempt` /
   `execute_composition_request_inner` a concrete error type. Correct, and the
   real fix; large blast radius.
2. **`#[source] Option<Box<dyn Error + Send + Sync + 'static>>`** fed by
   `Report`'s `Into<Box<dyn Error + Send + Sync>>`. Cheap, but I have **not**
   verified that the selection walk can traverse a boxed trait object here, and
   `error-architecture.md` rule 2 is explicit that boxing a source costs
   discoverability. **Do not adopt this without checking it against
   `no_registered_diagnostic_is_reachable_only_through_a_box` and a live
   `as_diagnostic` traversal test.**

I flag this rather than pick: the answer depends on a guard interaction I could
not confirm from the sources alone.

### `lib/src/messaging`, `lib/src/mcp`, `lib/src/reporting`, `lib/src/dispatch/runner`

| File | Symbol | Concrete type in hand | Cat | Mechanism | Effort | Risk |
|---|---|---|---|---|---|---|
| `messaging/send.rs` | `register_webhook_provider` | `messenger::MessengerError` (:162, :175). *`resolve_secret` sites are already `String` — not lossy.* | 1 | New `MessagingError` enum with `#[from] MessengerError` | moderate | **flag** — see below |
| `messaging/send.rs` | `send_desktop_notification` | `messenger::MessengerError` | 1 | Same | moderate | **flag** |
| `messaging/send.rs` | `send_payload` | `messenger::MessengerError` (:591, :609) | 1 | Same | moderate | **flag** |
| `messaging/send.rs` | `test_webhook_connection` | `messenger::MessengerError` | 1 | Same | moderate | **flag** |
| `mcp/import.rs` | `McpImporter::import_provider` | `ClaudineError` | 1 | Per §D9: add a `DiagnosticSnapshot` to `ImportError` beside `reason` | moderate | none |
| `mcp/session.rs` | `compute_session_set` | `ClaudineError` (`McpServerNotFound` / `McpAmbiguousMatch`) | 1 | Model structurally — the struct already has `missing_tags` / `ambiguous_tags` | moderate | none |
| `reporting/ingest.rs` | `sync_file` | `io::Error`, `rusqlite::Error`, `serde_json::Error`, `ClaudineError` | **2** | Re-tag `retained` | — | — |
| `dispatch/runner/mod.rs` | `execute_actions` | `ClaudineError` (:300, :313); `tokio::…::Elapsed` (:320) | **2** | Re-tag `retained` | — | — |
| `cli/src/commands/config_tui/mod.rs` | `run_tui` | **none** — `e` is already a `String` | **2** | Re-tag `retained` | — | — |

> **Correction (Batch 6, on contact with the code).** `compute_session_set` is
> **Category 3, not Category 1**. Its sink is `SessionSet.warnings`, a display
> buffer the CLI drains verbatim into `deferred_warnings`; control flow keys on
> `missing_tags` / `ambiguous_tags` and never on this text. "Model structurally"
> would add a `SessionSet` field no consumer reads and would change the warning
> prose — a §D10 violation for no gain. Re-tagged `retained`. The *other* defect
> the triage found in this function (follow-up 5, the `Err(_)` arm asserting a
> cause it did not read) stands and is untouched: `Err(_)` binds no identifier,
> so the guard never saw it.

**`sync_file` — Category 2, retain.** Proposed reason: *"The `Err` type is
`SyncFailure`, a `Serialize + Deserialize + Eq` report record persisted in the
sync summary. Per §D9 concrete error values never cross a persistence boundary;
a typed cause cannot live in this struct without breaking its derives."* Worth
recording as a follow-up: the flattening happens at the innermost `?` rather
than at the report boundary, so `ClaudineError` values that already wrap a
`rusqlite::Error` are double-flattened.

**`execute_actions` — Category 2, retain.** Proposed reason: *"The sink is
`HookResponse.reason`, the agent-facing hook-protocol JSON field that Claudine
emits for the provider CLI to consume. `String` is the wire contract, and
`execute_actions` returns `Ok(Some(response))` — the failure is a protocol
response, not an `Err`."*

**`run_tui` — Category 2, retain.** Proposed reason: *"`e` is already a `String`:
`app.pending_test` is a `Receiver<Result<(), String>>` fed by
`test_webhook_connection`. Nothing typed reaches this line."* Once the messaging
batch types `test_webhook_connection`, revisit — this becomes a Category 3
render-boundary conversion into a ratatui widget field, still `retained`.

**Messaging behavior flag — the biggest risk in the burn-down.**
`failure_hint(error: &str)` substring-matches the flattened prose
(`"429"`, `"invalid_auth"`, `"channel_not_found"`, `"rate limit"`, …) to choose
a user-facing hint. This is precisely the anti-pattern `error-architecture.md`
exists to remove: re-deriving by string-matching the classification
`MessengerError`'s variants already encoded. The load-bearing casualty is
`RateLimited { retry_after_ms: Option<u64> }` — a machine-actionable delay
flattened into `", retry after 3000ms"`.

Constraints that make this tractable and behavior-neutral:

- `failure_hint`, `report_send_failure`, and `report_notification_failure` are
  all private to `send.rs`, and `failure_hint` is tested only against literal
  strings.
- `report_send_failure` renders directly to stderr and returns nothing, so
  messaging failures never touch exit codes or lifecycle ordering.

**Discipline:** keep `MessagingError`'s `Display` byte-identical to today's
strings, and keep `failure_hint(&err.to_string())` at the render site — where it
is a legitimate Category-3 conversion. Rewriting `failure_hint` to match on
typed variants is a *better* end state and a *separate* change; doing both at
once makes the diff unreviewable. The `redact_webhook_urls` regex scrub is a
second symptom of the same flattening and should be revisited once the type
survives.

### `cli/src/*` — the render-boundary class

Six of these sites share one shape: a `ClaudineError` is printed with
`log::error`/`log::message` and the function then returns `Ok(())`. The string
conversion is genuinely Category 3 — it happens at the final render boundary and
never becomes an error.

**But every one of them also swallows a failure into a success exit.** That is a
real correctness bug and it is **not** this spec's to fix: §D10 states plainly
that typed propagation "must not accidentally alter process exit status." Fixing
the transport here is a re-tag; fixing the exit code is a behavior change that
must be filed separately. Recording both facts is the point of this row.

| File | Symbol | Concrete type in hand | Cat | Disposition | Exit-code bug? |
|---|---|---|---|---|---|
| `commands/actions.rs` | `run` | `ClaudineError` | 3 | Re-tag `retained` | **yes** — config load failure exits 0 |
| `commands/init/mod.rs` | `run_interactive` | `ClaudineError` | 3 | Re-tag `retained` | **yes** — prints "Done!" when every provider failed |
| `commands/init/mod.rs` | `run_quick` | `ClaudineError` | 3 | Re-tag `retained` | **yes** — same |
| `commands/init_wizard.rs` | `register_hooks_all_providers` | `ClaudineError` | 3 | Re-tag `retained` | **yes** — and uses `log::message`, so it is not even marked as an error |
| `commands/sync.rs` | `run` | `ClaudineError` | 3 | Re-tag `retained` | **yes** — `claudine sync` exits 0 when every register/deregister failed |
| `commands/uninstall.rs` | `run` | `ClaudineError` | 3 | Re-tag `retained` | **yes** — internally inconsistent: `remove_file` failure *does* exit 1 |
| `commands/logs/mod.rs` | `best_effort_sync` | `ClaudineError` | 3 | Re-tag `retained` | no — `-> ()`, a documented best-effort degradation |

Proposed reason for the six: *"The typed error is rendered to stderr at this
line and never returned; the conversion is the final render. The enclosing
function's `Ok(())`-on-failure is a separate exit-status defect tracked in
`<follow-up spec>` — §D10 forbids changing exit status in the transport
migration."*

For `best_effort_sync`: *"The function returns `()`; propagation is
structurally impossible and deliberately so — all nine call sites invoke it as a
bare statement. The `log::warn` is the final render."*

### `cli/src/cli_utils.rs` and the remaining CLI/wrap edges

| File | Symbol | Concrete type in hand | Cat | Mechanism | Effort | Risk |
|---|---|---|---|---|---|---|
| `cli_utils.rs` | `parse_naive_date` | `chrono::format::ParseError` | **2** | Re-tag `retained` | — | — |
| `cli_utils.rs` | `parse_provider` | **none** | **2** | Re-tag `retained` | — | — |
| `commands/init/prompts.rs` | `prompt_log_target` | `url::ParseError` | 1 | `.wrap_err("Invalid URL")` | trivial | none — exit stays 1 |
| `commands/wrap/composition/pipeline.rs` | `construct_argv_and_system_prompt` (`format_context`) | `ClaudineError` | 1 | `.wrap_err(…)` at :676 | trivial | none |
| `commands/wrap/composition/pipeline.rs` | `construct_argv_and_system_prompt` (`formatted_report`) | *same site as above* | 1 | Closed by the same fix | trivial | none |
| `commands/wrap/composition/prep_context.rs` | `CompositionPrepContext::new` | `sniff::SniffError` | 1 | Store the typed value — `ClaudineError::LaunchContextDetection(#[source] Box<SniffError>)` already exists | invasive | **cross-batch** — see below |
| `commands/wrap/env/mod.rs` | `detect_wrap_startup_or_fallback` (`formatted_report`, :165) | `color_eyre::Report` | 1 | `Err(error).wrap_err(…)` — byte-identical top line | trivial | none |
| `commands/wrap/env/mod.rs` | `detect_wrap_startup_or_fallback` (`format_context`, :169) | `color_eyre::Report` → `deferred_warnings` | **3** | Re-tag `retained` | — | — |
| `commands/wrap/env/mod.rs` | `build_child_env_with_launch` | `color_eyre::Report` → `EnvPlan.warnings` | **3** | Re-tag `retained` | — | — |
| `commands/wrap/mod.rs` | `run_provider_wrapper_inner` | **none** — `OutputFormat::Err = String` | **2** | Re-tag `retained` | — | — |
| `commands/wrap/wrapper_mcp.rs` | `merge_injected_env_into_plan` | `serde_json::Error` | 1 | `.wrap_err(…)` — the *next line* already does this | trivial | none |
| `commands/wrap/exec/wiring/dispatch.rs` | `dispatch_hook_request` | `ClaudineError` | **2** | Re-tag `retained` | — | — |

Proposed `retained` reasons:

- **`parse_naive_date`**: *"A clap `value_parser`. clap's `TypedValueParser`
  requires `Err: Into<String>`; clap owns the resulting error, prints usage, and
  exits. The `String` is structurally forced by an external ratified contract,
  and the value never reaches the error walker."*
- **`parse_provider`**: *"No typed error exists. `Provider::parse_cli_name`
  returns `Option<Self>`; the message is constructed from `None`. The sibling
  `String` shape exists to satisfy a clap `try_map`, and the only caller
  discards the report with `.ok()`."*
- **`run_provider_wrapper_inner`**: *"`impl FromStr for OutputFormat { type Err
  = String }` — the error is prose at its origin, so `eyre!(e)` is a faithful
  transport of an already-unstructured value. Typing `OutputFormat::Err` is the
  real fix and is a separate, additive change."*
- **`dispatch_hook_request`**: *"The signature is infallible by design (hook
  dispatch fails open to allow). The sink is
  `HookDispatchResult.warning`, which becomes an agent-facing synthetic
  Notification envelope whose `message` is a String field on the stream wire
  contract; the parser routes on `level`, never on the text."*
- **`env/mod.rs` ×2**: *"The sink is a deferred-warning display buffer drained
  only for rendering (`EnvPlan.warnings`). The conversion is the final render;
  the fallback path is chosen by the `Err` arm, never by the string."*

> **Correction (Batch 7, on contact with the code).** Two rows above are wrong
> about scope.
>
> - **`prep_context.rs` is not "presence-checked only" and not `moderate`.**
>   `enforce_repo_launch_detection` interpolates the captured string into its
>   `eyre!`. The value is stored as `Option<String>` and cloned into
>   `CompositionRequest.prep_launch_detection_error: Option<String>`
>   (`lib/src/composition/types.rs`) by `compose/prep.rs` and
>   `wrap/sequence/iterate.rs` — **both Batch-5 files**. `ClaudineError` is not
>   `Clone`, so retyping the field breaks both `.clone()` sites too. This entry
>   cannot close inside Batch 7; it needs one change spanning batches 5 and 7,
>   or a `CompositionRequest` field that is `Arc<ClaudineError>`. **Left as
>   `error-propagation-followup` with the blockage recorded in its reason.**
> - **`env/mod.rs`'s `format_context` row is not a `retained`** — it closed for
>   free. Both of that symbol's entries live in the same `Err(error)` arm, and
>   `retains_typed` clears an arm *wholesale* once the binding survives as a bare
>   path. The one `Err(error).wrap_err(…)` that closes the `formatted_report`
>   finding therefore also retires the `format_context` one, which then fails the
>   staleness test if left in the file. The same whole-arm clearing applies to
>   `pipeline.rs`, where the triage already predicted it for a different reason.

`pipeline.rs` is worth noting for the count: its two allowlist entries
(`format_context` and `formatted_report`) are **the same expression** — an
`eyre!` whose format string interpolates a typed error — so one `wrap_err`
closes both. The `formatted_report` finding the allowlist attributes to :797 is
a false positive; `format_interactive_timeout_conflict` involves no `Result`.

## Genuine §D10 deferrals

Exactly **one** of the 64.

### `cli/src/commands/wrap/inline.rs` :: `try_inline_closure`

**Concrete type in hand:** `claudine::composition::CompositionError` (from
`extract_replacement_body`), flattened at :120 into
`Result<(), Vec<String>>`.

**Why it is a real §D10 case, not a transport fix.** The flattened string is not
merely rendered — it is consumed by the recovery decision surface. In
`harness_orch/loop_control.rs:1140-1163` the joined failures become `fail_msg`,
which feeds
`LifecycleErrorInfo::from_action_failure("inline_closure", fail_msg)`, whose
result is projected into the lifecycle expression-evaluation context as
`err.msg` / `err.kind` / `err.variant`. Author-written frontmatter rules evaluate
against those to decide recovery and retry.

Threading the typed `CompositionError` through so `err.code` / `err.category` /
`err.detail.*` populate would change **which aliases `to_value()` projects**:
`lifecycle/context.rs:84-92` documents that a classifiable error projects
`category`/`code` under the `err.kind` / `err.variant` aliases *instead of* the
action-failure spellings. Any existing rule matching `err.kind ==
"LifecycleAction"` or `err.variant == "inline_closure"` would **silently stop
matching**.

That is squarely what §D10 reserves for a separate spec — a change to how a
route's failure is classified for retry/recovery — and it collides with
`error-architecture.md`'s rule that removing or renaming an `err.*` value is
breaking and out of scope for anything that is not an explicit versioned
migration.

**Recommendation.** Defer to its own spec **unless** an additive path is
ratified first: populate `err.code` / `err.category` / `err.detail` from the
typed cause *while preserving* the `err.kind = "LifecycleAction"` /
`err.variant = "inline_closure"` aliases. That path is additive by construction
and would let this site close inside the feature. Either way, the decision is
Ken's, not the implementer's.

**Re-tag it now** from `error-propagation-followup` to a distinct
`d10-routing-change` tag with the reasoning above, so it is not confused with the
frozen set. It must **not** be tagged `retained` — a typed error is genuinely in
hand.

## Sites the guard should stop reporting

Three entries are scanner false positives, not exceptions. They are recorded as
Category 2 above so the allowlist stays honest, but the better fix is a scanner
refinement: in all three the binding's type is *already* `String`
(`run_tui`, `run_provider_wrapper_inner`, `resolve_loop_config`) or is
constructed from `Option::None` (`parse_provider`). If `source_scan.rs` can
prove a binding's type is `String`, four exceptions disappear rather than being
argued.

## Batch plan

Seven batches, no overlapping files, ordered so shared error-type changes land
before their dependents. Each is independently testable and independently
reviewable.

| # | Batch | Files | Entries | C1 | Depends on |
|---|---|---|---|---|---|
| 1 | **`ClaudineError` foundations + permissions** | `lib/src/error.rs`, `lib/src/config/atomic.rs`, `lib/src/permissions/mutation.rs`, `lib/src/permissions/providers/{claude,codex,gemini,goose,kimi,opencode,qwen}.rs`, `lib/src/actions/bash_executor.rs`, `lib/src/dispatch/loader.rs`, `lib/src/runaway/config.rs`, `cli/src/commands/wrap/runaway_guard.rs` | 14 | 14 | — |
| 2 | **`HarnessError` gains a cause chain** | `lib/src/harness/{error,audit,shell}.rs` | 3 | 2 | — |
| 3 | **`CompositionError` variants + composition edges** | `lib/src/composition/error/mod.rs`, `lib/src/composition/{preflight,resolve,closure}.rs` | 5 | 5 | 2 |
| 4 | **Lifecycle execution + the `ShellRunner` seam** | `lib/src/composition/lifecycle/{executor,parse,action_shape,context}.rs` | 8 | 8 | 3 |
| 5 | **Looping + loop/sequence result records** | `lib/src/composition/looping/{actions,expression,config}.rs`, `cli/src/commands/compose/prep.rs`, `cli/src/commands/wrap/sequence/iterate.rs` | 5 | 4 | 3 |
| 6 | **Messaging, MCP, reporting** | `lib/src/messaging/send.rs`, `lib/src/mcp/{import,session}.rs`, `lib/src/reporting/ingest.rs`, `lib/src/dispatch/runner/mod.rs`, `cli/src/commands/config_tui/mod.rs` | 9 | 6 | — |
| 7 | **CLI and wrap edges** | `cli/src/cli_utils.rs`, `cli/src/commands/{actions,init_wizard,sync,uninstall}.rs`, `cli/src/commands/init/{mod,prompts}.rs`, `cli/src/commands/logs/mod.rs`, `cli/src/commands/wrap/{mod,wrapper_mcp}.rs`, `cli/src/commands/wrap/composition/{pipeline,prep_context}.rs`, `cli/src/commands/wrap/env/mod.rs`, `cli/src/commands/wrap/exec/wiring/dispatch.rs` | 19 | 6 | — |
| — | **Deferred** | `cli/src/commands/wrap/inline.rs` | 1 | 1 | §D10 spec |

**Ordering rationale.**

- **1 → first, unconditionally.** It narrows `atomic_write`, which is the
  keystone for `PolicyApplyFailed` *and* retires the `AtomicWriteFailed` entry in
  `boxed-diagnostic-allow.toml`. It also establishes the `PolicyParseCause`
  pattern the later batches copy. It touches `lib/src/error.rs`, which no other
  batch may.
- **2 → before 3.** `CompositionError`'s preflight fix carries a `HarnessError`
  as `#[source]`; that enum needs a cause chain first.
- **3 → before 4 and 5.** Batch 3 owns `composition/error/mod.rs` exclusively and
  must add **every** new variant the lifecycle and looping batches consume, up
  front. This is the one place the "no overlapping files" rule forces a
  look-ahead: batches 4 and 5 add call sites only.
- **4 and 5 are parallel** after 3.
- **6 and 7 are independent of all the above** and can start immediately —
  useful for parallelizing the burn-down. 7 is mostly re-tagging and five
  one-line `wrap_err`s; it is the cheapest way to shrink the list and should not
  be scheduled last just because it is numbered last.

**Guard change.** Once a batch lands, delete its entries rather than re-tagging
them. When all seven are done, the only `error-propagation-followup` tag left
should be zero: 18 entries become `retained`, 45 disappear, and 1 moves to
`d10-routing-change`. At that point the header's §D10 paragraph
(`transport-allow.toml:18-28`) must be deleted — it is the reasoning the review
rejected, and leaving it would re-license the next freeze.

## Follow-up defects found during triage

None of these are transport work; all are out of scope for this spec and should
be filed separately.

1. **Six CLI commands swallow failures into exit 0** (`actions`, `init`
   interactive + quick, `init_wizard`, `sync`, `uninstall`). `claudine sync`
   printing success when every provider failed is the sharpest instance.
2. **`compile_canonical_mapper` reports a user's bad regex as `internal.bug`**
   via `TemplateError`.
3. **`validate_js_ts` classifies a permission-denied script as `config.invalid`**,
   losing the `io.permission_denied` that `ClaudineError::code()` would derive
   from `io::ErrorKind`.
4. **Two `HarnessError` detail projections emit prose under a `command` key**
   (`ShellCommandExecutionFailed`, `ShellAuditParseError`) where the value is a
   spawn/wait/PATH/parse message.
5. **`compute_session_set` asserts a cause it did not read** — the `Err(_)` arm
   claims "not found in catalog" when `resolve` can equally have returned
   `McpAmbiguousMatch`.
6. **§D8's migration-anchor list is stale**: it names
   `lib/src/harness/error.rs (PathResolutionFailed { detail: String })`, but that
   variant is now the *most* typed one in the enum and is the model the other four
   should follow.
