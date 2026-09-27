---
status: draft — awaiting CLI ergonomics decisions
created: 2026-07-22
area: darkmatter
packages:
  - darkmatter
  - darkmatter-cli
  - dmls
amends:
  - ../../features/_completed/2026-06-19-cleanup-fixed-line-length/spec.md
  - ../2026-07-13-fixed-width-lists/spec.md
---

# Cleanup Context Drift and Compose Line-Width Controls

## Summary

Darkmatter's cleanup pipeline can indent unrelated top-level blocks by four spaces when a later
list item contains an additional paragraph. In Markdown, four leading spaces turn ordinary prose
and headings into indented code. This changes document structure, causes `md compose` to emit
visibly corrupted Markdown, and makes `md compose ... | md` render the affected region as a code
block rather than as prose and a heading.

The same investigation exposed two related gaps in the shipped line-width contract:

1. collapsing a soft break after one trailing source space can concatenate words; and
2. the library supports preserving incidental newlines and reflowing composed output to a fixed
   width, but `md compose` exposes neither control. Only `md clean` currently exposes
   `--ignore-incidental-newlines` and `--fixed-width`.

This is a shared cleanup correctness fix plus missing compose CLI wiring. It is not a terminal
renderer defect and it is not an expression-function defect.

Follow-up investigation also establishes that the two existing `md clean` flags are not wholly
unwired. They work on isolated prose, but the shared structural defect can mask or defeat their
observable effect on realistic mixed documents. The specification therefore treats end-to-end
effectiveness—not flag presence, parsing, or completion—as the contract.

## Reported Fixture

The reproducer is:

```text
darkmatter/example-docs/safe-expressions/test.md
```

The file deliberately combines:

- top-level wrapped prose;
- an H2;
- conditional `::block` regions;
- `ctx.dirty_files` interpolation;
- an ordered list;
- indented expression placeholders that become additional list-item paragraphs; and
- expression results that include plain text, unordered Markdown lists, and ordered Markdown
  lists.

The safe functions (`length`, `as_line_separated`, `as_space_separated`,
`as_unordered_list`, and `as_ordered_list`) evaluate. They provide the data shape that exposes the
cleanup problem but are not the source of the structural corruption and MUST NOT be changed as
part of this fix.

## Reproduction

From the repository root:

```sh
md compose darkmatter/example-docs/safe-expressions/test.md
```

The beginning of the current output is:

```md
# Safe Expressions

    Darkmatter provides a set of operators and functions which are deemed to be _safe_ (aka, no side effects) which can be used to gather usefulinformation or to mutate a document's Frontmatter.

    ## Example 1: changing a CSV list into other forms
```

The four spaces are not present in the source. They make the paragraph and H2 parse as an indented
code block. Piping the result into the terminal renderer exposes, but does not cause, the problem:

```sh
md compose darkmatter/example-docs/safe-expressions/test.md | md
```

The renderer correctly presents the corrupted leading region as code. The fix MUST correct the
composed Markdown before rendering; it MUST NOT add a terminal-rendering exception for this
fixture.

The defect also reproduces without composition or expressions:

```sh
printf '# T\n\nTop prose.\n\n## H2\n\n1. one\n\n    second paragraph\n' | md clean -
```

Current output:

```md
# T

    Top prose.

    ## H2

1. one

    second paragraph
```

Required output:

```md
# T

Top prose.

## H2

1. one

    second paragraph
```

This minimal case establishes that cleanup is the failing shared layer.

## Confirmed Failure Mechanism

`cleanup_content_internal` parses the source, captures list metadata, serializes the event stream,
and then runs string-level list normalization. The relevant path is:

```text
cleanup_content_internal
  -> extract_additional_paragraph_contexts
  -> pulldown_cmark_to_cmark serialization
  -> normalize_list_spacing / blockquote and marker restoration
  -> fix_list_indentation
```

`extract_additional_paragraph_contexts` records `AdditionalParagraphContext` values for second and
later paragraphs owned by list items. Each context currently carries list depth, blockquote depth,
the authored body column, and the parent item's content column. It does not carry an identity that
ties the context to the corresponding line or block in serialized output.

`fix_list_indentation` walks serialized lines from the beginning. Its additional-paragraph branch
currently consumes the next context when all of the following are true:

