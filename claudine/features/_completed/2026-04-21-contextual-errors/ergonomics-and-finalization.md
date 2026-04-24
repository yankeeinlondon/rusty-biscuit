---
parent_spec: ./spec.md
parent_plan: ./plan.md
addresses: review-1.md sections #3 and #4
created: 2026-04-23
status: design
---

# Design: Ergonomics & Finalization (Contextual Errors, Phase 5)

The core redesign from [`spec.md`](./spec.md) has landed: `SystemPromptComposition` carries `#[from] MarkdownError`, the `error_walker.rs` cause-chain walker renders Darkmatter `BlockError`s at the CLI top level, the legacy `shell_expansion_error.rs` renderer and `PRE_RENDERED_MARKER` sentinel are gone, and the three headline failure paths are covered by both unit and integration tests ([`review-2.md`](./review-2.md) confirms).

Two items flagged by [`review-1.md`](./review-1.md) remain unaddressed:

- **§3 Ergonomics and Performance** — `#[from]` propagation is only half-delivered. `ClaudineError::SystemPromptComposition` gets `?`, but `CompositionError::ComposeFailed` and `CompositionError::PreFlightDiscoveryFailed` still use `#[source]`, so callers in `composition/prepare.rs` and `composition/preflight.rs` continue to hand-roll `.map_err(...)` wrappers. Enum size has also never been measured, leaving the spec-authorized `Box<MarkdownError>` escape hatch (Decision 2) un-evaluated.
- **§4 Suggestions for Finalization** — no public-facing breaking-change note has been added (plan step 4.3), the final grep-based cleanup sweep (plan step 4.4) has not been executed, and there is no recorded evidence of the three manual acceptance scenarios from the spec (plan step 4.1).

This document designs the remaining work as a single focused cleanup phase.

## Goals

1. Close the ergonomic gap on `CompositionError` so every `MarkdownError`-producing call site in the library uses `?` or an equivalent single-token conversion — without collapsing the two variants that tell the user *which phase* failed.
2. Measure `ClaudineError` and `CompositionError` enum sizes and decide whether Decision 2's `Box<MarkdownError>` escape hatch needs to be exercised.
3. Ship the feature: record the breaking `ClaudineError` payload change, run the final cleanup grep, and capture manual verification evidence for the three headline scenarios.

## Non-Goals

- The deferred harness sites at `claudine/lib/src/harness/parse.rs:964` and `claudine/lib/src/harness/audit.rs:80` remain out of scope (Decision 1, spec). They ship with the same lossiness as before.
- `CompositionError::PreFlightFailed(String)` and the other `String`-payload variants in `CompositionError` are **not** targets — none of them wrap a `MarkdownError`. They are deliberate flat-string variants for approval-handler failures, blacklist rejections, and I/O errors with no Darkmatter source.
- No further changes to the walker in `claudine/cli/src/output/error_walker.rs` or to the deleted shell renderer.

## Current State (Verified 2026-04-23)

| File | Line | Current Shape | Gap |
|------|------|--------------|-----|
| `claudine/lib/src/error.rs` | 234 | `SystemPromptComposition(#[from] MarkdownError)` | None — `?` works. |
| `claudine/lib/src/composition/error.rs` | 45 | `ComposeFailed(#[source] MarkdownError)` | No `From` impl — callers must map manually. |
| `claudine/lib/src/composition/error.rs` | 113 | `PreFlightDiscoveryFailed(#[source] MarkdownError)` | No `From` impl — callers must map manually. |
| `claudine/lib/src/composition/prepare.rs` | 18-26 | `map_compose_error` fn that hand-picks `ShellExpansion` vs. other `MarkdownError` variants | Required regardless of `#[from]` because of the `ShellExpansion` split — **this helper stays**. |
| `claudine/lib/src/composition/preflight.rs` | 58 | `collect_shell_commands(...).map_err(CompositionError::PreFlightDiscoveryFailed)?` | One awkward `.map_err` per call site where `?` would be nicer. |
| `claudine/lib/README.md` | — | No mention of the typed `SystemPromptComposition` payload change | Breaking-change note missing. |
| Enum sizes | — | Never measured | Decision 2 escape hatch un-evaluated. |

The `map_compose_error` helper in `prepare.rs` is doing real dispatch work (routing `MarkdownError::ShellExpansion` to `ShellExpansionFailed` so the CLI also gets `source_path`). `#[from]` cannot replace it — any design that attempts to do so would regress the spec's shell-path rich rendering. The target for improved ergonomics is **preflight**, plus any future call site that needs only the plain `PreFlightDiscoveryFailed` or `ComposeFailed` wrapping.

## Design

### §3.A — Resolve the `#[from]` Dilemma on `CompositionError`

