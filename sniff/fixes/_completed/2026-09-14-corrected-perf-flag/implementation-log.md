---
source_files_during_phase_1: []
docs_updated_during_phase_1:
  - sniff/fixes/corrected-perf-flag/spec.md
  - sniff/fixes/corrected-perf-flag/plan.md
docs_created_during_phase_1:
  - sniff/fixes/corrected-perf-flag/baseline-before.txt
  - sniff/fixes/corrected-perf-flag/implementation-log.md
skills_files_updated_during_phase_1: []
source_files_during_phase_2:
  - sniff/cli/src/output/perf_tree.rs
  - sniff/cli/src/output/mod.rs
docs_updated_during_phase_2:
  - sniff/fixes/corrected-perf-flag/plan.md
  - sniff/fixes/corrected-perf-flag/spec.md
  - sniff/fixes/corrected-perf-flag/implementation-log.md
docs_created_during_phase_2: []
skills_files_updated_during_phase_2: []
source_files_during_phase_3:
  - sniff/cli/src/output/perf_tree.rs
  - sniff/cli/src/output/render.rs
  - sniff/cli/tests/cli.rs
docs_updated_during_phase_3:
  - sniff/fixes/corrected-perf-flag/plan.md
  - sniff/fixes/corrected-perf-flag/implementation-log.md
docs_created_during_phase_3: []
skills_files_updated_during_phase_3: []
source_files_during_phase_4:
  - sniff/cli/tests/cli.rs
docs_updated_during_phase_4:
  - sniff/fixes/corrected-perf-flag/plan.md
  - sniff/fixes/corrected-perf-flag/spec.md
  - sniff/fixes/corrected-perf-flag/implementation-log.md
docs_created_during_phase_4:
  - sniff/fixes/corrected-perf-flag/baseline-after.txt
skills_files_updated_during_phase_4:
  - .claude/skills/sniff/cli.md
  - .claude/skills/os/build-hosts.md
source_code:
  - sniff/cli/src/output/perf_tree.rs
  - sniff/cli/src/output/mod.rs
  - sniff/cli/src/output/render.rs
  - sniff/cli/tests/cli.rs
documentation:
  - sniff/fixes/corrected-perf-flag/spec.md
  - sniff/fixes/corrected-perf-flag/plan.md
  - sniff/fixes/corrected-perf-flag/implementation-log.md
  - sniff/fixes/corrected-perf-flag/baseline-before.txt
  - sniff/fixes/corrected-perf-flag/baseline-after.txt
  - .claude/skills/sniff/cli.md
  - .claude/skills/os/build-hosts.md
completed_phase: "4"
implementation_1: "2026-09-15T19:13:09-07:00"
implementation_2: "2026-09-15T21:24:33-07:00"
implemented: true
packages:
  - sniff-cli
human_review: true
human_review_items:
  - >-
    UNCHANGED AND STILL OPEN — the S-1 scope ruling, the one decision this fix
    never obtained. Phase 1 reproduced the underscore/ellipsis corruption in
    `MetricsTree` and verified that the only working fix lives in
    `biscuit-terminal::build_markup` (escape the already-padded label, as
    `metrics_tree.rs:355` already does for the share cell). The plan's own
    remedy — escaping in Sniff's projection — is falsified: it makes the value
    column ragged at every width. Phase 1's full evidence, measurements, and
    three options are under "S-1 · Verdict" in the implementation log.
  - >-
    Phase 3 could not obtain that ruling (non-interactive session), so it took
    the branch the plan defines for a DECLINED ruling: `biscuit-terminal` is
    untouched, no projection-side escaping was added, and the required deferral
    comment now sits at the label-construction site in
    `perf_tree.rs::timing_node`. Phase 4 changed nothing here. This was a
    default, not a decision — the author should still rule. Consequence if it is
    never made: `sniff --perf` human output misaligns its value column and can
    inject italics on terminals narrower than 49 columns; above 48 columns it is
    clean at every width tested (20–140).
  - >-
    NEW in Phase 4, and needs a human: the Linux build host's cross-check lock
    is stale. `~/ci-verification/.cross-check.lock` on `$BUILD_LINUX` has been
    held since 2026-09-14T18:25:30Z by `{"purpose": "nightly-reward-spike",
    "owner": "reward-20260914-c3e60d0", "branch": "feat-nightly-perf"}`. Any
    `just cross-check ... --os all` or `--os linux` now blocks for 30 minutes
    and exits 75. Repo rule is that only the lock's owner removes it by hand, so
    it was left alone. Until someone clears it, native Linux has no local
    pre-push evidence path and CI's `ubuntu-latest` leg is its only proof — that
    is the one OS this fix could not exercise locally (native Windows and WSL2
    archive mode both ran green).
  - >-
    NEW in Phase 4, minor, non-blocking, and NOT fixed here: `scripts/
    cross-check.sh` reports `FAIL` for the `wsl` leg after a run whose own
    `cross-check-exit:` marker is 0 and whose every test passed. Verified on the
    guest: the archive run writes its JUnit report to
    `<clone>/target/nextest/ci/test-results.xml`, which is neither path
    `publish_wsl_receipt` searches, and the same fresh `target` makes the
    restoring `mv target.hold target` nest the warm cache at
    `target/target.hold` instead of restoring it — so every WSL cross-check also
    rebuilds from cold and leaves an orphaned cache copy on a host where disk
    matters. Out of this fix's scope (Rule 3; `scripts/` is not in the Affected
    Code table). Recorded in `.claude/skills/os/build-hosts.md` so it is not
    re-diagnosed. The exact source of the non-zero SSH exit was not pinned.
  - >-
    CARRIED FORWARD from Phase 3, minor: a pre-existing CLI test
    (`cli.rs::repo_aggregate_perf_covers_complete_command`) asserted the retired
    flat format on stderr and was rewritten to assert the same facts segment-wise
    against the tree. In scope per the spec's Affected Code table; flagged only
    because it changed an existing test rather than adding one.
message_to_agent: >-
  Phase 4 is complete and so is the plan. There is no Phase 5; the terminal
  state is "implementation complete, ready for review". Do not move this fix to
  `_completed` and do not run `just complete` — that is the author's call after
  the review cycle closes.

  What Phase 4 added: four L1 CLI contract tests in `sniff/cli/tests/cli.rs`
  (`perf_plain_output_is_ansi_free_and_hierarchical`,
  `perf_on_a_scriptable_text_command_stays_off_stdout`,
  `json_perf_stdout_is_exactly_one_document`,
  `counter_tree_reaches_the_human_report`) plus four shared helpers above them
  (`performance_section`, `metric_row`, `metric_value`, `metric_offset`). No
  production code changed in this phase.

  If you touch `--perf` rendering again, reuse those helpers rather than writing
  new string assertions. They encode the three rules that keep a CLI perf
  assertion portable: locate a row by whole whitespace cell (never substring),
  read its value as the cell AFTER the label (never the last cell — that is the
  share, which folds from an em dash to a hyphen without Unicode), and express
  hierarchy as an offset ordering rather than by naming a connector glyph.

  Gates: `cd sniff && just test` (2660 passed, 24 pre-existing skips) and
  `just lint` (exit 0, zero lints) are green, as is
  `cargo clippy -p sniff-cli --all-targets`. Native Windows and WSL2 archive-mode
  cross-checks both ran 46/46 green on the perf suite. Native Linux could NOT be
  exercised: the `$BUILD_LINUX` cross-check lock is stale since 2026-09-14 and
  only its owner may clear it (see `human_review_items`). CI's `ubuntu-latest`
  leg is that OS's proof.

  Still open and still the author's: the S-1 scope ruling from Phase 1. Nothing
  in the shipped code depends on the answer.
---

# Implementation Log — Corrected `--perf` Rendering

## Phase 1

Phase 1 is a rulings-and-spikes phase. It ships no production behavior; the
working tree contains no modification to any file under `sniff/lib`,
`sniff/cli`, or `biscuit-terminal` at the end of it.

### Rulings — verification results

Every ruling was checked against the `file:line` reference the plan cites.
Ten verified as written. Two needed a correction, recorded in `plan.md`.

| Ruling | Verdict | Evidence |
|---|---|---|
| R-1 | **Verified** | `collect_rows` overrides the root share unconditionally: `if is_root { MetricShare::Full.render(unicode) }` at `metrics_tree.rs:459-463`. A `Counters` root set to `Unknown` will still print `100%`. |
| R-2 | **Verified, and live** | `filesystem.shared_walk` is measured (241.48 ms) *and* parents `filesystem.shared_walk.docs` (1483.78 ms) on this host. Not hypothetical. |
| R-3 | **Verified, simplified** | Alias set `{os, hardware, network, filesystem}` confirmed against the live stage set. No live stage is named `detect.<domain>.<rest>`, so re-parenting collides with nothing. See correction 1 below. |
| R-4 | **Verified, with a caveat** | `detect.total` is present and must be suppressed. The live HOT winner is `filesystem.shared_walk.docs`, a depth-4 leaf — see correction 2 below. |
| R-5 | **Verified** | `terminal.rs:78`: `supports_unicode: crate::discovery::locale::env_says_utf8().unwrap_or(true)`. Locale-derived, with no reference to `is_tty`. CLI tests must not assert connector glyphs. |
| R-6 | **Verified** | `render_optimistic` hardcodes `self.build_markup(true, width)` at `metrics_tree.rs:574`; `render` passes `term.supports_unicode` at `metrics_tree.rs:583`. `worktree/cli/src/perf.rs:42` does indeed call `render_optimistic` — do not copy it. |
| R-7 | **Verified** | `build_markup` ends `body.trim_end().to_string()` (`metrics_tree.rs:381`); `emit_text`/`emit_stderr` use `print!`/`eprint!` with no newline of their own (`output/mod.rs:228-243`). The returned string owns its trailing `\n`. |
| R-8 | **Verified** | The label cap against available width is at `metrics_tree.rs:311-319`. `worktree` wraps in `BlockQuote` and pre-computes `term_width.saturating_sub(2)`; Sniff must not. |
| R-9 | **Verified as necessary** | Longest full stage key is `filesystem.file_inventory.classify.embedded_language_hint` (57 chars) plus a 9-char connector prefix at depth 4 — unusable as a label. Last-segment labels confirmed readable at width 80 (see rendered sample below). |
| R-10 | **Verified** | `Duration::from_secs_f64` panics on NaN/infinite/negative. A single defensive helper is required. |
| R-11 | **Verified** | `with_current_collector` has 49 references across `sniff` and `claudine`, spanning `sniff/lib/src/lib.rs`, `hardware/mod.rs`, CLI command paths, and the collection-completeness tests. Frozen. |
| R-12 | **Verified** | `sniff/cli/src/output/render.rs` is 179 lines today. The projection plus its unit tests will very likely push it past ~450, so plan on the `perf_tree.rs` escape hatch. `output/mod.rs:143` re-exports `render_performance_section`. |

#### Correction 1 — R-3 (applied to `plan.md`)

The ruling as written is correct. The clarification added: the domain test is a
plain `matches!` on the **first dotted segment**. `detect` is not a member of
the alias set, so `detect.*` keys can never re-parent and no extra
`!starts_with("detect.")` guard is needed. The redundant guard was removed from
the ruling's wording so Phase 2 does not implement a condition that cannot fire.

#### Correction 2 — R-4 (applied to `plan.md`)

R-4 is correct that HOT candidates are rendered, measured, non-root nodes. The
plan did not anticipate that the winner is a **repeated leaf whose accumulated
total exceeds wall-clock**: `filesystem.shared_walk.docs` accumulates 1483.78 ms
over 4286 calls against a 722.71 ms wall clock. Two consequences recorded in
the plan:

