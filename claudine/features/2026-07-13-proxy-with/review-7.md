---
$schema: feature-review.yaml
ready: false
agent: codex/default
created: 2026-07-18T00:39:57-07:00
spec: 2026-07-13-proxy-with/spec.md
implemented: true
description: A **feature** review of `2026-07-13-proxy-with/spec.md`
feature: 2026-07-13-proxy-with/review-7.md
previous: 2026-07-13-proxy-with/review-6.md
---

# Review 7: Proxy With

## Verdict

The feature is **not ready for production**. Review 6's dry-run, AC9/AC10
composition-command matrix, and route-equivalent diagnostic work materially
improved the implementation. Dry run now exits before lifecycle construction,
the surfaced command coordinator has Level 2 coverage for the target launch
bundle, and typed diagnostics are compared across all three routes in tmux.

Three release blockers remain. The diagnostic-parity fix validates a proxied
target's schema before its required `initialize` stage. The complete resume
compatibility contract remains end-to-end reachable only for `model`. The
direct-provider-wrapper fallback still borrows profile/binary, argv, and MCP
state instead of canonically preparing the active target. The latter two gaps
are explicitly recorded as partial in the acceptance map; partial acceptance
criteria cannot support a production-ready verdict.

## Findings

### 1. High: proxy-target schema pre-validation now runs before target `initialize`

R4 requires a fresh target to run its narrow initialize safety gate and then
its own `initialize` before schema validation and full pre-flight
(`spec.md:430-462`). AC11 and AC12 repeat that ordering and require the
stabilized reread to observe initialize-time mutations (`spec.md:901-906`). The
review-6 diagnostic fix does the reverse. For every non-first active document,
`prepare_and_run_active_document` calls `pre_validate_schema` before it builds
the preparation context or enters the staged target bootstrap
(`cli/src/commands/compose/prep.rs:385-418`).

This makes direct and proxied schema diagnostics agree by moving the proxied
route to the caller's already-early validation seam, not by preserving the
specified lifecycle order. A target whose `initialize` would add or repair a
schema property now fails before that action can run. A still-invalid target
also fails before its lifecycle configuration exists, so its target-owned
blocked/finalize routing cannot follow R4's post-lifecycle failure contract.

The new Level 2 diagnostic fixtures cannot detect this. Their schema-invalid
targets contain no `initialize` action
(`level2_lifecycle_control.rs:5013-5035`), and the shared assertion checks only
the diagnostic, provider-launch count, and source-router markers
(`level2_lifecycle_control.rs:5181-5278`). The existing narrow-gate test uses a
schema-valid target, while the initialize-mutation reread test does not combine
the mutation with schema validation.

Move typed schema error conversion into the shared post-initialize canonical
preparation stage. Add Level 2 direct/initialize-proxy/recovery-proxy rows where
target `initialize` runs exactly once and satisfies a required property before
validation, plus a still-invalid row proving target initialize and the owed
catch/finalize ordering occur before the route-equivalent diagnostic is
rendered.

**Verification level:** the user-observable ordering has partial Level 2
coverage, but no Level 2 test combines target initialize with schema validation.
The current implementation contradicts the required order. Level 3 is not
applicable.

### 2. High: AC15 still has no complete, reachable resume-compatibility check

AC15 requires canonical resume refresh to compare the complete session key,
refuse every incompatible live session, name changed facets, and recommend
retry (`spec.md:914-917`). Review 6 made `model` genuinely movable by rebuilding
`AGENT`/`MODEL`/`YOLO` at each fresh-read boundary. It did not rebuild the rest
of the launch bundle. `session_key.rs` deliberately reads system-prompt and MCP
signals from invocation-fixed canonical argv and leaves provider-specific
`extra` identity empty (`session_key.rs:24-43`). `target_launch.rs` likewise
states that the other compatibility facets cannot move across a same-document
refresh (`target_launch.rs:51-55`).

The acceptance map therefore remains candidly partial: only the model refusal
and compatible converse run at Level 2; provider, binary/resume protocol,
workspace CWD, permission and interactivity modes, structured output, system
prompt, and MCP identity are projection-only Level 1 tests
(`notes/acceptance-map.md:127`). A key that unit tests can distinguish but the
canonical refresh can never change does not implement the user-facing refusal
contract.

