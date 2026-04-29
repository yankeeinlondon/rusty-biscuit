# Pre-Flight Inconsistencies

Audit date: 2026-04-11

Sources reviewed:
- `claudine/docs/topics/pre-flight-checks.md` (documentation)
- `claudine/docs/topics/composition.md` (documentation)
- `claudine/lib/src/composition/preflight.rs` (implementation)
- `claudine/lib/src/harness/audit.rs` (implementation)
- `claudine/cli/src/commands/compose.rs` (CLI: compose, inline-compose)
- `claudine/cli/src/commands/wrap/mod.rs` (CLI: wrapper)
- `claudine/cli/src/commands/wrap/composition.rs` (CLI: composition executor)
- `claudine/cli/src/commands/wrap/sequence.rs` (CLI: sequence)
- `darkmatter/lib/src/markdown/compose/shell_expansion/discovery.rs` (implementation)
- `darkmatter/lib/src/markdown/compose/frontmatter_shell_expansion.rs` (implementation)

## Summary

The implementation is largely consistent with the documentation, but there are
several noteworthy inconsistencies ranging from documentation gaps to a split
pre-flight path that leaves a coverage hole.

---

## Inconsistency 1: Wrapper path runs a separate shell audit that duplicates (and can conflict with) pre-flight

**Severity: Medium**

The documentation (`pre-flight-checks.md` line 17) states:

> The pre-flight runs as part of every wrapper command — `claudine compose`,
> `claudine inline-compose`, `claudine claude`, `claudine codex`, and all
> other provider wrappers.

In practice, the **wrapper path** (`claudine claude`, `claudine codex`, etc.)
does **two** separate approval flows:

1. `resolve_shell_approvals(None, None, Some(&plan), ...)` — harness-only,
   at `wrap/mod.rs:1563`
2. `collect_auditable_commands(&plan, source_text)` followed by
   `audit_shell_commands(...)` — at `wrap/mod.rs:2770-2778`

Flow (1) is the pre-flight from `composition/preflight.rs`. Flow (2) is the
older audit path from `harness/audit.rs`. They use **different approval
mechanisms**: flow (1) passes through the `ShellApprovalOptions.approval_handler`
callback, while flow (2) uses `validate_and_approve_command_parts` which checks
the same whitelist/blacklist but through a different code path and produces
different error types (`HarnessError` vs `CompositionError`).

The composition path (`claudine compose`, `claudine inline-compose`) correctly
uses only flow (1) via `resolve_shell_approvals` (see `compose.rs:239` and
`compose.rs:345`).

**Impact:** A command approved via the pre-flight handler in flow (1) could
theoretically be denied by the audit in flow (2) if the two code paths
diverge in their normalization or policy resolution. In practice the
underlying whitelist/blacklist files are the same, but the approval _cache_
used by flow (1) is not shared with flow (2).

**Recommendation:** The wrapper path should consolidate to a single
pre-flight pass via `resolve_shell_approvals`, the same as the composition
path. The separate `collect_auditable_commands` + `audit_shell_commands`
audit in `wrap/mod.rs` is redundant for composition modes and should only
apply in `Passthrough` mode where `source_text` is provided.

---

## Inconsistency 2: Harness commands are not pre-flighted in compose/inline-compose

**Severity: Medium**

In `compose.rs`, both `run_compose_inner` (line 239) and
`run_inline_compose_inner` (line 345) call:

```rust
composition::resolve_shell_approvals(
    Some(&source.markdown),
    Some(&compose_options),
    None,  // <-- no harness plan!
    &approval_options,
)
```

The `None` for `harness_plan` means the pre-flight for `compose` and
`inline-compose` only discovers template `::shell` directives from
Darkmatter. It does **not** discover harness pre-check, post-check, or
handler shell commands.

The harness plan is only parsed _after_ the pre-flight, inside the
composition executor (`wrap/composition.rs:618-628`), where a second
call to `resolve_shell_approvals(None, None, Some(&plan), ...)` handles
harness commands.

The documentation (`pre-flight-checks.md` Step 1) says:

> Claudine gathers shell commands from **all three sources**:
> - Template directives
> - Harness checks
> - Harness handlers

This is implemented as two separate passes rather than a single unified
pass. The effect is that the user gets prompted twice: once for template
commands and once for harness commands. The documentation describes a
single prompt loop.

