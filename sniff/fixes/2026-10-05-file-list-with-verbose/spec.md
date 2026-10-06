---
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
area: sniff
status: finalized-spec
created: 2026-10-05
implemented: false
packages:
    - sniff-cli
reviewed: true
reviewed_by: codex/gpt-6.1-sol
reviewed_on: 2026-10-05
review_iterations: 0
clarified: true
clarified_by: codex/gpt-6.1-sol
needs_rulings: false
review_note: the clarification process served as a review
human_review: false
message_to_agent: |-
    Phase 3 (command wiring, root resolution, integration tests) is
    complete; see
    sniff/fixes/2026-10-05-file-list-with-verbose/implementation-log.md
    under "## Phase 3", including a session-recovery note: the phase began
    from an interrupted prior pass whose implementation was verified
    task-by-task, with two stray comment edits reverted. Essentials for
    Phase 4 (docs, lint, hand-off): (1) The feature works end to end —
    `sniff files --association <cat> -v` renders the list; root resolution
    lives in `resolve_files_link_root` in sniff/cli/src/commands/mod.rs and
    flows through `output::render_text`'s `link_root` parameter into
    `render_files_section`; no JSON, sniff-library, or biscuit-terminal
    code changed. (2) Phase 4 owns plan 4.1-4.5: the docs sweep
    (sniff/cli/README.md, sniff/docs/cli/files.md with a root-selection
    Mermaid flow for a repo-newcomer audience, sniff/docs/
    sniff-library-architecture.md only if it names the verbose behavior,
    .claude/skills/sniff/cli.md — note the skill moved to
    .opencode/skill/sniff/cli.md in this worktree layout; check both), the
    drift grep (render_files_section, "identical verbose", files doc
    comments), final `just lint` + both clippy gates, cross-OS statement
    per the os skill, and the final verification sweep including the
    stdout/stderr split check and the git-diff scope check. (3) Gates at
    Phase 3 close, all green: `just test` 3190/3190 (32 routine skips),
    `just lint`, clippy -D warnings for sniff and sniff-cli; cross-rig
    windows 12/12 + 6/6 and linux 13/13 cli::files_ tests (linux needed
    the native path via `--features test-fixtures` because the standing
    clone's target/release is kache-poisoned — documented in the os
    skill). (4) No human-review items were raised in Phases 2 or 3; the
    Phase 2 ruling-5 notation precision (C1 scalars as \u{XXXX} on Unix)
    remains flagged for the author's review cycle only.
---

# File Lists for Filtered Verbose Association Reports

## Problem

`sniff files --association image -v` shows the same count-and-percentage
table as `sniff files --association image`. It does not identify the files
that contributed to the count. Users must switch to JSON and extract paths
to answer the natural follow-up: which files matched?

The `sniff` library already exposes the matching paths through the `files`
field of [FileAssociationStats](../../lib/src/filesystem/file_types/model.rs),
which holds each category's count, percentage, and paths. JSON already
serializes those paths. This is a CLI presentation defect, not missing
discovery capability.

This review assumes the existing package/base scan scope is intentional. The
fix succeeds when filtered verbose text identifies the captured matches
without changing discovery or JSON, and each supported hyperlink points to
the file represented by its label.

## Required Behavior

When an association filter is supplied and verbosity is greater than zero,
the text report must show every captured matching file after the existing
summary table and any incomplete-scan notice. This applies to every supported
association category, including `unknown`.

| Invocation | Required text output |
| --- | --- |
| `sniff files` | Existing summary table |
| `sniff files -v` | Existing summary table and language/framework details |
| `sniff files --association image` | Existing filtered summary table |
| `sniff files --association image -v` | Filtered summary table followed by matching paths |
| `sniff files --association programming-language -v` | Filtered table, matching paths, and existing language details |
| `sniff files --association framework-file -v` | Filtered table, matching paths, and existing framework details |

For example, a filtered verbose image report with three matches includes:

```text
Association    Count
Image          3 (25.0%)

Files:
  - assets/banner.png
  - assets/logo.svg
  - screenshots/example.png
```

