---
$schema: feature-review.yaml
ready: false
findings:
  - title: Cached records bypass schema compatibility validation
    priority: high
  - title: Full JSON indexes bypass declaration ordering
    priority: medium
  - title: JSON path projection leaves absolute paths in reports
    priority: medium
  - title: File spelling splits cache identity and prelude reuse
    priority: medium
  - title: Default terminal reports retain every result before writing
    priority: medium
  - title: Declaration styling lacks real-terminal verification
    priority: high
human_review: false
has_blocked_findings: false
blocked: false
reviewed_by: codex/gpt-6.1-sol
recurrence: false
created: "2026-10-04T23:10:15-07:00"
spec: 2026-10-03-symbol-improvements/spec.md
implemented: true
implemented_by: claude/opus
log: tree-hugger/features/2026-10-03-symbol-improvements/log.md
description: "A **feature** review of `2026-10-03-symbol-improvements/spec.md`"
feature: 2026-10-03-symbol-improvements/review-1.md
next: 2026-10-03-symbol-improvements/review-2.md
---

The feature is **not production ready**. Six findings remain. None requires a human decision or is blocked. This review changes only this report and the specification's review count; it does not repair the implementation.

## Scope and verification

I read the specification, implementation hand-off and recorded departures, then reviewed the selective analysis pipeline, source snapshots, persistent cache, header extraction, prelude resolution, ordering, JSON projection, renderer, documentation, and test registration. I used the tree-hugger, rust-testing, and OS skills.

| Check | Result |
|---|---|
| `just test` in `tree-hugger/` | 797 passed, 0 skipped |
| `just lint` in `tree-hugger/` | Passed for both packages |
| `just test-l2` in `tree-hugger/` | 3 passed; tmux and WezTerm exercised `god-files`; the Kitty test returned when unavailable |
| `just check-tier-coverage tree-hugger` | 0 stranded tests |
| Additional shipped-CLI probes | Reproduced the five implementation defects below in isolated scratch repositories |

CLI probes used the workspace's `target/debug/hug`, static shipped fixtures, test-owned home/cache directories, `GIT_TERMINAL_PROMPT=0`, and two Rayon workers. They did not edit implementation files. The source fixture tree was `tree-hugger/lib/tests/fixtures/symbols/multi/`; prelude probes copied `tree-hugger/cli/tests/fixtures/prelude_pkg/`.

Cross-OS proof is left to CI and does not affect this readiness assessment. The performance measurements in the implementation log are supporting evidence, not timing guarantees; the reported larger-tree JSON size is not a blocker because the target was met on its original baseline input.

## Findings

### High — Cached records bypass schema compatibility validation

**Defect class:** a persisted reader validates the enclosing schema version but accepts incompatible versions on the records it serves.

In the tree-hugger package, [StoredEntry::validate](../../lib/src/cache/symbol_cache.rs) decides whether cached analysis can answer a query. It checks the entry's version against 4.0 but never checks `records[].schema_version`. The tree-hugger-cli package then emits those records unchanged inside a version-4 envelope. This violates the requirement that incompatible entries become misses and makes the envelope an unreliable description of its contents.

**Reproduction:** copy the 16-language `multi/` tree to a scratch Git repository and run `hug --json-full symbols` with an isolated cache. In each written entry, change only the first record's `schema_version` from `{"major":4,"minor":0}` to `{"major":3,"minor":0}`. Run both `hug --json symbols` and `hug --json-full symbols`. Both succeed and serve the version-3 records under a version-4 report/index.

| Site | Shape tested | Observed result | Expected result |
|---|---|---|---|
| Entry schema reader | Entry version changed to 3.0 | Miss; regenerated 4.0 records | Same; clean |
| Record schema reader, Bash, Zsh, Lua, Perl | First stored record changed to 3.0 | Served in both JSON modes | Miss; regenerate the entry |
| Record schema reader, C, C++, C#, Java | Same edit | Served in both JSON modes | Miss; regenerate the entry |
| Record schema reader, Rust, Go, Swift, Scala | Same edit | Served in both JSON modes | Miss; regenerate the entry |
| Record schema reader, Python, PHP, JavaScript, TypeScript | Same edit | Served in both JSON modes | Miss; regenerate the entry |
| Fresh extraction and synthetic prelude construction | Unedited source/cache controls | Records use 4.0 | Same; clean |
| Default and full report projections | A served incompatible record | Preserve its 3.0 version while the enclosing version is 4.0 | Receive only compatible cached records |

