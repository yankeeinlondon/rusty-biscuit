# Frontmatter Interpolation Review

## Findings

### 1. CLI frontmatter-only `ctx.*` interpolation is broken

The library implementation supports `ctx.*` lookups during frontmatter interpolation, but the CLI path can fail to capture the required runtime context groups when those references exist only in frontmatter.

- [`darkmatter/cli/src/commands.rs:353-361`](/Users/ken/.claudine/worktrees/rusty-biscuit/darkmatter/darkmatter/cli/src/commands.rs#L353) builds the shared compose context with `ComposeContext::capture_for_content(..., md.content())`.
- `md.content()` is the body only, so frontmatter references are already stripped.
- [`darkmatter/lib/src/markdown/compose/context/capture.rs:121-155`](/Users/ken/.claudine/worktrees/rusty-biscuit/darkmatter/darkmatter/lib/src/markdown/compose/context/capture.rs#L121) only scans the provided string for `ctx.*`, so repo/docs/os/hardware groups referenced exclusively from frontmatter are never captured.

Observed repro from this workspace:

```bash
cargo run -q -p darkmatter-cli -- compose /tmp/fm_ctx.md --output markdown
```

With:

```yaml
---
repo_root_copy: "{{ctx.repo_root}}"
repo_name_copy: "{{ctx.repo}}"
---
repo_root={{repo_root_copy}}
repo_name={{repo_name_copy}}
```

Observed output:

```text
repo_root=
repo_name=
```

Control case with the same `ctx.*` references in the body works and returns the expected repo values. That makes this a real CLI regression, not just a missing test.

Recommendation:

- Capture context from the raw source text before frontmatter is stripped, or extend the demand-driven scan to include frontmatter values.
- Add a CLI integration test covering frontmatter-only `ctx.repo*` interpolation.

### 2. Nested external/inherited state does not participate in frontmatter interpolation

The design says frontmatter interpolation runs after inherited/external state has been merged into the current document, but the mutable frontmatter-prep path only fills top-level missing keys.

- [`darkmatter/lib/src/markdown/compose/mod.rs:297-309`](/Users/ken/.claudine/worktrees/rusty-biscuit/darkmatter/darkmatter/lib/src/markdown/compose/mod.rs#L297) copies `external_state` into frontmatter only when the top-level key is missing or null.
- [`darkmatter/lib/src/markdown/compose/types.rs:267-270`](/Users/ken/.claudine/worktrees/rusty-biscuit/darkmatter/darkmatter/lib/src/markdown/compose/types.rs#L267) documents `external_state` as using deep-merge semantics.
- [`darkmatter/lib/src/markdown/compose/state.rs:13-33`](/Users/ken/.claudine/worktrees/rusty-biscuit/darkmatter/darkmatter/lib/src/markdown/compose/state.rs#L13) already has a deep-merge implementation that the final `EffectiveState` uses.

Observed repro:

```yaml
---
meta:
  author: Local
spec: "{{meta.base}}/spec.md"
---
{{spec}}
```

Run with:

```bash
cargo run -q -p darkmatter-cli -- compose /tmp/nested_state.md --state '{meta:{base:"/root",author:"Parent"}}' --output markdown
```

Observed output:

```text
/spec.md
```

But a body-only control using `{{meta.base}}` with the same `--state` returns `/root`, which shows the nested external state exists in the final `EffectiveState` but not in the pre-interpolation frontmatter state.

This also impacts inherited parent state for child documents, because child `external_state` is routed through the same shallow top-level fill path before frontmatter interpolation runs.

Recommendation:

- Reuse the existing deep-merge logic when mutating frontmatter from `external_state` before `FrontmatterInterpolation`.
- Add a regression test for nested inherited state, not just flat top-level keys.

## Coverage Gaps

The tech design explicitly asked for compose integration tests for child-derived frontmatter, interpolated `prologue`/`epilogue`, and page-block visibility based on interpolated frontmatter values; see [`darkmatter/features/2026-03-30-fm-interpolation/tech-design.md:531-545`](/Users/ken/.claudine/worktrees/rusty-biscuit/darkmatter/darkmatter/features/2026-03-30-fm-interpolation/tech-design.md#L531).

The current integration coverage in [`darkmatter/lib/src/markdown/compose/mod.rs:3321-3444`](/Users/ken/.claudine/worktrees/rusty-biscuit/darkmatter/darkmatter/lib/src/markdown/compose/mod.rs#L3321) covers:

- the basic spec example
- `--set` overrides
- arrays/objects
- disabling the stage
- body code-span skipping
- report counting/summary/merge

What is still missing:

- child document frontmatter deriving from parent state
- interpolated `prologue` / `epilogue` paths
- page blocks consuming interpolated frontmatter values
- CLI integration coverage for frontmatter interpolation
- regression coverage for nested object state in `--state` / inherited external state

These omissions are why both verified regressions above were able to land.

## Ergonomics / Performance Suggestions

### 1. Unify state preparation semantics

Right now frontmatter interpolation has its own pre-state merge behavior, while body interpolation uses the final `EffectiveState` merge behavior. That semantic split is already causing drift. The better direction is to centralize “what state is visible at compose time” behind one merge path and build the frontmatter-interpolation seed state from that canonical result.

Benefits:

- fixes the nested-state bug
- makes child inheritance semantics easier to reason about
- reduces the chance that docs, CLI behavior, and library behavior diverge again

### 2. Scan raw source once for demand-driven `ctx` capture

The current demand-driven capture idea is good, but it should operate on the raw document source, not just the post-frontmatter body. That keeps the optimization while making frontmatter interpolation correct.

Benefits:

- fixes the CLI/frontmatter `ctx.*` bug without forcing full context capture
- avoids introducing a second “scan the frontmatter too” code path
- keeps context-capture cost proportional to actual usage

### 3. Update the stale interpolation docs

The design called for updating the main interpolation docs, but [`darkmatter/docs/inline/interpolation.md:3-9`](/Users/ken/.claudine/worktrees/rusty-biscuit/darkmatter/darkmatter/docs/inline/interpolation.md#L3) still describes only body interpolation and even references `.transform()`. The dedicated `fm-interpolation` doc exists, but the main interpolation doc is now misleading and should be brought in sync.

## Verification Notes

Checked with:

- `cargo test -p darkmatter frontmatter_interpolation -- --nocapture`
- direct CLI repros via `cargo run -q -p darkmatter-cli -- compose ...`
