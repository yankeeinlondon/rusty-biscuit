---
prompt: |-
    Review the claudine code base to understand what the current state is for
    error handling. You'll likely find references to pre-check and post-check validations.

    Write up a summary that focuses more on breadth the depth but does provide a few code examples to round out some of the depth.
last_updated: 2026-06-21
---
# Claudine Error Handling: Current State

Claudine splits its error surface across three independent `thiserror` enums rather than one god-enum. Each one owns a distinct failure domain, and they meet only at the CLI's top-level reporting walker, which renders whichever one bubbles up.

| Enum               | Location                                | Domain                                                                                    |
|--------------------|-----------------------------------------|-------------------------------------------------------------------------------------------|
| `ClaudineError`    | `claudine/lib/src/error.rs`             | Library-wide: I/O, JSON/TOML/YAML, HTTP, SQLite, config, MCP, PolicyEngine, system prompt |
| `HarnessError`     | `claudine/lib/src/harness/error.rs`     | Pre/post-checks, timeouts, handlers, shell audit                                          |
| `CompositionError` | `claudine/lib/src/composition/error.rs` | Markdown composition, schema validation, sequence + loop pipelines                        |

A fourth, narrower surface lives in the contract crate (`claudine/contract/src/error.rs`), mapping session outcomes onto the stable `biscuit_contract::InferenceErrorKind` categories.

## Pre-checks and Post-checks (the harness)

The harness is the most structured error-handling surface in claudine. It is scoped to `compose`, `inline-compose`, `sequence` (each step runs the harness independently), and wrapper passthrough prompts that resolve to Markdown with harness frontmatter (`pre_checks`, `post_checks`). It runs in three phases:

1. **Before (pre_checks)** — should the run even start? File/dir/JSON/YAML/TOML existence, write permissions, clean repo state, shell setup checks.
2. **During** — normal provider execution. Non-zero exit or timeout becomes a failure event.
3. **After (post_checks)** — did the run accomplish what the document claimed? File diff, frontmatter comparison, response-content checks.

All three phases share the same `ValidationEvent` taxonomy (20 variants) and `FailurePhase` discriminator (`PreCheck`, `PostCheck`, `Agent`, `ShellAudit`) defined in `claudine/lib/src/harness/failure.rs:17-87`. Failures are collected, not short-circuited — `run_checks` walks every rule and accumulates `ValidationCheckOutcome` records so a single pre-check pass reports every missing prerequisite at once.

The engine itself is `evaluate_single` in `claudine/lib/src/harness/validate/mod.rs:203-300`. Each rule is a `match` arm returning `Result<(), String>` (aliased as `CheckResult`). A representative pre-check arm:

```rust
ValidationKind::FileExists { file } => {
    if file.exists() && !file.is_dir() {
        Ok(())
    } else {
        Err(format!("file does not exist: {}", file.display()))
    }
}
```

Post-only comparison checks consume a `PreRunSnapshot` (BLAKE3 fingerprints for `file_changed`/`file_unchanged`, captured frontmatter values for `frontmatter_prop_changed`) and the post-run `AttemptOutcome` (for `response_includes`, `response_length_at_least`, etc.). Snapshots only capture state for files/properties actually referenced by `post_checks`, keeping them small and deterministic (`capture_pre_run_snapshot`, `validate/mod.rs:60-102`).

## Path resolution

Every path-bearing validation uses document-centric resolution with three rules (`harness/resolve.rs`):

1. **Absolute** — returned as-is.
2. **`@`-prefixed** — resolved from repo root; errors if no repo root is detected (`HarnessError::RepoRootRequired`).
3. **Relative** — resolved from the source document's parent directory.

## Parse-time validation

`parse_harness_plan` (`harness/parse/mod.rs:65`) does structural and relational validation up front, before any rule executes. It produces typed `HarnessError` variants for:

- `InvalidFrontmatter` — wrong types or shapes (e.g. `timeout` not a string).
- `UnknownValidation` — an unrecognized check name.
- `PostOnlyInPreChecks` — e.g. `file_changed` placed under `pre_checks`.
- `InvalidTimeout` — bad duration grammar, zero/negative values, or a `step_timeout` greater than `timeout`.
- `MissingHandlerField` — e.g. `resume` without a `prompt`.

Two cross-field timeout invariants are enforced here and emit `tracing::error!` adjacent to the user-facing message: `step_timeout <= timeout`, and each `*_warn` must be strictly less than its corresponding hard threshold (`parse/mod.rs:153-236`).

## Shell audit (pre-execution, no execution)

`collect_auditable_commands` (`harness/audit.rs:17`) walks the plan and gathers every shell command from pre-checks, post-checks, the programmatic `handle`, declarative `deviate` handlers, and any `::shell` directives in the source page. `audit_shell_commands` then runs each through shell policy **without executing it**. Denials and blacklist matches become `ShellAuditOutcome` entries with structured `passed`/`message` fields, ready for the recovery pipeline.

## Failure events and handler resolution

Four categories of failure are recognized (`harness/failure.rs` + `harness/handlers.rs`):

| Event                | Trigger                                                              |
|----------------------|----------------------------------------------------------------------|
| `agent_failure`      | Non-zero exit from provider                                          |
| `timeout`            | Execution exceeded declared timeout                                  |
| `shell_audit_denied` | Shell command denied by approval policy                              |
| `<validation_event>` | Any pre/post check failure (e.g. `file_exists`, `response_includes`) |

`resolve_handler` (`harness/handlers.rs:49-91`) walks four recovery tiers in order:

1. Subject-specific YAML handler (e.g. `handle_file_exists` keyed on a specific path).
2. Generic YAML handler (e.g. `handle_timeout`).
3. Programmatic `handle` command (JSON on stdin, env vars, returns one of `retry`/`resume`/`redirect`).
4. Unhandled — terminal failure.

