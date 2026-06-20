# System Prompt Mode — Frontmatter-Driven Replace for Discovered Files

## Problem

Today, the auto-discovered `system-prompt.md` is **always treated as append-mode** (`claudine/lib/src/system_prompt/prepare.rs` — `mode_for_source` returns `SystemPromptMode::Append` for every `StandardDiscovered` source). The only way to get replace semantics is to pass the file explicitly via the `--replace-system-prompt <FILE>` (alias `--rsp`) CLI switch, which also skips standard discovery entirely.

This forces an awkward choice: a project that wants replace semantics must either

1. stop using the standard `system-prompt.md` filename and point at a differently    named file with `--rsp` on every invocation, or
2. accept append semantics even when the intent is to fully replace the    provider's default system prompt.

There is no way for the file itself to declare which mode it wants.

## Goal

Allow a discovered `system-prompt.md` to control its own delivery mode via a `mode` frontmatter property.

### Valid values

| Frontmatter value | Resulting mode | Notes                                                              |
|-------------------|----------------|--------------------------------------------------------------------|
| absent / null     | `Append`       | Current behavior — backwards compatible default                    |
| `"append"`        | `Append`       | Explicit append (same as default)                                  |
| `"replace"`       | `Replace`      | **New capability** — replaces the provider's default system prompt |

Any other value is invalid and must produce a typed error rather than a silent fallback (see Error Handling below).

## Scope

### In scope

- Parsing the `mode` frontmatter property from auto-discovered   `system-prompt.md` files during the resolve phase.
- Propagating the resolved mode through to provider delivery so that   `mode: replace` in a discovered file triggers the same replace delivery path   as `--replace-system-prompt`.
- A typed error variant for unrecognized `mode` values.
- Tests covering: absent mode (default append), explicit `"append"`, explicit   `"replace"`, and invalid values.
- Documentation update to `claudine/docs/topics/system-prompt.md`.

### Out of scope

- Changes to the explicit CLI flags (`--append-system-prompt` / `--replace-system-prompt`).   When a user passes an explicit flag, the flag determines the mode; the file's   frontmatter `mode` property is **not consulted**. This preserves the existing   contract that explicit flags are authoritative and skip discovery.
- Changes to `non-interactive.md` handling — the non-interactive safety appendix   is always appended and is not affected by this feature.
- Changes to the discovery hierarchy (package → package-area → repo → user home).
- Changes to the composition/Darkmatter pipeline. The `mode` property is read   from frontmatter but, like all frontmatter, is **not forwarded** to the   provider as part of the composed prompt body.

## Design

### Where mode is resolved

The resolve phase (`claudine/lib/src/system_prompt/resolve.rs`) already reads each discovered file's text. The frontmatter `mode` property is parsed there and carried alongside the existing `path` and `scope` on the `SystemPromptSource::StandardDiscovered` variant.

This mirrors how `SystemPromptSource::ExplicitFile` already carries its own `mode` field (set from the CLI flag that selected it).

### Data model change

`SystemPromptSource::StandardDiscovered` gains a `mode: SystemPromptMode` field. The `mode_for_source` helper in `prepare.rs` then reads the mode from the source variant rather than hardcoding `Append` for all discovered files.

### Frontmatter parsing

Mode is extracted from the raw file text using Darkmatter's `Markdown` frontmatter parser — the same library already used in the prepare phase for composition. The extraction is a simple typed lookup:

- `frontmatter.as_map().get("mode")` returns `None` → `Append` (default).
- Returns `Some(Value::String("append"))` → `Append`.
- Returns `Some(Value::String("replace"))` → `Replace`.
- Returns `Some(Value::String(other))` → error.
- Returns `Some(Value::NonString)` → error (the property must be a string).

### Error handling

An unrecognized `mode` value produces a new `ClaudineError::InvalidSystemPromptMode` variant carrying:

- `value` — the offending value as it appeared in frontmatter (stringified for   non-string values).
- `path` — the file path, so the error message points the user at the right file.

The error is **fatal** (propagates as `Err`), not a warning. Rationale: a typo like `mode: replce` would silently produce append behavior — the opposite of the user's intent — and the system prompt is high-stakes enough that silent misconfiguration is worse than a blocked startup with a clear message.

### Interaction with explicit flags

| Invocation                     | Mode source              | Frontmatter consulted? |
|--------------------------------|--------------------------|------------------------|
| `claudine claude` (discovery)  | Frontmatter `mode`       | Yes                    |
| `claudine claude --asp <file>` | Flag (`--asp` → Append)  | No                     |
| `claudine claude --rsp <file>` | Flag (`--rsp` → Replace) | No                     |

When an explicit flag is used, standard discovery is skipped entirely (existing behavior), so the discovered file's frontmatter is never read.

### Non-interactive sessions

When a discovered `system-prompt.md` declares `mode: replace` and the session is non-interactive, the replace semantics are preserved even after the non-interactive safety appendix is appended (matching the existing behavior for explicit `--replace-system-prompt`). The appendix is content, not a mode change.

### CLI display

The `describe_source` function in `claudine/cli/src/commands/wrap/system_prompt.rs` should reflect the resolved mode for discovered files (e.g., appending the mode label to the scope label), so dry-run output and diagnostics show whether a discovered file is being used in append or replace mode.

## Affected files

| File                                              | Change                                                                                                             |
|---------------------------------------------------|--------------------------------------------------------------------------------------------------------------------|
| `claudine/lib/src/system_prompt/types.rs`         | Add `mode: SystemPromptMode` to `StandardDiscovered` variant                                                       |
| `claudine/lib/src/error.rs`                       | Add `InvalidSystemPromptMode { value, path }` variant                                                              |
| `claudine/lib/src/system_prompt/resolve.rs`       | Parse `mode` frontmatter in `discover_standard_file`                                                               |
| `claudine/lib/src/system_prompt/prepare.rs`       | Update `mode_for_source` to read mode from `StandardDiscovered`; update existing test construction sites           |
| `claudine/cli/src/commands/wrap/system_prompt.rs` | Update `describe_source` pattern match for new field; optionally show mode in display                              |
| `claudine/docs/topics/system-prompt.md`           | Document the `mode` frontmatter property and update the "standard discovered file is always append-mode" statement |

## Test plan

1. **Absent mode → Append (default)** — discovered file with no `mode` frontmatter    resolves to `SystemPromptMode::Append`. (Covers backwards compatibility.)
2. **Explicit `"append"`** — discovered file with `mode: append` resolves to `Append`.
3. **Explicit `"replace"`** — discovered file with `mode: replace` resolves to `Replace`.
4. **Invalid string value** — discovered file with `mode: overwrite` produces    `InvalidSystemPromptMode` error.
5. **Non-string value** — discovered file with `mode: 42` produces    `InvalidSystemPromptMode` error.
6. **Full pipeline** — `resolve_and_prepare_for_session` with a discovered    `mode: replace` file produces a `PreparedSystemPrompt` with `mode: Replace`    that flows through to provider delivery.
7. **Explicit flag ignores frontmatter** — `--replace-system-prompt` pointing at    a file with `mode: append` in frontmatter still uses Replace (flag wins).
8. **Non-interactive + replace mode** — discovered file with `mode: replace` in a    non-interactive session preserves Replace mode after the safety appendix is    appended.
