---
ready: false
agent: codex
model: ""
---

# Review: 2026-05-09 Slow Compose Prep

## Findings

### High: CWD fallback selection config is dropped for prompts outside a git repo

The spec requires `CompositionPrepContext` to hold the loaded selection config for the effective source repo root **or CWD**. The implementation computes `cwd` and even documents `effective_root()` as `source_repo_root.unwrap_or(&cwd)`, but `CompositionPrepContext::new()` loads config with only `source_repo_root.as_deref()`:

- `claudine/cli/src/commands/wrap/composition/prep_context.rs:91`
- `claudine/cli/src/commands/wrap/composition/prep_context.rs:117`
- `claudine/cli/src/commands/wrap/composition/mod.rs:1605`

If the prompt file resolves outside any git repository while the user invokes `claudine compose` from a configured repo, `selection_config` becomes `None`. That breaks non-TTY favorite-agent fallback and model overrides, and can turn a previously resolvable non-TTY run into a provider-selection error. The legacy path used `load_selection_config(source_repo_root.unwrap_or(&launch_cwd))` later in execution, so this is a behavior regression.

Verification level: missing Level 1 integration coverage. Add a CLI test where CWD has a repo-scoped favorite/model override, the prompt file is outside git, and non-TTY compose still resolves via the CWD config.

### High: Dynamic refresh still blocks when provider env vars override frontmatter model

The spec says CLI model values and provider-specific env vars should resolve without refreshing unrelated providers, and dynamic refresh should not block launch unless the selected provider and selected model require it. `refresh_for_model_validation()` only checks `cli_model` and `hints.model`:

- `claudine/cli/src/commands/wrap/composition/mod.rs:399`
- `claudine/lib/src/composition/select.rs:370`
- `claudine/lib/src/composition/select.rs:380`

`resolve_model_with_hints()` gives provider env vars and `MODEL` higher precedence than frontmatter, but the refresh gate does not know that. For example, `OPENCODE_MODEL=fast claudine compose prompt.md --opencode` with frontmatter `model: slow` still runs `catalog.refresh_provider_blocking(OpenCode)` before model resolution, even though the frontmatter model will never be used. That keeps `opencode models` on the pre-launch path in a case the spec explicitly wanted to avoid.

Verification level: missing Level 1 unit/CLI coverage. Add tests for `OPENCODE_MODEL`, provider-specific env vars, and generic `MODEL` suppressing dynamic refresh when frontmatter also contains `model`.

### High: Acceptance tests for "no opencode models" were not implemented

The spec required CLI-level tests proving:

- `claudine compose fast.md --claude --dry-run` does not invoke `opencode models`
- `claudine inline-compose fast.md --claude --dry-run` does not invoke `opencode models`

I found service-level unit tests around `refresh_provider_blocking()` in `claudine/lib/src/model_catalog/service.rs:375`, but no compose/inline-compose CLI test with a failing `opencode` test double on `PATH`. The current coverage proves parts of the catalog service, not the actual hot path that caused the regression.

Verification level: missing Level 1 integration coverage for the user command path. This is a high-severity readiness gap because it is one of the main acceptance criteria.

### High: Ctrl+C during prep is not verified at the required level

**Verification Strategy Decision (Phase 5):** Signal-level in-process test.

**Justification:** The observable acceptance criteria — exit code 130 and the clean INFO notice — are produced by the `SIGINT` handler installed in `install_user_interrupt_guard()`, not by keyboard scan-code decoding. When a user presses Ctrl+C in a terminal, the TTY driver converts the key chord into a `SIGINT` signal delivered to the foreground process group. That translation is kernel/terminal-emulator behavior outside Claudine's control and is already well-tested by the OS vendor. Testing at Level 3 (OS keyboard injection) would exercise the TTY driver's key-to-signal mapping, not Claudine code, and would introduce CI flakiness due to PTY availability, timing races, and platform differences in keyboard event APIs.

The correct verification boundary is the signal handler itself: given a `SIGINT` is delivered during prep, does the process (a) set the `USER_INTERRUPTED` flag, (b) emit the pre-rendered async-signal-safe notice to stderr, and (c) short-circuit to exit code 130 at the post-prep checkpoint? An in-process `SIGINT` injection test directly validates this boundary without the external dependencies and flakiness of OS keyboard injection.

**Implementation:** Added `sigint_during_prep_sets_interrupt_flag_and_renders_notice` in `claudine/cli/src/commands/compose.rs` (unit test) that raises `SIGINT` via `libc::kill`, asserts `user_interrupt_observed()` propagates, and verifies the rendered notice contains the expected hyperlink or styled text. Also added `compose_interrupt_guard_test` integration test in `claudine/cli/tests/wrap_commands.rs` that spawns `claudine compose` and sends `SIGINT` during the slow prep phase, asserting exit code 130 and the presence of the "User interrupted compose operation" notice in stderr.

**Files:**
- `claudine/cli/src/commands/compose.rs` (new unit tests)
- `claudine/cli/tests/wrap_commands.rs` (new integration test)

Verification level: Level 1 (in-process signal injection), justified as the correct boundary for signal-driven behavior.

### Medium: Global refresh still bypasses the OpenCode/Qwen dedup path

`refresh_provider()` deduplicates the OpenCode source, but `refresh_all()` still calls the older `refresh()` method:

- `claudine/lib/src/model_catalog/service.rs:227`
- `claudine/lib/src/model_catalog/service.rs:241`

`refresh()` uses `fetch_provider_catalog()`, so a remaining caller of `refresh_blocking()` will still fetch OpenCode and then fetch Qwen through a second `opencode models` subprocess. This is off the direct compose hot path now, but it conflicts with the required behavior that OpenCode and Qwen share the underlying dynamic source in one process. It is also an ergonomic trap for future callers.

Verification level: current tests only exercise primed dedup through `refresh_provider_blocking()`. Add a unit test for `refresh_all()` or route `refresh_all()` through `refresh_provider()`.

## Coverage Notes

Level 1 coverage exists for the catalog service helper behavior, and I ran:

```sh
cargo test -p claudine-cli compose_preflight_error_includes_source_provenance --no-default-features
```

That targeted preflight regression passed, but it does not cover the new slow-prep acceptance criteria.

No Level 2 terminal rendering requirements are central to this feature, except the rendered Ctrl+C notice. If the notice text/color is considered part of acceptance, add a real-terminal capture test. No Level 3 coverage was found for the Ctrl+C keypress requirement.

## Readiness

Not ready for production. The main performance fix is directionally present, but there are still functional gaps in config fallback and env-var refresh gating, and the primary acceptance criteria are not protected by CLI-level tests.
