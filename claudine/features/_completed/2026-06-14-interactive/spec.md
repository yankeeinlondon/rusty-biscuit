---
created: 2026-06-14
reviewed: true
status: ready for planning and implementation
area:
  - claudine
  - claudine-cli
---

# Frontmatter-Driven Interactive Sessions & Session-Independent Schema Collection

## Problem

Two related gaps exist in Claudine composition today.

**1. There is no way for a prompt document to request an interactive session.**
Interactive provider-session mode is controlled exclusively by the `-i` /
`--interactive` CLI flag (`SharedComposeArgs::interactive`, wired into
`CompositionExecutionRequest::session_interactive`). `compose` defaults to a
non-interactive (one-shot) session. Some prompts are *inherently* dialog-shaped —
they exist to open a back-and-forth with the model — and today the author has no
way to encode that intent in the document. Every invocation must remember to pass
`-i`, and the prompt's nature is invisible to anyone reading it. The current
catalog of Claudine-recognized frontmatter properties in
`docs/topics/frontmatter-properties.md` has **no** `interactive` member, and
the current composition selection hints only expose `agent` and `model`.

**2. The relationship between schema-required collection and session mode is
unspecified, and confidence that collection always precedes session start is
low.** The schema missing-property prompt is gated solely by
`InteractiveSchemaOptions::allowed()`:

```rust
// claudine/lib/src/composition/schema_validation.rs
pub const fn allowed(self) -> bool {
    self.prompt_for_missing && self.stdin_is_tty && self.stderr_is_tty && !self.silent
}
```

This gate is already independent of `session_interactive`, and the call sites in
`run_compose_inner` / `run_inline_compose_inner` run it near the top of the
function — before any loop/interactive/execution divergence. **In principle the
behavior is already correct.** But it is neither documented as a guarantee nor
covered by tests for the interactive case: the only L2 PTY coverage
(`level2_schema_prompt_pty.rs`) exercises `compose --goose` *without* `-i`.
Introducing a frontmatter `interactive` switch makes a future regression more
likely (a document that opens an interactive session could plausibly skip
collection), so the invariant must be pinned down and tested.

## Goals

1. Add an `interactive` boolean frontmatter property that sets the default
   session mode for a `compose` document. `{ interactive: true }` makes the
   composed prompt open an interactive provider session without requiring `-i` on
   the command line.
2. Define a clear, conventional precedence between the CLI flag and the
   frontmatter value, and add a CLI escape hatch to force a non-interactive run
   when the document requests interactive.
3. Formalize — and lock with tests — the invariant that schema-required values
   are collected (when a TTY is available) **before the provider session starts,
   regardless of the resolved session mode**.
4. Preserve a diagnostic source for the resolved session mode so conflicts can
   explain whether interactivity came from `--interactive`, `--no-interactive`,
   `interactive: true`, or the default.

## Non-Goals

- No change to *how* an interactive session is delivered per provider; this spec
  only changes how `session_interactive` is *resolved*.
- No new interactive collection for shapes that are already unsupported
  (`object`, `any`, property-level unions, raw JSON Schema) — those continue to
  surface `UnsupportedInteractiveSchema`.
- No document-authored interactive mode for `sequence`. The existing
  `sequence --interactive` CLI surface is not changed by this spec, but
  `interactive: true` frontmatter must not opt a serial sequence into dialog
  mode.

---

## Feature 1 — The `interactive` Frontmatter Property

### Semantics

`interactive` is a boolean frontmatter property naming the document's **default
session mode**.

- `interactive: true` — the composed prompt is prepared and then handed to the
  provider as the opening message of an **interactive** session (equivalent to a
  default of `-i`).
- `interactive: false` — explicit non-interactive default (same as omitting it).
- Absent — defaults to `false`, preserving today's behavior.

A non-boolean value is a hard, typed error
(`CompositionError::InteractiveHintWrongType`, mirroring `AgentHintWrongType`),
named with the offending JSON type. The value is **not** templated/composed — it
is read from authored frontmatter as a literal boolean, like `fail_fast`.

`null` is treated the same as an absent property. This matches `agent`/`model`
hint parsing and lets shared prompt templates clear the document default
without tripping a type error.

### Resolution & Precedence

Resolution follows the established CLI-over-frontmatter-over-default convention
(cf. `model`, `fail_fast`):

1. **CLI `--no-interactive`** → `false` (highest precedence)
2. **CLI `-i` / `--interactive`** → `true`
3. **Frontmatter `interactive: <bool>`** → that value
4. **Default** → `false`

Because clap's existing `-i` is a presence flag (a plain `bool` cannot express
"the user explicitly wants non-interactive"), this spec adds a paired
`--no-interactive` flag. Resolution reads the two flags into an explicit
override:

```rust
// pseudocode in SharedComposeArgs resolution
let cli_override: Option<bool> = if self.no_interactive {
    Some(false)
} else if self.interactive {
    Some(true)
} else {
    None
};
let session_interactive = cli_override
    .or(frontmatter_interactive)   // from EffectiveSelectionHints
    .unwrap_or(false);
```