The example illustrates content and order; existing table styling remains
authoritative. Render the file section as an unordered list using
`biscuit-terminal` components. Preserve directory segments so identical
basenames remain distinguishable. Use the existing file-path styling and
OSC8 hyperlink conventions, with links resolving to the actual files. OSC8
is the terminal escape sequence that makes a label clickable; terminals
without hyperlink support must still show the label. `--plain` must show the
same paths as readable text without color or hyperlink escape sequences.

All new report content, including the file list and the incomplete-scan
notice, belongs on stdout because it describes the observed files. CLI
guidance and errors belong on stderr. Do not add a verbosity hint or other
diagnostic to JSON stdout.

### Path order, labels, and targets

- Sort by the original `PathBuf` values in ascending order before formatting,
  using Rust's path ordering. Do not sort by styled strings, basenames, or
  locale, and do not change case. Determinism means repeatable ordering of
  the same captured paths on the same platform; it does not promise identical
  ordering across operating systems.
- Display the paths supplied by the filtered statistics, preserving directory
  segments and their relative basis. Do not rebase labels onto the caller's
  directory. Use the reversible filename-label policy below in both rich and
  plain text; native separator spelling may differ across operating systems.
- Resolve relative hyperlink targets against the root used to classify these
  paths. For the focused `files` request, this is the owning package root
  when a package is found, otherwise the effective base directory. An
  absolute path remains absolute. Resolve a relative `--base` against the
  invocation directory before constructing targets.
- Obtain that context from the already captured library result and the
  effective base. The `sniff` library's
  [RepoInfo::package_for_dir](../../lib/src/filesystem/repo/types.rs) selects
  the owning package; the CLI must use that authority rather than inspect
  manifests or implement package ownership itself. Pass the resolved root
  explicitly to the renderer, without adding a JSON field.
- Treat names as literal text, including spaces, readable Unicode,
  underscores, brackets, ampersands, and characters resembling formatting
  tags. Keep native paths for sorting, joining, and hyperlink targets;
  never reconstruct a target from an escaped label. Delegate
  file-URL rendering to existing terminal support where its contract fits,
  ensuring valid drive and network-share URLs on Windows.
- Rendering must not probe or canonicalize every listed file. A file removed
  after discovery remains listed, and does not turn a successful scan into
  an error. Reuse already resolved root context.

For example, if a scan of `/work/project/lib` returns `assets/logo.svg`,
the label stays `assets/logo.svg` and the link targets
`/work/project/lib/assets/logo.svg`, even when the command was launched from
`/work/project/lib/src` or outside the repository with `--base`.

### Reversible filename labels

This policy applies only to the new filtered verbose text list. Discovery,
stored native paths, JSON, and other reports retain their existing behavior.

- Preserve readable Unicode and directory segments. Render control characters
  visibly so a filename cannot introduce terminal commands or extra report
  lines. Newline, carriage return, tab, and escape appear as `\n`, `\r`, `\t`,
  and `\x1B`, respectively; other control characters also require visible,
  reversible escapes.
- Escape literal backslashes as `\\`, including native Windows directory
  separators. For example, the native Windows path `assets\logo.svg` appears
  as `assets\\logo.svg`. A literal backslash followed by `n` appears as `\\n`,
  distinct from the `\n` label for an actual newline. Consequently, even an
  otherwise ordinary Unicode filename containing a backslash has an escaped
  display label in this list.
- Preserve invalid Unix filename bytes using `\xNN` escapes instead of
  replacement characters. Preserve non-Unicode Windows native code units
  using a distinct reversible notation that cannot be confused with byte
  escapes or readable Unicode. This is a renderer contract for native path
  values, not a claim that arbitrary code units are legal on-disk filenames.
- Distinct native filename spellings must remain distinguishable in labels.
  Use the same label spelling in rich and plain output.
- Neutralize filename controls before escaping the label for the
  `biscuit-terminal` components
  [Prose](../../../biscuit-terminal/lib/src/components/prose/prose.rs) or
  [InlineProse](../../../biscuit-terminal/lib/src/components/prose/inline_prose.rs),
  which interpret text styling. In `biscuit-terminal`, `Prose::escape_text`
  preserves terminal escape sequences, so markup escaping alone does not
  satisfy this requirement. Protect hyperlink attributes separately so a
  quote or URL delimiter cannot change the destination.