- the current line follows a blank line;
- the current line was not recognized as a list marker; and
- the queued context has the same blockquote depth.

Those conditions are insufficient. A top-level paragraph or heading earlier in the document also
matches them. It can consume metadata belonging to a future list paragraph and receive the future
paragraph's continuation indentation. In the minimal fixture, `Top prose.` and `## H2` are
misidentified this way and gain four spaces.

The implementation MUST establish positional ownership between each parser-derived additional
paragraph context and its serialized block. Depth and "follows blank" alone MUST NOT be used as
identity. The exact representation is an implementation decision; a second Markdown parser pass or
a broad new formatter abstraction is not required by this specification.

## Incidental-Newline Join Corruption

The reported source contains this ordinary soft break:

```md
... gather useful 
information or ...
```

One trailing space does not create a CommonMark hard break. Default cleanup should remove the
incidental newline while retaining a single word separator. The current full-cleanup event-stream
path emits `usefulinformation`.

`separator_at_boundary` / `join_separator` inspect the source and may choose no replacement
separator because the source line already ends in whitespace. However, the parser event supplied
to `pulldown_cmark_to_cmark` does not necessarily retain that incidental trailing space. Replacing
`Event::SoftBreak` with empty text can therefore remove both the newline and the only separator.

Required result:

```md
... gather useful information or ...
```

This requirement applies to the full event-stream cleanup path as well as the direct
`strip_incidental_newlines` transform. Existing Unicode-script joining, zero-width-space behavior,
hard-break preservation, and structural-boundary rules remain authoritative. The fix MUST NOT
blindly insert spaces at every soft break.

## Existing `md clean` Flags: Wired but Defeated by the Shared Defect

`md clean` already exposes both line-width controls in Clap and dynamic shell completion:

```text
--ignore-incidental-newlines
--fixed-width <#>
```

The command path is wired:

```text
Command::Clean
  -> run_subcommand
  -> CleanOptions
  -> run_clean
  -> apply_cleanup
      -> apply_cleanup_no_strip                       (preserve mode)
      -> Markdown::cleanup* -> reflow_to_width       (fixed-width mode)
```

Minimal top-level prose demonstrates that each option has an effect:

```sh
printf 'Alpha beta\ngamma delta\n' | md clean -
# Alpha beta gamma delta

printf 'Alpha beta\ngamma delta\n' | md clean --ignore-incidental-newlines -
# Alpha beta
# gamma delta

printf 'Alpha beta gamma delta epsilon\n' | md clean --fixed-width 12 -
# Alpha beta
# gamma delta
# epsilon
```

The reported mixed document exposes why the switches can appear to do nothing:

- `--ignore-incidental-newlines` selects the preserving cleanup entry points and does retain
  several authored soft breaks, but those entry points still run `fix_list_indentation`. The
  introduction and H2 are therefore still consumed as future list paragraphs and indented as
  code. Preserve mode controls soft-break collapse; it cannot compensate for unrelated structural
  corruption later in the shared cleanup sequence.
- `--fixed-width` runs only after full cleanup. By then, the introductory paragraph has already
  gained four spaces and reparses as protected indented code. The remaining long prose in the
  reported source is inside complete Darkmatter directive bodies that `md clean` intentionally
  protects. Consequently, `--fixed-width 40` and `--fixed-width 80` currently produce
  byte-identical output for the reported file because no eligible prose remains for reflow.

The same fixed-width no-op is reproducible without expressions or directives:

```md
# T

This top level paragraph contains enough ordinary Latin words to require visibly different wrapping at forty columns and eighty columns.

1. item

    short second paragraph
```

Both widths currently emit the top-level paragraph as one four-space-indented line. The
additional list paragraph triggers context drift; the resulting indented-code classification then
protects the top-level paragraph from width reflow.

The completion listing in the issue report is dynamic shell **completion**, not shell expansion.
It confirms that Clap advertises the arguments and their help strings. Completion metadata does
not execute `run_clean` and is not evidence that either option has an observable end-to-end effect.

### Required `md clean` behavior

After the shared structural fix:

- `--ignore-incidental-newlines` MUST preserve eligible authored soft breaks even when top-level
  blocks precede a list with additional paragraphs; it MUST NOT cause or retain structural drift.
