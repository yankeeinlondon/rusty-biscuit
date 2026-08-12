---
$schema: feature-review.yaml
ready: true
agent: codex/default
created: 2026-08-12T23:42:45+01:00
spec: 2026-08-12-ctx-launch-anchor/spec.md
implemented: false
description: A **fix** review of `2026-08-12-ctx-launch-anchor/spec.md`
fix: 2026-08-12-ctx-launch-anchor/review-2.md
previous: 2026-08-12-ctx-launch-anchor/review-1.md
---

# Review 2: Ctx Launch Anchor

## Verdict

The fix is **ready for production**. All four findings from Review 1 are
resolved, the launch-context contract is implemented across the specified
canonical routes, and no new findings were identified. The full Claudine Level
1, Level 2, and lint gates pass on macOS.

The direct-wrapper passthrough route now applies the finalized `AGENT` and
`MODEL` layer before composing provider memory. The document-epoch owner records
populated consumption at preflight, body/frontmatter, loop, and lifecycle
boundaries while the construction and extension counters continue to prove that
retained launch evidence is reused. The direct/inline/loop CLI regression binary
is no longer Unix-only and supplies native Windows provider fixtures.

## Findings

No findings.

## Previous Review Closure

| Review 1 finding | Resolution |
|---|---|
| High — passthrough omitted resolved target identity | Closed. `detect_wrapper_harness` projects finalized `AGENT`/`MODEL` entries from the child environment plan into passthrough materialization. Conflicting-ambient Level 1 coverage verifies `ctx.agent`, `ctx.model`, `env.AGENT`, `env.MODEL`, retained lifecycle context, and propagated overrides. |
| Medium — AC5 lacked consumer observations | Closed. Typed Claudine-only observations cover preflight, body/frontmatter, loop, and lifecycle seams. The deterministic Level 1 proof asserts one construction, retained-evidence extension, zero ambient fallbacks, and one populated observation per required consumer. The capture-owner inventory prevents an unaccounted direct Darkmatter capture from replacing the invocation seam. |
| Medium — direct/inline/loop matrix excluded Windows | Closed. The integration binary now compiles on all platforms and uses native `.cmd` launchers with a non-interactive PowerShell provider fixture on Windows. The native Windows L1 CI leg is the execution proof; this macOS review could not execute that leg. |
| Low — Level 2 comment described retired recapture behavior | Closed. The comment now describes fresh proxy-target epoch capture and retained-evidence extension. |

## Requirement Verification Levels

| Requirement | Strongest verification present | Assessment |
|---|---|---|
| AC1 — direct and loop root/package pair | Level 1 spawned CLI/provider-fixture matrix | Appropriate. Direct and loop copies report the launch area through lifecycle interpolation and `when:` evaluation. |
| AC2 — opposing launch/source areas | Level 1 spawned CLI tests | Appropriate. Body, effective frontmatter, shell-expanded bytes, and lifecycle use the launch area. |
| AC3 — external source and outside-repository inverse | Level 1 spawned CLI tests | Appropriate. Moving the source across repository boundaries neither replaces nor invents launch facts. |
| AC4 — real CLI capture owner | Level 1 spawned `claudine` process | Appropriate. The regression matrix crosses the production CLI seam. |
| AC5 — exact epoch snapshot reuse | Level 1 owner tests with construction, extension, fallback, and consumer counters | Appropriate. One invocation-owned construction feeds every observed consumer; the capture-owner guard excludes unapproved alternate constructors. |
| AC6 — resolved target identity | Level 2 tmux direct/proxy equivalence plus Level 1 owner, re-entry, sequence, loop, and passthrough tests | Appropriate. The repaired passthrough test includes conflicting ambient values and all four identity roots. |
| AC7 — sequence parity and graph rejection | Level 1 real CLI and unit tests | Appropriate. Launch facts propagate across graph/JIT/task routes, and pre-selection target roots produce typed rejection. |
| AC8 — proxy/re-entry/loop parity | Level 2 tmux proxy-route test plus Level 1 re-entry and loop tests | Appropriate. This is semantic context resolution; no input-encoder behavior is involved. |
| AC9 — system-prompt and overlay parity | Level 1 canonical-owner tests | Appropriate. Relocation remains launch-anchored, and passthrough overlay identity is now target-adjusted. |
| AC10 — source-relative file resolution | Level 1 real CLI and owner tests with conflicting files | Appropriate. Schema, eager file, and transclusion behavior remains source-relative/repository-first as specified. |
| AC11 — no additional discovery | Level 1 deterministic work counters | Appropriate. Construction and extension reuse retained invocation evidence. |
| AC12 — capture-owner guard | Level 1 production-source inventory plus guard self-test | Appropriate. New direct prepared-context captures fail the inventory. |
| AC13 — cross-platform paths | Level 1 portable path tests and platform-native CLI fixtures | Appropriate. macOS tests pass; the Windows-specific route is enabled for the native Windows CI leg rather than being filtered out. |
| AC14 — validation gates | Package-area Level 1, Level 2, and lint gates | Green in this review. |

No Level 3 coverage is required. The fix does not define keyboard, mouse,
paste, IME, focus, or terminal input-encoder behavior. Level 2 is not required
for the launch-context values themselves because they are semantic prompt and
lifecycle data, but the existing real-terminal proxy-equivalence coverage also
passes.

## Verification Performed

- `just test` from `claudine/` passed all five packages. The launch-anchor CLI
  matrix passed 5/5 cases, and the capture-owner, passthrough identity, and
  document-epoch accounting tests passed.
- `just test-l2` from `claudine/` passed 230 `claudine-cli` tests and 3
  `claudine-gen` tests.
- `just lint` from `claudine/` passed all five packages, the 18 error-guard
  tests, and the lifecycle documentation-facet guard.
- `git diff --check` passed after the review frontmatter updates.

The macOS linker emitted its existing compact-unwind size warning during test
builds; it did not affect compilation or execution. Native Windows and Linux
jobs were not executed from this macOS host.
