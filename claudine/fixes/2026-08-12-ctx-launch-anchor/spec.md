---
status: draft
created: 2026-08-12
reviewed: true
reviewed_by: codex/default
reviewed_on: 2026-08-12
area: claudine
packages:
    - claudine
---

# `ctx.*` answers "where does the prompt live?" instead of "where did the caller launch from?"

## Summary

Claudine's prepared `ctx.*` catalog describes the immutable **launch
context**: the directory in which the caller invoked Claudine and the
repository/package-area facts projected from that directory. It must not
describe the prompt document's storage location or the mutable process CWD.

The CLI violates that contract today. Canonical composition paths obtain
repository evidence from a prompt-specific `SourceContext` and pass the
prompt's parent directory to `ComposeContext::capture_with_evidence`. A shared
prompt therefore changes `ctx.area`, `ctx.repo_root`, `ctx.current_packages`,
and other repository-backed `ctx.*` values merely by moving between prompt
directories, even when the caller and invocation are unchanged.

The immediate failure appears in direct compose, but the capture pattern is
not confined to the two sites that first exposed it. Sequence preflight and
task preparation, harness re-materialization, overlays, and system-prompt
composition also contain source-anchored captures. Fixing only the observed
sites would preserve route-dependent answers and violate direct/proxy/sequence
equivalence.

This fix introduces one invocation-owned launch-context capture seam, uses it
throughout canonical CLI paths, and reuses one coherent early-binding snapshot
through every phase of a document preparation epoch. Prompt-specific
`SourceContext` remains authoritative for document/file resolution and other
explicitly source-relative behavior.

> **Reader's note:** The initial repair was a two-site replacement of
> `source_context.base_dir()` with `launch_cwd`. Review expanded it into a
> shared launch-capture seam because changing only the directory while retaining
> prompt-derived evidence creates an internally inconsistent context, and
> changing only direct compose leaves sequence and re-materialization paths on
> the wrong contract.

## Established contracts

This specification repairs accidental drift; it does not introduce a new
meaning for `ctx.*`.

- The 2026-06-27 lifecycle path-resolution fix established
  `ctx_base_dir` as the caller's launch area and kept the prompt parent as the
  separate document base.
- Proxy requirement R5 requires body interpolation, effective frontmatter,
  lifecycle DM2 lookup, schema/file evaluation, and shell preflight to consume
  the same stored early-binding `ComposeContext` and environment override
  layer. Proxy acceptance criterion 9 requires direct and proxied execution to
  agree on `ctx.area`, `ctx.agent`, and `ctx.model`.
- The propagated-context design distinguishes the immutable launch snapshot
  from a per-document `SourceContext`. Source context carries the document's
  authoring base and repository evidence needed for source-relative work; it
  does not redefine the caller's launch location.
- `current.ctx.*` is intentionally live event-time state and is outside this
  repair. This specification concerns prepared plain `ctx.*` only.

## Observed behavior (verified 2026-08-12, binary built the same day)

A minimal prompt whose lifecycle reports `ctx.area` was run twice from the
same shell directory (`darkmatter/`), changing only where the prompt file was
stored:

| Prompt file location | Launched from | `ctx.area` reports |
|---|---|---|
| `darkmatter/zz-probe.md` | `darkmatter/` | `darkmatter` |
| `<repo root>/zz-probe.md` | `darkmatter/` | empty |

The same launch directory produces different answers, so the value tracks the
prompt. Both template interpolation (`{{ ctx.area }}` in `warn:`) and `when:`
guard evaluation see the same wrong value because both read the prepared
snapshot.

The real-world symptom was a repository-root shared prompt whose warning tried
to distinguish a caller at the repository root from one inside a package area.
Its `ctx.area` was permanently empty because the prompt lived at the root. The
warning has since been made location-neutral, but shared prompts still cannot
ask where their caller launched from.

This bug is independent of file-reference semantics. Eager `file(...)` schema
parameters resolve through the request-scoped file resolver and currently use
the launch inputs correctly. The prepared runtime context is the component
anchored incorrectly.

## Root cause

`InvocationContext::derive_source` correctly sets `SourceContext::base_dir` to
the prompt file's parent. That value belongs to source-relative resolution. The
bug is that callers also use the source context to build prepared `ctx.*`:

1. Direct compose shell preflight in
   `claudine/cli/src/commands/compose/prep.rs` calls
   `runtime_evidence(&source_context, ...)` and captures against
   `source_context.base_dir()`.
2. Direct/loop preparation in the same module repeats that construction for
   `PrepareOptions.prepared_context`.
3. Sequence graph preflight, per-document shell approval, JIT template
   preparation, and task execution repeat the source-anchored construction.
4. Canonical overlay, harness re-materialization, passthrough, and
   system-prompt paths contain the same pattern.

The comments at the original direct-compose sites claim launch-area anchoring.
The earlier contracts and tests establish that intent, so this is one of the
exceptional cases where the code—not the comments—drifted.

