---
$schema: feature-review.yaml
next: 2026-10-11-extra-report-formats/review-2.md
ready: false
findings:
  - title: "R1-F1 — Markdown and Prose discard required listing content and layout"
    priority: high
  - title: "R1-F2 — Plain Prose deletes escaped source text"
    priority: high
  - title: "R1-F3 — Source controls escape neutralization in Markdown and tags warnings"
    priority: high
  - title: "R1-F4 — Prelude exports mix analysis-file paths with declaration locations"
    priority: high
  - title: "R1-F5 — Class record exports do not sort the flattened file"
    priority: medium
  - title: "R1-F6 — Tags encoding changes real filenames and qualified names"
    priority: high
  - title: "R1-F7 — The tags reader test passes failed assertions and never navigates"
    priority: high
human_review: false
has_blocked_findings: false
blocked: false
reviewed_by: codex/gpt-6.1-sol
recurrence: false
created: "2026-10-08T15:10:55-07:00"
spec: 2026-10-11-extra-report-formats/spec.md
implemented: true
description: "A **feature** review of `2026-10-11-extra-report-formats/spec.md`"
feature: 2026-10-11-extra-report-formats/review-1.md
log: tree-hugger/features/2026-10-11-extra-report-formats/log.md
implemented_by: claude/opus
---

# Review 1: Extra report formats

The feature is **not ready for production**. All four formats are exposed, but Markdown and Prose omit required listing behavior, several exports corrupt source text or locations, and the editor-reader test can report success after its assertions fail. There are seven blocking findings below. No human decision is needed to repair them: the implementation log records authorization for the extended tags subset and headless Neovim reader.

There is no earlier implementation review for this feature, so there are no carried findings, partial repairs, or recurrence to reconcile. The spec's implementation message and the plan already acknowledge unfinished Markdown/Prose work; that work is included in R1-F1 rather than treated as an approved reduction of scope. Previously repaired coordinate offsets, raw golden normalization, native URL encoding, and work-counter checks passed the focused verification; they do not resolve the findings below.

All production symbols linked below belong to **tree-hugger-cli**, unless another package is named. Only this review and the spec's review count were changed. The active feature directory remains in place.

## Verification and reproduction basis

The review read the spec, plan, implementation log, current documentation, output resolver, shared declaration dispatch, all four writers, record-flattening helper, terminal listing/outline/class renderers, and related tests. The plan is the supplied implementation design; there is no separate design document.

Reproductions used the built `target/debug/hug`, isolated home/config/temp directories, and scratch source trees outside repository fixture exclusions. In particular, copies of `lib/tests/fixtures/symbols/multi/java.java` and `php.php` reproduced the layout, generic-text, control-character, class-ordering, and tags-encoding defects. Each hostile-path case changed only the copied file's name; the generic case changed only `int start` to `java.util.List<Integer> start`; the PHP case added `namespace Outer\Inner;`. Reduced fixtures below make the failures easy to repeat. Temporary probes linked the already-built biscuit-terminal library to render Prose and pulldown-cmark to parse Markdown. No implementation or existing test was edited for those probes.

Expected results come from the specified export contracts, actual declaration locations, independently ordered names, and exact file/line checks in Neovim. Byte agreement across cache states is recorded separately and is not used as proof of correctness.

The input robustness matrix is **not applicable**: this feature adds output writers, not a production manifest, configuration, or interchange-format reader. Existing source/config/cache readers are unchanged. Hostile source values at the new output boundaries were reviewed instead.

## R1-F1 — High: Markdown and Prose discard required listing content and layout

**Defect class:** independent export serializers bypass the shared listing structure and therefore lose accepted layout options and required declaration content.

[write_export](../../cli/src/declarations.rs:286) passes flattened rows directly to [prose::write_file](../../cli/src/prose.rs:10) and [markdown::write_file](../../cli/src/markdown.rs:29). Neither receives the requested outline view or grouping options. Class summaries have already become containers and displayed methods, so their field sections cannot be reconstructed there. Markdown also constructs a new `name + signature_cell` declaration instead of sharing [Renderer::declaration](../../cli/src/render.rs:561), the formatter used to show declaration labels, access, and container context.

