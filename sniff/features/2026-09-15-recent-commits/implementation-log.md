---
source_files_during_phase_1:
- sniff/cli/src/args/mod.rs
- sniff/cli/src/args/recent_commits_flag_shadowing.rs
- sniff/cli/src/output/mod.rs
- sniff/cli/src/output/recent_commits_prose_layout.rs
docs_updated_during_phase_1:
- sniff/features/2026-09-15-recent-commits/plan.md
- sniff/features/2026-09-15-recent-commits/spec.md
docs_created_during_phase_1:
- sniff/features/2026-09-15-recent-commits/contract.md
skills_files_updated_during_phase_1:
- .claude/skills/sniff/cli.md
source_files_during_phase_2:
- sniff/lib/src/filesystem/path_kind.rs
- sniff/lib/src/filesystem/file_types/mod.rs
- sniff/lib/src/filesystem/git/discovery.rs
- sniff/lib/src/filesystem/git/mod.rs
- sniff/lib/src/filesystem/mod.rs
- sniff/lib/src/filesystem/git/recent_commits.rs
- sniff/lib/src/filesystem/git/recent_commits/mod.rs
- sniff/lib/src/filesystem/git/recent_commits/options.rs
- sniff/lib/src/filesystem/git/recent_commits/payload.rs
- sniff/lib/tests/integration.rs
docs_updated_during_phase_2:
- sniff/features/2026-09-15-recent-commits/plan.md
- sniff/features/2026-09-15-recent-commits/implementation-log.md
- sniff/docs/cli/repo_source-code-changes.md
- sniff/docs/cli/repo_dirty-source-code.md
docs_created_during_phase_2: []
skills_files_updated_during_phase_2:
- .claude/skills/sniff/architecture.md
source_files_during_phase_3:
- sniff/lib/src/error.rs
- sniff/lib/src/filesystem/mod.rs
- sniff/lib/src/filesystem/git/mod.rs
- sniff/lib/src/filesystem/git/api.rs
- sniff/lib/src/filesystem/git/commit_links.rs
- sniff/lib/src/filesystem/git/discovery.rs
- sniff/lib/src/filesystem/git/remote_refresh.rs
- sniff/lib/src/filesystem/git/remote_resolver.rs
- sniff/lib/src/filesystem/git/recent_commits/mod.rs
- sniff/lib/src/filesystem/git/recent_commits/collect.rs
- sniff/lib/src/filesystem/git/recent_commits/options.rs
- sniff/lib/src/filesystem/git/recent_commits/payload.rs
- sniff/lib/tests/recent_commits.rs
- sniff/cli/src/output/filesystem/mod.rs
- sniff/cli/tests/cli.rs
docs_updated_during_phase_3:
- sniff/features/2026-09-15-recent-commits/plan.md
- sniff/features/2026-09-15-recent-commits/implementation-log.md
- sniff/docs/cli/repo_hash.md
- sniff/docs/cli/repo_git-status.md
docs_created_during_phase_3: []
skills_files_updated_during_phase_3:
- .claude/skills/sniff/architecture.md
source_files_during_phase_4:
- sniff/lib/src/filesystem/git/recent_commits/mod.rs
- sniff/lib/src/filesystem/git/recent_commits/options.rs
- sniff/lib/src/filesystem/git/recent_commits/payload.rs
- sniff/lib/src/filesystem/git/recent_commits/render.rs
- sniff/cli/src/args/mod.rs
- sniff/cli/src/args/repo.rs
- sniff/cli/src/args/recent_commits.rs
- sniff/cli/src/args/recent_commits_flag_shadowing.rs
- sniff/cli/src/commands/mod.rs
- sniff/cli/src/output/mod.rs
- sniff/cli/src/output/commit_blocks.rs
- sniff/cli/src/output/recent_commits.rs
- sniff/cli/src/output/recent_commits_prose_layout.rs
- sniff/cli/src/output/repo_json.rs
- sniff/cli/tests/cli.rs
- sniff/cli/tests/level2_recent_commits_rendering.rs
docs_updated_during_phase_4:
- sniff/features/2026-09-15-recent-commits/plan.md
- sniff/features/2026-09-15-recent-commits/implementation-log.md
docs_created_during_phase_4: []
skills_files_updated_during_phase_4:
- .claude/skills/sniff/cli.md
- .claude/skills/sniff/architecture.md
source_files_during_phase_5:
- sniff/lib/src/filesystem/git/commit_links.rs
- sniff/lib/src/filesystem/git/recent_commits/mod.rs
- sniff/lib/src/filesystem/git/recent_commits/collect.rs
- sniff/lib/src/filesystem/git/types.rs
- sniff/lib/src/filesystem/git/mod.rs
- sniff/lib/src/filesystem/mod.rs
- sniff/lib/src/filesystem/repo/aggregate_view.rs
- sniff/lib/tests/git_parity.rs
- sniff/lib/tests/integration.rs
- sniff/lib/tests/recent_commits.rs
- sniff/lib/benches/cases/git_ops.rs
- sniff/cli/src/output/repo_json.rs
- sniff/cli/tests/cli.rs
- sniff/cli/tests/snapshots.rs
- sniff/cli/tests/snapshots/snapshots__repo_aggregate_json.snap
docs_updated_during_phase_5:
- sniff/features/2026-09-15-recent-commits/plan.md
- sniff/features/2026-09-15-recent-commits/implementation-log.md
- sniff/docs/topics/repo/recent-commits.md
- sniff/docs/topics/repo/recent-commits-schema.md
- sniff/docs/cli/repo_recent-commits.md
- sniff/docs/cli/repo_source-code-changes.md
- sniff/docs/cli/repo_documentation-changes.md
- sniff/docs/cli/repo.md
- sniff/docs/topics/json-output.md
- sniff/cli/README.md
- sniff/lib/README.md
docs_created_during_phase_5: []
skills_files_updated_during_phase_5:
- .claude/skills/sniff/architecture.md
- .claude/skills/sniff/cli.md
packages:
- sniff
human_review: true
message_to_agent: 'All 6 phases are implemented and ready for author review; read ''## Phase 6'' in implementation-log.md.
  Phase 6 fixed two defects: the InvalidPeriod message now names the count form (CLI regression test),
  and an unused_mut in the aggregate_view test fixture. `just lint` in sniff/ lacks --all-targets/-D warnings;
  use `cargo clippy -p sniff --all-targets -- -D warnings` (and sniff-cli) to match CI. Pre-existing,
  out of scope: 3 redundant_closure errors under --features remote in lib/tests/{remote_observation,focused_provider}.rs
  (identical to main); `research` cannot compile on the macOS host (corrupt sqlx-macros dylib). Decision
  13 differs from the spec as recorded in contract.md: clap merges -v by id, so a subcommand -v also bumps
  global cli.verbose; logging stays on --debug.'
source_files_during_phase_6:
- sniff/lib/src/error.rs
- sniff/lib/src/filesystem/repo/aggregate_view.rs
- sniff/cli/tests/cli.rs
docs_updated_during_phase_6:
- sniff/features/2026-09-15-recent-commits/plan.md
- sniff/features/2026-09-15-recent-commits/implementation-log.md
- sniff/features/2026-09-15-recent-commits/contract.md
docs_created_during_phase_6: []
skills_files_updated_during_phase_6:
- .claude/skills/os/build-hosts.md
- .claude/skills/sniff/SKILL.md
source_code:
- sniff/cli/src/args/mod.rs
- sniff/cli/src/args/recent_commits_flag_shadowing.rs
- sniff/cli/src/output/mod.rs
- sniff/cli/src/output/recent_commits_prose_layout.rs
- sniff/lib/src/filesystem/path_kind.rs
- sniff/lib/src/filesystem/file_types/mod.rs
- sniff/lib/src/filesystem/git/discovery.rs
- sniff/lib/src/filesystem/git/mod.rs
- sniff/lib/src/filesystem/mod.rs
- sniff/lib/src/filesystem/git/recent_commits.rs
- sniff/lib/src/filesystem/git/recent_commits/mod.rs
- sniff/lib/src/filesystem/git/recent_commits/options.rs
- sniff/lib/src/filesystem/git/recent_commits/payload.rs
- sniff/lib/tests/integration.rs
- sniff/lib/src/error.rs
- sniff/lib/src/filesystem/git/api.rs
- sniff/lib/src/filesystem/git/commit_links.rs
- sniff/lib/src/filesystem/git/remote_refresh.rs
- sniff/lib/src/filesystem/git/remote_resolver.rs
- sniff/lib/src/filesystem/git/recent_commits/collect.rs
- sniff/lib/tests/recent_commits.rs
- sniff/cli/src/output/filesystem/mod.rs
- sniff/cli/tests/cli.rs
- sniff/lib/src/filesystem/git/recent_commits/render.rs
- sniff/cli/src/args/repo.rs
- sniff/cli/src/args/recent_commits.rs
- sniff/cli/src/commands/mod.rs
- sniff/cli/src/output/commit_blocks.rs
- sniff/cli/src/output/recent_commits.rs
- sniff/cli/src/output/repo_json.rs
- sniff/cli/tests/level2_recent_commits_rendering.rs
- sniff/lib/src/filesystem/git/types.rs
- sniff/lib/src/filesystem/repo/aggregate_view.rs
- sniff/lib/tests/git_parity.rs
- sniff/lib/benches/cases/git_ops.rs
- sniff/cli/tests/snapshots.rs
- sniff/cli/tests/snapshots/snapshots__repo_aggregate_json.snap
documentation:
- sniff/features/2026-09-15-recent-commits/plan.md
- sniff/features/2026-09-15-recent-commits/spec.md
- sniff/features/2026-09-15-recent-commits/contract.md
- sniff/features/2026-09-15-recent-commits/implementation-log.md
- sniff/docs/cli/repo_source-code-changes.md
- sniff/docs/cli/repo_dirty-source-code.md
- sniff/docs/cli/repo_hash.md
- sniff/docs/cli/repo_git-status.md
- sniff/docs/topics/repo/recent-commits.md
- sniff/docs/topics/repo/recent-commits-schema.md
- sniff/docs/cli/repo_recent-commits.md
- sniff/docs/cli/repo_documentation-changes.md
- sniff/docs/cli/repo.md
- sniff/docs/topics/json-output.md
- sniff/cli/README.md
- sniff/lib/README.md
completed_phase: '6'
implemented: true
human_review_items:
- 'Local cross-OS runtime evidence is blocked, so native Windows and WSL2 results for this change are
  CI-only. build-win-native W: has 8 KB free: the standing clone W:\ci-verification\rusty-biscuit (99.3
  GB, own target\ never swept because the sweep targets the nonexistent W:/rusty-biscuit-target), the
  orphan W:\ci-verification\rb-pr66 (62 GB, 2026-08-30), and the 130.8 GB WSL VHDX. The WSL guest resets
  every SSH connection. build-linux holds a cross-check lock from nightly-reward-spike (reward-20260914-c3e60d0,
  since 2026-09-14) with no live cargo process. Only the owner may delete those artifacts or remove the
  lock.'
- darkmatter/features/2026-09-09-more-context (active, unimplemented) still plans on get_recent_commits_by_count
  and CommitDescSet::describe, which this feature deleted (Decision 11). It needs re-planning onto RecentCommits::collect
  + to_plain before implementation.
---

# Recent Commits — Implementation Log

## Phase 1

### Scope

Phase 1 ("Contract and Risk Closure") has no production behavior changes. It
delivered one planning artifact (`contract.md`) and two executable spikes,
pinned as test-only modules in `sniff-cli`.

### Work Group 1A — Contract Baseline

- **Contract matrix:** written to `contract.md` § Contract Matrix. Every
  decision (D1–D15) has rows naming the behavior, its proving boundary (U, LI,
  P, CI, L2, DS), the owning phase, and the defect the assertion must catch.
