---
$schema: feature-review.yaml
ready: true
findings: []
human_review: false
has_blocked_findings: false
blocked: false
reviewed_by: codex/gpt-6.1-sol
recurrence: false
created: "2026-10-08T18:16:30-07:00"
spec: 2026-10-11-extra-report-formats/spec.md
implemented: false
description: "A **feature** review of `2026-10-11-extra-report-formats/spec.md`"
feature: 2026-10-11-extra-report-formats/review-3.md
previous: 2026-10-11-extra-report-formats/review-2.md
---

# Review 3: Extra report formats

The feature is **ready for production**. The remaining finding from review 2, **R2-F1 — Tags silently change filenames that are not valid Unicode**, is verified fixed across its required instances and consumers. The seven findings from review 1 remain fixed. No new blocking finding or repair regression was found, and no human decision is required.

The accepted tags contract remains the extended subset with numeric addresses, recorded Hug kind/container values, and headless Neovim navigation. The original implementation log's Phase 5 authorization predates both repair cycles. Rejecting a filename the UTF-8 tags file cannot represent implements that accepted contract; it does not require another format decision. The reader harness was repaired before the earlier backslash repair. No finding is waiting on a newly removed blocker.

## Scope and evidence

I read the specification, plan, both earlier reviews, original implementation log, both repair ledgers, current report documentation, output resolver, selection and class-member projections, path helpers, shared listing/outline renderer, all four writers, and their focused tests. Completion flags were checked against code and independently stated output expectations.

All CLI symbols discussed here belong to **tree-hugger-cli**. The public path projection belongs to **tree-hugger**. In tree-hugger, [lossless_portable_relative](../../lib/src/file/naming.rs:167) shares the display projection's route calculation but checks whether every emitted path component has an exact Unicode spelling. In tree-hugger-cli, [native_selected_records](../../cli/src/declarations.rs:509) retains native paths for navigation writers, and [ctags::write_record](../../cli/src/ctags.rs:109) rejects an unrepresentable route before writing a tag. Display-only exports retain their existing path contract.

Fresh local runs checked document structure, literal preservation, selectors, class membership, sorting, coordinates, prelude aliases, controls, format conflicts, URLs, all-format goldens, work counters, cache/worker/width stability, editor navigation, and real-terminal rendering. Fresh Linux executions checked native byte filenames and the positive controls. Windows-only repairs were independently inspected through the native-path invariant and their registered test expectations; the repair log reports passing native Windows executions, but this review does not claim a fresh Windows execution. Cross-OS suite receipts are left to CI and do not determine readiness.

## Previous Finding Verification

Every parent finding from both earlier reviews is included. The following detailed tables retain the original child IDs, including siblings discovered by the implementer. A clean control marked verified fixed means its required behavior remains checked, not that it originally needed a repair.

| Finding ID/title | Required instances and consumers | Outcome | Verification evidence | Remaining work or blocker |
|---|---|---|---|---|
| R1-F1 — Markdown and Prose discard required listing content and layout | M1–M7, P1–P7, C1–C2: all commands, outlines, classes, groups, packages, sorting, signatures, comments, and terminal/row-export controls | verified fixed | Parsed CommonMark expectations and both Prose/plain comparisons passed; shared Layout and ListingStream still own structure/buffering; existing terminal goldens and tmux/WezTerm checks passed. | None |
| R1-F2 — Plain Prose deletes escaped source text | D1–D4, C1–C2, X1: generic headers, summaries, docs, owners, paths, styled exports, and god-files sibling | verified fixed | Parsed text-node stripping preserves source text; focused literal tests and god-files filename regression passed. | None |
| R1-F3 — Source controls escape neutralization in Markdown and tags warnings | M1–M3, T1, C1–C4, X1: every export boundary and enumerated stderr consumers | verified fixed | Raw-byte/parsed document checks passed; complete-message neutralization inspected in settlement, cache diagnostics, catalog/discovery errors, refs and main per-file errors. | None |
| R1-F4 — Prelude exports mix analysis-file paths with declaration locations | T1, P1, M1, C1–C2, X1: aliases, ordinary/class records, omission, subdirectory tags | verified fixed | Fresh cross-file alias test asserts exact declaration path, coordinates and href; subdirectory invocation remains correct. | None |
| R1-F5 — Class record exports do not sort the flattened file | T1–T2, C1–C2, S1–S4: all sort modes, commands, member filters, deduplication and document hierarchy | verified fixed | Independent name/kind/file sequences and static/instance membership assertions passed; flattening still sorts once after gathering and ID deduplication. | None |
| R1-F6 — Tags encoding changes real filenames and qualified names | N1, V1, P1, S1–S6, X1: name/container/path encodings, supported odd names, duplicates, outside-root paths, kinds and rejection policies | verified fixed | Exact byte corpus and fresh headless Neovim file/cursor navigation passed. The later non-Unicode sibling is also repaired under R2-F1 below. | None |
| R1-F7 — The tags reader test passes failed assertions and never navigates | A1–A3, C1–C2, X1: assertion status, actual navigation, tags location, registration and child-process checks | verified fixed | Fresh reader executes actual jumps and deliberate incorrect metadata/file expectations; pcall/cquit and completion marker checks enforce failures; tier audit found zero stranded tests. | None |
| R2-F1 — Tags silently change filenames that are not valid Unicode | P1–P7, C1–C7, D1–D9, DOC1, plus implementer sibling X1: native route decisions, every command/class projection, navigation and current docs | verified fixed | Native records reach the single lossless boundary; fresh Linux fixture/CLI checks pass every Linux shape and command/filter case. Windows route/CLI assertions and invariant inspected. Native table/Prose URL probe passed for all four commands. | None |