Two variants of `CompositionError` wrap `MarkdownError`. `thiserror` refuses to generate two `From<MarkdownError> for CompositionError` impls because of coherence, so a blanket `#[from]` on both is impossible. Three options were considered:

| Option | Ergonomics | Semantic Clarity | Implementation Cost | Verdict |
|--------|-----------|------------------|---------------------|---------|
| **A.** Collapse `ComposeFailed` and `PreFlightDiscoveryFailed` into a single `MarkdownFailed` variant | Best — single `#[from]` | Lost — "compose failed" vs "pre-flight discovery failed" phase labels disappear from `Display` | Low | Rejected: the `Display` strings are user-observable when the walker falls back to generic rendering (e.g., if a cause chain never hits a `BlockError` impl). Phase context is a cheap win we should not throw away. |
| **B.** Add `#[from]` to the more common variant (`ComposeFailed`), keep `#[source]` on `PreFlightDiscoveryFailed` | Partial — `?` works for `ComposeFailed` only | Preserved | Low | Rejected: asymmetric, surprising for readers. |
| **C.** Keep both variants with `#[source]`, add explicit constructor methods on `CompositionError` for each | Good — one-token conversion everywhere (`CompositionError::preflight_from(e)`) | Preserved | Low | **Chosen.** |

**Decision: Option C — named constructor methods.**

Add two small inherent-impl constructors next to the enum definition:

```rust
impl CompositionError {
    /// Wrap a Darkmatter `MarkdownError` as a compose-stage failure.
    ///
    /// Prefer `map_compose_error` in `prepare.rs` for call sites that also
    /// need to route `ShellExpansion` to `ShellExpansionFailed`; this
    /// constructor is for callers that only want the plain `ComposeFailed`
    /// wrapper.
    pub fn compose_failed(err: MarkdownError) -> Self {
        CompositionError::ComposeFailed(err)
    }

    /// Wrap a Darkmatter `MarkdownError` as a pre-flight discovery failure.
    pub fn preflight_discovery_failed(err: MarkdownError) -> Self {
        CompositionError::PreFlightDiscoveryFailed(err)
    }
}
```

Call sites become:

```rust
// before
collect_shell_commands(md, opts)
    .map_err(CompositionError::PreFlightDiscoveryFailed)?;

// after (unchanged — it was already a one-token conversion)
collect_shell_commands(md, opts)
    .map_err(CompositionError::preflight_discovery_failed)?;
```

This is the honest assessment: `CompositionError::PreFlightDiscoveryFailed` is already a one-token `map_err` closure because the variant is itself a single-argument constructor function. The named constructors exist for **discoverability** (showing up in rustdoc alongside `#[error]` messages) and as the canonical entry points that future call sites should reach for. They are not a meaningful keystroke reduction.

**What this buys vs. leaves on the table:**

- **Buys:** A documented conversion API, symmetric with the `From<MarkdownError> for ClaudineError` impl at the top level; a single place to add diagnostics, tracing, or context-capture logic in the future.
- **Leaves on the table:** True `?` propagation for `CompositionError`. This is a deliberate tradeoff — it is the cost of keeping the two variants semantically distinct, and the cost is one five-character closure at each call site.

**Implementation steps:**

1. Add `impl CompositionError { ... }` block with the two constructors directly below the enum definition in `claudine/lib/src/composition/error.rs`.
2. Add rustdoc linking each constructor to its paired `#[error]` message and explaining the semantic distinction.
3. Update the two call sites:
   - `claudine/lib/src/composition/preflight.rs:58` — `.map_err(CompositionError::preflight_discovery_failed)?`
   - `claudine/lib/src/composition/prepare.rs` (inside `map_compose_error`) — use `CompositionError::compose_failed(other)` instead of direct variant construction, for consistency.
4. No test churn expected — the constructors are thin wrappers around the existing variants.

### §3.B — Enum Size Measurement & `Box<MarkdownError>` Decision

The spec's Decision 2 authorizes a one-line upgrade to `Box<MarkdownError>` if `size_of::<ClaudineError>` is problematic. This has never been measured. The deliverable is a **recorded measurement** — not a speculative boxing.

**Measurement approach:**

Add a `#[cfg(test)] mod size_checks` block to `claudine/lib/src/error.rs` and `claudine/lib/src/composition/error.rs`. Use `std::mem::size_of::<T>()` plus a threshold assertion so future regressions are caught:

```rust
#[cfg(test)]
mod size_checks {
    use super::*;

    // Threshold set generously above the current measured size.
    // If this assertion fails, investigate — large error enums bloat
    // every Result in the hot path.
    #[test]
    fn claudine_error_size_is_bounded() {
        let size = std::mem::size_of::<ClaudineError>();
        assert!(
            size <= 256,
            "ClaudineError grew to {size} bytes; consider Box<MarkdownError>"
        );
    }
}
```