- **Caller census:** GitNexus upstream impact plus `rg -t rust`. The index was
  stale only by this untracked log file.
  - `is_source_code_path` is CRITICAL (53 impacted, 8 direct). Downstream
    consumers are darkmatter (`capture/changes.rs`, `capture/docs.rs`) and
    worktree (`lib/src/worktree.rs`, `cli/src/commands/dirty_tree.rs`). Both
    are required downstream validation in Phases 2 and 6.
  - `is_documentation_path`: the graph reports LOW with 1 direct caller but
    missed the CLI callers in `commit_blocks.rs` and `repo_json.rs`; text
    search found them.
  - `CommitDescSet` returned UNKNOWN from the graph (ambiguous candidates).
    Text search confirms callers only inside the sniff package area.
  - `commit_browser_url`: LOW, one caller (`repo hash` in
    `cli/src/commands/mod.rs::run`).
  - `populate_recent_commit_remotes_from_snapshot`: LOW, called by the deep
    tier (`types.rs::detect_with_request`) and a test-only wrapper.
  - No external public callers of the legacy API. Sniff's own
    `lib/tests/integration.rs`, `lib/tests/git_parity.rs`, and
    `lib/benches/cases/git_ops.rs` use it.
  - `discovery::get_recent_commits_{with_decorations,fallible}` share the
    prefix but are unrelated `pub(crate)` GitInfo helpers, not legacy targets.
  - **Drift found:** the CLI URL parser and "pushed" heuristic Decision 9
    deletes live in `render_git_section` (`sniff repo git-status`), not in
    recent-commits rendering. Recorded for Phase 3C.
- **Fixture inventory:** `contract.md` § Fixture Inventory lists reusable
  hermetic fixtures, one hermeticity gap (`integration.rs::run_git` reads host
  global git config), and the missing fixtures, each assigned to the phase
  that first needs it.
- **Visit budget:** `5_000_000` commit visits, about 9.6× the spike's largest
  normal-state engine cost (521,005 visits, fork-style C2). The rationale and
  expected wall costs are in `contract.md` § Containment Visit Budget.

### Work Group 1B — Interface Spikes

#### Prose rendering

- `bt prose --no-wrap` on a 15-line, two-commit verbose report returned 15
  lines. Blank lines and indentation survived, styles rendered, OSC8 links
  were emitted, and links fell back to `[text](url)` without OSC8. (Note:
  `cat` is aliased to `bat` on this host, so use `/bin/cat`.)
- Default `Prose` layout is `WordWrap::None`, so it never folds lines.
- `WrapProse(None, Some(2))` was tried first and **rejected**. Its hanging
  indent applies to every line after the document's first line, which shifted
  the second commit header to column 2 and every indented line by +2.
- `WrapProse(None, None)` is the smallest layout that wraps at word boundaries
  while keeping each source line on its own rendered line, with no blank-line
  inflation. Continuation lines start at column 0. Indentation-preserving
  continuation would need a `biscuit-terminal` enhancement, which is out of
  scope.
- Tests in `sniff/cli/src/output/recent_commits_prose_layout.rs`:
  - `single_render_preserves_every_line_and_blank_line_without_inflation`
  - `single_render_emits_styles_and_osc8_links_when_supported`
  - `links_fall_back_to_markdown_without_osc8_support`
  - `colorless_terminal_keeps_every_visible_line`
  - `colorless_terminal_drops_color_and_emphasis_sequences`
  - `word_wrap_layout_keeps_source_lines_and_wraps_within_width`. Mutation
    check: switching it to `Some(2)` fails with "source line lost its own
    rendered line".

#### Flag shadowing (clap 4.6)

- **The spec premise needs correcting.** Global `-v/--verbose` is not a
  log-level counter. `init_tracing` is driven only by `--debug`/`RUST_LOG`,
  and `--verbose` is styled-output verbosity.
- Clap merges global values by argument id at every command level. Three
  shapes were tested:
  1. `verbose: bool` with the same id **panics** on every parse ("Mismatch
     between definition and access of `verbose`").
  2. A distinct id (`report_verbose`) with `-v`/`--verbose` makes the global
     flag win, so the report flag is never set.
  3. The same id with `u8`/`Count` works. The two values unify, so `-v` is
     position-independent; `--debug` stays 0.
- Clap enforces the conflict between compact and verbose only when both flags
  are at the subcommand level. `sniff -v repo recent-commits -c` parses with
  both set, so the Phase 4 adapter must decide how to resolve it.
- `--operation` as a free-form `Vec<String>` with `ArgValueCandidates` accepts
  `planning` and completes `feat`/`fix`/`chore`/`docs` via
  `clap_complete::engine::complete`.
- Tests in `sniff/cli/src/args/recent_commits_flag_shadowing.rs`:
  - `no_verbosity_flags_select_normal_report`
  - `subcommand_verbose_selects_verbose_report_without_enabling_tracing`
  - `verbose_is_position_independent_because_the_flags_unify`
  - `repeated_verbose_across_positions_does_not_sum`
  - `compact_short_and_long_select_compact_report`
  - `compact_and_verbose_conflict_within_the_subcommand_in_either_order`
  - `global_verbose_with_subcommand_compact_parses_with_both_set`
  - `operation_accepts_free_form_repeated_values`
  - `operation_completion_suggests_common_values`
  - `rejected_shapes::same_id_bool_flag_panics_even_without_verbose_input`
  - `rejected_shapes::distinct_id_flag_loses_verbose_to_the_global_counter`

### Requirement → test mapping

| Phase 1 requirement | Evidence |
| --- | --- |
| Contract matrix with owner and proving boundary per decision | `contract.md` § Contract Matrix (document; later phases implement the rows) |
| Caller census documented | `contract.md` § Caller Census |
| Fixture inventory | `contract.md` § Fixture Inventory |
| Budget choice documented | `contract.md` § Containment Visit Budget |
| Prose risk has executable regression tests | 6 tests in `recent_commits_prose_layout.rs` |
| Clap risk has executable regression tests | 11 tests in `recent_commits_flag_shadowing.rs` |

### Gates

- `cargo nextest run -p sniff-cli --lib recent_commits_flag_shadowing`:
  11/11 passed.
- `cargo nextest run -p sniff-cli --lib recent_commits_prose_layout`: 6/6
  passed.
- `just test` (sniff): 2678 passed, 24 skipped, exit 0. The 17 new tests were
  selected. The 24 skips are the suite's pre-existing ignored/gated tests;
  none are new.
- `just lint` (sniff): exit 0.
- `cargo clippy -p sniff-cli --lib --tests -- -D warnings`: exit 0. This was
  run because `just lint` does not cover `#[cfg(test)]` modules.
- `node .gitnexus/run.cjs detect-changes --scope all`: low risk, 0 affected
  processes.
- **Cross-OS:** not run. The new tests are pure in-process parsing and
  rendering of fixed strings, with no filesystem paths, subprocesses, or
  OS-specific code, so CI's normal matrix is enough.
- No `cargo fmt` was run.

## Phase 2

### Scope and approach

Phase 2 ("Library Primitives") adds the public data, classification, diff,
and options foundations. Collection (`RecentCommits::collect`), rendering,
and linking are Phases 3–4. The legacy `CommitDescSet` API keeps working
unchanged until Phase 5 deletes it.

### Work Group 2B — Path Categories

- Impact analysis re-run before editing: `is_source_code_path` is
  **CRITICAL** (53 impacted, 8 direct: sniff lib/CLI, darkmatter capture,
  worktree lib/CLI); `is_documentation_path` is LOW (1 direct in the graph;
  text search adds CLI `commit_blocks.rs` and `repo_json.rs`).
- Added public `ChangeCategory` (`source_code`, `web_assets`, `images`,
  `documentation`, `configuration`, `cicd`, `other`; `snake_case` serde) and
  `classify_path` to `sniff/lib/src/filesystem/path_kind.rs`.
  - Precedence: CI/CD rules → registry exact file name → registry basename
    pattern (Angular `.component.html`) → `.html`/`.htm` web-asset override →
    registry extension → `other`.
  - CI/CD rules: directory sequences (`.github/workflows`, `.gitea/workflows`,
    `.forgejo/workflows`, `.circleci`, `.buildkite`, `.woodpecker`) matched as
    consecutive `Path::components()` anywhere in the parent, so absolute
    worktree paths and native Windows separators classify the same; plus a
    case-insensitive file-name table (`.gitlab-ci.yml`, `Jenkinsfile`,
    `azure-pipelines.yml`, `.drone.yml`, `buildspec.yml`, `.travis.yml`,
    `bitbucket-pipelines.yml`, `appveyor.yml`, `cloudbuild.yaml`, …).
    `.github/actions/**` is deliberately **not** CI/CD, so composite-action
    JavaScript stays source code for worktree/darkmatter consumers.
  - `lookup_basename_pattern` is now `pub(crate)`-re-exported from
    `file_types` (it was module-private and unused by the old predicates).
- `is_source_code_path` / `is_documentation_path` are now one-line wrappers
  over `classify_path`. Behavior changes (intentional, Decision 8):
  - CSS/SCSS/Sass/Less/PCSS: source code → web assets.
  - `.html`/`.htm`: source code **and** documentation → web assets.
  - **Additional change found:** exact registry file names now win over the
    extension for documentation too. `requirements.txt` was documentation
    (the old predicate fell through to the `.txt` extension after a
    non-documentation exact match); it is now configuration only.
  - Drifted legacy tests (`css_file_is_source_code`, `html_file_is_source_code`,
    `htm_file_is_source_code`, `scss_is_source_code`) were inverted and renamed.

### Work Group 2C — Diff Metadata

- **Design choice:** the public `get_commit_files*` family keeps its
  documented "rename tracking disabled" contract. It has other callers
  (`commit_files_at`, path history, the legacy recent-commits walk), and
  changing it would ripple through outputs this feature does not own. A new
  crate-private `discovery::committed_file_changes_with_cache` returns
  `CommittedFileChange { path, kind: DeltaKind, original_path, line_counts:
  Option<LineCounts> }`. It shares `commit_trees` (first parent, shallow
  boundary as root, fallible), so first-parent and initial-commit semantics
  are the same code path.
- Rewrites use Git's `-M`/`-C` defaults: 50% similarity, copies from the set of
  modified files only (`CopySource::FromSetOfModifiedFiles`, the cheap mode),
  1000-file limit, empty blobs untracked.
- Line counts come from `Change::diff(cache).line_counts()`; gix returns
  `None` for binary, which stays `None` (never `0/0`). Submodule/non-blob
  sides are `None`. A rename/copy reuses the similarity pass's
  `DiffLineStats`, and an identical pair reports `0/0` with no extra diff,
  so blob diffs are not duplicated. Each computed content diff increments
  the existing `git.file_diffs` counter at one chokepoint. The resource cache
  is cleared (keeping its allocation) after every commit.
- Changes are detached during traversal and diffed afterward, because the
  traversal holds the cache mutably.
- **gix quirk found and fixed:** the rewrite tracker marks a copy's source
  modification as emitted (`gix-diff 0.64 tracker.rs:496–497`), so a file
  edited and copied in the same commit **disappeared** from the file list.
  `restore_copy_source_modifications` rebuilds the `Modified` record from the
  parent tree entry plus the rewrite's `source_id` (with
  `FromSetOfModifiedFiles`, every copy source is a modification). The
  regression test failed before the fix, with only `copy.txt` listed.
- Copy line counts compare against the source's **post-image** (gix's
  `source_id`), so `copy.txt` identical to the pre-image of an edited source
  reports `1/1`.
- Public projection: `RecentCommitFile: From<CommittedFileChange>` maps
  `Renamed | Copied → moved` and keeps `original_path`.
- New items not used until Phase 3 carry
  `#[cfg_attr(not(test), expect(dead_code, reason = "consumed by
  RecentCommits::collect in recent-commits Phase 3"))]`. Because `expect`
  becomes an unfulfilled-expectation warning once the item is used, Phase 3
  must delete these attributes as it wires collection.

### Work Group 2A — Shared Types

- `recent_commits.rs` moved **unchanged** to `recent_commits/mod.rs` (plain
  `mv`, not `git mv`, since this phase does not stage). The legacy code's
  `super::` paths still resolve to `git`. New private submodules are
  `options.rs` and `payload.rs`; Phase 5 finishes the split.
