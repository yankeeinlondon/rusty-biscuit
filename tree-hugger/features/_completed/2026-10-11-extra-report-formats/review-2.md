---
$schema: feature-review.yaml
ready: false
findings:
  - title: "R2-F1 — Tags silently change filenames that are not valid Unicode"
    priority: high
human_review: false
has_blocked_findings: false
blocked: false
reviewed_by: codex/gpt-6.1-sol
recurrence: true
created: "2026-10-08T17:17:13-07:00"
spec: 2026-10-11-extra-report-formats/spec.md
implemented: true
implemented_by: claude/opus
log: tree-hugger/features/2026-10-11-extra-report-formats/log.md
description: "A **feature** review of `2026-10-11-extra-report-formats/spec.md`"
feature: 2026-10-11-extra-report-formats/review-2.md
previous: 2026-10-11-extra-report-formats/review-1.md
next: 2026-10-11-extra-report-formats/review-3.md
---

# Review 2: Extra report formats

The feature is **not ready for production**. All seven findings from the first review are verified fixed within their enumerated scope. One newly discovered sibling remains: tags output silently changes native filenames that are not valid Unicode into different UTF-8 paths. It can direct an editor to a nonexistent file or an existing, unrelated file while appearing successful. This is not a regression in the repaired backslash cases.

No human decision blocks the repair. The accepted extended tags subset already requires unsupported entries to be omitted with a warning and exit 1. The implementation log's Phase 5 authorization predates these repairs; the stricter filename boundary does not require reopening that decision. The reader harness was repaired before the backslash encoding repair, as recorded in the repair log, and its independent navigation and failure checks now pass.

## Scope and evidence

I read the specification, implementation plan, original implementation log, first review, repair ledger, current documentation, resolver, selection/projection pipeline, four writers, shared listing/outline/class renderer, and associated tests. The first review is the only earlier review in this directory. Completion flags and ledger dispositions were treated as claims, then checked against current code and fresh executions.

All production symbols linked here belong to **tree-hugger-cli**, except the explicitly identified public path projection in **tree-hugger**. Only this review, the previous review's `next` field, the specification's review count, and the OS skill's newly observed Windows filename fact are changed. Existing implementation edits in the worktree are preserved.

## Previous Finding Verification

The parent rows summarize the outcome; the instance rows below retain every child identifier from the first review and the repair ledger, including the implementer's discovered siblings. “Verified fixed” for a previously clean site means its required invariant remains verified, not that it originally required a code change.

| Finding ID/title | Required instances and consumers | Outcome | Verification evidence | Remaining work or blocker |
|---|---|---|---|---|
| R1-F1 — Markdown and Prose discard required listing content and layout | All Markdown/Prose listing, tree, class, grouping, package, order, declaration and comment instances; TSV/ctags and terminal controls. | **verified fixed** | Fresh parsed Markdown expectations and both Prose/plain comparisons; shared `Layout` dispatch and bounded `ListingStream`; existing terminal goldens and L2 checks. | None in the original scope. |
| R1-F2 — Plain Prose deletes escaped source text | Generic headers, short signatures, summaries, docs, names/owners, paths, styled/other exports; added god-files sibling. | **verified fixed** | Parsed text-node stripping replaces the destructive scan; literal rendered equality tests pass, including the god-files regression. | None. |
| R1-F3 — Source controls escape neutralization in Markdown and tags warnings | Markdown path/declaration/doc controls, tags warnings, TSV/Prose/tag controls, and added stderr sites. | **verified fixed** | Fresh hostile source/path tests and source inspection of each complete-message neutralization boundary. | None. |
| R1-F4 — Prelude exports mix analysis-file paths with declaration locations | TSV/Prose prelude aliases, Markdown locations, synthetic tag omission, ordinary records/members; subdirectory tags sibling. | **verified fixed** | Cross-file prelude regression checks exact declaration path, coordinates and href, including invocation from `src/`. | None. |
| R1-F5 — Class record exports do not sort the flattened file | Class TSV/ctags name/kind/file order, ordinary symbols, document hierarchy, member filters and deduplication. | **verified fixed** | Whole-file sort after gathering/deduplication; exact independent sequences in `export_order` and membership/helper assertions. | None. |
| R1-F6 — Tags encoding changes real filenames and qualified names | PHP tag name/container, Unix backslash filename, unusual supported names, duplicates, outside-root, owners, kind/control mappings; expansion/pseudo-tag siblings. | **verified fixed** | Fresh byte tests and real Neovim file/cursor navigation; current raw-field writer and explicit rejection decisions. | New sibling R2-F1 is outside the previously enumerated cases; the original cases remain fixed. |
| R1-F7 — The tags reader test passes failed assertions and never navigates | Failed assertions, actual navigation, tags location, L1 controls, live reader target/tier; other scripted subprocess checks. | **verified fixed** | Fresh L2 reader execution checks navigation and deliberately wrong expectations; `pcall` plus `cquit 1` and completion marker are enforced. | None. |

