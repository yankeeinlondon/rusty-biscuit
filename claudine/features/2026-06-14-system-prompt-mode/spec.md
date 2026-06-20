---
clarified: claude/claude-opus-4-8
reviewed: true
review_iterations: 3
status: ready for planning and implementation
---

# System Prompt Mode — Frontmatter-Driven Replace for Discovered Files

## Problem

Today, the auto-discovered `system-prompt.md` is **always treated as append-mode** (`claudine/lib/src/system_prompt/prepare.rs` — `mode_for_source` returns `SystemPromptMode::Append` for every `StandardDiscovered` source). The only way to get replace semantics is to pass the file explicitly via the `--replace-system-prompt <FILE>` (alias `--rsp`) CLI switch, which also skips standard discovery entirely.

This forces an awkward choice; a project that wants replace semantics must either:

1. stop using the standard `system-prompt.md` filename and point at a differently named file with `--rsp` on every invocation, or
2. accept append semantics even when the intent is to fully replace the provider's default system prompt.

There is no way for the file itself to declare which mode it wants.

## Goal

Allow a discovered `system-prompt.md` to control its own delivery mode via a `mode` frontmatter property.

### Valid values

| Frontmatter value | Resulting mode | Notes                                                              |
|-------------------|----------------|--------------------------------------------------------------------|
| absent / null     | `Append`       | Current behavior — backwards compatible default                    |
| `"append"`        | `Append`       | Explicit append (same as default)                                  |
| `"replace"`       | `Replace`      | **New capability** — replaces the provider's default system prompt |

Any other value is invalid and is **rejected by Darkmatter's schema validation during compose** rather than silently falling back to append (see Design → Baseline schema and Error handling below). The valid-value set is enforced by a `mode` enum constraint in a baseline schema claudine supplies to the compose operation — not by a hand-rolled validator.

## Scope

### In scope

- A baseline `SimplifiedSchema` (declaring `mode` as `enum(append, replace; default(append))`) that claudine supplies to the compose operation **only for `StandardDiscovered` sources**, so the `mode` value is validated automatically during compose.
- Reading the resolved mode for a discovered file from the **composed frontmatter** (`mode: replace` → `Replace`, absent or `append` → `Append`) after compose returns, and propagating it through to provider delivery so that a discovered `mode: replace` file triggers the same replace delivery path as `--replace-system-prompt`.
- Keeping prompt-reporting behavior stable: the new `mode` property affects delivery only and does not change `verbosity`, report rendering, token counts, or the raw-text path used by `parse_frontmatter_verbosity`.
- Tests covering: absent mode (default append), explicit `"append"`, explicit `"replace"`, invalid values (rejected at compose), non-string values, and the empty-body edge case.
- Documentation update to `claudine/docs/topics/system-prompt.md`.
- Documentation update to `claudine/docs/topics/frontmatter-properties.md` so the new `mode` key is listed in the authoritative frontmatter catalog.

### Out of scope

- A bespoke typed error variant for unrecognized `mode` values. Validation is delegated to schema validation during compose; a bad value surfaces through the existing `SystemPromptComposition` compose-error path (see Error handling).
- Changes to the explicit CLI flags (`--append-system-prompt` / `--replace-system-prompt`). When a user passes an explicit flag, the flag determines the mode; the file's frontmatter `mode` property is **not consulted**, and the explicit path composes **without** the baseline schema. This preserves the existing contract that explicit flags are authoritative and skip discovery.
- Changes to `non-interactive.md` handling — the non-interactive safety appendix is always appended and is not affected by this feature.
- Changes to the discovery hierarchy (package → package-area → repo → user home).
- Changes to the composition/Darkmatter pipeline. The `mode` property is read from frontmatter but, like all frontmatter, is **not forwarded** to the provider as part of the composed prompt body.

## Design

### Baseline schema

claudine constructs a baseline `SimplifiedSchema` declaring a single property:

```
mode: enum(append, replace; default(append))
```

and passes it to the compose operation via
[`ComposeOptions::with_baseline_schema`]
(`darkmatter/lib/src/markdown/compose/context/options.rs`), **only for
`StandardDiscovered` sources**. Darkmatter merges this baseline with any
`$schema` the document declares (document side wins on conflict) and runs
schema validation as an always-on compose stage.