- `--fixed-width N` MUST visibly reflow every eligible prose block to `N` display columns even
  when a later list has additional paragraphs; no eligible block may become protected merely
  because cleanup misassigned list metadata.
- widths that require different legal wrap decisions (for example, `40` and `80`) MUST produce
  observably different boundaries for a sufficiently long eligible fixture.
- the flags' existing conflict, value range, completion values, stdin/file/`--save` behavior, and
  meaning remain unchanged.
- preserve mode remains neutral only with respect to incidental soft breaks. It does not disable
  unrelated canonical cleanup such as marker restoration, list spacing, or trailing-newline
  normalization.

No separate CLI workaround is required. Repairing positional ownership in shared cleanup should
make the existing `md clean` wiring effective; any production edit in `commands/clean.rs` MUST be
justified by a failing CLI-specific test rather than assumed necessary.

## Missing `md compose` Controls

The library already has the required state and builders:

- `IncidentalNewlineMode::{Strip, Preserve}`;
- `ComposeOptions::with_incidental_newline_mode`;
- `ComposeOptions::with_fixed_width`; and
- the inline-post cleanup dispatch in `Markdown::run_inline_post_operation`.

The `md clean` CLI already establishes the public flag names and semantics:

```text
--ignore-incidental-newlines
--fixed-width <#>
```

`Command::Compose`, `run_subcommand`, and `run_compose` do not currently accept or wire these
values. Consequently:

- `md compose --help` does not advertise them;
- generated shell completions cannot advertise the missing compose flags;
- users cannot preserve source soft breaks while diagnosing composition; and
- users cannot request canonical fixed-width composed Markdown even though the compose library
  supports it.

### Required compose CLI contract

`md compose` MUST expose the same two options as `md clean`:

```sh
md compose document.md --ignore-incidental-newlines
md compose document.md --fixed-width 80
```

The contract is:

- neither flag supplied: preserve the current default, `Strip` with no fixed width;
- `--ignore-incidental-newlines`: set `IncidentalNewlineMode::Preserve` for the compose cleanup
  stage;
- `--fixed-width N`: strip incidental soft breaks first, then reflow logical prose blocks to `N`
  display columns through the existing compose option;
- the two flags conflict at argument parsing time;
- `N` uses the existing `parse_fixed_width` range (`1..=1000`);
- `--fixed-width` uses the existing common-width value completer (`40`, `60`, `80`, `100`,
  `120`); and
- Clap help and generated shell completions expose the flags for the `compose` subcommand.

There is no new environment-variable or frontmatter override. The CLI MUST reuse the established
library behavior rather than post-process the composed output in a separate cleanup pass.

## Required Behavior

### Structural ownership

For every cleanup mode and list-spacing mode:

1. A parser-derived context for an additional list-item paragraph MUST be consumed only by that
   paragraph's serialized representation.
2. Top-level paragraphs, ATX and Setext headings, blockquotes, directives, HTML, tables, code
   blocks, thematic breaks, and link-reference definitions MUST NOT consume a later list context.
3. The additional paragraph MUST remain owned by its original list item after cleanup.
4. Ordered, unordered, task, nested, and blockquoted lists MUST retain their existing marker,
   indentation, and container semantics.
5. Cleanup MUST be structurally idempotent: a second identical cleanup pass produces the same
   bytes and the same parsed block/list structure.

### Mode matrix

| Surface | Default | Preserve source soft breaks | Fixed width |
|---|---|---|---|
| Cleanup library | Existing `Strip` behavior | Existing preserving entry points | Existing fixed-width entry points |
| Compose library | `IncidentalNewlineMode::Strip` | `with_incidental_newline_mode(Preserve)` | `with_fixed_width(N)` |
| `md clean` | Existing behavior | Existing `--ignore-incidental-newlines` | Existing `--fixed-width N` |
| `md compose` | Existing default | Add `--ignore-incidental-newlines` | Add `--fixed-width N` |
| DMLS formatting | Existing configured cleanup behavior | Preserve current configuration mapping | Preserve current fixed-width mapping |

The structural fix and safe join behavior MUST be identical across equivalent modes on every
surface. The compose CLI addition does not authorize divergence from `md clean` or direct library
use.

## Scope

### Primary production symbols

