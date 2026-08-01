# Claudine native Windows stabilization plan

Integrated execution plan for:

- [`2026-07-31-claudine-win/spec.md`](./spec.md), the
  umbrella Windows-stability specification;
- [`2026-07-29-windows-paths/spec.md`](../2026-07-29-windows-paths/spec.md), the
  blocking security and path-matching tranche; and
- biscuit-file's implemented
  [`to_portable_string` contract](../../../biscuit-file/features/2026-07-31-portable-strings/spec.md),
  with Darkmatter as the adoption precedent.

The implementation order is deliberately strict:

```text
baseline and ownership
    -> security-sensitive matching
    -> Windows atomic replacement
    -> path rendering and file URIs
    -> residual failure classification
    -> generator timeout work
    -> native Windows acceptance
```

Matching lands before rendering so portable output cannot accidentally make a
broken matcher appear healthy. Atomic replacement follows because it is the
other confirmed Windows product defect. Output rendering, unclassified
failures, and performance work come only after those correctness seams are
sound.

Every file and symbol below was checked against commit `d44143189` on
2026-08-01. Re-locate by symbol when a line has moved. GitNexus is indexed at
that commit, but its Rust symbol and process queries return no targets; the
blast-radius statements below therefore come from direct source and Cargo
dependency inspection and must be refreshed with both GitNexus and `rg` before
each implementation phase.

## Completion contract

This plan is complete only when all of the following are true:

- [x] Directory-scoped allow and deny rules work with `/` and `\` spellings on
   every host, without prefix collisions such as `C:\proj2` matching
   `C:\proj`.
- [x] Home-relative sensitive paths and Windows absolute allow entries work on
   native Windows; the temporary warning about broken Windows matching is gone.
- [x] Concurrent config writers retain atomic, intact, last-successful-rename-wins
   behavior on Windows without a non-atomic fallback.
- [x] User-facing paths and completion values follow biscuit-file's portable-text
   policy. Local hyperlinks are constructed as real file URIs rather than by
   concatenating `file://` with display text.
- [x] Every original Windows L1 failure is either fixed or proved to exercise a
   genuinely Unix-only contract. A failing Windows product path must never be
   hidden behind `#[cfg(unix)]`, `#[ignore]`, or a tier prefix.
- [x] The three generator drift tests either finish inside the ordinary timeout or
   have a measured, test-specific timeout policy justified by unavoidable
   bounded work.
- [x] Claudine's constrained build plus full L1 test, doctest, and lint gates
   pass on native `x86_64-pc-windows-msvc` Windows. The all-platform dependency
   graph retains `rendezvous-daemon -> duckdb -> libduckdb-sys`; focused native
   Windows live-daemon tests pass, and the Windows-GNU target graph contains the
   same chain.

Linux, macOS, xwin, and GNU-target checks are deferred portability follow-ups,
not completion authorities for this native-Windows stabilization plan. Their
unavailable or inconclusive results must be recorded without being represented
as passes.

## Verified starting state