- HOT lands on a deeply nested leaf, not on a detection domain. Any test that
  assumes a domain-level HOT row is wrong.
- Shares legitimately exceed 1.0 and must not be clamped in the projection.
  `MetricShare::Of` caps *display* at 99% (`metrics_tree.rs:95`). The plan
  already said this; the baseline now proves it fires on every full run rather
  than being a defensive hypothetical.

### S-2 · Baseline capture — complete

`sniff/fixes/corrected-perf-flag/baseline-before.txt` holds the full capture.
Headline facts:

- `total_duration_ms` = 722.7065; **33 stage keys**, **34 counter keys**.
- Maximum dotted depth **4** (`filesystem.file_inventory.classify.*`).
- Longest single stage segment: `command_exists_in_path` (22 chars).
  Longest single counter segment: `classified_embedded_language_hint` (33).
  Longest full counter key: `filesystem.file_inventory.classified_embedded_language_hint` (59).
- `--json --perf` stdout is a single JSON document (5,607,808 bytes) with top
  keys `filesystem, hardware, network, os, performance`; the human report goes
  to stderr (4,072 bytes). The existing stream split is already correct.

**Gotcha worth carrying forward:** bare `sniff --perf` (no subcommand) prints
the clap help screen and emits **no performance section at all**. The plan's
S-2 step named exactly that invocation. Substituted `sniff os --perf` and
`sniff filesystem --perf` for the human-output captures; both are recorded.

The live key set confirms rather than contradicts R-2 and R-3: the
measured-node-that-is-also-a-prefix case is live, the unmeasured-intermediate
case is live, all four `detect.<domain>` stages are present on a full run, and
the `classify.extension` (stage) vs `classified_extension` (counter) near-miss
the spec anticipated is real — which is precisely what two separate trees (R3)
sidesteps.

### S-1 · Underscore/ellipsis markup spike — **REPRODUCED**

Method: a throwaway integration test (`sniff/cli/tests/perf_spike_tmp.rs`,
since deleted) projected the **real** S-2 stage and counter key sets into
`MetricsTree` with last-segment labels, and rendered every width from **20 to
140 inclusive**, in both `supports_unicode` modes, for both trees — 484
renders. Each render was checked for (a) an italic SGR `\x1b[3m`, and (b) a
share column that does not terminate at the same visible byte column on every
row.

**Result: the hazard is real and reachable with production data.**

Corruption occurred at 25 (tree, width, unicode) combinations, all in the
band **width 27–48**. Above 48 columns, output is clean. Both the timing tree
and the counter tree are affected, in both Unicode and ASCII modes.

Minimal deterministic reproduction — two labels each truncated immediately
after an `_`, at width 36:

```text
raw:   "\u{1b}[1mTotal                \u{1b}[0m  100.0\u{1b}[2mms\u{1b}[0m  100%\n
        ├─ aaaaaaaaaaaaaaaa\u{1b}[3m…   50.0\u{1b}[2mms\u{1b}[0m   50%\n
        └─ cccccccccccccccc\u{1b}[0m…   30.0\u{1b}[2mms\u{1b}[0m   30%"

plain: Total                  100.0ms  100%
       ├─ aaaaaaaaaaaaaaaa…   50.0ms   50%
       └─ cccccccccccccccc…   30.0ms   30%
```

The mechanism is exactly the one the plan predicted, and the damage is worse
than "styling only":

1. `truncate_to_width` (`metrics_tree.rs:554-565`) cuts the label at
   `label_w`, landing immediately after an `_`, and appends `…`.
2. `_` is now flanked by an alphanumeric on the left and `…` on the right.
   `is_word_neighbour` tests `char::is_alphanumeric` (`markdown.rs:30-32`), so
   `…` is not a word neighbour, `is_intra_word` is false, and the `_` becomes
   a **live italics opener** (`markdown.rs:397-416`).
3. `find_closing_single` (`markdown.rs:478+`) scans forward, correctly skipping
   intra-word underscores, until it finds the **next orphaned** `_` — which is
   the same truncation happening on a later row. It pairs across the row
   boundary.
4. Both underscores are **consumed** as delimiters. Each affected row therefore
   loses one visible character, so the padding computed from the pre-Prose
   string no longer holds and the value/share columns shift left by one.

So it is not merely a stray `<i>`: the column alignment the component exists to
guarantee is broken. Two orphaned underscores are required for a pair; a single
orphan is inert.

### S-1 · Verdict — the plan's proposed remedy does not work

The plan's contingency reads: "the fix belongs in Sniff's projection — escape
`_` in labels with a backslash, which Prose consumes invisibly and which is
width-neutral (the same technique `metrics_tree.rs:355` already uses for `<` in
`<1%`)."

**The width-neutrality claim is false for labels,** and the analogy to line 355
is what misleads. At line 355 the escape is applied *after* padding:

```rust
let share = format!("{:>share_w$}", row.share).replace('<', "\\<");
```

A projection-time label escape is applied *before* `MetricsTree` has done any
width math. `build_markup` measures `raw_label_w` from the caller's string
(`metrics_tree.rs:299`) and pads with `format!("{truncated:<label_w$}")`
(`metrics_tree.rs:324`) — both counting the backslash as a visible character.
Prose then deletes it (`tokens.rs:383-385` consumes `\_` to a literal `_`), and
the row ends up one column short per underscore.

Measured, same 484-render sweep, with `label.replace('_', "\\_")` applied in the
projection:

| Variant | Italic injection | Ragged value column |
|---|---|---|
| Unescaped (status quo) | 25 combinations, widths 27–48 | the same 25 |
| **Escaped in the projection** | **none** | **every width 28–140** |

Visual confirmation at width 120, escaped — each row is displaced left by its
own underscore count (`classified_binary_signature` by 2,
`files_classified_by_content` by 3):

```text
│  ├─ file_inventory                             32327     —
│  │  ├─ classified_binary_signature                1     —
│  │  ├─ classified_exact_filename                299     —
│  │  ├─ files_accepted                          10000     —
│  │  └─ files_classified_by_content            1551     —
```

The proposed remedy trades a defect confined to sub-49-column terminals for a
defect present at every width. It must not be implemented.

### S-1 · The fix that does work (prototyped, verified, reverted)

Escape the Prose-special characters in the **already-padded** label inside
`MetricsTree::build_markup` — i.e. do for the label precisely what line 355
already does for the share cell. Prototype:

```rust
// metrics_tree.rs, in build_markup
let truncated = truncate_to_width(&row.label, label_w, unicode);
let mut label = escape_prose(&format!("{truncated:<label_w$}"));
```