- Public types (re-exported from `filesystem::git` and `filesystem`):
  `RecentCommits` (`#[serde(transparent)]` over `Vec<RecentCommit>`;
  `commits()`, `len()`, `is_empty()`, `to_json() -> serde_json::Value`),
  `RecentCommit`, `RecentCommitAuthor`, `RecentCommitFile`,
  `RecentCommitFileKind`, `RecentCommitFileTypes` (`from_files`),
  `RecentCommitPackages`, `RecentCommitsOptions`, `RecentCommitsVerbosity`,
  `RecentCommitsProjection`, `Selection`, `NamedDate`,
  `DEFAULT_RECENT_COMMIT_COUNT`. `ChangeCategory`/`classify_path` stay under
  `filesystem::path_kind`, where downstream crates already import from.
- Payload decisions:
  - The key is **`bullet_points`** (schema doc and legacy field), not the
    contract matrix's shorthand `bullets`. `hash` is included (schema:
    required); the contract row omitted it.
  - `operation`, `scope`, and `remote` always serialize, as `null` when
    absent. `commit_url`, `original_path`, `added`, and `removed` are omitted
    when absent.
  - `packages`/`package_areas` are one `#[serde(flatten)] attribution:
    Option<RecentCommitPackages>`, so they are structurally emitted together
    (arrays, possibly empty) or omitted together.
  - `datetime` is `DateTime<Utc>`, serialized with
    `to_rfc3339_opts(AutoSi, use_z = false)` → `+00:00`. chrono's default
    serde form would emit `Z`. Deserialization accepts any RFC 3339 offset
    and normalizes to UTC.
  - `file_types` flags also count a `moved` file's `original_path`, because a
    move touches both paths. `other` sets no flag.
  - `to_json()` returns a `serde_json::Value`, whose map sorts keys (the
    workspace does not enable `preserve_order`). Declaration order is
    preserved only by direct `serde_json::to_string(&commits)`. Phase 4's CLI
    should serialize the struct if key order matters for readability.

### Work Group 2D — Runtime Options

- `Selection { Count(usize), Duration(chrono::Duration), NamedDate(NamedDate),
  Date(NaiveDate), Hash(String) }`, defaulting to `Count(10)`.
- **`parse_period` placement:** the parser now lives in `options.rs` as
  `Selection::parse` (+ `FromStr`) with unchanged precedence (named day → ISO
  date → all-digit count, where zero or overflow is rejected → duration →
  hex ≥ 7 chars). The legacy public `parse_period -> PeriodSpecifier` became a
  thin mapping over it, so there is one parser and the CLI keeps compiling.
  Phase 5 should make `parse_period` return `Selection` (or remove it in
  favor of `Selection::parse`) when `PeriodSpecifier` is deleted.
- Builder semantics: the five selectors plus `selection(Selection)` are
  last-wins. `operation` accumulates (OR). `scope`, `author`, `package`,
  `package_area`, and `branch` are single-valued last-wins. `has_file_type`
  accumulates into a `BTreeSet<ChangeCategory>` (AND). Plus `verbosity`,
  `show_author`, `projection`, and `timezone(FixedOffset)`, which defaults to
  `*Local::now().offset()`. There are no public getters; collection and
  rendering read `pub(crate)` fields.
- `count(0)` cannot be rejected in an infallible builder, so crate-private
  `validate()` returns `InvalidPeriod("0")`. Phase 3's `collect` must call it
  first.
- Calendar bounds: `Selection::time_window(now, tz) -> Option<TimeWindow>`
  (crate-private). `Today` = `[local midnight, now]`; `Yesterday` and
  `Date(d)` = `[local midnight, next local midnight)`; `Duration` =
  `[now − d, now]`, independent of the offset; `Count`/`Hash` = `None`. The
  inclusive "now" end preserves the legacy walk's exclusion of future-skewed
  commits for durations.

### Additional decisions for later phases

- **Line counts on every kind.** `RecentCommitFile` carries `added`/`removed`
  whenever the diff is textual, for `added`, `deleted`, and `moved` as well as
  `modified` (like `git diff --numstat`). The schema doc's parenthetical
  "only set when kind is modified" predates `moved` and line counts riding
  along (Decision 1). The contract row requires only "binary/unavailable is
  absent, never 0". To follow the schema literally, drop counts for
  non-`modified` kinds in the `From<CommittedFileChange>` projection
  (`payload.rs`) and update `committed_changes_project_to_public_file_records`.
- **Downstream doc drift fixed:** `sniff/docs/cli/repo_source-code-changes.md`
  and `sniff/docs/cli/repo_dirty-source-code.md` still defined Styling and
  HTML/HTM as source code. Their shipped behavior changed through the wrapper,
  so both definitions now point at `classify_path`. Phase 5 still owns the
  full rewrite of the commit-family CLI docs.
- Contract-matrix rows D1 "rename and copy surface as moved" and "line counts
  binary absent" were planned as LI. They are proven here with in-crate unit
  tests against real in-process git2 repositories, because no public API
  reaches the diff until Phase 3's `collect`. Phase 3's library regression
  should repeat one rename/copy/binary case through `RecentCommits::collect`.

### Requirement → test mapping

| Phase 2 requirement | Tests (all new unless noted) |
| --- | --- |
| Module scaffold + two-level re-exports | `tests/integration.rs::recent_commits_public_surface::{options_and_payload_types_are_reachable_from_both_levels, classifier_is_public_with_serialized_category_names}` |
| Bare array, exact keys/order/values, UTC `+00:00` | `payload::tests::serializes_as_a_bare_array_with_the_exact_keys_order_and_values`, `datetime_parses_non_utc_offsets_into_utc`, `empty_collection_is_an_empty_array` |
| Non-monorepo omits attribution; nullable `operation`/`scope`/`remote`; `commit_url` omitted; empty description/bullets/files present | `payload::tests::non_monorepo_commit_omits_attribution_and_url_but_keeps_nullable_fields` |
| Monorepo empty arrays | `payload::tests::monorepo_commit_without_packaged_files_serializes_empty_arrays` |
| Three-state `remote` | `remote_false_is_serialized_distinctly_from_null` (+ null in the non-monorepo test) |
| Read/write/read round trip; malformed input | `json_round_trips_through_repeated_read_write_read`, `malformed_datetime_is_a_deserialization_error`, `unknown_file_kind_is_a_deserialization_error` |
| `renamed`/`copied` → `moved` + `original_path`; absent line counts | `delta_kinds_normalize_rewrites_to_moved`, `committed_changes_project_to_public_file_records` |
| `file_types` derivation | `file_types_reflect_every_category_touched_including_move_sources` |
| Classifier precedence, CI/CD variants, HTML/CSS/fonts, SVG, `.component.html`, exact-name precedence, `other` | `path_kind::tests::classification::*` (11 tests incl. `#[cfg(windows)]` separators and `#[cfg(unix)]` non-UTF-8) |
| Passive corpus over every registry extension/file name | `registry_corpus_classifies_consistently_with_registry_and_wrappers`, `registry_corpus_covers_every_file_association` |
| Wrappers + intentional HTML/CSS change | `path_kind::tests::source_code_detection::*` (4 inverted/renamed, 1 added), `documentation_detection::{html_file_is_not_documentation, requirements_txt_keeps_its_configuration_file_name_category}` |
| Downstream consumers stay green | worktree + worktree-cli (536 selected incl. `classify_dirty_lines_*`, `dirty_tree::*`), darkmatter `compose::context`/`compose::cache` (254) |
| Rename/copy detection, first parent, initial commit, merges, ordering, corruption | `discovery::committed_file_change_tests::*` (15 tests), including the copy-source regression `copy_keeps_its_modified_source_as_a_separate_record` (failed before the fix) and `legacy_file_listing_keeps_rename_tracking_disabled` |
| Line stats exact; binary ≠ 0; no duplicate diffs | `add_modify_and_delete_carry_exact_line_counts`, `binary_changes_have_no_line_counts_rather_than_zero`, `identical_rename_is_one_renamed_record_without_a_content_diff` / `similar_rename_reuses_similarity_line_counts` (`git.file_diffs == 0`), `content_diffs_are_counted_once_per_diffed_file` |
| Selection parse precedence, count zero rejected | `options::tests::parsing::*` (9 tests); legacy `parse_period_tests` still pass through the delegating wrapper |
| Builder last-wins / accumulation / defaults | `options::tests::builder::*` (6 tests) |
| Calendar bounds `+05:30`/`-08:00`, boundary commit, durations ignore offset | `options::tests::calendar_bounds::*` (7 tests) |
| Doc examples | doctests for `classify_path`, `Selection::parse`, `RecentCommitsOptions` (3 passed) |

### Gates

- `just test` (sniff): **2745 passed, 24 skipped**, exit 0 (Phase 1: 2678;
  the 24 skips are the same pre-existing gated tests).
- `just lint` (sniff): exit 0.
- `cargo clippy -p sniff --features remote --lib --test integration -- -D
  warnings`: exit 0. A wider `--tests` clippy fails on **pre-existing**
  `redundant_closure` errors in untouched `lib/tests/remote_observation.rs`
  (lines 522, 737) and `lib/tests/focused_provider.rs` (line 2528).
- Downstream: `cargo nextest run -p darkmatter -p worktree -p worktree-cli
  --lib --bins -E 'test(/capture|dirty|porcelain|source|tree/)'` → 536 passed;
  `cargo nextest run -p darkmatter --all-targets -E
  'test(/compose::context|compose::cache|changes|docs/)'` → 254 passed.
- `cargo test -p sniff --features remote --doc -- path_kind recent_commits` →
  3 passed.
- `just check-windows` (sniff): compile evidence only, exit 0. Its warnings
  are pre-existing, in untouched `programs/windows_apps.rs`,
  `executable_index.rs`, and `tests/git_parity.rs`.
- No native Windows/WSL/Linux runtime run. The new code is in-process (git2
  trees, pure time math, `Path::components`); the host matrix is Phase 6.
- `node .gitnexus/run.cjs detect-changes --scope all`: risk **critical**,
  driven by the intended classifier change (validated downstream above) plus
  graph noise attributing unrelated flows to Phase 1's uncommitted
  `recent_commits_flag_shadowing` module.
- No `cargo fmt` was run.

## Phase 3

### Work Group 3C — Commit Linking (done first)

Linking was built before collection because collection's enrichment step
calls it, and it has no dependency on the collection core.

- **Impact analysis before editing:** `commit_browser_url` LOW (1 direct:
  `repo hash`); `populate_recent_commit_remotes_from_snapshot` LOW (deep tier
  and a `#[cfg(test)]` wrapper); `select_preferred_remote` LOW (2 direct).
  The CLI-private `parse_git_url`, `build_commit_url_base`, and
  `build_git_status_items` were not in the index (UNKNOWN); text search
  confirmed that every caller is in `cli/src/output/filesystem/mod.rs`.
- **New `filesystem/git/commit_links.rs`** is the single URL and containment
  authority:
  - `parse_remote_identity` (moved from `remote_resolver::parse_identity`,
    which now calls it) parses URL-form and SCP-form remotes into endpoint,
    namespace, and repository.
  - Public `repository_link(url) -> Option<RepositoryLink { owner_repo,
    browser_url }>` and `commit_url(url, sha)` build provider URLs from
    `GitHostingProvider`. The `remote` module's `parse_remote_url` is **not**
    used because `remote` is feature-gated and linking must work in default
    builds.
  - Public `CommitLink { remote: Option<bool>, commit_url }` and
    `api::commit_links_at(path, &[sha])`.
  - Crate-private `link_commits` / `link_commits_with_budget` and
    `COMMIT_VISIT_BUDGET = 5_000_000` (contract.md).
- **One shared ancestry walker:** `remote_refresh::walk_ancestry(repo, tip,
  &mut budget, visit)` now backs both the deep tier's containment and linking.
  The deep tier passes `usize::MAX`, and its behavior (positional cap, skipped
  unreadable ancestors, per-walk target stop) is unchanged. The
  `populate_commit_remotes_*` tests still pass unmodified. Each visit
  increments `git.commit_visits` at this one chokepoint.
