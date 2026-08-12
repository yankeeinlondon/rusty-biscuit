---
$schema: feature-review.yaml
ready: false
agent: codex/default
created: 2026-08-12T20:04:08+01:00
spec: 2026-08-12-ctx-launch-anchor/spec.md
implemented: false
description: A **fix** review of `2026-08-12-ctx-launch-anchor/spec.md`
fix: 2026-08-12-ctx-launch-anchor/review-1.md
---

# Review 1: Ctx Launch Anchor

## Verdict

The fix is **not ready for production**. The invocation-owned launch capture,
direct/inline/loop relocation matrix, sequence graph handling, system-prompt
composition, source-relative file resolution, and capture-owner inventory are
implemented and pass focused Level 1 verification. However, one canonical
passthrough route composes its provider memory file before applying the resolved
provider/model environment, so target-dependent context is still wrong on that
route. The exact epoch-reuse proof required by AC5 is also incomplete, and the
flagship direct/inline/loop CLI matrix does not run on Windows.

## Findings

### 1. High — direct-wrapper passthrough context omits the resolved target environment

`materialize_passthrough_harness_seed` captures the invocation's launch context
and immediately composes the provider memory file, but it accepts no resolved
environment overrides and returns an empty `env_overrides` vector
(`cli/src/commands/wrap/overlay.rs:40-78`). Its caller already has the selected
`provider`, `profile`, and completed child `EnvPlan`, yet passes only the
invocation and source context into materialization
(`cli/src/commands/wrap/wrapper_stages.rs:383-416`).

This violates D1's requirement that every canonical launch capture apply the
resolved target's agent/model layer. For a command such as `claudine claude ...`
whose `CLAUDE.md` activates the passthrough harness, `{{ ctx.agent }}`,
`{{ ctx.model }}`, `{{ env.AGENT }}`, and `{{ env.MODEL }}` in the memory file's
body or frontmatter are composed from the invocation environment rather than
the already-selected Claude target. They can therefore resolve to
`unknown`/`default`, or to conflicting ambient values. The existing passthrough
tests verify launch area/repository and `ctx.os`, but never any target-dependent
root.

Required change:

- Pass the resolved provider/model environment layer into passthrough seed
  materialization and apply it to the launch context before `compose_with`.
- Preserve the source-owned `FileResolutionContext` exactly as the current code
  does.
- Add a Level 1 test through `detect_wrapper_harness` with conflicting ambient
  `AGENT`/`MODEL` values. Assert all four target-dependent roots in composed
  frontmatter and lifecycle context, not only host or repository facets.

### 2. Medium — AC5 does not prove that every consumer received the shared epoch snapshot

AC5 specifically requires Claudine work accounting to prove one construction,
zero or more retained-evidence extensions, zero ambient fallbacks, **and that
each consumer seam observed a populated prepared context**. The implemented
work snapshot records constructions, extensions, and fallbacks, but has no
consumer-observation counters (`lib/src/invocation_context.rs:49-79`).

The principal test reads `epoch.as_object()` and labels that value “preflight”
instead of invoking the shell-preflight consumer, then calls canonical prepare
twice (`cli/src/commands/compose/prep/tests.rs:104-179`). A separate lifecycle
helper test proves that a missing value increments the fallback counter
(`cli/src/commands/wrap/composition/tests.rs:63-88`). The real CLI matrix proves
that the resulting values agree, but it cannot distinguish one shared snapshot
from separately constructed equal values, which is the exact false positive AC5
was written to prevent.

Required change:

- Add Claudine-only, per-consumer work observations for the preflight, body/
  frontmatter preparation, loop, and lifecycle boundaries, or an equivalent
  counter-based seam that does not add identity fields to Darkmatter.
- Exercise the real owner functions and assert one construction, the expected
  extensions, zero fallbacks, and one populated observation at every required
  consumer.

### 3. Medium — the direct/inline/loop regression matrix does not exercise Windows

The main CLI regression binary is excluded wholesale on Windows by
`#![cfg(unix)]` (`cli/tests/ctx_launch_anchor_baseline.rs:1`). Consequently the
real CLI capture owner for direct compose, inline-compose, lifecycle, shell
preflight, and loop reuse is verified on macOS/Linux only. The portable
drive/UNC key test is useful, but it tests `RepositoryKey` inequality rather
than running the repaired CLI route with Windows path parsing
(`lib/src/invocation_context/tests.rs:863-879`).

