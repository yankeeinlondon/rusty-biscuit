---
kind: plan
name: docs-audit-code-defects
spec: spec.md
total_phases: 4
created: 2026-10-11
phase: 3
agent: codex/default
yolo: true
source_files_during_phase_1:
  - tree-hugger/lib/tests/l1/symbols_cache.rs
docs_updated_during_phase_1: []
docs_created_during_phase_1: []
skills_files_updated_during_phase_1: []
source_files_during_phase_2:
  - tree-hugger/lib/src/file/tree_file.rs
  - tree-hugger/lib/src/file/mod.rs
  - tree-hugger/lib/src/queries/mod.rs
  - tree-hugger/lib/src/rule_registry.rs
  - tree-hugger/lib/queries/php/lint.scm
  - tree-hugger/lib/tests/l1/main.rs
  - tree-hugger/lib/tests/l1/lint_diagnostics.rs
  - tree-hugger/lib/tests/l1/lint_severity_registry.rs
  - tree-hugger/lib/tests/l1/rule_registry_consistency.rs
  - tree-hugger/lib/tests/l1/symbols_cache.rs
  - tree-hugger/lib/tests/fixtures/lint/undefined_symbol.py
  - tree-hugger/cli/Cargo.toml
  - tree-hugger/cli/tests/cli.rs
  - tree-hugger/cli/tests/lint_severity_agreement.rs
  - tree-hugger/cli/tests/symbols/cache_upgrade.rs
  - tree-hugger/cli/tests/fixtures/golden/symbols.json
  - tree-hugger/cli/tests/fixtures/golden/symbols.jsonl
docs_updated_during_phase_2:
  - tree-hugger/docs/library/diagnostics.md
  - tree-hugger/docs/cli.md
  - tree-hugger/docs/release-notes.md
docs_created_during_phase_2: []
skills_files_updated_during_phase_2:
  - .claude/skills/tree-hugger/diagnostics.md
  - .claude/skills/tree-hugger/query-system.md
source_files_during_phase_3:
  - tree-hugger/lib/src/file/tree_file.rs
  - tree-hugger/lib/src/lib.rs
  - tree-hugger/lib/src/queries/mod.rs
  - tree-hugger/lib/src/queries/drift.rs
  - tree-hugger/lib/tests/l1/phase6_neovim_query_reuse.rs
  - tree-hugger/lib/tests/l1/tree_file.rs
docs_updated_during_phase_3:
  - tree-hugger/docs/library.md
  - tree-hugger/docs/queries.md
  - tree-hugger/docs/release-notes.md
docs_created_during_phase_3: []
skills_files_updated_during_phase_3:
  - .claude/skills/tree-hugger/api-reference.md
  - .claude/skills/tree-hugger/query-system.md
source_files_during_phase_4:
  - tree-hugger/lib/src/queries/provenance.rs
  - tree-hugger/cli/src/main.rs
  - tree-hugger/cli/tests/command_feedback.rs
  - tree-hugger/cli/tests/changed/empty.rs
  - tree-hugger/cli/tests/symbols/filters.rs
docs_updated_during_phase_4:
  - tree-hugger/docs/cli.md
  - tree-hugger/docs/library/diagnostics.md
  - tree-hugger/docs/queries.md
  - tree-hugger/docs/release-notes.md
docs_created_during_phase_4: []
skills_files_updated_during_phase_4:
  - .claude/skills/tree-hugger/cli-architecture.md
  - .claude/skills/tree-hugger/diagnostics.md
  - .claude/skills/tree-hugger/query-system.md
packages:
  - tree-hugger
  - tree-hugger-cli