| Area | Current evidence | Risk |
|------|------------------|------|
| Permission matching | `lib/src/permissions/matchers.rs::path_matches` compares, splits, trims, and globs only on `/`; its only production caller is `permissions/query.rs` | **Critical** for deny rules because a false negative fails open |
| Sensitive paths | `lib/src/protect/path.rs::is_prefix_match` hardcodes `/`; `SensitivePathChecker::is_sensitive` builds home prefixes with string formatting | **Critical** because credential paths are treated as non-sensitive |
| Allow paths | `protect/path.rs::is_path_allowed` recognizes only `/`-rooted absolute entries and splits only on `/` | High: Windows absolute allow rules do not work |
| Interim warning | `warn_windows_path_matching_is_broken` remains in `protect/path.rs` and is called from both matching paths | Must be deleted with the matching fix |
| Atomic writes | `config/atomic.rs::atomic_write` calls `NamedTempFile::persist` once; at least 24 production calls span config, MCP, dispatch, permissions, and inline composition | **High** blast radius; preserve the public `io::Result` contract and atomicity |
| Portable text | Claudine library/CLI/generator production code has zero `to_portable_string` or `try_portable_string` uses; all three crates already depend on biscuit-file | No dependency addition is needed |
| File URIs | System-prompt, completion, dry-run, agent-error, interrupt, and MCP surfaces hand-build `file://` strings from `Path::display` | Broken drive-letter form and missing URI encoding on Windows |
| Completion output | `completion/composition/compose.rs`, `setter_value.rs`, and `operation_file.rs` render relative paths through `to_str`/`display` | Direct cause of the measured backslash failures |
| Generator drift | `gen/tests/drift.rs` regenerates provider data per slug and calls `generate_all` independently in catalog and family tests | Profile before optimizing or widening timeouts |
| Existing dependencies | `claudine` and `claudine-cli` already depend on biscuit-file and `url`; `claudine-gen` also depends on biscuit-file | Manifest changes are not expected |

The source-derived impact scope is the full Claudine package area:
`claudine-catalog-types`, `claudine`, `claudine-contract`, `claudine-cli`, and
`claudine-gen`. `atomic_write` alone feeds provider config, MCP state/export,
dispatch persistence, permission mutation, and inline-compose closure writes.
No workspace-wide Cargo gate is justified; use the Claudine area recipes.

## Standing design boundaries

These are decisions, not implementation options:

- **Matching identity is not rendered text.** Do not feed security comparisons
  through `biscuit_file::to_portable_string`. That API may retain native UNC or
  verbatim spelling and is deliberately unsuitable as an identity key.
- **Normalize the permission-rule grammar once.** Inside matching, `/` and `\`
  both mean a segment separator, independent of the host running the test.
  Preserve current case sensitivity and lexical `.`/`..` behavior; widening
  either is a separate policy change.
- **Portable text is output only.** Use `to_portable_string` for diagnostics,
  visible labels, completion values, and YAML/JSON string projections where a
  native fallback remains meaningful.
- **Markdown/Prose destinations must handle decline.** Use
  `try_portable_string` where a native fallback would be parsed as escape syntax,
  and render plain text or return the owning typed error when it returns `None`.
- **File URIs are URLs.** Use `url::Url::from_file_path`; do not combine
  `file://` with `display`, `to_string_lossy`, or `to_portable_string`.
- **OS-facing values stay native.** Process arguments, environment values,
  filesystem operations, `PathBuf` map keys, and path-identity/session keys do
  not become portable display strings.
- **Tests are tiered by resource, not OS.** Pure Windows path, filesystem, and
  generator tests are L1. Use `#[cfg(windows)]` only when the implementation
  requires actual Windows semantics, not merely because the input looks like a
  Windows path.
- **No non-atomic write fallback.** Retrying an atomic replacement is allowed;
  copying bytes over the destination is not.
- **Comments move with behavior.** Remove the temporary warning documentation,
  update `atomic_write`'s concurrency/error contract, and update any item docs
  that currently describe native path rendering in the same commit as the code.

## Phase 0 — Ratify ownership and capture a reproducible baseline

This is a documentation and measurement phase. It makes the two active specs
complementary before any behavior changes.

### Spec alignment

1. In `2026-07-31-claudine-win/spec.md`:
   - make `2026-07-29-windows-paths` an explicit dependency, not merely a
     related document;
   - retain biscuit-file portable strings as the implemented supporting API;
   - state that July 31 owns all path-to-text and file-URI work;
   - replace the blanket allowance for `#[cfg(unix)]` with the narrower rule in
     this plan's completion contract.
2. In `2026-07-29-windows-paths/spec.md`:
   - retain permission matching, sensitive paths, absolute allow entries, the
     single-boundary guard, and warning removal;
   - move `render/prompt/system.rs`, mixed-separator output, and file-URI
     acceptance to the July 31 umbrella spec;
   - resolve the design decision in favor of a private comparison
     representation, explicitly excluding `to_portable_string` from matching.