Rebuild the final typed launch bundle at every retry/resume fresh-read boundary,
derive both sides of the compatibility comparison from those bundles, and add
Level 2 refusal rows for every compatibility facet. Each row must prove no
resume launch occurred, the changed facet was named, and retry guidance was
rendered.

**Verification level:** Level 2 is appropriate and present only for `model`;
the remaining observable refusal behavior is Level 1 projection-only. This is
a wrong-level and implementation gap. Level 3 is not applicable.

### 3. High: direct provider-wrapper proxy handoffs still use a reduced launch path

R3 prohibits a rich direct path alongside a reduced handoff composer, and R6 /
AC10 require every document-dependent launch decision to be rebuilt for the
active target (`spec.md:384-428,492-505,897-900`). Composition commands now meet
that contract through the surfaced coordinator. Direct provider wrappers do
not: their no-ledger fallback adopts in the harness and rebuilds only the
`AGENT`/`MODEL`/`YOLO` environment and early-binding context. The module itself
states that profile/binary selection, the argv entrypoint, and MCP runtime
injection remain borrowed from the invocation
(`target_launch.rs:31-49`).

The acceptance map marks AC10 partial for precisely this reason
(`notes/acceptance-map.md:122`). This is user-observable whenever a wrapper
handoff's target-owned launch configuration differs from the source, and there
is no Level 2 direct-wrapper equivalence row proving the complete bundle.

Surface wrapper handoffs to an owning coordinator that can re-enter the full
selection/MCP/argv pipeline, or reject unsupported handoffs explicitly until
that coordinator exists. Under the current specification, silently borrowing
the source launch bundle is not an acceptable fallback. Add Level 2 wrapper
rows for every previously borrowed facet.

**Verification level:** the composition-command path has the required Level 2
matrix; the provider-wrapper fallback has only reduced-path unit coverage and
an admitted implementation limitation. Level 3 is not applicable.

### 4. Medium: the acceptance map overstates AC11 and overall completion

The acceptance map reports 28 of 30 criteria complete and marks AC11 complete
using a pure narrow-gate test, a preparation-policy unit test, and a schema-valid
Level 2 shell-denial row (`notes/acceptance-map.md:113-127`). None verifies the
required initialize-before-schema order, and the production code now violates
it. Reclassify AC11/AC12 until Finding 1 is fixed, and keep the headline count
derived from the actual per-criterion statuses. The two already-admitted
partials, AC10 and AC15, are release criteria rather than scoped sign-off
exceptions.

## Verification-level audit

| User-observable requirement | Strongest present | Assessment |
|---|---:|---|
| `proxy.with` parsing, recursive typed values, precedence, null/shallow semantics | Level 1 | Appropriate for pure data semantics |
| Handoff consumption, target lifecycle, loop ownership, inline/sequence closure | Level 2 | Appropriate |
| Dry-run has no lifecycle side effects or proxy traversal | Level 2 | Appropriate; review-6 behavior now matches the spec |
| Full AC9 context equivalence across body/frontmatter/lifecycle | Level 2 | Appropriate |
| Full AC10 launch equivalence on composition commands | Level 2 | Appropriate for that path |
| Full AC10 launch equivalence on direct provider wrappers | Reduced-path Level 1 only | **Mismatch / incomplete** |
| Target initialize before schema validation and stabilized reread | Partial Level 2 | **Mismatch / implementation regression** |
| Resume incompatibility for `model` | Level 2 | Appropriate |
| Resume incompatibility for every other session facet | Level 1 projection only | **Mismatch / incomplete** |
| Route-equivalent typed diagnostics and pane rendering | Level 2 | Appropriate, but the fix introduced the stage-order regression |
| Overlay-value non-disclosure in terminal status | Level 2 pane capture | Appropriate |
| OS keyboard/input encoder behavior | None | Not applicable; this feature has no Level 3 input requirement |

## Validation performed

- Reviewed the specification, review 6, the current acceptance map, the
  review-6 implementation commits, and the production/test call sites for dry
  run, schema preparation, launch rebuilding, and session compatibility.
- `just test-cli compose_dry_run` — stopped with exit 130 at the
  non-interactive command guard while compiling dependencies; no test executed,
  so this review claims no fresh local pass from that command.
- The repository's acceptance map records prior full-area green gates at this
  HEAD (`just test`, `just test-l2`, and `just lint`). Those results show the
  existing suite is green; they do not close the missing/wrong-level assertions
  above.

