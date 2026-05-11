# Debug Design: Research Library & CLI

## CLI Skill Alignment

This document reviews the research CLI and library's tracing and debugging infrastructure against the [CLI skill](../.opencode/skill/cli/cli-best-practices.md).

### Key CLI Skill Principles

1. **`--verbose` is NOT debug logging** — `-v` produces richer human-facing output; it should not map to trace-level logging
2. **Separate `--debug <level>` flag** — `trace`, `debug`, `info`, `warn`, `error` alongside `RUST_LOG` env
3. **STDOUT = data, STDERR = metadata/traces** — tracing output must not corrupt data pipes
4. **Spans for major boundaries** — command entry, session, external process execution, network calls, retries, validation, expensive operations
5. **Timing + outcome fields on spans** — duration, exit code, success/failure
6. **Structured fields over prose** — low-cardinality, stable identifiers
7. **No secrets in traces** — redact tokens, raw prompts, sensitive payloads

---

## Current State

### Tracing Infrastructure

**Dependencies**:
- `tracing` v0.1 — core tracing facade
- `tracing-subscriber` v0.3 with `env-filter` and `json` features

### CLI Initialization (`research/cli/src/main.rs:179-221`)

```rust
fn init_tracing(verbose: u8, json: bool) {
    let base_filter = match std::env::var("RUST_LOG") {
        Ok(filter) => filter,
        Err(_) => match verbose {
            0 => "warn".to_string(),
            1 => "warn,research=info,shared::tools=info".to_string(),
            2 => "info,research=debug,shared=debug".to_string(),
            _ => "debug,research=trace,shared=trace".to_string(),
        },
    };
    // ...
}
```

**Current verbosity levels**:
| Flag | Filter | Alignment with CLI Skill |
|------|--------|------------------------|
| (none) | `warn` | Correct default |
| `-v` | `warn,research=info,...` | ❌ CLI skill: `-v` is human output, NOT debug |
| `-vv` | `info,research=debug,...` | ❌ CLI skill: `-vv` is human output, NOT debug |
| `-vvv` | `debug,research=trace,...` | ❌ CLI skill: `-vvv` is human output, NOT debug |

**Critical misalignment**: The CLI conflates `-v`/`-vv`/`-vvv` with `RUST_LOG` debug levels. Per CLI skill: `--verbose` produces richer human-facing output only. Debug logging requires `--debug <level>` or `RUST_LOG`.

---

## Issues & Recommendations

### 1. CRITICAL: `--verbose` Conflated with Debug Logging

**Issue** (`main.rs:13-15`, `main.rs:183-196`): `-v`/`-vv`/`-vvv` directly sets `RUST_LOG` filter levels. The CLI skill explicitly states:

> `--verbose` is for richer human-facing output only; it should NOT map directly to debug reporting

**Recommendation**: Redesign the flag architecture:

```rust
// args.rs - separate verbose from debug
#[derive(Parser)]
struct Cli {
    /// Increase output verbosity (-v, -vv, -vvv)
    #[arg(short = 'v', action = ArgAction::Count, global = true)]
    verbose: u8,

    /// Debug trace level: trace, debug, info, warn, error
    #[arg(long, global = true, default_value_t = DebugLevel::Warn)]
    debug: DebugLevel,

    /// Output logs as JSON (for both user output and traces)
    #[arg(long, global = true)]
    json: bool,
}

enum DebugLevel {
    Trace, Debug, Info, Warn, Error
}
```

And in `init_tracing`:
```rust
fn init_tracing(debug_level: DebugLevel, json: bool) {
    // Precedence: RUST_LOG > --debug > default
    let filter = std::env::var("RUST_LOG")
        .ok()
        .or_else(|| match debug_level {
            DebugLevel::Trace => Some("trace".to_string()),
            DebugLevel::Debug => Some("debug".to_string()),
            DebugLevel::Info => Some("info".to_string()),
            DebugLevel::Warn => Some("warn".to_string()),
            DebugLevel::Error => Some("error".to_string()),
            DebugLevel::Off => None, // No tracing
        })
        .unwrap_or_else(|| "warn".to_string());
    // ...
}
```

---

### 2. CRITICAL: `--json` Flag Conflates User Output and Tracing Format