The implementation is expected to involve or explicitly verify these symbols:

| Symbol | File | Responsibility |
|---|---|---|
| `cleanup_content_internal` | `darkmatter/lib/src/markdown/cleanup/mod.rs` | Shared event-stream cleanup and ordered string-pass orchestration |
| `AdditionalParagraphContext` | `darkmatter/lib/src/markdown/cleanup/lists.rs` | Metadata used to restore ownership of later list-item paragraphs |
| `extract_additional_paragraph_contexts` | `darkmatter/lib/src/markdown/cleanup/lists.rs` | Captures parser-derived additional-paragraph ownership |
| `fix_list_indentation` | `darkmatter/lib/src/markdown/cleanup/lists.rs` | Applies list depth/column metadata to serialized lines; current structural failure site |
| `collapse_incidental_soft_break_events` | `darkmatter/lib/src/markdown/cleanup/reflow.rs` | Replaces eligible soft-break events during full cleanup |
| `SoftBreakModel`, `separator_at_boundary` | `darkmatter/lib/src/markdown/cleanup/reflow/semantic.rs` | Source-aware eligibility and join-separator decisions |
| `ComposeOptions::with_incidental_newline_mode` | `darkmatter/lib/src/markdown/compose/context/options.rs` | Existing compose preserve-mode API |
| `ComposeOptions::with_fixed_width` | `darkmatter/lib/src/markdown/compose/context/options.rs` | Existing compose fixed-width API |
| `Markdown::run_inline_post_operation` | `darkmatter/lib/src/markdown/compose/pipeline/phases.rs` | Existing compose cleanup dispatch; should remain the single application point |
| `CleanOptions` | `darkmatter/cli/src/commands/clean.rs` | Existing clean-mode option carrier; already receives both values |
| `run_clean` | `darkmatter/cli/src/commands/clean.rs` | Existing clean input/repair/output orchestrator |
| `apply_cleanup` | `darkmatter/cli/src/commands/clean.rs` | Existing strip/fixed-width lowering; fixed-width reflow follows full cleanup |
| `apply_cleanup_no_strip` | `darkmatter/cli/src/commands/clean.rs` | Existing preserve-mode lowering into shared cleanup |
| `Command::Clean` | `darkmatter/cli/src/args/command.rs` | Existing Clap declarations and completion metadata; behavior reference for compose |
| `Command::Compose` | `darkmatter/cli/src/args/command.rs` | Add the two compose flags, conflict, parser, and value completion metadata |
| `run_subcommand` | `darkmatter/cli/src/commands/mod.rs` | Forward parsed compose values |
| `run_compose` | `darkmatter/cli/src/commands/compose.rs` | Lower CLI values into the existing `ComposeOptions` builders |

`darkmatter/cli/src/commands/clean.rs::apply_cleanup` is the existing CLI behavior reference. It
should not be duplicated into the compose output path.

### Test files

Expected test scope includes:

- `darkmatter/lib/src/markdown/cleanup/tests/lists.rs` and/or `reflow.rs` for minimal fail-first
  ownership, exact-output, parsed-structure, mode-matrix, and idempotence tests;
- `darkmatter/lib/src/markdown/compose/tests/rendering.rs` for compose-option parity and a fixture
  containing top-level blocks before a list with additional paragraphs;
- `darkmatter/cli/src/args/cli.rs` for compose flag parsing, conflict, range, and help metadata;
- `darkmatter/cli/src/args/completion.rs` for fixed-width value completion reuse where needed;
- `darkmatter/cli/tests/compose_basic.rs` or a focused compose integration test for spawned CLI
  stdout behavior; and
- DMLS formatting tests for byte parity with the repaired cleanup behavior.

Existing `darkmatter/cli/tests/clean.rs` coverage proves each option on isolated prose and proves
list behavior when the list starts the fixture. It does not cover top-level blocks followed by a
loose list with an additional paragraph. Add fail-first spawned-CLI cases that:

- compare default and preserve modes on the same mixed fixture;
- compare at least two fixed widths on sufficiently long eligible top-level prose before the
  triggering list;
- assert exact output and parsed structure rather than only `contains(...)` or maximum line width;
- exercise stdin, file output, and `--save`; and
- prove that an option advertised by dynamic completion reaches observable command output.

