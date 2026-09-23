---
$schema: feature-review.yaml
ready: false
agent: codex/default
created: 2026-07-20T15:53:09-07:00
spec: 2026-07-13-error-propogation/spec.md
implemented: true
description: A **feature** review of `2026-07-13-error-propogation/spec.md`
feature: 2026-07-13-error-propogation/review-6.md
previous: 2026-07-13-error-propogation/review-5.md
next: 2026-07-13-error-propogation/review-7.md
---

# Review 6: End-to-End Typed Error Propagation

## Verdict

The feature is **not ready for production**. The central typed-transport,
discovery, effective-selection, snapshot, restoration, and component-rendering
architecture remains sound, and no regression was found in those seams.
However, the live proxy routes do not supply the property path promised by the
diagnostic contract, the real-terminal suite does not exercise automatic TTY
color detection, the living error-architecture documentation has drifted from
the integrated file-resolution behavior, and the required current full-gate
evidence remains incomplete.

## Findings

### 1. High — Live proxy failures do not project the authored property path

The specification requires a semantic file-reference wrapper to carry the
authored action/property path and, when exact span data is unavailable, at
least the nearest stable property path (§D3 and §D6). The public
`FileReferenceContext::property` contract is correspondingly documented as a
dotted path such as `initialize.stack[0].proxy`
(`lib/src/composition/error/mod.rs:2140-2154`).

The production constructors do not meet that contract:

- `route_initialize` stores only `"initialize"`
  (`cli/src/commands/wrap/composition/pipeline.rs:1168-1175`).
- `run_target_initialize` also stores only `"initialize"`
  (`cli/src/commands/wrap/harness_orch/loop_control/proxy.rs:89-100`).
- `dispatch_terminal_control` stores only `"proxy"`
  (`cli/src/commands/wrap/harness_orch/loop_control/control_dispatch.rs:190-199`).

Consequently, `err.detail.property` does not identify the authored value. The
initialize value identifies an event root rather than a property, while the
terminal value omits the event/stack location entirely. The unit tests that
assert a dotted path construct `FileReferenceContext` directly with the desired
value; they do not exercise these production constructors. The Level-2 parity
test's `plain.contains("proxy")` assertion
(`cli/tests/level2_typed_error_render_capture.rs:655-660`) can pass because the
word also appears in the fixture, hint, and surrounding prose, so it does not
verify the structured field.

**Required change:** retain origin metadata when a lifecycle action becomes
`StackControl::Proxy`, and populate the exact indexed path when available (or a
clearly wildcarded stable path such as `initialize.stack[*].proxy` when it is
not). Add a live-route Level-1 assertion that evaluates or serializes
`err.detail.property` for initialize and terminal/recovery proxy failures. Keep
the Level-2 assertion focused on the rendered property label, but make it
specific enough that unrelated uses of the word `proxy` cannot satisfy it.

### 2. High — Automatic TTY color detection has no Level-2 verification

The specification preserves TTY detection, `NO_COLOR`, `FORCE_COLOR`, OSC 8,
and ANSI stripping as distinct rendering contracts, and its Level-2 strategy
requires the real CLI variants where those contracts differ. The Level-2 suite
correctly covers forced color and `NO_COLOR`, but every colored case uses
`TTY_COLOR`, which always injects `FORCE_COLOR=1`
(`cli/tests/level2_typed_error_render_capture.rs:338-396`). That proves the
forced-color path in a real tmux pane; it does not prove that an unforced
`claudine` process recognizes its real stderr TTY and enables the styled block
and OSC 8 link automatically.

The strongest relevant evidence for automatic detection is therefore below the
required Level 2. This is a test-rigor gap, not a claim that current rendering
is visibly broken.

**Required change:** add a Level-2 real-pane case that removes both
`NO_COLOR` and `FORCE_COLOR`, runs the motivating typed failure, and asserts the
expected SGR and OSC 8 output. Retain the existing forced-color, `NO_COLOR`, and
piped-stderr cases because they verify different branches.

### 3. Medium — The living error-architecture docs describe obsolete null fields

The integrated resolver now populates `failure`, and, after a probe,
`repository_root` and ordered `candidates` from typed `biscuit-file` data
(`lib/src/harness/error.rs:337-377`). The authoritative living topic still says
all three are reserved `null` values and that the file-resolution feature
*will* populate them (`docs/topics/error-architecture.md:241-271`). The
`diagnostics` module documentation similarly says the remaining catalog fields
are reserved as null for future file-resolution work
(`lib/src/diagnostics/mod.rs:34-39`).

This is behavior/documentation drift in the public `err.detail.*` contract. It
can cause lifecycle authors to avoid fields that are now available or to write
incorrect fallback logic.