Keep this alignment in its own documentation commit so later behavior commits
have unambiguous ownership.

### Native Windows baseline

Build all tests at constrained Cargo parallelism before invoking nextest. This
host can exhaust RAM/pagefile at the default `-j 12`, producing misleading
compiler/linker errors.

```powershell
cargo build --tests -j 4 `
  -p claudine-catalog-types -p claudine -p claudine-contract `
  -p claudine-cli -p claudine-gen
just test --profile ci --no-fail-fast
```

Capture the complete output rather than piping through `tail`. Record, by exact
test name:

- pass/fail/skip/timeout totals per test binary;
- runtime for every test over five seconds;
- `raw_os_error()` for the atomic-write failure;
- every failure's first product frame, not merely its assertion text; and
- the current results of the two targeted security suites.

Run the baseline with the CI profile so deterministic failures are not retried
into four copies. The later final gate uses both this no-retry diagnostic run
and the canonical local recipe.

### Checkpoint 0

- The two specs have disjoint ownership.
- The baseline is attached to the implementation notes or summarized in this
  plan; no failure is represented only by an inferred class.
- The known security, atomic-write, completion, and generator failures reproduce
  on native Windows.

## Phase 1 — Fix path matching and sensitive-path enforcement

This phase implements the July 29 security tranche and lands before any
rendering change.

### One comparison boundary

Add one private library module, `lib/src/path_semantics.rs`, owning these
operations:

- normalize both separator spellings into the permission-rule grammar's `/`;
- recognize POSIX-rooted, Windows drive-rooted, and UNC absolute spellings on
  every host while rejecting drive-relative `C:foo` as absolute;
- test exact-or-descendant relationships with a segment boundary; and
- expose normalized segments to the existing single- and double-star glob
  implementation.

The module operates on comparison strings only and never emits user-facing
text. Normalize each input once at the public matching boundary. Do not add a
general path abstraction or move unrelated filesystem resolution into it.

### Adopt the boundary

1. `lib/src/permissions/matchers.rs`
   - normalize `path` and the trimmed pattern once;
   - make exact, `/*`, trailing-separator, implicit-prefix, `*`, and `**`
     branches consume the normalized representation;
   - keep the current wildcard semantics (`*` stays inside one segment; `**`
     crosses segments);
   - remove the warning call and the stale Windows warning docs.
2. `lib/src/protect/path.rs`
   - replace local `is_prefix_match` with the shared exact-or-descendant helper;
   - construct home-relative prefixes with `PathBuf::join`, then normalize only
     for comparison;
   - make `is_path_allowed` use the shared absolute-spelling detector and
     normalized component/boundary logic for both target and allow entry;
   - retain `canonicalize_existing_ancestor` for native paths and preserve the
     no-non-atomic-fallback behavior elsewhere;
   - delete both `warn_windows_path_matching_is_broken` definitions and all
     docs referring to the interim warning.
3. `lib/src/permissions/query.rs` and `protect/service.rs`
   - keep their policy decisions unchanged; add end-to-end assertions proving
     the corrected boolean reaches deny/allow behavior rather than testing only
     the helper.

### L1 tests

All separator-grammar tests run on every host:

- Windows path + Windows pattern, Windows path + POSIX pattern, and the inverse;
- exact, explicit `/*`, trailing separator, implicit directory prefix, `*`, and
  `**` cases;
- `C:\proj2` does not match `C:\proj`;
- drive-relative `C:proj\file` is not treated as an absolute allow entry;
- a directory deny rule blocks a child path, separately from an allow-rule test;
- a directory allow rule grants a child path;
- a fixture checker with home `C:\Users\user` classifies `.ssh`, `.aws`,
  `.gnupg`, and `.claude` children as sensitive;
- an absolute `C:\...` allow entry is honored; and
- existing POSIX behavior and glob cases remain unchanged.

