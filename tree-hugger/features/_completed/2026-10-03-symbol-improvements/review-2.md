---
$schema: feature-review.yaml
ready: false
findings:
  - title: Real-terminal assertions can inspect an earlier command's output
    priority: high
  - title: Package heading detection still buffers the entire first package
    priority: medium
human_review: false
has_blocked_findings: false
blocked: false
reviewed_by: codex/gpt-6.1-sol
recurrence: true
created: "2026-10-05T00:50:05-07:00"
spec: 2026-10-03-symbol-improvements/spec.md
log: tree-hugger/features/2026-10-03-symbol-improvements/log.md
implemented: true
implemented_by: claude/opus
description: "A **feature** review of `2026-10-03-symbol-improvements/spec.md`"
feature: 2026-10-03-symbol-improvements/review-2.md
previous: 2026-10-03-symbol-improvements/review-1.md
next: 2026-10-03-symbol-improvements/review-3.md
---

The feature is **not production ready**. Four previous findings are resolved; two are only partially resolved. Two unblocked findings remain below. No human decision is required to investigate or repair them.

## Previous review follow-through

The previous review has a `Findings` section rather than separate `Unblocked Findings` and `Blocked Findings` sections. Its six findings were all unblocked, and its frontmatter explicitly says that none was blocked. There were consequently no blocked findings to become unblocked before this implementation.

| Previous finding | Assessment in this review | Evidence |
|---|---|---|
| Cached records bypass schema compatibility validation | Resolved | Entry and individual record versions must equal 4.0; public-output robustness, upgrade, and same/wider-query regression tests pass. |
| Full JSON indexes bypass declaration ordering | Resolved | Full indexes now use the shared comparator; four-command/three-order tests, nested C/Go declarations, equal spans, and class member ordering pass. |
| JSON path projection leaves absolute paths in reports | Resolved for the documented path contract | External file/error paths, independent prelude-key paths, unchanged IDs, and symlinked directory spellings have passing regression tests. Another Windows drive/share deliberately retains an absolute portable path. |
| File spelling splits cache identity and prelude reuse | Resolved | Shared file identity is established before analysis/reuse; equivalent spellings, symlink controls, one entry, one listing, and prelude parse counts pass. |
| Default terminal reports retain every result before writing | Partially resolved | Single-package file listings, classes, and JSON stream; late package-heading detection retains an arbitrarily large first package. See the second finding. |
| Declaration styling lacks real-terminal verification | Partially resolved | A registered Level 2 target now exists and passes on WezTerm, but its tmux test fails because assertions can read earlier reports. See the first finding. |

## Scope and verification

I used the tree-hugger, rust-testing, and OS skills. I read the specification, previous review, implementation log, repair log, and relevant source/tests for selective analysis, capture reuse, headers, cache coverage, source snapshots, prelude resolution, ordering, projections, streaming, and rendering. This review changes only the review documents and requested specification metadata.

| Check | Result |
|---|---|
| `just test` in `tree-hugger/` | 821 passed, 0 skipped |
| `just lint` in `tree-hugger/` | Passed for both packages |
| `just check-tier-coverage tree-hugger` | 0 stranded tests |
| `just test-l2` | Failed in `level2_declarations_in_tmux`; stopped before the remaining tests |
| `just test-l2 level2_declarations_in_tmux` | Independently reproduced the same failure |
| `just test-l2 level2_declarations_in_wezterm` | Passed |
| `just test-l2 level2_god_files` | tmux and WezTerm passed; Kitty returned through its unavailable-backend gate |
| Additional shipped-CLI probes | Reproduced retained-capture collisions and late package buffering in isolated scratch repositories |

The initial Level 2 run also reached the Kitty declaration test, which returned through its unavailable-backend gate. That is not Kitty rendering evidence. Cross-OS evidence is left to CI and is not a readiness blocker. No keyboard behavior is introduced, so Level 3 is unnecessary. The probes used `target/debug/hug`, copied static fixtures, isolated home/temp directories, two Rayon workers, and non-interactive Git initialization. The additional tmux pane was detached and removed afterward.