### Required instance and consumer checks

Evidence abbreviations in the table are paths to current tests or code, not historical completion claims:

- **Document checks:** [export_prose_markdown.rs](../../cli/tests/symbols/export_prose_markdown.rs). Parsed Markdown expectations state the required heading/list/emphasis/content structure independently; Prose checks both specified plain comparisons across commands, signature modes, comments, selections and groups.
- **Control checks:** [export_controls.rs](../../cli/tests/symbols/export_controls.rs), plus the TSV one-pass decoder and tag control matrix in [export.rs](../../cli/src/export.rs) and [ctags.rs](../../cli/src/ctags.rs).
- **Location checks:** [prelude.rs](../../cli/tests/symbols/prelude.rs), especially `prelude_exports_locate_cross_file_aliases_at_their_declaration_in_every_format`.
- **Order checks:** [export_order.rs](../../cli/tests/symbols/export_order.rs), whose expected sequences are independently written for file/name/kind order and static/instance membership.
- **Reader checks:** [level2_declarations.rs](../../cli/tests/level2_declarations.rs), including the actual jump, canonical file/cursor assertions, wrong-expectation checks and skipped-entry checks.
- **Export byte checks:** [export_tsv.rs](../../cli/tests/symbols/export_tsv.rs) and [export_ctags.rs](../../cli/tests/symbols/export_ctags.rs).

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

The ledger's additional siblings are included above rather than silently ignored. Its completion summary says “64” instance rows; the actual ledger contains 62 unique child rows (57 from the first review and five added sibling rows), plus seven parent rows. Every one is accounted for here; the inconsistent summary count is not evidence of an omitted repair. Its seven parent dispositions are supported within the first review's enumerated scope. Its broader claim that the filename defect class is closed is disproved by R2-F1 below. The old specification message and unchecked plan entries still describe the earlier unfinished document phase; current implementation and current topic documentation establish that those document behaviors now exist. These snapshot statements are not used to mark the repaired code incomplete.

## Unblocked Findings

### R2-F1 — High: Tags silently change filenames that are not valid Unicode

**Defect class:** a lossy display-path conversion is reused as a navigation path without validating that it still identifies the declaration file.

The tree-hugger package's [portable_relative](../../lib/src/file/naming.rs:142) projects each native path through `route_text`, which calls `to_string_lossy()` on every remaining path component. In tree-hugger-cli, [selected_records](../../cli/src/declarations.rs:490) invokes [trim_record](../../cli/src/report.rs:129) on ordinary records, class containers, and each displayed member before [write_export](../../cli/src/declarations.rs:315) hands them to [ctags::write_record](../../cli/src/ctags.rs:114). The writer calls `to_string_lossy()` again and accepts the replacement character as ordinary text. The original bytes are already unavailable to its rejection checks.

For a Linux file named with bytes `bad\xff.java`, the projected filename is `bad�.java` (a literal U+FFFD replacement character). Every emitted tag identifies that different filename. If no such file exists, navigation fails. If `bad�.java` exists, the tags point to the wrong file. The current control, pseudo-tag and filename-expansion checks reject neither case, so no skipped-tag error contributes to exit status 1.

This is reachable through ordinary discovery, not only a malformed record: Linux accepts these names, `SourceFile::read` reads their UTF-8 contents, and the parsed records retain the actual native path until export projection. A native path may be non-UTF-8 even though the source text and every symbol name are valid UTF-8. Windows stores native names as UTF-16 code units and can accept a lone half of a surrogate pair. That filename or directory name is not valid Unicode text and cannot be represented losslessly as UTF-8. A probe using `OsString::from_wide` with U+D800 returned eight real declarations, then emitted eight nonexistent replacement-character paths with zero rejected entries for both shapes. The scan-root-only surrogate control retained eight valid relative paths. The specification requires the exported file itself to be UTF-8; that is a reason to reject an unrepresentable navigation path, not to substitute another file.

#### Reproduction and evidence