The reported example document SHOULD remain as a human-readable regression fixture, but automated
tests MUST also include a deterministic context-free minimal case. Tests MUST NOT depend on the
worktree having a particular set of dirty files.

### Documentation and completion surfaces

Update these surfaces when behavior is implemented:

- `darkmatter/docs/cli/compose.md`;
- the compose section of `darkmatter/cli/README.md`;
- `darkmatter/docs/darkmatter-compose-pipeline.md` and
  `darkmatter/docs/inline/cleaning.md` if their CLI surface descriptions are incomplete; and
- `.claude/skills/darkmatter/SKILL.md` / `compose.md`, whose current text documents the library
  compose controls but describes the two flags only for `md clean`.

Shell completions are Clap-derived. The acceptance requirement is that generated completions for
supported shells contain both compose flags and the common `--fixed-width` values; no manually
maintained completion script should be introduced.

### CLI ergonomics: open design discussion

The completion capture also exposes a separate usability problem: even `md clean` presents a tall,
flat option list, while broader commands such as `md compose` expose substantially more switches
than a terminal completion window can scan comfortably. The flags are organized primarily by the
implementation shape of one Clap variant rather than by a small user-facing hierarchy.

This draft records the concern but deliberately does not choose the refactor before the requested
brainstorming discussion. Candidate design dimensions include:

- which controls are common document-transformation policy versus command-specific behavior;
- whether related advanced controls belong under nested subcommands, named profiles/presets, or
  reusable flattened argument groups;
- whether completion should progressively disclose common versus advanced switches;
- how to preserve scripting stability and discoverability while the codebase has no established
  external user base; and
- whether `clean` and `compose` should share one CLI-neutral cleanup-policy model so equivalent
  flags cannot drift in parsing, help, completion, or lowering.

The spec MUST remain `draft` until those decisions are discussed and the chosen boundary is
recorded. A broad CLI reorganization is not implicitly authorized by the cleanup correctness fix;
the final spec must say whether the refactor is in this fix, a prerequisite, or a separately
tracked feature.

### Package and downstream scope

`sniff repo packages` identifies the affected package-area crates as `darkmatter`,
`darkmatter-cli`, and `dmls`. `sniff repo package-dependencies` also identifies direct consumers of
the public `darkmatter` crate, including `claudine`, `claudine-cli`, `claudine-gen`,
`biscuit-icon-cli`, `biscuit-speaks-cli`, `playa-cli`, `research`, and `sniff-cli`.

No public type or enum change is required. Downstream source changes are not expected, but the
implementation plan MUST use fresh GitNexus impact analysis and Sniff discovery to decide which
direct consumers need targeted verification after the changed symbols are known.

## Risk and Impact Evidence

GitNexus was queried before writing this specification. Its Darkmatter worktree index was 102
commits behind `HEAD`; an attempted refresh failed in the installed runner before analysis because
`pino` called an unavailable `diagnostics_channel.tracingChannel`. The following counts are useful
directional evidence, not a substitute for fresh pre-implementation analysis:

- `cleanup_content_internal`: **CRITICAL**, 9 direct / 172 total upstream symbols through depth
  three, six modules;
- `fix_list_indentation`: **CRITICAL**, 2 direct / 155 total, five modules;
- `collapse_incidental_soft_break_events`: **HIGH**, 1 direct / 154 total, four modules;
- `run_compose`: **LOW**, 1 direct / 3 total, two CLI execution-flow families; and
- `run_subcommand`: **LOW**, 1 direct / 2 total.

The follow-up clean-path query reports:

- `run_clean`: **LOW**, 2 direct / 3 total upstream symbols and two CLI execution-flow families;
- `apply_cleanup`: **LOW**, 1 direct / 4 total and the same two CLI flow families; and
- `apply_cleanup_no_strip`: **HIGH**, 1 direct / 4 total across the CLI and shared Markdown
  cleanup modules.

Before editing any production symbol, the implementer MUST refresh the GitNexus index successfully,
rerun upstream impact analysis for that exact symbol, and warn before proceeding if the result is
HIGH or CRITICAL. Before completion, run `detect_changes(scope: "compare", base_ref: "main")` and
confirm the affected flows match the recorded scope.

## Acceptance Criteria