There is a second contract violation at the direct-compose seam: preflight and
main preparation capture two separate contexts. They currently tend to agree
because invocation evidence is cached, but they are not the exact stored
snapshot R5 requires and can differ for time-sensitive groups. “Constructed
the same way” is weaker than “the same snapshot.”

The library compatibility fallback in `derive_compose_context` already prefers
the explicit launch directory when a caller supplies one. Canonical CLI paths
normally supply `prepared_context`, so that correct fallback is bypassed on the
paths users exercise.

## Design decisions

### D1 — Prepared `ctx.*` is launch-anchored

For every canonical CLI invocation, plain `ctx.*` is projected from:

- `InvocationContext::launch_cwd()` as the capture anchor;
- the invocation's launch-repository observation and launch package topology;
- the invocation's captured environment and host evidence; and
- the resolved target's explicit environment override layer for agent/model
  identity.

Moving a prompt, proxy target, task, group, overlay, or system-prompt file must
not change launch-facing `ctx.*` values. A source in another repository does
not replace the launch repository for plain `ctx.*`.

`ctx.agent` and `ctx.model` remain target-dependent: apply the resolved
target's `env_overrides` after the launch capture, using the existing
precedence. This preserves the June “Bug C” behavior.

### D2 — Evidence and anchor are one operation

Add an invocation-owned API that returns a `ComposeContext` captured from
launch evidence for a supplied `ContextRequirements`. The exact name is an
implementation choice; its semantic shape is:

```rust
fn capture_launch_context(
    &self,
    requirements: &ContextRequirements,
) -> ComposeContext;
```

The method must use the retained launch repository/topology entry and the
launch CWD together. Callers must not be able to pair a launch anchor with
prompt-derived evidence accidentally.

Refactor `runtime_evidence` internally if necessary so launch and source
projections share caching and work accounting without fabricating a prompt
path or performing another host scan. Do not construct a fake `SourceContext`
for the launch directory.

Alternatives considered:

- **Replace only the anchor at each call site.** Small diff, but it combines
  launch coordinates with source-derived Git/package evidence and therefore
  can produce a mixed snapshot. Rejected.
- **Derive a synthetic source under `launch_cwd`.** Reuses the current API, but
  invents document identity and couples runtime facts to file-resolution
  semantics. Rejected.
- **Provide one invocation-owned launch capture seam.** Keeps anchor/evidence
  pairing enforceable, centralizes work accounting, and makes route audits
  mechanical. Chosen because it encodes the contract in the API rather than in
  comments at every caller.

### D3 — Source context remains source-relative

Do not alter `SourceContext::base_dir`, its repository identity, or its
`FileResolutionContext`. They continue to drive:

- document-authored file references and transclusion;
- `$schema` and source-relative schema discovery;
- request-scoped file resolution and provenance; and
- source-specific selection/workspace behavior where an existing contract
  explicitly calls for it.

The launch directory retained as `file_ref_fallback_dir` remains diagnostic
metadata under the current repository-first file-resolution design; this fix
must not revive the superseded launch-directory fallback as a file candidate.
Eager `file(...)` parameters and document-authored references keep their
existing resolution order.

### D4 — One snapshot per document preparation epoch

A **document preparation epoch** begins when direct, proxy-target, retry, or
resume entry reads and prepares an active document. It derives one coherent
early-binding context after provider/model resolution and reuses that exact
snapshot for:

- narrow and full shell preflight;
- body and non-lifecycle frontmatter composition;
- schema/file evaluation that consumes prepared context;
- loop conditions that use plain `ctx.*`; and
- every lifecycle event for that prepared document.

Do not capture a second “equivalent” context for preflight or lifecycle.
Proxying to another document, retrying, or resuming starts the canonical fresh
preparation required by the lifecycle contracts and may construct a new
snapshot, but immutable launch-facing values remain projections of the same
invocation evidence. Loop iterations that reuse one prepared document also
reuse its stored snapshot. `current.ctx.*` remains separately captured and
live.

Static sequence graph preflight occurs before per-task target selection, so it
uses one launch-evidence base context for launch-facing expressions. Each task's
prepared epoch then clones or projects that launch context and applies its own
resolved target environment overrides. Any command audited in both phases must
resolve to identical bytes or retain the existing late-binding rejection.

### D5 — Canonical-path audit and guard

Audit every production `ComposeContext::capture*` call reachable from compose,
inline-compose, sequence, proxy/retry/resume, harness re-materialization,
overlay preparation, and system-prompt preparation.

For a canonical path that has an `InvocationContext`:

- prepared plain `ctx.*` must use the launch-capture API;
- document/file resolution must still use the active `SourceContext`; and
- compatibility capture is not an acceptable fallback.

Add a corpus or inventory guard that fails when canonical Claudine code adds a
new direct prepared-context capture outside the approved owner. Library-only
compatibility APIs and explicitly live `current.ctx.*` capture sites may remain
allowlisted with a reason.

