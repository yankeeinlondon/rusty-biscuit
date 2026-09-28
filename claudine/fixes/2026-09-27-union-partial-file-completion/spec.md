---
created: 2026-09-27
status: draft-spec
clarified: false
reviewed: false
review_iterations: 0
implemented: false
$schema:
    status: |-
        enum(
            draft-spec,
            finalized-spec,
            planned,
            implemented,
            review-findings,
            human-in-the-loop,
            completed,
            on-hold,
            abandoned
        ) -> an indicator of progress for this specification
    reviewed: boolean -> indicates whether the specification file has been reviewed by another agent from the one which created the spec
    reviewed_by: string -> the agent and model used in the spec review
    reviewed_on: date -> the date the spec was reviewed
    review_iterations: number -> the number of implementation reviews have taken place in the review/fix cycle
    clarified: boolean -> indicates whether the specification was built -- _in part_ -- with the 'clarify.md' prompt
    implemented: boolean -> indicates whether this spec's plan has been implemented
    implemented_by: string -> the agent who implemented the plan
area: claudine
packages:
    - claudine
    - claudine-cli
    - biscuit-terminal
related:
    - 2026-06-30-completion-failures
human_review: false
message_to_agent: |-
    Phase 4 is complete. `just test` in claudine/ is green (7567 passed, 9 skipped). `just test` and `just lint` are green in biscuit-terminal/ and darkmatter/, `just lint` in claudine/ is clean, and nothing is stranded.

    - Excerpts are focused. `FrontmatterExcerpt` holds regions, renders one `CodeBlock` per region with `with_start_line`, and joins them with `⋮` placed in the next block's gutter column. `EXCERPT_CONTEXT_LINES = 3`. `BlockOnly` is gone.
    - Deviation from N3, recorded in the log: Claudine keeps its own `locate_property_line` for semantic paths (sequence indexes, re-rooted `tasks[0].…` suffixes), which `YamlKeyPath` cannot express. `biscuit-terminal` gained `SourceContext::focused_line_regions(lines, context)`, which shares window, ancestor, merge, and unsafe-YAML logic with `focused_yaml_regions` through one private `select_regions`. The locator also got a real bug fix: the keys of a `- key:` item after the first are now siblings, not children.
    - Anchor rule (S4's open question): an anchored block is shown when the selection holds every line between the delimiters, and is omitted otherwise. That keeps the L1 PTY and L2 inline-compose mismatch excerpts.
    - New highlight variants: `Properties` (mismatch: `prompt` + `sequence`) and `SchemaProperties` (schema problems: the frontmatter key plus the `$schema` declaration in every union arm). Darkmatter gained `ShellExpansionError::origin()`.
    - Docs already done in Phase 4, so Phase 5 only needs to review them: `claudine/docs/topics/composition.md` § "Frontmatter YAML blocks in errors" (rewritten, with a Mermaid flow), `biscuit-terminal/README.md` (`focused_line_regions`), and the claudine skill's "Composition diagnostics" row.
    - The changed-expectations table has Phase 4 rows. Phase 5 must still run the full `just test-l2`: only the tmux excerpt captures (8/8) ran here, and the WezTerm variants did not.
    - Still open from Phase 3 (unchanged): the Darkmatter root-union relative-caller-path defect, and the pre-existing Windows `shipped_implement_plan_prepares_with_unset_optional_commit_message` failure. Surface both to the author in the hand-off.
    - Known cosmetic limits (not changed): each region repeats `CodeBlock`'s `yaml` label, and at `ColorDepth::None` the `CodeBlock` is a plain fence without line numbers (as before).
---

# Partial file completion is skipped for union schemas, and the failure arrives late and unreadable

## Outcome

`claudine compose prompts/clarify.md spec=fix` offers the `fixes/*` specs
that match `fix` and composes with the one the user picks. The file chooser
comes before any other question. Every interactive prompt, including the
provider picker, opens as an inline partial-screen prompt below the cursor,
the way `fzf` does. When an input really cannot be resolved, the error
appears before the user is asked anything. Its frontmatter excerpt shows only
the lines involved plus a few lines around them, never the whole block.

## Report

```sh
claudine compose prompts/clarify.md spec=fix
```

1. No `--<provider>` flag was given, so the provider picker opened. It took
   over the whole screen instead of opening inline below the cursor.
2. After the user chose a provider, the run failed:

   ```text
   CompositionError: schema validation
   ┃ Schema validation failed for …/prompts/clarify.md.
   ┃ /spec: no existing file matched reference `fix` while resolving from `…/prompts`
   ┃ Problems:
   ┃ - `/spec`
   ```

   The error was followed by all 29 lines of `clarify.md`'s frontmatter,
   with no line highlighted.

`fix` should not have been an error. It is a partial, and
`2026-06-30-completion-failures` added exactly this completion: filter the
property's `match(**/*spec*.md)` candidates by the substring and let the user
choose. That completion never ran.

The relevant frontmatter of `prompts/clarify.md`:

```yaml
$schema:
    - spec: file(required; match(**/*spec*.md); eager) -> …
      doc: file
    - design: file(required; match(**/*design*.md)) -> …
      doc: file
doc: "{{spec || design}}"
initialize:
    stack: …
```

## Reproduction (2026-09-27, `feat/schema-enhancement` debug build)

These runs used a stub `claude` on `PATH`, `--claude`, and a PTY. Each
variant is a minimal prompt with `spec=fix`:

| `$schema` shape | Sibling frontmatter | Chooser offered? |
|---|---|---|
| single `spec: file(required; match; eager)` | none | yes |
| single, plus `doc: file` | `doc: "{{spec}}"` | yes |
| union `spec` / `design` arms | none | yes |
| union, each arm with `doc: file` | none | yes |
| union, each arm with `doc: file` | `doc: "nope.md"` (literal) | yes |
| union, each arm with `doc: string` | `doc: "{{spec \|\| design}}"` | yes |
| union, each arm with `doc: file` | `doc: "{{spec}}"` | **no** |
| union, each arm with `doc: file` | `doc: "{{ \"nope.md\" }}"` | **no** |

The chooser is skipped only for a **union** schema when a **`file`-typed**
property's frontmatter value is a **template**. `--dry-run` shows none of
this, because it stops before schema validation.

## Cause

### C1. Union arm selection validates uncomposed frontmatter (primary bug)

Before any other prompt, `resolve_supplied_file_inputs`
(`cli/src/commands/compose/prep.rs:186`) calls `unresolved_supplied_files`
(`lib/src/composition/schema/supplied.rs`). For a root union,
`supplied_file_shape` (`supplied.rs:119`) must first pick the single arm the
caller's inputs apply to. It does this by validating each arm against
`build_effective_instance`: the **raw** frontmatter overlaid with the
caller's overrides (`supplied.rs:166`). It relaxes `eager` only on the
caller-supplied file properties.

In `clarify.md`, `doc` still holds the literal string `"{{spec || design}}"`.
Darkmatter correctly rejects that string as a `file`, so both arms fail. No
arm is selected, `supplied_file_shape` returns `None`, and the pass reports
nothing pending.

The main pre-validator does not have this problem. It is
composition-tolerant: it drops `Type`/`Invalid` verdicts for any value where
`value_needs_composition` is true (`lib/src/composition/schema/mod.rs`,
around line 607). The arm selector never adopted that rule.

The residual fallback cannot rescue this either.
`classify_unresolved_file_reference` returns `None` for every union
(`lib/src/composition/schema/classify.rs:220`), so the late failure
becomes the generic `SchemaValidation`.

Darkmatter is behaving correctly here. The defect is that Claudine asks it to
judge frontmatter that has not been composed yet.

### C2. The failure is deferred past the provider picker

`clarify.md` authors an `initialize` stack. Under
`staged_boot::authors_initialize` (`prep.rs:194`, `:470`), the full schema
verdict is withheld until after `initialize`, and `initialize` runs after
provider selection. The early supplied-file pass is the only check that runs
before the picker. Because of C1 it passed silently, so the first failure
surfaced only after the user had answered the picker.

### C3. The late error names the wrong base directory

The value `fix` came from the caller, who launched from the repository root.
The late verdict still reports `while resolving from …/prompts`, which is the
document's directory. The early pass reports the caller's origin
(`record.origin().base_dir()`). The late Darkmatter validation resolves the
unresolved caller value in document context instead, so the message sends
the user to look in the wrong place.

### C4. The provider picker is full-screen

`prompt_one_shot_provider` calls `run_standalone(ChooseOne::new(), state,
None)` (`cli/src/commands/wrap/selection_ui.rs:40`). `None` means alternate
screen, full screen. Every schema prompt in `schema_interactive` passes
`inline_height(n)` instead (`cli/src/commands/schema_interactive/mod.rs:604`).

### C5. The frontmatter excerpt always shows the whole block

`FrontmatterExcerpt` (`lib/src/composition/frontmatter_excerpt.rs`) captures
the entire frontmatter block and at most adds a highlight line. It has no
windowing. For a single-problem `SchemaValidation`, the highlight property is
`pointer_to_dotted("/spec")` = `spec` (`lib/src/composition/error/mod.rs:3318`).
`spec` is a caller input, not a top-level frontmatter key, so nothing is
located and the whole 29-line block renders with nothing highlighted.
Multi-problem validation, `MissingProperties` with several entries,
`FrontmatterParse`, `ComposeFailed`, and `ShellExpansionFailed` map to
`BlockOnly`, which also renders the whole block.

`biscuit-terminal` already has the needed machinery:
`SourceContext::focused_yaml_excerpt` (key-focused, with ancestors and
elision) and `SourceContext::excerpt_prose` (a line window). Claudine's
excerpt uses neither. Note that `focused_yaml_excerpt` itself falls back to
the whole frontmatter when no key resolves.

## Requirements

### R1. Union arm selection is composition-tolerant

When `supplied_file_shape` decides whether an arm applies, it ignores
`Type`/`Invalid` verdicts on properties whose raw value needs composition.
It uses the same `value_needs_composition` rule the pre-validator already
applies, from one shared helper rather than a copy. `Missing` verdicts and
verdicts on literal values keep deciding arm applicability exactly as they
do today.

With this in place, `clarify.md` + `spec=fix` selects the `spec` arm and
reaches the existing glob+substring chooser before any other prompt.

### R2. Decidable input failures come before any interactive question

Any failure that depends only on the caller's inputs and the static document
must be reported before the provider picker or any other prompt opens. This
includes a caller-supplied eager `file(match)` value that resolves to no
file and that the chooser cannot complete (zero candidates, declined,
cancelled, or non-interactive). Documents that author `initialize` are
included: `initialize` can repair or route values, but it cannot make a
caller's literal `fix` exist.

For the union case, when arm selection still cannot pick a unique arm, the
existence check still runs early when every arm that declares the property
types it as an eager `file`. The failure does not depend on which arm wins.
In that case the chooser searches the de-duplicated union of those arms'
`match` patterns (ruling **D1**). The chosen file is then a literal path, so
ordinary union validation settles which arm applies. A path that fails an
arm's `match` rules that arm out, and no further question is asked.

### R3. Interactive prompts run in a fixed order

For `compose` and `inline-compose`, prompts run in this order: supplied-file
completion (R1), then required-missing collection where that runs
pre-picker, then the provider picker. The provider picker never opens while
an input that can be checked early is still unchecked.

### R4. Unresolved caller values name the caller's base directory

Every surface that reports an unresolved caller-supplied file reference
names the directory the caller's value was resolved from: the record's
origin, which is the launch CWD for a CLI `key=value`. The document's
directory is named only for values the document supplied. The late verdict
must not report `…/prompts` for `spec=fix` typed at the repository root.

### R5. The provider picker is an inline partial-screen prompt

`prompt_one_shot_provider` runs inline below the cursor with a bounded
height, sized to the option count plus chrome and capped like the
`schema_interactive` choosers. It never switches to the alternate screen.
After submit or cancel, the terminal keeps the prior scrollback, and no
blank full-screen residue is left behind.

The sequence review table (`review_sequence`) is out of scope (see
Non-goals).

### R6. Frontmatter excerpts are focused, never the whole block

A frontmatter excerpt attached to a Claudine error shows only the lines
involved, each with **3 lines of context** above and below. The 3 is a
named constant with no configuration surface (ruling **D2**). It uses real
source line numbers, marks elision between non-adjacent regions, and
highlights the offending lines. It never renders the full frontmatter block.
The one exception is a block whose focused window already covers every line
of it.

- **Locating a caller-input property.** When a schema problem names a
  property that is not a top-level frontmatter key, such as `/spec` here, the
  excerpt locates that property's declaration inside `$schema`. For union
  schemas that means inside every arm, so `clarify.md` highlights line 5.
  If a top-level frontmatter key of the same name exists, it is shown too.
- **Multiple problems.** Show the union of each problem's focused region.
- **Nothing locatable.** Omit the excerpt entirely. The textual diagnostic
  already names the path, and a whole-block dump adds noise, not
  information. This replaces every `FrontmatterHighlight::BlockOnly` path.
  A `FrontmatterParse` error with no usable location also omits the excerpt.
- **Reuse, don't re-implement.** Build on `biscuit-terminal`'s
  `SourceContext` locator (`locate_key_region` / focused excerpt). Where it
  needs a non-falling-back variant or union-arm (`- key:`) support, add that
  in `biscuit-terminal`. Keep today's TTY/`NO_COLOR` privacy gating and
  syntax highlighting. If the chosen renderer cannot start line numbers at
  an offset, add that capability to the renderer rather than faking numbers
  in the text.

### R7. Verification

Order matters: reproduce first, then fix.

1. **Reproduce C1 (L1 PTY).** Add a case next to
   `cli/tests/l1/level1_provided_partial_file_pty.rs`. Use a union schema
   whose arms both declare `doc: file`, with `doc: "{{spec || design}}"` in
   frontmatter and an `initialize` stack. Pass `spec=<substring>` with an
   explicit provider stub. On today's code the chooser does not appear and
   `no existing file matched reference` is printed. Record that failure in
   the plan.
2. **Fixed:** the same test reaches `Use this file? (Y/n)` (single match)
   or the chooser (several matches), then launches the stub with the
   resolved path.
3. **Ordering (R2/R3):** without a provider flag and with zero candidates,
   the error is printed and the provider picker never renders. Assert on
   the absence of the picker's title text in the PTY capture. With one
   candidate, the file dialog renders before the picker.
4. **D1 (L1 PTY):** use the two-arm `features`/`fixes` shape from D1 with
   `spec=<substring>` matching one file in each tree. The chooser lists both.
   Picking the `fixes` file launches the stub with that path, and the
   composed frontmatter validates against the `fixes` arm.
5. **Lib unit:** `supplied_file_shape` selects the right arm when a sibling
   `file` property holds `{{…}}` or `$(…)`. It still returns `None` for
   genuinely conflicting discriminants and for string-only alternatives. The
   existing tests in `supplied.rs` keep passing unchanged.
6. **R4:** an L1 test asserts the late verdict's message names the caller's
   launch directory, not the document directory.
7. **R5:** an L1 PTY test asserts the picker runs without entering the
   alternate screen (no `\x1b[?1049h` in the capture).
8. **R6:** unit tests on the excerpt:
   - a mid-file property yields at most 7 lines around it, plus ancestors;
   - `/spec` against `clarify.md` highlights the `$schema` arm-0 `spec` line;
   - an unlocatable property yields no excerpt;
   - two problems yield two regions with an elision marker;
   - line numbers match the source file.

   Update any snapshot or L1 assertion that currently expects the
   whole-block excerpt, and give each changed expectation a reason in the
   plan.

## Non-goals

- The sequence review screen (`review_sequence`), which is a full-screen
  editable table by design.
- Partial completion for **non-eager** `file(match)` properties (such as
  `clarify.md`'s `design` arm). `unresolved_supplied_files` deliberately
  skips them, because a non-eager file may name an output that does not
  exist yet. Whether `design=foo` should get completion is a separate
  question.
- Changing Darkmatter's `file` validation of template strings. It is
  correct to reject them. Claudine must stop asking before composition.

## Rulings

Decided by the author on 2026-09-27.

- **D1 (was OQ1).** Several arms declare the same eager `file` property with
  different `match` patterns, and no unique arm applies. The chooser searches
  the de-duplicated union of those arms' patterns. The file the user picks
  decides which arm applies. Rejected alternative: failing early without a
  chooser.

  Example: arms
  `spec: file(required; eager; match(**/features/**/spec.md))` and
  `spec: file(required; eager; match(**/fixes/**/spec.md))`, discriminated
  by an optional `kind`. With `spec=cli`, the chooser lists matching specs
  from both trees. Picking `fixes/2026-07-13-cli-switches/spec.md` fails the
  first arm's `match`, which leaves the second arm. R7 adds an L1 test for
  this shape.
- **D2 (was OQ2).** The excerpt context width is a named constant of 3 lines,
  with no env var or config key.