1. The minimal top-level-prose/H2/loose-list fixture cleans to the required exact Markdown without
   indenting either top-level block.
2. `darkmatter/example-docs/safe-expressions/test.md` composes without adding leading indentation
   to its introductory prose or H2.
3. The composed output reparses with the introduction as a paragraph and the example title as an
   H2, not as an indented code block.
4. `md compose ... | md` renders the introduction as prose and the example title as a heading; no
   renderer-specific workaround is used.
5. A later additional paragraph remains inside its original list item across normal, compact, and
   loose list-spacing modes.
6. Criterion 5 holds in default strip mode, preserve mode, and fixed-width mode, including nested,
   task, ordered, unordered, and blockquoted representative cases.
7. Cleanup is byte-idempotent and preserves a parser-derived structural fingerprint for every new
   regression fixture.
8. A soft break after exactly one trailing source space joins Latin words with one `U+0020`, never
   zero spaces or multiple spaces, in both the direct strip transform and full cleanup.
9. Existing hard breaks, Unicode-script joins, zero-width-space joins, structural boundaries,
   protected blocks, list-marker restoration, and indentation-width behavior continue to pass.
10. On the mixed top-level-plus-loose-list fixture, `md clean --ignore-incidental-newlines`
    preserves eligible source soft breaks without indenting or reclassifying any top-level block.
11. On a sufficiently long mixed fixture, `md clean --fixed-width 40` and
    `md clean --fixed-width 80` produce their respective legal wrap boundaries rather than
    byte-identical no-op output; every eligible line respects its requested display width except
    the established atomic-token overflow case.
12. Existing `md clean` parsing, conflict, completion values, stdin/file/`--save` behavior, and
    isolated-prose behavior remain intact.
13. `md compose --ignore-incidental-newlines` parses, appears in help/completions, and produces the
    same cleanup result as compose library use with `IncidentalNewlineMode::Preserve`.
14. `md compose --fixed-width N` parses, appears in help/completions, accepts `1..=1000`, suggests
    the established common widths, and produces the same result as compose library use with
    `with_fixed_width(N)`.
15. Supplying both compose flags fails in Clap with a conflict before input composition begins.
16. Default `md compose` remains strip mode with no fixed width.
17. Equivalent library, `md clean`, `md compose`, and DMLS formatting sequences remain
    byte-equivalent for their shared cleanup configuration.
18. CLI integration tests cover both stdin and file-backed composition without depending on host
    line endings; LF and CRLF inputs remain supported on macOS, Linux, and Windows.
19. Public docs and the Darkmatter skill no longer imply that the line-width flags are available
    only to `md clean`.
20. The final specification records the agreed CLI-ergonomics boundary and moves out of draft
    status before implementation planning begins.

## Required Verification

Verification scope is the Darkmatter package area plus downstream consumers identified by fresh
impact analysis. At minimum, from `darkmatter/`:

```sh
just build
just test
just lint
```

Run `just test-l2` because the acceptance contract includes spawned CLI behavior and the
compose-to-terminal pipeline. Use focused Nextest selectors during iteration, but the final area
gates must cover `darkmatter`, `darkmatter-cli`, and `dmls`. Do not use a workspace-wide Cargo gate
as a substitute for impact analysis, and do not run `cargo fmt` in write mode.

## Non-goals

This fix does not:

- change expression-function evaluation, aliases, or result formats;
- make the terminal renderer reinterpret valid indented code as prose;
- change the default from incidental-newline stripping to preservation;
- add frontmatter or environment configuration for line width;
- rename either established CLI flag;
- change the accepted fixed-width range or display-column measurement;
- redefine CommonMark soft breaks, hard breaks, or indented code;
- replace the explicit cleanup pass pipeline with a generic pass framework;
- add a second post-compose cleanup path in the CLI;
- silently treat shell-completion advertisement as behavioral verification;
- perform a broad CLI hierarchy refactor before the open design decisions are recorded; or
- perform unrelated formatting or list refactoring.

## Completion Definition

The fix is complete when the structural corruption and word concatenation have fail-first
regressions that pass at the shared library layer, compose exposes and documents both established
line-width controls, every equivalent surface agrees byte-for-byte, fresh impact/diff analysis
matches the declared scope, and the scoped build, test, L2, and lint gates pass.