The baseline is a workspace-wide constraint registered by the caller without
editing every prompt file — exactly the use case `with_baseline_schema`
documents for callers like claudine.

**Construction.** The SimplifiedSchema enum grammar
(`enum(member1, member2; default(x))`) is verified against Darkmatter's grammar
tests. The verified programmatic entry point is
`darkmatter::markdown::schemas::parse_yaml_schema(value: &serde_yaml_ng::Value)`,
which accepts a YAML mapping whose property values are grammar strings — i.e.
the mapping `{ mode: "enum(append, replace; default(append))" }` parses to the
baseline `SimplifiedSchema`. (Darkmatter's own tests build comparable schemas
either from this YAML-mapping form or by constructing the AST directly via
`SimplifiedSchema::Single(SchemaShape { .. })`; the implementer may pick
whichever is least brittle. Both are verified to exist.) The baseline should
live as a small constant/builder helper in `prepare.rs` (or a dedicated
`schema.rs` sibling) so it is constructed once.

> Implementation note: JSON Schema `default(append)` is **annotation-only**.
> Darkmatter's frontmatter coercion does **not** backfill defaults into an
> absent key (verified: `schemas/coerce.rs` has no default-application path).
> So a discovered file with no `mode` key validates fine, but the composed
> frontmatter will **not** contain `mode` — the read-back below must therefore
> treat an absent key as `Append` in claudine's own code. The schema default
> only documents intent and keeps `mode` optional; it does not write a value.

### Where mode is resolved

For discovered files the mode is determined **after compose**, not before.
After `md.compose_with(options)` returns in `compose_prompt_markdown`, prepare
reads the composed frontmatter:

- `composed.fm_get::<String>("mode")` returns `Ok(Some(mode))` for string values, `Ok(None)` when absent, and an error for non-string values that escaped schema validation.
- `Ok(Some("replace"))` → `Replace`.
- `Ok(Some("append"))` → `Append`.
- `Ok(None)` → `Append` (default; the key is never backfilled — see note above).

Because schema validation already runs during compose, any value outside
`{append, replace}` (including a non-string) is normally rejected before this
read-back is reached. The one way an unexpected value could still arrive is if a
discovered file declares its **own** `$schema` with a conflicting `mode` type:
the merge rule is *document-side-wins*, so a user-declared schema could relax
the baseline enum. The read-back must therefore stay defensive — any string
that is neither `append` nor `replace` falls through to `Append` (the
backwards-compatible default), matching the "absent" arm rather than panicking
or silently mis-delivering. A non-string read-back error should also resolve to
`Append` in this conflict-override case, optionally with a `tracing::warn!`, not
abort: if the document has intentionally overridden the baseline schema, runtime
delivery should keep the historical default rather than introduce a second,
schema-bypassing validation channel. This keeps the common path (no document
`$schema`) strictly enum-validated while remaining robust to the obscure
override case.

This requires restructuring `prepare_system_prompt_with_ctx` /
`prepare_system_prompt`: today they call `mode_for_source(&source)` **before**
compose and hardcode `Append` for discovered files. The discovered-file mode
must instead be computed from the composed result. `mode_for_source` stays
responsible only for `ExplicitFile` (flag-driven) and the Append-by-default
sources (`NonInteractiveFile`, `BuiltInNonInteractive`).

### Data model

No change to `SystemPromptSource::StandardDiscovered`. The original plan to add
a `mode` field to the variant is **dropped**: the mode is no longer carried on
the source, it is read from the composed frontmatter. `resolve.rs` therefore
needs no change either — `discover_standard_file` continues to return
`(SystemPromptSource::StandardDiscovered { path, scope }, String)` with a plain
`read_to_string`.

### Error handling

There is **no** custom `ClaudineError::InvalidSystemPromptMode` variant. An
unrecognized `mode` value (e.g. `mode: replce`) is rejected during compose as a
`MarkdownError::SchemaValidationFailed`, which already flows into claudine
through the existing `ClaudineError::SystemPromptComposition(MarkdownError)`
path (`claudine/lib/src/error.rs`). The compose error message names the
offending property and the valid values (e.g. `mode: must be one of: append,
replace`).