**Issue** (`main.rs:19`, `main.rs:199-220`): The `--json` flag controls both user-facing output AND tracing format. Per CLI skill, these should be independent.

**Recommendation**: `--json` should only affect the `println!()` output. Tracing format should be controlled separately or via `RUST_LOG`.

---

### 3. Tracing Output Destination

**Issue** (`main.rs:203`, `main.rs:216`): The tracing subscriber writes to `std::io::stderr`. This is correct per CLI skill (traces go to STDERR to keep STDOUT clean for data). However, the `println!()` calls throughout the CLI (`main.rs:260-276`, etc.) write to STDOUT. This separation is correct but needs verification.

**Verification needed**: Ensure all user-facing status/progress uses `println!()` to STDOUT, and all tracing uses the STDERR-bound subscriber.

---

### 4. Missing Span Boundaries for Major Operations

**Issue**: The CLI skill recommends spans for:
- Command entry
- Session/run
- External process execution
- Network calls
- Retries
- Validations
- Expensive operations

The current `#[instrument]` usage covers some of these:
| Operation | Current Instrumentation |
|-----------|----------------------|
| `list()` | `#[instrument]` ✓ |
| `list_with_migrate()` | `#[instrument]` ✓ |
| `link()` | `#[instrument]` ✓ |
| `research()` | `#[instrument]` ✓ |
| `run_agent_prompt_task()` | `#[instrument]` ✓ |
| `run_prompt_task()` | ❌ No instrumentation |
| `run_question_task()` | ❌ No instrumentation |
| `run_changelog_agent_task()` | ❌ No instrumentation |
| `run_changelog_completion_task()` | ❌ No instrumentation |
| Changelog aggregation | ❌ No instrumentation |

**Recommendation**: Add spans to all major operations. At minimum:
```rust
#[instrument(skip_all, fields(
    task_name = %name,
    output_dir = %output_dir.display(),
    total_tasks = total,
))]
async fn run_prompt_task(...) { ... }
```

---

### 5. Missing Timing + Outcome Fields on Spans

**Issue**: The CLI skill recommends:
> Record latency at useful boundaries: total command runtime, subcommand runtime, external process execution, network requests, retries, validation passes, parsing, rendering, and file I/O.
> Include outcome fields: success/failure, exit code, retry count, timeout, cancellation, fallback mode.

Current spans lack outcome recording. `run_agent_prompt_task` has `#[instrument]` with `task`, `filename`, `prompt_len` fields, but no timing or outcome fields.

**Recommendation**: Add span events with timing and outcomes:
```rust
async fn run_agent_prompt_task<M>(...) -> PromptTaskResult {
    let start = Instant::now();
    
    // ... task execution ...
    
    let elapsed = start.elapsed();
    span.record("duration_ms", elapsed.as_millis() as u64);
    span.record("outcome", match &metrics {
        Some(_) => "success",
        None => "failed",
    });
    
    PromptTaskResult { metrics }
}
```

---

### 6. Human-Readable Output from Library is Mixed with Tracing

**Issue** (`lib.rs:408`, `lib.rs:2607`, etc.):
```rust
tracing::info!("✓ Extracted when_to_use from SKILL.md frontmatter");
tracing::info!("✓ Updated metadata.when_to_use");
```

These go to STDERR via tracing subscriber. At default verbosity (`warn`), users never see them. But these are user-facing status messages, not developer debugging traces.

**Recommendation**: Distinguish between:
- **Traces** (developer debugging) → `tracing::info!()` via subscriber to STDERR
- **User status** (always visible) → `println!()` to STDOUT or `eprintln!()` to STDERR

For validation success/failure that requires user attention, use `eprintln!()` directly. Reserve `tracing` for internal flow.

---

### 7. No `--debug <level>` Flag

**Issue**: The CLI has no explicit debug flag. Users must use `RUST_LOG` env var, which is not documented in `--help` output.