### Review 1 child instances and consumers

Evidence references in this table denote current tests or explicit code invariants, not ledger claims:

- **Document checks:** [export_prose_markdown.rs](../../cli/tests/symbols/export_prose_markdown.rs), fresh parsed CommonMark expectations and both specified Prose/plain comparisons across commands, groups, selectors, comments and signature modes.
- **Control checks:** [export_controls.rs](../../cli/tests/symbols/export_controls.rs), [export.rs](../../cli/src/export.rs), [ctags.rs](../../cli/src/ctags.rs), and complete-message stderr boundaries.
- **Location checks:** [prelude.rs](../../cli/tests/symbols/prelude.rs), including exact target coordinates/href and subdirectory invocation.
- **Order checks:** [export_order.rs](../../cli/tests/symbols/export_order.rs), independent expected sequences and member sets, plus flattening unit tests.
- **Reader checks:** [level2_declarations.rs](../../cli/tests/level2_declarations.rs), fresh headless Neovim lookup, canonical file/cursor checks, intentional failures and omitted-entry checks.
- **Export byte checks:** [export_tsv.rs](../../cli/tests/symbols/export_tsv.rs), [export_ctags.rs](../../cli/tests/symbols/export_ctags.rs), and current four-format goldens.

| Finding ID/title | Required instances and consumers | Outcome | Verification evidence | Remaining work or blocker |
|---|---|---|---|---|
| `R1-F1.M1` | Markdown `functions --tree`: methods nested under owner context | verified fixed | Document checks; current `write_listing`/`write_tree`/`write_classes` use the same renderer structure. | None in this instance. |
| `R1-F1.P1` | Prose `functions --tree`: owner context and indentation preserved | verified fixed | Document checks; current `write_listing`/`write_tree`/`write_classes` use the same renderer structure. | None in this instance. |
| `R1-F1.M2` | Markdown `types --tree --signatures none`: type outline with declared bases | verified fixed | Document checks; current `write_listing`/`write_tree`/`write_classes` use the same renderer structure. | None in this instance. |
| `R1-F1.P2` | Prose same request: same relation/outline content as plain | verified fixed | Document checks; current `write_listing`/`write_tree`/`write_classes` use the same renderer structure. | None in this instance. |
| `R1-F1.M3` | Markdown `symbols --tree` with filtered owner: context heading, no added declaration | verified fixed | Document checks; current `write_listing`/`write_tree`/`write_classes` use the same renderer structure. | None in this instance. |
| `R1-F1.P3` | Prose same requests: outline structure/context | verified fixed | Document checks; current `write_listing`/`write_tree`/`write_classes` use the same renderer structure. | None in this instance. |
| `R1-F1.M4` | Markdown `classes` all/static/instance: method and field sections | verified fixed | Document checks; current `write_listing`/`write_tree`/`write_classes` use the same renderer structure. | None in this instance. |
| `R1-F1.P4` | Prose same requests: same sections as class listing | verified fixed | Document checks; current `write_listing`/`write_tree`/`write_classes` use the same renderer structure. | None in this instance. |
| `R1-F1.M5` | Markdown file/module groups, packages, empty sections: headings and package sections | verified fixed | Document checks; current `write_listing`/`write_tree`/`write_classes` use the same renderer structure. | None in this instance. |
| `R1-F1.P5` | Prose same requests: existing layout | verified fixed | Document checks; current `write_listing`/`write_tree`/`write_classes` use the same renderer structure. | None in this instance. |
| `R1-F1.M6` | Markdown ungrouped `--sort name` across files: package-wide sort | verified fixed | Document checks; current `write_listing`/`write_tree`/`write_classes` use the same renderer structure. | None in this instance. |
| `R1-F1.P6` | Prose same request: package-wide sort | verified fixed | Document checks; current `write_listing`/`write_tree`/`write_classes` use the same renderer structure. | None in this instance. |
| `R1-F1.M7` | Markdown declaration projection: shared declaration text, one full header, semantic emphasis | verified fixed | Document checks; current `write_listing`/`write_tree`/`write_classes` use the same renderer structure. | None in this instance. |
| `R1-F1.P7` | Prose flat projection/comments: plain indentation and doc continuation layout | verified fixed | Document checks; current `write_listing`/`write_tree`/`write_classes` use the same renderer structure. | None in this instance. |
| `R1-F1.C1` | TSV/ctags: no document layout (sibling check) | verified fixed | Export byte checks; TSV/ctags dispatch still has no document headings. | None in this instance. |
| `R1-F1.C2` | Existing plain listing/outline/classes unchanged (sibling check) | verified fixed | Existing plain/tree/table/signature goldens and real-terminal L2 checks. | None in this instance. |
| `R1-F2.D1` | Full declaration header with generic type retained literally | verified fixed | Document checks assert retained literal text after rendering, not just absence of generated tags; `unstyled` walks parsed text nodes. | None in this instance. |
| `R1-F2.D2` | Short signature and class summary text preserved | verified fixed | Document checks assert retained literal text after rendering, not just absence of generated tags; `unstyled` walks parsed text nodes. | None in this instance. |
| `R1-F2.D3` | Raw doc lines with tags / unmatched `<` retained | verified fixed | Document checks assert retained literal text after rendering, not just absence of generated tags; `unstyled` walks parsed text nodes. | None in this instance. |
| `R1-F2.D4` | Source/owner names with angle brackets retained | verified fixed | Document checks assert retained literal text after rendering, not just absence of generated tags; `unstyled` walks parsed text nodes. | None in this instance. |
| `R1-F2.C1` | Location path `x<tag>.rs` preserved (sibling check) | verified fixed | Document checks assert retained literal text after rendering, not just absence of generated tags; `unstyled` walks parsed text nodes. | None in this instance. |
| `R1-F2.C2` | Styled Prose, TSV, Markdown generic text preserved (sibling check) | verified fixed | Document checks assert retained literal text after rendering, not just absence of generated tags; `unstyled` walks parsed text nodes. | None in this instance. |
| `R1-F2.X1` | Discovered sibling: `god-files` escaped source with `Prose::escape_text`, so `--plain` ESC stripping deleted filename text | verified fixed | Fresh god-files regression in the full L1 suite; six source-bearing `main.rs` sites use `render::literal` before rendering/escape stripping. | None in this instance. |
| `R1-F3.M1` | Markdown path with LF + blank line + indent: inert literal | verified fixed | Control checks: parsed Markdown and raw stdout/stderr assertions; generated doc line breaks are separated from `literal` source values. | None in this instance. |
| `R1-F3.M2` | Markdown source name / declaration LF cannot create structure | verified fixed | Control checks: parsed Markdown and raw stdout/stderr assertions; generated doc line breaks are separated from `literal` source values. | None in this instance. |
| `R1-F3.M3` | Markdown docs: only intentional continuation structure | verified fixed | Control checks: parsed Markdown and raw stdout/stderr assertions; generated doc line breaks are separated from `literal` source values. | None in this instance. |
| `R1-F3.T1` | Tags warning path LF / C1 CSI: visible escaped diagnostic | verified fixed | Control checks: parsed Markdown and raw stdout/stderr assertions; generated doc line breaks are separated from `literal` source values. | None in this instance. |
| `R1-F3.C1` | TSV cell encoder (sibling check) | verified fixed | TSV one-pass decoder corpus preserves LF/CR/TAB/backslash/C1/ESC without raw cell controls. | None in this instance. |
| `R1-F3.C2` | Prose literal boundary (sibling check) | verified fixed | Control checks: parsed Markdown and raw stdout/stderr assertions; generated doc line breaks are separated from `literal` source values. | None in this instance. |
| `R1-F3.C3` | Tags stdout field controls rejected (sibling check) | verified fixed | Tag writer tests reject every control in each name/container/path field before writing any bytes. | None in this instance. |
| `R1-F3.C4` | Markdown non-LF controls inert (sibling check) | verified fixed | Control checks: parsed Markdown and raw stdout/stderr assertions; generated doc line breaks are separated from `literal` source values. | None in this instance. |
| `R1-F3.X1` | Discovered siblings: per-file `error:` line (`declarations::settle`), package-catalog and `--changed` errors, `--verbose` cache lines, lint/imports/refs per-file errors | verified fixed | Code invariant: `settle`, catalog error, changed/discovery error, cache miss/write diagnostics, refs errors and main per-file errors neutralize their entire source-bearing message before stderr output. | None in this instance. |
| `R1-F4.T1` | TSV synthetic alias: target path + coordinates | verified fixed | Location checks assert target path/coordinates/href and synthetic omission; ordinary records and the subdirectory invocation are explicit controls. | None in this instance. |
| `R1-F4.P1` | Prose alias link href and label both target declaration | verified fixed | Location checks assert target path/coordinates/href and synthetic omission; ordinary records and the subdirectory invocation are explicit controls. | None in this instance. |
| `R1-F4.M1` | Markdown alias location (sibling check) | verified fixed | Location checks assert target path/coordinates/href and synthetic omission; ordinary records and the subdirectory invocation are explicit controls. | None in this instance. |
| `R1-F4.C1` | Tags synthetic alias omitted (sibling check) | verified fixed | Location checks assert target path/coordinates/href and synthetic omission; ordinary records and the subdirectory invocation are explicit controls. | None in this instance. |
| `R1-F4.C2` | Ordinary declarations and class members (sibling check) | verified fixed | Location checks assert target path/coordinates/href and synthetic omission; ordinary records and the subdirectory invocation are explicit controls. | None in this instance. |
| `R1-F4.X1` | Discovered sibling: ctags re-relativized projected paths against the working directory | verified fixed | Location checks assert target path/coordinates/href and synthetic omission; ordinary records and the subdirectory invocation are explicit controls. | None in this instance. |
| `R1-F5.T1` | Classes TSV `--sort name`: whole file name order | verified fixed | Order checks and `flatten_selected` unit assertions; Markdown/Prose classes use class sections instead of the flat export helper. | None in this instance. |
| `R1-F5.C1` | Classes ctags `--sort name`: same order as TSV | verified fixed | Order checks and `flatten_selected` unit assertions; Markdown/Prose classes use class sections instead of the flat export helper. | None in this instance. |
| `R1-F5.T2` | Classes TSV `--sort kind`: whole-file kind order | verified fixed | Order checks and `flatten_selected` unit assertions; Markdown/Prose classes use class sections instead of the flat export helper. | None in this instance. |
| `R1-F5.C2` | Classes ctags `--sort kind`: same | verified fixed | Order checks and `flatten_selected` unit assertions; Markdown/Prose classes use class sections instead of the flat export helper. | None in this instance. |
| `R1-F5.S1` | Non-class symbols TSV/ctags ordering (sibling check) | verified fixed | Order checks and `flatten_selected` unit assertions; Markdown/Prose classes use class sections instead of the flat export helper. | None in this instance. |
| `R1-F5.S2` | Class exports `--sort file` source order (sibling check) | verified fixed | Order checks and `flatten_selected` unit assertions; Markdown/Prose classes use class sections instead of the flat export helper. | None in this instance. |
| `R1-F5.S3` | Markdown/Prose class documents keep section order | verified fixed | Order checks and `flatten_selected` unit assertions; Markdown/Prose classes use class sections instead of the flat export helper. | None in this instance. |
| `R1-F5.S4` | Member selectors / dedupe membership unchanged (sibling check) | verified fixed | Order checks and `flatten_selected` unit assertions; Markdown/Prose classes use class sections instead of the flat export helper. | None in this instance. |
| `R1-F6.N1` | Tags name with PHP namespace backslash: exact recorded name | verified fixed | Export byte checks and fresh Reader checks verify exact values and actual navigation. | None in this instance. |
| `R1-F6.V1` | Tags container with qualified owner: exact recorded owner | verified fixed | Export byte checks and fresh Reader checks verify exact values and actual navigation. | None in this instance. |
| `R1-F6.P1` | Tags filename with literal POSIX backslash | verified fixed | Export byte checks and fresh Reader checks verify exact values and actual navigation. | None in this instance. |
| `R1-F6.S1` | Spaces, Unicode, leading space, `%`, `$`, `#` filenames navigate (sibling check) | verified fixed | Export byte checks and fresh Reader checks verify exact values and actual navigation. | None in this instance. |
| `R1-F6.S2` | Duplicate names navigate (sibling check) | verified fixed | Export byte checks and fresh Reader checks verify exact values and actual navigation. | None in this instance. |
| `R1-F6.S3` | Outside-root filename navigates (sibling check) | verified fixed | Export byte checks and fresh Reader checks verify exact values and actual navigation. | None in this instance. |
| `R1-F6.S4` | Unicode / unqualified owner (sibling check) | verified fixed | Reader checks pin Unicode owner Ω; byte checks pin unqualified Same; invariant: writer emits only recorded `container.qualified_name`, unchanged, never inferred by name equality. | None in this instance. |
| `R1-F6.S5` | Kind mapping completeness (sibling check) | verified fixed | Exhaustive kind/language matches, explicit generic type/unknown assertions, and the unchanged 16-language tags golden. | None in this instance. |
| `R1-F6.S6` | Controls rejected from tags fields (sibling check) | verified fixed | Tag control matrix rejects each field before writing; control warning regression checks stderr and exit 1. | None in this instance. |
| `R1-F6.X1` | Discovered siblings: filenames Neovim expands (`~` prefix, backtick, `[`, `$` + identifier) and `!_` names read as pseudo-tags | verified fixed | Pseudo-tag/expansion rejection corpus and CLI skip tests; fresh Reader checks retain only supported entries and verify header-only rejected-source output. | None in this instance. |
| `R1-F7.A1` | Reader assertion failure fails the test | verified fixed | Fresh Reader checks: actual navigation and deliberate assertion failures, with tags saved at the scan root. | None in this instance. |
| `R1-F7.A2` | Lookup plus actual file/line navigation | verified fixed | Fresh Reader checks: actual navigation and deliberate assertion failures, with tags saved at the scan root. | None in this instance. |
| `R1-F7.A3` | Tags saved at scan root | verified fixed | Fresh Reader checks: actual navigation and deliberate assertion failures, with tags saved at the scan root. | None in this instance. |
| `R1-F7.C1` | L1 tags string assertions (sibling check) | verified fixed | Export byte checks assert in the Rust process; no dependence on a child assertion exit status. | None in this instance. |
| `R1-F7.C2` | Reader target/tier registration (sibling check) | verified fixed | Cargo declares the reader target with terminal-tests; CI enables that feature; live L2 recipe selects it; tier audit reports zero stranded tests. | None in this instance. |
| `R1-F7.X1` | Sibling sweep of other tests that run scripted child processes | verified fixed | Code sweep of child-process tests: remaining git/hug/pane capture checks assert returned status/output in Rust; no other in-child assertion script is substituted for a Rust assertion. | None in this instance. |