with `escape_prose` prefixing `\` to each of `\ _ * < > { [ ]`. Results:

- The 484-render sweep over the real Sniff key set: **zero** italic injections
  and **zero** ragged columns at every width 20–140, both Unicode modes, both
  trees — with Sniff's labels left completely unescaped.
- All **17** existing `metrics_tree` component tests pass unchanged
  (`cargo test -p biscuit-terminal --lib metrics_tree`).

This also closes a latent hole the component's own comment already gestures at:
`Glyphs::for_terminal` documents that its ASCII fallbacks "avoid every
Prose-special character (`<`, `>`, `{`, `*`, `_`, `[`, `]`, `(`, `)`, `\`) so
the folded markup never needs escaping" (`metrics_tree.rs:422-423`). The
component escaped its own glyphs and its own share cell but never the caller's
label. Sniff is simply the first consumer with underscore-heavy labels;
`worktree` and `claudine` use hyphenated and spaced labels and have never
exercised the path.

**The prototype has been reverted.** `git status` over `biscuit-terminal` and
`sniff/cli` is clean.

#### Options for the ruling (Phase 3 blocker)

1. **Fix it in `biscuit-terminal`** (recommended). ~10 lines plus a regression
   test. It is the component's own invariant — the component promises unit
   alignment and owns every other escape on the row. Cost: the plan's Phase 3
   checkpoint "`git diff --stat sniff/lib biscuit-terminal` is empty" and its
   Phase 4 equivalent must be amended to permit `biscuit-terminal/lib/src/
   components/metrics_tree.rs`. `sniff/lib` stays frozen either way, so R-11
   and spec R1 are untouched.
2. **Ship without a fix and accept it.** The corruption is confined to
   terminals narrower than 49 columns. Defensible, but it silently breaks the
   one guarantee the component is used for, and it leaves the trap armed for
   the next consumer.
3. **Work around it in Sniff** by shortening labels so truncation cannot land
   on an underscore. This means duplicating the component's width math in
   Sniff, which spec R4 and ruling R-8 both forbid, and it would only ever be
   approximate. Not recommended.

Recommendation: **option 1**, as a separate, clearly-scoped commit against
`biscuit-terminal` with its own regression test, kept out of the Sniff
projection commit.

### S-3 · Count/Unknown column sanity — **PASSED**

Folded into the S-1 harness. A `Count`-valued tree with `MetricShare::Unknown`
on every non-root node renders cleanly:

- The share column degrades correctly — `—` under Unicode, `-` under ASCII
  (`metrics_tree.rs:98`).
- `share_w` does not collapse the layout. With `Unknown` everywhere the share
  cell is one character wide and the root's `100%` is four, so `share_w = 4`
  and the em dashes right-align under the `%`. Alignment holds at every width
  tested (outside the S-1 band).
- Counts render with **no unit suffix** (`metrics_tree.rs:60`), so `unit_w`
  collapses to 0 and the count column sits flush against the label column.
- **R-1 confirmed empirically:** the `Counters` root is set to
  `MetricShare::Unknown` in the projection and the component still prints
  `100%` on it. This is the accepted, expected behavior. Not a bug.

Rendered sample (Unicode, width 120):

```text
Counters                                    63097674  100%
├─ filesystem                               63075702     —
│  ├─ docs                                      8572     —
│  │  └─ documents_parsed                       8572     —
│  ├─ file_inventory                           18094     —
│  │  ├─ classified_embedded_language_hint        14     —
│  │  ├─ classified_extension                   8080     —
│  │  └─ files_accepted                        10000     —
│  ├─ io                                    63035977     —
│  │  ├─ bytes_read                         63028365     —
│  │  └─ file_opens                             7612     —
```

And the timing tree under the same harness, ASCII mode, which also demonstrates
the R-5 fallback and the `×N` call counts:

```text
Total                                  722.7ms  100%
+- detect                                1.1s    99%
   +- filesystem                       721.3ms   99%
   |  +- file_inventory                  1.1s    99%
   |  |  +- classify                   578.9ms   80%
   |  |  |  +- embedded_language_hint    2.3ms   <1%  x14
   |  |  |  +- extension                 8.6ms    1%  x8080
   |  |  |  +- fallback                478.6ms   66%  x1604
   |  |  +- walk                       528.0ms   73%
   |  |     +- entry                   528.0ms   73%  x10000
   |  +- shared_walk                   241.5ms   33%
   |     +- docs                         1.5s    99%  x4286
```

This is the first direct evidence that the Overview's Definition of Success
item 1 is achievable as specified: the hierarchy is legible, units align, call
counts surface, and last-segment labels (R-9) read well at depth 4.

### Requirement-to-evidence mapping

Phase 1 changes no behavior, so there is no regression test to add. The
mapping below is spike-evidence to ruling, which is the phase's deliverable.

| Plan item | Evidence produced |
|---|---|
| R-1 … R-12 | Table above; each checked against the cited `file:line`. |
| R-2, R-3 structural assumptions | Live key set in `baseline-before.txt` confirms both. |
| S-1 | 484-render sweep, real key set, widths 20–140, both Unicode modes, both trees. Reproduced at 25 combinations; minimal repro recorded verbatim. |
| S-1 remedy | Same sweep re-run with projection-time escaping: remedy falsified. Same sweep re-run with component-side escaping: remedy verified, 17/17 component tests green. |
| S-2 | `baseline-before.txt`, plus the bare-`--perf`-is-help gotcha. |
| S-3 | Count/Unknown column behavior confirmed, R-1's `100%` root confirmed empirically. |

### Gates run

| Gate | Result |
|---|---|
| `cargo build -p sniff-cli` | green |
| `cargo test -p biscuit-terminal --lib metrics_tree` | green, 17 passed (both with and without the reverted prototype) |
| `cd sniff && just test` / `just lint` | **not run.** Phase 1 modifies no source; the plan schedules these for Phase 4 Work-Group 4.C. No production file differs from `HEAD`. |

No skipped or pre-existing failures were observed in what was run.

### Cross-platform note

Nothing in Phase 1 is platform-conditional. The S-1 finding is pure string
handling in `biscuit-terminal` and reproduces identically on any OS — the
truncation band depends only on terminal width and label content, never on the
host. `os` skill not loaded; no OS-specific claim is made, and no cross-check
run was warranted. The baseline itself is macOS-specific in its *values* (stage
durations, `hardware.audio` cost) but its *key set* is what Phase 2 consumes,
and the keys are platform-independent apart from detectors that simply do not
fire elsewhere.

### End state

Rulings recorded and two corrected. Both hazards retired: R-10's
`Duration::from_secs_f64` panic is confirmed and has a prescribed defensive
helper; S-1 is reproduced with a verified fix whose location needs a scope
ruling. Baseline captured. **No production file modified.**

---

## Phase 2

Phase 2 ships the report-to-tree projection as a pure function. Nothing in
production calls it yet — `render_performance_section` still emits the flat
lists it emitted at `HEAD`. That is the phase boundary the plan drew, and it is
what lets every assertion below be made against a returned `MetricNode` rather
than against rendered text.

### Where the code went, and why

R-12's escape hatch was taken: the projection lives in a new private module,
`sniff/cli/src/output/perf_tree.rs` (721 lines: 232 production, 489 tests),
declared as `mod perf_tree;` in `sniff/cli/src/output/mod.rs`. `render.rs` is
179 lines; folding the projection and its 20 tests in would have put it past
900, well beyond the ~450 threshold the ruling names. `render_performance_section`
stays the sole public seam and `output/mod.rs`'s re-export of it is untouched.

Two `pub(crate)` entry points are exposed:

| Symbol | Purpose |
|---|---|
| `timing_tree(&PerformanceReport) -> MetricNode` | The bare projection. Everything the unit tests assert against. |
| `timing_metrics_tree(&PerformanceReport) -> MetricsTree` | The same root wrapped with `OVERLAP_NOTE` attached via `with_notes`. Phase 3 renders this. |

Everything else — `PathNode<M>`, `StageFacts`, `stage_path`, `timing_node`,
`sorted_children`, `node_duration`, `share_of`, `ms_to_duration` — is private to
the module.

### Design decisions taken during implementation

**`PathNode<M>` is generic over the measured payload, as the plan asked.** The
counter tree in Phase 3 is `PathNode<u64>` and reuses `insert` and `get_mut`
unchanged. `sorted_children` and `node_duration` are *not* reusable as written:
both are hard-typed to `MetricValue::Duration` and `StageFacts`. That is
recorded in `message_to_agent`.

**HOT selection compares resolved `Duration`s, not raw `f64`s.** The plan says
"pick maximum `total_duration_ms`". Comparing the raw `f64` needs `total_cmp` to
be total, and `total_cmp` ranks `NaN` above every real measurement — so a single
malformed stage would silently win HOT. Comparing `ms_to_duration(…)` is
order-equivalent for every finite positive value, is already total, and collapses
`NaN`/negative/infinite to `Duration::ZERO`, which is exactly what those stages
render as. Tie-break is unchanged: full dotted stage name ascending.

**Domain re-parenting requires a non-empty `<rest>`.** R-3 specifies
`<domain>.<rest>`. Matching on the first segment alone would let a hypothetical
bare `filesystem` stage rewrite to `detect.filesystem` and overwrite the real
`detect.filesystem` measurement. Requiring `rest` to be non-empty is part of
matching the specified shape, not the redundant `!starts_with("detect.")` guard
Phase 1 removed — `detect` is still absent from the alias set and no such check
exists.

**`insert` refuses to write to the root.** A path whose segments are all empty
(`""`, `"."`) places nothing rather than overwriting the synthetic root's
payload. This is the "empty segments … handled without panic" clause of the
plan's builder task.

### 14 dead-code warnings were left in place, deliberately

`cargo build -p sniff-cli` emits 14 `dead_code` warnings, all naming symbols in
`perf_tree.rs`, because no production path reaches the module yet. `just lint`
exits 0 and reports zero clippy lints and zero errors, so the gate is green.

The alternative — a module-level `#[allow(dead_code)]` — was rejected. It would
silence the warnings for one phase and then survive indefinitely if Phase 3
forgot to remove it, hiding genuinely unreachable code from that point on. The
warnings are a compiler-maintained to-do list that wiring
`render_performance_section` erases in full. A warning that still fires after
Phase 3's wiring task is real signal.

### Requirement-to-test mapping

All tests are in `sniff/cli/src/output/perf_tree.rs::tests`. 20 added, 0 modified,
0 removed.

| Plan / spec requirement | Test |
|---|---|
| R-10 — defensive `f64 → Duration` | `ms_to_duration_collapses_non_finite_and_negative_input` |
| R-10 — malformed report never panics (negative/error behavior) | `a_malformed_report_projects_without_panicking` |
| R2.1.1–2 — synthetic `Total` root, `detect.total` suppressed | `detect_total_is_the_root_and_never_a_child` |
| R2.1.4 / R-3 — re-parent below an existing `detect.<domain>` | `a_domain_stage_reparents_below_an_existing_detect_branch` |
| R2.1.5 / R-3 — no synthetic `detect` when the branch is absent | `a_domain_stage_stays_generic_when_its_detect_branch_is_absent` |
| R2.1.3 / R-3 — alias set is exactly the four domains | `only_the_four_detection_domains_reparent` |
| R2.1.3 — a name with no `.` | `a_single_segment_stage_is_a_direct_child_of_the_root` |
| R2.1.6 / R-2 — measured parent keeps its value; synthetic sums *immediate* children | `a_measured_parent_keeps_its_own_duration_and_a_synthetic_parent_sums_children` |
| R2.2 / R-4 — share is of wall clock, unclamped | `shares_are_of_wall_clock_and_are_not_clamped` |
| R2.2 — zero-duration report, no non-finite share | `a_zero_duration_report_yields_unknown_shares_and_no_non_finite_values` |
| R2.1.8 — duration descending, then label ascending | `siblings_sort_by_duration_descending_then_label_ascending` |
| R2.1.7 — `×N` above one call, absent at one | `calls_surface_above_one_and_stay_absent_at_one` |
| R2.3 / R-4 — greatest measured non-root stage is HOT | `the_largest_measured_non_root_stage_is_the_sole_hot_node` |
| R2.3 / R-4 — synthetic intermediates are never candidates | `synthetic_intermediates_are_never_hot` |
| R-4 — ties break on the full dotted name | `an_equal_duration_tie_breaks_on_the_full_dotted_stage_name` |
| R2.3 — no stages ⇒ root only, no marker | `an_empty_stage_map_yields_a_bare_root_with_no_marker` |
| R2.3 — `detect.total`-only report ⇒ root only, no marker | `a_total_only_report_yields_a_bare_root_with_no_marker` |
| Plan 2.B — S-2 baseline key set projects correctly | `the_baseline_key_set_projects_to_the_expected_shape` |
| Plan 2.A — at most one `Highlight` anywhere | `at_most_one_hot_node_exists_for_every_projected_report` |
| R2.2 — overlap note reaches the output | `the_overlap_note_reaches_the_rendered_tree` |

Notes on how the harder cases were pinned:

- **The original reported input is present verbatim.** `baseline_stages()` is the
  complete 33-key live stage set from `baseline-before.txt`, with its real totals
  and call counts, including the three keys the spec's Problem Statement quotes.
  It is the corpus fixture for `the_baseline_key_set_projects_to_the_expected_shape`
  and for `at_most_one_hot_node_exists_for_every_projected_report`.
- **The tie-break test distinguishes the two candidate rules.** Stages `zeta.aaa`
  and `alpha.zzz` are given identical durations. Breaking on the *display segment*
  picks `aaa`; breaking on the *full dotted name* picks `alpha.zzz`, whose label is
  `zzz`. The test asserts `zzz`, so a regression to segment-ordering fails it.
  A same-value pair alone could not tell the two rules apart.
- **Synthetic sums are asserted to be immediate-children-only, not subtree.** The
  fixture puts a measured `walk` (100 ms) above a much larger measured
  `walk.entry` (500 ms) under a synthetic `file_inventory`. `file_inventory` must
  be 140 ms, not 640 ms; a subtree rollup fails.
- **Dependent outputs are asserted, not just the value under test.** The HOT tests
  check both `hot_labels(&tree)` (exactly one row, tree-wide) and the marker on
  the specific node. `detect_total_is_the_root_and_never_a_child` checks the
  root's label, `emphasize`, share, duration, *and* that no `total` node exists
  anywhere in the flattened tree. The malformed-report test walks every node
  asserting both finite durations and no non-finite `MetricShare::Of`.
- **Representation variants are covered:** measured vs. synthetic nodes, present
  vs. absent `detect.<domain>`, detection-domain vs. generic dotted names,
  single-segment vs. depth-5 names, `calls == 1` vs. `calls > 1`, zero vs.
  non-zero wall clock, shares below and above 1.0, and NaN / ±infinity /
  negative / zero / ordinary millisecond inputs.

### Verification level

The projection is arithmetic and string handling over an in-memory struct, so L1
unit coverage is the right level for 19 of the 20 tests — no filesystem,
subprocess, terminal, or persistence boundary is crossed by
`timing_tree`. `the_overlap_note_reaches_the_rendered_tree` does cross the
crate boundary into `biscuit-terminal`: it builds the real `MetricsTree`, renders
it through `TerminalRenderable::render` with an explicit 200-column terminal, and
asserts the note survives Prose's markup pass into visible output. It asserts no
connector, marker, or width — glyph assertions are Phase 3's, per the Phase 2
checkpoint. The CLI, stream-routing, and `--plain` boundaries are Phase 4's.

### Gates run

| Gate | Result |
|---|---|
| `cargo test -p sniff-cli --lib output::perf_tree` | green — 20 passed, 0 failed |
| `cargo test -p sniff-cli --lib output::render` | green — 6 passed, unchanged |
| `cd sniff && just test` | green — 2640 passed, 0 failed, 24 skipped |
| `cd sniff && just lint` | green — exit 0, 0 errors, 0 clippy lints, 14 transient `dead_code` warnings (above) |

The 24 skipped tests are nextest's pre-existing default-filter exclusions (L2 /
real-resource suites that `just test` does not select); they are unrelated to
this change and were skipped identically before it. Nothing in the Sniff package
area failed.

**One pre-existing failure was observed outside this package area,** and is
recorded here only because it was seen. A `just test` was accidentally issued
from the repository root rather than from `sniff/`, which runs the workspace-wide
L1 suite (29,930 tests, 74 packages) that the repo's test-scope discipline tells
us not to run for a Sniff-only change. It reported 4 failures, all in
`claudine-cli` and all shipped-prompt fixture-hash drift:

- `compose_caller_file_provenance::shipped_implement_router_keeps_the_callers_launch_origin_for_its_lazy_target`
- `shipped_prompt_route_drift::fixture_preserves_the_shipped_schema_and_loop_semantics`
- `shipped_prompt_route_drift::shipped_implement_prompts_have_not_drifted_from_their_fixture`
- `shipped_prompt_contract::shipped_prompts_have_parseable_schemas_and_expressions`

They are unrelated to this fix: the working tree contains no change under
`claudine/`, and the suite's own remediation hint is
`CLAUDINE_UPDATE_SHIPPED_PROMPT_HASHES=1`. Nothing was done about them; they
belong to whoever owns the Claudine prompt fixtures.

### Phase 2 validation checkpoint

- **`cargo test -p sniff-cli render::tests` green** — filter corrected, since
  R-12's escape hatch moved the code to `output::perf_tree`. Both modules run
  green; the plan checkpoint records the substitution.
- **`sniff/lib/` unmodified** — `git diff --stat sniff/lib biscuit-terminal` is
  empty. R-11 and spec R1 hold.
- **No test asserts on rendered glyphs** — the single rendering test asserts only
  the note's literal text.

### Cross-platform note

`perf_tree.rs` contains exactly one `cfg` attribute, `#[cfg(test)]` on the test
module. There is no `#[cfg(windows)]`, `#[cfg(unix)]`, or any other
platform-conditional code, and no test asserts a path separator, a
locale-dependent glyph, or a terminal width read from the environment — the one
rendering test builds its own 200-column `Terminal`. Under R-5,
`Terminal::default().supports_unicode` is locale-derived, so a `LANG=C` CI runner
would change connector glyphs; no Phase 2 test can see that. The macOS, Linux,
native-Windows, and WSL2 legs therefore see identical expectations and no
`just cross-check` run was warranted for this phase. The `os` skill was not
loaded and no OS-specific claim is made.

### End state

Projection complete and unit-tested; `render_performance_section` untouched;
`sniff/lib` and `biscuit-terminal` untouched. The S-1 scope ruling from Phase 1
remains the only open blocker, and it still gates only Phase 3's "Apply the S-1
verdict" task.

---

## Phase 3

Phase 3 makes the projection reachable. `render_performance_section` now emits
two trees instead of two bullet lists, the counter projection joins the timing
one, and the flat rendering is deleted. This is the first phase whose output a
user can see.

### What changed, and where

| File | Change |
|---|---|
| `sniff/cli/src/output/perf_tree.rs` | Added the counter projection (`counter_tree`, `counter_metrics_tree`, `counter_node`, `sorted_counter_children`, `sum_counts`, `node_count`) and the S-1 deferral comment. 11 tests added. |
| `sniff/cli/src/output/render.rs` | Rewrote `render_performance_section` over both trees; deleted the flat stage and counter blocks. 6 tests added. |
| `sniff/cli/tests/cli.rs` | Repaired `repo_aggregate_perf_covers_complete_command`, which asserted the retired flat format. |

`sniff/lib` and `biscuit-terminal` are untouched — `git diff --stat sniff/lib
biscuit-terminal` is empty. The signature of `render_performance_section` and
the `output/mod.rs` re-export are unchanged, and `sniff-cli` has no reverse
dependency in the workspace, so the public seam moved for nobody.

All **14** `dead_code` warnings Phase 2 left behind cleared themselves the
moment the wiring landed, exactly as Phase 2 predicted. `cargo build -p
sniff-cli` is warning-free; nothing in `perf_tree.rs` turned out to be
genuinely unreachable.

### Design decisions taken during implementation

**The counter tree got its own node/sort pair rather than a generalization of
the timing one.** `message_to_agent` offered both. `timing_node` and
`counter_node` differ in four places at once — how a measured payload becomes a
value, how a synthetic node aggregates, which decorations apply (`calls` and
`marker` are timing-only), and the sort key. A generic version would need three
closures and a trait bound to hide about thirty lines of body. `PathNode<M>`,
`insert`, and `get_mut` are reused verbatim, which is where the real duplication
would have been.

**A measured counter that is also a prefix keeps its own count**, the same rule
R-2 gives timing. The plan phrases R3 as "measured leaves … synthetic
intermediates sum their immediate children", which does not say what a measured
*non-leaf* does. Silently summing over it would discard a real measurement and
would diverge from the timing tree for no stated reason. Pinned by
`a_measured_counter_parent_keeps_its_count_and_a_synthetic_parent_sums_children`.

**`sum_counts` saturates.** Counter totals are `u64` and a synthetic parent adds
its children, so unlike the duration side the overflow input is representable
and easy to construct. A hand-built report reports a ceiling rather than
aborting a release build's wrapping arithmetic or panicking in debug.

**The counter tree carries no notes.** `OVERLAP_NOTE` is about durations
overlapping wall-clock; nothing in R3 asks for a counter note, and a second
italic line under a tree of `—` shares would be noise.

**The section assembles by trimming, not by counting newlines.** Each tree is
`trim_end()`ed, joined with a literal blank line, and the whole section gets one
`\n` pushed at the end. `MetricsTree` already trims its own body
(`metrics_tree.rs:381`) but `Prose` is free to change what it appends; trimming
at the seam means R-7 holds regardless.

### The S-1 verdict task — resolved by its declined branch, ruling still open

The plan marks "Apply the S-1 verdict" **BLOCKED pending a scope ruling** and
defines two branches. This session is non-interactive and could not obtain the
ruling, so it took the branch that does not widen scope without one:

- `biscuit-terminal` was not modified.
- No projection-side escaping was added (Phase 1 proved it strictly worse).
- The deferral comment the declined branch requires is now at the
  label-construction site, `perf_tree.rs::timing_node`, recording that
  underscore flanking was checked, why the projection-side escape is wrong, that
  the working fix is the component's, and that corruption is confined to
  terminals narrower than 49 columns.

Both Phase 3 checkpoints that require `git diff --stat sniff/lib
biscuit-terminal` to be empty therefore stand as written and pass. **This was a
default, not a decision** — `human_review_items` carries the ruling forward
unchanged, and it does not block Phase 4.

### A pre-existing CLI test asserted the retired format

`repo_aggregate_perf_covers_complete_command` (`cli.rs:346`) failed on the first
full `just test` after wiring. Its stderr assertions were
`stderr.contains("cli.repo.aggregate_projection")` and
`stderr.contains("git.repository_discoveries: 1")`. Neither string can exist in
a tree whose rows are labelled by the last dotted segment (R-9) and whose values
sit in their own aligned column — the failure is the intended contract change,
not a regression.

The test's *intent* — the human report on stderr must carry the post-detection
aggregate stage and the command-wide counter bounds — is still valid and still
worth pinning, so it was rewritten rather than deleted. It now asserts the
`aggregate_projection` label is present, and for each counter reads the
whitespace cell **immediately after the label cell**. That is deliberately not
"the last cell": the last cell is the share, which renders `—` under Unicode and
`-` under ASCII, so reading from the end would have made the test
locale-sensitive in violation of R-5. The connector glyph is never asserted for
the same reason. The JSON half of the test, which pins the same stage and
counters exactly, was not touched.

The spec's Affected Code table already lists `sniff/cli/tests/cli.rs`, so this
is in scope; it is called out because it modified an existing test.

### Requirement-to-test mapping

17 tests added, 1 rewritten, 0 removed. Phase 2's 20 projection tests all still
pass unmodified.

| Plan / spec requirement | Test | File |
|---|---|---|
| R3 — empty counter map omits the tree entirely | `an_empty_counter_map_produces_no_tree` | `perf_tree.rs` |
| R3 — dotted counter names nest generically | `nested_counter_names_nest_generically` | `perf_tree.rs` |
| R3 — no domain aliasing, no `detect.total` suppression for counters | `counter_names_never_reparent_and_are_never_suppressed` | `perf_tree.rs` |
| R3 — siblings by count descending, then label ascending | `counter_siblings_sort_by_count_descending_then_label_ascending` | `perf_tree.rs` |
| R3 — synthetic intermediates sum *immediate* children; measured non-leaf keeps its count | `a_measured_counter_parent_keeps_its_count_and_a_synthetic_parent_sums_children` | `perf_tree.rs` |
| R3 — `MetricValue::Count`, no HOT marker, no call counts | `counter_nodes_carry_counts_and_never_a_marker_or_call_count` | `perf_tree.rs` |
| R3 / R-1 — every share `Unknown`; the component's `100%` root is expected | `every_counter_share_is_unknown_and_only_the_component_renders_the_root_full` | `perf_tree.rs` |
| R3 — counts carry no unit suffix; live corpus projects correctly | `the_baseline_counter_key_set_projects_to_the_expected_shape` | `perf_tree.rs` |
| Negative/boundary — extreme counts must not overflow | `extreme_counter_values_saturate_rather_than_overflowing` | `perf_tree.rs` |
| R4 / R-5 / R-6 — glyph and marker fallback follows the detected terminal | `connectors_and_the_hot_marker_follow_the_terminal_unicode_capability` | `perf_tree.rs` |
| R-7 — heading verbatim, exactly one trailing newline | `the_performance_section_keeps_its_heading_and_one_trailing_newline` | `render.rs` |
| R2 / R-9 — the tree replaces the flat list; full dotted keys are gone; overlap note travels | `the_timing_tree_replaces_the_flat_stage_list` | `render.rs` |
| R3 — second tree omitted when counters are empty | `the_counter_tree_is_omitted_when_the_report_has_no_counters` | `render.rs` |
| R3 — second tree follows after one blank line; counters never enter the timing tree | `the_counter_tree_follows_the_timing_tree_after_a_blank_line` | `render.rs` |
| R2.3 — no stages ⇒ root only, no marker, through the real seam | `a_report_with_no_stages_renders_the_root_alone` | `render.rs` |
| R-10 — a malformed report renders without panicking | `a_malformed_report_renders_without_panicking` | `render.rs` |
| R4 — end-to-end stderr routing still carries stage and counter facts | `repo_aggregate_perf_covers_complete_command` *(rewritten)* | `cli.rs` |

Notes on how the harder cases were pinned:

- **The retired format is asserted absent, not merely the new format present.**
  `the_timing_tree_replaces_the_flat_stage_list` asserts `"Total: "`,
  `"Stages:"`, `"ms total"`, and the full key `"hardware.gpu"` are all gone. A
  partial revert that re-emitted the bullet list alongside the tree would fail.
- **Representation variants are covered.** Unicode versus ASCII terminals in one
  test with both assertion sets; counters present versus absent; measured versus
  synthetic counter parents; equal-count ties versus distinct counts; the empty
  map, the single-segment name, the depth-4 name, and `u64::MAX`.
- **Dependent outputs are asserted.** The counter tests check the *timing* tree
  is unaffected by a shared namespace, not just that the counter tree is right.
  The blank-line test checks counters do not appear above the `Counters` root.
  The ASCII test asserts Unicode glyphs are absent as well as ASCII present.
- **The corpus fixture is the real shipped key set.** `baseline_counters()` is
  the complete 34-key live counter set from `baseline-before.txt` with its real
  values, matching `baseline_stages()` on the timing side.

### Verification level

The counter projection is arithmetic over an in-memory struct, so L1 unit
coverage is right for it. Four tests deliberately cross a boundary, because the
phase's whole point is that the projection now reaches a terminal:

- `connectors_and_the_hot_marker_follow_the_terminal_unicode_capability` and
  `every_counter_share_is_unknown_…` cross into `biscuit-terminal`, rendering
  through `TerminalRenderable::render` with explicitly built terminals.
- The six `render.rs` tests exercise the real production seam,
  `render_performance_section`, not a projection helper.
- `repo_aggregate_perf_covers_complete_command` is an L1 CLI integration test
  that spawns the real binary and reads its real stderr.

Manual end-to-end evidence, `sniff os --perf --plain`, byte-inspected for escape
sequences (none present; Unicode glyphs retained, per R4 and R-5):

```text
## Performance

Total                             12.5ms  100%
└─ detect                         10.1ms   81%
   └─ os                          10.1ms   81% ▇ HOT
      ├─ package_managers          5.4ms   43%
      ├─ command_exists_in_path    3.3ms   27%
      │  ├─ port                   3.2ms   26%
      │  └─ fink                  79.0µs   <1%
      ├─ time                      2.1ms   17%
      ├─ locale                  846.0µs    7%
      └─ identity                 39.0µs   <1%

Concurrent, nested, and repeated stages may overlap; their durations do not sum to wall-clock time.

Counters                      108  100%
└─ os                         108     —
   └─ path                    108     —
      ├─ directories_scanned  106     —
      └─ command_misses         2     —
```

Compare with `baseline-before.txt`, which for the same command emitted a
`Total: 0.63 ms` line, a `Stages:` header over eight flat `- <full.dotted.key>:
N ms total (…)` bullets, and a `Counters:` header over two more — no hierarchy,
no column alignment, no call counts, no HOT row, and the full dotted key
repeated on every line. Definition-of-Success items 1, 2, 3, and 5 are visible
in the sample above. (Wall-clock figures differ between the two captures simply
because they are separate runs; the shape is the subject of the comparison.)

### Gates run

| Gate | Result |
|---|---|
| `cargo build -p sniff-cli` | green — **0 warnings** (was 14 `dead_code` at the end of Phase 2) |
| `cargo test -p sniff-cli --lib output::perf_tree` | green — 30 passed (20 carried, 10 added) |
| `cargo test -p sniff-cli --lib output::render` | green — 12 passed (6 carried, 6 added) |
| `cd sniff && just test` | green — **2656 passed, 0 failed, 24 skipped** |
| `cd sniff && just lint` | green — exit 0, zero clippy lints |
| `cargo clippy -p sniff-cli --all-targets` | green — zero lints |

`just lint` runs `cargo clippy -p sniff -p sniff-cli` without `--all-targets`,
so it does not lint test code. Clippy was therefore also run with
`--all-targets`; it found five `needless_borrows_for_generic_args` in the new
test code, all of which were fixed rather than suppressed. It is now clean.

The 24 skipped tests are nextest's pre-existing default-filter exclusions (L2 /
real-resource suites `just test` does not select) and are identical to Phase 2's
count. The four `claudine-cli` shipped-prompt fixture failures Phase 2 recorded
were seen only from an accidental repo-root run; no workspace-wide gate was run
this phase, per the repo's CI/CD test-scope discipline, and nothing outside
`sniff/` was touched.