- **Tip priority:** remotes in `preferred_remote_order` (new, beside
  `select_preferred_remote`, which is now its first element): `origin`, the
  other non-`upstream` remotes alphabetically, then `upstream`. **Addition
  beyond the spec's wording:** within a remote, the branch that
  `refs/remotes/<remote>/HEAD` points to is walked first, then the rest
  alphabetically. Without this, a tip-rich clone would still walk thousands of
  alphabetically earlier stale `origin/*` branches before `origin/main`, which
  is the spike's finding 3 inside a single remote.
- **Three-state result:** the first containing remote in walk order wins, so a
  target is determined when any walk reaches it, and linking stops once all
  targets are determined. `false` is emitted only when every walk completed.
  An exhausted budget **or an unreadable ancestor** (`AncestryWalkEnd::Failed`)
  leaves undetermined targets `null`, because neither can prove absence. With
  no remote-tracking refs at all, every walk trivially completes, so targets
  are `false`. The URL comes from the winning remote's configured
  `remote.<name>.url`. A self-hosted provider, or a remote-tracking ref with no
  configured remote, gives `remote: true` with no URL.
- **Ref-store errors propagate** as `SniffError::Git`, consistent with
  fallible collection. Linking reads refs with its own
  `RefSnapshot::observe(remote branches only)`; nothing touches the
  fetch-coupled deep-tier path.
- **`commit_browser_url` behavior change:** it is now a thin wrapper over
  `commit_links_at`, so `repo hash` links a commit **only when a
  remote-tracking ref contains it**. Previously it always linked on `origin`,
  producing a 404 for an unpushed commit.
- **CLI migration (Decision 9 deletion):** `parse_git_url`,
  `build_commit_url_base`, and the decoration-prefix "pushed" heuristic in
  `build_git_status_items` are deleted. `repo git-status` gets commit URLs
  from `commit_links_at` and renders commits unlinked if that lookup fails.
  The Remotes section uses `repository_link`, which also fixes
  `ssh://git@host/owner/repo` URLs, which the old parser split on `:`.
- Tests (unit, `commit_links::tests`): `urls::{repository_link_parses_every_url_form,
  repository_link_is_none_without_a_repository_segment,
  commit_url_uses_the_provider_commit_segment}`;
  `containment::{no_remote_refs_determines_every_target_absent_without_walking,
  pushed_and_unpushed_commits_are_true_and_false,
  origin_wins_over_other_remotes_that_also_contain_the_commit,
  alphabetical_non_upstream_wins_without_origin_and_upstream_is_last,
  self_hosted_or_url_less_remote_is_contained_without_a_url,
  skewed_timestamps_do_not_hide_an_ancestor,
  stale_tips_are_walked_after_the_default_branch_finds_the_targets,
  budget_exhaustion_leaves_undetermined_targets_null_not_false,
  budget_spent_on_one_remote_leaves_later_remotes_unwalked,
  empty_target_list_reads_no_refs}`. CLI:
  `cli.rs::test_repo_hash_and_git_status_link_only_remote_contained_commits`.
  It fails against the old origin-only `commit_browser_url`, which linked the
  unpushed commit.
- **Test note:** terminal output wraps long URLs mid-string, so the CLI test
  matches the `[short-hash](https://…/commit/` link prefix rather than the
  full URL.

### Work Groups 3A/3B/3D — Collection, Attribution, Assembly

- **New `recent_commits/collect.rs`:** `RecentCommits::collect(&GitRepo,
  &RecentCommitsOptions)`.
  1. `options.validate()` rejects `Count(0)` before any history is read.
  2. `PackageCatalog::observe` runs `detect_repo_structure` once (skipped
     for bare repos) and builds one `PackageOwnershipIndex`. It validates the
     package filters **before** the walk:
     - `NotAMonorepo` outside a monorepo.
     - `UnknownPackage` / `UnknownPackageArea`, with sorted, de-duplicated
       valid names.
     - `AmbiguousPackage` when the case-insensitive name matches more than one
       entry.
     - The area filter keeps the legacy `area` / `area/…` prefix semantics.
  3. `resolve_tip`: `HEAD` (unborn yields an empty `Ok`) or the branch, which
     resolves as `refs/heads/<b>`, then `refs/remotes/<b>` (e.g.
     `origin/feature`), then `refs/remotes/<remote>/<b>` over **configured**
     remote names in preferred order. An invalid reference name, or no match,
     is the new `SniffError::UnknownBranch { name }`.
  4. Hash selection uses `discovery::resolve_single_opt` (now `pub(crate)`)
     and a merge-base ancestor check. Both "no such object" and "not an
     ancestor of the tip" are `HashNotReachable`; ambiguous or corrupt lookups
     stay `Git` errors. The walk stops after including the target, as the
     legacy walk did.
  5. One `ByCommitTime` walk. Out-of-window commits are skipped with
     `continue`, never `break` (skewed timestamps). Message filters come first
     (operations OR'd, `eq_ignore_ascii_case`; non-conventional commits
     excluded when set; scope case-insensitive; author a Unicode-lowercased
     substring of name or email). Then, only when a path filter is active, the
     cheap untracked `get_commit_files_with_cache_fallible` listing feeds the
     package/area (deepest owner) and file-type (AND over `classify_path`)
     filters. That listing has the same path set as the rename-aware diff,
     because a rewrite's source and destination are both present. A count
     selection stops at N **matches**.
  6. Enrichment runs only after the candidate set is final: one
     `link_commits` call, then `committed_file_changes_with_cache` per
     survivor, `ParsedMessage::parse`, `RecentCommitFileTypes::from_files`,
     and `PackageCatalog::attribute`.
- **Attribution:** deepest owner only (nested packages), sorted
  de-duplicated arrays, and a moved file's `original_path` counts. Area-root
  files stay unattributed, and a non-monorepo gives `None`, so both keys are
  omitted.
- **Scope filter drift fixed:** the Phase 2 builder doc said scope "equals";
  collection matches case-insensitively, consistent with operation and package
  matching, and the doc now says so.
- **Message parsing (`ParsedMessage`, Decision 15):** operation and scope come
  from `ConventionalCommit::parse` on the subject line only. The heading is the
  subject after any prefix, up to its first `.`. The description is the rest
  of that line plus body prose (lines and paragraphs joined by single spaces)
  until the first `- `/`* ` bullet. **Implementation choices the spec did not
  rule on:** a non-blank line directly under a bullet continues that bullet
  (wrapped bullets), and paragraphs after a blank line following the bullets
  are dropped, which keeps trailers such as `Co-authored-by:` out of the
  payload. The legacy `parse_commit_message` is untouched and still serves
  `CommitDescSet` until Phase 5.
- **Non-UTF-8 policy:** the message, author name, and author email go through
  lossy UTF-8 (U+FFFD), matching `CommitInfo`. No panic and no error.
- **Private report context:** `RecentCommits` now carries a `#[serde(skip)]
  repo_root: Option<PathBuf>`, set by `collect`, so the Phase 4 renderer can
  build `file://` links while JSON stays a bare array. Its crate-private
  `repo_root()` accessor has `#[expect(dead_code)]` until Phase 4 uses it.
  Deserialized collections have `None`, so a round trip compares
  `commits()`, not the whole struct.
- **Removed Phase 2 `expect(dead_code)` markers:** `from_commits`,
  `time_window`, `TimeWindow::contains`, `validate`, `committed_rewrites`,
  `committed_file_changes_with_cache`, `restore_copy_source_modifications`,
  and `committed_file_change`.

### Requirement → test mapping

| Phase 3 requirement | Tests |
| --- | --- |
| Count / default last-10 / exact JSON fields | `recent_commits.rs::selection::default_is_the_last_ten_commits_as_the_exact_bare_array_schema` (full object equality, UTC `+00:00`, `remote: false`, no `commit_url`/attribution keys) |
| Count zero rejected; count > history | `selection::count_zero_is_rejected_before_any_history_is_read` (0 commit visits), `count_larger_than_history_returns_every_commit` |
| Empty success (unborn HEAD, no matches) | `selection::empty_matches_and_unborn_head_are_successful_empty_collections` (`to_json() == []`) |
| Duration | `selection::duration_keeps_only_commits_inside_the_window` |
| Specific date, local day, ± offsets, UTC JSON | `selection::specific_date_is_one_local_day_in_the_options_offset` (`-08:00`, `+05:30`, UTC-empty) |
| Today / yesterday local days | `selection::today_and_yesterday_are_local_calendar_days_in_the_options_offset` (offset chosen so now = local noon, so the test is not midnight-flaky) |
| Hash through tip; unknown/unreachable hash | `selection::hash_selects_from_the_tip_through_the_hash_inclusive` (abbreviated), `unknown_or_unreachable_hashes_are_typed_errors` |
| Branch base, local-first, remote fallback, unknown | `selection::branch_is_a_history_base_with_local_first_then_remote_tracking_fallback` |
| Count filters during traversal; enrichment after filtering | `filters::count_is_satisfied_by_matching_commits_found_during_the_walk` (the contract's exact `fix,feat,feat,fix,feat` input; `git.file_diffs == 2`). Mutation: stopping after N *visited* commits fails it. |
| Operations OR, case-insensitive, non-CC excluded | `filters::repeated_operations_match_any_case_insensitively_and_exclude_non_conventional` |
| Filters AND | `filters::scope_and_operation_filters_combine_with_and`, `attribution::package_and_area_filters_use_the_same_deepest_owner_as_attribution` |
| Author substring name OR email, captured name + email | `filters::author_is_a_case_insensitive_substring_of_name_or_email` (`EXAMPLE.com` email-only, `ada` name-only) |
| File-type filters | `filters::file_type_filters_require_every_listed_category` |
| Rename / copy / binary through collect | `files::rename_and_binary_changes_surface_through_collection` |
| Merge first-parent; no-change merge `files: []` | `files::merges_are_included_with_first_parent_files_and_no_change_merges_have_none` |
| Non-UTF-8 metadata | `files::non_utf8_author_and_message_bytes_are_replaced_not_rejected` (raw ISO-8859-1 commit object) |
| Monorepo arrays, deepest package, area-root unattributed | `attribution::monorepo_commits_carry_deepest_package_arrays_and_area_files_stay_unattributed` |
| Non-monorepo omits keys | `attribution::non_monorepo_commits_omit_both_attribution_keys` |
| Unknown package/area typed errors listing names; NotAMonorepo | `attribution::unknown_package_or_area_is_a_typed_error_listing_valid_names`, `package_filter_outside_a_monorepo_is_not_a_monorepo_error` |
| Structure tier only (no inventory/docs/enrichment/lockfile, no spawn/provider) | `attribution::attribution_uses_the_structure_tier_without_inventory_language_or_doc_walks`, with a full-tier `detect_repo` control. Mutation: swapping in `detect_repo` fails it. |
| Links through collect; one ref walk; no fetch/provider | `linking::pushed_commits_link_to_the_containing_remote_without_network_work` (`git.ref_walks == 1`, `process.spawns == 0`, `remote.api_requests == 0`), `empty_collection_performs_no_linking`, `self_hosted_remote_is_contained_without_a_commit_url` |
| Read/write/read round trip | `recent_commits.rs::collected_json_round_trips` |
| Message parsing (Decision 15 cases) | `collect::tests::message_parsing::*` (9 tests: `e.g.` truncation, punctuation-free, multi-paragraph, bullet-only, wrapped bullets/trailers, non-conventional, subject-only operation, empty) |
| Linking (priority, budget, null vs false, skew, no URL) | `commit_links::tests::*` (13 tests, listed under 3C) |
| Deep tier unchanged after walker extraction | existing `remote_refresh::tests::populate_commit_remotes_*` (4, unmodified) |

### Gates

- `just test` (sniff): **2796 passed, 24 skipped**, exit 0. Phase 2 had
  2745; the +51 are exactly this phase's new tests (13 `commit_links`, 9
  message parsing, 28 library integration, 1 CLI), so every new test was
  selected. The 24 skips are the same pre-existing gated tests.