source_code:
  - tree-hugger/lib/tests/l1/symbols_cache.rs
  - tree-hugger/lib/src/file/tree_file.rs
  - tree-hugger/lib/src/file/mod.rs
  - tree-hugger/lib/src/queries/mod.rs
  - tree-hugger/lib/src/rule_registry.rs
  - tree-hugger/lib/queries/php/lint.scm
  - tree-hugger/lib/tests/l1/main.rs
  - tree-hugger/lib/tests/l1/lint_diagnostics.rs
  - tree-hugger/lib/tests/l1/lint_severity_registry.rs
  - tree-hugger/lib/tests/l1/rule_registry_consistency.rs
  - tree-hugger/lib/tests/fixtures/lint/undefined_symbol.py
  - tree-hugger/cli/Cargo.toml
  - tree-hugger/cli/tests/cli.rs
  - tree-hugger/cli/tests/lint_severity_agreement.rs
  - tree-hugger/cli/tests/symbols/cache_upgrade.rs
  - tree-hugger/cli/tests/fixtures/golden/symbols.json
  - tree-hugger/cli/tests/fixtures/golden/symbols.jsonl
  - tree-hugger/lib/src/lib.rs
  - tree-hugger/lib/src/queries/drift.rs
  - tree-hugger/lib/tests/l1/phase6_neovim_query_reuse.rs
  - tree-hugger/lib/tests/l1/tree_file.rs
  - tree-hugger/lib/src/queries/provenance.rs
  - tree-hugger/cli/src/main.rs
  - tree-hugger/cli/tests/command_feedback.rs
  - tree-hugger/cli/tests/changed/empty.rs
  - tree-hugger/cli/tests/symbols/filters.rs
documentation:
  - tree-hugger/docs/library/diagnostics.md
  - tree-hugger/docs/cli.md
  - tree-hugger/docs/release-notes.md
  - tree-hugger/docs/library.md
  - tree-hugger/docs/queries.md
  - .claude/skills/tree-hugger/diagnostics.md
  - .claude/skills/tree-hugger/query-system.md
  - .claude/skills/tree-hugger/api-reference.md
  - .claude/skills/tree-hugger/cli-architecture.md
completed_phase: 4
implemented: true
---

# Implementation plan: docs audit code defects

## Phase 1: Establish scope and implementation readiness

### Necessary Rules

- The finalized specification settles the design. Preserve its three ordered
  changes: Change 1 combines F1/F2; Change 2 combines F3/F5; Change 3 combines
  F4/F6/F7. Each implementation change includes its tests, documentation,
  skill updates, and one signed local commit. This planning task creates no
  implementation changes or commits.
- **API spelling correction:** the current registry exposes `has_rule`, not
  the specification's `is_known`. Use `RuleRegistry::builtin().has_rule(name)`
  for F7: it already recognizes aliases. Record this departure in the
  implementation log; leave the specification snapshot unchanged. No new
  registry method or author decision is needed.
- **Severity lookup:** use registry `get` with an explicit `Info` fallback.
  Do not use `get_or_default`, whose unknown-rule severity is `Warning`.
  Enablement must not alter a registered rule's default severity. Include
  syntax diagnostics in the registry agreement coverage, even though their
  current hard-coded `Error` already agrees.
- **Selector placement:** warn once where command selectors are assembled,
  outside `parse_selectors`. The current `AnalysisOptions::from_command`
  clones selectors; preserve once-per-run behavior through all consumers
  rather than relying on the specification's older call-path description.
- **No exploratory spikes:** the specification already resolves cache replay,
  PHP grammar categories, dependency scope, and API callers. The one PHP
  sample parse below is an implementation prerequisite, performed once before
  writing its query. Query compilation afterward is a correctness check,
  not another discovery spike. No performance measurements are required.
- **Input robustness matrix: not applicable.** These changes repair diagnostic
  policy, queries, provenance metadata, API exposure, and CLI messages. They
  do not change a file-format/configuration reader, deserializer, cache schema,
  or cache field interpretation. The revision-50 rejection regression uses
  existing cache readers and fixtures; do not expand it into a parser project.
  Reassess applicability only if implementation actually changes a reader.
- Keep the work bounded: no broader PHP shell-execution rule, new re-export
  support, upstream drift feature, library selector validation, malformed
  category-selector change, manifest licensing change, or new CI gate.