Add a source-inventory guard covering `permissions/matchers.rs` and
`protect/path.rs`. It fails if those modules reintroduce raw separator-boundary
constructs such as byte equality against `b'/'`, path splitting on only `/`, or
the deleted warning symbol. The helper module is the only permitted definition
of descendant-boundary semantics.

### Documentation and gates

Update both `claudine/docs/topics/protect-service.md` and its skill mirror
`.claude/skills/claudine/protect-service.md` to state that rule separators are
portable and boundary-aware. Review every changed `///`, `//!`, and inline
comment for the old broken-Windows claim.

```powershell
just test-library --profile ci --no-fail-fast
just test-cli --profile ci --no-fail-fast
just lint
```

Checkpoint 1 is green only when the warning is absent, the source guard passes,
and deny/sensitive-path cases are proved through their policy callers.

## Phase 2 — Make atomic replacement reliable on Windows

Treat `config/atomic.rs::atomic_write` as a high-impact shared primitive. Do not
change its signature or any caller unless profiling proves a caller-specific
problem.

### Reproduce and classify

1. Strengthen `concurrent_writers_produce_intact_payload` with a `Barrier` so
   writers contend at the persist seam rather than starting opportunistically.
2. Run it repeatedly on native Windows with retries disabled and record the
   persist error's raw Windows code.
3. Confirm the error originates at `NamedTempFile::persist`, not temp-file
   creation, writing, `sync_all`, or final reading.

### Implementation

On Windows only, retry `NamedTempFile::persist` when the persist error is one of
the measured transient replacement/share conditions. Preserve the
`PersistError.file` value between attempts so the same fully written and
synced temporary file is retried; never rewrite or copy the payload.

Use a small bounded exponential backoff with a fixed maximum attempt count.
Classify Windows errors through named `windows` constants already available to
the library, not unexplained numeric literals. After exhaustion, return the
originally meaningful `io::Error`. Non-Windows keeps the single persist call.

Update `atomic_write`'s contract to say:

- unique temporary files prevent torn interleaving;
- Windows may retry transient atomic-replacement failures;
- success still means the final bytes equal one complete writer payload; and
- permanent errors remain errors, with no byte-copy fallback.

### Tests

- concurrent writers start behind a barrier and always leave one intact input
  payload;
- repeated contention leaves no temporary siblings;
- transient-error classification is table-tested on Windows;
- a non-transient persist error is not retried; and
- existing create-parent, overwrite, sync, and cleanup tests remain green.

Keep the stress loop bounded below five seconds so it remains ordinary L1. If
it cannot be both reliable and fast, separate a small deterministic unit test
for retry state from a `slow_`-prefixed stress test.

### Gates

```powershell
just test-library --profile ci --no-fail-fast
just test-cli --profile ci --no-fail-fast
just lint
```

Checkpoint 2 requires repeated native-Windows passes with no permission/share
failure, no torn content, and no stray temp file. Because this primitive has
many callers, run the full library and CLI suites rather than only its unit
test.

## Phase 3 — Adopt portable text and valid file URIs

Start by generating a fresh inventory of production `.display()`,
`to_string_lossy()`, and hand-built `file://` uses under `claudine/lib/src`,
`claudine/cli/src`, and `claudine/gen/src`. Classify every hit before editing:

| Classification | Required treatment |
|----------------|--------------------|
| Visible path, completion value, diagnostic, serialized display value | `biscuit_file::to_portable_string(&path)` |
| Markdown/Prose destination without a URL layer | `try_portable_string`; plain-text or typed-error branch on `None` |
| Local hyperlink/OSC8 file destination | `url::Url::from_file_path` on an absolute path; plain text if conversion fails |
| Process argument, environment value, filesystem input | Preserve `Path`/`OsString` or native spelling |
| Comparison, map key, session identity/hash input | Preserve typed/native identity; never use portable output |
| Tracing-only structured field | Leave native unless the field has a documented portable external contract |