No test was skipped, ignored, or weakened to make a gate pass.

### Observation logged but not acted on

`ms_to_duration` guards NaN, infinity, and negative input (R-10) but not
`Duration` overflow: `Duration::from_secs_f64` also panics when the value
exceeds `Duration::MAX`, which a `total_duration_ms` around `1e30` would do.
This is unreachable through the CLI — every `PerformanceReport` originates from
a live `Instant::elapsed` — and closing it was outside Phase 3's task list, so
it was deliberately left alone rather than changed under Rule 3.
`Duration::try_from_secs_f64(…).unwrap_or(Duration::ZERO)` would close it in one
line if the author wants it. Recorded in `message_to_agent`.

### Cross-platform note

No `#[cfg]` was added beyond the existing `#[cfg(test)]`; the diff is arithmetic
and string handling. No test asserts a path separator, a terminal width, or a
locale-dependent glyph *in a CLI context* — the two glyph-asserting tests build
their own `Terminal` with `supports_unicode` set explicitly, which is what R-5
requires and what makes them safe on a `LANG=C` runner. The one existing CLI
test that was rewritten was specifically made locale-insensitive in the process
(reading the cell after the label rather than the last cell, which is the share
and folds `—` → `-`).

`Terminal::default()` falls back to 80 columns when stdout is not a TTY
(`dimensions.rs:107`), so CLI-test output is 80 columns wide on every CI leg,
and every label the live key set produces fits there without truncation. The
macOS, Linux, native-Windows, and WSL2 legs see identical expectations, so no
`just cross-check` run was warranted for this phase. The `os` skill was not
loaded and no OS-specific claim is made.