**Decision rule:**

- Measured size ≤ 256 bytes: **do not box.** Keep the enum as-is. The spec authorizes boxing but does not require it; inline storage keeps pattern-matching and `From<MarkdownError>` idiomatic.
- Measured size > 256 bytes: box `MarkdownError` in **both** `ClaudineError::SystemPromptComposition` and the `CompositionError` variants. If we box one, we box all three for consistency. Update the `#[from]` impl to a manual `From<MarkdownError> for ClaudineError` that calls `Box::new`.

**Why 256 bytes:** arbitrary but defensible. `Result<T, ClaudineError>` lives on every composition/system-prompt code path; keeping the error tail in one cache line (64 bytes) is unrealistic given `MarkdownError`'s structured payloads, so 4×64 = 256 bytes is the working ceiling before we pay the boxing cost.

**Implementation steps:**

1. Add `size_checks` mod to `claudine/lib/src/error.rs`.
2. Add matching mod to `claudine/lib/src/composition/error.rs` asserting on `CompositionError`.
3. Run `cargo test -p claudine size_checks` and record the measured sizes in the feature's completion record (see §4.C).
4. If either assertion fails, implement the `Box<MarkdownError>` upgrade in a follow-up PR rather than wedging it into this cleanup.

### §4.A — Breaking-Change Documentation

`ClaudineError::SystemPromptComposition(String)` → `SystemPromptComposition(MarkdownError)` is a breaking change for any external consumer of `claudine/lib`. Per plan step 4.3, this must be documented. There is no `CHANGELOG.md` in `claudine/lib/`; the nearest stable surface is `claudine/lib/README.md`.

**Proposed README section** (to be added near the bottom, above any existing status/roadmap section):

```markdown
## Breaking Changes

### 2026-04 — Typed `SystemPromptComposition` payload

`ClaudineError::SystemPromptComposition` previously carried a `String`
payload produced by calling `.to_string()` on Darkmatter's
`MarkdownError`. It now carries the typed `MarkdownError` directly:

    // Before
    ClaudineError::SystemPromptComposition(String)

    // After
    ClaudineError::SystemPromptComposition(darkmatter::markdown::MarkdownError)

This preserves line numbers, file paths, transclusion chains, and
contextual hints that were previously flattened to text. External
consumers that pattern-match on this variant or inspect its payload
must update to the typed form. The `Display` impl still renders a
human-readable summary, so string-only consumers (`println!("{err}")`,
log lines) are unaffected.

A blanket `From<darkmatter::markdown::MarkdownError> for ClaudineError`
impl is provided via `thiserror`'s `#[from]`, so call sites that
previously did `.map_err(|e| ...SystemPromptComposition(e.to_string()))`
can collapse to `?`.
```

### §4.B — Final Cleanup Sweep

Plan step 4.4 prescribes a grep-based cleanup. Run these queries and commit fixes for any hits that fall within the feature's scope (system-prompt, composition, preflight, the CLI error path):

```bash
rg -n "PRE_RENDERED_MARKER" claudine
rg -n "pretty_markdown_report|pretty_or_report" claudine
rg -n "SystemPromptComposition\(String\)" claudine
rg -n "is_pre_rendered" claudine
# In the targeted paths only — not monorepo-wide.
rg -n "\.to_string\(\)" \
    claudine/lib/src/system_prompt \
    claudine/lib/src/composition \
    claudine/cli/src/commands/compose.rs \
    claudine/cli/src/commands/sequence.rs \
    claudine/cli/src/commands/wrap/mod.rs
```

**Expected outcome:** the first four greps return zero hits (Review 2 already confirms this). The last grep will return hits for `to_string()` calls that are unrelated to `MarkdownError` flattening (formatting labels, path display, etc.) — these are not in scope; record them as reviewed-and-retained in the completion record.

**Out-of-feature `to_string()` hits** that should be triaged but **not** changed as part of this cleanup:

- `claudine/lib/src/composition/preflight.rs:73` — `CompositionError::PreFlightFailed(e.to_string())` wraps an approval handler error, not a `MarkdownError`. Flattening is appropriate here; the source is an opaque handler result.
- `claudine/lib/src/composition/preflight.rs:148` — same category.

### §4.C — Manual Verification Record

Plan step 4.1 requires running three manual acceptance scenarios. The deliverable is a short checked-in record (inline in this document's "Verification Log" section) with:

1. **Scenario A — Denied shell command in compose.** Run `claudine compose` against a fixture markdown file containing `::shell rm -rf /`. Expect a rich `BlockError` report with the command, the source line, and a blacklist hint.
2. **Scenario B — Broken system prompt.** Run any wrapper command with `--append-system-prompt` pointing to a file with a bad `::shell` directive. Expect a rich `BlockError` report with the system-prompt path, line, and hint.
3. **Scenario C — Transclusion cycle.** Run `claudine compose` against two fixture files that transclude one another. Expect a rich `BlockError` report with the full file chain and line numbers.

For each scenario, capture: command run, exit code, and the first ~20 lines of stderr (ANSI-stripped). These three scenarios are already covered by integration tests in `claudine/cli/tests/contextual_errors.rs`, so the manual pass is low-risk confirmation that the wired-up CLI binary matches test expectations.

**Verification Log template** (fill in during execution):

```markdown
### Scenario A — Denied shell command in compose
- Command: `claudine compose /tmp/fx-shell.md`
- Exit: 1
- Stderr excerpt: <paste first 20 lines, ANSI-stripped>
- Result: PASS / FAIL