Do not mechanically replace every `.display()` call. Commit the inventory table
to this plan or the implementation notes so skipped sites have an explicit
reason.

### Phase 3A — Library rendering

Adopt the shared renderer first in the library:

- `render/prompt/system.rs::resolve_display_label` — portable relative and
  absolute labels;
- `render/prompt/system.rs::render_system_prompt_summary` — URL-aware file href;
- `stream/path_link.rs::format_file_link` — portable repo/home-relative visible
  text and a real file URI for an absolute hyperlink target; and
- any other library user-facing path found by the inventory.

Preserve the current anchoring and truncation decisions. Portability changes
spelling, not whether cwd, home, or absolute display wins. Add pure L1 tests for
drive paths, spaces, `#`, `%`, and successful/declined portable spellings.

### Phase 3B — CLI completion and reporting

Convert completion output at the path-valued seams, before strings are quoted
or combined:

- `completion/composition/compose.rs::format_relative_insert`;
- `completion/setter_value.rs::format_relative`;
- `completion/operation_file.rs::format_relative_insert`;
- `completion/autocomplete_ui.rs::path_label` and its hyperlink target; and
- other completion candidates identified in the baseline failures.

Then convert user-facing report and link sites, including the known seams in:

- `commands/wrap/composition/dry_run.rs`;
- `commands/wrap/composition/target.rs`;
- `commands/compose/interrupt.rs`;
- `commands/mcp/init.rs`;
- `commands/link_display.rs`; and
- system-prompt and timeout summaries.

Do not convert provider CLI arguments, temporary-file arguments, launch plans,
environment overlays, MCP state keys, or session compatibility keys. These are
native/identity consumers even though they eventually contain `String`.

### Rendering tests

- keep Windows-shaped portable-text tests host-independent where no filesystem
  semantics are required;
- run actual `Url::from_file_path` drive/UNC cases under `#[cfg(windows)]`;
- assert completion output directly, with no test-only slash normalization;
- assert `file:///C:/...` shape and percent encoding for spaces/reserved URL
  characters rather than only checking the `file://` prefix;
- assert a declined path never becomes a malformed slash-replaced namespace;
- retain POSIX and macOS snapshots byte-for-byte except where a previously
  malformed URL is intentionally corrected; and
- keep all of these L1 because they require no real terminal.

### Gates

```powershell
just test-library --profile ci --no-fail-fast
just test-cli --profile ci --no-fail-fast
just lint
```

Checkpoint 3 requires all known Class A failures to pass, zero hand-built
`file://{path.display()}`-style production sites, and a reviewed classification
for every remaining native path-to-string conversion.

## Phase 4 — Reclassify and close the remaining Windows failures

Run the complete native-Windows L1 suite again with the same command and
no-retry profile used in Phase 0. Diff exact test names against the baseline.
Do not assume the original class sizes remain accurate after the first three
fixes.

For each remaining failure, record one of:

1. **Product path defect** — fix the owning path boundary and add an end-to-end
   regression.
2. **Windows filesystem/process defect** — reproduce at the smallest product
   seam, then fix it without weakening Unix behavior.
3. **Portable test-fixture defect** — replace assumptions such as `pwd`/`echo`
   executables, POSIX root shapes, verbatim-prefix expectations, or `HOME`
   behavior with platform-correct fixtures.
4. **Genuinely Unix-only contract** — add `#[cfg(unix)]` with a reason naming the
   Unix facility. This is the only class allowed to be gated.
5. **Timing/performance** — move to Phase 5; do not disguise it as correctness.

Revisit the previously sampled unknowns only after path fixes land:

- `mcp_cli::effective_defaults_repo_replaces_user`;
- the nine `skills_integration` empty-result failures;
- provider configuration and discovery failures; and
- any completion binary still returning native separators.

If they disappear, record them as downstream effects rather than adding
unneeded code. If they remain, trace the first missing path/discovery decision
and fix that owning seam. Do not broaden this phase into adjacent refactors.