The entire schema-field robustness row was also exercised through `hug --json-full symbols` on the real Python fixture. Every edit started from the original CLI-written entry:

| Shape | Entry `schema_version` | Record `schema_version` | Expected |
|---|---|---|---|
| Positive control, 4.0 | Served | Served | Served |
| Absent | Recomputed | Recomputed | Recomputed |
| Explicit null | Recomputed | Recomputed | Recomputed |
| Wrong whole-field type, `123` | Recomputed | Recomputed | Recomputed |
| One wrong member, `minor: "bad"` | Recomputed | Recomputed | Recomputed |
| Every member wrong | Recomputed | Recomputed | Recomputed |
| Empty object | Recomputed | Recomputed | Recomputed |
| Duplicate field key | Recomputed | Recomputed | Recomputed |
| Trailing invalid document content | Recomputed | Recomputed | Recomputed |
| Well-typed incompatible version, 3.0 | Recomputed | **Served** | Recomputed |

**Repair and regression check:** validate each stored record's schema before serving or widening an entry. Extend the existing public-output cache matrix with the record version, including incompatible major/minor values. The CLI [cache-upgrade test](../../cli/tests/symbols/cache_upgrade.rs) currently constructs version-2 records under a current entry and explicitly asserts that this payload is served. Keep its key-invalidation control, but use a compatible renamed record for that control so the test no longer requires incompatible payloads to be accepted. Imports and exports have no independent schema-version field; the entry and declaration records are the two persisted version-bearing sites.

### Medium — Full JSON indexes bypass declaration ordering

**Defect class:** one output branch serializes extraction order instead of applying the shared presentation comparator.

In tree-hugger-cli, [write_json](../../cli/src/declarations.rs) applies ordering through `json_file_entry` for default JSON, but its full-index branch only projects paths and serializes the index. Consequently, all four commands accept `--sort` while ignoring it under `--json-full`. Even default file/source order misses the specified tie-breakers in C and Go.

**Reproduction:** copy the unedited 16-language `multi/` fixture tree and run each command with both JSON modes and each of `--sort file`, `--sort name`, and `--sort kind`. Compare each file's `symbols` array with the documented name/kind key followed by start byte, end byte, kind rank, and ID. Keep file entries in path order.

| Site | Shape tested | Observed result | Expected result |
|---|---|---|---|
| `functions --json` | All three orders, 16-language tree | Correct selected-record order | Same; clean |
| `types --json` | Same | Correct selected-record order | Same; clean |
| `symbols --json` | Same | Correct selected-record order | Same; clean |
| `classes --json`, container array | Same | Correct container order | Same; clean |
| Each of `functions`, `types`, `symbols`, `classes` with `--json-full --sort name` | Same tree | Every language has at least one array out of order | Sort every index's records by name and tie-breakers |
| Each of those four commands with `--json-full --sort kind` | Same tree | Every language has at least one array out of order | Sort every index's records by kind and tie-breakers |
| Each of those four commands with `--json-full --sort file` | Same tree | C and Go violate record tie-breakers; the other 14 languages pass | Apply all tie-breakers |
| File-entry order, both JSON modes | All commands/orders | Remains in portable path order | Same; clean |
| Ordinary terminal listings and file/directory groups | Existing order tests and independent two-file probe | Requested order applied | Same; clean |
| Class members | Source order | Matches the implementation log's intentional member-section policy | No change requested to that policy |

**Repair and regression check:** call the shared comparator on `index.symbols` before writing every full index. The existing [JSON ordering test](../../cli/tests/symbols/order.rs) checks record order for default JSON but checks only file-entry order for full JSON. Add the same record-order assertion to its full-index branch and cover equal-start spans so default tie-breakers cannot be skipped.