I copied the real [Java fixture](../../lib/tests/fixtures/symbols/multi/java.java) into an owned temporary directory on the configured Linux build host. Each path case changed the copied fixture's filename, directory or scan-root spelling; its contents stayed unchanged. A narrow Rust probe used the public `SourceFile::read`, `SymbolQuery` with containment, and `portable_relative` APIs, then compiled this worktree's **exact current `ctags.rs`** as its output writer. The host's existing tree-hugger library was reused; its naming module was compared byte-for-byte with this worktree and matched. This was an isolated API/writer reproduction, **not** a claim that the entire current CLI was rebuilt or tested on Linux. An attempted whole-CLI link against the older host dependencies was incompatible and is not counted as evidence.

The public API returned eight actual records (class, interface, constructor, method, field and parameter kinds). Before projection each declaration path existed. After projection, all eight tags for the invalid filename, invalid directory, outside-root path and resolved alias named nonexistent paths; none returned an unsupported-entry result. In the replacement-character collision case, all eight pointed at the existing wrong file. Positive controls returned eight correct paths and zero skips. A second isolated probe compiled the exact current writer on the native Windows host against its existing tree-hugger library, created names through `OsStringExt::from_wide`, and verified filename, directory and root-only outcomes. Both probes used their real public source/query/path APIs; neither is a whole-package OS receipt.

The essential reproduction uses native bytes rather than a Rust Unicode escape:

```rust
use std::ffi::OsString;
use std::os::unix::ffi::OsStringExt;
use tree_hugger::{SourceFile, SymbolQuery, portable_relative};

let native = root.join(OsString::from_vec(b"bad\xff.java".to_vec()));
// Copy the existing Java fixture to `native`, then query it.
let result = SourceFile::read(&native)?.query(&SymbolQuery::new())?;
let record = &result.index.symbols[0];
assert!(record.source.file_path.exists());
let projected = portable_relative(&record.source.file_path, &root);
assert_eq!(projected, "bad�.java");
assert!(!root.join(projected).exists());
```

The last assertion uses the no-collision fixture. The collision control instead creates `bad�.java` with different source and checks that its canonical path differs from the actual declaration path. The current writer emits the projected spelling in both cases.

| Child ID | Site / shape tested | Observed result | Expected result | Verification evidence |
|---|---|---|---|---|
| R2-F1.P1 | Declaration filename contains byte `FF` | Eight tags to nonexistent `bad�.java`; zero rejected entries | Reject unrepresentable tags, warn, exit 1; retain unrelated valid entries | Linux fixture/API/current-writer probe |
| R2-F1.P2 | Parent directory contains byte `FF`; filename is ordinary | Eight tags to nonexistent `bad�/Example.java` | Same explicit unsupported-entry behavior | Same probe, directory-only edit |
| R2-F1.P3 | Outside-root declaration filename contains byte `FF` | Eight nonexistent `../outside�.java` paths accepted | Validate the native relative route before writing | Same probe, outside-root edit |
| R2-F1.P4 | Lossy replacement filename already exists with different source | Eight tags identify the wrong existing file | Never accept a different declaration file | Same probe with `bad�.java` collision; canonical identities compared |
| R2-F1.P5 | Validly named symlink resolves to a non-UTF-8 declaration filename | Source API resolves the target; eight invalid target paths accepted | Validate the recorded native declaration route, including resolved targets | Same probe using `alias.java` |
| R2-F1.P6 | Native Windows filename contains an unpaired U+D800 surrogate | Eight real records; eight tags to nonexistent replacement-character filenames; zero rejected entries | Reject unrepresentable tags, warn and exit 1 | Native Windows filesystem/public-API/current-writer probe |
| R2-F1.P7 | Native Windows parent directory contains an unpaired U+D800 surrogate | Same eight wrong paths accepted | Validate the native relative route before writing | Same probe, directory-only edit |
| R2-F1.C7 | Unpaired surrogate occurs only in the native Windows scan root | Eight correct relative tags; no skips | Retain the representable `Example.java` route | Same probe, root-only control |
| R2-F1.C1 | Non-UTF-8 bytes occur only in the scan root; relative declaration path is `Example.java` | Eight correct tags, no skips | Keep this provably valid relative path; do not reject merely because the absolute path is non-UTF-8 | Same probe, scan-root-only edit |
| R2-F1.C2 | Actual filename contains the valid UTF-8 encoding of U+FFFD | Eight correct tags, no skips | Accept the actual Unicode filename; do not ban the replacement character itself | Same probe, valid-Unicode control |
| R2-F1.C3 | Names and container values | These are UTF-8 Strings; backslashes written raw; controls and pseudo-tags rejected | Preserve exact recorded names/owners, retain current unsupported policies | Current type invariant, byte corpus and fresh Neovim navigation |
| R2-F1.C4 | TSV and Markdown declaration labels | Use existing display projection; no editor-navigation promise | Preserve their specified display-path contract; avoid expanding this repair into an unrelated schema change | TSV explicitly uses existing conversion; Markdown generates no file links |
| R2-F1.C5 | Prose source-location href | Native record paths reach the URL helper before display projection | Preserve the existing URL boundary; never fabricate a different target | `write_listing`/`write_tree` use native records; on Unix `Url::from_file_path` percent-encodes raw component bytes. On Windows an unpaired surrogate makes that helper return None, so no incorrect href is emitted; `file_url` explicitly permits unrepresentable paths to have no URL. |
| R2-F1.C6 | Synthetic/pathless tags | Omitted before path checks | Continue omission without inventing navigation | `ctags::write_record` early return and fresh prelude regression |