## Scope

- `claudine/lib/src/invocation_context.rs`: expose the paired launch-evidence
  context capture and retain demand-driven caching/work counters.
- Direct compose/inline-compose preparation: remove the two independent
  source-anchored captures and pass the epoch snapshot through preflight,
  canonical preparation, loops, and lifecycle.
- Sequence: correct root graph preflight, referenced-document shell
  resolution, approval composition, JIT template preflight, and task runtime
  preparation. A task/group/prompt stored in another repository must still
  report the caller's launch area through plain `ctx.*`.
- Proxy, retry/resume, harness re-materialization, passthrough, and overlay
  paths: reuse the active prepared document's snapshot or create it through the
  canonical epoch owner; do not recapture from the active source.
- System-prompt composition: prepared plain `ctx.*` uses launch evidence while
  file and schema references retain the system-prompt source context.
- Tests and a capture-owner drift guard covering the route inventory above.
- Update drifted comments and relevant module/docs descriptions wherever they
  currently say source-backed `ctx.*` is intentional. Preserve comments that
  already state the launch contract once code again matches them.

## Acceptance criteria

- **AC1 — discriminating direct/loop pair.** From one package area, run two
  equivalent prompts: one stored at the repository root and one stored inside
  that package area. A lifecycle `warn:` interpolation and a `when:` branch
  both report the launch package area on plain compose and loop routes.
- **AC2 — opposing areas.** A prompt stored inside package area X and launched
  from package area Y reports Y, not X, across body, effective frontmatter,
  preflight-expanded bytes, and lifecycle.
- **AC3 — external-source matrix.** A prompt stored in another repository but
  launched from a package area reports the launch repository/area. Conversely,
  a prompt stored inside a repository but launched from outside every
  repository reports no package area and no launch repository. Prompt location
  does not fill missing launch facts.
- **AC4 — CLI-seam coverage.** Regressions exercise the real CLI capture owner;
  they do not pass a hand-built executor snapshot and thereby skip the faulty
  seam.
- **AC5 — exact snapshot reuse.** A deterministic test proves preflight, body,
  effective frontmatter, and lifecycle receive the same epoch snapshot. Prefer
  a capture identity/counter assertion so two separately constructed but
  usually equal values cannot pass.
- **AC6 — target identity.** `ctx.agent`, `ctx.model`, `env.AGENT`, and
  `env.MODEL` reflect the resolved target's environment overrides on direct,
  proxy, retry/resume, loop, and sequence-task paths.
- **AC7 — sequence parity.** Root graph preflight, nested prompt documents,
  task/group expressions, template preflight, and task execution all use launch
  facts. A sequence file and task prompt stored in different repositories do
  not substitute either source repository for the launch repository.
- **AC8 — proxy/re-entry parity.** Direct and proxied execution of the same
  target agree on launch-facing `ctx.*`; retry and resume fresh reads do not
  drift to the prompt directory; loop iterations retain the epoch snapshot.
- **AC9 — system-prompt/overlay parity.** Moving a system-prompt or overlay
  source without changing the invocation does not change its launch-facing
  `ctx.*` expansion.
- **AC10 — file-resolution non-regression.** Eager `file(...)` schema
  parameters and existing repository-first/source-relative document references
  retain their current behavior. `$schema` remains document-relative. Include
  conflict fixtures where launch and source directories contain different files.
- **AC11 — no extra discovery.** The fix performs no additional ambient CWD,
  HOME, Git, or topology scan. Invocation work counters show retained launch
  evidence is reused.
- **AC12 — capture-owner guard.** The production inventory test rejects a new
  direct prepared-context capture outside the invocation owner, with explicit
  allowlist entries only for compatibility paths and live `current.ctx.*`.
- **AC13 — cross-platform paths.** L1 fixtures use `Path`/`PathBuf`, work with
  Windows drive/prefix behavior and macOS symlinked temporary directories, and
  introduce no separator, case-sensitivity, or ambient-CWD assumptions.
- **AC14 — validation.** `just test`, `just test-l2`, and `just lint` pass from
  the `claudine/` package area. L2 terminal/browser fixtures must not take
  focus.

## Non-goals

- Redesigning source-relative file resolution or restoring launch-directory
  file fallback as a resolution candidate.
- Exposing a new variable for the prompt file's repository or package area. If
  prompt authors need that information, define an explicit source-facing
  namespace in a separate feature rather than overloading `ctx.*` again.
- Changing live `current.ctx.*` semantics.
- Reworking provider selection, workspace selection, or child CWD except where
  needed to keep the existing resolved identity in the shared context.
- Changing the already location-neutral warning that exposed this bug.

## Open questions

No open design questions remain. Outside a repository, launch-facing repository
and package-area values are absent even when the prompt itself lives in a
repository; AC3 locks that decision. The prompt's own location remains
available to source/file-resolution infrastructure but is not projected into
plain `ctx.*`.