Checkpoint 4 is a zero-failure native-Windows L1 correctness run excluding only
the separately recorded generator timeout tests. Every `cfg` added in this
phase must be audited against the OS-specific-test rules.

## Phase 5 — Profile and bound generator drift tests

The three affected tests are:

- `committed_data_matches_regenerated_inputs`;
- `committed_catalog_matches_regenerated_inputs`; and
- `committed_families_match_regenerated_inputs`.

### Measurement first

Build `claudine-gen` tests with `-j 4`, then run each test alone at least three
times with nextest retries disabled and full output captured. Add temporary
phase timers around input discovery, parsing, per-provider generation,
serialization, and comparison. Remove temporary instrumentation before the
commit unless it is useful low-noise tracing.

Compare the tests with the passing vocabulary/signals drift tests. Specifically
check whether:

- `check_area` reloads shared inputs for every provider slug;
- `generate_all` repeats repository or Darkmatter composition discovery;
- catalog and families checks regenerate an identical intermediate; or
- the test is simply bounded CPU work made slower by full-suite contention.

### Decision gate

1. If repeated discovery/parsing dominates, cache or batch it inside the
   generator API used by the tests. Preserve one production generation path;
   do not create a test-only fast path.
2. If the same generated intermediate is recomputed within one test, compute it
   once and pass it to the existing check functions.
3. If isolated runtime is under 30 seconds but suite contention crosses the
   ceiling, add exact-name `slow-timeout` overrides to both default and CI
   profiles. Keep CI retries at zero and set the deadline above measured p99,
   not an arbitrary large value.
4. If unavoidable isolated runtime remains over five seconds, prefix the tests
   with `slow_` so `just sanity` excludes them while `just test` still runs
   them. Update exact-name filters accordingly.

A timeout override is not accepted without the timing breakdown and an
explanation of why further simplification would change generator correctness.

### Gates

```powershell
just test-gen --profile ci --no-fail-fast
just sanity
just lint
```

Checkpoint 5 requires all generator tests to pass on the first attempt and no
test to reach its termination ceiling in a full Claudine run.

## Phase 6 — Full acceptance and lifecycle closeout

### Native Windows

```powershell
cargo build --tests -j 4 `
  -p claudine-catalog-types -p claudine -p claudine-contract `
  -p claudine-cli -p claudine-gen
just sanity
just lint
just doctest
just test --profile ci --no-fail-fast
just test
```

The CI-profile run supplies deterministic no-retry evidence; the canonical
`just test` proves the local package-area contract. Capture complete output for
both. Do not run L2/L3/browser tiers for these changes unless implementation
introduces a real-terminal requirement; the specified behaviors are L1.

### Deferred portability follow-ups

- When exact-candidate runners are available, run the same Claudine `build`,
  `lint`, `doctest`, and `test` area gates on Linux and macOS CI.
- From a supported non-Windows host with xwin installed, run
  `cargo xwin check --target x86_64-pc-windows-msvc -p claudine --all-targets`
  as a supplemental cross-check.
- Retain `just check-windows` as the repository's supplemental GNU-target
  compile check.
- Do not substitute a workspace-wide Cargo build or test.

These checks do not replace or override the native Windows MSVC evidence. An
unavailable tool or runner is recorded as deferred, not inferred green. The
WSL mounted-tree and native-snapshot attempts are documented in the Phase 6
acceptance note and are not accepted as Linux behavior evidence because the
environment failed at the storage/subsystem boundary.

Dependency selection is not deferred: DuckDB and `rendezvous-daemon` are
required on every target. `cargo tree --target x86_64-pc-windows-gnu` must keep
the `rendezvous-daemon -> duckdb -> libduckdb-sys` chain even when the GNU
compiler is unavailable. Graph inclusion is evidence of dependency selection,
not a claim that GNU compilation passed.

### Final audits