**Impact:** Minor UX issue — the user sees two rounds of approval
prompts instead of one. No security gap since both paths are covered.

**Recommendation:** Parse the harness plan earlier (before pre-flight)
and pass it to `resolve_shell_approvals` in a single call, matching the
documented behavior. Alternatively, update the documentation to describe
the two-phase approval.

---

## Inconsistency 3: Sequence pre-flight does not include harness commands

**Severity: Low**

In `wrap/sequence.rs:99`, the per-step pre-flight call is:

```rust
composition::resolve_shell_approvals(
    Some(&source.markdown),
    Some(&compose_options),
    None,  // <-- no harness plan!
    &approval_options,
)
```

Same pattern as compose/inline-compose — harness commands are not
discovered during the sequence pre-flight loop. The harness plan is
handled later inside the composition executor.

**Impact:** Same as Inconsistency 2 — two-phase approval rather than
single-pass.

---

## Inconsistency 4: Documentation does not mention Darkmatter frontmatter `$(...)` shell expansion as a pre-flight source

**Severity: Low**

The documentation (`pre-flight-checks.md`) lists three sources of shell
commands:

1. Template `::shell` directives
2. Harness pre-checks and post-checks
3. Harness handlers

It does **not** mention frontmatter `$(cmd)` shell expressions as a
fourth source. However, the implementation in Darkmatter's
`discovery.rs` correctly discovers both sources:

- **Phase 1:** `scan_frontmatter(...)` — discovers `$(cmd)` in top-level
  frontmatter string values (line 94-132)
- **Phase 2:** Body `::shell` directives from composed output (line 134-189)

The Darkmatter skill (`darkmatter/compose.md`) and the darkmatter SKILL.md
both describe frontmatter shell expansion, but `pre-flight-checks.md`
omits it.

**Impact:** Documentation gap. Users may not realize that frontmatter
`$(cmd)` values are also pre-flighted.

**Recommendation:** Add frontmatter `$(cmd)` expressions as a fourth
source in `pre-flight-checks.md` Step 1.

---

## Inconsistency 5: `pre-flight-checks.md` says Darkmatter exposes `collect_shell_commands` but does not name frontmatter scanning

**Severity: Low**

From `pre-flight-checks.md` line 67:

> **Darkmatter's role is discovery.** It knows how to walk the document
> graph — following transclusions, resolving interpolation, parsing
> `::shell` directives. It exposes a function (`collect_shell_commands`)
> that returns every shell command in the document tree. **It does not
> check any policy files or make any approval decisions during this call.**

The description says "parsing `::shell` directives" but does not mention
frontmatter `$(...)` scanning. In reality, `collect_shell_commands` in
`discovery.rs` runs two phases: frontmatter shell scanning first, then
body directive scanning. The Darkmatter skill correctly documents this
two-phase approach, but the Claudine pre-flight documentation describes
only the body `::shell` half.

**Impact:** Documentation gap only.

**Recommendation:** Update the description to say "parsing `::shell`
directives **and frontmatter `$(...)` shell expressions**".

---

## Inconsistency 6: Composition pipeline docs omit Frontmatter Interpolation and Shell Expansion from the Inline Pre phase description

**Severity: Informational**

The `darkmatter/compose.md` skill file lists the Inline Pre phase as:

1. Text Replacement
2. Page Blocks
3. Interpolation
4. Shell Expansion

But the SKILL.md for darkmatter lists the actual compose pipeline order as:

1. **Frontmatter Interpolation** — `{{ variable }}` in frontmatter
2. **Frontmatter Shell Expansion** — top-level `$(cmd)` frontmatter values
3. Text Replacement
4. Page Blocks
5. Interpolation
6. Shell Expansion

The `compose.md` skill file omits frontmatter interpolation and frontmatter
shell expansion from the pipeline overview. This is relevant because the
discovery function (`collect_shell_commands`) in `discovery.rs` correctly
runs frontmatter interpolation before scanning for frontmatter shell
commands — matching the actual pipeline order. But the pipeline docs in
`compose.md` skip these two steps.

**Impact:** Documentation gap. Could mislead someone reading the compose
pipeline docs into thinking frontmatter shell commands are not part of the
pipeline.
