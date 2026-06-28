---
status: ready for planning and implementation
reviewed: true
created: 2026-06-26
review_iterations: 2
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

## Reader's note: design decision

This review keeps the original design goal but makes the integration contract
more explicit: malformed near-miss frontmatter is rejected in **Darkmatter**,
while **Claudine** classifies and enriches the resulting typed error at the
same render boundary used by existing frontmatter failures. This avoids adding a
Claudine-only pre-scan that could drift from Darkmatter's parser and still keeps
Claudine's prompt-execution surfaces from sending authored YAML to a provider.

The fix intentionally does **not** accept `----` as an alternate fence. Treating
it as valid would be easier for this one file but would broaden the frontmatter
grammar in a way other Markdown tools do not recognize, making documents less
portable. The user-facing repair remains one character: change each fence to
`---`.

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
3. The content strictly between the two fences is non-empty, parses as a YAML
   **mapping**, and that mapping has at least one key. A scalar, sequence, empty
   map, or parser failure is treated as body text, not frontmatter.

When any condition fails, behavior is unchanged: the document is treated as
having no frontmatter (a genuine leading `----` thematic break is left intact).

Implementation notes:

- Prefer `serde_yaml_ng::from_str::<serde_yaml_ng::Value>` for this probe, then
  check for a non-empty mapping. Do not reuse `parse_yaml_with_fallbacks` for
  detection, because the fallback path protects Darkmatter interpolation and
  shell syntax inside already-valid `---` frontmatter; near-miss detection
  should stay a conservative shape check.
- Preserve the current `---` behavior, including the existing "missing closing
  delimiter means body text" behavior. This fix only handles a matched
  near-miss fence pair.
- Match the closing fence to the exact opening dash run. `----` opened by
  `-----` should not be normalized into a fence pair.

### New error variant

Add to `MarkdownError` (darkmatter):

```rust
/// The document opens with a near-miss frontmatter fence (e.g. `----`)
/// wrapping YAML-shaped content. Frontmatter fences must be exactly `---`.
FrontmatterFenceMismatch {
    ctx: SourceContext,
    found: String, // the offending fence, e.g. "----"
    line: usize,   // document-absolute line number; currently always 1
}
```

Message guidance: name the offending fence and the fix, e.g.
*"frontmatter fence must be exactly `---`, found `----` on line 1"*.

Darkmatter rendering must add a `BlockError` branch for this variant, parallel
to `FrontmatterParse`, with:

- header: `MarkdownError` / `frontmatter fence mismatch`;
- body: source path when available plus the offending fence and line;
- hint: `Use exactly three dashes (---) for Markdown frontmatter fences.`;
- excerpt: highlight line 1 using the existing source-excerpt style. The
  excerpt should use the full document context (`SourceContext.content`), not
  only the YAML body, because the offending token is the delimiter itself.

### Claudine error mapping and excerpt enrichment

Claudine's compose error walker already appends an authored-frontmatter excerpt
for errors classified as frontmatter-rooted
(`CompositionError::enrich_frontmatter` → `WithFrontmatter`), emitting a
syntax-highlighted, line-numbered `CodeBlock` with an offending line highlighted
(TTY-gated; stripped at `ColorDepth::None`). The new
`FrontmatterFenceMismatch` must wire into that existing path:

- `claudine/lib/src/composition/resolve.rs::map_load_error` must map
  `MarkdownError::FrontmatterFenceMismatch { .. }` to
  `CompositionError::FrontmatterParse(err)` or to a new equivalently
  frontmatter-rooted `CompositionError` variant. Reusing `FrontmatterParse` is
  acceptable because the user-facing category is still "the authored
  frontmatter block cannot be accepted."
- `claudine/lib/src/composition/prepare.rs::map_compose_error` must preserve the
  typed `MarkdownError` through `ComposeFailed` for compose-time failures that
  originate in transcluded or reloaded Markdown.
- `CompositionError::frontmatter_block_spec` must highlight the fence line for
  this error. `Some(None)` is not enough for this case because it renders the
  block without pinpointing the bad delimiter. If the existing
  `FrontmatterExcerpt::capture` only supports key-path lookup, extend it with a
  line-target capture helper rather than inventing a separate renderer.

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
6. The same malformed-fence document loaded through `Markdown::try_from(path)`
   and through `Markdown::try_from_content(content)` returns the typed mismatch
   error; the infallible `From<String>` behavior is not used by Claudine's
   prompt-loading path and does not mask this failure.
7. Non-TTY output still reports the typed error and actionable hint, but does
   not emit ANSI styling or the TTY-only frontmatter appendix unless the
   existing `FORCE_COLOR=1` behavior requests it.

## Test plan

- **darkmatter unit** (`frontmatter.rs`): `----`/`-----` open+close around a YAML
  map → `FrontmatterFenceMismatch`; leading `----` thematic break + prose → no
  frontmatter, no error; matched `----` fences around non-mapping scalar content
  → no error (treated as body); correct `---` → parses (existing tests stay
  green).
- **darkmatter error rendering** (`MarkdownError` block tests): the mismatch
  block names the offending fence, suggests `---`, includes the source path when
  present, and highlights document line 1 without relying on a YAML parser
  location.
- **claudine** (compose handler): the fence-mismatch error renders the
  `FrontmatterExcerpt` code block with line 1 highlighted, TTY-gated and stripped
  at `ColorDepth::None`.
- **claudine resolve mapping**: `resolve_composition_source` maps
  `MarkdownError::FrontmatterFenceMismatch` to a frontmatter-rooted
  `CompositionError`, not a flat `MarkdownLoad` string.
- **L2** (optional): real-terminal capture proving the highlighted excerpt and
  the absence of YAML in the Agent Prompt section.

## Open Questions

### Should near-miss detection cover other frontmatter dialects?

This spec only targets dash-only YAML fences because Claudine/Darkmatter
frontmatter is YAML and the live bug is a `----` typo. There is a broader class
of near misses (`++++`, `;;;`, or mismatched `---`/`----`) that could also look
like frontmatter to an author, but broadening detection risks false positives
for prose or examples.

Suggested solutions:

1. **Dash-only YAML near miss only (recommended).**
   Pros: directly fixes the observed provider-leak bug; aligns with the
   current YAML-only frontmatter contract; keeps false-positive risk low.
   Cons: other malformed dialect-looking blocks continue to render as body
   text.
2. **All repeated punctuation fences when the inner content parses as a mapping.**
   Pros: catches more authoring mistakes before they reach a provider.
   Cons: invents a broader quasi-frontmatter detector than the parser itself,
   and may reject legitimate Markdown examples or notes that happen to be
   wrapped in punctuation.
3. **Provider-command-only guard in Claudine.**
   Pros: limits the hard error to the dangerous surfaces that send prompts to
   agents.
   Cons: duplicates frontmatter recognition outside Darkmatter, leaves other
   Darkmatter consumers with silent behavior, and is easier to drift.

Recommendation: implement option 1. It addresses the actual failure mode with
the smallest grammar change and keeps the parser as the single source of truth.

## Out of scope

- Accepting `----` as a valid fence. The convention stays exactly `---`.
- Auto-correcting the user's file during composition.
- TOML (`+++`) or other front-matter dialects.
- Rejecting arbitrary leading YAML-looking prose that is not enclosed by a
  matched near-miss dash fence.