### Review 2 child instances and consumers

[export_ctags_paths.rs](../../cli/tests/symbols/export_ctags_paths.rs) copies the existing Java fixture and changes one path property per case. Its expected names, lines, kinds, owners, safe entries, warnings and exit statuses are written independently. Each case runs functions, types, symbols, symbols --all-scopes, classes, classes --static-only and classes --instance-only. This checks missing safe results as well as incorrect extra results. Windows-only rows below are code/test-contract verification, not a fresh Windows run.

| Finding ID/title | Required instances and consumers | Outcome | Verification evidence | Remaining work or blocker |
|---|---|---|---|---|
| R2-F1.P1 | Linux declaration filename contains byte FF | verified fixed | Fresh Linux mixed-file and header-only CLI tests: exact safe tags, one warning per rejected record, exit 1. | None |
| R2-F1.P2 | Linux parent directory contains byte FF | verified fixed | Fresh directory-only edit: exact safe output and rejected-route warnings in all seven command/filter cases. | None |
| R2-F1.P3 | Non-Unicode declaration outside scan root | verified fixed | Fresh outside-root test reaches fixture through valid symlink; ../bad�.java is diagnostic text only, never a tag. | None |
| R2-F1.P4 | Replacement-character spelling already names a different file | verified fixed | Fresh decoy test tags Decoy only at its own canonical file; Counter and other rejected declarations never identify it. | None |
| R2-F1.P5 | Valid symlink resolves to non-Unicode declaration name | verified fixed | Fresh resolved-target test rejects the recorded native target route, not merely the alias argument. | None |
| R2-F1.P6 | Windows declaration filename contains unpaired surrogate U+D800 | verified fixed | Inspected from_wide fixture, exact mixed/header-only stdout, warning and exit assertions across seven cases. OsStr::to_str fails on the emitted surrogate component before write_tag; no replacement spelling is accepted. Prior native execution is reported in repair log, not rerun here. | None |
| R2-F1.P7 | Windows parent directory contains unpaired surrogate U+D800 | verified fixed | Same route-component invariant; registered Windows directory test checks all seven CLI cases, exact safe output/warnings/exit. No separate member or container bypass. | None |
| R2-F1.C1 | Non-Unicode spelling occurs only in Linux scan root | verified fixed | Fresh root-only control keeps exact Example.java and Safe.java tags, exit 0; canonical identities independently checked. | None |
| R2-F1.C2 | Actual filename contains valid Unicode U+FFFD | verified fixed | Fresh macOS and Linux control accepts bad�.java exactly; no scan for the replacement character is used to reject paths. | None |
| R2-F1.C3 | Names and recorded containers | verified fixed | UTF-8 String invariant; raw name/container fields, control and pseudo-tag checks unchanged; PHP byte corpus and Neovim navigation passed. | None |
| R2-F1.C4 | TSV and Markdown display paths | verified fixed | trim_record remains display-only; TSV/Markdown golden and source-control cases passed. Markdown generates no file links. | None |
| R2-F1.C5 | Prose native source-location URL | verified fixed | Fresh Linux URL probe checks original byte FF encoded as %FF for functions/types/symbols/classes. Native Windows URL conversion may return None, preventing a fabricated link. | None |
| R2-F1.C6 | Synthetic and pathless tag omission | verified fixed | Early return precedes route validation; synthetic flattening and cross-file prelude tests passed; no warning manufactured for omitted aliases. | None |
| R2-F1.C7 | Unpaired surrogate occurs only in Windows scan root | verified fixed | Inspected positive from_wide cwd control: exact relative tags and exit 0 across all seven CLI cases. route_text checks only emitted forward components; root spelling is not rejected. | None |
| R2-F1.D1 | Discovery, read and query preserve native source paths | verified fixed | Fresh Linux CLI cases read the actual fixture; native_selected_records never calls trim_record; native source paths survive preparation and class summaries. | None |
| R2-F1.D2 | functions selection and export | verified fixed | Fresh Linux cases assert independently listed constructor/method entries, exact safe entries, rejection count and status. | None |
| R2-F1.D3 | types selection and export | verified fixed | Fresh Linux cases assert Counter/Resettable and Safe container membership; container tags use the same rejection boundary. | None |
| R2-F1.D4 | symbols default scope and --all-scopes | verified fixed | Fresh independent sets distinguish the start parameter from default-scope declarations; rejected and safe membership checked separately. | None |
| R2-F1.D5 | classes selected containers | verified fixed | Fresh container expectations for classes, static-only and instance-only; native paths retained before flattening. | None |
| R2-F1.D6 | classes displayed members and static/instance filters | verified fixed | Fresh seven-case table independently distinguishes constructor/increment/reset from static zero and safe keep; unsupported member records never bypass write_record. | None |
| R2-F1.D7 | Flattening, sorting and ID deduplication | verified fixed | Flattening carries native references; ordering compares identities/spans rather than path representability. Fresh exact whole-file order, membership and duplicate-ID tests passed. | None |
| R2-F1.D8 | Unsupported decision, warnings, headers and exit status | verified fixed | No tag bytes written before route validation; Unsupported::UnrepresentablePath becomes inert stderr warning plus report error. Fresh mixed and all-rejected reports assert complete stdout/stderr/status. | None |
| R2-F1.D9 | Navigation must never receive an unsupported route | verified fixed | Fresh Linux public CLI outputs checked against expected declaration canonical identities, including decoy and root-only controls; fresh Neovim test retains supported navigation cases. | None |
| R2-F1.DOC1 | CLI README, symbol-report topic, tree-hugger skill/API guidance | verified fixed | Current docs describe native-path validation, omitted entries and exit 1, literal U+FFFD/root-only acceptance, and display-versus-navigation distinction. Spec snapshot is not substituted for current docs. | None |
| R2-F1.X1 | Implementer sibling: terminal table Location links used lossy paths | verified fixed | write_table now selects native records; Renderer::table projects label only and links native source. Fresh Linux all-command byte FF URL probe and subdirectory table regression passed; unchanged table golden and L2 class-table checks passed. | None |