1. Search for the deleted warning and raw separator-boundary copies.
2. Re-run the path-to-text inventory and explain every remaining native
   conversion.
3. Confirm no path identity, process argument, or environment value was routed
   through portable rendering.
4. Review docs/comments for every behavior-changing symbol. Update the Claudine
   README, topic docs, timeline, and skill mirrors only where public behavior
   changed; no dependency document changes are needed unless the implementation
   unexpectedly changes manifests.
5. Run `git diff --check` and inspect `git diff` for unrelated formatting or the
   user's pre-existing `CLAUDE.md` change.
6. Run GitNexus `detect_changes` and upstream impact again. If Rust symbols are
   still unavailable, record that limitation and repeat the direct `rg` caller
   inventory rather than reporting a false LOW risk.
7. Walk every acceptance item in both source specs and this plan, attaching a
   test or gate result to each.

After the native Windows evidence is attached, mark both fixes complete
together. The July 31 fix remains the umbrella record and the July 29 fix
records the completed first tranche. Move this integrated plan with the
umbrella record during lifecycle completion so it does not remain as an orphan
under `fixes/`. Deferred portability checks remain recorded follow-up work.

## Commit sequence

Keep the work reviewable and sequentially landable:

1. `docs(claudine): align native Windows fix ownership`
2. `fix(claudine): make security path matching separator-neutral`
3. `fix(claudine): retry transient Windows atomic replacement`
4. `fix(claudine): render library paths and file URIs portably`
5. `fix(claudine): render CLI completions and reports portably`
6. One narrowly named fix per independently confirmed residual failure class
7. `perf(claudine-gen): keep drift checks within their measured budget`, or a
   test-policy commit if profiling proves the work inherently bounded and slow
8. `docs(claudine): record Windows acceptance evidence`

Do not combine speculative residual fixes into the security or atomic-write
commits. Do not put behavior changes in the final documentation-only commit.

## Risk register

| Risk | Severity | Mitigation |
|------|----------|------------|
| Portable rendering masks broken security matching | Critical | Phase 1 is a hard dependency of Phase 3; policy-level deny tests must pass first |
| Rendered text is reused as path identity | High | Inventory by consumer role; retain native `Path`/`OsString` for identity and OS boundaries |
| Hand-built file URLs remain malformed | High | URL-aware conversion plus drive, UNC, encoding, and decline tests |
| Atomic retry hides permanent permission errors | High | Retry only measured transient persist codes, bound attempts, return permanent/exhausted errors |
| Atomic fix weakens crash safety | High | Reuse the written/synced temp file; prohibit copy/truncate fallback |
| Broad `.display()` replacement changes provider arguments | High | Separate library and CLI adoption commits; review every conversion classification |
| Windows failures are silenced by gating | High | Permit `cfg` only for a named Unix facility; OS-specific pure tests remain L1 |
| Generator timeout is widened without understanding cost | Medium | Isolated repeated profiling and p99-based exact-name policy |
| Full suite exhausts Windows memory | Medium | Build tests at `-j 4` before nextest; capture complete output |
| A deferred Linux/macOS run exposes behavior drift | Medium | Host-independent portable-shaped unit cases run now; investigate on an exact-candidate runner when available |
| GitNexus reports false safety because Rust symbols are absent | Medium | Treat UNKNOWN as unknown; use direct caller inventory and package dependencies |

## Stop conditions

Pause the current phase rather than guessing if any of these occurs:

- matching requires case-folding or lexical normalization beyond separator
  equivalence;
- a Windows path namespace cannot be represented safely in the intended output
  grammar;
- atomic replacement fails with a code outside the measured transient set;
- a proposed fix requires a new dependency or changes a public error/signature;
- a remaining failure cannot be reproduced independently of suite contention;
  or
- a later Linux/macOS follow-up shows output changes outside the explicitly
  approved portable spelling or file-URI correction.

Resolve the contract in the owning spec, then resume from the last green
checkpoint.