Four declarative recovery actions (`retry`, `resume`, `redirect`, `deviate`) share `msg`/`say`/`set` common fields; `deviate` runs an approved external command before retrying, and is **declarative-only** — programmatic handlers cannot return it because deviate commands must be pre-screened at parse time (`handlers.rs:260-270`).

## Failure reporting

Passing checks render as a single compact `Status` line on stderr. Failing checks with `RuleSource` metadata render a four-section block (`harness/report.rs:231-287`):

1. **Status header** — red glyph plus a phase label (`Pre-validation failed`, `Post-validation failed`, `Agent execution failed`, `Shell audit failed`).
2. **Source line** — `in <path>:N-M` with OSC8 hyperlink to the failing rule.
3. **YAML snippet** — chrome-free, syntax-highlighted, four-space indented.
4. **Reason line** — muted prose with the underlying diagnostic.

A programmatic handler receives a rich `FailureContext` (`handlers.rs:18-40`) serialized as JSON on stdin, with parallel env vars (`CLAUDINE_PROVIDER`, `CLAUDINE_FAILURE_EVENT`, `CLAUDINE_FAILURE_PHASE`, `CLAUDINE_SESSION_ID`, `CLAUDINE_TERMINATION`, `CLAUDINE_ERROR_KIND`, `CLAUDINE_SOURCE_FILE`, `CLAUDINE_ATTEMPT`). The response is parsed into a `HandlerAction`, with `resume` validated against provider support and an existing session ID (`validate_resume`, `handlers.rs:421-435`).

## Library-wide errors (`ClaudineError`)

The library enum (`claudine/lib/src/error.rs`) groups ~39 variants by subsystem, with structured fields where context matters:

```rust
#[error("MCP ambiguous match for `{query}`: {}", candidates.join(", "))]
McpAmbiguousMatch {
    query: String,
    candidates: Vec<String>,
},

#[error("protect rule parse error for pattern `{pattern}`: {source}")]
ProtectRuleParse {
    pattern: String,
    source: regex::Error,
},
```

Notable design choices:

- `#[from]` conversions for third-party errors (`std::io::Error`, `serde_json::Error`, `rusqlite::Error`, `reqwest::Error`, `chrono::ParseError`, `regex::Error`, `url::ParseError`, `MarkdownError`).
- Structured tuple-style fields where a single string loses information (`LockError { path }`, `InvalidReportingDateRange { from, to }`, all `Mcp*` variants, all `Policy*` variants).
- `SystemPromptComposition(#[from] MarkdownError)` carries the typed error so the CLI walker can render Darkmatter's rich `BlockError` report (path, line, hint, transclusion chain) instead of a flat string.

## Composition errors (`CompositionError`)

The composition enum (`claudine/lib/src/composition/error.rs`) is the largest — ~58 variants covering frontmatter problems, schema validation, sequence and loop pipelines, provider/model selection, atomic writes, and rate-limit handling. It implements `biscuit_terminal::errors::BlockError`, with bespoke `status_block` renderings for high-signal variants. Examples:

- `SchemaLoad`, `SchemaValidation`, `MissingProperties`, `UnsupportedInteractiveSchema` — schema validation tier, with structured `problems` and typed `MissingProperty` records (including `InteractiveShape` mapping for biscuit-tui prompts).
- `SequenceMissingProperties` aggregates per-step failures so a multi-step sequence can be fixed in one edit pass.
- `LoopIterationFailed` carries the structured `exit_reason` from the iteration's `session_end` JSONL row (`extra.exit_reason`: `step_timeout`, `wall_clock_timeout`, `signal`, …), so loop iteration errors surface the actual cause rather than a generic "loop failed" — see `claudine/docs/topics/timeouts.md:554-572`.
- `LoopRateLimited` maps to exit code `75` (`EX_TEMPFAIL`) so shell wrappers can distinguish transient rate-limit halts from generic non-zero exits (`composition/error.rs:26, 525-538`).

`DroppedOptional` records (`composition/error.rs:699-732`) are warnings, not errors: schema-invalid optional values are dropped with attribution across three pipeline stages (`PreValidation`, `Composition`, `PostShellExpansion`).

## Contract crate mapping

`claudine/contract/src/error.rs` runs in the deterministic, tool-free inference path. Provider detail (stderr, raw error text, exit codes) is treated as **potentially secret-bearing**: it is logged through `tracing` only and never placed in `InferenceError::message`. `classify_text` lowercases provider/stderr text and keyword-matches it onto `InferenceErrorKind::{RateLimited, Unauthorized, Timeout, Unavailable, Provider}`, while `check_security` rejects any session that attempted tool use, permission prompts, or interactive input — the contract authorizes none of that.

## Observations

- **No `anyhow`** or `color-eyre` — every error is a typed enum variant, with `thiserror::Error` derives and `#[error("...")]` format strings. The `Display` impls are the user-facing contract.
- **Bounded but unbounded-looking enums** — a prior review (`claudine/reviews/_completed/2026-04-17-comprehensive/review-glm.md:148`) flagged that `Display`/`Error` impls have no snapshot tests; a single format-string typo would go undetected.
- **Phase-tagged failures** — the `FailurePhase` discriminator is threaded from rule evaluation through handler resolution through reporting, so a single `ValidationFailure` carries enough metadata for recovery, structured logging, and rich terminal rendering without re-derivation.
- **Schema validation as composition concern** — schema errors live in `CompositionError`, not `HarnessError`, even though they are validation-shaped. The harness owns behavioral pre/post contracts; composition owns document-shape contracts.
