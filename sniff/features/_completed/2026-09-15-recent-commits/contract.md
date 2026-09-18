# Recent Commits — Phase 1 Contract Baseline

Phase 1 output for [plan.md](./plan.md). It turns [spec.md](./spec.md) Decisions
1–15 into a test matrix, records the caller census and fixture inventory, fixes
the containment visit budget, and records the two interface spikes. Where this
document and the spec disagree, the spec wins, except for the two spike
findings in [Interface Spikes](#interface-spikes). Those record measured
library behavior that the spec's wording did not anticipate.

## Proving Boundaries

| Code | Boundary | Where |
| --- | --- | --- |
| U | Library unit test (in-crate, may use `pub(crate)` seams) | `sniff/lib/src/**` `#[cfg(test)]` |
| LI | Library integration test through public API against a real temp repo | `sniff/lib/tests/` |
| P | CLI parser/unit test (no process spawn) | `sniff/cli/src/**` `#[cfg(test)]` |
| CI | CLI process test through `SniffCliFixture` | `sniff/cli/tests/cli.rs` |
| L2 | Real-terminal test through `just test-l2` | `sniff/cli/tests/level2_*.rs` |
| DS | Downstream package L1 tests | `darkmatter`, `worktree` |

Pick the cheapest boundary that can observe the failure. Unit coverage alone is
insufficient where the behavior crosses the library/CLI, filesystem, process,
or terminal boundary.

## Contract Matrix

Each row names the behavior, the boundary that proves it, the phase that owns
the assertion, and the defect the assertion must catch.

### D1 — JSON contract

| Behavior | Boundary | Phase | Catches |
| --- | --- | --- | --- |
| `to_json()` is a bare array; no `period_label`/`repo_root` keys anywhere | U + CI (`--json` stdout parses as array) | 2, 4 | Old envelope leaking back; CLI re-wrapping |
| Exact per-commit key set (`heading`, `description`, `bullet_points`, `operation`, `scope`, `author{name,email}`, `datetime`, `files`, `file_types`, `remote`, optional `commit_url`) | U (serde snapshot of a fixed value) | 2 | Renamed/missing/extra keys |
| File kinds serialize as `modified`, `added`, `deleted`, `moved` only | U | 2 | `renamed`/`copied` leaking |
| Rename **and** copy both surface as `moved` with `original_path` | LI (real rename + copy commit) | 2 | Rename tracking still disabled (`track_rewrites(None)` today) |
| Per-file `added`/`removed` line counts; binary/unavailable is absent, never `0` | LI (text + binary fixture) | 2 | Binary files reported as 0/0 |

### D2 — Default scope and empty results

| Behavior | Boundary | Phase | Catches |
| --- | --- | --- | --- |
| No selection ⇒ `Count(10)` | U (options default) + CI (no period arg on a 12-commit repo ⇒ 10) | 2, 4 | 3-day default surviving in CLI |
| Valid query, zero matches ⇒ `Ok`, empty collection | LI | 3 | Error on empty |
| CLI empty ⇒ exit 0; `--json` stdout exactly `[]`; human note (if any) never on JSON stdout | CI (all three commands × text/plain/json) | 4 | Exit 1; note polluting JSON |
| `--no-error`/`--on-error` rejected by parser on all three commands | P | 4 | Flags silently retained |
| Count filters during traversal: `count 2 --operation fix` over `fix,feat,feat,fix,feat` returns both fixes | LI | 3 | Collect-N-then-filter (returns 1) |

### D3 — Operation filter

| Behavior | Boundary | Phase | Catches |
| --- | --- | --- | --- |
| Free-form, repeatable, OR'd, case-insensitive (`--operation planning`) | P (done: fixture) + LI | 1, 3, 4 | Enum restriction |
| Non-conventional commits excluded when any operation filter is set | LI | 3 | Non-CC commits passing filter |
| Completion suggests common words without restricting input | P (done: fixture) | 1, 4 | `value_enum` regression |

### D4 / D13 — Author and verbosity flags

| Behavior | Boundary | Phase | Catches |
| --- | --- | --- | --- |
| Author captured as name + email | LI | 3 | Missing email |
| `--author` substring, case-insensitive, matches name OR email | LI (`"EXAMPLE.com"` matches email only; `"ada"` matches name only) | 3 | Exact/case-sensitive match |
| `--show-author` adds author to header only; JSON unchanged | U (render) + U (`to_json` invariance) | 4 | Author toggle changing JSON |
| `-v`/`--verbose`, `-c`/`--compact` shape, conflicts, position semantics | P (done: fixture; move to real `Cli`) | 1, 4 | See [clap spike](#flag-shadowing-clap) |

### D5 — Calendar windows

| Behavior | Boundary | Phase | Catches |
| --- | --- | --- | --- |
| `today` = since local midnight in the options offset | U (pure bounds fn with fixed `now`) | 2 | UTC midnight |
| `yesterday` = `[local midnight −1d, local midnight)` | U + LI (commit at 23:59 yesterday included, 00:00 today excluded) | 2, 3 | Open-ended "yesterday to now" |
| `YYYY-MM-DD` = that single local day | U + LI (boundary commits either side, `+05:30` and `-08:00`) | 2, 3 | Since-date-forward semantics |
| Durations/count/hash ignore offset | U | 2 | Offset applied to durations |
| JSON `datetime` stays UTC `+00:00` regardless of offset | U | 2 | Local offset leaking into JSON |
| Human date label uses options offset (no UTC/local mismatch) | U (render) | 4 | Selected "today" but labeled yesterday |

### D6 — Merge, branch base

| Behavior | Boundary | Phase | Catches |
| --- | --- | --- | --- |
| Merge commits included; files = diff vs first parent | LI | 2, 3 | Merges skipped or diffed vs all parents |
| No-change merge ⇒ `files: []` serialized and rendered | LI + U (render) | 2, 4 | Merge dropped because files empty |
| `--branch` walks from branch tip; local wins over same-named remote-tracking; remote-tracking fallback | LI (local `feature` ≠ `origin/feature` tips) | 3 | Filter semantics; remote preferred |
| Unknown branch ⇒ typed error; no fetch | LI | 3 | Empty success / network use |

### D7 — Package attribution

| Behavior | Boundary | Phase | Catches |
| --- | --- | --- | --- |
| Structure-tier catalog only; no full inventory/language/framework walk | LI (work counters) | 3 | Full-tier detection cost |
| Monorepo: `packages`/`package_areas` always arrays (possibly empty) | U + LI | 2, 3 | Omitted/`null` in monorepo |
| Non-monorepo: both keys absent | U + LI | 2, 3 | Empty arrays on single-package repo |
| Area-root file not under a package stays unattributed | LI (`sniff/README.md`-style file) | 3 | Area-directory fallback |
| Deepest package wins for nested package roots | LI | 3 | Parent package attribution |
| Unknown `--package`/`--package-area` ⇒ typed error listing names | LI + CI (stderr, non-zero) | 3, 4 | Empty success |
| Filters and attribution share one ownership index (no rescan) | U (single construction) | 3 | Duplicate linear rescan |

### D8 — Classifier

| Behavior | Boundary | Phase | Catches |
| --- | --- | --- | --- |
| Precedence: CI/CD path rules → registry → `other` | U (table over every rule) | 2 | `.github/workflows/x.yml` classified `configuration` |
| `.html`/`.htm`/CSS/fonts ⇒ `web_assets`; SVG ⇒ `images`; `.component.html` ⇒ `source_code` | U | 2 | HTML fallback; CSS as source |
| Exactly one category per path over a passive corpus of every registry extension/filename | U (corpus) | 2 | Overlapping categories |
| `is_source_code_path`/`is_documentation_path` are wrappers over the enum | U | 2 | Parallel logic drifting |
| Downstream consumers keep passing with the documented HTML/CSS change | DS (darkmatter capture, worktree dirty tree) | 2, 6 | CRITICAL-risk regression |
| Sibling presets prune files by the same categories | U (render projection) + CI | 4 | CLI-local filter surviving |

### D9 — Linking

| Behavior | Boundary | Phase | Catches |
| --- | --- | --- | --- |
| `remote: true` iff reachable from some local remote-tracking tip | U (engine fixture, `add_fake_remote`) | 3 | Decoration-prefix heuristic |
| `false` only after every tip walk completed | U | 3 | False negative under budget |
| `null` for targets undetermined at budget exhaustion, no URL | U (budget = small constant) | 3 | Exhaustion reported as `false` |
| Tip order: origin, alphabetical non-upstream, upstream; winner uses same order | U (three remotes all containing) | 3 | Alphabetical/positional truncation |
| `commit_url` only when contained **and** provider has a browser URL | U (self-hosted remote ⇒ `remote: true`, no URL) | 3 | URL for uncontained commit |
| Skewed timestamps do not prune ancestors | U (reuse `populate_commit_remotes_skewed_timestamp_ancestor_not_pruned` shape) | 3 | Date-pruned walk |
| No fetch, no provider request | LI (counters: zero fetch/provider) | 3, 5 | Deep-tier fetch coupling |
| `repo hash` and single-commit helper resolve through `commit_links` | CI + U | 3, 5 | Divergent URL builders |

### D10 — Aggregate

| Behavior | Boundary | Phase | Catches |
| --- | --- | --- | --- |
| `sniff repo --json` families equal focused `--json` output byte-for-byte (same repo, last 10) | CI | 5 | Envelope surgery |
| One history collection, zero fetch/provider, no full inventory | LI (aggregate counters) | 5 | Double walk / network |
| Budget exhaustion keeps aggregate JSON valid with `remote: null` | U/LI | 5 | Invalid JSON on exhaustion |

### D11 — API shape

| Behavior | Boundary | Phase | Catches |
| --- | --- | --- | --- |
| Selector methods are last-wins on one `Selection` | U | 2 | First-wins / accumulation |
| Filters AND together | LI | 3 | OR semantics |
| Count zero rejected; `parse_period` precedence preserved (named day, ISO date, count, duration, hash) | U (retain existing `parse_period` cases) | 2 | `10` parsed as hash |
| Types reachable via `filesystem::git` and `filesystem` | LI (compile-level `use`) | 2 | Missing re-export |
| Legacy API gone | Compile (grep in Phase 5) | 5 | Compatibility shim left behind |

### D12 — Rendering

| Behavior | Boundary | Phase | Catches |
| --- | --- | --- | --- |
| One layout walk; compact/normal/verbose exact fixtures | U | 4 | Divergent per-format walks |
| `to_prose` tags + hash link only with `commit_url` + `file://` links (Windows drive paths) | U | 4 | Hash linked without containment; bad Windows URL |
| `to_markdown` uses `[t](u)`, `**`, `_`; no color tags | U | 4 | Prose tags in Markdown |
| `to_plain` has no tags, no `**`, no links, no escapes | U | 4 | Today's `**Description:**` in plain |
| CLI renders `to_prose` with one `Prose` call; no paragraph inflation; link fallback | P (done: spike) + L2 | 1, 4, 6 | CLI-side renderer; blank-line inflation |

### D14 — Sibling presets

| Behavior | Boundary | Phase | Catches |
| --- | --- | --- | --- |
| `source-code-changes`/`documentation-changes` share options, defaults, exit-0 | P + CI | 4 | Siblings keeping old flags |
| Commits with no files in the projected category are omitted | U | 4 | Empty commit blocks |

### D15 — Message parsing

| Behavior | Boundary | Phase | Catches |
| --- | --- | --- | --- |
| Heading ends at first `.` or newline (`"fix: use e.g. foo"` ⇒ `"use e"`) | U | 3 | Sentence heuristic sneaking in |
| No punctuation ⇒ heading = first line; no bullets ⇒ description `""` present | U | 3 | Missing description key |
| Multi-paragraph prose joins until bullets; bullet order kept | U | 3 | Paragraphs dropped |
| Operation/scope parsed from subject line only | U | 3 | Body text parsed as scope |

## Caller Census

Graph evidence came from `node .gitnexus/run.cjs impact <symbol> --direction
upstream` against an index whose only staleness was the untracked
implementation log. Text evidence came from `rg -t rust`, excluding feature
docs.

| Symbol | Graph risk | Callers (text-confirmed) |
| --- | --- | --- |
| `CommitDescSet` | UNKNOWN (ambiguous candidates) | lib: `git/recent_commits.rs`, `git/types.rs` (aggregate evidence), `git/mod.rs` + `filesystem/mod.rs` re-exports, `repo/aggregate_view.rs`; cli: `output/commit_blocks.rs`, `output/recent_commits.rs`, `output/repo_json.rs` |
| `get_recent_commits_*` (5 public + `_with_repo`) | not run per symbol | lib: `git/recent_commits.rs`, `git/types.rs:1141` (aggregate), re-exports; `lib/tests/integration.rs` (64 hits), `lib/tests/git_parity.rs`, `lib/benches/cases/git_ops.rs`; cli: `output/recent_commits.rs`. `discovery::get_recent_commits_{with_decorations,fallible}` are **different** `pub(crate)` functions (GitInfo commit list) and are **not** legacy targets |
| `commit_browser_url` | LOW, 1 direct | `cli/src/commands/mod.rs::run` (`repo hash`) |
| `populate_recent_commit_remotes_from_snapshot` | LOW, 2 direct | `git/remote_refresh.rs::populate_recent_commit_remotes` (`#[cfg(test)]`), `git/types.rs::detect_with_request` (deep tier) |
| `is_source_code_path` | **CRITICAL**, 53 impacted, 8 direct | sniff: `blast_radius.rs` (re-export + 2 uses), `git/recent_commits.rs::file_matches_kind`, `repo/aggregate_view.rs::area_change_facts`, cli `output/filesystem/mod.rs:1393`, `output/commit_blocks.rs`, `output/repo_json.rs:941`; **darkmatter**: `compose/context/capture/changes.rs::populate_file_changes`, `capture/docs.rs::populate_docs`; **worktree**: `lib/src/worktree.rs::{dirty_status, from_porcelain}`, `cli/src/commands/dirty_tree.rs::format_label` |
| `is_documentation_path` | LOW, 1 direct (graph) | graph missed CLI callers; text adds `cli/src/output/commit_blocks.rs` and `cli/src/output/repo_json.rs:946` (via `blast_radius` re-export) |
| `aggregate_commit_family_value` | not indexed | `cli/src/output/repo_json.rs` only |

Findings:

- No public caller of the legacy recent-commits API exists outside the
  `sniff` package area. The only ones outside `sniff/lib/src` are Sniff's own
  tests and benches (`lib/tests/integration.rs`, `lib/tests/git_parity.rs`,
  `lib/benches/cases/git_ops.rs`), which Phase 5 rewrites or deletes. The
  bench case (`get_recent_commits_in_range`/`_by_count`) should be migrated to
  `RecentCommits::collect`. If its bench IDs change, `benches/ci-bench-ids.txt`
  and `lib/tests/bench_ids_sync.rs` must be updated together.
- **Required downstream validation** for the classifier change: `darkmatter`
  (capture changes/docs) and `worktree` (lib dirty status, CLI dirty tree).
  Both call `is_source_code_path`, whose HTML/CSS result changes.
  `claudine`'s `path_kind` hits are an unrelated field name.
- The three URL implementations Decision 9 consolidates are:
  1. `lib/src/filesystem/git/api.rs::browser_url_from_url` (origin-only, used by `commit_browser_url`);
  2. `cli/src/output/filesystem/mod.rs::parse_git_url` + `build_commit_url_base` (origin-or-first remote);
  3. `lib/src/remote/url_parser.rs::parse_remote_url` (provider-aware owner/repo parser; keep as the parsing authority `commit_links` builds on).
- The decoration-prefix "pushed" heuristic lives in
  `cli/src/output/filesystem/mod.rs::render_git_section` (lines ~487–520),
  which renders `sniff repo git-status` text output and the
  `render_git_status_fixture` L2 binary, not the
  recent-commits renderer. Deleting it (Phase 3C) means migrating that
  section's commit links onto `commit_links`, which is observable in
  `level2_git_status_styling.rs`.
- Old-shape test sites to rewrite: `cli/tests/cli.rs` (18 envelope/family
  references), `cli/tests/snapshots.rs` (2), `cli/src/output/repo_json.rs`
  unit tests (15), `lib/src/filesystem/repo/aggregate_view.rs` (3),
  `lib/tests/git_parity.rs` (5).

## Fixture Inventory

### Reusable today

| Fixture | Location | Hermetic? | Reuse for |
| --- | --- | --- | --- |
| `setup_repo` + `add_fake_remote` (git2, writes `refs/remotes/<r>/<b>` directly) | `lib/src/filesystem/git/remote_refresh.rs` tests | Yes: in-process, explicit signature, no network | Containment, preference order, multi-remote, budget |
| Skewed-time commit builder (`git2::Time::new(seconds, 0)`) | `remote_refresh.rs:1361` | Yes | Clock-skew containment |
| `commit_file_with_timestamp` / `commit_file_with_message` | `lib/tests/integration.rs:3344`, `:2853` | Yes for git2 paths (explicit signature or repo-local identity) | Calendar windows, message parsing, count-after-filter |
| `create_monorepo_repo` (Cargo workspace, two packages) | `lib/tests/integration.rs:3045` | Yes (repo-local identity) | Attribution, package filters |
| `make_workspace` / `commit_file` | `lib/src/filesystem/blast_radius.rs` tests | Yes | Nested package roots |
| `build_linear_main` | `lib/tests/git_parity.rs:46` | Yes | Count selection, ordering |
| `SniffCliFixture::git()` + isolated env (`HOME`, `GIT_CONFIG_GLOBAL`, `GIT_CONFIG_NOSYSTEM`, XDG, PATH) | `cli/tests/common/mod.rs` | Yes | All CLI process tests |
| Benchmark repo builder | `lib/benches/support/builder.rs` | Yes | Large-history cost sanity only |

### Hermeticity gap

`lib/tests/integration.rs::create_merge_conflict_repo::run_git` spawns the host
`git` with only `GIT_DIR`-family variables removed and `commit.gpgsign=false`.
It still reads host global config, hooks, and templates. New recent-commit
library fixtures must not copy it. Use git2 in-process builders or set
`GIT_CONFIG_GLOBAL`/`GIT_CONFIG_NOSYSTEM`/`HOME` as `SniffCliFixture` does.

### Missing, to add in the phase that first needs them

| Requirement | Owner phase | Construction note |
| --- | --- | --- |
| Fixed author identity **and non-zero timezone offset** (`git2::Time::new(secs, ±offset_minutes)`) | 2 (calendar), 3 | Existing builders hard-code offset `0` |
| Commits straddling local midnight for `+05:30` and `-08:00` | 2 | Pure bounds tests plus one LI repo |
| Merge commit with changes, and empty (no-change) merge | 2 | git2 `commit` with two parents; tree identical to first parent |
| Local branch and remote-tracking ref with the same short name but different tips | 3 | `add_fake_remote(repo, "origin", "feature", other)` + local `feature` |
| Rename and copy (with `track_rewrites` enabled) plus a binary file | 2 | Copy detection needs the source unchanged in the same commit |
| Nested package roots (package inside package) and area-root file outside packages | 3 | Extend `make_workspace` |
| Three remotes (`origin`, `fork`, `upstream`) containing the same commit; self-hosted URL | 3 | `add_fake_remote` + `remote.<n>.url` config |
| Unpushed commits beyond every remote tip | 3 | Commit after the last `add_fake_remote` |
| Budget exhaustion | 3 | Many stale tips on disjoint history plus a crate-private small budget (e.g. `8` visits); assert `null` only for undetermined targets |
| Non-UTF-8 author/message bytes | 3 | `Signature::new` accepts only `&str`; write the raw commit object with `Odb::write(ObjectType::Commit, bytes)` (or `git commit-tree` under the isolated fixture env with `i18n.commitEncoding=latin1`) |

## Containment Visit Budget

**Chosen budget: `5_000_000` commit visits per collection**, total across all
remote-tracking tip walks.

- The spike's largest normal-state engine cost is **521,005 visits** (fork-style
  checkout C2, ~50 tips, 5 unpushed). `5_000_000` is about 9.6× that ceiling,
  which meets the plan's ~10× headroom.
- Expected wall cost at the measured per-visit rates: about 0.75 s at 0.15 µs
  (monorepo), about 1.3–2.5 s at 0.25–0.5 µs (vscode), and about 30 s only in
  the uncharacterized ~6 µs degraded state (spike finding 4). The degraded
  state is a known variance risk, not a design point.
- Priority ordering makes the budget irrelevant for the normal pushed case:
  `origin`'s tip reaches recent targets within a handful of visits and the
  walk stops once every target is determined. The budget binds only on
  tip-rich clones with unpushed or off-line targets. For example, vscode's
  5,197 tips at ~140k visits each exhaust after roughly 35 tip walks, which
  then yields `remote: null` instead of minutes of work.
- Visits are counted locally by the engine rather than read from the global
  `PerformanceCollector`, which may be disabled. The existing
  `git.commit_visits` counter is still incremented at the same chokepoint.
- The budget is a crate-private constant with a `pub(crate)` override for
  deterministic small-budget unit tests. It gets no public builder method
  because Decision 11 lists none.

## Interface Spikes

### Prose rendering

Command evidence: `bt prose --no-wrap -- "$(cat report.prose)"` on a
15-line, two-commit verbose report produced 15 lines. Blank lines survived,
no paragraph was inflated, styles rendered, OSC8 links were emitted, and
without OSC8 support links fell back to `[text](url)`.

Executable regressions: `sniff/cli/src/output/recent_commits_prose_layout.rs`
(6 tests).

Findings:

1. **No paragraph inflation.** A single
   `Prose::new(report).render(&terminal)` preserves every line, blank line, and
   leading indentation byte for byte in visible text.
2. `Prose`'s default layout is `WordWrap::None`, so the spec's literal call
   never folds lines. Long lines are left to the terminal's own soft wrap,
   which breaks mid-word at column 0. (`bt prose` enables wrapping itself.)
3. **Smallest layout that survives one render with word wrapping:**
   `Prose::new(report).with_word_wrap(WordWrap::WrapProse(None, None)).render(&terminal)`.
   It wraps at word boundaries within the terminal width, loses no words, adds
   no blank lines, and every source line still starts its own rendered line.
   Continuation lines start at column 0.
4. **`WrapProse(_, Some(n))` must not be used.** Its hanging indent applies to
   every line after the first line of the entire document, not per source
   line. It shifts the second commit's header and every indented line.
   Indentation-preserving continuation lines would need a `biscuit-terminal`
   wrap enhancement (per-line hanging indent from leading whitespace), which
   is out of scope for this feature. Phase 4 accepts continuation at column 0
   or raises that enhancement separately.
5. Colorless terminals (`ColorDepth::None`) drop color and emphasis sequences
   while keeping all visible text.
6. With OSC8 fallback, a linked hash inside literal brackets renders as
   `[[abc1234](url)]`. It is readable, but Phase 4 should consider whether the
   header bracket should be omitted around linked hashes in `to_markdown`
   output.

### Flag shadowing (clap)

Executable regressions: `sniff/cli/src/args/recent_commits_flag_shadowing.rs`
(11 tests, including two rejected-shape guards).

The spec premise needs correcting. Decision 13 describes the global `-v` as a
"log-level counter". In this CLI, raw tracing is driven only by `--debug`
(`init_tracing`), and global `--verbose` already means styled user-output
verbosity per command. Clap 4.6 merges global argument values across command
levels **by argument id**, so true independent shadowing cannot be expressed:

| Subcommand flag shape | Result |
| --- | --- |
| id `verbose`, `bool` (`SetTrue`) | **Panics** on every parse, even without `-v`: `Mismatch between definition and access of verbose` |
| distinct id (`report_verbose`), `-v`/`--verbose` | Parses, but `recent-commits --verbose` feeds the **global** counter and leaves the report flag `false` |
| id `verbose`, `u8` `Count` (**working shape**) | Local and global values unify: `-v` in any position yields report verbosity `1`; `--debug` stays `0` |

Consequences for Phase 4:

- Declare the subcommand flag as `#[arg(short, long, action = Count,
  conflicts_with = "compact")] verbose: u8`. The field name must stay
  `verbose`, and the report is verbose when the value is `> 0`.
- `-v` is **position-independent**. `sniff -v repo recent-commits` also
  selects the verbose report. This replaces Decision 13's "different meaning
  by position" risk with a simpler behavior and meets its real requirement:
  the flag never drives tracing.
- Repetition across positions does not sum (`-v repo recent-commits -v` ⇒ 1).
- `-c` with `-v` is a clap `ArgumentConflict` only when both appear at the
  subcommand level. `sniff -v repo recent-commits -c` parses with both set, so
  the Phase 4 adapter must resolve that case explicitly (for example, compact
  wins or a usage error) and test it.
- `--operation` as a free-form `Vec<String>` with `ArgValueCandidates` accepts
  arbitrary words and completes common ones.