### Skill drift

None. `.claude/skills/sniff/` documents performance *collection* — counters,
`WorkerCollector` propagation, baseline comparison rules — and says nothing
about the shape of `--perf` human output. `sniff/README.md` and `sniff/docs/`
contain no `--perf` example either, so there is nothing to bring into line. The
plan schedules the drift check for Phase 4; this is its result as of Phase 3.

### End state

Both trees render through the real seam, the flat lists are gone, and the Sniff
package-area gates are green. `sniff/lib` and `biscuit-terminal` are untouched.
The S-1 ruling remains the single open decision; Phase 3 resolved its task by
the declined branch and Phase 4 is not blocked by it.

---

## Phase 4

Phase 4 adds no production code. It pins the end-to-end contracts the spec
names, runs the package-area gates, captures after-evidence, and closes the
drift pass. Every source change in this phase is in `sniff/cli/tests/cli.rs`.

### Requirement-to-test mapping

Four tests added, 0 modified, 0 removed. All are L1 CLI integration tests that
spawn the real `sniff` binary through `common::owned_sniff_command()` — the
supported fixture spawn, with its disposable cwd/home/config/cache roots and
bounded PATH. None uses a `host_path` or `fake_only_path` escape, so
`spawn_site_guard.rs` needs no call-site justification comment.

| Plan / spec requirement | Test | Invocation |
|---|---|---|
| Plan 4.A / spec R4 — `--perf --plain` is ANSI-free and hierarchical | `perf_plain_output_is_ansi_free_and_hierarchical` | `sniff os --perf --plain` |
| Plan 4.A / spec R4 — a scriptable text command splits its streams | `perf_on_a_scriptable_text_command_stays_off_stdout` | `sniff --base <mono> repo language --perf --plain` |
| Plan 4.A / spec R4 — `--json --perf` stdout is exactly one JSON document | `json_perf_stdout_is_exactly_one_document` | `sniff os --json --perf --plain` |
| Plan 4.A / spec R3 — the counter tree reaches the human report | `counter_tree_reaches_the_human_report` | `sniff --base <mono> filesystem --perf --plain` |
| Plan 4.B / spec Success Criterion 4 — the collector seam is untouched and green | existing `sniff` collector suites, run unmodified | — |

Four different invocations were chosen deliberately, none of them the one
`repo_aggregate_perf_covers_complete_command` already covers, so the four tests
overlap neither each other nor the existing coverage.

### The original failing input is present, and the retired format is asserted absent

The spec's Problem Statement quotes the flat format. `baseline-before.txt`
records it verbatim for this exact command:

```text
Total: 0.63 ms

Stages:
- detect.total: 0.63 ms total (1 call, max 0.63 ms, last 0.63 ms)
- detect.os: 0.51 ms total (1 call, max 0.51 ms, last 0.51 ms)
```

`perf_plain_output_is_ansi_free_and_hierarchical` asserts `"Stages:"`,
`"ms total"`, `"Total: "`, and the full dotted key `"detect.os"` are all gone —
each of those four strings is present in the captured pre-fix output above, so
the test fails on the reported behavior and passes only after the fix. It is
not merely a present-tense assertion about the tree.

### How the harder assertions were made locale- and glyph-independent

Three shared helpers sit above the tests (`performance_section`, `metric_row`,
`metric_value`, `metric_offset`):

- **A row is located by whole whitespace cell**, never by substring, so `os`
  cannot match inside `package_managers` and `git` cannot match inside
  `git-status`.
- **A row's value is the cell immediately after its label**, not the last cell.
  The last cell is the share, which the component folds from `—` to `-` on a
  runner without Unicode (R-5). Reading from the end would make every value
  assertion locale-sensitive. This is the same rule Phase 3 applied when it
  rewrote `repo_aggregate_perf_covers_complete_command`.
- **Hierarchy is asserted as an offset ordering**, `metric_offset("detect") <
  metric_offset("os")`, rather than by naming a connector glyph. Connectors
  occupy that prefix under both glyph sets, so a child always starts further
  right than its parent whether the runner renders `├─` or `+- `. No test in
  this phase contains a box-drawing character, a `▇`, or a `×`.
- **The assertion is scoped to the section**, everything after the
  `## Performance` heading, so a label can never be satisfied by the host
  report printed above it.

### Dependent outputs and negative behavior

Each test asserts more than the fact it is named for:

- The ANSI test also asserts the root row owns `100%` and that four separate
  retired-format strings are absent — a partial revert that emitted the bullet
  list *alongside* the tree would fail it.
- The stream-split test asserts stdout equals `Rust` exactly, and that four
  distinct fragments (`## Performance`, `Total`, `Counters`, the overlap note)
  are each absent from it; it then asserts all of them present on stderr. Both
  directions are pinned, so a routing flip fails in both tests, not one.
- The JSON test asserts one document **and nothing after it** by driving
  `serde_json::Deserializer::into_iter` and requiring the second `next()` to be
  `None`. `serde_json::from_slice` alone would not catch a second document
  appended to stdout. It then asserts the report's schema survived
  (`total_duration_ms`, object `stages`, object `counters`), that stdout has no
  `## Performance`, and that the tree still renders on stderr.
- The counter test splits the section at `Counters` and asserts the timing half
  contains `shared_walk` but **not** `bytes_read` — spec Success Criterion 2,
  that counters never enter the timing tree, in its falsifiable direction. It
  parses the value cell as a `u64`, which fails if a duration unit were ever
  suffixed to a count, and asserts no `HOT` marker exists in the counter tree
  (R3).

### Verification level

L1 CLI integration is the required level here and unit coverage would not have
been sufficient: every one of these four contracts lives at a boundary the
projection cannot see. Stream routing is a process-level fact, `--plain`'s
escape stripping happens in `output::emit_*` after the renderer returns, and
"stdout is one JSON document" is a statement about the whole process's stdout,
not about any function's return value. Phase 2 and Phase 3 already own the
in-memory and single-render assertions; this phase deliberately adds none.

No new passive-corpus test was needed: Phase 2 and Phase 3 already run the
complete live 33-key stage set and 34-key counter set from `baseline-before.txt`
through the projection (`the_baseline_key_set_projects_to_the_expected_shape`,
`the_baseline_counter_key_set_projects_to_the_expected_shape`), and this phase's
four tests are themselves end-to-end runs through the real shipped binary.

### Collection-contract regression check (4.B) — green, unmodified

Run exactly as the plan requires, with no edit to the collector seam:

| Suite | Result |
|---|---|
| `cargo nextest run -p sniff --features remote --lib performance` | 6 passed, 0 failed |
| `cargo nextest run -p sniff --features remote --test integration -E 'test(performance)'` | 5 passed, 0 failed |

`git diff --stat sniff/lib biscuit-terminal` is empty. The R-11 stop condition
did not fire.

### Cross-platform assurance (4.B)

The diff adds no `#[cfg]` of any kind; `perf_tree.rs` and `render.rs` each
contain exactly one, the pre-existing `#[cfg(test)]`. No test in the phase
asserts a path separator, a terminal width, or a locale-dependent glyph, for the
reasons given above.

That reasoning was checked against the real hosts rather than left as an
argument. `just cross-check sniff-cli --os windows` and `--os wsl`, filtered to
the perf suite, both ran **46 tests, 46 passed** — including all four new tests
and Phase 3's rewritten one — on native Windows and on the WSL2 guest in nextest
**archive mode**, which is the faithful reproduction of CI's `wsl2-ubuntu` leg.