**Required change:** update both documents to describe the current two paths
accurately: shared `biscuit-file`/harness resolution populates typed failure and
probe data when known, while legacy lower-layer diagnostics that genuinely lack
that information retain present-null fields. Preserve the explanation that
values must never be back-derived from display text.

### 4. High — Acceptance Criterion 9 still lacks complete current gate evidence

Review 5's remaining blocker was the absence of complete current runs for
`just test`, `just test-l2`, and `just lint`. No later implementation commit or
gate record exists after that review in this worktree.

This review attempted `just lint-transport`. It spent the non-interactive
60-second command budget compiling the cold dependency graph and was terminated
with exit 130 before any tests ran. No compiler, assertion, or guard failure was
observed, but the attempt is not a pass. The broader canonical recipes were not
started after that ceiling was reached.

**Required change:** after the findings above are fixed, run `just test`,
`just test-l2`, and `just lint` to completion in the Claudine package area and
record their exact summaries. A partial build or focused suite cannot satisfy
the explicit acceptance criterion.

## Requirement Verification Levels

| Requirement | Strongest verification observed | Assessment |
|---|---|---|
| Preserve typed causes or a versioned snapshot across in-process and erased boundaries | Level 1 Rust-aware transport guards, typed-source tests, snapshot round trips, and restoration tests | Appropriate in design. The current guard run did not complete, but source inspection found no new competing transport path. |
| Discover every Claudine diagnostic through one registry and select one effective diagnostic | Level 1 registry parity, runtime downcast, semantic/transparent selection, repeated-chain, and depth-guard tests | Appropriate. The CLI walker and lifecycle snapshot both use the shared selector. |
| Keep rendering, lifecycle `err.*`, and machine projection on the same selected diagnostic | Level 1 error-walker, `LifecycleErrorInfo`, `DiagnosticSnapshot`, and restored-diagnostic tests | Appropriate for selection and data transport. |
| Carry source document, event, and authored property context through proxy wrapping | Hand-constructed Level 1 unit tests plus live-route source inspection | **Gap.** The production routes populate `initialize` or `proxy`, not the documented dotted property path, and no live-route test asserts `err.detail.property`. |
| Render initialize and terminal/recovery proxy failures as the same typed block | Level 2 tmux route capture, with Level 1 process characterization | Appropriate for block rendering and route behavior. The structured property subrequirement remains open. |
| Cover composition lookup, schema/file-reference, transclusion, harness pre-flight, and unstructured fallback rendering | Level 2 tmux captures plus Level 1 component/process tests | Appropriate. Each typed route distinguishes `StatusBlock` output from the generic fallback. |
| Preserve forced color, `NO_COLOR`, plain/piped output, and OSC 8 behavior | Level 2 tmux for `FORCE_COLOR` and `NO_COLOR`; Level 1 spawned-process coverage for piped stderr | Appropriate for those branches. |
| Preserve automatic real-TTY detection | Forced-color Level 2 only | **Gap.** `FORCE_COLOR=1` bypasses the automatic detection branch the requirement names. |
| Preserve exit codes, lifecycle ordering, retry/control decisions, and exactly-once emission | Level 1 route characterization plus representative Level 2 captures | Appropriate for the covered routes. |
| Keep diagnostic detail catalog-shaped, structured, present-null when unknown, and one-cause deep | Level 1 catalog/corpus guards and snapshot tests | Appropriate structurally; live proxy `property` content is incorrect, and the living documentation is stale. |
| Pass all required Claudine gates | Historical focused/full evidence plus an interrupted current `lint-transport` build | **Gap.** Complete current `just test`, `just test-l2`, and `just lint` results are absent. |

Level 3 is not applicable. This feature does not specify OS keyboard/mouse
delivery, terminal input encoding, paste, IME, or hotkey behavior. Its terminal
requirements concern rendered output, for which Level 2 is the correct tier.

## Verification Performed

- Read the specification, decisions, prior review, living error-architecture
  documentation, central discovery/selection implementation, snapshot and
  restoration boundaries, lifecycle projection, top-level error walker, live
  proxy constructors, route characterization, and typed-error Level-2 suite.
- Used GitNexus execution-flow search and symbol context to enumerate all
  production `FileReferenceContext` constructors. The two initialize paths and
  terminal-control path are the complete live constructor set outside tests.
- Attempted `just lint-transport`; it was terminated at the non-interactive
  command ceiling during dependency compilation, before tests ran (exit 130).
- Preserved the caller's unrelated existing changes to `CLAUDE.md` and
  `prompts/_implement/implement-suggestions.md`.

## Production Readiness Closure

Fix the live property-path projection and add a production-route Level-1
regression, add an unforced real-TTY Level-2 capture, align the living
documentation with the integrated resolver, and then complete all three
canonical Claudine gates. Until those conditions are met, this feature is not
production ready.