The complete decision chain is: the resolver accepts `--tree` and groups; command queries request containment for trees/classes; extraction and selection retain the eligible records; class summarization correctly partitions members and fields; `selected_records` and `flatten_selected` discard the layout/field-section representation; the writers emit only top-level bullets. The terminal's [ListingStream](../../cli/src/render.rs:615), outline ownership, relation labels, and class-section routines are never consulted. Computing containment alone does not implement a tree.

For example, use this source as `Example.java`:

```java
class Base {}
class Box extends Base {
    static void build() {}
    void open() {}
    int field;
}
```

Compare `hug symbols Example.java --tree --signatures none --format plain` with the same command using `markdown` and `prose`. Plain includes `Box extends Base` and indented members. Both exports emit top-level rows and omit `extends Base`. For `hug classes Example.java`, plain has Static Methods, Instance Methods, and Instance Fields; both exports have only flattened container/method bullets. A compact field string in a class signature is not the required field section and disappears under `--signatures none`.

| Child ID | Site / shape tested | Observed result | Expected result | Verification evidence |
|---|---|---|---|---|
| R1-F1.M1 | Markdown, `functions --tree` | Methods are top-level bullets | Nest methods under owner context | Copied Java fixture, plain comparison |
| R1-F1.P1 | Prose, `functions --tree` | Same flattening | Preserve owner context and indentation | Same fixture/command |
| R1-F1.M2 | Markdown, `types --tree --signatures none` | Flat type names; no inherited relation labels | Type outline with declared bases | Java fixture and reduced inheritance example |
| R1-F1.P2 | Prose, same request | Flat types; no base labels | Same relation/outline content as plain | Reduced example: `Box` lacks `extends Base` |
| R1-F1.M3 | Markdown, `symbols --tree`, including selected method with excluded owner | No nesting or owner-context heading | Keep the method under context without adding an owner declaration | `--name open` gives only a top-level `open` |
| R1-F1.P3 | Prose, same requests | Flat member with an inline owner suffix | Required outline structure/context | Same filtered command; inline suffix is not nesting |
| R1-F1.M4 | Markdown, `classes`, all/static/instance selections | No class-section hierarchy or field section | Preserve selected method and field sections | Copied Java and `--static-only`/`--instance-only` selection tests |
| R1-F1.P4 | Prose, same requests | Same missing sections | Same sections as existing class listing | Copied Java/plain comparison |
| R1-F1.M5 | Markdown, file/module groups, multiple packages, empty package selections | Flags produce no headings; empty output is one global message | Existing package sections and requested headings, including empty sections | Two Cargo packages, `--name absent`; paired group flags |
| R1-F1.P5 | Prose, same requests | Same missing headings/sections | Preserve existing layout | Same fixtures |
| R1-F1.M6 | Markdown, ungrouped `--sort name` across files | `zebra` in `a.rs` before `alpha` in `z.rs` | `alpha` then `zebra` within the package | Two-file fixture; independently stated name order |
| R1-F1.P6 | Prose, same request | Same file-first ordering | Same package-wide listing sort | Same fixture |
| R1-F1.M7 | Markdown declaration projection, short/full/none | Kind/access/container labels and semantic emphasis missing; full mode emits `sample fn sample(...)` | Shared declaration text, one full header, semantic emphasis | `fn sample(x: Vec<i32>) {}` with all signature modes |
| R1-F1.P7 | Prose flat declaration projection and comments | Declaration text is shared, but bullets lose existing initial indentation and docs are flush-left | Preserve plain listing indentation and doc continuation layout | Public Prose render probe and plain comparison |
| R1-F1.C1 | TSV / ctags grouping and flat record content | No headings, as intended | Fixed rows/tags; no document layout | Grouped TSV test; copied class fixture |
| R1-F1.C2 | Existing plain listing/outline/classes | Required headings, nesting, owner context, relations, and fields present | Existing behavior | Same commands on the same source |

**Repair:** complete the planned shared structure/markup extraction and route Markdown/Prose through the listing's existing buffering and ownership rules. Preserve per-file tree/class order and package-wide flat name/kind order without a second whole-scan accumulator. Add parsed Markdown nesting/emphasis tests and both specified Prose/plain comparisons over all commands, signature modes, grouping, comments, filtered owners, overloads, and empty sections. Update the export goldens after implementing the contract. Current docs mark these behaviors **planned**, which accurately describes the gap but does not satisfy the spec.