- `just lint` (sniff): exit 0.
- `cargo clippy -p sniff --features remote --lib --test recent_commits --test
  integration -- -D warnings` and `cargo clippy -p sniff-cli --lib --tests --
  -D warnings`: clean. The wider `--tests` pre-existing `redundant_closure`
  failures noted in Phase 2 were not re-run.
- Doctests: `cargo test -p sniff --features remote --doc -- commit_links
  recent_commits remote_resolver` → 4 passed (includes the new
  `repository_link` example; `collect`'s example is `no_run`).
- Downstream: `resolve_remote_at`/`preferred_remote_url` callers.
  `cargo nextest run -p darkmatter -E 'test(/provider|remote/)'` → 206
  passed; `cargo check -p rendezvous-daemon` → ok.
- `just check-windows` (sniff): exit 0, compile evidence only. The only
  warnings are the pre-existing ones in `windows_apps.rs` and
  `executable_index.rs`.
- **Cross-OS runtime:** not run. The new code is in-process gix/git2 plus the
  same `PackageOwnershipIndex::lookup_relative` path lookup the legacy
  attribution used, so there is no new path-separator, subprocess, or
  terminal surface. The non-UTF-8 test was deliberately left ungated so every
  CI OS runs it. The host matrix belongs to Phase 6.
- `node .gitnexus/run.cjs detect-changes --scope all`: risk **critical**, 21
  files. The text report attributes affected flows to Phase 1's uncommitted
  `recent_commits_flag_shadowing` module (graph noise) and Phase 2's intended
  classifier change. The index predates this phase's new files (`commit_links.rs`,
  `collect.rs`), so they are not listed; Phase 6's Graph review re-indexes
  (`just gitnexus`) and re-runs untruncated.
- No `cargo fmt` was run.

### Documentation drift fixed

- `sniff/docs/cli/repo_hash.md` and `sniff/docs/cli/repo_git-status.md`
  described origin-only and ahead-count "pushed" linking. Both now describe
  containment-based linking through the library.
- `.claude/skills/sniff/architecture.md` gained "Recent commits and commit
  links".

## Phase 4

### Work Group 4A — Library Rendering

- **New `recent_commits/render.rs`.** A single layout walk (`layout`) builds
  `Line`s (`Blank`, `Heading`, `Text(Vec<Span>)`), where each `Span` has a
  style (`Plain`, `Bold`, `Italic`, `Operation`, `Scope`) and an optional
  link. `to_prose`, `to_markdown`, and `to_plain` fold the same lines:
  - Prose uses `<bold>`, `<italic>`, `<blue>`, and `<blue><dim>` tags, plus
    `[text](url)` links.
  - Markdown uses `**`, `_`, and `[text](url)`, and drops color.
  - Plain is raw text with no escaping and no links.
  - A crate-private `render(options, now, format)` takes an injected clock,
    so fixtures are exact.
- **Layout** (normative for Phase 5 docs):
  - Compact: one header per commit, with no blank lines between commits.
  - Normal: the header, then `  **Files Impacted:**` and `  - {kind}: {path}`
    lines. A no-change merge renders `  **Files Impacted:** none`. Commits are
    separated by one blank line.
  - Verbose: the header, then `  {description}` (if non-empty), then a blank
    line, `  **Details:**`, a blank line, and `  - {bullet}` lines (if any),
    then a blank line (only if either was written), then the files block.
  - Header: `- [{hash7}] {op}({scope}) [by {author}] at {h:mm}{am|pm} {Today|Yesterday|YYYY-MM-DD}: {heading}`.
    The day is judged in `options.timezone`. Author is the name, or the email
    when the name is empty.
  - Styles follow Decision 12: the hash is bold and links to `commit_url` only
    when present. The operation and the scope's parentheses are blue, and the
    scope is blue plus dim. `at` is italic; the time and day are bold; labels
    are bold.
  - Files link to `file://` through `url::Url::from_file_path(repo_root.join(path))`,
    which gives `file:///C:/…` on Windows. **Deleted files are not linked**
    (the target is gone), and a deserialized collection (no `repo_root`)
    renders unlinked files. A `moved` file renders
    `moved: {path} (from {original_path})`.
- **Escaping:** Prose and Markdown backslash-escape `\ * _ [ ] < > \``
  in every span (dynamic text and the literal `[`/`]` around the hash). Link
  targets percent-encode `( ) space < >` so a path such as `dir (copy)/x.md`
  cannot close the link early. Prose honors these escapes; the CLI test
  `escaped_markup_in_commit_text_renders_literally` proves a heading
  containing `<red>now</red> and _em_` renders literally through `Prose`.
- **Projection decision (not ruled by the spec):** new public
  `RecentCommits::projected(RecentCommitsProjection) -> RecentCommits` is the
  single pruning authority for text and JSON. It keeps files whose path
  **or** original path is in the category, and drops commits left with no
  files. Commit-level `file_types`, attribution, `remote`, and `commit_url`
  stay whole-commit facts. `All` is the identity, so it keeps no-change
  merges. The sibling presets are **projections, not collection filters**:
  `source-code-changes` collects the selection (last 10 by default) and
  prunes it, like the legacy behavior. This lets Phase 5's aggregate collect
  once and project three times, and still byte-match the focused commands
  (Decision 10 plus plan 5A). The renderer re-applies `options.projection`
  (idempotent) and adds a `Source Code Changes` / `Documentation Changes`
  heading line (Prose `<bold>`, Markdown `##`, plain bare text) when any
  commit remains.
- Removed the `#[expect(dead_code)]` on `RecentCommits::repo_root()`. Fixed
  the drifted `RecentCommitsProjection` doc ("JSON output ignores it"): it
  now points at `projected`.

### Work Group 4B — CLI Arguments

- **New `cli/src/args/recent_commits.rs`:** one `RecentCommitsArgs` shape
  (`clap::Args`) that all three commit-family subcommands use as tuple
  variants (`RepoSubcommand::RecentCommits(RecentCommitsArgs)` and the same
  for `RepoAction`). Flags: `[period]`, repeatable free-form `--operation`
  (with completion candidates), `--scope`, `--author`, `--branch`,
  `--package`, `--package-area`, `--source-code`, `--web`, `--images`,
  `--documentation`, `--configuration`, `--cicd`, `--show-author`,
  `-v/--verbose` (`u8` `Count` with the global id, per the Phase 1 spike), and
  `-c/--compact`.
- Removed `--action`, `RecentCommitActionArg`, `--no-error`, and
  `--on-error` from all three subcommands.
- `RecentCommitsArgs::to_options(projection)` is plain flag-to-builder
  plumbing: the period goes through `Selection::parse`, and compact beats
  verbose. **The adapter resolves `sniff -v repo recent-commits -c` as
  compact.**
- The `repo` after-help no longer says "Show commits from last 3 days".
- `recent_commits_flag_shadowing.rs` now runs against the real `Cli`
  (16 tests). It covers:
  - Defaults are `RecentCommitsOptions::new().count(10)` for every
    subcommand.
  - Every flag maps to its builder method (whole-options equality).
  - Each file-category flag sets its own category.
  - Period forms parse, and invalid or `0` periods are `InvalidPeriod`.
  - Verbose is position-independent, repeated `-v` does not sum, and `-v` never
    enables `--debug`.
  - Compact works, the subcommand-level conflict holds, and global `-v` plus
    `-c` resolves to compact.
  - `--action`, `--no-error`, and `--on-error` fail with `UnknownArgument` on
    all three subcommands.
  - Long help advertises every new flag and none of the removed ones.
  - Operation completion suggests common words and still accepts `planning`.
  The two rejected-shape guards are kept. In `args/mod.rs`, the three
  `--action` parse tests became one repeated-operation test across the three
  subcommands.

### Work Group 4C — Thin CLI