Two things went wrong with the rigs, neither caused by this change:

1. **The Linux host's lock is stale.** `~/ci-verification/.cross-check.lock`
   on `$BUILD_LINUX` has been held since **2026-09-14T18:25:30Z** by
   `{"purpose": "nightly-reward-spike", "owner": "reward-20260914-c3e60d0",
   "branch": "feat-nightly-perf"}`. A `--os all` run therefore blocks in the
   lock waiter until its 30-minute timeout. The script's contract — and this
   repo's rule — is that a lock left by a dead run is reported and never
   removed by anyone but its owner, so it was left in place and the run was
   re-issued per-OS to reach the two hosts that were free. **Native Linux is
   the one OS with no local evidence for this change; CI's `ubuntu-latest` leg
   remains its proof.** WSL2 follows the Linux code path and is green, but per
   the `os` skill that is not a substitute for the native Linux cell.
2. **The `wsl` leg prints `FAIL` after a passing run.** Its own marker is
   `cross-check-exit: 0` and all 46 tests passed; the leg-level verdict comes
   from the post-run bookkeeping. Verified on the guest: the archive run writes
   its JUnit report to `<clone>/target/nextest/ci/test-results.xml`, which is
   neither path `publish_wsl_receipt` searches — hence "produced no JUnit
   report" — and the same fresh `target` directory makes the restoring
   `mv target.hold target` nest the warm cache at `target/target.hold` rather
   than restoring it, so the next WSL run rebuilds from cold. Both are
   `scripts/cross-check.sh` defects. They were **not** fixed here (Rule 3,
   and `scripts/` is outside this fix's scope); they are recorded in
   `.claude/skills/os/build-hosts.md` so the next agent does not re-diagnose
   them, and raised in `human_review_items`. I did not pin the exact source of
   the non-zero SSH exit status and do not claim to have.

### Gates run (4.C)

| Gate | Result |
|---|---|
| `cargo test -p sniff-cli --test cli` (the four new tests) | green — 4 passed |
| `cd sniff && just test` | green — **2660 passed, 0 failed, 24 skipped** |
| `cd sniff && just lint` | green — exit 0, zero clippy lints, zero warnings |
| `cargo clippy -p sniff-cli --all-targets` | green — exit 0, zero lints |
| `just cross-check sniff-cli --os windows <perf filter>` | green — 46 passed |
| `just cross-check sniff-cli --os wsl <perf filter>` | green — 46 passed (`cross-check-exit: 0`); leg verdict `FAIL` for the harness reason above |
| `just cross-check sniff-cli --os linux` | **blocked** — stale lock, above |

The 24 skipped tests are nextest's pre-existing default-filter exclusions (L2 /
real-resource suites `just test` does not select), unchanged from Phase 2 and
Phase 3. No test was skipped, ignored, or weakened to make a gate pass, and no
`cargo fmt` was run. No workspace-wide gate was run, per the repo's test-scope
discipline; `sniff-cli` has no reverse dependency in the workspace, so there is
no dependency-derived downstream package to include.

### After-evidence (4.C)

`sniff/fixes/corrected-perf-flag/baseline-after.txt` records the same four
invocations `baseline-before.txt` captured, from the same host and binary
profile. The comparison:

| | Before | After |
|---|---|---|
| `total_duration_ms` | 722.7065 | 721.761208 |
| stage keys | 33 | **33** |
| counter keys | 34 | **34** |
| human timing output | `Total:` line + flat `Stages:` bullet list of full dotted keys | hierarchical tree, unit-aligned, `×N` calls, one `▇ HOT` row |
| human counter output | flat `Counters:` bullet list | separate `Counters` tree |
| stream routing | rich → stdout, scriptable/JSON → stderr | unchanged |
| bare `sniff --perf` | help screen, no section | unchanged |

Same key sets and same schema before and after is the evidence for spec R1 and
Non-Goals: collection did not move.

### Drift (4.C)

- `sniff/README.md` and `sniff/docs/` contain no `--perf` example — grepped for
  `--perf` and for `## Performance` across both. Nothing to update, confirming
  Phase 3's assessment.
- `.claude/skills/sniff/cli.md` documented output modes but said nothing about
  `--perf`. A `## --perf output` section was added recording the two-tree shape,
  the stream split, and the three traps that have already cost time here: bare
  `sniff --perf` is the help screen, rows carry the last dotted segment only,
  and `--plain` does not force ASCII. `performance.md` in the same skill covers
  collection and needs no change.
- `.claude/skills/os/build-hosts.md` gained the `wsl`-leg gotcha described
  above, per the `os` skill's rule that an OS fact learned the hard way is
  recorded in the same change.
- `docs/dependencies.md` needs no change: `sniff-cli` already depends on
  `biscuit-terminal`.

### Pre-commit graph check (4.C)

`detect_changes({scope: "all"})` against this worktree: `partial` and
`truncated` both absent, `risk_level: "low"`, `affected_count: 0`. It named two
changed symbols, both Markdown sections in an unrelated pre-existing edit to
`sniff/docs/cli/repo_recent-commits.md`.

Per repo policy a zero is "unseen", not "unaffected", so it was confirmed
independently rather than taken at face value: `sniff-cli` is a binary crate
with **no reverse dependency anywhere in the workspace** (grep of every
`Cargo.toml`), `render_performance_section` keeps its signature and its
`output/mod.rs` re-export, and `perf_tree` is a private module. The blast radius
really is nil.

### Working-tree scope

The Definition-of-Done checkpoint asks that the diff touch only
`sniff/cli/src/output/`, `sniff/cli/tests/cli.rs`, and this fix directory. What
is actually in the tree, and why:

| Path | Origin |
|---|---|
| `sniff/cli/src/output/{mod,render,perf_tree}.rs`, `sniff/cli/tests/cli.rs` | this fix, Phases 2–4 |
| `sniff/fixes/corrected-perf-flag/*` | this fix |
| `.claude/skills/sniff/cli.md`, `.claude/skills/os/build-hosts.md` | this phase's own drift task |
| `sniff/docs/cli/repo_recent-commits.md`, `sniff/docs/topics/repo/`, `sniff/features/2026-07-16-performance/` → `sniff/features/_completed/…` | **pre-existing**, present in the working tree before Phase 1 opened; untouched by any phase of this fix |

### Definition of Success — where each item is demonstrated

| # | Evidence |
|---|---|
| 1 | `baseline-after.txt`, and `perf_plain_output_is_ansi_free_and_hierarchical` |
| 2 | `counter_tree_reaches_the_human_report`, plus Phase 3's `the_counter_tree_is_omitted_when_the_report_has_no_counters` |
| 3 | Phase 2's HOT tests; the single `▇ HOT` row in `baseline-after.txt` |
| 4 | the two collector suites above, run unmodified; empty `git diff --stat sniff/lib` |
| 5 | `perf_plain_output_is_ansi_free_and_hierarchical` (no `\x1b`) and `json_perf_stdout_is_exactly_one_document` |
| 6 | Phase 2's zero-duration and malformed-report tests; Phase 3's saturating counter test |
| 7 | `just test` and `just lint` green; no `#[cfg]` beyond `#[cfg(test)]` |

### End state

**Implementation complete, ready for review.** All four phases are implemented,
the Sniff package-area gates are green, and native Windows and WSL2 archive-mode
evidence is in hand. `sniff/lib` and `biscuit-terminal` remain untouched, so
R-11 and spec R1 hold across the whole fix.

Two decisions are the author's, not an agent's: the S-1 scope ruling carried
forward unchanged from Phase 1, and moving this directory to `_completed`. This
fix was not moved and `just complete` was not run.

## Implementation of Review Findings #1

> **started at:** 2026-09-15T19:13:09-07:00

- this implementation is attempting to implement _all_ of the review findings found in '/Volumes/coding/wt/rusty-biscuit/feat-better-sniff/sniff/fixes/corrected-perf-flag/review-1.md'
- this is iteration 1 of the review-to-implement cycle
- the review carries two findings, both `high`, and both were routed to a
        dedicated subagent in series:
        - **F-1** — narrow terminals corrupt underscore-heavy metric rows; the
                remedy the reviewer prescribes lands in
                `biscuit-terminal::MetricsTree::build_markup`, not in Sniff
        - **F-2** — the new terminal rendering has no Level 2 verification; the
                remedy is a `level2_` Sniff test through the shared terminal
                harness
- **scope note:** F-1 resolves the S-1 scope ruling that Phases 1–4 left open in
        `human_review_items`. The reviewer explicitly rules on it — "Fix
        `MetricsTree::build_markup` so literal label text is escaped after
        truncation and padding … Do not escape labels in Sniff before width
        calculation" — so the review supplies the authorization the earlier
        phases lacked, and `biscuit-terminal` is now in scope for this cycle.
        Gates therefore run in both the `biscuit-terminal` and `sniff` package
        areas rather than `sniff` alone.