## R1-F2 — High: Plain Prose deletes escaped source text

**Defect class:** removing generated markup with an escape-unaware character scan also removes literal source content.

[strip_generated_tags](../../cli/src/prose.rs:32) treats every `<` as the start of a tag, including the escaped `\<` produced by the source-literal helper. The function runs over declarations and docs after their escaping. Thus `--plain` changes the data, rather than only removing generated styles and links.

The copied Java generic parameter contains `java.util.List<Integer>` in styled Prose, TSV, and Markdown, but loses `Integer` in plain Prose. A reduced `fn sample(x: Vec<i32>) {}` becomes `fn sample(x: Vec\\)` in the exported Prose. Rendering that result with biscuit-terminal produces `fn sample(x: Vec\)`. A doc line `literal <img> and **bold**` loses `<img>` and retains an unwanted backslash.

| Child ID | Site / shape tested | Observed result | Expected result | Verification evidence |
|---|---|---|---|---|
| R1-F2.D1 | Full declaration header, generic type | Escaped `<Integer>` / `<i32>` deleted | Entire header retained literally | Copied Java one-type edit; rendered reduced Rust fixture |
| R1-F2.D2 | Short signature and class summary text | Any escaped angle-delimited type enters the same stripping function | Source text preserved | Precise invariant: `declaration` is stripped regardless of signature mode or record kind |
| R1-F2.D3 | Raw doc lines, tags / unmatched angle brackets | `<img>` deleted; unmatched `<` suppresses the remainder | Literal docs retained | CLI `--comments --plain`; scan has no escape handling |
| R1-F2.D4 | Source names and owner names containing angle brackets | Same destructive rule would apply | Names remain literal | Invariant: `literal(named)` is inside the declaration passed to the stripping scan |
| R1-F2.C1 | Location path `x<tag>.rs` | Preserved | Literal filename retained | Location is escaped separately and never passed through the strip function; CLI reproduction |
| R1-F2.C2 | Styled Prose, TSV, Markdown on generic fixture | Generic text preserved | Preserve source text | Same one-field edit across writers |

**Repair:** choose styled/plain presentation when constructing markup, or remove only generated style nodes before serialization. Do not scan the escaped final string for angle brackets. Assert rendered literal equality, not merely absence of `<b>`, `<dim>`, and `<a>`; the present plain-Prose test passes this corruption.

## R1-F3 — High: Source controls escape neutralization in Markdown and tags warnings

**Defect class:** some output boundaries preserve source control characters instead of making them inert.

[markdown::escape_text](../../cli/src/markdown.rs:9) preserves LF before its control-character branch. It is used for paths and declaration strings, not only intentional document line breaks. Rename the copied Java fixture to `prefix\n\n        injected.java` (the `\n` here denotes actual filename newlines), then run `hug functions --format markdown`. The filename creates a blank line and indented continuation. Parsing the shipped output with pulldown-cmark yields `Start(CodeBlock(Indented))`; a literal filename must not create a code block.

Separately, [write_export](../../cli/src/declarations.rs:324) interpolates the unsafe tags path directly into its warning. A copied fixture named `bad<U+009B>31m.java` is correctly omitted from tags and returns exit 1, but stderr contains the raw C1 CSI character. A filename newline also creates an extra warning line. Rejecting the stdout record does not neutralize the warning displayed in a terminal.