`-i` and `--no-interactive` are mutually exclusive (`conflicts_with`).

The resolver must return both the boolean and its source:

```rust
pub enum SessionInteractivitySource {
    NoInteractiveFlag,
    InteractiveFlag,
    Frontmatter,
    Default,
}

pub struct ResolvedSessionInteractivity {
    pub value: bool,
    pub source: SessionInteractivitySource,
}
```

This source is required for timeout conflict messages, inline-compose provider
gating, dry-run metadata, tracing, and future reporting.

### Scope by Command

| Command          | `interactive` frontmatter behavior |
|------------------|-------------------------------------|
| `compose`        | **Fully supported.** Sets the session-mode default per the precedence above. |
| `inline-compose` | Honored, but subject to the existing constraint: `inline-compose` in interactive mode is rejected for providers that cannot capture the final assistant message (`CompositionError::InlineInteractiveUnsupported`). When `interactive` resolves to `true` for such a provider, emit that same typed error family. The diagnostic must name the *resolved source* of the interactive intent (frontmatter vs flag) so the remediation is clear. |
| `sequence`       | **Hard-rejected when `interactive: true` is authored in the sequence document frontmatter.** A sequence is serial automation; a document-level dialog default would be ambiguous across steps and hard to reason about. `interactive: false`, `interactive: null`, and an absent key are all equivalent no-op defaults. The error should name `interactive`, point to `compose`/`inline-compose` for dialog-shaped prompts, and mention that `--interactive` remains the only explicit override if the existing CLI behavior is intentionally used. |

Reader's note: the hard-reject decision is intentional. Warn-and-ignore is more
forgiving, but it hides a control key that the author likely expected to matter;
honoring it per step would require defining TTY ownership, failure semantics, and
inline closure behavior for every step. Rejection keeps the new document contract
clear without removing today's explicit CLI behavior.

### Interaction with `--timeout` / `--step-timeout`

Today `--timeout` and `--step-timeout` conflict with `--interactive`
in the compose/inline/sequence command entry points and again in the wrapper
executor. With frontmatter-driven interactivity the conflict check must run
against the **resolved** session mode, not just the raw CLI flag:

- Frontmatter `interactive: true` + a timeout (CLI or otherwise) → the same
  conflict diagnostic currently produced for `-i` + `--timeout`.
- `--no-interactive` + `interactive: true` frontmatter + `--timeout` → allowed
  (the CLI forced non-interactive, so timeouts are valid again).

The conflict diagnostic must state which signal made the session interactive.
Because timeout values can also come from effective frontmatter and environment
defaults, the implementation must check conflicts at the last point where both
the resolved session mode and resolved timeout plan are known. Early CLI-only
syntax validation may stay, but it must not be the only conflict check.

### Interaction with `$schema`

When a document declares `$schema`, `interactive` is an ordinary undeclared
frontmatter key and is **not** validated (schema validation only checks declared
properties; extra keys pass through, exactly as `title`/`description` do today).
If a future "closed schema" mode (`additionalProperties: false`) is added,
`interactive` must be treated as a reserved control key exempt from that
rejection — alongside `loop`, `sequence`, `fail_fast`, `prompt`, `$schema`,
`last_updated`, `agent`, `model`, `timeout`, `step_timeout`, lifecycle keys, and
harness keys. (No carve-out is needed today; noted for forward-compatibility.)

### Interaction with `--dry-run`

`--dry-run` still resolves `interactive` and includes the resolved mode/source in
the dry-run metadata table, but it never launches a provider session. Schema
collection, shell approval, provider/model resolution, and timeout conflict
validation keep their normal pre-launch behavior so dry-run remains a rehearsal
of the real invocation.

---

## Feature 2 — Schema Collection Is Independent of Session Mode

### Requirement

Schema-required property collection MUST be attempted **before** the provider
session is launched, and its decision to prompt MUST depend only on:

1. `prompt_for_missing` (user config, default `true`),
2. stdin is a TTY,
3. stderr is a TTY,
4. `--silent` is not set,

and MUST NOT depend on the resolved `session_interactive` value (neither the
`-i` flag nor `interactive: true` frontmatter). When a TTY is unavailable, the
non-interactive `MissingProperties` report path is used as today.

This makes "open an interactive session" and "the session has all required
inputs" two orthogonal guarantees: an interactive session never starts with a
required schema value left unfilled when the terminal could have asked for it.

### Why this is mostly already true

The current code already satisfies the gate-independence in both `compose` and
`inline-compose` (the `pre_validate_with_interactive_collection` call precedes
execution and ignores `session_interactive`). This feature's job is to (a) make
it a *documented invariant*, (b) guarantee the **ordering** holds once a document
can self-select interactive mode, and (c) add the missing test coverage.

### Ordering guarantee

`pre_validate_with_interactive_collection` runs while Claudine still owns the
controlling terminal. The provider — interactive or not — is only spawned in the
executor, strictly after collection completes and the collected values are merged
into the overrides. The biscuit-tui prompt and the eventual interactive session
must never contend for the TTY simultaneously. Any refactor that moves session
launch earlier must preserve this ordering.