**Recommendation**: Add `--debug` flag (see issue #1) and document in `--help`:
```
    -d, --debug <LEVEL>    Enable debug traces: trace, debug, info, warn, error
                           (default: warn, use RUST_LOG env to override)
```

---

### 8. Verbose Flag Help Text Should Be Updated

**Issue** (`main.rs:13-15`):
```rust
/// Increase verbosity (-v, -vv, -vvv)
#[arg(short = 'v', action = ArgAction::Count, global = true)]
log_verbosity: u8,
```

The help text says "Increase verbosity" but this maps to debug levels. Per CLI skill:

> Help text and docs must describe `--verbose` and debug logging separately; never imply that `-vv` means "debug"

**Recommendation**: Update help text:
```rust
/// Increase output verbosity (-v, -vv, -vvv) for richer console output
```

And add separate `--debug` flag for tracing control.

---

### 9. Secrets and Sensitive Data in Traces

**Issue**: `TracingPromptHook` logs tool call arguments and results:
```rust
async fn on_tool_call(&self, tool_name: &str, tool_call_id: Option<String>, _internal_call_id: &str, args: &str) -> ToolCallHookAction {
    info!(
        tool.args = %args,  // Tool args may contain sensitive data
```

**Recommendation**: Never log raw tool arguments. Use redaction or safe summaries:
```rust
info!(
    tool.name = %tool_name,
    tool.args_len = args.len(),  // Count only, not content
    "Invoking tool"
);
```

For tool results:
```rust
info!(
    tool.name = %tool_name,
    tool.result_len = result.len(),
    tool.result_truncated = truncated,
    // Never log tool.result_preview with actual content
    "Tool returned result"
);
```

---

### 10. Structured Fields vs Prose in Traces

**Issue**: Many traces use prose format:
```rust
tracing::info!("✓ Extracted when_to_use from SKILL.md frontmatter");
```

**Recommendation**: Use structured fields per CLI skill:
```rust
tracing::info!(
    field = "when_to_use",
    source = "SKILL.md frontmatter",
    status = "extracted",
    "Metadata field extracted"
);
```

---

### 11. Changelog Aggregation Missing Trace Spans

**Issue** (`changelog/aggregator.rs:87`): `aggregate_version_history()` silently handles failures from GitHub, registry, and file sources. No visibility into which sources succeeded/failed at debug level.

**Recommendation**: Add debug spans:
```rust
#[instrument(skip(client), fields(
    library_name = %library_name,
    package_manager = %package_manager,
))]
pub async fn aggregate_version_history(...) {
    let (github_result, registry_result, file_result) = tokio::join!(...);
    
    debug!(
        sources.github = github_result.is_ok(),
        sources.registry = registry_result.is_ok(), 
        sources.file = file_result.is_ok(),
        "Aggregation sources completed"
    );
}
```

---

### 12. Token Metrics Not Recorded in Traces

**Issue**: `PromptMetrics` (input_tokens, output_tokens, total_tokens, elapsed_secs) are printed via `println!()` but never recorded in tracing spans.

**Recommendation**: Add metrics as structured span fields:
```rust
#[instrument(skip_all, fields(
    task = %name,
    prompt_len = prompt.len(),
))]
async fn run_prompt_task(...) {
    // ...
    let metrics = PromptMetrics { ... };
    
    // Record in span on completion
    span.record("input_tokens", metrics.input_tokens);
    span.record("output_tokens", metrics.output_tokens);
    span.record("duration_ms", (elapsed * 1000.0) as u64);
}
```

---

## Summary of Recommendations

| Priority | Issue | CLI Skill Reference |
|----------|-------|---------------------|
| **Critical** | `--verbose` conflates with debug logging | "verbose is NOT debug" |
| **Critical** | No `--debug <level>` flag | "prefer --debug <level>" |
| **Critical** | `--json` controls both output and tracing | Separate concerns |
| High | Library traces mixed with user status | "traces to STDERR, data to STDOUT" |
| High | Missing span boundaries | "spans for major boundaries" |
| High | Missing timing + outcome fields | "latency at boundaries, outcome fields" |
| High | Tool args/results may contain secrets | "never emit secrets" |
| Medium | Help text implies `-vv` = debug | "never imply -vv means debug" |
| Medium | Changelog aggregation no traces | "network calls need spans" |
| Medium | Token metrics not in traces | "record metrics at boundaries" |
| Low | Prose traces instead of structured | "structured fields over prose" |

---

## Test Infrastructure

The project uses `tracing-test` for testing tracing output:

```rust
tracing_test::traced_test();
```

Per CLI skill, for integration tests verifying tracing behavior, ensure `NO_COLOR=1` is set for snapshot tests, and use `assert_cmd::Command` for end-to-end CLI tests that exercise the full trace pipeline.