### Scenario B — Broken system prompt
- Command: `claudine compose --codex --append-system-prompt /tmp/fx-sp.md /tmp/fx.md`
- Exit: 1
- Stderr excerpt: <paste>
- Result: PASS / FAIL

### Scenario C — Transclusion cycle
- Command: `claudine compose /tmp/fx-a.md`
- Exit: 1
- Stderr excerpt: <paste>
- Result: PASS / FAIL
```

## Validation Checkpoints

After each section is implemented, these commands must pass:

```bash
# §3.A — ergonomics refactor
cargo check -p claudine
cargo test -p claudine composition::error
cargo clippy -p claudine -- -D warnings

# §3.B — size measurements
cargo test -p claudine size_checks

# §4.A — docs
# Human review of claudine/lib/README.md diff

# §4.B — cleanup sweep
rg -n "PRE_RENDERED_MARKER|pretty_markdown_report|pretty_or_report|is_pre_rendered" claudine
# expect: 0 hits outside of feature spec/plan/review markdown files

# §4.C — manual verification
# Run three scenarios, log results inline in this file

# Full regression guard
cargo test -p claudine && cargo test -p claudine-cli
cargo clippy -p claudine -p claudine-cli -- -D warnings
```

## Dependency Graph & Sequencing

```
§3.A (constructors)  ──┐
                       ├──→  §4.B (cleanup sweep) ──→  §4.C (manual verify) ──→  done
§3.B (size checks)  ──┤
                       │
§4.A (README note)  ──┘
```

§3.A, §3.B, and §4.A are independent and can run in parallel. §4.B depends on §3.A finishing (to confirm the new constructors are in use at the expected sites). §4.C is the final gate.

## Acceptance Criteria

The remaining work is "done" when:

1. `CompositionError` has documented `compose_failed` and `preflight_discovery_failed` constructor methods, and both non-shell call sites use them.
2. `size_checks` modules exist in both `claudine/lib/src/error.rs` and `claudine/lib/src/composition/error.rs`, pass in CI, and either (a) confirm the enums are under 256 bytes, or (b) document the decision to box `MarkdownError`.
3. `claudine/lib/README.md` contains the breaking-change note verbatim (or an equivalent rewording) covering the typed `SystemPromptComposition` payload.
4. The §4.B grep sweep returns no in-scope hits, with any triaged-and-retained hits documented inline.
5. The §4.C Verification Log is filled in with three PASS records.
6. `cargo clippy -p claudine -p claudine-cli -- -D warnings` and the full test suite pass.

## Risks & Mitigations

| Risk | Likelihood | Mitigation |
|------|-----------|------------|
| Enum size assertion fails in CI for developers with a different feature-flag combination. | Low | Use a generous 256-byte ceiling; document the `Box<MarkdownError>` escape hatch so a failing assertion has a clear fix path. |
| Manual verification uncovers a regression the integration tests missed (e.g., TTY-dependent rendering). | Low | Integration tests already cover the three scenarios with `NO_COLOR=1`. Any mismatch is a test-coverage gap to fix, not a design flaw. |
| Adding constructor methods duplicates the `#[from]` style asymmetrically with `ClaudineError`. | Medium | Documented explicitly in §3.A as a deliberate tradeoff; rustdoc on both sites explains why. |
| README note drifts from reality if the enum shape changes again. | Low | Tie the note's dated heading (`2026-04`) to this feature's ship date so future breaking changes are appended rather than overwriting. |

## Verification Log

_To be filled in when §4.C executes. Left blank here so the template is visible._

### Scenario A — Denied shell command in compose
- Command: _pending_
- Exit: _pending_
- Stderr excerpt: _pending_
- Result: _pending_

### Scenario B — Broken system prompt
- Command: _pending_
- Exit: _pending_
- Stderr excerpt: _pending_
- Result: _pending_

### Scenario C — Transclusion cycle
- Command: _pending_
- Exit: _pending_
- Stderr excerpt: _pending_
- Result: _pending_
