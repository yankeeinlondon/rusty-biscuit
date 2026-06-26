---
status: ready for planning
created: 2026-06-26
area: claudine
packages:
    - darkmatter
    - claudine
---

# Malformed Frontmatter Fence Silently Leaks YAML Into the Agent Prompt

## Symptom

Composing `prompts/cross-platform.md` renders the file's YAML frontmatter
(`name:`, `description:`, and the `1. macOS / 2. Linux / 3. Windows` list) as
the first lines of the **Agent Prompt**, immediately followed by the real body
heading `# Ensuring Cross Platform Support`:

```
Agent Prompt

  name: cross-os description: |- A review process that looks for indications
  that source code is NOT appropriately cross-platform. Packages in this
  monorepo are expected to run on:

      1. macOS
      2. Linux
      3. Windows:
          - native
          - WSL
  ----------------------------------------------------------------
  # Ensuring Cross Platform Support
```

The frontmatter was never stripped. It was sent verbatim to the provider as
part of the prompt. No error and no warning were emitted.

## Root cause

The prompt file uses a **four-dash** fence, not the standard three-dash fence:

```
----                       # line 1  (byte-verified: "----\n")
name: cross-platform
description: |-
    ...
----                       # line 12
# Ensuring Cross Platform Support
```

Darkmatter's frontmatter parser requires the opening delimiter to equal `---`
exactly. `darkmatter/lib/src/markdown/frontmatter.rs:213`:

```rust
pub(super) fn parse_frontmatter(
    content: &str,
    ctx: SourceContext,
) -> MarkdownResult<(Frontmatter, String)> {
    let lines: Vec<&str> = content.lines().collect();

    // Check if document starts with frontmatter delimiter
    if lines.is_empty() || lines[0].trim() != "---" {        // line 220
        return Ok((Frontmatter::new(), content.to_string()));
    }
    ...
    .position(|line| line.trim() == "---")                   // line 228 (closing)
```

`lines[0].trim()` is `"----"`, which `!= "---"`, so the guard at line 220 takes
the early return: an **empty** `Frontmatter` plus the **entire raw document**
(both fences and the YAML included) as the body. The YAML-parsing path is never
reached, so there is nothing to fail on — the function succeeds and the caller
treats the whole file as prompt text. The closing-fence scan at line 228 has the
same exact-match constraint, so a mixed `---` / `----` pair would fail too.

The damage is specific to the prompt-execution surfaces (`compose`,
`inline-compose`, `sequence`): a silently-empty frontmatter means provider
selection, `name`, `description`, lifecycle events, and schema all vanish, and
the frontmatter text is shipped to the model as instructions.

This is the inverse of a guarantee the codebase already holds elsewhere — *a
frontmatter value that is executable state must never leak into the prompt as
raw text*. Here an **entire malformed frontmatter block** leaks, and unlike the
existing leak guards it does so without a diagnostic.

## Immediate workaround

Change both `----` fences in `prompts/cross-platform.md` to `---`. The file is
left unmodified for now so it remains a live reproduction for the fix.

## The principle

> A document that *almost* declares frontmatter — a dash-only fence that is not
> exactly `---` wrapping YAML-shaped content — is an authoring mistake, not
> prose. Composition must refuse it loudly instead of shipping the raw YAML to
> the provider.

Three-dash fences are the universal frontmatter convention (Jekyll, Hugo,
Obsidian, every static-site generator). We do **not** broaden what counts as
valid frontmatter; `---` stays the one true fence. We only convert the
near-miss silent-leak into a precise, actionable error.

## Proposed fix

Add near-miss-fence detection to `parse_frontmatter` and raise a new typed
`MarkdownError` instead of silently returning the raw document as body.

### Detection heuristic (conservative — must not fire on legitimate prose)

A line of three-or-more dashes at the very top of a document is also a valid
CommonMark thematic break, so detection must be tight enough that a real leading
horizontal rule is never misread as a broken fence. Raise the error only when
**all** of the following hold:

1. `lines[0].trim()` is a dash-only run (`^-+$`) of length `>= 3` **and** is not
   exactly `---` (i.e. length `>= 4`).
2. A later line exists whose trimmed content is the *same* dash-only run
   (matched closing fence).
3. The content strictly between the two fences is non-empty and parses as a YAML
   **mapping** (at least one `key: value` pair) — i.e. it is shaped like
   frontmatter, not free prose.

When any condition fails, behavior is unchanged: the document is treated as
having no frontmatter (a genuine leading `----` thematic break is left intact).

### New error variant

Add to `MarkdownError` (darkmatter):

```rust
/// The document opens with a near-miss frontmatter fence (e.g. `----`)
/// wrapping YAML-shaped content. Frontmatter fences must be exactly `---`.
FrontmatterFenceMismatch {
    ctx: SourceContext,
    found: String,   // the offending fence, e.g. "----"
}
```

Message guidance: name the offending fence and the fix, e.g.
*"frontmatter fence must be exactly `---`, found `----` on line 1"*.

### Error rendering reuses existing machinery

No new rendering work in claudine. Claudine's compose error walker already wraps
any frontmatter-rooted composition error with `composition::FrontmatterExcerpt`
(`CompositionError::enrich_frontmatter` → `WithFrontmatter`), emitting the
authored frontmatter as a syntax-highlighted, line-numbered `CodeBlock` with the
offending line highlighted (TTY-gated; stripped at `ColorDepth::None`). The new
`FrontmatterFenceMismatch` must:

- map to a `CompositionError` frontmatter-rooted variant so `enrich_frontmatter`
  picks it up, and
- expose the fence line (line 1) as the highlight target.

The user then sees the exact two-dash-too-many fence highlighted in their own
file, with the one-line fix.

### Surface scope

Detection lives in darkmatter `parse_frontmatter`, so it is shared by every
consumer. The conservative heuristic (matched dash-only fences wrapping a YAML
mapping) makes false positives on display/render of arbitrary markdown
negligible, so a hard error repo-wide is acceptable and keeps the fix in one
place. If a future consumer needs to render such a document tolerantly, that is a
separate, explicit opt-out — not a reason to weaken the compose-time guard.

## Acceptance criteria

1. Composing a file whose frontmatter is fenced with `----` (or `-----`, etc.)
   wrapping YAML-mapping content fails with `FrontmatterFenceMismatch` and
   renders the highlighted frontmatter excerpt — it never ships the YAML to the
   provider.
2. The same applies symmetrically to `inline-compose` and `sequence` (each step
   surfaces the error; `sequence` aggregates per-step as it already does).
3. A document that legitimately opens with a `----` thematic break followed by
   prose (no matched closing dash fence, or non-YAML content between) renders
   unchanged — no error.
4. A correctly-fenced `---` document is unaffected (regression guard).
5. `prompts/cross-platform.md`, once fixed to `---`, composes with the
   frontmatter stripped and only `# Ensuring Cross Platform Support …` reaching
   the Agent Prompt.

## Test plan

- **darkmatter unit** (`frontmatter.rs`): `----`/`-----` open+close around a YAML
  map → `FrontmatterFenceMismatch`; leading `----` thematic break + prose → no
  frontmatter, no error; matched `----` fences around non-mapping scalar content
  → no error (treated as body); correct `---` → parses (existing tests stay
  green).
- **claudine** (compose handler): the fence-mismatch error renders the
  `FrontmatterExcerpt` code block with line 1 highlighted, TTY-gated and stripped
  at `ColorDepth::None`.
- **L2** (optional): real-terminal capture proving the highlighted excerpt and
  the absence of YAML in the Agent Prompt section.

## Out of scope

- Accepting `----` as a valid fence. The convention stays exactly `---`.
- Auto-correcting the user's file during composition.
- TOML (`+++`) or other front-matter dialects.