## Unblocked Findings

### High — Real-terminal assertions can inspect an earlier command's output

**Defect class:** real-terminal assertions search an unbounded captured pane instead of identifying the current command's report, so earlier matching text supplies the wrong evidence.

In tree-hugger-cli, [run_hug](../../cli/tests/level2_declarations.rs:109) runs five reports in one pane. Its completion marker confirms that the latest command finished, but the returned frame is not restricted to that command. [style_before](../../cli/tests/level2_declarations.rs:194), which checks the styling of a displayed label, returns the first escaped occurrence of the requested text anywhere in that frame. In the actual tmux recipe run, earlier `functions` and `types` output remained visible in the captured frame despite the attempted `clear`.

The test failed on `tools/long.py`: it found the previous function listing's dim file location, SGR 2, instead of the current grouped listing's bold heading, SGR 1. The captured output contains the correctly bold current heading. This is a test defect, not evidence that the renderer emitted the wrong style. It matters because the canonical Level 2 gate fails and the same unscoped searches can accept old output when the current report is broken.

**Reproduction:** run `just test-l2`, then `just test-l2 level2_declarations_in_tmux`. Both failed at the same heading check. To finish the sibling sweep beyond that first assertion, copy `cli/tests/fixtures/declarations/` into a scratch Git repository, run the same five shipped-CLI commands sequentially in a detached tmux pane retaining the earlier reports, capture through `tmux capture-pane -p -e`, and apply the test's first-occurrence style rule to every style check. All checks were evaluated, including the class checks the failing test never reaches.

| Site | Shape tested | Observed result | Expected result |
|---|---|---|---|
| Functions: visibility, callable label, signature, location, package heading | First report in the sequence | All five style checks pass | Same; clean for this control |
| Types: interface label, name/fields, location | Earlier functions report retained | All three checks pass | Evidence must belong to the current types report |
| Grouped symbols: `tools/long.py` heading | Earlier function location retained | **Finds dim SGR 2; fails although the current heading is bold** | Find the current heading's SGR 1 |
| Grouped symbols: parameter label and `[5:44]` location | Same capture | Both checks pass | Same styles, scoped to this report |
| Directory grouping: `(root)` heading | Earlier reports retained | Check passes | Current directory heading only |
| Classes: `geometry.ts` file heading | Earlier function/type locations retained | **Finds dim SGR 2 instead of bold SGR 1** | Current class-report file heading |
| Classes: `create(size: number)` method | Earlier compact function signature retained | **Finds bold SGR 1 instead of green SGR 32** | Current class method's green style |
| Classes: language, container label/name, section title, member location, field | Same capture | Remaining six checks pass | Same styles, scoped to this report |
| Ordered row checks and long-line capture | Whole pane, same shared runner | Search space can include earlier reports; the row/long-line checks passed before the actual failure | Restrict all text checks to the current report too |
| OSC 8 assertions | Shared runner on WezTerm | Current recipe run passes; tmux does not retain OSC 8 for this assertion | Restrict link evidence to the current report |
| WezTerm backend | Unmodified declaration recipe | Full test passes | Preserve the passing behavior while fixing capture isolation |
| Kitty backend | Unavailable on this host | Gate returns without rendering | No execution claim; shared capture/assertion code needs the same repair |
| Existing `god-files` terminal tests | Separate report path | Available backends pass | Clean control; no renderer change requested here |

**Repair and regression check:** give every report unique begin and end markers and restrict both plain and styled captures to that region before any assertion. Alternatively, use a fresh pane per report with a verified empty capture; do not rely on `clear` succeeding without checking it. Sweep `assert_rows_in_order`, `style_before`/`assert_styled`, the independently captured long-line text, and the OSC 8 searches on all three backends. Match a heading or member by its row/context rather than an arbitrary first substring. Add a regression with earlier correctly styled and differently styled duplicates retained, proving the current report determines both success and failure. Run the live Level 2 recipe afterward.