### Medium — JSON path projection leaves absolute paths in reports

**Defect class:** path projection assumes every path lies under the scan root and every stable key names the record's declaration file.

In tree-hugger-cli, [portable_relative and relativize_record](../../cli/src/report.rs) prepare machine-readable paths. Paths outside the root stay absolute. A resolved synthetic prelude record has a target declaration path but a stable key derived from the prelude file; the replacement searches for the target path and leaves the prelude's absolute path untouched. Both violate the single `root_dir` anchor contract.

**Reproduction:** use the real Rust `multi/rust.rs` fixture as a file beside the scratch repository, then run each declaration command with `--language rust /absolute/path/to/external.rs` in both JSON modes. Separately, copy the prelude package fixture and run `hug --json symbols --prelude --no-cache`.

| Site | Shape tested | Observed result | Expected result |
|---|---|---|---|
| Default JSON file entry, all four commands | Explicit absolute external Rust file | Absolute `files[].file` | Portable path relative to `root_dir`, such as `../external.rs` |
| Full JSON index, all four commands | Same input | Absolute `indexes[].file` | Root-relative portable path |
| Default/full declaration record, all four commands | Same input | Absolute `source.file_path` and stable-key path segment | Root-relative paths; unchanged IDs |
| Resolved prelude aliases `VerboseType` and `ElevatedEnum` | Unedited prelude fixture | `source.file_path` is `src/alpha.rs`, but stable key contains the absolute `src/prelude.rs` path | Relative target path and relative prelude key path |
| Ordinary records beneath the root, both modes | Unedited 16-language fixture tree | File path and stable-key segment projected | Same; clean |
| File-error paths, both modes and all four commands | Same external fixture with one invalid UTF-8 byte appended | Exit 1 and an absolute `errors[].file` | Exit 1 with a root-relative error path |

The two aliases demonstrate why replacing a key path by searching for `source.file_path` is insufficient: the key and the evidence location deliberately have different owners.

**Repair and regression check:** project external files relative to the root when the paths share a filesystem root, and explicitly define the representation when a Windows path is on another drive. Project the stable key's own file segment rather than assuming it equals `source.file_path`; preserve IDs. Extend the report/prelude tests to assert these paths and external-file error paths. The existing OS skill documents symlink and Windows path spelling concerns relevant to the implementation.

### Medium — File spelling splits cache identity and prelude reuse

**Defect class:** equivalent file spellings are retained as distinct analysis identities instead of normalized before cache and request-local lookup.

The tree-hugger package's [SourceFile::read_with_language](../../lib/src/file/source_file.rs) stores the supplied path verbatim, and [entry_key](../../lib/src/cache/symbol_cache.rs) only normalizes separators. In tree-hugger-cli, [prelude reuse](../../cli/src/declarations.rs) looks up that spelling in a map whose prelude resolver uses absolute paths. Forced-language explicit inputs can therefore bypass both reuse layers.

**Reproduction:** copy the prelude package fixture. Run `hug --json <command> --prelude --language rust src/alpha.rs --no-cache --verbose`, then repeat with the canonical absolute path to the same file. Independently, without `--prelude`, run cached `functions --language rust` with `src/alpha.rs`, its canonical absolute spelling, and `src/../src/alpha.rs` under one isolated cache.

| Site | Shape tested | Observed result | Expected result |
|---|---|---|---|
| Prelude reuse, `functions` | Relative explicit file plus forced language | 3 parses for 2 distinct files | 2 parses |
| Prelude reuse, `types` | Same | 3 parses | 2 parses |
| Prelude reuse, `symbols` | Same | 3 parses | 2 parses |
| Prelude reuse, `classes` | Same | 3 parses | 2 parses |
| Prelude reuse, each of the four commands | Canonical absolute spelling, same forced language | 2 parses | Same; clean |
| Prelude reuse without forced language | Ordinary explicit file goes through the root-based walker | 2 parses | Same; clean |
| Persistent cache key | Relative, canonical absolute, and `..` spellings; same content and forced language | Three misses and three entry files | First miss, then hits on one normalized identity |
| Snapshot hashing/parsing | Existing read-once snapshot test | Parsing uses the captured bytes | Same; clean |