| Child ID | Site / shape tested | Observed result | Expected result | Verification evidence |
|---|---|---|---|---|
| R1-F3.M1 | Markdown path, LF + blank line + indentation | Unintended indented code block | Inert literal filename in its declaration item | Copied Java filename edit; parser event stream |
| R1-F3.M2 | Markdown source name / declaration string | Same LF-preserving escape function | Source LF cannot become document structure | Invariant tied to both arguments passed to `escape_text` in `write_file` |
| R1-F3.M3 | Markdown docs | Deliberate doc line breaks; tested markup punctuation remains literal | Intentional continuation structure only | Existing parsed comment test; docs split into lines before escaping |
| R1-F3.T1 | Tags warning path, LF / C1 CSI | Raw controls on stderr | Visible escaped diagnostic text | Actual unsafe filenames; status 1 and safe stdout omission verified |
| R1-F3.C1 | TSV cell encoder, LF / CR / TAB / ESC / C1 / literal escapes | Encoded, reversible cells | One row per record, eight cells | Existing one-pass decoder test and copied hostile filenames |
| R1-F3.C2 | Prose literal boundary, same filename edits | Controls become visible escapes; no raw control in output | Literal text remains inert | Copied filenames across both Prose presentation modes |
| R1-F3.C3 | Tags stdout name/container/path controls | Record rejected before partial tag output | Omit entry, warn, exit 1 | Writer corpus checks TAB, CR, LF, ESC, U+0085, U+009B for each field |
| R1-F3.C4 | Markdown non-LF controls | Replaced with spaces | No terminal control emitted | Control branch invariant and copied C1 path |

**Repair:** neutralize source text before Markdown punctuation escaping; add document newlines only in the structure writer. Neutralize every source-derived warning value before stderr emission. Add parsed event tests for newline/indentation filenames and raw-byte assertions on stderr as well as stdout.

## R1-F4 — High: Prelude exports mix analysis-file paths with declaration locations

**Defect class:** a batch's analysis filename is substituted for a selected record's actual declaration filename.

Prelude aliases intentionally carry target declaration locations while their identity and analysis grouping belong to the prelude file. [selected_records](../../cli/src/declarations.rs:473) retains that distinction, but [tsv::write_rows](../../cli/src/tsv.rs:20) uses its batch `file` argument for every row. [prose::write_file](../../cli/src/prose.rs:16) uses the batch path for the link while displaying the record's path in the label.

Reproduce with a Cargo package containing `src/lib.rs` declaring `alpha` and `prelude`, `src/alpha.rs` containing a blank line then `pub struct Original;`, and `src/prelude.rs` containing `pub use crate::alpha::Original as Alias;`. Run `hug symbols --prelude --format FORMAT`.

The alias is declared at `src/alpha.rs:2:1`. TSV reports `src/prelude.rs`, line 2. Prose labels the link `src/alpha.rs:2:1`, but its href ends in `src/prelude.rs#L2`. There is no line 2 declaration in the one-line prelude. The existing prelude fixture independently pins this target-location contract for synthetic aliases.

| Child ID | Site / shape tested | Observed result | Expected result | Verification evidence |
|---|---|---|---|---|
| R1-F4.T1 | TSV resolved synthetic alias | Prelude path + target line/column | Target path + target line/column | Reduced Cargo fixture; `source.file_path` contract in prelude tests |
| R1-F4.P1 | Prose resolved synthetic alias link | Target label but prelude href | Link and label both target actual declaration | Same CLI fixture; inspected URL |
| R1-F4.M1 | Markdown alias location | Target path/line | Target path/line | Same fixture: `src/alpha.rs:2` |
| R1-F4.C1 | Tags synthetic alias | Omitted | Omitted | Same fixture: headers only |
| R1-F4.C2 | Ordinary declarations and class members | Batch and declaration paths agree | Their actual path/coordinates | Java fixture and existing first-byte/JSON Lines cross-check |

**Repair:** project each record's declaration path relative to the scan root for TSV; derive each Prose URL from the corresponding native declaration path. Keep alias ID, qualified-name prefix, and batch ownership unchanged. Preserve access to native paths before portable projection rather than interpreting an already projected path against an unrelated directory. Add a cross-file prelude test for all four formats, with exact path/coordinate/URL expectations.

## R1-F5 — Medium: Class record exports do not sort the flattened file

**Defect class:** sorting containers and each member set separately is mistaken for sorting the combined record export.

[flatten_selected](../../cli/src/export.rs:10) emits each sorted container followed by its sorted members. For TSV the spec requires flattening the selected class records and then applying the per-file order. Tags also inherit the selected per-file record order. The shared helper carries the mistake into both consumers.

Use `Order.java`:

```java
class Zebra { void aardvark() {} }
class Alpha { void zulu() {} }
```