#### Required consumers and decision trace

| Child ID | Consumer / decision | State and required repair |
|---|---|---|
| R2-F1.D1 | Native discovery, source read, query results | Keep native declaration paths available. The Linux API reproduction proves the file is readable and its declaration records are real. |
| R2-F1.D2 | `functions` export | Function/method/constructor kind selection changes membership, not path representability; every selected record reaches the same lossy projection. Assert skip/warn/status through the CLI. |
| R2-F1.D3 | `types` export | Class/interface and other selected type records reach the same projection. Assert the same outcome; do not treat container records as safe. |
| R2-F1.D4 | `symbols` export | Default item/member scope and explicit broader scopes still use the same path projection. Test default and a broader scope, with independent selected-name expectations. |
| R2-F1.D5 | `classes` container projection | Every selected container is trimmed before flattening. Preserve the native path validation for its tag. |
| R2-F1.D6 | `classes` displayed-member projection | Members are separately trimmed before flattening; static/instance filtering must still retain exactly the eligible safe records and reject unsupported ones. |
| R2-F1.D7 | Flattening, sorting, ID deduplication | These operations currently preserve membership/IDs and cannot repair lost bytes. Carry representability without changing their selection or order. |
| R2-F1.D8 | Tags unsupported-value decisions and report status | Detect unsupported path encoding before any lossy conversion; return a skipped-entry reason so the existing warning/error accumulation yields exit 1. Header-only and mixed-safe-file reports need exact assertions. |
| R2-F1.D9 | Neovim lookup/navigation | A replacement spelling can locate the wrong file. Add a Linux headless reader or exact native-identity check demonstrating that unsupported entries never reach navigation; retain valid-Unicode and root-only controls. |

**Repair:** preserve enough native path information to validate the *emitted relative route* before display projection discards bytes. Under the existing UTF-8 tags contract, omit an unrepresentable path with a visible, inert diagnostic and exit 1; keep safe entries, ordering and IDs unchanged. Do not detect this by searching the projected string for `�`: that would reject a legitimate Unicode filename. Do not simply reject a non-UTF-8 absolute path: the scan-root-only control has a valid relative route and must stay navigable. No lossy path may become a successful tag.

Add registered Linux tests starting from the copied fixture for all failing shapes and both positive controls, plus native Windows tests using `OsStringExt::from_wide` for filename, parent directory and scan-root-only cases. Sweep both class container and member projections and the three ordinary declaration commands. Update the accepted filename boundary in the CLI README, current symbol-report documentation and tree-hugger skill. This is an unsupported-value repair under the already accepted tags contract, not a request for a new editor format or a new approval.

## Blocked Findings

None. R2-F1 is executable repair work; neither human review nor cross-OS receipt collection blocks it.

## Recurrence

R2-F1 repeats the path-identity class of **review-1.md, R1-F6 — Tags encoding changes real filenames and qualified names**. It is a **newly discovered sibling**, not an unchanged earlier reproduction and not a repair regression. PHP backslashes and Unix literal-backslash paths now pass exact reader checks.

The first review enumerated the writer's backslash replacement and name/container escaping but missed `route_text`'s lossy native-byte conversion, the ordinary-record and class-member projection boundaries, and navigation when a replacement-character sibling exists. The repair removed writer-level separator rewriting but still receives a preprojected path. This review carries Linux byte and Windows unpaired-surrogate filename/directory instances, outside-root, resolved-alias and existing-file collision instances, all command/container/member consumers, and the controls that prevent an overbroad rejection from losing valid results.