### Ledger reconciliation and decision trace

Review 1 has seven parent findings and 62 unique child IDs after adding its five implementer-discovered siblings. Review 2 verified those children and introduced one new parent. The second repair ledger contains **25 child IDs**: seven failing path shapes, seven controls, nine consumer/decision rows, the documentation row, and the table-link sibling. Its summary calls these 26 child rows; 26 is the count including the parent. This bookkeeping mismatch hides no missing repair. All 25 child IDs are accounted for above. The prior review's implemented/implemented_by values are preserved as implementer claims.

The corrected route runs through these decisions: native discovery/read/query → shared filtering and class summaries → native selection → flattening/whole-file sorting/ID deduplication → synthetic/pathless omission → exact Unicode representability of the emitted route → control/pseudo-tag/reader-expansion checks → tag emission or warning/report-error → final status and reader navigation. Each declaration command and both class partitions use this same path. No display projection decides editor navigation. Table and Prose link targets independently retain the native record path; TSV, Markdown and JSON display projections do not claim a lossless navigation path.

The spec's old message and the plan's unchecked Phase 4 items still describe the original unfinished document writers. The later repair ledger and current code show shared listing buffering, outline/class structure, headings and emphasis, and focused document tests are implemented. These historical statements were reconciled against actual behavior rather than interpreted as approved scope reductions or new blockers. They are nonblocking snapshot bookkeeping; current topic documentation describes the repaired behavior.