For `hug classes Order.java --sort name --format tsv`, the names are `Alpha, zulu, Zebra, aardvark`; the independently sorted required sequence is `Alpha, Zebra, aardvark, zulu`. Ctags gives the same wrong sequence. The copied multi-language Java fixture, with only `increment` renamed to `zulu`, also puts `zero`/`zulu` ahead of the next container `Resettable`.

| Child ID | Site / shape tested | Observed result | Expected result | Verification evidence |
|---|---|---|---|---|
| R1-F5.T1 | Classes → TSV, `--sort name` | Container/member blocks | Whole flattened file in name order | Reduced two-class fixture and copied Java rename |
| R1-F5.C1 | Classes → ctags, `--sort name` | Same wrong blocks | Same selected record order as TSV | Same fixtures |
| R1-F5.T2 | Classes → TSV, `--sort kind` | `Zebra, aardvark, Alpha, zulu` | Both classes, then methods in source order | Reduced fixture; kind rank then declaration-span tie-breakers |
| R1-F5.C2 | Classes → ctags, `--sort kind` | Same wrong blocks | Same whole-file kind order | Same fixture |
| R1-F5.S1 | Non-class symbols → TSV/ctags, file/name/kind | Correct combined ordering | Independently calculated file/name/kind sequences | Same declarations, `symbols` command |
| R1-F5.S2 | Class exports, `--sort file` on lexical Java containers | Correct source order | Source order | Same fixture |
| R1-F5.S3 | Markdown/Prose class documents | Current helper also preserves blocks | Final document hierarchy must preserve section/outline order, not impose flat row order | R1-F1 owns their missing structural repair |
| R1-F5.S4 | Member selectors, static/instance partition, deduplication, summary-only fields | Correct membership before sort | No extra/missing record from flattening | Existing classes TSV and helper tests; ID-set invariant |

**Repair:** gather and deduplicate eligible flat records, then sort the resulting reference vector once. Keep the helper's synthetic-record policy and class member selectors. Test exact name and kind sequences across both row/tag consumers; the current helper test's accepted container-block sequence does not prove the required ordering.

## R1-F6 — High: Tags encoding changes real filenames and qualified names

**Defect class:** unverified escaping and separator conversion alter values consumed by the named tags reader.

[ctags::write_record](../../cli/src/ctags.rs:51) doubles backslashes in tag names and container fields and replaces filename backslashes with `/`. A POSIX filename can contain a literal backslash. The copied Java file renamed to `back\slash.java` produces tags for the nonexistent `back/slash.java`, exits 0, and does not warn. A reduced `back\slash.rs` fixture produces `target\tback/slash.rs...`; headless Neovim `:tag target` reports `E429: File "back/slash.rs" does not exist`.

The PHP fixture with `namespace Outer\Inner;` produces a doubled-backslash namespace name and doubled-backslash `class:` value. Neovim `taglist('deposit')` returns a container with length 21 instead of the recorded `Outer\Inner\Account` length 19. The reduced `Outer\Inner\Box` example returns length 17 instead of 15. Neovim does not decode this output into the recorded name. This affects both namespace tag identity and container-based disambiguation.

The filename loss begins in shared portable-path projection before the writer's additional replacement. That broader utility limitation predates this feature; the new navigation format still must honor its explicit rule to encode a supported path or skip it with a warning and exit 1, never manufacture a filename. TSV's existing path-conversion contract does not establish that the resulting spelling is navigable by an editor.