- Implementation is non-interactive. Do not wait for author feedback between
  changes, invoke credential prompts, push, create a PR, run `cargo fmt`, move
  the fix to `_completed`, or run `just complete`. If an acceptance gate or
  signing requirement cannot be met, report the blocker without weakening it.

### Work and successful completion

The work makes library diagnostics agree with the rule registry, removes
unimplemented rules and APIs, repairs PHP lint coverage and query licenses,
and makes command failures and unknown selectors readable and safe on stderr.
Success means all F1–F7 acceptance criteria pass, stale revision-50 caches
cannot replay the old severity, docs and skills describe the resulting
behavior, and the three implementation changes are ready for author review.

Only `tree-hugger` and `tree-hugger-cli` are implementation targets. Waves have
globally unique indices. Tasks within a wave are concurrent only when explicitly
marked; all other tasks run in listed order. Each concurrent task owns its code
and focused tests. One integration owner merges shared-file edits, updates
docs/release notes, runs gates, and creates commits. Any implementation subagent
must receive the non-interactive constraints and a bounded file ownership brief.

### Wave 1: Sequential readiness tasks

- [x] **Prove cache replay**
  - Make this the first implementation task, before editing production code.
    In `lib/tests/l1/symbols_cache.rs`, analyze an experimental undefined-symbol
    fixture with the unmodified revision-50 build, persist it, then prove a
    cache hit returns the stored `Error` through the public analysis result.
  - Capture the passing baseline command/result in `implementation-log.md`.
    In Phase 2, replace the temporary old-behavior expectation with final
    regression assertions; never commit an obsolete `Error` expectation.
  - Use existing cache APIs and test isolation. If hashing is needed, use
    `biscuit-hash` and load its skill rather than introducing a new hasher.
- [x] **Confirm implementation seams**
  - Confirm revision `50`, the six severity call sites, rule metadata/message
    tables, query capture access, removed API consumers, provenance enumeration,
    `main` error boundary, and command selector construction against the tree
    actually being implemented. Record the two corrections above.
  - Parse one PHP sample containing `exec('ls')`, `\\exec('ls')`, and
    `Foo\\exec('ls')` with the pinned grammar. Record nodes/prefixes and the
    exact bare-global discriminator before writing the qualified query.
    Retain the sample as regression input; do not repeat this discovery later.
  - Verify manifest license keys in root, library, and CLI manifests and record
    what exists; this check authorizes no manifest change.
  - Inspect existing test registration and scratch helpers. Library L1 uses
    `autotests = false`: register new modules in `lib/tests/l1/main.rs`.
    Use runtime `manifest_dir!`/`bin_exe!` and the area's isolated command
    builder so tests also work from CI archives.
  - Record initial worktree changes without overwriting unrelated work. Confirm
    the separate all-language re-export work remains queued behind this fix.

**Checkpoint:** old cache severity replay is demonstrated; the PHP query has a
confirmed grammar target; test seams and API spellings are recorded. No open
design ruling or performance spike blocks implementation.

## Phase 2: Deliver Change 1 — diagnostic and rule consistency

### Wave 2: Concurrent implementation tasks

- [x] **Unify rule policy**
  - Own `lib/src/file/tree_file.rs`, `lib/src/queries/mod.rs`,
    `lib/src/rule_registry.rs`, and `lib/src/file/mod.rs`. Replace all six
    `severity_for_rule` calls with registry severity lookup; remove that
    function and any exposure/imports. Preserve unknown-ID `Info` fallback.
  - Remove `unused-variable`, `shadowed-variable`, and `deprecated-syntax`
    registry entries and message arms; remove the unregistered
    `undefined-variable` arm if present. Preserve alias resolution for
    `unreachable-code`. Correct the uppercase-name skip comment to match
    actual single-character/underscore exclusions.
  - Add Python and its example to `eval-call`. Set `eval-call` and `exec-call`
    versions to `1.1.0`; set the exec title to “Dynamic code or command
    execution (exec)”, align its message, add the PHP example, and document
    the excluded PHP functions/operators in registry caveats.
  - Publish `SEMANTIC_RULE_IDS` beside semantic checks with exactly
    `dead-code`, `undefined-symbol`, `unused-symbol`, `unused-import`,
    `undefined-module`, and `invalid-syntax`. Bump extractor revision to `51`.
  - Add/register `lib/tests/l1/lint_severity_registry.rs` and
    `rule_registry_consistency.rs`. Iterate canonical registry IDs; check
    actual emitted diagnostic severities, default severity independent of
    enablement, query language sets, and query-or-semantic coverage.
    Provide emit-capable fixture cases so unexercised rules cannot silently
    pass. Check unknown-ID fallback and message-table IDs, resolving aliases.
    Inspect message arms as needed without adding public API only for a test.