On macOS, `/var` and `/private/var` also name the same temporary tree; the absolute control above used the canonical spelling so that alias did not disguise a failed control.

**Repair and regression check:** establish a consistent absolute file identity before prelude map lookup and cache key construction, with an explicit symlink policy. Preserve the existing I/O error contract and read-once bytes. Extend the prelude parse-count test to forced-language relative inputs and add a public cache-result test for equivalent spellings. This is the spec's normalized identity/no-duplicate-parse requirement, not a request to make IDs equal across different checkouts.

### Medium — Default terminal reports retain every result before writing

**Defect class:** a bounded parallel producer feeds an unbounded presentation accumulator, delaying output and discovery of write failures until the whole scan finishes.

In tree-hugger-cli, [write_terminal](../../cli/src/declarations.rs) retains all `FileListing` or class-summary results, then invokes [Renderer](../../cli/src/render.rs). Chunking limits worker results but not these accumulated records. Default file order and the file-oriented class report need not retain an entire repository. This defeats the bounded-output requirement and makes pipelines such as `hug symbols | head` perform the complete scan before noticing the closed reader.

**Reproduction:** copy the real `multi/nested/python.py` fixture into 96 separately named files. With two workers, start each command using `--no-cache --verbose`; read 16 stdout bytes, close the read end, and inspect the final work-counter line. The same input and closure procedure was used in every row.

| Site | Shape tested | Observed result | Expected result |
|---|---|---|---|
| `functions --plain`, default file order | Reader closes after 16 bytes | All 96 files parsed | Deliver bounded output and stop scheduling after failure is observed |
| `types --plain`, default file order | Same | All 96 files parsed | Same bounded behavior |
| `symbols --plain`, default file order | Same | All 96 files parsed | Same bounded behavior |
| `classes --plain`, default file order | Same | All 96 files parsed | Same bounded behavior |
| Default JSON, functions/symbols/classes | Same | 8 files parsed, then stops | Same; clean bounded chunks |
| Default JSON, types | Same | 16 files parsed, then stops as the buffered writer reaches the closed pipe | Same; clean bounded chunks |
| Full JSON, all four commands | Same | 8 files parsed, then stops | Same; clean bounded chunks |
| Exit handling, every mode/command | Same | Exit 0 without panic | Same; clean |

**Repair and regression check:** stream terminal output where the requested ordering permits it, retaining only the current chunk/group and any state needed for headings. Global name/kind ordering can legitimately require wider buffering; this finding concerns default file order. Extend the broken-pipe test with work counters and enough files to cross several chunks. Its current assertions prove quiet exit, not early termination or bounded retention.

### High — Declaration styling lacks real-terminal verification

**Defect class:** user-visible terminal rendering is verified only through manufactured subprocess output, while the real-terminal tests exercise a different renderer.

The spec requires the new tree-hugger-cli [Renderer](../../cli/src/render.rs) to preserve compact layout, styling, OSC 8 links, grouping, and plain behavior. Its golden and forced-color tests run at Level 1. The only Level 2 target is [level2_god_files](../../cli/tests/level2_god_files.rs), which exercises `god-files` rather than this renderer. Passing those tests does not verify the new component composition in a terminal.

| Site | Shape tested | Observed verification | Expected verification |
|---|---|---|---|
| `functions` renderer | Compact callable labels/signatures and locations | Level 1; no real-terminal declaration test | Level 2 capture for rendered layout/style |
| `types` renderer | Type labels and locations | Level 1 only | Level 2 capture |
| `symbols` renderer | Plain golden; forced-color subprocess comparison | Level 1 only | Level 2 capture for styling/grouping and long-line behavior |
| `classes` renderer | Static/instance sections, fields and methods | Level 1 only | Level 2 capture for section layout/style |
| Shared link/style composition | `Prose` in `UnorderedList`, capability-dependent OSC 8 | Code inspection and Level 1 output checks | Real-terminal run with appropriate text/style/link evidence |
| `--plain`, JSON selection/order, cache behavior | Hermetic subprocess/public-library tests | Level 1 | Same; appropriate and clean |
| `god-files` | Existing real-terminal report tests | Level 2 for its separate output path | Same; clean but unrelated to declaration rendering |