### Hyperlink failures

If one entry cannot be represented by a faithful absolute file URL, show its
escaped label without a hyperlink, emit no diagnostic for that entry, and
retain a successful report. A non-Unicode native path does not inherently
require this fallback: retain the hyperlink when its native target can be
represented faithfully. Terminal capability fallback follows the existing
text-only behavior.

When preparing the new filtered verbose text list, if the correct root context
for relative paths cannot be established, report an error on stderr and exit
nonzero. Do not guess a root or emit links against the invocation directory as
a substitute. This preparation must not add root-resolution failures to JSON,
unfiltered, or non-verbose reports. A file removed after discovery remains
listed; do not add existence probes to decide whether to link it.

> **Reader's note:** A scan root and a path's relative basis are not always
> interchangeable. The library can also project a package from a wider
> repository inventory without rebasing its stored paths. This fix retains
> the focused package/base request; using the Git root blindly would produce
> incorrect links for package-relative paths. Also, the `sniff-cli`
> [shared path formatter](../../cli/src/output/filesystem/path_format.rs)
> currently escapes only underscores and splits directories on `/`. Reuse
> it only after meeting the literal-label and native-path requirements.
> Keep any helper change limited to what this report needs and verify existing
> callers if a shared helper changes.

The file section precedes any existing verbose language/framework details.
Higher verbosity levels show each matching path once. Global `-v` placement
must remain position-independent. These rules apply equally to rich text and
`--plain`. Aggregate `sniff filesystem` and full-system reports do not gain
this new section.

## Empty and Incomplete Results

- With no matching files, retain the existing empty summary behavior and
  omit the `Files:` section. An empty successful observation exits 0;
  filtered JSON retains its existing zero-count shape. Do not adopt the
  exit-1-on-empty convention used by some repository path-list commands.
- If the inventory is truncated, retain the incomplete-scan notice and list
  only the matching files captured in that inventory. Do not present the
  list as a complete enumeration of the directory. The notice remains visible
  even when the filtered category has no captured matches.
- Counts and percentages retain their existing meaning. Verbosity changes
  presentation only; it must not change acquisition, scan scope, file counts,
  percentage denominators, or classification limits.

Filtered JSON's `total_files` continues to count matching files, while each
category's percentage continues to use all classified files in the original
scan as its denominator. For a capped scan, both describe the captured sample.

## Scope and Existing Design Constraints

- The change must work on macOS, Linux, native Windows, and WSL2.
- Reuse `FileAssociationStats::files`; do not start another filesystem walk,
  reclassify files, or rescan to populate the list.
- Keep classification, scan scope, and package ownership in the `sniff`
  library. The `sniff-cli` change only selects presentation, carries existing
  context, orders labels, and renders library observations. Preserve its
  identity-only Git and structure-only repository request, with docs and
  formatting disabled; verbosity must not widen that request.
- Reuse existing terminal rendering components and path-list helpers where
  their path-root and escaping contracts fit. If root context must reach the
  renderer, carry it explicitly without changing public JSON output.
- Keep JSON output unchanged, including when `-v` is supplied.
- Preserve `--perf` behavior: JSON stdout remains one valid JSON document;
  the optional performance report follows its existing stream routing. Do
  not compare timing values for equality across invocations.
- Unfiltered verbose reports do not gain file lists. This fix adds no
  `--list`, `--csv`, or additional filter flags.

### Authorized implementation preparation

Future implementation may create small disposable local fixtures and make
narrowly scoped changes to shared `sniff-cli` presentation helpers, with checks
of their affected callers. This authorization does not extend to new crates,
test frameworks, or benchmarks. If investigation establishes a defect in
`biscuit-terminal`, document the evidence and obtain a scope decision before
changing that dependency package. The clarification work authorizes updates
to this specification; it does not authorize implementing the fix.

## Acceptance Criteria