This is a **deliberate, accepted trade-off**: the user explicitly chose to let
schema validation own mode validation rather than maintain a bespoke typed
error. The error is still **fatal** (it propagates as `Err`), preserving the
prior rationale — a typo like `mode: replce` must not silently produce append
behavior, since the system prompt is high-stakes enough that a blocked startup
with a clear message beats silent misconfiguration. The only thing that changes
is the error *channel*: schema validation, not a bespoke variant.

If a document-owned `$schema` intentionally redefines `mode`, that is treated as
an advanced schema override rather than a user typo. In that override case,
schema validation may permit values that claudine does not use for delivery;
read-back falls back to `Append` as described above. The docs must call this out
briefly so users understand that redefining `mode` in `$schema` opts out of
baseline validation but does not create additional delivery modes.

[`ComposeOptions::with_baseline_schema`]: ../../../../darkmatter/lib/src/markdown/compose/context/options.rs

### Interaction with explicit flags

| Invocation                     | Mode source              | Frontmatter consulted? |
|--------------------------------|--------------------------|------------------------|
| `claudine claude` (discovery)  | Composed frontmatter `mode` | Yes (baseline schema applied) |
| `claudine claude --asp <file>` | Flag (`--asp` → Append)     | No (no baseline schema)       |
| `claudine claude --rsp <file>` | Flag (`--rsp` → Replace)    | No (no baseline schema)       |

When an explicit flag is used, `resolve_system_prompt_source` returns early
from its `--asp` / `--rsp` branches **before** `discover_standard_file` runs, so
a discovered file's frontmatter is structurally never read. This is the
clarification behind "the explicit switch also skips standard discovery
entirely": it is an existing property of the resolve order, requiring no new
code. Explicit-file sources also compose **without** the baseline schema, so
their own frontmatter `mode` (if any) is never validated or consulted — the
mode comes from the flag.

### Non-interactive sessions

When a discovered `system-prompt.md` declares `mode: replace` and the session is non-interactive, the replace semantics are preserved even after the non-interactive safety appendix is appended (matching the existing behavior for explicit `--replace-system-prompt`). The appendix is content, not a mode change.

### Empty-body edge case

A discovered file whose composed body trims to empty resolves to
`ResolvedSystemPrompt::Disabled`, **regardless of `mode`** (mode-independent).
A frontmatter-only `mode: replace` file with no body is therefore `Disabled`
(the provider keeps its default) — it is **not** an error and **not** an empty
replacement. This keeps the change surgical: "replace with nothing" is a
degenerate case with no demonstrated use case, and the existing empty-body
→ `Disabled` check in `prepare_system_prompt*` already produces this outcome
without special-casing mode. The test plan pins this behavior.

### CLI display

The `describe_source` function in
`claudine/cli/src/commands/wrap/system_prompt.rs` describes the source by
`path` and `scope` only; it does **not** pattern-match a `mode` field on
`StandardDiscovered` (there is none). The resolved mode is already surfaced by
`describe_effective`, which prints `mode: append|replace` from
`prepared.mode` — the value prepare computed from the composed frontmatter. No
new field-match is required in `describe_source`; the existing
`StandardDiscovered { path, scope }` arm stays as-is, and dry-run output
continues to show the effective mode via `describe_effective`.

### Prompt reporting and frontmatter catalog

`mode` is a delivery-control property for discovered `system-prompt.md` files.
It must not be reused as a prompt-reporting setting and must not be parsed by
`prompt_reporting::frontmatter`. Prompt reports already render the effective
mode from `PreparedSystemPrompt.mode`; after this change that value will reflect
the composed-frontmatter decision for discovered files. `parse_frontmatter_verbosity`
continues to read only the raw `verbosity` property so reporting verbosity stays
orthogonal to delivery mode.

Because `claudine/docs/topics/frontmatter-properties.md` is the catalog of
frontmatter keys with special meaning, add a Prompt Reporting/System Prompt row:

- `mode` — controls delivery mode for automatically discovered
  `system-prompt.md` files only; accepts `append` or `replace`; absent/null
  defaults to append; explicit `--append-system-prompt` and
  `--replace-system-prompt` files ignore this key.

This is documentation-only; no runtime code should consult the catalog.

## Affected files