## Unblocked Findings

None. No unresolved required repair, unchanged failing reproduction, repair regression, or newly discovered defective sibling remains in this review's sweep.

## Blocked Findings

None. Human review and cross-OS suite receipts are not repair blockers.

## Recurrence

No finding is carried as an open defect, so recurrence is false. Historically, review-2.md's R2-F1 was a newly discovered sibling of review-1.md's R1-F6 path-identity defect, not a regression in the PHP/backslash repairs. This review verifies both the original instances and that later sibling. The table-link sibling R2-F1.X1 was found and repaired during implementation, and its actual native URL behavior is checked above; it is not a new open finding.

## Input Robustness Matrix

Not applicable. This feature adds output writers and adapters over renderer-generated inline markup, not an external format/configuration loader. It introduces no load-bearing manifest, lockfile or configuration fields. Source literals, native path representation and output boundaries were checked directly.

## Requirement-to-verification map

| User-facing requirement | Appropriate level and evidence checked | Assessment |
|---|---|---|
| Format flags, one output mode, tree/signature/comment compatibility, help | L1 resolver tests and public CLI option cases | Passing |
| TSV header, eight cells/trailing empties, one-pass reversible encoding, Unicode, visibility/kind names, signatures | L1 exact bytes, test decoder and JSONL comparison; independent ordering tests | Passing |
| Same selected declarations, class member filters and no summary-field invention | L1 shared-selection/class tests, independent expected record sets and ID deduplication | Passing |
| Markdown flat/tree/class lists, contextual owners, groups/packages, relations, semantic emphasis and literal comments | L1 parsed CommonMark structure and independent declaration-content expectations | Passing |
| Prose layout/literal preservation, both plain comparisons, generated links and all signature/comment/group modes | L1 public Prose rendering and paired CLI checks; exact prelude href and native URL probes | Passing |
| Spaces/Unicode/parentheses/native Windows URLs | L1 native URL helper cases, public link assertions and native-path contract inspection | Passing checked contract; no fresh Windows runtime claim |
| Source markup/control safety on stdout and stderr; UTF-8/LF/no ANSI or OSC | L1 parsed/rendered hostile cases, raw-byte checks before golden normalization, neutralization/rejection invariants | Passing |
| Empty selection, partial file failure, discovery errors, stderr separation | L1 public CLI settlement/empty/error tests | Passing |
| Broken pipe exits 0, other write errors exit 1, bounded stopping/buffering | L1 write-status tests and shared streaming implementation; review 2's actual all-format pipe/write-limit probes remain prior evidence | Passing; permanent expanded probes remain nonblocking recommendation |
| Stable bytes across width/workers/cache; no extra reference/diagnostic/semantic/metrics work | L1 all-format stability and public analysis work counters | Passing |
| Tags headers, duplicates, kinds/owners, synthetic omission, exact paths, unsupported warnings/status | L1 bytes/independent native identity checks; L2 headless Neovim actual file/cursor navigation and deliberate harness failure controls | Passing |
| Shared terminal rendering, glyphs/styles/wrapping/tree/class/table behavior | L2 tmux and WezTerm captures; unchanged L1 terminal goldens | Passing executed backends; Kitty availability skips are not evidence |
| Keyboard input, modifier encoding, paste/IME/mouse | No new requirement | L3 not applicable |
| Every added test compiled and selected by a live tier | Cargo targets/module declarations, CI terminal-tests feature and zero-stranded tier audit | Passing |