| Child ID | Site / shape tested | Observed result | Expected result | Verification evidence |
|---|---|---|---|---|
| R1-F6.N1 | Tags name, PHP namespace with backslash | Reader retains doubled backslash; 12 bytes instead of 11 in `Outer\Inner` | Exact recorded name | Copied PHP namespace edit; Neovim field/length checks |
| R1-F6.V1 | Tags container extension, PHP qualified owner | Reader retains doubled separators | Exact recorded qualified owner | Copied PHP `deposit`: 21 versus 19 bytes |
| R1-F6.P1 | Tags filename, literal POSIX backslash | Invented directory path; exit 0 | Reader opens actual file, or entry rejected with warning/exit 1 | Copied Java rename; reduced Rust navigation fails E429 |
| R1-F6.S1 | Filename spaces, Unicode, leading space, `%`, `$`, `#` | Numeric navigation reaches actual files | Exact canonical file and source line | Headless reader with `pcall` + `cquit 1` on failure |
| R1-F6.S2 | Duplicate names | Both entries found; second entry navigates to `two.rs:2` | Preserve duplicates and numeric navigation | `taglist` plus `2tag duplicate`, exact file/line assertion |
| R1-F6.S3 | Outside-root filename | `../outside.rs`, numeric navigation works | Actual outside declaration file at line 2 | CLI absolute `--file` request; tags saved at scan root |
| R1-F6.S4 | Unicode owner and unqualified owner | Exact container field | Exact recorded owner, no guessed ownership | Python `Ω` and Rust `Same` fixtures |
| R1-F6.S5 | All 16 language / 19 kind mappings | Exhaustive matches preserve generic `type`/`unknown` | Published Hug kind words without promotion | Code invariant in `kind`; 16-language tags golden |
| R1-F6.S6 | Controls in name/container/filename | Rejected from tags | Skip, warning, exit 1 | Existing field-by-control writer matrix; warning safety separately tracked by R1-F3.T1 |

**Repair:** verify separate name, filename, and extension encodings against Neovim rather than assuming backslash escaping works for every field. Preserve native filename information long enough to detect unsupported names. Either emit values the reader round-trips exactly or apply the specified warning/exit-1 omission policy. Add PHP qualified-name cases and POSIX literal-backslash filename navigation to the reader fixture, documenting the accepted boundary.

## R1-F7 — High: The tags reader test passes failed assertions and never navigates

**Defect class:** a reader check mistakes successful process shutdown for successful assertions and substitutes metadata inspection for the required operation.

[level2_ctags_reader_resolves_duplicates_and_numeric_lines_headlessly](../../cli/tests/level2_declarations.rs:1102) executes Lua assertions followed by `qa!`, then checks only `reader.status.success()`. Repeating that process shape with `lua assert(false, "deliberate reader assertion failure")` prints `E5108` and a Lua stack trace, but exits **0**. The current test therefore can pass after any reader assertion fails.

It only calls `taglist()` and inspects `cmd` strings; it never asks Neovim to open a tag or checks the resulting file/cursor. It also saves tags under the temporary directory while sources are in the repository scratch directory, so the root-relative paths are not positioned for normal navigation. This is why R1-F6's nonexistent-file defect is invisible to the reader check. The documentation's statement that numeric-line navigation is verified is consequently inaccurate.

| Child ID | Site / shape tested | Observed result | Expected result | Verification evidence |
|---|---|---|---|---|
| R1-F7.A1 | Reader assertion failure | Error printed, exit 0, Rust assertion would pass | Failed assertion must fail test | Deliberately false Lua assertion with identical headless flags and `qa!` |
| R1-F7.A2 | Duplicate/Unicode/space/container reader checks | `taglist` metadata only | Lookup plus actual file/line navigation for supported subset | Entire test body has no tag jump or cursor assertion |
| R1-F7.A3 | Tags file location | File is outside source scan root | Save at root or explicitly rebase paths before navigation | `fixture.temp/.../tags` versus `fixture.repo` sources |
| R1-F7.C1 | L1 tags string assertions | Correctly assert numeric output strings/duplicate count | Appropriate byte-level checks; cannot replace reader operation | `export_ctags` target module and passing tests |
| R1-F7.C2 | Reader target/tier registration | Compiled and selected by live L2 recipe | No stranded verification | `required-features`, CI `features`, root tier audit, focused L2 run |

**Repair:** run assertions inside an error handler that calls `cquit 1`, or use another reader interface with reliable failure propagation. Put tags at their promised root, execute navigation for duplicate entries and unusual supported filenames, and assert the exact opened canonical file and cursor line. Keep the reader headless. Update the docs and implementation-log claims once these operations are actually verified. A successful execution of today's test is not evidence that its assertions or navigation contract passed.

## Requirement-to-verification map