- [x] **Repair PHP queries**
  - Own `lib/queries/php/lint.scm` and PHP regressions in
    `lib/tests/l1/lint_diagnostics.rs`. Depend on the Phase 1 sample parse.
    Use anchored case-insensitive `#match?` predicates for `exec` and `eval`.
    Match bare `exec` and the bare-leading-backslash global form only.
  - Assert `exec`, `EXEC`, `\\exec`, and mixed-case global spellings report
    `exec-call`; `Foo\\exec`, `namespace\\exec`, `shell_exec`, and other
    excluded shell APIs/operators do not report that rule. Include `eval`,
    `EVAL`, and mixed-case eval positives. Keep Python case sensitivity.
  - Run the existing query-compile check after the new expressions land.
    Each positive fixture must parse cleanly and assertions must inspect
    public diagnostics, not only query compilation or AST nodes.

These tasks can use separate subagents. The policy owner alone changes registry,
messages, revision, and L1 registration; the PHP owner changes only its query
and existing lint test module. Consistency tests may be red until both land.

### Wave 3: Sequential integration and delivery

- [x] **Verify cache agreement**
  - Rewrite the Phase 1 baseline for final `Warning` behavior. Add a revision
    `"50"` `StaleRevision` row in `cli/tests/symbols/cache_upgrade.rs`, using
    the existing computed fingerprint mechanism and test-owned temp directory.
    Preserve historical key formulas and avoid adding a nonexistent revision
    49 row. Assert revision-50 entries are misses and never served.
  - Add library coverage retaining old `Error` payload evidence if the CLI
    row cannot observe severity. Prove recomputation returns `Warning` and
    a subsequent current-cache hit equals uncached analysis.
  - Use one shared fixture under `lib/tests/fixtures/` for library/CLI severity
    agreement. Add `cli/tests/lint_severity_agreement.rs`; copy its source
    into an isolated scratch repo because `hug` excludes fixture directories.
    Assert the library and `hug lint --experimental-semantics` agree.
  - Declare the fixture in CLI `[package.metadata.ci.tests] source-inputs`
    and spell its runtime path through a CI-resolvable `manifest_dir!` join.
    Load `rust-devops` before editing this CI input metadata; follow
    `rust-testing` and `docs/cicd/test-inputs.md` without expanding CI scope.
  - Check `hug lint --list-rules` omits all three removed IDs. Prove consistency
    guards reject a query-only or registry-only addition, using a test-local
    perturbation or a temporary edit restored before delivery.
- [x] **Document diagnostic behavior**
  - Update `docs/library/diagnostics.md`: explain undefined-symbol plainly,
    remove the library-Error discrepancy and never-emitted-rule claims, and
    correct eval/exec languages, versions, PHP semantics, and namespace limits.
    Note that `--deny exec-call` now affects PHP.
  - Update skill `diagnostics.md` and `query-system.md` to remove severity-table
    instructions and correct rule tables. Review comments on changed symbols.
  - Add release-note entries for severity agreement, rule removals, PHP exec
    findings, and newly detected case-insensitive PHP eval calls. Defer the
    removed-name warning claim until Change 3 supplies that behavior.