1. A filtered verbose report lists exactly the captured paths returned by the
   same filtered JSON report on an unchanged, complete fixture whose names
   the existing JSON representation supports, once each and in native sorted
   order. Compare paths through the reversible label representation. Existing
   JSON limitations for non-Unicode names do not need to change; validate
   those native values directly through text rendering. Verify images and
   another association, plus preservation of language/framework details.
   Exercise the category-independent renderer
   with `unknown` as well; no separate large fixture is needed.
2. Unfiltered reports and filtered non-verbose reports retain their existing
   output. JSON is identical with and without verbosity on a complete fixture
   when performance collection is disabled. With `--json --perf -v`, stdout
   parses as one document and contains no file-list heading or terminal bytes.
3. `-v` before and after the subcommand, and `-vv`, produce the required list
   without duplication. Plain output contains no ANSI or OSC sequences.
4. Small fixtures cover duplicate basenames in different directories, spaces,
   Unicode, and markup-sensitive filename characters valid on the test OS.
   Cover invocation from a package subdirectory, a base outside any package,
   and relative/absolute `--base` from outside the target repository.
   Compare decoded hyperlink destinations with the complete expected native
   absolute file identity, allowing the consistent Windows prefix and
   separator conversions required by file URLs, including verbatim drive and
   network-share spellings. Basename, suffix, or URL-prefix checks alone do
   not establish identity. Normalize fixture/root aliases once if needed to
   align test expectations; do not add per-file probes or canonicalization.
   Exercise capability fallback and plain output without requiring a particular
   bullet glyph. Keep escaped labels literal through the list's final render.
5. Empty and truncated observations follow the rules above. Assertions for
   truncated results use one constructed, captured breakdown shared by text
   and JSON projections, rather than compare independent capped CLI runs.
   Reuse existing cap tests; do not create another large tree just to verify
   this renderer. Assert the incomplete notice appears before the list and
   survives an empty filter.
6. Work counters on a small complete fixture show the same acquisition work
   with and without verbosity. Compare scan/classification, manifest parsing,
   Git discovery/status, and docs counters, treating absent counters as zero;
   ignore timings and presentation-only counters. Rendering captured results
   performs no additional acquisition. No timing benchmark or performance
   spike is required for this display-only change.
7. Direct renderer tests cover controls and non-Unicode native values without
   requiring illegal on-disk names. Verify newline, carriage return, tab, and
   escape cannot inject report lines or terminal sequences; distinguish them
   from literal escape-looking names. Verify invalid Unix bytes remain
   distinguishable, non-Unicode Windows code units retain a distinct reversible
   representation, and native Windows separators appear doubled in labels.
   Rich and plain labels agree after removing renderer-generated styling.
8. A constructed per-entry file-URL failure produces the escaped label without
   a link, no diagnostic, and success. Test representable native targets and
   removed files as well. Failure to establish the correct root produces a
   stderr error and nonzero exit, with no links against a guessed root.

## Implementation Verification and Documentation

Use the `sniff-cli`
[SniffCliFixture](../../cli/tests/common/mod.rs) test harness to isolate
subprocess directories and configuration. Level 1 tests use nextest
through the package-area recipes without the `test-fixtures` feature,
which is reserved for Level 2 terminal helpers. Update
[the existing association regression](../../cli/tests/l1/cli.rs), which
currently asserts identical verbose and non-verbose text, to expect the
new list while retaining its scope, count, and percentage assertions.
Validate implementation with `just test`, `just lint`, and
`cargo clippy -p sniff --all-targets -- -D warnings` plus
`cargo clippy -p sniff-cli --all-targets -- -D warnings`. These are
implementation checks, not prerequisites for this document review.

Update the CLI README, file-association topic documentation, and Sniff CLI
skill reference with the new filtered verbose behavior during implementation.

## Open Questions

Independent review found no further human rulings for this bounded presentation
change. Root selection, ordering, filename labels, hyperlink fallback, output
streams, JSON compatibility, captured-sample limits, and implementation
preparation boundaries are decided above. No spike or proof of concept was
warranted or performed.

Implementation ends at implementation complete and ready for review. The
author closes the review cycle and moves the fix to its completed lifecycle.