These exports promise serialized data and document markup, not keyboard input. Level 1 is appropriate for bytes, CLI selection/errors, parsed CommonMark structure, and in-process Prose rendering. The actual named editor-reader contract is checked at Level 2 through the area's headless reader recipe. There is no Level-3 keyboard or focus requirement. The implementation did not change shared terminal presentation; its conditional real-terminal regression requirement does not create a new exported-file rendering requirement.

| Requirement | Strongest implemented check | Review outcome |
|---|---|---|
| Output conflicts, comments/signature restrictions, accepted formats | L1 resolver/CLI tests; review ran all 80 command × export × competing-mode cases | Conflict cases clean; accepting an option does not prove its layout |
| UTF-8/LF/no ANSI or OSC, width/worker/cache stability | L1 raw-byte goldens and all-format stability test | Ordinary corpus passes; hostile control failures in R1-F3 |
| TSV header/eight cells/escaping/signatures/coordinates | L1 writer decoder and CLI cross-check | Ordinary rows pass; prelude path R1-F4 and class sort R1-F5 fail |
| Same selected declarations, class static/instance membership | L1 selectors and class TSV tests | Shared selection is retained; document fields/layout lost in R1-F1 |
| Markdown tree, groups, class sections, relations and emphasis | L1 flat golden; no expected nested event coverage | Missing implementation and verification, R1-F1 |
| Markdown source punctuation and comments are literal | L1 parsed hostile-comment test | Tested punctuation clean; newline path creates code block, R1-F3 |
| Prose source escaping, local URLs, plain equivalence and layout | L1 substring test, flat golden, native URL helper test | Helper URL test clean; actual exported alias href R1-F4, destructive plain stripping R1-F2, equivalence/layout R1-F1 |
| Partial failure, stderr separation, empty discovery/selection | L1 CLI tests; review checked successful plus invalid UTF-8 source in all exports | Clean statuses and retained successful records; unsafe warning bytes R1-F3 |
| Broken pipe and other write failures | L1 status-helper test; review exercised actual closed stdout pipes for all four exports | Closed pipes exit 0 quietly; other-error helper exits 1. Add writer-level failure/stop-work regressions during repair |
| No extra references, semantics, diagnostics or metrics; bounded buffering | L1 uncached work-counter/query tests; writer/scan code inspection | Counters pass; export rows bounded by existing chunks. R1-F1 must retain listing's required buffering |
| Tags headers, duplicates, synthetic omission, kind/container fields | L1 byte tests and writer control matrix | Ordinary cases clean; encoding R1-F6 and sorting R1-F5 fail |
| Tags actual reader lookup and numeric navigation | L2 headless Neovim test | Wrong operation and unreliable failure detection, R1-F7 |
| Test compilation/tier selection | Declared symbols modules, L2 target and feature settings; tier audit | No stranded tests found |

The broader hostile-input matrix requested by the spec is still incomplete: the three Markdown/Prose tests exercise one doc-comment punctuation string, basic format availability, and generated-tag absence. They do not independently verify hostile names/signatures/paths, all backtick runs, both corrected Prose/plain comparisons, nested parsed lists, or class/grouped documents. The regression requirements in R1-F1–R1-F4 cover these gaps; they are not omitted nonblocking work. Ordinary backtick runs in a copied doc comment remained literal in the review's Prose render probe, but that single reproduction does not replace the requested corpus.

## Executed checks

- `just test export`: 44 passed, including all-format stability and real uncached analysis counters.
- `just test golden --test-threads 4`: 19 passed, including all four export goldens and existing terminal/plain goldens.
- `just lint`: passed for tree-hugger and tree-hugger-cli.
- `just check-tier-coverage tree-hugger-cli` from the repository root: no stranded tests.
- `just test-l2 level2_ctags_reader`: one selected test passed; R1-F7 explains why that result is insufficient reader evidence. The recipe and reader were run without requesting window focus.
- Full `just test --test-threads 4`: 1,970 passed, zero skipped (11 slow), in 374 seconds.

Cross-OS receipts and human approval are not used as readiness blockers. No new crate, CI cell, gate, or schema change is recommended.

## Nonblocking recommendations

After correcting behavior, consider sharing the lower-case kind mapping between TSV and tags if their published spellings are intended to stay identical. Keep their field encoders separate: these formats have different reader contracts. This maintenance suggestion is not an additional readiness finding.