- [x] **Validate first change** _(gates passed; the signed commit is made by the separate commit process, as this session's instructions forbid committing — see the implementation log)_
  - From `tree-hugger/`, run `just test` and `just lint` for both packages.
    Record commands/results and map F1/F2 acceptance items to test evidence.
    Repair failures and rerun affected checks; never use `cargo test` or
    `cargo fmt`. Keep the fixture input declaration and test registration
    in this same change.
  - Create only Change 1's signed local commit, using configured author identity
    and no agent trailers, then verify it with `git verify-commit HEAD`.
    Export `GIT_TERMINAL_PROMPT=0` for Git operations that could prompt.
    Check repository-local identity/signing configuration first. Signing must
    be unattended; if it hangs or needs credentials, abort and report rather
    than retrying or bypassing signing. Apply this procedure to later commits.

**Checkpoint:** F1/F2 pass together, registry/query/message consistency is guarded,
and stale caches cannot restore old severity. Change 1 is signed and verified.

## Phase 3: Deliver Change 2 — remove unused public APIs

### Wave 4: Concurrent removal tasks

- [x] **Remove placeholder method**
  - Own the `TreeFile::reexported_symbols()` removal and its obsolete docs/
    imports. Preserve `ImportSymbol::is_reexport` and existing Rust import
    extraction; do not introduce its replacement feature.
  - Add or retain an L1 public `imported_symbols()` regression containing
    `pub use a::B;` and `use c::D;`: only B has `is_reexport == true`.
  - Update `docs/library.md` and skill `api-reference.md` to explain the
    remaining Rust-only signal without promising all-language coverage.
- [x] **Remove drift module**
  - Own deletion of `lib/src/queries/drift.rs`, `pub mod drift`, and crate-root
    exports of `DriftItem`, `DriftReport`, `check_query_drift`,
    `check_all_drift`, and `summarize_drift`; remove
    `expected_upstream_revisions` with the module.
  - Remove only drift tests/imports/steps from
    `lib/tests/l1/phase6_neovim_query_reuse.rs`. Preserve provenance and
    compatibility checks; correct its header and workflow banner.
  - Remove drift API instructions from `docs/queries.md` and skill
    `query-system.md`, including “so drift checks skip it”. Retain concise
    manual refresh guidance using recorded upstream revision and notes.
    Preserve provenance revisions and the unrelated Claudine drift module.

The method owner edits TreeFile/library guidance; the drift owner edits module
wiring/query guidance. They can run concurrently after Change 1 is delivered.

### Wave 5: Sequential removal validation

- [x] **Verify removal coverage** _(gates passed; the signed commit is made by the separate commit process, as this session's instructions forbid committing — see the implementation log)_
  - Integrate both tasks and add one release-note line per removed API group.
    Search exact removed symbol names across `tree-hugger/lib/`,
    `tree-hugger/cli/`, `tree-hugger/docs/`, and the tree-hugger skill.
    Only removed-API release-note lines may remain. Do not search/ban generic
    “drift” or “re-export”; do not edit historical specs/cross-check files.
  - Prove provenance still covers every query with nonempty revisions and
    compatibility tests still run. Compile the workspace to catch removed API
    consumers, without scheduling workspace-wide tests. Run `just test` and
    `just lint` from the area; record results and F3/F5 acceptance coverage.
  - Commit only Change 2 with the Phase 2 signing procedure and verify it.

**Checkpoint:** removed APIs have no live callers/docs, Rust import flags and query
provenance remain correct, and Change 2 is signed and verified.

## Phase 4: Deliver Change 3 — provenance and CLI feedback

### Wave 6: Concurrent implementation tasks

- [x] **Correct query licenses**
  - Own `lib/src/queries/provenance.rs` and its tests. Use
    `REPOSITORY_LICENSE` for original overlays; retain Perl's original license.
    Iterate all provenance records: every `Original` is `AGPL-3.0-only`,
    every vendored non-original retains `Apache-2.0`, including modified queries.
    Do not hard-code a list of original languages/query kinds in the test.
  - Update license wording in `docs/queries.md` and any corresponding skill
    wording that disagrees. Record Phase 1 manifest findings without adding
    license keys or changing upstream revision strings.
- [x] **Repair CLI feedback**
  - Own `cli/src/main.rs` and focused CLI tests. Move the existing body into
    `run() -> Result<(), TreeHuggerError>`; make `main` print
    `error: <neutralized Display>` on any returned error and exit 1.
    Use existing `render::neutralized`; do not print duplicate source chains.
    Preserve Clap status 2, explicit command exits, and broken-pipe handling.
  - Add a table-driven L1 test for failing `symbols`, `refs`, `lint`,
    `imports`, and `god-files` invocations in isolated scratch repositories.
    Assert status 1, leading `error: `, expected Display text, and absence
    of a struct dump. Retain existing “does not exist”/“not a directory” tests
    unchanged. Add a Unix-only control-character filename test asserting
    escaped `\\u{…}` rather than raw characters in stderr.
  - In the one command selector assembly path, warn for unknown rule IDs in
    `--warn`, `--deny`, and `--allow` using `has_rule`, once per distinct name
    across all flags. Neutralize names; exempt aliases, `all`, and `category:`
    selectors. Keep `RuleSelector::parse` non-validating and statuses unchanged.
  - Test removed `unused-variable` and typo `unwrap-cal`, repeated names within
    and across flags, all three flags, a known rule, an alias, special selectors,
    and a control-bearing unknown name. Assert warning counts and expected
    statuses on clean input; legitimate lint findings still determine status.
  - Grep `cli/tests/symbols/export_controls.rs`, `export_ctags.rs`, and
    `export_ctags_paths.rs` for `--warn|--deny|--allow` and record the result
    before finishing F7. Preserve their empty-stderr expectations where valid.

F6 and F7 intentionally share one owner because both edit `main.rs`; splitting
them into independent simultaneous writers would create unnecessary conflicts.
The provenance task is independent and can run in a separate subagent.

### Wave 7: Sequential final integration

- [x] **Update CLI guidance**
  - Update `docs/cli.md` and `docs/library/diagnostics.md` for unknown-selector
    warnings, once-per-name behavior, aliases, and unchanged status semantics.
    Correct command-level error examples/exit documentation where applicable.
  - Add release notes for provenance correction, readable command errors, and
    removed rule names accepted with a warning. Review relevant CLI skill
    wording and changed-symbol comments; retain README overview scope.
    Current docs must not name or link this dated fix or another spec.
- [x] **Run final gates**
  - Run `just test`, `just lint`, and the existing `just test-l2` from
    `tree-hugger/`; use nextest and the shared focus-preserving terminal
    harness. No new styled view is introduced, so new L2 scenarios are not
    required for these plain stderr messages. Consult terminal/harness skills
    if implementation changes a styled component.
  - Preserve macOS, Linux, native Windows, and WSL2 compatibility. Record actual
    environment evidence separately from assumed portability. Load the `os`
    skill before path-comparison/Windows-cfg edits, an environment-specific
    failure, or a claim an OS cannot be tested. Do not introduce a speculative
    all-OS spike or new CI matrix for this fix.
  - Audit F1–F7 against the specification, including revision-50 rejection,
    shared-fixture CI declaration, consistency guards, removal searches,
    all-record licenses, five command failures, and warning deduplication.
    Record results/departures in the log, fixing current docs rather than
    rewriting the specification. Do not repeat gates already passed unless
    subsequent changes or failures justify it.
  - Create and verify only Change 3's signed local commit using the same
    unattended procedure. Keep unrelated work out of every commit.
- [x] **Prepare review handoff**
  - Check off only completed tasks with evidence. Report the three verified
    commits, validation results, observed environments, and any unmet criterion.
    If a criterion cannot be met, stop and report it without weakening the gate.
  - Finish with “implementation complete, ready for review” only after all
    required gates pass. Leave the fix active and the separate re-export
    feature untouched; no push, PR, lifecycle move, or `just complete`.

**Final checkpoint:** all seven defects are resolved in the specified three
changes, tests/lint and existing L2 checks pass, documentation and skills match
the code, and signed local history is ready for author review.
