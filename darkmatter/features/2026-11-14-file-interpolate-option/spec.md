---
kind: feature
created: 2026-11-14
status: human-in-the-loop
area: darkmatter
owner: Ken Snyder <ken@ken.net>
related:
    - 2026-11-14-edit-action
    - 2026-11-14-absent-equals-null
depends-on:
    - 2026-11-14-lifecycle-mutation-root
next_spec: 2026-11-14-concise-message-clamp
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
reviewed: true
reviewed_by: codex/gpt-6.1-sol
reviewed_on: 2026-11-14
review_iterations: 6
completed: true
needs_rulings: false
required_rulings:
implemented: true
implemented_by: "claude/opus"
---

# `::file interpolate=false`: transclude prose without evaluating it

## Why this exists

Caller skills (`{package-area}/docs/use-skills/`) are composed when they are
published, so they can transclude passages of the package's canonical
`docs/` instead of restating them. Those docs pages are written to be read
as they are, so they show expressions raw, for example
``Use `{{ name }}` to interpolate.``. Transcluded, that example is
evaluated: against the skill's own frontmatter if a name matches (the line
above became ``Use `s` to interpolate.`` in a test), otherwise to an
undeclared-name warning. Fenced code and `::code` content already stay
verbatim; inline prose does not, and `::file` has no way to say "bring this
passage in as written".

## Author rulings (2026-11-14)

- **One option, `interpolate: boolean(default(true))`.** Written on the
  directive as `interpolate=false`; absent (or `null`, per
  `2026-11-14-absent-equals-null`) means `true`, today's behavior.
- **Its scope is the transcluded document's prose only.** The child's
  frontmatter is always interpolated; there is no option to stop that.

```md
::file ../../topics/lifecycle/index.md interpolate=false
```

### Editor behavior (author ruling, 2026-11-14)

Each file is analyzed on its own terms. An included file's expressions keep
their normal highlighting, completion, and diagnostics when that file is
open, because the setting belongs to one inclusion, not to the file. The
editor explains the option at the parent's `::file` line: it completes and
validates `interpolate=`, and hover on `interpolate=false` reads:

> interpolation in this file will not be interpolated but instead kept as
> literal text

This replaces the draft's requirement to show the included file's
expressions as inactive. An inclusion-specific "view as included by…" mode
is out of scope.

## Review decisions

These details resolve the draft's runtime ambiguities while keeping the
author's prose-only scope. The editor presentation is settled by "Editor
behavior" above.

### Option values and placement

*(Confirmed by the author, 2026-11-14: the value table, case-insensitive trimmed boolean words, and last-valid-wins with every occurrence validated. Planner note: the directive value reader drops quoting today, and the `null` / `"null"` distinction needs it.)*

`interpolate` is an option of `::file`, not a frontmatter property or a
new command-line flag. It applies to the child, so the including document's
own prose continues to interpolate normally.

| Directive spelling | Local value before inheritance |
|---|---|
| Option omitted, or `interpolate=null` | `true` |
| `interpolate=true`, `yes`, `on`, or `1` | `true` |
| `interpolate=false`, `no`, `off`, or `0` | `false` |
| A quoted boolean word, such as `interpolate="false"` | The same converted boolean |
| Empty value, an object, an array, or any other text | Located directive error |

Boolean words follow the existing conversion rules of SimplifiedSchema, the
repository's shared type and conversion library, including case-insensitive
matching and trimming. Directive values are text tokens:
`0` here is the accepted boolean word, not a new number-to-boolean
conversion. Only an unquoted `null` is the null value; `"null"` is text and
fails boolean conversion. Reuse the shared conversion and default rules
rather than introducing another boolean parser or applying truthiness.
The missing/null default depends on `2026-11-14-absent-equals-null`, reached
through the existing `2026-11-14-lifecycle-mutation-root` dependency.

Repeated `interpolate=` assignments follow existing scalar directive options:
the last valid assignment wins. Every occurrence must be valid; a later
value does not hide an earlier invalid one. A missing `=` or missing value
is malformed syntax, not an omitted option. Validation errors name the
including file and the authored option position, using existing directive
error handling and blocking-policy behavior.

**`::file` and `::url` accept the option; `::code` does not (author ruling,
2026-11-14).** `::url` includes Markdown prose exactly as `::file` does (both
reach `resolve_url_target`), so `::url https://example.com/guide.md
interpolate=false` and `::file https://example.com/guide.md interpolate=false`
behave identically, under the existing network permissions. `::code`
content is already literal, so the option is not offered there and its
existing handling of unknown options is unchanged. This feature does
not add supported file types or remote access permissions.

### Which text remains literal