### Medium — Package heading detection still buffers the entire first package

**Defect class:** a bounded parallel producer feeds a presentation buffer whose size depends on all declarations in a package, delaying writes and discovery of a closed output pipe until that package ends.

In tree-hugger-cli, [ListingStream](../../cli/src/render.rs:355) is the shared streaming renderer for functions, types, and symbols. With multiple discovered packages, `sections` starts undecided. [flush_unit](../../cli/src/render.rs:523) writes rendered bytes into `OpenPackage.held`, and [close_package](../../cli/src/render.rs:500) transfers them to `held`. They are not sent to stdout until a second package has matching declarations or the scan finishes.

Moving the accumulation from records to `Vec<u8>` does not bound it. The entire first package can be retained, and a pipe that has already closed is not noticed during that work. This remains true for default file order and `--group-by-file`, where individual files otherwise stream. If later packages have no selected declarations, the first package stays held to the end. The new documentation describes this exception, but it does not satisfy the specification's bounded file-order output requirement.

**Reproduction:** copy the shipped `multi/nested/python.py` fixture into 96 files under package `a`, then put one more copy under package `z`. Both directories have a minimal package `Cargo.toml`; the scratch root is a Git repository. Run each declaration command with `--no-cache --verbose`, two workers, and default file order. Read 16 stdout bytes, close the reader, and inspect the work counters. Repeat with the read end closed **before** spawning the process, which removes timing uncertainty. Finally replace `z/last.py` with a comment-only file and repeat the preclosed-pipe case. Each edit leaves the copied source fixture unchanged.

| Site | Shape tested | Observed result | Expected result |
|---|---|---|---|
| Functions, types, symbols: default terminal layout | Large first package, declaring second package; reader closes after 16 bytes | **All 97 files parsed** for each command | Deliver file-order output and discover the closed pipe without scanning the whole first package |
| Those three commands: `--group-by-file` | Same tree/closure | **97 parsed** for each | Same bounded behavior |
| Those three commands: `--group-by-module` | Same tree/closure | **97 parsed** for each | Package-section detection must not add whole-package buffering to directory grouping |
| Those three commands: both grouping flags | Same tree/closure | **97 parsed** for each | Same |
| Those three commands: default layout | Same tree; reader closed before spawn | **97 parsed** for each | Detect the first failed output write within bounded scheduled work |
| Those three commands: default layout | Second package changed to comment-only; preclosed reader | **97 parsed** for each | No whole-first-package retention while searching for a second declaring package |
| Those three commands: single selected package control | `a/*.py`, same preclosed reader | 8 parsed for each | Same; clean, one worker chunk |
| Classes terminal report | Same two-package tree | 16 parsed after reader closes; 8 with preclosed reader | Same; clean bounded producer |
| Default JSON: functions, symbols, classes | Same tree; reader closes after 16 bytes | 8 parsed | Same; clean |
| Default JSON: types | Same tree/closure | 16 parsed | Clean; buffered serialization notices closure in the next chunk |
| Full JSON: all four commands | Same tree/closure | 8 parsed for each | Same; clean |
| Quiet broken-pipe exit, all probed branches | Same closure procedures | Exit 0, no panic | Same; clean |
| Flat name/kind sorts and prelude projection | Source review of intentional whole-package ordering | Retain a package for global record ordering/projected locations | Distinct documented ordering requirement; not the default-order defect |
| Imports/lint and `god-files` | Source/log scope check | Separate output paths; imports/lint retain legacy buffering and god-files requires global ranking | Outside this feature's declaration-output repair; no new readiness finding for them |