The native-path tests are L1 filesystem/subprocess checks, not terminal tests. Their Linux and Windows cfg gates match the resource they need. The symbols target declares export_ctags_paths as a module, and its fixture uses include_str!, so compilation and repository-input scheduling are visible. The headless editor remains in the declared terminal-tests L2 target. No production-readiness claim rests on test count alone.

## Executed checks

- Local `just test export --test-threads 4`: **90 passed**; includes selection, structure/literals, controls, ordering, prelude locations, byte exports, work counters and four-format stability.
- Local `just test naming:: table_location_links output:: ctags:: golden:: --test-threads 4`: **45 passed**; includes all four export goldens, existing terminal goldens, option rules, path projection and tags rejection/encoding.
- Local `just test god_files_plain file_url prose:: markdown:: prelude:: --test-threads 4`: **51 passed**; includes god-files sibling, native URLs, inline adapters and prelude consumers. These filters overlap earlier runs; their counts are not added as unique coverage.
- Local `just test-l2`: **7 reported passed**. Neovim, tmux and WezTerm executed. Kitty checks finished in roughly 0.01 seconds and are treated as availability-gated skips. The shared harness opened its panes without a focus request; Neovim was headless.
- Local `just lint`: **passed** for both packages. No cargo fmt command was run.
- Root `just check-tier-coverage tree-hugger-cli`: **zero stranded tests**.
- Linux `just test export_ctags_paths naming:: --test-threads 4`: **15 passed**, including all seven Linux CLI path tests, the valid-U+FFFD control and native-byte projection test. BUILD_LINUX was configured; BUILD_WIN and BUILD_WSL were unset for this session.
- Additional Linux public CLI probe: copied unchanged Java fixture under native filename bytes bad\xff.java; every functions/types/symbols/classes table and Prose link used the original %FF URL, never the replacement spelling. Temporary fixture was automatically removed.

The Linux execution reused the clean standing checkout at 781dfe5ff3fdc82215a0bb3384e82c315010cd91. A local object comparison against this worktree's HEAD found no differences in tree-hugger, Cargo manifests/lockfile, biscuit-file, biscuit-terminal, renderable, test toolkit/harness, Cargo/nextest configuration or just recipes. No checkout/reset, commit or push was performed. This is fresh matching-code defect verification, not a full cross-OS suite receipt. A full local suite was not rerun: the scoped runs cover the feature and its repaired consumers, and the previous full-suite claim was not used as a substitute for their assertions.

## Nonblocking recommendations

The previous recommendation to keep the actual all-export write-failure/stop-work probes and expanded backtick corpus as permanent regressions remains useful. The repair ledger explicitly defers it; it was not an unresolved required finding. The current tests, shared writer invariant and review 2's successful public probes support the behavior. The accepted skip policy for Neovim-expanded filenames remains unchanged and needs no human reaffirmation.

The table-link repair is correct and now has fresh native-byte evidence. A permanent native-byte table URL regression would preserve that evidence more directly than the existing Unicode/subdirectory regression. This is a testing improvement, not an observed production defect.