- starting the work on 'F-1 narrow-terminal label corruption' at 19:14:24
        - **root cause confirmed by repro, not by reading.** A throwaway width
                probe in `metrics_tree.rs` rendered the production key set at
                widths 20/27/33/40/48/49/60. Width 48 came back
                `italics=true` with the root row 48 visible columns wide and
                both child rows 47 — one column short. The mechanism is
                `prose::markdown::convert_italics`: its flanking rule treats `_`
                as an italics opener whenever it is *not* between two word
                characters, and `truncate_to_width` leaves
                `classified_embedded_language_` pressed against `…`, which is
                not a word character. `find_closing_single` then finds the
                closing `_` on a *later row*, so the span crosses rows and both
                underscores are deleted from the visible text — after
                `build_markup` already computed the column grid from the bare
                strings.
        - **escaping approach:** `Prose::escape_text` applied to the
                already-truncated, already-padded label in
                `MetricsTree::build_markup`, immediately before the optional
                `<b>` wrap. Verified rather than assumed: the backslashes Prose
                consumes are invisible, so the padded width computed upstream is
                exactly what reaches the screen — the re-run probe shows width
                48 back to a uniform 48 columns with the underscore intact. This
                is the same precedent the `share` cell already set with
                `.replace('<', "\\<")`, widened from one character to the full
                Prose special set (`< > { * _ [ ] ( ) \`). Escaping *before* the
                column math was rejected as the review directed: it inflates the
                measured label by one column per special character.
        - **widths verified:** 20 through 140 inclusive, Unicode and ASCII, in
                both regressions. The old failing band (27–48) is inside the
                sweep; so is the width-20 floor where `MIN_LABEL_W` holds the
                row wider than the terminal.
        - **tests added:**
                - `biscuit-terminal` —
                        `components::metrics_tree::tests::truncated_underscore_labels_do_not_leak_markup_at_any_width`.
                        Asserts no `ESC[3m` reaches the output and that the
                        undecorated row grid stays uniform, plus explicit
                        value-column (unit) alignment.
                - `sniff-cli` —
                        `output::perf_tree::tests::the_production_key_set_renders_clean_on_narrow_terminals`.
                        Drives the real `baseline_counters()` /
                        `baseline_stages()` key sets through
                        `counter_metrics_tree` and `timing_metrics_tree`. It
                        carries a non-vacuity guard: the sweep must actually
                        produce at least one label cut immediately after an
                        underscore, otherwise it fails rather than passing
                        silently.
        - **both regressions were proven to fail without the fix.** The escape
                call was temporarily reverted and each test re-run: the
                component test failed at `unicode=true, width=39` with a visible
                `ESC[3m` in the rendered bytes, and the Sniff test failed on the
                counter-row width check. The escape was then restored.
        - **stale comment removed.** `perf_tree.rs::timing_node` no longer
                claims the fix "is deliberately deferred pending a scope ruling"
                or that "corruption is confined to terminals narrower than 49
                columns" — both are now false. What replaces it is the four-line
                WHY a future reader still needs: labels reach `MetricsTree`
                unescaped on purpose, because the component measures and pads
                before Prose sees them. A grep for `deferred` / `scope ruling` /
                `49 column` / `escap` across `perf_tree.rs` and `render.rs`
                found no sibling comments repeating the claim.
        - **reverse dependencies checked.** `grep` over `Cargo.toml` found 17
                packages depending on `biscuit-terminal`; only `worktree/cli`
                and `claudine/cli` construct a `MetricsTree`. Their perf suites
                were run scoped (`-p worktree-cli -p claudine-cli -E
                'test(perf)'`): **88 passed, 0 failed**. `worktree` labels are
                `&'static str` stage names with no Prose specials, so its output
                is byte-identical. `claudine` labels can carry `::shell` command
                text; those characters now render literally instead of being
                parsed as markup, which is what `MetricNode::label`'s "rendered
                verbatim" contract already promised — a latent corruption of the
                same family, closed by the same change.
        - **gates, all four green, no `cargo fmt` run:**
                - `biscuit-terminal` `just test` — **3,262 passed, 0 failed, 55
                        skipped** (tier skips)
                - `biscuit-terminal` `just lint` — clean
                - `sniff` `just test` — **2,661 passed, 0 failed, 24 skipped**
                        (the same pre-existing tier skips the review recorded)
                - `sniff` `just lint` — clean
                - the shared Cargo target directory was writable this session,
                        so the reviewer's `CARGO_TARGET_DIR` workaround was not
                        needed.
        - nothing deferred.
- work completed for 'F-1 narrow-terminal label corruption' at 19:24:56

- starting the work on 'F-2 Level 2 terminal verification' at 19:26:30
        - **what the gap was.** The `--perf` rendering contract — hierarchy,
                unit-aligned duration and count columns, the styled HOT marker,
                truncation, and width degradation — had only Level 1 coverage:
                `perf_tree.rs` builds a synthetic `Terminal` in-process and
                `cli.rs` spawns the CLI with captured pipes and `--plain`.
                Neither has an emulator interpreting SGR, measuring glyph cells,
                or deciding a wrap. A piped run also never truncates at all,
                because nothing reports a narrow width to it.
        - **new test binary:** `sniff/cli/tests/level2_perf_tree_rendering.rs`,
                registered as a `[[test]]` with `required-features =
                ["test-fixtures"]` alongside the area's two existing L2 targets.
                Modelled on `level2_cicd_styling.rs` and
                `level2_git_status_styling.rs`: same `require_level!(Level::L2,
                TmuxHarness::available(), Backend::Tmux)` gate, same
                `common::capture_until` poll, same `#![cfg(feature =
                "test-fixtures")]` guard.
        - **no fixture binary, no production change.** Both tests run the real
                `sniff` binary (`cargo_bin("sniff")`) with `os --perf` inside
                the pane, so the evidence is the shipped CLI's own terminal
                detection and render path rather than a helper's. Nothing under
                `src/` was touched; the only non-test edit is the `[[test]]`
                stanza in `sniff/cli/Cargo.toml`.
        - **test 1 — `level2_perf_trees_render_aligned_and_styled_in_tmux`**,
                pane **100x70** (wider than the longest label plus columns, and
                wider than the 98-column overlap note, so nothing truncates).
                What it proves that L1 cannot:
                - **interpreted SGR.** The HOT row's captured *raw* line carries
                        bold (`ESC[1m`) for the marker and red (`ESC[31m`) for
                        its value; the overlap note's line carries italic
                        (`ESC[3m`); the value units carry a dim or grey SGR. L1
                        asserts markup text, not what the emulator received.
                - **glyph column alignment as the emulator measures it.** Every
                        row's value mantissa ends on the same *display column*
                        and every share cell's right edge lands on the same
                        display column, with `TAB`-free box connectors, `…`,
                        `µ`, and `—` counted as the cells they occupy. The L1
                        helpers use `str::find`, whose byte offsets are not
                        columns in a pane containing those glyphs.
                - **no wrapped or corrupted rows.** Every contiguous non-blank
                        line of each tree must parse as a `label value share`
                        triple; a wrapped row loses that triple on both
                        fragments, so a parse failure *is* the wrap detector.
                - **hierarchy and tree separation.** `Total` owns column 0;
                        `Total < detect < os` by label column, with at least
                        three distinct depths; the `Counters` tree is a separate
                        root rendered after the timing tree *and* its note,
                        every one of its rows carrying a unitless count.
                - **exactly one HOT row**, and it is a measured stage rather
                        than the synthetic root.
        - **test 2 —
                `level2_perf_trees_survive_narrow_pane_truncation_in_tmux`**,
                sweeping panes **30x70 through 50x70**. The component derives
                its label column from the pane width, so consecutive widths
                slide the truncation point one character at a time across every
                label — which is what walks the cut onto an underscore without
                hard-coding a width or a host-specific stage name. At each width
                it re-asserts column alignment, row parseability, absence of a
                leaked escape character, and — the F-1 signature — that **no
                tree row carries an italic SGR**, since rows use only bold, red,
                dim, and grey. It then *requires* that at least one swept width
                actually reached an underscore boundary, so the sweep cannot
                pass vacuously.
        - **boundaries actually exercised on this host** (printed by the test):
                `(33, "directories_…")`, `(39, "package_…")`,
                `(39, "command_…")`, `(46, "command_exists_…")`,
                `(49, "command_exists_in_…")`. All five render intact — F-1's
                fix holds in a real terminal.
        - **proven non-vacuous.** The F-1 escape call in
                `MetricsTree::build_markup` was temporarily reverted and the
                sweep re-run: it failed at 38 columns with `` `Total` row
                `package…` breaks the unit-aligned value column (`391.0µs` ends
                at 24, `684.0µs` at 25) ``. The escape was then restored and the
                file verified byte-identical to its pre-experiment state.
        - **readiness is polled, never slept.** Each invocation appends
                `printf '\nPERF-DONE-%s\n' <width>`; the predicate requires the
                performance heading, both tree roots, and that marker on a line
                of its own. The marker's *output* form cannot appear in the
                echoed command line and its nonce differs per width, so a stale
                frame from the previous width can never satisfy it. Its leading
                blank line also keeps it out of the counter tree's block.
        - **owned panes, not the shared broker pane.** Pane geometry *is* the
                contract here, and the broker's pane is not this test's to
                resize. Each test spawns its own tmux session, which `Drop`
                kills; tmux is headless and carries no global OS state, so
                nothing is shared and nothing leaks (`tmux ls` reported no
                server after the run).
        - **assertion helpers: re-derived, not promoted.** `cli.rs`'s
                `performance_section` / `metric_row` / `metric_value` /
                `metric_offset` are private to another test binary, but the
                decisive reason is that they locate rows with `str::find` —
                a byte offset, not a display column — which cannot express the
                alignment equality a real-terminal test has to assert. The
                pane-grid equivalents keep all three portability rules the
                originals encode: a label is matched as a whole whitespace cell,
                a value is read as the cell *after* the label (never the last
                cell, which is the share and folds from an em dash to a hyphen
                without Unicode), and hierarchy is an offset ordering rather
                than a named connector glyph. Nothing was added to
                `cli/tests/common/`.
        - **no Level 3 added**, per the review: this feature has no keyboard,
                mouse, paste, or IME interaction.
        - **gates, all green, no `cargo fmt` run** (the file was hand-formatted
                to match; `rustfmt --check` on it is clean):
                - `sniff` `just test-l2` — **4 tests run, 4 passed, 852
                        skipped**. Both new tests **ran**, neither skipped:
                        `level2_perf_trees_render_aligned_and_styled_in_tmux`
                        in 0.91 s and
                        `level2_perf_trees_survive_narrow_pane_truncation_in_tmux`
                        in 9.62 s (nextest marks the sweep `SLOW` at its 5 s
                        notice threshold; the local termination ceiling is 30 s
                        and CI's is 90 s).
                - `sniff` `just test` — **2,661 passed, 0 failed, 24 skipped**
                        (the same pre-existing tier skips).
                - `sniff` `just lint` — clean. `just lint` builds without
                        `test-fixtures`, so the new target was additionally
                        checked with `cargo clippy -p sniff-cli --features
                        test-fixtures --all-targets -- -D warnings` — clean.
                - the shared Cargo target directory stayed writable, so no
                        isolated `CARGO_TARGET_DIR` was needed.
        - nothing deferred.
- work completed for 'F-2 Level 2 terminal verification' at 19:45:40

### Orchestrator verification

- both findings were dispatched to a dedicated subagent in series; F-2 depended
        on F-1 landing first, because the review's narrow-pane requirement is
        only assertable once the component defect is closed
- the orchestrator re-ran the `biscuit-terminal` gates itself **after** F-2
        finished, because F-1's gate run predated F-2's non-vacuity experiment
        (which temporarily reverted the escape in `metrics_tree.rs` and restored
        it). Independent confirmation of the final tree:
        - `biscuit-terminal` `just test` — 3,262 passed, 0 failed, 55 skipped
        - `biscuit-terminal` `just lint` — clean
        - `metrics_tree.rs:332` holds the single
                `Prose::escape_text(&format!("{truncated:<label_w$}"))` call, so
                the restore was genuine
- `sniff` gates were last run by F-2, i.e. with both findings in the tree:
        `just test` 2,661 passed / 0 failed / 24 pre-existing skips,
        `just test-l2` 4 run / 4 passed, `just lint` clean

### Successful Completion

The implementation of review cycle 1 has completed successfully in 33 minutes.
During this implementation all 2 review findings were evaluated to see if they
could be fixed as a part of this implementation cycle: 2 were fixed, 0 were
deferred.

Nothing was deferred, and no performance measurement was required by either
finding, so no performance-deferral record was created.

Of particular note, this cycle closes the **S-1 scope ruling** that Phases 1
through 4 carried as an open `human_review_items` entry. The reviewer ruled on
it directly — the fix belongs in `biscuit-terminal::build_markup`, and labels
must *not* be escaped in Sniff before width calculation — which supplied the
authorization the earlier phases could not obtain in a non-interactive session.
The deferral comment in `perf_tree.rs::timing_node` was drift the moment the
component was fixed, and was rewritten to the short WHY that survives.

### Files changed in this cycle

| File | Change |
|---|---|
| `biscuit-terminal/lib/src/components/metrics_tree.rs` | F-1 — escape the padded label through `Prose::escape_text` before it enters component markup; component regression across widths 20..=140 in both glyph modes |
| `sniff/cli/src/output/perf_tree.rs` | F-1 — retire the stale deferral comment in `timing_node`; add the production-key-set narrow-terminal regression |
| `sniff/cli/tests/level2_perf_tree_rendering.rs` | F-2 — new, the two `level2_` real-terminal tests |
| `sniff/cli/Cargo.toml` | F-2 — register the new `[[test]]` target behind `required-features = ["test-fixtures"]`; correct the drifted CI-policy comment that named the CI/CD test as the area's only L2 target |
| `sniff/fixes/corrected-perf-flag/implementation-log.md` | this log |
| `sniff/fixes/corrected-perf-flag/review-1.md` | metadata only — `log`, `implemented`, `implemented_by` |

### Carried forward for the author

- **`claudine` visible-output change, not a regression but worth a look.** Of
        the 17 packages depending on `biscuit-terminal`, only `worktree/cli` and
        `claudine/cli` construct a `MetricsTree`. `worktree`'s labels are
        `&'static str` stage names with no Prose specials, so its output is
        byte-identical. `claudine`'s labels carry `::shell` command text, and
        those characters now render literally rather than being parsed as
        markup. That matches `MetricNode::label`'s "rendered verbatim" contract,
        and it closes a latent corruption of the same family — but it is a real
        change to a second package's output. Scoped verification
        `-p worktree-cli -p claudine-cli -E 'test(perf)'` ran 88 passed, 0
        failed.
- the three other open `human_review_items` from Phase 4 (the stale
        `$BUILD_LINUX` cross-check lock, the `scripts/cross-check.sh` WSL
        receipt path defect, and the rewritten pre-existing CLI test) are
        untouched by this cycle and remain open.
- **implementation complete, ready for review.** This fix was not moved to
        `_completed` and `just complete` was not run.

---

## Implementation of Review Findings #2

> **started at:** 2026-09-15T21:24:33-07:00

- this implementation is attempting to implement _all_ of the review findings found in '/Volumes/coding/wt/rusty-biscuit/feat-better-sniff/sniff/fixes/corrected-perf-flag/review-2.md'
- this is iteration 2 of the review-to-implement cycle
- the review carries a single `medium` finding, routed to one dedicated
        subagent:
        - **F-1** — the two new Level 1 width-sweep regressions each render 242
                widths and take over seven seconds, which puts a `slow_`-class
                cost in the fast `sanity` tier; the remedy is to narrow both
                matrices to the recorded 27–48 corrupt band (plus optional
                minimum-width and first-clean boundary cases) while keeping an
                explicit non-vacuity guard that at least one render truncates
                immediately after an underscore
- **scope note:** the finding touches one test in each of two package areas
        (`biscuit-terminal/lib/src/components/metrics_tree.rs` and
        `sniff/cli/src/output/perf_tree.rs`), so gates run in both areas, as in
        cycle 1. The review explicitly forbids shrinking the Level 2 30–50
        sweep, which is therefore left untouched.
- starting the work on 'F-1 narrow the L1 width sweeps' at 21:25:01
        - **retained widths: `once(20).chain(27..=49)` — 24 per glyph mode, 48
                renders per test, down from 121 / 242.** The band is the
                recorded corrupt range 27–48; 20 is the narrowest width the
                component supports and 49 is the first clean width, so the two
                extra cases pin both edges of the band for the price of two
                renders. Widths 50–140 only re-prove the clean case and were
                dropped. The Level 2 30–50 sweep in
                `sniff/cli/tests/level2_perf_tree_rendering.rs` was **not**
                touched, per the review.
        - **non-vacuity is now measured, not assumed.** Before narrowing, both
                tests were instrumented to record every `(unicode, width)` at
                which a row actually cuts immediately after an underscore
                (`_…` / `_...`), across the original 20..=140 sweep:
                - component test — unicode `{26, 28, 30, 31, 39, 48}`, ASCII
                        `{28, 30, 32, 33, 41, 50}`
                - Sniff production-key test — unicode `{27, 28, 30, 31, 32, 34,
                        35, 36, 37, 38, 39, 43, 44, 46, 55}`, ASCII `{29, 30,
                        32, 33, 34, 36, 37, 38, 39, 40, 45, 46, 48, 57}`
                - the retained set keeps 5 cut widths per glyph mode in the
                        component test and 13–14 per mode in the Sniff test, so
                        the guard has a wide margin in **both** modes rather
                        than resting on a single width. Only 26/50 and 55/57
                        fall outside, and each is redundant with several
                        retained widths.
                - the instrumentation was reverted; the shipped tests carry the
                        assertion, not the recorder.
        - **the component test gained the guard it lacked.** `metrics_tree.rs`
                now tracks `saw_underscore_cut` over the stripped rows and
                asserts it at the end, mirroring the Sniff test's existing
                guard verbatim. A truncation change that stops stranding an
                underscore now fails the test instead of leaving it green and
                vacuous.
        - comments in both tests were rewritten so the stated range matches the
                code: they keep the WHY (unescaped underscore opens a Prose
                italic span, shearing the value column off the pre-markup grid)
                and now state which band is swept and why the edges are there.
                No assertion was removed or weakened — italics absence, equal
                visible row width, value-column alignment, the four-row count,
                and timing-tree markup cleanliness all survive unchanged.
        - **the review's 7.4 s / 7.6 s timings did not reproduce on this host,
                by two orders of magnitude.** Measured A/B on the same warm
                isolated `CARGO_TARGET_DIR`, three targeted Nextest runs each,
                only the width range differing:
                | test | before (20..=140) | after (20, 27..=49) |
                |---|---|---|
                | `truncated_underscore_labels_do_not_leak_markup_at_any_width` | 0.063 / 0.059 / 0.061 s | 0.023 / 0.020 / 0.020 s |
                | `the_production_key_set_renders_clean_on_narrow_terminals` | 0.256 / 0.256 / 0.254 s | 0.065 / 0.060 / 0.060 s |
                - roughly 3.0× and 4.3× faster respectively; the component
                        test's residual is fixed per-test overhead, not sweep
                        work, which is why it gains less than the 5× the render
                        count alone predicts.
                - under the full area suite the same tests reported 0.073 s →
                        0.069 s (`biscuit-terminal`) and 0.311 s → 0.100 s
                        (`sniff`). The whole `biscuit-terminal` L1 suite
                        executes in 4.6–12.9 s wall clock, so a single 7.4 s
                        test inside it is arithmetically impossible here.
                - **the reduction is still worth keeping** on the review's own
                        grounds — 194 of the 242 renders per test bought no
                        distinct evidence — but the `slow_`-threshold breach
                        the finding describes could not be observed on this
                        machine. The author may want to know the recorded
                        numbers likely came from a cold-target or
                        build-inclusive measurement rather than test execution.
        - **gates — `biscuit-terminal`** (`CARGO_TARGET_DIR` on an isolated
                temp dir throughout, to avoid the `.rmeta` permission failure
                the reviewer hit on the shared target):
                - `just test` — 3,262 tests run, 3,262 passed, 0 failed, 55 tier
                        skips.
                - `just lint` — exit 0, no clippy or rustfmt output.
        - **gates — `sniff`:**
                - `just test` — 2,661 tests run, 2,661 passed, 0 failed, 24 tier
                        skips.
                - `just lint` — exit 0, no clippy or rustfmt output.
        - `just test-l2` was **not** run: the L2 targets are untouched by this
                change and cycle 1 already recorded the tmux evidence.
        - two pre-existing `SLOW` reports surfaced during the area runs and are
                unrelated to this change: `biscuit-terminal::layout_matrix`'s
                `layout_matrix_browser_snapshots` and
                `layout_matrix_markdown_snapshots`, and `sniff::bench_fixtures`'s
                two `large_monorepo_*` tests. They crossed the threshold only on
                the more loaded runs (host load average was 5.7 on 16 cores at
                session start) and are noted for the author, not acted on.
- work completed for 'F-1 narrow the L1 width sweeps' at 21:40:02

### Orchestrator verification

- the single finding was dispatched to one dedicated subagent; the orchestrator
        then re-ran every gate itself against the final tree, because the
        subagent's own runs used an isolated `CARGO_TARGET_DIR` and the
        instrumentation it used to locate the underscore-cut widths was applied
        and reverted inside that same session:
        - `biscuit-terminal` `just test` — **3,262 run, 3,262 passed, 0 failed,
                55 tier skips** (suite wall clock 4.633 s)
        - `biscuit-terminal` `just lint` — exit 0
        - `sniff` `just test` — **2,661 run, 2,661 passed, 0 failed, 24
                pre-existing tier skips** (suite wall clock 22.395 s)
        - `sniff` `just lint` — exit 0
- the two narrowed tests were also re-timed individually by the orchestrator on
        the shared warm target, confirming the subagent's figures:
        - `metrics_tree::tests::truncated_underscore_labels_do_not_leak_markup_at_any_width`
                — **0.023 s**
        - `perf_tree::tests::the_production_key_set_renders_clean_on_narrow_terminals`
                — **0.067 s**
        - both are three orders of magnitude below the five-second `slow_`
                threshold, so their ordinary unprefixed names are correct for
                the `sanity` tier.
- `git diff --stat` over the two package areas shows the change is confined to
        the two test bodies and their comments; no production code and no Level
        2 file was touched this cycle.

### Successful Completion

The implementation of review cycle 2 has completed successfully in 22 minutes.
During this implementation all 1 review findings were evaluated to see if they
could be fixed as a part of this implementation cycle: 1 were fixed, 0 were
deferred.

Nothing was deferred. The finding did require a performance measurement, and
the measurement was obtainable: the host carried enough headroom for a credible
warm A/B, taken three times per arm on one isolated target directory with only
the width range differing, and the orchestrator reproduced the post-change
figures independently on the shared target. No performance-deferral record was
created and `deferred_perf_measurement` remains unset.

One correction to the review's premise is worth the author's attention, and is
recorded above in full: the 7.404 s and 7.557 s per-test durations the finding
cites **do not reproduce on this host by two orders of magnitude**. The two
tests measured 0.061 s and 0.256 s *before* the narrowing, and the whole
`biscuit-terminal` Level 1 suite executes in under five seconds, which makes a
7.4 s test inside it arithmetically impossible. The recorded numbers most
plausibly came from a cold-target or build-inclusive measurement. The remedy was
implemented anyway on the review's other, independently sound grounds — 194 of
the 242 renders per test re-proved the already-clean case and bought no distinct
regression evidence — and it delivered the predicted reduction, roughly 3.0× and
4.3×.

### Files changed in this cycle

| File | Change |
|---|---|
| `biscuit-terminal/lib/src/components/metrics_tree.rs` | F-1 — narrow the sweep in `truncated_underscore_labels_do_not_leak_markup_at_any_width` to `once(20).chain(27..=49)`; add the `saw_underscore_cut` non-vacuity guard the test previously lacked; correct the comment's stated band |
| `sniff/cli/src/output/perf_tree.rs` | F-1 — narrow the sweep in `the_production_key_set_renders_clean_on_narrow_terminals` to the same range; correct the comment's stated band. Its existing non-vacuity guard was kept and confirmed to retain 13–14 underscore-cut widths per glyph mode |
| `sniff/fixes/corrected-perf-flag/implementation-log.md` | this log |
| `sniff/fixes/corrected-perf-flag/review-2.md` | metadata only — `log`, `implemented`, `implemented_by` |

### Carried forward for the author

- **the review's timing figures could not be reproduced** (above). If the
        `slow_` classification of these tests matters to the area's tier
        policy, the measurement method behind the 7.4 s / 7.6 s numbers is
        worth pinning down before acting on them further.
- **pre-existing `SLOW` reports, untouched and unrelated to this change:**
        `biscuit-terminal::layout_matrix`'s `layout_matrix_browser_snapshots`
        and `layout_matrix_markdown_snapshots`, and `sniff::bench_fixtures`'s
        two `large_monorepo_*` tests. They crossed the notice threshold only on
        the more loaded runs.
- **a concurrent editor in this worktree.** `sniff/docs/topics/repo/recent-commits.md`
        changed size during this cycle without either agent touching it; it was
        already modified at session start. Confirm its ownership before staging
        anything here.
- the `human_review_items` carried from Phase 4 and cycle 1 — the stale
        `$BUILD_LINUX` cross-check lock, the `scripts/cross-check.sh` WSL
        receipt path defect, the rewritten pre-existing CLI test, and the
        `claudine` visible-output change — are untouched by this cycle and
        remain open.
- **implementation complete, ready for review.** This fix was not moved to
        `_completed` and `just complete` was not run.