**Repair and regression check:** extend the live Level 2 target with a representative static fixture that exercises all four declaration commands, grouped symbols, class sections, styled locations, and a long unwrapped line. Capture through the existing terminal harness without stealing focus. Keep Level 1 escape/link assertions where pane-text capture cannot expose the full OSC 8 metadata; use terminal-supported evidence for rendered style. Declare the module/target and select it through the existing `test-l2` recipe. No keyboard-driven behavior is introduced, so Level 3 is unnecessary.

The spec's statement that the feature needs no real terminal is inconsistent with this review's explicit requirement that styling be verified at Level 2. Under the requested review standard, the current Level 1-only renderer proof is a high-severity coverage gap, not production readiness.

## Requirement-to-verification assessment

| Requirement group | Strongest relevant evidence | Assessment |
|---|---|---|
| Selective passes, prerequisites, metadata selection, one definition capture, honest recursion state | Level 1 public-library matrices and work counters | Passed existing coverage |
| Keywords and contiguous signature headers across all 16 languages | Level 1 header matrix, shape-coverage guard and adapter tests | Passed existing coverage |
| Read-once snapshots, cache coverage/widening, atomic writes and concurrent processes | Level 1 public-library and shipped-CLI tests | Passed except incompatible record validation and equivalent-path reuse |
| Versioned envelope, selection, complete indexes, errors and quiet stderr | Level 1 shipped-CLI tests and additional probes | Path projection remains incomplete |
| Source/name/kind ordering and grouping | Level 1 comparator/CLI tests and all-language independent checks | Full-index ordering remains incomplete |
| Prelude aliases, unresolved exports, environment names and targets outside a narrowed scan | Level 1 CLI tests and copied-package probes | Path identity reuse and synthetic-key projection remain incomplete |
| Broken pipes and bounded output | Level 1 subprocess closure and work-counter probes | Quiet status is correct; default terminal scheduling/retention is incomplete |
| Styled declaration layout and links | Level 1 goldens/forced-color tests | Needs Level 2 as described above |
| Performance sample and documentation | Recorded release measurements and documentation inspection | Measurements recorded; path/order docs currently overstate implementation behavior |

## Test placement and input robustness

The library's explicit `tests/l1/main.rs` declares the feature's header, query, cache, and coverage modules. Cargo discovers the CLI `symbols` target through `tests/symbols/main.rs`, which declares its feature modules. Their ordinary names are selected by L1. The Level 2 target has a live recipe and the declared `terminal-tests` feature. No stranded or undeclared new test was found.

The changed persisted format is JSON symbol-cache data; source-language grammars are not manifest/configuration loaders. The existing CLI-written-fixture matrix passed for its declared entry/record columns. Its allowed absent/default record values are explicitly documented in `docs/cache-design.md`; this review does not mistake those documented schema defaults for a defect. The record-schema compatibility row above was missing and exposes a real defect. When extending the matrix, also account for the entry's `file`, `language`, `imports`, `exports`, and `diagnostics`: they are required fields in the reader but are absent from the existing matrix table/test columns.

## Departures and closure

Reusing `visibility.is_synthetic` is a reasonable alternative to adding another marker. The claimed v1 recursion adapter has no corresponding v1 field, so no adapter repair is requested. Keeping kind labels for callables and keeping class member sections in source order are documented layout choices; this review does not reopen them. Re-parsing for missing reference/diagnostic capabilities is recorded as a limitation; the existing projection and widening tests passed.

This is implementation review 1 for this feature, so `recurrence` is false. Earlier reviews of the separate visibility/kind fix were consulted for context, but none of this feature's six findings is an unresolved instance of their extraction/linkage classes. Implementation is ready for another review after the six findings and their sibling regression checks are addressed. The feature directory remains in place for the author to close later.