*(Confirmed by the author, 2026-11-14: triple braces stay triple; the option overrides the child's `interpolate_code_blocks: true`.)*

Here, **prose** means the child's Markdown body outside executable directive
inputs and command bodies. It includes headings, paragraphs, lists, table
cells, inline code, and link labels and destinations. With an effective
`interpolate=false`:

- Body `{{ … }}` spans are neither parsed as expressions nor evaluated.
  They produce no expression diagnostics, variable lookups, file reads, or
  runtime-context requests. This includes malformed expression examples.
- Body `{{{ … }}}` interpolation literals keep all three braces, rather than
  reducing to two. No escaping or brace rewriting is added to the output.
- Fenced and indented code remain uninterpolated even when the child sets
  `interpolate_code_blocks: true`. That setting cannot override this option.
- The child's frontmatter still follows its normal interpolation, schema,
  coercion, and shell-expansion rules. The option introduces no frontmatter
  opt-out and does not turn on operations the caller disabled.

For example, including this child with `interpolate=false` preserves the
three examples, even if the caller has a `name` property:

```md
Use `{{ name }}` to read a property.
An unknown name looks like {{ undeclared }}.
Write {{{ x }}} for an interpolation literal.
```

**Reader's note:** "as written" concerns interpolation syntax, not identical
file bytes. Text replacement, section exclusion, link resolution, heading
adjustment, quotation/disclosure wrapping, and cleanup retain their existing
behavior. An explicit replacement can therefore change an example, and a
relative link can change spelling. The option does not make a document safe
to execute: its directives and frontmatter can still perform approved effects.

### Directives still execute

*(Confirmed by the author, 2026-11-14, including that the including directive's own inputs remain the parent's.)*

Inputs to executable directives keep their normal evaluation. This includes
`when=` conditions, expressions in file targets and option values, collection
sources and callbacks in `::map`/`::filter`, and expressions in `::shell`
arguments and `::shell-block` command bodies. Existing escaping, code-region
rules, single-pass evaluation, shell approval, and failure handling still
apply. Prose inside a selected or repeated block stays literal.

For example, inside an included child:

```md
::block when="show_example"
Example: {{ name }}
::file {{ detail_file }} set.name="{{ name }}"
::end-block
```

The condition and the nested directive's target and option input evaluate as
usual; `Example: {{ name }}` remains literal. The nested file's frontmatter
sees the supplied value, while its prose inherits the disabled setting.
The including directive's own `set=`, `replace=`, `exclude=`, `when=`, and
wrapper inputs likewise remain inputs of the parent.

### Inheritance is confined to the inclusion branch

*(Confirmed by the author, 2026-11-14: a nested `interpolate=true` under a disabled ancestor is accepted without a warning; prologue/epilogue includes inherit with no option of their own.)*

Once prose interpolation is disabled, all Markdown descendants of that
inclusion inherit it. This includes nested `::file` and `::url` includes,
frontmatter prologue/epilogue includes, and nested composition requested by
an active expression. A nested `::file interpolate=true`, an omitted option,
or `interpolate=null` cannot turn it back on. An explicit `true` under a
disabled ancestor is accepted without a warning: it is a valid local value,
and a warning would unnecessarily break publication with `--deny all`.

Conceptually, a child's prose may interpolate only when its ancestor allows
it, its own option allows it, and the caller enabled body interpolation.
Frontmatter and directive inputs are separate from that prose decision.

The setting does not escape to the parent or siblings. The same target can
be included twice, once normally and once with interpolation disabled, with
independent results. No file is permanently marked uninterpolated, and no
new public option for frontmatter prologue/epilogue references is introduced.

The literal output is protected for this composition only. Saving it and
composing it again as a new root uses normal rules; the serialized Markdown
contains no hidden setting that would preserve the former inclusion mode.

## Implementation requirements

The implementation must preserve one interpretation across every path that
examines a child. Merely skipping the entire body interpolation stage is
insufficient, because that stage also handles executable directive inputs.

- Establish the child's effective prose setting before passive call checks,
  context-demand discovery, remote-expression discovery, shell-approval
  discovery, or body evaluation inspect it. Suppressed prose must not cause
  prefetches or rejection of sample calls under `--deny all`. Active
  frontmatter and directive inputs keep their checks and diagnostics.
- Share the classification of inactive prose and active inputs between
  execution, preflight walks (which discover commands before execution), and
  passive reference-graph preparation. Discovery must approve exactly the
  command bytes execution will use; it
  must not execute a sample expression to discover more commands or files.
  Passive validation and editor analysis continue to perform no composition
  effects. A separately opened child is judged under its own invocation.
- Carry the setting through child-request derivation and every recursive
  composition path. Preserve the existing request context and source spans.
  Use the existing origin tracking to keep produced text from becoming new
  executable syntax; do not rewrite examples into sentinel text or rescan
  the assembled parent output.
- Include the effective setting in composed-child cache identity and
  reference-graph preparation identity, and include the option in directive
  parameter identity. Two uses of one file must not share prepared content,
  warnings, or composed results across different effective settings. Raw
  file and HTTP-body caches can still be shared.
- Parsing, passive validation, composition, and `md compose --shell`
  discovery recognize the option. DMLS, Darkmatter's language server, offers
  the option and `true`/`false` value completions, explains defaulting and
  inheritance in hover, and
  reports malformed values at the option. An open included file keeps its
  normal expression treatment (see "Editor behavior").

## Documentation and caller-skill adoption

Implementation updates the current documentation alongside the behavior:

- `darkmatter/docs/transclusion/block-transclusion.md`, "Options and
  Conditionals": the value table, literal-prose example, inheritance,
  active directive inputs, frontmatter behavior, and fresh-composition limit.
- `darkmatter/docs/inline/interpolation.md`: explain this alternative to
  triple-brace escaping when reusing documentation, and its precedence over
  `interpolate_code_blocks`.
- The `darkmatter` repo skill's composition/expression guidance and the
  canonical caller skill under `darkmatter/docs/use-skills/use-darkmatter/`:
  describe the feature for their respective audiences. Do not edit an
  installed caller skill directly.
- `CLAUDE.md`'s Skill Audiences rule: replace the planned workaround with
  guidance to use `interpolate=false` when including documentation whose
  expression examples must survive publication. Keep the separate escaping
  guidance for expressions authored directly in the caller skill.
- Review existing caller-skill transclusions and add the option where they
  include literal documentation examples; leave intentional template
  evaluation enabled. Check the published text, including names that happen
  to match the caller skill's frontmatter, because `--deny all` alone cannot
  catch a successful but unwanted substitution. Keep that publication guard.

Use `just publish-skills` from affected areas after implementation to publish
the updated canonical sources. No new CLI command, dependency, or README
reference section is needed; the option reference belongs in `docs/`.

## Verification

Use L1 tests (unit and in-process behavior checks) for the parser, library,
CLI, and DMLS through the package-area `just` recipes and nextest. No real
terminal, focused window, or browser is needed.
Include a passive shipped-artifact check and a normal `md compose` invocation
against a frozen caller-skill fixture; do not read internal `prompts/` in CI.

| Case | Required result |
|---|---|
| Default, explicit true, explicit null, boolean word aliases and quoted words | Shared coercion/default behavior; default output matches today's behavior |
| False with known and unknown names, an invalid call, malformed expression text, and triple braces | Literal prose survives; no expression warning or failure, including under `--deny all` |
| False with `interpolate_code_blocks: true` | Code examples also stay literal |
| Child frontmatter interpolation and a `set` override under false | Normal frontmatter resolution, validation, and supplied-value precedence |
| Conditional and collection blocks under false | Conditions/callbacks execute; selected/repeated prose stays literal |
| Expressions in nested file targets, option values, and approved shell inputs under false | Inputs evaluate normally; shell discovery and execution agree |
| False ancestor with nested omitted, false, true, and null options | Descendant prose stays literal; explicit true adds no warning |
| Prologue/epilogue, permitted remote Markdown, and nested composition | Inherit false; existing permission, cycle, depth, and failure rules hold |
| Same child included normally and under false, in both directive orders and with caching enabled/disabled | Independent correct output and warnings; parent and sibling interpolation continue normally |
| Inactive prose containing a read-side function or `ctx.*` sample | No read, remote fetch, context demand, or command approval caused by that sample |
| Replacement, exclusion, link handling, and wrappers under false | Existing transformations apply without activating preserved expression syntax |
| Invalid value, quoted `"null"`, missing value, malformed assignment, and repeated assignments | Located errors and documented scalar-option precedence; library and editor agree |
| Published caller-skill fixture whose own `name` collides with a docs example | `{{ name }}` and `{{{ x }}}` survive unchanged through the ordinary publication compose arguments |

Exercise reference-graph preparation and shell preflight alongside direct
composition so a passing runtime test cannot hide an earlier evaluation of
inactive prose. Check locations in nested files after block expansion as well
as on the including directive. Editor tests follow "Editor behavior": the
hover text on `interpolate=false`, and an open child used by two parents
with different settings keeps its normal expression treatment.

This is a selective evaluation change, not a new expensive operation. Reuse
existing caching and work-count checks where applicable; no performance
spike or benchmark expansion is required.

## Open Questions

None.