### TTY caveat (explicit)

"Regardless of whether the session is interactive" refers to the **session
mode**, not to terminal availability. Interactive *collection* still
fundamentally requires a TTY to prompt on; in a non-TTY environment (CI, piped
stdin/stderr) the hard `MissingProperties` error remains the only correct
outcome. The spec does not promise prompting where no terminal exists.

---

## Implementation Touchpoints

| Concern | Location |
|--------|----------|
| Frontmatter parse → hint | `EffectiveSelectionHints` (`lib/src/composition/types.rs`), `parse_selection_hints_from_frontmatter` (`lib/src/composition/prepare.rs`) — add `interactive: Option<bool>` and a `parse_interactive_hint` mirroring `parse_model_hint` for `bool`/`null` |
| New typed error | `CompositionError::InteractiveHintWrongType` (`lib/src/composition/error.rs`) |
| Resolution type | Add `ResolvedSessionInteractivity` + `SessionInteractivitySource` in the CLI command layer or shared composition types; pass it far enough downstream for diagnostics and dry-run metadata |
| CLI flags | `SharedComposeArgs` (`cli/src/commands/compose.rs`) — add `--no-interactive`; resolution helper computing `ResolvedSessionInteractivity` |
| Wiring | Every `session_interactive: shared.interactive` request construction site in `cli/src/commands/compose.rs` and `cli/src/commands/wrap/sequence.rs` → route through the resolver; keep sequence frontmatter rejection separate from CLI `--interactive` |
| Sequence rejection | `cli/src/commands/sequence.rs` after source load and before schema/plan execution — parse raw authored frontmatter and hard-error on literal `interactive: true` |
| Timeout conflict | command entry points and `wrap/composition/mod.rs` → check resolved mode and resolved timeout plan; keep early duration syntax validation |
| inline-compose guard | `wrap/composition/mod.rs` `InlineInteractiveUnsupported` guard and `error.rs` diagnostic rendering |
| Schema invariant | `cli/src/commands/schema_interactive.rs` (no logic change expected; add doc comment asserting independence from session mode) |
| Docs | `docs/topics/composition.md` (§ `The --interactive Flag`, § Schema Validation, § Dry Run), `docs/topics/frontmatter-properties.md`, the `claudine` skill |

## Testing

- **Unit:** `parse_interactive_hint` accepts `true`/`false`/absent; rejects
  non-boolean with `InteractiveHintWrongType`; treats `null` as absent. Resolver precedence table
  (`--no-interactive` > `-i` > frontmatter > default), including the
  mutually-exclusive flag conflict.
- **Unit:** timeout-conflict fires for frontmatter `interactive: true`; is
  suppressed by `--no-interactive`; conflict messaging names the source.
- **Unit:** `sequence` rejects authored `interactive: true` with the
  sequence-specific diagnostic, while `interactive: false` and `null` behave as
  absent.
- **Unit:** dry-run render includes the resolved session mode/source when a doc
  uses `interactive: true`.
- **L2 PTY (new, fills the current gap):**
  1. `compose -i` with a missing required `string` property → prompt appears and
     is collected *before* the provider stub launches.
  2. `compose` on a doc with `interactive: true` frontmatter (no `-i`) → same:
     prompt precedes session start.
  3. `--no-interactive` on an `interactive: true` doc → non-interactive session,
     but schema prompt still appears under a TTY.
- **L1:** non-TTY run of an `interactive: true` doc with a missing required
  property still emits the typed `MissingProperties` report (no hang, no prompt).

## Backward Compatibility

- Documents without `interactive` are unchanged (default `false`).
- `-i` / `--interactive` keep their exact current meaning; `--no-interactive` is
  purely additive.
- The schema-collection behavior is unchanged for existing flows; only its
  guarantee surface (docs + tests) grows.
- `sequence --interactive` is not removed by this spec. Only the new
  frontmatter key is rejected under `sequence` to avoid ambiguous document-level
  behavior.

## Open Questions

1. **Reporting:** should the resolved session mode and its *source*
   (flag/frontmatter/default) be recorded in the JSONL session row alongside the
   existing `interactive` TTY field used by `reporting`? Useful for `logs trends`
   but adds a column. Suggested solutions:
   - Add columns now. Pros: complete observability from day one, easier trend
     analysis. Cons: reporting migration surface grows with a feature that is
     otherwise mostly CLI/frontmatter resolution.
   - Emit only tracing/dry-run metadata now. Pros: lower implementation risk and
     no schema migration. Cons: historical JSONL cannot answer why a session was
     interactive.
   - Store source only in an extensible metadata blob. Pros: avoids dedicated
     columns. Cons: less ergonomic for `logs trends`.

   *Recommendation:* add tracing/dry-run metadata now and defer JSONL schema
   expansion until a reporting feature needs it. The implementation already has
   several pre-launch touchpoints; adding a log migration here is avoidable.
2. **`config` default:** is a user-config "default interactive" knob desirable,
   or is per-document + per-invocation control sufficient? *Recommendation:*
   sufficient; do not add a global default.