## Input Robustness Matrix

Not applicable: this feature adds output serializers, not a manifest/configuration/file-format reader. The adapters parse inline markup generated by the existing renderer; they do not introduce an external document loader or load-bearing configuration fields. Source literals, native path representation, output structure and stderr boundaries were tested separately above.

## Requirement-to-verification map

| User-facing requirement | Appropriate level and current evidence | Assessment |
|---|---|---|
| Format choices, conflicting flags, comments/signatures/tree eligibility | L1 resolver and shipped CLI tests | Passing |
| TSV eight cells, reversible escapes, coordinates, alias paths, signatures and flattened ordering | L1 byte/decoder/location/order assertions | Passing |
| Markdown list nesting, class sections, groups, empty selections, relations and semantic emphasis | L1 parsed CommonMark event expectations | Passing repaired cases |
| Prose literal escaping, full content, layout and both plain comparisons | L1 public Prose rendering and CLI tests across commands/modes/groups | Passing repaired cases |
| Source-local URLs including spaces/Unicode/parentheses/native Windows paths | L1 URL tests and exact cross-file alias href assertions | Passing implemented checks; OS receipts are not readiness blockers |
| No source-derived controls or markup, including filename LF and doc backtick runs | L1 raw-byte assertions and parsed/rendered hostile-input probes | Passing tested cases; additional review corpus includes backtick lengths 1–8 in symbols/classes and both Prose presentations |
| Worker/cache/width stability and no extra analysis | L1 four-format stability test and uncached work counters | Passing |
| Partial failure retains successful declarations and separates stderr | L1 CLI tests plus fresh all-format mixed-file probe | Passing |
| Closed-pipe/ordinary write failure status and stopping work | L1 status-helper/shared-stream tests plus fresh actual all-format writer probes | Passing review probes: exit 0/1 as specified, four parses out of forty files in both cases |
| Tags duplicates, kinds, owners, omission and per-file ordering | L1 exact bytes and L2 Neovim lookup/navigation | Original required cases passing; native names that are not valid Unicode fail, R2-F1 |
| Terminal rendering after shared renderer extraction | L2 real tmux/WezTerm checks, plus unchanged L1 goldens | Passing executed backends |
| Keyboard, IME, mouse, or modifier encoding | No new requirement | L3 not applicable |
| Declared targets, required features and live tiers | Cargo/module inspection and tier audit | Zero stranded tests |

No production-readiness claim is based on test count. The comparisons and independently stated expected structure, locations, ordering and file identities above identify what the checks actually prove.

## Executed checks

- `just test export --test-threads 4`: **89 passed**. Includes the document/control/location/order regressions, export byte checks, work-counter tests and cache/worker/width stability.
- `just test-l2 level2_ctags_reader`: **1 passed**. Actual file/cursor jumps and deliberate wrong-expectation failures executed headlessly.
- `just test-l2`: **7 reported passed**. tmux and WezTerm checks executed; Kitty checks finished in about 0.01 seconds and are treated as availability-gated skips, not rendering evidence. No focus request was made.
- `just lint`: **passed** for both packages.
- `just check-tier-coverage tree-hugger-cli`: **zero stranded tests**.
- Full `just test --test-threads 4`: **2,024 passed**, zero skipped, in 384 seconds.
- Additional copied-fixture probes: literal hostile docs with backtick runs 1–8; all four formats with a closed pipe and an ordinary file-write limit failure; successful declarations retained alongside invalid UTF-8 source contents.
- Linux native-path/API/current-writer probe: failing filename/directory/outside/alias/collision cases and passing Unicode/root-only controls, as detailed in R2-F1. The native Windows probe also reproduced filename/directory failures and the valid root-only control. Remote temporary files were removed automatically. This is targeted defect evidence, not a cross-OS suite receipt.

## Nonblocking recommendations

Keep the actual write-failure/stop-work probes and expanded backtick corpus as permanent focused regressions when next editing those tests. Existing export write-error tests directly exercise the status helper, and the shared pipe tests exercise existing report modes; permanent all-export writer coverage would preserve the positive review evidence more directly. This recommendation does not add a second readiness finding: the fresh public CLI probes verified those behaviors successfully.

The accepted skip policy for paths Neovim expands, including bracketed filenames, remains within the specification's skip-or-prove boundary. This review does not reopen that accepted design or require human reaffirmation.