**Repair and regression check:** remove the late heading decision from the file-order writer so a large first package cannot monopolize memory or postpone the first stdout write. Establish a heading policy that can be decided before writing the first package, and record any necessary layout departure in the implementation log and current report documentation. Spooling rendered bytes to disk alone would cap memory but would still miss the early-output/closed-pipe contract. Extend [the early-stop tests](../../cli/tests/symbols/report.rs:471) with an uneven package distribution: a first package spanning many chunks, followed by both a declaring and an empty package. Cover all three listing commands and all grouping combinations; keep classes and both JSON branches as controls. The current sectioned test gives every package exactly one chunk, which hides this case.

## Recurrence

The package-buffer finding repeats **review-1.md, “Default terminal reports retain every result before writing.”** That repair should have swept the default listings of functions/types/symbols with a large first package, file/directory/both grouping, and later packages with no selected declarations. It swept multiple packages only with evenly sized one-chunk packages. This review carries the full affected list and the clean classes/JSON controls, so the next repair can address the class together.

The prior “Declaration styling lacks real-terminal verification” requirement is also not fully closed. Its missing target was added, but the newly added capture-isolation defect prevents reliable completion of the tmux test. This is a new test defect explaining the incomplete closure, rather than another missing-target instance.

## Requirement-to-verification assessment

| User-facing requirement | Strongest relevant evidence | Assessment |
|---|---|---|
| Keywords and display headers across the accepted declarations in all 16 languages | Level 1 library matrices and coverage guard | Appropriate; pass |
| Selective kinds/passes, one capture per file, skipped diagnostics, recursion unknown | Level 1 public queries and work counters | Appropriate; pass |
| Cache identity, fingerprint invalidation, narrowing/widening, concurrent writes, malformed input | Level 1 public API and shipped-CLI cache matrices | Appropriate; pass |
| JSON selection/envelope/errors, source path projection, ordering and repeatability | Level 1 shipped-CLI tests | Appropriate; pass |
| Prelude aliases, target locations, unresolved exports, environment names, no duplicate parsing | Level 1 shipped-CLI tests | Appropriate; pass |
| Class selection and static/instance metadata | Level 1 library/CLI tests | Appropriate for data; pass |
| Styled declaration layout, grouped headings, locations, class sections, links and long rows | Level 2 WezTerm passes; tmux fails through unscoped evidence | High-severity reliability gap above |
| Plain text and escape-free output | Level 1 goldens and subprocess checks | Appropriate; pass |
| Bounded output and prompt detection of closed stdout | Level 1 pipe/work-counter tests and independent uneven-package probes | Existing tests miss the affected package shape |

Test registration is correct: library modules are declared in the consolidated L1 target; the CLI `symbols` target declares its new spelling module; both terminal targets are declared with `terminal-tests`, that feature is enabled in CI metadata, and `test-l2` is live. The defect is what the new Level 2 test asserts, not whether it is selected.

## Input robustness assessment

The changed persisted reader is the JSON symbol cache. Source-language grammars are not manifest/configuration loaders. The existing public-output matrix starts from a CLI-written Python fixture and restores it before each edit; its control sentinel proves whether the entry was served. The expanded columns now include entry `file`, `language`, `imports`, `exports`, and `diagnostics`, as requested previously, plus individual record schema versions. All matrix tests passed.

For both entry and record `schema_version`, the 4.0 control is served; absent, explicit null, wrong whole type, one wrong member, every member wrong, empty object, duplicate key, trailing invalid content, incompatible major, and incompatible minor are rejected and recomputed. The entry/record compatibility finding is therefore closed across the whole version row. Other matrix columns cover entry coverage/fingerprint/hash/record lists and record kind/container/visibility/modifiers/header/declaration text/relations/type-field data. Documented optional values and empty collections retain their declared schema meaning; malformed elements are rejected rather than silently filtered. No additional malformed-input finding was reproduced.

## Closure

The recorded departures concerning synthetic prelude markers, retained v1 APIs, unavailable v1 recursion fields, and partial-hit reparsing remain reasonable and are not reopened. Existing performance measurements support the performance goals, but do not excuse a reproducible unbounded output path. No implementation fix was made during this review. The feature remains in its active directory for the author to close after the review cycle.