| File                                              | Change                                                                                                                                                                                     |
|---------------------------------------------------|------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------|
| `claudine/lib/src/system_prompt/prepare.rs`       | Define the baseline `SimplifiedSchema` (constant/builder); pass it via `ComposeOptions::with_baseline_schema` in `compose_prompt_markdown` for `StandardDiscovered` sources only; return the composed `Markdown` (not just its string) so prepare can read `mode`; restructure `prepare_system_prompt*` to compute the discovered-file mode from composed frontmatter rather than `mode_for_source`; update test construction sites |
| `claudine/lib/src/system_prompt/types.rs`         | **No change.** `StandardDiscovered` does not gain a `mode` field; mode is read from composed frontmatter                                                                                  |
| `claudine/lib/src/system_prompt/resolve.rs`       | **No change.** `discover_standard_file` keeps its `read_to_string`; the discovered source carries no mode                                                                                  |
| `claudine/lib/src/error.rs`                       | **No change.** No new variant; bad `mode` surfaces through the existing `SystemPromptComposition(MarkdownError)` path                                                                      |
| `claudine/cli/src/commands/wrap/system_prompt.rs` | **No change.** `describe_source` keeps the `StandardDiscovered { path, scope }` arm; effective mode is already shown by `describe_effective` from `prepared.mode`                          |
| `claudine/docs/topics/system-prompt.md`           | Document the `mode` frontmatter property; update the "standard discovered file is always append-mode" statement; note that validation is enforced by the baseline schema during compose    |
| `claudine/docs/topics/frontmatter-properties.md`  | Add `mode` to the authoritative frontmatter-property catalog as a system-prompt delivery key; make explicit that it only applies to discovered `system-prompt.md` files                     |

## Test plan

1. **Absent mode → Append (default)** — discovered file with no `mode` frontmatter resolves to `SystemPromptMode::Append`. (Covers backwards compatibility; also confirms the absent key is treated as Append by claudine, since the schema default is not backfilled into the composed frontmatter.)
2. **Explicit `"append"`** — discovered file with `mode: append` resolves to `Append`.
3. **Explicit `"replace"`** — discovered file with `mode: replace` resolves to `Replace`.
4. **Invalid string value rejected at compose** — discovered file with `mode: overwrite` fails compose; the error is a `ClaudineError::SystemPromptComposition` wrapping a `MarkdownError::SchemaValidationFailed` (not a bespoke `InvalidSystemPromptMode`).
5. **Non-string value rejected at compose** — discovered file with `mode: 42` fails schema validation during compose and surfaces via the same `SystemPromptComposition` path.
6. **Full pipeline** — `resolve_and_prepare_for_session` with a discovered `mode: replace` file produces a `PreparedSystemPrompt` with `mode: Replace` that flows through to provider delivery.
7. **Explicit flag ignores frontmatter** — `--replace-system-prompt` pointing at a file that happens to contain `mode: append` in frontmatter still uses Replace. The flag wins because the explicit path composes without the baseline schema and the discovered-file frontmatter is structurally never read (resolve returns early before discovery).
8. **Non-interactive + replace mode** — discovered file with `mode: replace` in a non-interactive session preserves Replace mode after the safety appendix is appended.
9. **Empty body + `mode: replace` → Disabled** — a discovered file that is frontmatter-only (`mode: replace`, no body) composes to an empty body and resolves to `ResolvedSystemPrompt::Disabled` regardless of the declared mode (mode-independent; not an error, not an empty replacement).
10. **Document `$schema` conflict falls back defensively** — a discovered file whose `$schema` redefines `mode` to allow another value composes successfully, but claudine resolves the effective delivery mode to `Append` rather than panicking, inventing a new mode, or returning a bespoke mode error. If a warning is emitted, assert it through tracing only where the existing test harness has a stable capture path.
11. **Prompt report mode reflects effective delivery mode** — the existing system prompt summary/dry-run report should show `mode: replace` for a discovered `mode: replace` file because it reads `PreparedSystemPrompt.mode`, while `verbosity` frontmatter continues to control only report verbosity.
12. **Frontmatter catalog updated** — docs test/review check only: `claudine/docs/topics/frontmatter-properties.md` includes the new `mode` row and does not claim explicit system-prompt files consult the property.