This leaves AC13 only partially verified. The implementation is
component-aware, but a Windows-only launch/source path or provider-stub issue
can regress without failing the acceptance matrix.

Required change:

- Replace the Unix shell provider stubs with a portable fixture binary, or add
  a Windows-specific equivalent, so the direct/inline/loop matrix runs in the
  native Windows L1 leg.
- Retain the current Unix test and the drive/UNC unit cases; this is additional
  route coverage, not a request for Level 2 or Level 3 testing.

### 4. Low — an existing Level 2 test documents the retired recapture behavior

The comment above
`level2_lifecycle_initialize_proxy_target_resolves_ctx_not_in_source` says a
proxy drops the source snapshot and “re-captures per expression”
(`cli/tests/level2_lifecycle_control.rs:3988-3993`). The repaired design does the
opposite: a target starts a fresh epoch and a stabilized reread extends that
epoch from retained launch evidence. The test remains valuable, but its
explanation now contradicts D4 and the implementation.

Update the comment to describe fresh target-epoch capture and retained-evidence
extension. No behavior change is needed for this finding.

## Requirement Verification Levels

| Requirement | Strongest verification present | Assessment |
|---|---|---|
| AC1 — direct and loop root/package pair | Level 1 real CLI/provider-stub matrix | Appropriate level and green on macOS; Windows route gap is Finding 3. |
| AC2 — opposing launch/source areas across body, frontmatter, preflight, and lifecycle | Level 1 real CLI tests | Appropriate and green. |
| AC3 — external source and outside-repository inverse | Level 1 real CLI tests | Appropriate and green. |
| AC4 — real CLI capture owner | Level 1 spawned `claudine` process | Appropriate and green for the exercised platforms. |
| AC5 — exact epoch snapshot reuse | Level 1 value assertions and partial work counters | **Incomplete:** consumer-observation proof is absent; Finding 2. |
| AC6 — target identity on direct, proxy/re-entry, loop, and sequence | Level 2 tmux direct/proxy equivalence plus Level 1 owner/CLI tests | Appropriate for listed routes; the additional canonical passthrough route is functionally wrong (Finding 1). |
| AC7 — sequence launch facts and pre-selection rejection | Level 1 real CLI plus unit tests | Appropriate and green. |
| AC8 — proxy/re-entry/loop parity | Level 2 tmux direct/proxy equivalence plus Level 1 re-entry and loop tests | Appropriate. The affected behavior is semantic context resolution, not terminal encoding. |
| AC9 — system-prompt and overlay relocation | Level 1 canonical-owner tests | Launch-facing relocation is green; target identity in passthrough overlay materialization is missing and broken (Finding 1). |
| AC10 — source-relative schema/file resolution | Level 1 real CLI and owner tests with conflicting files | Appropriate and green. |
| AC11 — no extra discovery | Level 1 deterministic work counters | Appropriate and green for covered routes. |
| AC12 — capture-owner guard | Level 1 production-source inventory and self-test | Appropriate and green. |
| AC13 — cross-platform paths | Level 1 path-shape tests; direct/inline/loop CLI matrix is Unix-only | **Incomplete on Windows; Finding 3.** |
| AC14 — area validation gates | Checked-in acceptance record for build/L1/L2/lint; focused L1 and both affected lint gates rerun in this review | Full L1/L2 gates were not rerun by this review. |

No Level 3 coverage is required. This fix defines no keyboard, mouse, paste,
IME, focus, or terminal input-encoder behavior. Level 2 is also not required for
the new launch-context values themselves; they are semantic prompt/lifecycle
data rather than glyph, width, SGR, or scrolling contracts. Existing Level 2
proxy-equivalence coverage is useful additional evidence.

## Verification Performed

- `git diff --check` passed.
- The two new real-CLI test binaries passed: 8 tests, 0 skipped.
- The capture-owner inventory and its normalized-path self-test passed: 2 tests.
- Focused epoch, lifecycle, re-entry, overlay, system-prompt, sequence, and
  launch-extension tests passed: 11 tests.
- `just lint` passed in `claudine/` for all five packages, including the
  lifecycle documentation-facet guard.
- `just lint` passed in `darkmatter/` for all three packages.
- The macOS linker emitted the existing compact-unwind warning while building
  the focused CLI test binary; it did not fail the build or tests.

I did not rerun `just test-l2`; Level 2 assessments above are from source
inspection of the checked-in tmux tests and the acceptance record. I also did
not run native Windows or Linux jobs in this macOS session.