- **`cli/src/output/recent_commits.rs` rewritten** (252 → about 55 lines):
  parse the options, `GitRepo::discover` (or `NotARepository`), then
  `RecentCommits::collect(...).projected(projection)`. Then:
  - `--json`: `print_json_value(report.to_json())`. `--perf` keeps the repo-wide
    `{data, performance}` wrapper, which is still valid JSON on stdout.
  - Empty result: stdout stays empty, `No commits matched.` goes to stderr,
    exit 0. JSON prints `[]` with nothing on stderr.
  - `--plain`: `to_plain`.
  - Terminal: one `Prose::new(to_prose).with_word_wrap(WrapProse(None, None)).render(&Terminal::default())`
    (the contract's spike finding 3).
  - Typed library errors (`InvalidPeriod`, `UnknownBranch`,
    `HashNotReachable`, `UnknownPackage`/`Area`, `NotAMonorepo`) propagate
    through `?` to stderr with a non-zero exit, and stdout stays empty.
- **Deleted `cli/src/output/commit_blocks.rs`** (the styled renderer, the
  `filter_commit_set` URL/label helpers, and its 4 unit tests). The CLI's
  `url`, `darkmatter`, and `chrono` dependencies are still used elsewhere, so
  no manifest change.
- **Aggregate bridge until Phase 5:** `repo_json.rs` imported
  `commit_family_value` and `RecentCommitsMode` from the deleted CLI code. The
  legacy `CommitDescSet` pruning was inlined into
  `aggregate_commit_family_value(commit_set, Option<fn(&Path) -> bool>)`
  using `is_source_code_path` / `is_documentation_path`. It is byte-identical
  to the old aggregate output, so the aggregate snapshot and tests are
  unchanged. Its doc says Phase 5 deletes it.
- `recent_commits_prose_layout.rs` (the Phase 1 spike) now renders real
  `RecentCommits::to_prose` output from a deserialized payload instead of the
  hand-authored stand-in. It gains
  `escaped_markup_in_commit_text_renders_literally`.
- **Bug found by manual run and fixed:** the first escape set included the
  backtick for Prose as well. Prose has no backtick escape, so a real
  commit's `` `just complete` `` rendered as `` \`just complete\` ``. Now
  only Markdown escapes backticks. The Prose-layout fixture carries a
  backtick, so the "no visible backslash" assertion guards the regression.
- **Old CLI integration tests replaced** (`cli/tests/cli.rs`, about 40 tests
  pinning the envelope, `--action`, `--no-error`, exit-1 empties, and
  `**Files Impacted:**` in plain output) with 12 contract tests, listed below.
  The `--json --perf` test now asserts `data` is an array.
- **New L2** `cli/tests/level2_recent_commits_rendering.rs`: an owned 60-column
  tmux pane runs the real binary against a temporary repository with
  `recent-commits -v`. It checks:
  - the description wraps at word boundaries across more than one row;
  - bold and blue SGR are present;
  - an OSC8 link or its `](file` fallback is present;
  - no Prose tags, `\[`, `\_`, or `**` leak.

  It polls with `capture_until` against a nonce marker, with no fixed sleep.
  **Mutation check:** removing `with_word_wrap` makes it fail with
  "description must wrap at word boundaries".

### Requirement → test mapping

| Phase 4 requirement | Tests |
| --- | --- |
| One layout walk; compact/normal/verbose exact fixtures | `render::tests::layouts::{compact_prose_is_one_styled_header_per_commit, normal_prose_lists_files_with_links_and_marks_empty_merges, verbose_prose_adds_description_and_details_before_files, verbose_without_commentary_matches_normal, description_without_bullets_is_followed_by_one_blank_line}` |
| Markdown: links/emphasis, no color | `layouts::verbose_markdown_uses_links_and_emphasis_without_color_tags` |
| Plain: no tags, markers, links, or escapes | `layouts::verbose_plain_has_the_same_text_without_markup_or_links`; CLI `test_recent_commits_plain_output_is_the_library_plain_report` |
| Hash linked only with `commit_url`; `file://` links (Windows drive form under `cfg!(windows)`); deleted files unlinked; no root → unlinked | `layouts::normal_prose_…`, `layouts::deserialized_collections_render_files_without_links`; `prose_layout::links_fall_back_to_markdown_without_osc8_support` |
| `show_author` adds the author to the header only | `layouts::show_author_adds_the_name_to_the_header_only`; CLI `test_recent_commits_plain_output_…` (`by Test at`); CLI `test_commit_family_json_ignores_display_flags` |
| `to_json()` invariant under verbosity/show-author | CLI `test_commit_family_json_ignores_display_flags` (byte-equal stdout, all 3 commands, `-v --show-author` and `-c`) |
| Timezone-aware day labels | `render::tests::dates::{day_labels_use_the_options_timezone, older_commits_show_the_local_date}` |
| Empty-file merge honored | `layouts::normal_prose_…` (`Files Impacted: none`); CLI `test_recent_commits_no_change_commit_is_reported_with_no_files` |
| Escaping of dynamic text | `render::tests::escaping::{prose_and_markdown_escape_dynamic_text_and_plain_keeps_it_verbatim, link_targets_cannot_close_early}`; `prose_layout::escaped_markup_in_commit_text_renders_literally`; CLI `test_recent_commits_terminal_output_renders_library_prose` (exact input `fix(cli): handle <red>tags</red> and _em_ literally`) |
| Sibling projection pruning and headings | `render::tests::projections::*` (5); CLI `test_source_and_documentation_changes_project_their_files` |
| Flag mapping, repeats, conflicts, count-10 default, global/subcommand `-v` | `recent_commits_flag_shadowing::*` (16); `args::tests::…::commit_family_subcommands_parse_repeated_operations` |
| Removed flags not accepted or advertised | `removed_flags_are_rejected_on_every_commit_family_subcommand`, `help_advertises_the_new_flags_and_not_the_removed_ones` |
| Count-10 default through the process | CLI `test_recent_commits_defaults_to_the_last_ten_as_a_bare_json_array` (12 commits → 10; exact key set) |
| Empty results exit 0; `[]`; note only on stderr | CLI `test_commit_family_empty_results_succeed_with_empty_output` (3 commands × json/terminal/plain) |
| Typed failures: stderr, non-zero exit, empty stdout | CLI `test_commit_family_invalid_inputs_are_typed_failures` (invalid/zero period, unknown branch, unreachable hash, NotAMonorepo, unknown package/area × 3 commands) |
| Library filters and branch base through the CLI | CLI `test_recent_commits_filters_run_through_the_library`, `test_recent_commits_branch_is_a_history_base`, `test_recent_commits_hash_period_walks_back_to_the_hash`, `test_recent_commits_package_filters_and_attribution_in_a_monorepo` |
| Terminal output is library prose rendered once | CLI `test_recent_commits_terminal_output_renders_library_prose`; `prose_layout::*` (7); L2 `level2_recent_commits_verbose_report_wraps_styles_and_links_in_tmux` |
| Corrupt history still fails | existing `test_repo_{recent_commits,source_code_changes}_surfaces_corrupt_history` (unchanged, passing) |

### Gates

- `just test` (sniff): **2797 passed, 24 skipped**, exit 0. The log confirms
  every new test was selected: 20 render, 16 flag shadowing, 7 prose layout,
  1 args, and 12 CLI integration tests. The 24 skips are the pre-existing gated
  tests.
- `just lint` (sniff): exit 0. `cargo clippy -p sniff --features remote --lib
  --tests -D warnings` and `cargo clippy -p sniff-cli --features test-fixtures
  --lib --tests -D warnings`: clean.
- `just test-l2` (sniff): 5 passed (4 existing plus the new one), with tmux,
  WezTerm, and Apple Terminal backends spawned.
- `just check-windows`: exit 0, compile evidence only. The only warnings are
  the pre-existing `windows_apps.rs`, `executable_index.rs`, and
  `git_parity.rs::stage_raw_path`.
- **Native Windows runtime: blocked.** `just cross-check sniff --os windows`
  failed before running tests because `build-win-native` is out of disk
  (`scp: write remote "W:/ci-verification/…": Failure`, then
  `fatal: write error: No space left on device`). The host was not cleaned in
  this session. See the `storage-strategy` skill.
- **Linux runtime: blocked.** `just cross-check sniff --os linux` waited on
  the `build-linux` lock, which is held by `nightly-reward-spike` (owner
  `reward-20260914-c3e60d0`, started 2026-09-14T18:25:30Z). The run was
  stopped rather than breaking another job's lock. WSL was not attempted.
- Manual runs of the built binary against this repository (terminal, `--plain`,
  `--json`, sibling, `--show-author`) looked correct after the backtick fix.
- GitNexus: `impact` on `handle_recent_commits_command`, `commit_family_value`,
  `filter_commit_set`, `render_commit_set_styled`, and
  `aggregate_commit_family_value` returned "not found"/UNKNOWN (the index
  predates these files), and `RecentCommitActionArg` returned UNKNOWN with no
  callers. Text search confirmed every caller was in `commands/mod.rs`,
  `output/{recent_commits,repo_json,commit_blocks}.rs`, and `args`, and all
  were migrated. `detect-changes` was not re-run (no commit in this phase).
  Phase 6 re-indexes.
- No `cargo fmt` was run.

### Documentation not changed in this phase

The public CLI docs (`sniff/docs/cli/repo_recent-commits.md`,
`repo_source-code-changes.md`, `repo_documentation-changes.md`) and the topic
docs still describe the old envelope, `--action`, the 3-day default, and exit-1
empties. Plan Work Group 5C owns that rewrite, so they now **drift from shipped
behavior until Phase 5**. The layout and projection facts above are the source
for that rewrite.

## Phase 5

### Impact analysis before editing

- GitNexus was re-indexed first (`just gitnexus`, up to date).
- `project_repo_aggregate`: **CRITICAL**. Most listed callers are
  name-collision noise (`handle_sony_input`, `research`), but this function
  really is the `sniff repo --json` core.
- `detect_filesystem_with_request_inner`: **HIGH**. It is the core of every
  filesystem detection, so the change is confined to its `collect_aggregate`
  branch.
- `link_commits_with_budget`: **HIGH**, internal. Behavior is preserved, and
  its tests pass unmodified.
- `parse_commit_message`: MEDIUM. Its one production caller outside the
  module is CLI `repo git-status`.
- `observe_aggregate_evidence` and `aggregate_commit_family_value`:
  UNKNOWN. Text search confirmed their callers are `filesystem/mod.rs` and
  `repo_json.rs` only.

### Work Group 5A — Aggregate Pipeline

- **One collection, reusing the request's observations.**
  `observe_aggregate_evidence(structure: Option<&RepoInfo>)` now calls the new
  crate-private `RecentCommits::collect_observed(repo, &RecentCommitsOptions::new(), structure, &refs)`.
  - `refs` is the `RefSnapshot` the aggregate already took for branch facts.
    `commit_links::link_commits_from_snapshot` is new, and
    `link_commits_with_budget` now observes refs and delegates to it.
  - `structure` is the `RepoInfo` this request's repo detection already
    produced. `PackageCatalog::observe` became `PackageCatalog::new(root, info, options)`,
    and `collect` still runs `detect_repo_structure` itself.
- **Design decision (not ruled by the spec):** aggregate evidence moved off the
  Git thread. It now runs after the Git and repo-detection threads join
  (`filesystem/mod.rs`), because commit attribution needs the joined
  `RepoInfo`.
  - The alternative, a fresh `detect_repo_structure` on the Git thread, would
    parse every manifest twice. The mutation run below shows 3 → 6 manifest
    parses.
  - Cost: branch, worktree, and history evidence no longer overlap repo
    detection in wall-clock time. Work counts are unchanged.
- **Attribution source:** the aggregate attributes through the focused-tier
  `RepoInfo`, the same source the deleted second pass used. Its package list
  is the structure-tier list plus manifest facts. `PackageCatalog` only reads
  `root`, `packages[].{path,relative,name,package_area}`, and `is_monorepo`.
- **Array projection.** `RepoAggregate.commits` is `RecentCommits`. The CLI
  emits `commits.to_json()` and `commits.projected(SourceCode|Documentation).to_json()`.
  `aggregate_commit_family_value` and the `period`/`filter`/`repo_root`/`packages`
  envelope surgery are deleted.
- **Snapshot.** `snapshots.rs::stable_aggregate_json` now keeps each family
  array minus `hash`/`datetime`, replacing the old `commit_family_keys` that
  read `period.label`/`filter`. The diff was reviewed by hand: one commit,
  monorepo `packages`/`package_areas` arrays, `remote: false`, and siblings
  pruned to `.rs` and `.md` files.
- **Offline counters (compatible evidence).** Both runs used
  `sniff --perf repo --json` on this repository, on this host, with the same
  request shape. "Before" is the installed 2026-09-11 build, which still emits
  the `last 3d` envelope; "after" is this phase's debug build.

  | counter | before | after |
  | --- | --- | --- |
  | `git.repository_discoveries` / `status_walks` / `ref_walks` | 1 / 1 / 1 | 1 / 1 / 1 |
  | `filesystem.repo.manifest_parses` | 82 | 82 |
  | `docs.documents_parsed` / `walk.walks_started` / `process.spawns` / `remote.api_requests` | 0 | 0 |
  | `git.commit_visits` | 13,071 | **153,323** |
  | `git.file_diffs` | 39 | 194 |
  | families (recent/source/docs) | 99 / 20 / 87 commits | 10 / 6 / 9 |

  - **Where the new visits come from:** containment. This branch has unpushed
    commits, and `remote: false` is emitted only after every remote-tracking
    tip (14 refs here) has been walked to exhaustion. Both `true` and `false`
    appear in the output. The total is 3% of `COMMIT_VISIT_BUDGET`.
  - **`file_diffs` is per file,** so a few large commits in the last 10
    outweigh the old 99-commit window's smaller ones.
  - **Wall time:** the debug build took 1.9–2.4 s for the whole command.
  - Spec Decision 10 accepted this every-invocation containment growth, and the
    Implementation Notes asked for this comparison.
- **Budget exhaustion.** The aggregate uses the production budget, so no
  fixture can exhaust it end to end. The evidence is layered instead:
  - `commit_links::a_shared_branch_snapshot_links_like_a_fresh_one_without_a_ref_walk`
    shows the aggregate's (local + remote) snapshot giving identical links at
    budget 8 and at the production budget, `[true, null, null]` at 8, and zero
    ref walks.
  - `repo_json::aggregate_projects_supplied_facts_verbatim` builds an aggregate
    around a payload with `remote: null` and asserts that valid JSON carries
    `null` through every family.

### Work Group 5B — Legacy Cleanup

- **Deleted** from `recent_commits/mod.rs` (1,691 → 13 lines; it is now module
  declarations and re-exports only):
  - the five `get_recent_commits_*` functions plus
    `get_recent_commits_by_duration_with_repo`;
  - `CommitDesc`, `CommitDescSet`, and `CommitFileChange`;
  - `describe`/`source_code_changes`/`documentation_changes` rendering;
  - `filter_by_actions`/`filter_by_package`/`filter_by_package_area`;
  - `attribute_from_repo` (the aggregate's second pass) and
    `attribute_commit_files`;
  - `AGGREGATE_COMMIT_WINDOW_DAYS` (`git/types.rs`);
  - `PeriodSpecifier` and `parse_period`.
- **Decision on `parse_period`:** the spec says it "stays in the library beside
  `Selection`". That parser is `Selection::parse` (Phase 2). `parse_period` had
  become a zero-caller mirror returning the duplicate `PeriodSpecifier` enum, so
  it was deleted rather than left as a compatibility shim. Its 21 unit tests
  duplicated `options::tests::parse::*`.
- **Moved, not deleted:** `parse_commit_message` is now in `git/types.rs`
  beside `ConventionalCommit`, with its 5 tests, because CLI `repo git-status`
  still uses it. The public path `sniff::filesystem::git::parse_commit_message`
  is unchanged. Its doc now states that recent-commits reports use their own
  split.
- **Re-exports:** removed from both `filesystem::git` and `filesystem`.
- **Tests migrated:**
  - `git_parity.rs`: five legacy corrupt-history tests became
    `recent_commits_collection_surfaces_corrupt_history_for_every_selection`
    (count, duration, date, today, hash).
  - `integration.rs`: about 870 lines of legacy recent-commit tests were
    removed; they were duplicated by `tests/recent_commits.rs`.
  - Two behaviors were not covered by the new suite and were ported to
    `recent_commits.rs`: `selection::a_skewed_old_head_does_not_hide_in_window_ancestors`
    and `selection::an_empty_commit_is_a_hash_boundary_with_no_files`.
- **Bench target:** `lib/benches/cases/git_ops.rs` still called the deleted
  functions, which `just test` and `just lint` do not compile.
  - `revwalk_recent_gated` now collects with `hash(<midpoint commit>)`, and
    `revwalk_recent_full` collects with `count(depth)`.
  - The doc records that both IDs now measure the full collect pipeline, so
    older baselines are not comparable.
  - Verified with `cargo test -p sniff --features network,bench-internals --bench perf -- --test revwalk`
    (4 × Success).
- **URL consolidation.** Phase 3 already removed the CLI parser and pushed
  heuristic. This phase verified the callers:
  - `repo hash` goes through `commit_browser_url`, then `commit_links_at`.
  - `repo git-status` uses `commit_links_at` and `repository_link`.
  - Recent commits and the aggregate go through `link_commits*`.
  - Preferred-remote identity goes through `remote_resolver`, then
    `parse_remote_identity`.
  - Text search found no URL parsing left in `cli/src` outside test fixtures.
- **Parsers deliberately not consolidated** (out of scope; none of them builds
  a commit link):
  - `types.rs::parse_org_repo`, which feeds `GitInfo.org`/`repo`.
  - `types.rs::extract_remote_host`, used for provider classification.
  - `repo/identity.rs::parse_remote_basename`, which also accepts local paths.
  - `remote_observation.rs::remote_git_path` and `remote::parse_remote_url`,
    both behind the network or `remote` feature.
- **Pre-existing bug observed, not fixed:** `parse_org_repo` splits an
  `ssh://git@host:22/owner/repo` URL on the last `:`, so `org` comes out as
  `"22"`. Not recorded in a skill; see `message_to_agent`.
- **Dependency audit:** no manifest dependency changed, so
  `sniff/docs/dependencies.md` is untouched.
  - `cargo machete lib cli` flags only `ec4rs`. It predates this feature (added
    in 7867f8b88 with EditorConfig support) and is untouched here.
  - `cargo tree -p sniff-cli -i chrono-tz` finds no match: no `chrono-tz` and no
    new network dependency. (`chrono-tz` in `Cargo.lock` belongs to `claudine`.)

### Work Group 5C — Documentation

A Documenter sub-agent did the rewrite from the Phase 3/4 log facts. I checked
its output against the code, `--help`, and a grep for banned claims.

- **Rewritten:**
  - `docs/topics/repo/recent-commits.md`: non-generic builder, last-wins,
    `FixedOffset`, three-state `remote`, a 5,000,000-visit budget, Prose-tag
    styling table, layout, and escaping.
  - `docs/topics/repo/recent-commits-schema.md`: `files` is `min(0)`, and the
    document states payload optionality.
  - `docs/cli/repo_recent-commits.md`, `repo_source-code-changes.md`, and
    `repo_documentation-changes.md`.
- **Updated:** `docs/cli/repo.md` (new Commit History table and aggregate
  families), `docs/topics/json-output.md`, `cli/README.md`, and
  `lib/README.md`.
- The dangling Darkmatter prose-grammar link now points to
  `../../../../biscuit-terminal/docs/components/prose.md`; the path resolves.
- **Remaining grep hits for `--action`/`--no-error`/`CommitDescSet`/`period_label`:**
  only explicit "removed" or "breaking change" migration notes, plus `3d` used
  as a duration example. None claims current behavior.
- **Doc/spec drift the agent resolved to the code** (the code is correct):
  - JSON uses `bullet_points`, not the contract's `bullets`.
  - `remote: null` also covers an unreadable ancestor.
  - A hash needs at least 7 hex characters.
  - Within a remote, the default branch is walked first.
  - Global and subcommand `-v` resolve as recorded in Phase 4.
- Skills: `.claude/skills/sniff/architecture.md` gained the aggregate
  collection facts, the counter expectation, and the list of non-linking
  parsers. `.claude/skills/sniff/cli.md` gained the aggregate/focused equality
  note.

### Requirement → test mapping

| Phase 5 requirement | Tests |
| --- | --- |
| Aggregate families equal focused bare arrays (same options), last 10 regardless of age, author, file types, attribution, links, pruning | CLI `cli.rs::test_repo_aggregate_commit_families_match_the_focused_commands`: 11 commits dated 2020 (outside any 3-day window), monorepo, `origin/main` at a mid commit with a GitHub URL. Asserts full array equality for all three families, `remote` true + `commit_url` / false, packages, empty-commit `files: []`, sibling pruning counts, and whole-commit `file_types` |
| Empty families are `[]` | CLI `test_repo_aggregate_embeds_empty_commit_families_as_empty_arrays` |
| Aggregate collection is the default-options collection | `repo_json::tests::aggregate::aggregate_carries_the_default_commit_family_set` (equals `RecentCommits::collect(.., new())`) |
| Aggregate leaves are bare arrays, no envelope | `aggregate_commit_family_leaves_are_bare_arrays`, `aggregate_does_not_duplicate_full_package_catalogs` |
| Pure projection carries `null` remote and prunes identically | `aggregate_projection::aggregate_projects_supplied_facts_verbatim`, `build_aggregate_value_performs_no_observation` (unchanged, still zero counters) |
| Offline; one ref walk; zero extra structure, docs, walk, discovery, status, or spawn work; exact visit and diff attribution | `aggregate_view::tests::commit_families::collection_reuses_the_request_catalog_and_ref_snapshot_offline` (delta vs the identical request without the companion: `ref_walks` +1, `commit_visits` +3 history +3 containment, `file_diffs` +7, others 0). **Mutations:** linking through `link_commits` (fresh refs) fails with `ref_walks` 2; collection through `detect_repo_structure` fails with `manifest_parses` 6 vs 3. CLI `repo_aggregate_perf_covers_complete_command` (unchanged, `ref_walks == 1`) |
| Budget exhaustion yields `null`, not an invalid aggregate | `commit_links::…::a_shared_branch_snapshot_links_like_a_fresh_one_without_a_ref_walk` plus the pure-projection test above |
| Aggregate golden | `snapshots.rs::repo_aggregate_json_snapshot` (reviewed diff) |
| Legacy removal keeps corrupt-history failure on every selection | `git_parity.rs::recent_commits_collection_surfaces_corrupt_history_for_every_selection` |
| Behaviors ported from deleted legacy tests | `recent_commits.rs::selection::{a_skewed_old_head_does_not_hide_in_window_ancestors, an_empty_commit_is_a_hash_boundary_with_no_files}` |
| Moved `parse_commit_message` | `git::types::tests::parse_commit_message_tests::*` (5) |

### Gates

- `just test` (sniff): **2735 passed, 24 skipped**, exit 0. Phase 4 had 2797.
  - Removed: the legacy `recent_commits/mod.rs` unit tests (parse_period,
    rendering, filtering, attribution), about 30 legacy `integration.rs` tests,
    and 4 net from `git_parity.rs`.
  - Added: 7 new tests (1 `commit_links`, 1 `aggregate_view`, 2
    `recent_commits.rs`, 1 `git_parity.rs`, 2 CLI integration). Every one, and
    every rewritten test, appears as PASS in the run log (checked by name).
  - The 24 skips are the pre-existing gated tests.
- `just lint`: exit 0.
- `just doctest`: 96 passed, 22 ignored.
- `just sanity`: `sniff` and `sniff-cli` passed.
- Strict clippy, `-D warnings`, clean on each target that compiles:
  - `cargo clippy -p sniff --features remote --lib --test recent_commits --test git_parity --test integration`
  - `cargo clippy -p sniff-cli --features test-fixtures --lib --tests`
  - `cargo clippy -p sniff --features remote,network,bench-internals --bench perf`
- **Pre-existing, unchanged:** without `remote`, the `network,bench-internals`
  bench build warns that `credentials::provider_token` is dead code; that file
  is untouched.
- `just check-windows`: exit 0, compile evidence only. The only warnings are
  the known pre-existing ones (`windows_apps`, `executable_index`,
  `git_parity::stage_raw_path`).
- **Cross-OS runtime: not run** in this phase. The host matrix is Phase 6 work,
  and Phase 4 recorded `build-win-native` out of disk and `build-linux` locked.
  The new code is in-process gix plus JSON equality, so it adds no path
  separator, subprocess, or terminal surface. The CLI fixture writes `\n`
  content through git2 under the pinned fixture home, so `autocrlf` does not
  apply.
- `node .gitnexus/run.cjs detect-changes --scope all`: risk **critical**, 42
  files and 230 symbols. That covers all uncommitted Phase 1–5 work. The
  affected flows are name matches on the `recent_commits` module (e.g.
  `Render_tool_result_body`, `Block_value_context`). Phase 6's Graph review
  owns the untruncated pass.
- No `cargo fmt` was run.

### Cross-feature conflict (for the author)

`darkmatter/features/2026-09-09-more-context/{spec,plan}.md` is an active,
unimplemented feature. Its R27 and plan tasks 2.4, 5.4, and 10.1 build on
`get_recent_commits_by_count(repo_root, 10)` and `CommitDescSet::describe(true)`,
which this phase deleted as spec Decision 11 directs. Decision 11's caller
census counted code only, and no code in `darkmatter/` calls them, so nothing
breaks today. That feature's spec must be re-planned onto
`RecentCommits::collect` + `to_plain`. Another area's spec was not edited.

## Phase 6

Agent: claude/opus. Validation and review handoff; no feature code was planned for this phase.

### Work Group 6A — Local Gates

- **L1 (`just test` in `sniff/`)**: 2735 passed, 24 skipped, exit 0. The count
  equals Phase 5's. Feature tests were selected, not cfg'd out: 129
  recent-commit tests PASS by name, including 30 `sniff::recent_commits`, 14
  `commit_links`, 20 `recent_commits::render`, 22 `recent_commits::options`,
  43 `path_kind`, 16 `args::recent_commits`, both aggregate-family CLI tests,
  and `recent_commits_collection_surfaces_corrupt_history_for_every_selection`.
  `level2_recent_commits_rendering` is correctly absent: it is behind
  `test-fixtures`, which is L2-only.
- **Downstream L1** (both call the classifier directly):
  - `worktree` `just test`: 149 passed, 11 skipped, exit 0.
  - `darkmatter` `just test`: 7820 passed, 7 skipped, exit 0.
- **Manual exercise** used `target/debug/sniff`; no source was newer than the
  binary. Checked on this repository:
  - `recent-commits 3 --plain`, `-c --show-author`, and `1 -v`: headers,
    Today/Yesterday local labels, moved-file `(from …)` lines, and the
    description paragraph all render correctly.
  - `source-code-changes 1w --json`: a bare array of 89 commits. A local merge
    carries `remote:false`, `commit_url:null`, and monorepo `packages`.
    stderr is empty.
  - Empty queries (`2001-01-01`, both `--json` and `--plain`) exit 0. JSON
    prints `[]`; text puts `No commits matched.` on stderr.
  - `notaperiod`, `--package nope`, and `--branch no-such-branch` exit 1 with
    empty stdout and a typed message. `--action` is rejected by clap.
  - `sniff --perf repo --json`: exit 0; the families are bare arrays of
    10/6/9. stderr holds only the pre-existing human `--perf` tree, and the
    JSON also embeds perf.
- **Defect found and fixed:** the `SniffError::InvalidPeriod` message listed
  duration, date, hash, today, and yesterday, but not a count. A count is the
  default selection since Decision 2, so `sniff repo recent-commits notaperiod`
  gave misleading guidance.
  - Impact: GitNexus `impact InvalidPeriod` returned UNKNOWN (target not
    indexed). Text search showed every caller matches on the variant, and no
    test or doc quotes the message.
  - Regression test first: `cli.rs::test_commit_family_invalid_inputs_are_typed_failures`
    now also asserts that `recent-commits notaperiod` and `recent-commits 0`
    fail with empty stdout and stderr containing `positive count (e.g., 10)`.
    It failed before the fix and passes after.
  - Fix: `lib/src/error.rs` message now begins "Expected a positive count
    (e.g., 10), duration …".
- **Doc drift fixed:** `contract.md` key-set row said `bullets`; the code (and
  Phase 5 docs) serialize `bullet_points`. Resolved to the code.
- **Second defect found and fixed:** `aggregate_view.rs:883` (the Phase 5
  `pushed_workspace` fixture) had `let mut commit = |…|`. The closure mutates
  nothing, so rustc warns `unused_mut` on every OS; `just check-windows`
  surfaced it. `sniff/justfile`'s `lint` recipe runs clippy without
  `--all-targets` or `-D warnings`, so it never reached this `#[cfg(test)]`
  code. CI's shared `_lint` (`--all-targets -D warnings`) would have failed.
  Removed the `mut`; test-fixture local only, no callers.
- **Terminal suite:** `just test-l2` ran inside `just all`: 5 tests passed
  against a real tmux backend, not skipped (`level2_recent_commits_verbose_report_wraps_styles_and_links_in_tmux`
  took 0.97 s and uses `capture_until` with no sleeps). Plain output and clean
  JSON stdout are proved at L1 (`test_recent_commits_plain_output_is_the_library_plain_report`,
  `test_commit_family_empty_results_succeed_with_empty_output`), so no L2
  duplicate was added.
- **Full gate:** `just all` exit 0 (sanity 1491 + 455, lint, doctest 96
  passed / 22 ignored, test 2735, test-l2 5, test-browser n/a). The kache
  `Cross-device link (os error 18)` lines are cache-copy advice, not
  failures.
- **After both fixes:** `just lint` exit 0; `just test` 2735 passed, 24
  skipped; `just check-windows` exit 0.
- **CI-equivalent strict clippy** (`cargo clippy -p <crate> --all-targets -- -D warnings`):
  - `sniff`, `sniff-cli`, and `sniff-cli --features test-fixtures`: clean.
  - `sniff --features remote`: 3 `redundant_closure` errors, in
    `lib/tests/remote_observation.rs:522,737` and
    `lib/tests/focused_provider.rs:2528`. Both files are byte-identical to
    `main`, so the errors are pre-existing and out of scope. CI's `_lint`
    uses no features, so it does not hit them.

### Work Group 6B — Platform Evidence

- **Windows compile:** `just check-windows` exit 0, both before and after the
  fixes. After the fixes only the known pre-existing warnings remain:
  `windows_apps.rs:291`, `executable_index.rs:676,717`, and
  `git_parity.rs::stage_raw_path`. This is compile evidence only.
- **Host matrix:** `BUILD_LINUX=build-linux`, `BUILD_WIN=build-win-native`,
  `BUILD_WSL=build-win`. No host produced runtime evidence:
  - **native Windows:** FAIL before tests. `W:` had 8 KB free; the patch
    `scp` upload failed and `git fetch` reported `No space left on device`.
    - The daily `RustyBiscuit-CargoSweep` ran at 04:00 with result 0 and
      `free_gib=0`. It sweeps `W:/rusty-biscuit-target`, which does not
      exist.
    - The space is held by `W:\ci-verification\rusty-biscuit` (99.3 GB, its
      own `target\`), `W:\ci-verification\rb-pr66` (62.0 GB, last written
      2026-08-30, orphan), and `W:\WSL\Ubuntu-26.04\ext4.vhdx` (130.8 GB).
    - Under storage-strategy rule 2, an agent may not delete another
      session's artifacts, so nothing was removed. This was recorded in
      `.claude/skills/os/build-hosts.md`.
  - **WSL2:** FAIL before tests. `kex_exchange_identification: Connection
    reset` on every SSH attempt, including a later manual retry. Its VHDX
    lives on the full `W:`.
  - **Linux:** blocked on
    `~/ci-verification/.cross-check.lock`, owned by
    `nightly-reward-spike / reward-20260914-c3e60d0` since
    2026-09-14T18:25:30Z. No cargo, nextest, or rustc process is running on
    the host, so the lock looks stale. The os skill says only its owner
    removes it, so the waiter was stopped rather than left to time out.
- **Linux fallback (os skill `macos.md`: Docker Desktop):** a
  `rust:1-bookworm` container on `linux/aarch64` with rustc 1.97.1, run with
  `TZ=Pacific/Kiritimati` (UTC+14; the macOS host is UTC−7) against a copy of
  this worktree.
  - `cargo test -p sniff --features remote --test recent_commits`: 30
    passed.
  - `--lib -- recent_commits commit_links path_kind`: 120 passed.
  - `sniff-cli --test cli -- commit source_code documentation aggregate`: 38
    passed.
  - 0 failures. This covers local-calendar windows, linking and budget,
    rename/binary diffs, `non_utf8_author_and_message_bytes_are_replaced_not_rejected`,
    escaping and link targets, empty/typed-error CLI behavior, and aggregate
    equality.
  - The scratch copy (`~/.p6-sniff-linux`) was deleted afterwards.
- **Platform fixtures:** Linux (container) and macOS behave identically at
  the contract level. Native Windows and WSL2 runtime results are **CI-only**
  for this change. No failure was unique to one OS, so no OS-specific code
  fact was learned; the host-storage fact was recorded.

### Work Group 6C — Final Audit

- **Graph review:**
  - `just gitnexus` refreshed the stale index (`incremental-in-progress`).
  - MCP `detect_changes(scope: all)` is untruncated: `changed_count` 254
    equals the array length, `partial`/`truncated` are absent, and there are
    43 files. Risk is **medium**, with 3 flows (`Run → Build_report`,
    `Run → Emit_stderr`, `Observe_aggregate_evidence → Configure_cache`).
    Phase 5's "critical / name-collision flows" came from the stale index.
  - `compare` against `main`: 86 files, the same 3 flows, medium. The extra
    files are committed planning docs and merge-from-main files
    (`biscuit-terminal`, `codebook.toml`, `.claudine`).
  - The graph maps no symbols for new, untracked modules (`commit_links.rs`,
    `recent_commits/*`, the new args/output files) or for several modified
    test and output files. That zero means "unseen", so those files are
    covered by the compile, test, and text-search evidence below.
  - **HIGH/CRITICAL classifier effects:**
    - `is_source_code_path` and `classify_path` are CRITICAL, each with 7
      direct callers. All are in `sniff` (`blast_radius`, `aggregate_view`),
      `darkmatter` (`capture/changes.rs`, `capture/docs.rs`), or `worktree`
      (`worktree.rs`, `dirty_tree.rs`), and all three suites are green.
    - `is_documentation_path` is UNKNOWN. A text search found only the same
      darkmatter/worktree files.
    - `claudine/lib/src/permissions/matchers.rs::classify_path` is an
      unrelated same-named function.
  - **Unexpected-caller check for the removed public API:**
    - `cargo check --tests` on the 17 other direct reverse dependencies
      (biscuit-speaks{,-cli}, biscuit-terminal-cli, claudine{,-cli,-gen},
      darkmatter-cli, messenger{,-cli}, model-citizen, playa{,-cli},
      rendezvous-core/daemon, repo-deps, unchained-ai, zed-dmls-cli): exit 0.
    - `research` (sqlx) could not be compiled on this host, because
      `libsqlx_macros-*.dylib` is rebuilt with a "mis-aligned LINKEDIT
      string pool". That is a host toolchain fault and happens with the
      wrapper off too. A text search of `research/` for every removed symbol
      finds nothing. Its compile is CI-only.
- **Contract review:** an Explore sub-agent traced all 15 decisions, and I
  spot-checked the findings.
  - Every decision has implementing code and at least one asserting test;
    all names were verified by grep.
  - No legacy symbols remain. `discovery.rs::get_recent_commits_with_decorations`
    / `_fallible` are unrelated `pub(crate)` helpers behind `GitInfo.recent`.
  - No TODO, FIXME, or spike markers. No CLI-side business logic:
    `output/recent_commits.rs` only picks the output form.
  - The Phase 1 spike files `args/recent_commits_flag_shadowing.rs` and
    `output/recent_commits_prose_layout.rs` are `#[cfg(test)]` modules that
    now exercise the real `Cli`, `to_options`, and `to_prose`. They are not
    dead harnesses; only their names still reflect the spike origin.
  - No credentials, network fixtures, or generated artifacts were added.
  - **Recorded deviations (not gaps):**
    - (1) Decision 13: clap merges `-v` by argument id, so a subcommand `-v`
      also increments global `cli.verbose`, and `sniff -v repo recent-commits`
      yields a Verbose report. The spec's guarantee still holds because
      logging is driven by `--debug`
      (`recent_commits_flag_shadowing.rs` asserts `cli.debug == 0`). This is
      documented in `contract.md#flag-shadowing-clap`; the spec text was not
      amended.
    - (2) Decision 11's `parse_period` was replaced by `Selection::parse`
      (Phase 5).
    - (3) `contract.md` `bullets` was corrected to `bullet_points` in this
      phase.
  - **Pre-existing drift outside this feature, not changed:**
    `docs/cli/repo_hash.md:148` lists `Renamed`/`Copied` kinds that `repo
    hash` (rename tracking off) cannot emit.

### Requirement → test mapping (Phase 6 changes)

| Change | Test | Level | Before → after |
| --- | --- | --- | --- |
| `InvalidPeriod` guidance names the count form (original input `notaperiod`, boundary `0`) | `cli.rs::test_commit_family_invalid_inputs_are_typed_failures` (asserts non-zero exit, empty stdout, and stderr containing `positive count (e.g., 10)`) | L1 CLI, real binary | FAIL → PASS |
| `unused_mut` in aggregate fixture | `cargo clippy -p sniff --all-targets -- -D warnings` and `just check-windows` (warning absent) | compile | warn → clean |
| Contract key name | doc-only (`contract.md`) | — | — |

### Review handoff

- **Implemented contracts:**
  - Bare-array JSON per commit, with `heading`/`description`/`bullet_points`,
    `author`, UTC `datetime`, `files` (`min(0)`, `moved`), `file_types`,
    three-state `remote`, optional `commit_url`, and monorepo-only
    `packages`/`package_areas`.
  - Default last 10; empty is success; a free-form `--operation`; `--author`
    and `--show-author`; local-time calendar windows; `--branch` as base.
  - Structure-tier attribution; one exclusive `ChangeCategory` classifier;
    containment-based linking with a 5M-visit priority budget.
  - A library-authored Prose/Markdown/plain renderer with a CLI passthrough;
    sibling presets; aggregate families equal to the focused default arrays.
- **Intentional breaking changes:**
  - The JSON envelope (`period_label`, `repo_root`) is removed.
  - `--action` and `--no-error` are removed; an empty query exits 0.
  - The aggregate window moves from 3 days to the last 10.
  - `.html`/CSS become `web_assets` (this flows into Darkmatter and Worktree
    classifier callers).
  - `--plain` strips markdown bold.
  - The legacy `get_recent_commits_*`, `CommitDescSet`, `filter_by_*`,
    `parse_period`, and `PeriodSpecifier` API is deleted.
  - Package keys are omitted outside monorepos.
- **Work-counter and budget evidence:** Phase 5 recorded aggregate
  `commit_visits` going from 13,071 to 153,323 on this repository. That is
  containment on an unpushed branch, 3% of the 5M budget. `ref_walks`,
  `manifest_parses`, network, and spawns are unchanged. The mutation tests are
  in the Phase 5 log.
- **Test and OS evidence:**
  - macOS: full `just all` plus downstream darkmatter and worktree L1.
  - Linux: a targeted container run at UTC+14.
  - Windows: compile only.
  - Native Windows and WSL2 **runtime evidence is pending CI**; the build
    hosts are blocked, as recorded above.
  - The `research` compile is pending CI.
- **For the author:**
  - (a) `darkmatter/features/2026-09-09-more-context` still plans on the
    deleted API (Phase 5 handoff item).
  - (b) The build-host storage and lock state blocks all local cross-OS runs.
  - (c) `parse_org_repo` reports `22` as the org for `ssh://…:22/…`
    (pre-existing).
- The feature directory stays active. The implementation is complete and
  ready for review.
