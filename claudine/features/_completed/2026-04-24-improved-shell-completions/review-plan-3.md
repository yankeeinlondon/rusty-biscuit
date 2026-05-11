---
phases: 5
created: 2026-04-25
source_feature: 2026-04-24-improved-shell-completions
review_source: ./review-3.md
package_area: claudine
packages:
  - claudine
  - claudine-cli
findings_addressed:
  - finding-1: Topic doc claim about sequence external reference resolution is wrong
  - finding-2: Test gap — nested setter-value file paths
  - finding-3: Test gap — plain git checkout with setter-value completion
  - finding-4: Magic mode does not surface directories at short prefix lengths (decision + doc)
  - finding-5: Test boilerplate duplicated across 5+ test files
  - finding-6: Bash/zsh/fish scripts swallow engine panics (CLAUDINE_COMPLETION_DEBUG=1)
  - finding-7: Topic doc lists only 2 of 7 agent-skill peer directories
  - finding-8: '#![allow(dead_code)] blanket on production modules'
---

# Review-Plan 3: Improved Shell Completions

This plan closes the eight follow-up findings raised in
[`review-3.md`](./review-3.md). All four blocking findings from `review-2.md`
are already addressed by `review-plan-2.md`; the issues here are
documentation drift, test-coverage extensions, one design-vs-spec
ambiguity (`@` magic + directory candidates), code-hygiene cleanups, and
one operational-visibility improvement (debug switch for the shell
bootstrap scripts).

Findings 1, 7 are docs only. Findings 2, 3 are pure new tests. Finding 4
is a small spec/UX decision plus tests + docs. Finding 5 is a test-helper
extraction. Finding 6 adds a debug env-var to the bootstrap scripts.
Finding 8 is a code-hygiene sweep that will likely surface unused items
(notably `SetterToken::raw_value`).

`review-plan-1.md` and `review-plan-2.md` already cover: setter-value
extension/size gating, `@` path-shaped magic forms, magic priority
ordering, repo-wide directory walk, plain-git scope rendering for
composition, fish file fallback. **Do not duplicate any of those
behaviors here.** This plan only adds what `review-3.md` raises.

## Validation baseline (run once before Phase 1)

```bash
cd /Users/ken/.claudine/worktrees/rusty-biscuit/claudine
cargo test -p claudine
cargo test -p claudine-cli
cargo clippy -p claudine -p claudine-cli --all-targets -- -D warnings
cd claudine && just test && just lint
```

Establish baseline green before editing. If any of those are red **before**
this plan starts, triage them as pre-existing — do not bundle their fixes
into this plan's commits.

## Dependency map (findings ↔ phases)

| Finding | Module(s) touched                                                  | Phase |
|--------:|---------------------------------------------------------------------|------:|
| 5       | `cli/tests/common/mod.rs` (+ all 6 `completion_*.rs` test files)    | 1     |
| 2       | `cli/tests/completion_setter.rs`                                    | 2     |
| 3       | `cli/tests/completion_setter.rs`                                    | 2     |
| 4       | `cli/src/completion/composition.rs`, integration tests, topic doc   | 3     |
| 6       | `cli/src/completion/bootstrap.rs`, bootstrap unit tests             | 4     |
| 8       | every `cli/src/completion/*.rs` carrying `#![allow(dead_code)]`     | 5     |
| 1       | `claudine/docs/topics/shell-completions.md`                         | 5     |
| 7       | `claudine/docs/topics/shell-completions.md`                         | 5     |

Ordering rationale:

1. **Phase 1 first** — extracting the test helpers reduces churn risk for
   every later test addition. Phase 2's new tests will land directly in
   the deduped helper world.
2. **Phase 2** — pure new integration tests (no source changes, low risk).
3. **Phase 3** — the single source change with semantic impact (magic
   directories at Short prefix length). Lands after Phase 2 so the
   helper-extraction churn does not flap with semantic changes.
4. **Phase 4** — operational-visibility flag in the bootstrap scripts.
   Independent of phases 1–3 but landed mid-plan to keep clippy gate
   simple in Phase 5.
5. **Phase 5** — `#![allow(dead_code)]` removal sweep + docs refresh.
   Sweeping last guarantees no test-helper or new-code addition reawakens
   a "now unused" item that has to be re-deleted. Docs-only items 1 & 7
   ride along here for a single docs commit.

Phases 1, 2, 4 are independent and could parallelize across executors;
Phases 3 and 5 must run in their listed order.

---

## Phase 1 — Test helper extraction (finding 5)

**Goal:** Lift the duplicated test boilerplate into `cli/tests/common/`
so the six completion integration test files stop drifting.

**Scope:**

- `claudine/cli/tests/common/mod.rs` — extend module
- New file: `claudine/cli/tests/common/completion.rs`
- `claudine/cli/tests/completion_compose.rs`
- `claudine/cli/tests/completion_inline_compose.rs`
- `claudine/cli/tests/completion_sequence.rs`
- `claudine/cli/tests/completion_setter.rs`
- `claudine/cli/tests/completion_perf.rs`
- `claudine/cli/tests/completion_cli.rs`

**Code changes:**

1. **Create `cli/tests/common/completion.rs`** as a sibling submodule.
   Move these helpers verbatim from any one of the integration files
   (pick `completion_compose.rs` as the canonical source — it has the
   fullest set):

   - `seed_cargo_workspace(root: &Path)`
   - `seed_cargo_workspace_members(root: &Path, members: &[&str])`
   - `write_file(path: &Path, contents: &str)`
   - `fake_home(ws: &Path) -> PathBuf`
   - `run_complete(ws: &Path, cwd: &Path, current: usize, argv: &[&str]) -> Vec<String>`
   - `run_complete_with_home(ws: &Path, home: &Path, cwd: &Path, current: usize, argv: &[&str]) -> Vec<String>`

   Mark each helper `pub(crate)` and add a short `///` rustdoc summary
   per helper. No behavior change.

2. **Re-export from `cli/tests/common/mod.rs`.** Add
   `pub mod completion;` (or `pub(crate) mod completion;`). Keep the
   existing `TestWorkspace` / `init_git_repo` exports untouched.

3. **Delete the local duplicates** from each of:

   - `completion_compose.rs`
   - `completion_inline_compose.rs`
   - `completion_sequence.rs`
   - `completion_setter.rs`
   - `completion_perf.rs`
   - `completion_cli.rs`

   Replace with `use crate::common::completion::{seed_cargo_workspace,
   write_file, fake_home, run_complete, run_complete_with_home, ...};`
   (only the names actually used per file).

4. **Verify no signature drift.** The helpers must be byte-equivalent
   across files at the moment of lifting. If any file has a slightly
   different signature (e.g. an extra `env_vars` parameter), pick the
   superset, parameterize the helper, and update every call site so the
   move is non-breaking.

5. **Do not introduce new helpers in this phase.** Scope creep risk —
   helper extraction only. New helpers may be added in Phase 2 or 3
   when actually needed.

**New tests:** None. Phase 1 is a refactor; existing tests remain the
contract.

**Verification:**

```bash
cd /Users/ken/.claudine/worktrees/rusty-biscuit/claudine
cargo test -p claudine-cli --test completion_compose
cargo test -p claudine-cli --test completion_inline_compose
cargo test -p claudine-cli --test completion_sequence
cargo test -p claudine-cli --test completion_setter
cargo test -p claudine-cli --test completion_perf -- --ignored
cargo test -p claudine-cli --test completion_cli
cargo test -p claudine-cli
cargo test -p claudine
cargo clippy -p claudine -p claudine-cli --all-targets -- -D warnings
```

**Success criterion:** every previously-passing test still passes; no new
clippy warnings; `cli/tests/common/completion.rs` exists; the six
integration files no longer carry local copies of the lifted helpers.

### Phase 1 tracking status

| Item                                                                | Status |
|---------------------------------------------------------------------|:------:|
| `common/completion.rs` created with all six helpers                 |   ☑   |
| `common/mod.rs` exposes the new submodule                           |   ☑   |
| `completion_compose.rs` uses the shared helpers                     |   ☑   |
| `completion_inline_compose.rs` uses the shared helpers              |   ☑   |
| `completion_sequence.rs` uses the shared helpers                    |   ☑   |
| `completion_setter.rs` uses the shared helpers                      |   ☑   |
| `completion_perf.rs` uses the shared helpers                        |   ☑   |
| `completion_cli.rs` uses the shared helpers                         |   ☑   |
| `cargo test -p claudine-cli` green                                  |   ☑   |
| `cargo clippy -p claudine -p claudine-cli --all-targets -- -D warnings` clean |   ☑   |

**Phase 1 notes:**
- The `common/completion.rs` module exposes all six shared helpers
  (`seed_cargo_workspace`, `seed_cargo_workspace_members`,
  `seed_plain_git_repo`, `write_file`, `fake_home`, `run_complete`,
  `run_complete_with_home`) lifted from `completion_compose.rs`.
- `completion_setter.rs` previously had a 2-arg `seed_cargo_workspace(root,
  members)`; migrated by importing the shared
  `seed_cargo_workspace_members` under the local alias `seed_cargo_workspace`,
  preserving every call site verbatim. The setter manifest also moved from
  edition 2024 to 2021 — the shared seed format — without breaking any
  test (sniff workspace detection is edition-agnostic).
- `completion_cli.rs` previously had `run_complete(cwd, &home, argv_tail,
  current)` plus a `run_complete_trailing` wrapper. Since every call site
  used the trailing-cursor wrapper, both local helpers were deleted and
  the shared `run_complete` is imported under the local alias
  `run_complete_trailing` so call sites remain untouched.
- `completion_perf.rs` shared `write_file` (re-imported under the local
  alias `write`) and `fake_home`; the timed `run_complete_once` is left
  local because it has a different return type (`Duration`) and is not a
  duplicate of the shared `run_complete`.
- Tests verified: `cargo test -p claudine-cli` (all completion test
  binaries green; full suite green); `cargo test -p claudine` (lib + doc
  tests green); `cargo clippy -p claudine -p claudine-cli --all-targets
  -- -D warnings` clean (also clean with `--all-features`); `cd claudine
  && just lint` clean.
- Commit hash: _set by orchestrator_.

---

## Phase 2 — Setter-value test coverage (findings 2, 3)

**Goal:** Add regression guards for two real-world setter-value scenarios
the existing tests skip over: deeply-nested feature-directory paths and
plain-git checkouts (no `Cargo.toml`, no `sniff` workspace detection).

**Scope:**

- `claudine/cli/tests/completion_setter.rs`

**Code changes:**

1. **Nested setter-value path test (finding 2).** Add a new integration
   test mirroring the spec's literal example:

   ```rust
   #[test]
   fn setter_resolves_nested_feature_directory_path() {
       // Spec example:
       //   `claudine compose foobar.md spec=@spec<tab>`
       //   resolves to `'docs/2026-04-24-improved-shell-completions/spec.md'`
       let ws = TestWorkspace::new();
       let root = ws.path();
       seed_cargo_workspace(root);
       init_git_repo(root);
       write_file(
           &root.join("docs/2026-04-24-improved-shell-completions/spec.md"),
           "# spec\n",
       );

       let candidates = run_complete(
           root,
           root,
           3,
           &["claudine", "compose", "foobar.md", "spec=@spec"],
       );

       assert!(
           candidates.iter().any(|c| {
               c == "spec='docs/2026-04-24-improved-shell-completions/spec.md'"
           }),
           "nested feature-dir spec path not surfaced: {:?}",
           candidates,
       );
   }
   ```

   The recursion contract is already implemented in the walker; this
   test exists purely to lock it in. The single-quote wrapping and the
   exact relative path (with the dated feature directory) are both load-
   bearing — assert the full token, not just `contains("spec.md")`.

2. **Plain-git setter-value test (finding 3).** Add a second integration
   test exercising `setter_value::resolve_setter_scopes` against a bare
   `.git` checkout (no `Cargo.toml`, no `sniff` workspace detection):

   ```rust
   #[test]
   fn setter_resolves_in_plain_git_checkout() {
       // review-3 finding 3: review-2 added plain-git fixtures for
       // composition, but setter-value completion also depends on
       // effective_repo_root via repo_or_cwd(ctx). Lock in the regression
       // guard so a future scopes.rs refactor does not silently break
       // setter-value behavior in repos without a Cargo workspace.
       let ws = TestWorkspace::new();
       let root = ws.path();
       std::fs::create_dir_all(root.join(".git")).unwrap();
       // intentionally NO Cargo.toml — sniff::detect_repo_structure
       // returns None for this layout.
       write_file(&root.join("docs/spec.md"), "# spec\n");

       let nested = root.join("nested").join("child");
       std::fs::create_dir_all(&nested).unwrap();

       let candidates = run_complete(
           root,
           &nested,
           3,
           &["claudine", "compose", "foobar.md", "spec=@spec"],
       );

       assert!(
           candidates.iter().any(|c| c == "spec='docs/spec.md'"),
           "plain-git setter-value did not resolve docs/spec.md: {:?}",
           candidates,
       );
   }
   ```

3. **No source changes.** Both tests assert behavior already implemented
   by `review-plan-2.md` Phase 4 (`effective_repo_root`). If either test
   fails on first run, the failure is an undiscovered regression in
   `setter_value::resolve_setter_scopes` — fix it in the same commit and
   note the fix in the phase tracking table.

**New tests:** Two integration tests, listed above. No unit tests
required — the unit-level walker recursion path is already covered by
existing `composition.rs` walker tests.

**Verification:**

```bash
cd /Users/ken/.claudine/worktrees/rusty-biscuit/claudine
cargo test -p claudine-cli --test completion_setter
cargo test -p claudine-cli
cargo test -p claudine
cargo clippy -p claudine -p claudine-cli --all-targets -- -D warnings
```

**Success criterion:** both new tests pass on first run (or, if not, the
underlying defect is fixed in the same phase); existing setter tests
continue to pass; zero clippy warnings.

### Phase 2 tracking status

| Item                                                            | Status |
|-----------------------------------------------------------------|:------:|
| `setter_resolves_nested_feature_directory_path` added & passing |   ☑   |
| `setter_resolves_in_plain_git_checkout` added & passing         |   ☑   |
| `cargo test -p claudine-cli --test completion_setter` green     |   ☑   |
| `cargo test -p claudine-cli` green                              |   ☑   |
| `cargo clippy -p claudine -p claudine-cli --all-targets -- -D warnings` clean |   ☑   |

**Phase 2 notes:**
- Both new tests pass on first run; the underlying behavior was already
  correctly implemented by review-plan-2 Phase 4 (`effective_repo_root`).
  No source changes were required.
- Tests use the shared `run_complete(cwd, argv_tail)` helper from
  `common::completion`, which prepends `claudine` and infers the cursor
  position from the trailing argv. The plan's example used a 4-arg
  signature (`root, root, 3, &["claudine", ...]`) — translated to the
  current 2-arg helper API without changing semantics.
- `setter_resolves_nested_feature_directory_path` seeds a Cargo workspace
  + a deeply-nested `docs/2026-04-24-improved-shell-completions/spec.md`
  and asserts the full single-quoted token surfaces.
- `setter_resolves_in_plain_git_checkout` seeds **only** `.git/` (no
  `Cargo.toml` — `sniff::detect_repo_structure` returns `None`), runs
  from a nested cwd, and asserts the setter-value completer still routes
  through `effective_repo_root` to surface `docs/spec.md`.
- `seed_plain_git_repo` is imported from `common::completion`; `std::fs`
  is imported at the top of the file for the nested-cwd `create_dir_all`.
- Tests verified: `cargo test -p claudine-cli --test completion_setter`
  (17 passed, was 15); full `cargo test -p claudine-cli` green (104
  integration + lib tests all pass); `cargo test -p claudine` green
  (lib + doc tests pass); `cargo clippy -p claudine -p claudine-cli
  --all-targets -- -D warnings` clean; `cargo clippy ... --all-features
  -- -D warnings` also clean.
- Commit hash: _set by orchestrator_.

---

## Phase 3 — Magic-mode directory parity (finding 4)

**Goal:** Resolve the asymmetry between Word and Magic modes for short
prefixes. The spec is silent on whether `@pl<TAB>` should surface
`prompts/` and `planning/` directories the way Word mode's `pl<TAB>`
does. We will **mirror Word-mode behavior into Magic mode** so the user's
mental model — "`@` is the search sigil" — is consistent across prefix
lengths.

**Decision rationale (record in topic doc, finding 4 close-out):** The
asymmetry is the kind of "spec-silent + UX surprise" that becomes
production friction. Mirroring is cheap (the walker, fuzzy matcher, and
prefix-progression machinery already exist for Word mode) and keeps the
magic and non-magic surfaces converged. The alternative — documenting
the divergence — increases the doc surface and locks in a behavior most
users will not expect.

**Scope:**

- `claudine/cli/src/completion/composition.rs`
- `claudine/cli/tests/completion_compose.rs`
- `claudine/cli/tests/completion_inline_compose.rs`
- `claudine/cli/tests/completion_sequence.rs`
- `claudine/docs/topics/shell-completions.md` (update only the magic
  section; finding 1 + 7 are landed in Phase 5)

**Code changes:**

1. **Surface directories from `gather_magic`.** Currently `gather_magic`
   gates directory inclusion on `partial_len.directories_allowed()`,
   which only fires at `Long`. Mirror Word-mode behavior by reusing the
   `dir_match_mode` axis added in `review-plan-1.md` Phase 3:

   - At `partial_len == Empty`: still no directory candidates (Empty
     stays directory-free across all modes — finding 4 explicitly
     excludes Empty).
   - At `partial_len == Short`: emit directory candidates from the
     repo-wide directory walk via the same helper Word mode uses
     (`resolve_repo_dir_walk_root` + the second-pass loop in
     `gather_empty_or_word`). Match by **prefix** on the leaf segment.
   - At `partial_len == Long`: emit directory candidates with **fuzzy**
     leaf matching, same as Word mode.

   The cleanest landing: extract the second-pass directory-walk loop
   from `gather_empty_or_word` into a private helper

   ```rust
   fn gather_repo_dirs(
       ctx: &ScopeContext,
       partial_active: &str,
       partial_len: PartialLen,
       seen: &mut HashSet<PathBuf>,
   ) -> Vec<CandidateEntry>
   ```

   so both `gather_empty_or_word` and `gather_magic` call the same
   function. The `seen` set is owned by the caller so file-tier dedup
   stays correct.

   In `gather_magic`, after collecting the first-tier file matches (the
   "first-hit-wins" tier from review-plan-2 Phase 2), invoke
   `gather_repo_dirs(ctx, magic.active.as_str(), partial_len, &mut seen)`
   and append. Critically: the **first-hit-wins shadowing rule from
   review-plan-2 still applies to file tiers**. Directory candidates are
   independent of the file-tier shadowing decision — they always come
   from the repo-wide walk regardless of which scope tier won the file
   match. Document this explicitly in the doc update below.

   The match target for directories is the leaf segment (`active`
   portion of `Magic { dir, active }`), not the full magic path. The
   `dir` portion (when non-empty) selects the walk root via
   `resolve_magic_walk_root`, exactly as for files.

2. **Render directory candidates as repo-relative + trailing `/`.** Same
   contract Word mode already uses. The existing `render_scope_insert`
   from `review-plan-2.md` Phase 1 handles repo-relative rendering; the
   trailing-`/` append happens in the helper. No new rendering paths.

3. **Source rank.** Use `REPO_DIR_WALK_RANK` (already defined per
   `review-plan-1.md` Phase 3). Magic-tier directory candidates sort
   after magic-tier file candidates within the rendered output.

**New / updated tests:**

Add to `composition.rs` `#[cfg(test)] mod tests`:

- `compose_magic_short_prefix_surfaces_repo_dir`: seed `prompts/plan.md`
  AND `planning/` dir; query `@pl`; assert both `prompts/plan.md` and
  `planning/` appear (planning/ has trailing `/`).
- `compose_magic_long_prefix_fuzzy_matches_repo_dir`: seed
  `documentation/` dir; query `@dcm`; assert `documentation/` surfaces
  (fuzzy leaf match).
- `compose_magic_empty_partial_does_not_surface_repo_dirs`: regression
  guard. Query `@`; assert no directory candidates emitted (Empty stays
  directory-free even for magic mode).
- `compose_magic_path_shaped_short_prefix_surfaces_subdir`: seed
  `prompts/plan.md` and `prompts/planning/` dir; query `@prompts/pl`;
  assert both `prompts/plan.md` and `prompts/planning/` appear (the
  `dir` portion drives the walk root, the `active` portion is the leaf
  match).

Add to `cli/tests/completion_compose.rs`:

- `compose_magic_short_prefix_surfaces_repo_dirs`: seed `docs/` and
  `features/` at repo root, plus `~/.claudine/prompts/plan.md` (so a
  file tier still wins for the leaf `d` in user-global). Query
  `compose @d`. Assert `docs/` and `features/` both surface alongside
  any file-tier hit. The directory walk must run regardless of which
  file tier won.
- `compose_magic_dirs_independent_of_file_tier_shadow`: seed
  `prompts/plan.md` (repo file tier wins),
  `<home>/.claudine/prompts/planning/something.md` (lower file tier,
  shadowed), and a real `planning/` directory at repo root. Query
  `@pl`. Assert `prompts/plan.md` (file tier 1) appears, the user-
  global `something.md` does NOT (shadowed), AND `planning/` appears
  (directory walk runs unconditionally).

Add to `cli/tests/completion_inline_compose.rs` and
`cli/tests/completion_sequence.rs`:

- One each — `inline_compose_magic_short_prefix_surfaces_repo_dirs` and
  `sequence_magic_short_prefix_surfaces_repo_dirs` — pinning that all
  three composition modes mirror Word/Magic for directories at Short
  prefix length.

**Documentation update (in this phase, magic section only):**

In `claudine/docs/topics/shell-completions.md`, update the `@` magic
section to state:

- Directories **also** surface in magic mode at Short and Long prefix
  lengths, mirroring Word mode. Empty (`@<TAB>`) remains directory-free.
- Directory candidates come from the **repo-wide directory walk**, not
  from the magic priority tier set. Therefore directory output is
  **independent** of which scope tier won the file-tier shadow check.
- Add a "Why" paragraph: keeping `@` and Word mode symmetrical means
  users do not have to remember which sigil "unlocks" directory drill-
  down at short prefix lengths. The first-hit-wins rule continues to
  apply to **files**, not directories.

The full docs sweep for findings 1 and 7 happens in Phase 5; this is a
narrow, in-phase update tied to the behavior change.

**Verification:**

```bash
cd /Users/ken/.claudine/worktrees/rusty-biscuit/claudine
cargo test -p claudine-cli composition::tests
cargo test -p claudine-cli --test completion_compose
cargo test -p claudine-cli --test completion_inline_compose
cargo test -p claudine-cli --test completion_sequence
cargo test -p claudine-cli
cargo test -p claudine
cargo clippy -p claudine -p claudine-cli --all-targets -- -D warnings
```

**Success criterion:** all new tests pass; existing tests still pass;
zero clippy warnings; the topic doc's magic section reflects the new
behavior.

### Phase 3 tracking status

| Item                                                                          | Status |
|-------------------------------------------------------------------------------|:------:|
| `gather_repo_dirs` extracted as private helper                                |   ☑   |
| `gather_magic` invokes `gather_repo_dirs` at Short/Long; skips at Empty       |   ☑   |
| Magic-tier directory candidates dedup against file-tier candidates via `seen` |   ☑   |
| `composition.rs` unit tests added (4 new)                                     |   ☑   |
| `completion_compose.rs` integration tests added (2 new)                       |   ☑   |
| `completion_inline_compose.rs` integration test added (1 new)                 |   ☑   |
| `completion_sequence.rs` integration test added (1 new)                       |   ☑   |
| Topic doc magic section updated for directory parity                          |   ☑   |
| `cargo test -p claudine-cli` green                                            |   ☑   |
| `cargo clippy -p claudine -p claudine-cli --all-targets -- -D warnings` clean |   ☑   |

**Phase 3 notes:**
- `gather_repo_dirs` was generalised to take an explicit `walk_scope` and
  `render_base` so both the repo-wide walk (Word mode) and the magic-
  resolved subdirectory walk (path-shaped Magic mode) can share the same
  matching/rendering body. Render base remains the repo / cwd root so
  `<repo>/prompts/planning/` renders as `prompts/planning/` regardless of
  walk root.
- `gather_magic` was split into `gather_magic_files` (first-hit-wins file
  tiers) and `gather_magic_dirs` (directory walk independent of file
  shadowing). The directory pass receives the file pass's `seen` set so a
  directory that already surfaced as a file is not double-emitted; for
  empty `dir` it walks `resolve_repo_dir_walk_root`, for path-shaped
  `dir` it walks the magic-resolved subdirectory of the highest-priority
  scope whose joined walk root resolves on disk.
- File-tier `seen` is intentionally re-initialised per scope inside
  `gather_magic_files` (to match the original first-hit-wins contract);
  the returned `seen` carries only the winning tier's files into the
  directory pass. This avoids stale dedup entries from earlier non-
  winning tiers leaking into directory output.
- 4 new unit tests in `composition.rs` (short/long/empty/path-shaped),
  2 new integration tests in `completion_compose.rs` (short prefix dirs;
  shadow independence), 1 each in `completion_inline_compose.rs` and
  `completion_sequence.rs` to pin parity across all three composition
  modes.
- Topic doc gained a `Directory candidates in magic mode` subsection
  under `Magic @ resolution` documenting the new behaviour, the
  Empty/Short/Long matrix, the bare-vs-path-shaped walk root distinction,
  and the file-tier shadow independence rule.
- Tests verified: `cargo test -p claudine-cli --bin claudine
  completion::composition` (39 tests, was 35); `cargo test -p
  claudine-cli --test completion_compose` (32 tests, was 30); `cargo
  test -p claudine-cli --test completion_inline_compose` (8 tests,
  was 7); `cargo test -p claudine-cli --test completion_sequence` (8
  tests, was 7); `cargo test -p claudine-cli --test completion_setter`
  (17 tests, unchanged); `cargo test -p claudine` green; `cargo clippy
  -p claudine -p claudine-cli --all-targets -- -D warnings` clean (also
  clean with `--all-features`); `cd claudine && just lint` clean.
- One pre-existing flaky test surfaced in `wrap_commands` (broken pipe
  in `explicit_provider_flag_bypasses_chooser`); passes when run in
  isolation. Unrelated to Phase 3 changes.
- Commit hash: _set by orchestrator_.

---

## Phase 4 — Bootstrap debug switch (finding 6)

**Goal:** Give users an opt-in path to surface engine panics from inside
the shell, without changing the production-default silent behavior.

**Scope:**

- `claudine/cli/src/completion/bootstrap.rs`
- `claudine/cli/tests` (bootstrap unit tests live in `bootstrap.rs`'s
  own `#[cfg(test)] mod tests`)

**Code changes:**

1. **Bash branch (around `bootstrap.rs:104`).** Replace the unconditional
   `2>/dev/null`:

   ```bash
   candidates=( $(command claudine __complete --current "$COMP_CWORD" -- "${COMP_WORDS[@]}" 2>/dev/null) )
   ```

   with a branch on `CLAUDINE_COMPLETION_DEBUG`:

   ```bash
   if [ -n "${CLAUDINE_COMPLETION_DEBUG:-}" ]; then
       candidates=( $(command claudine __complete --current "$COMP_CWORD" -- "${COMP_WORDS[@]}") )
   else
       candidates=( $(command claudine __complete --current "$COMP_CWORD" -- "${COMP_WORDS[@]}" 2>/dev/null) )
   fi
   ```

2. **Zsh branch.** Mirror the same `[ -n "${CLAUDINE_COMPLETION_DEBUG:-}" ]`
   branch on the equivalent zsh invocation. Use POSIX `[ ... ]`, not
   `[[ ... ]]`, so a strict ZSH_NULLCMD configuration cannot break the
   conditional.

3. **Fish branch.** Add the same gate using fish's own syntax:

   ```fish
   if set -q CLAUDINE_COMPLETION_DEBUG
       set -l candidates (command claudine __complete --current $idx -- $argv_all)
   else
       set -l candidates (command claudine __complete --current $idx -- $argv_all 2>/dev/null)
   end
   ```

4. **No protocol changes.** The bootstrap continues to read candidate
   lines from stdout. The only change is whether stderr is redirected.

5. **Update bootstrap doc-comment** at the top of the
   `render_bash_script` / `render_zsh_script` / `render_fish_script`
   functions. Each must mention the new env-var contract:

   > Setting `CLAUDINE_COMPLETION_DEBUG=1` in the user's shell environment
   > exposes engine stderr at the next `<TAB>` press; without it, stderr
   > is swallowed so transient panics do not corrupt the user's prompt.
   > This is an operational-visibility flag, not a feature flag — the
   > completion engine itself ignores the variable.

6. **No engine changes.** The completion engine never reads
   `CLAUDINE_COMPLETION_DEBUG`. The flag is a shell-script affordance
   only.

**New tests:**

In `bootstrap.rs`'s `#[cfg(test)] mod tests`:

- `bash_script_swallows_stderr_by_default`: assert the rendered bash
  script contains `2>/dev/null` and the conditional check on
  `CLAUDINE_COMPLETION_DEBUG`.
- `bash_script_exposes_stderr_under_debug_env`: assert the rendered bash
  script contains a branch guarded by the env var that drops the
  `2>/dev/null`.
- `zsh_script_swallows_stderr_by_default`: same as bash, for zsh.
- `zsh_script_exposes_stderr_under_debug_env`: same as bash, for zsh.
- `fish_script_swallows_stderr_by_default`: same for fish.
- `fish_script_exposes_stderr_under_debug_env`: same for fish.

The existing `fish_script_allows_file_completion_fallback` test (added
by review-plan-2) continues to pass because the file-fallback path is
orthogonal to the stderr branch.

**Verification:**

```bash
cd /Users/ken/.claudine/worktrees/rusty-biscuit/claudine
cargo test -p claudine-cli completion::bootstrap
cargo test -p claudine-cli --test command_routing
cargo test -p claudine-cli
cargo test -p claudine
cargo clippy -p claudine -p claudine-cli --all-targets -- -D warnings
```

Optional manual smoke (not gated automated):

```bash
# In a real bash shell, after building the binary:
CLAUDINE_COMPLETION_DEBUG=1 \
  bash -c 'eval "$(./target/debug/claudine completions bash)"; complete -p claudine'
```

The eval should succeed; engine panics now print to stderr.

**Success criterion:** all six new bootstrap tests pass; existing
bootstrap tests continue to pass; zero clippy warnings.

### Phase 4 tracking status

| Item                                                                | Status |
|---------------------------------------------------------------------|:------:|
| Bash bootstrap gates `2>/dev/null` on `CLAUDINE_COMPLETION_DEBUG`   |   ☑   |
| Zsh bootstrap gates `2>/dev/null` on `CLAUDINE_COMPLETION_DEBUG`    |   ☑   |
| Fish bootstrap gates `2>/dev/null` on `CLAUDINE_COMPLETION_DEBUG`   |   ☑   |
| Bootstrap doc comments mention the env-var contract                 |   ☑   |
| 6 new bootstrap unit tests added & passing                          |   ☑   |
| `cargo test -p claudine-cli` green                                  |   ☑   |
| `cargo clippy -p claudine -p claudine-cli --all-targets -- -D warnings` clean |   ☑   |

**Phase 4 notes:**
- Bash uses POSIX `if [ -n "${CLAUDINE_COMPLETION_DEBUG:-}" ]; then ... else ... fi`
  to choose between an unredirected `__complete` invocation (debug arm) and
  the original `2>/dev/null`-suffixed call (default arm). The conditional
  uses POSIX `[ ... ]` rather than bash `[[ ... ]]` so the script remains
  portable across shells with strict configurations.
- Zsh mirrors the bash pattern using POSIX `[ ... ]` (per the plan's
  guidance about ZSH_NULLCMD); the debug arm captures via
  `${(@f)$(... )}` without `2>/dev/null`, the default arm retains it.
- Fish uses native `if set -q CLAUDINE_COMPLETION_DEBUG ... else ... end`.
  The `set -l candidates` declaration is hoisted above the conditional
  (Fish requires `set -l` once before reassignment).
- Doc comments on `BASH_SCRIPT`, `ZSH_SCRIPT`, and `FISH_SCRIPT` now
  describe the env-var contract verbatim from the plan, plus an inline
  `#` comment in each script body so script readers see the contract.
- The completion engine itself is unchanged — `CLAUDINE_COMPLETION_DEBUG`
  is purely a shell-script affordance.
- 6 new unit tests in `bootstrap.rs::tests`:
  - `bash_script_swallows_stderr_by_default`
  - `bash_script_exposes_stderr_under_debug_env`
  - `zsh_script_swallows_stderr_by_default`
  - `zsh_script_exposes_stderr_under_debug_env`
  - `fish_script_swallows_stderr_by_default`
  - `fish_script_exposes_stderr_under_debug_env`
  Each pair asserts (a) the default arm contains `2>/dev/null` and the
  env-var name, and (b) the conditional structure exists with a debug arm
  invoking `__complete` without stderr redirection. Substring matching is
  scoped to the conditional shape rather than line-by-line equality so
  cosmetic edits do not flap.
- Tests verified: `cargo test -p claudine-cli --bin claudine
  completion::bootstrap` (22 passed, was 16); full `cargo test -p
  claudine-cli` green; `cargo test -p claudine` green (lib + doc tests);
  `cargo clippy -p claudine -p claudine-cli --all-targets -- -D warnings`
  clean; `cargo clippy ... --all-features -- -D warnings` also clean.
- Commit hash: _set by orchestrator_.

---

## Phase 5 — `dead_code` sweep + docs refresh (findings 1, 7, 8)

**Goal:** Remove the scaffolding-era `#![allow(dead_code)]` blanket
attributes, clean up any genuinely unused items the lint surfaces, and
fix the two documentation-drift findings (1 and 7) in a single docs
commit.

**Scope:**

- `claudine/cli/src/completion/composition.rs`
- `claudine/cli/src/completion/frontmatter.rs`
- `claudine/cli/src/completion/fuzzy.rs`
- `claudine/cli/src/completion/scopes.rs`
- `claudine/cli/src/completion/setter_value.rs`
- `claudine/cli/src/completion/walker.rs`
- `claudine/docs/topics/shell-completions.md`

**Code changes:**

1. **Remove `#![allow(dead_code)]`** from each of:

   - `composition.rs:26`
   - `frontmatter.rs:28`
   - `fuzzy.rs:28`
   - `scopes.rs:24`
   - `setter_value.rs:35`
   - `walker.rs:36`

2. **Run clippy and triage every newly-flagged item.** Each newly
   surfaced `dead_code` warning falls into one of three buckets:

   - **Genuinely unused** — delete the item. Review-3 explicitly calls
     out `SetterToken::raw_value` as an example. Other plausible
     candidates: leftover scaffold helpers from the original phase plan
     that no current call site exercises.
   - **Used only in tests** — annotate with `#[cfg(test)]` (or, for
     items used by both prod and tests, `#[allow(dead_code)]` on the
     specific item with a `// reason: ...` comment). Prefer per-item
     attribution over crate-level allow.
   - **Genuine API surface that lint can't see** (e.g. a `pub(crate)`
     item used only via a macro expansion) — annotate with
     `#[allow(dead_code)]` at the item level **with a one-line comment
     stating why** the lint cannot see the use.

   Crate-level `#![allow(dead_code)]` must not be re-added.

3. **No semantic changes.** This phase deletes unused items, narrows
   visibility, and adds per-item `#[allow]` where genuinely needed.
   Anything that requires a refactor to remove cleanly is **out of
   scope** — defer to a follow-up. The Phase 5 success bar is "no
   blanket `#![allow(dead_code)]` in the completion module + clippy
   passes with `-D warnings`."

4. **Topic doc fix — finding 1.** Update
   `claudine/docs/topics/shell-completions.md:359-363`. Replace the
   wrong claim about external sequence reference resolution with a
   statement that matches the actual behavior in
   `frontmatter::is_valid_sequence`:

   > **Why presence-only validation for sequence frontmatter.** The
   > completion validator accepts a `sequence:` markdown candidate so
   > long as the `sequence` key is present in frontmatter — it does
   > **not** resolve external `sequence` references (`sequence:
   > steps.yaml`) at completion time. The runtime composition pipeline
   > is the authority on whether a given sequence file actually runs.
   > Resolving externals in the validator would have to re-implement
   > the runtime resolver, double the per-candidate cost in the
   > frontmatter parse path, and still not catch every runtime failure
   > mode. Completion is content to surface the candidate and let
   > runtime fail loudly if the external is missing.

   The choice maps to review-3 finding 1's option (a). Update the
   surrounding paragraphs so the topic doc no longer claims "the
   validator resolves inline or external sequence specs so every
   offered candidate is runnable."

5. **Topic doc fix — finding 7.** Update
   `claudine/docs/topics/shell-completions.md:342-343`. Replace the
   elision:

   > **Scope extras:** `<repo>/docs/`; agent-skill peer directories
   > (`.claude/skills/`, `.codex/skills/`, …) with `follow_links = false`.

   with the full enumeration:

   > **Scope extras:** `<repo>/docs/`; agent-skill peer directories
   > with `follow_links = false`:
   > `.claude/skills/`, `.codex/skills/`, `.gemini/skills/`,
   > `.opencode/skills/`, `.goose/skills/`, `.qwen/skills/`,
   > `.kimi/skills/`. The same seven peers are enumerated in
   > `cli/src/completion/scopes.rs::SKILL_PEER_DIRS` — that constant is
   > the source of truth; if it changes, this list and the spec must
   > change with it.

   Match the spec's enumeration order so the doc, the constant, and
   the spec stay aligned.

6. **No new tests.** Phase 5 is a code-hygiene + docs sweep. The
   existing test suite is the regression guard; if removing an item
   breaks a test, the item was not actually dead and the change must
   be reverted (or the item demoted to per-item `#[allow]`).

**Verification:**

```bash
cd /Users/ken/.claudine/worktrees/rusty-biscuit/claudine
cargo test -p claudine
cargo test -p claudine-cli
cargo clippy -p claudine -p claudine-cli --all-targets -- -D warnings
cargo clippy -p claudine -p claudine-cli --all-targets --all-features -- -D warnings
cargo fmt -p claudine -p claudine-cli --check
cd claudine && just test && just lint
```

The `--all-features` invocation is added here because lint behavior can
diverge under feature flags — a final-phase sweep should catch that.

**Manual doc check:** open
`claudine/docs/topics/shell-completions.md` and verify:

- Section near line 359 no longer claims external references are
  resolved.
- Section near line 342 enumerates all seven peer directories.
- Section about `@` magic mode (Phase 3) describes directory parity.

**Success criterion:**

- No `#![allow(dead_code)]` blanket attributes remain in
  `cli/src/completion/`.
- `cargo clippy -p claudine -p claudine-cli --all-targets --all-features
  -- -D warnings` passes.
- `cd claudine && just test && just lint` passes.
- All eight findings from `review-3.md` are closed (cross-check against
  the closure-criteria checklist below).

### Phase 5 tracking status

| Item                                                                          | Status |
|-------------------------------------------------------------------------------|:------:|
| `#![allow(dead_code)]` removed from `composition.rs`                          |   ☑   |
| `#![allow(dead_code)]` removed from `frontmatter.rs`                          |   ☑   |
| `#![allow(dead_code)]` removed from `fuzzy.rs`                                |   ☑   |
| `#![allow(dead_code)]` removed from `scopes.rs`                               |   ☑   |
| `#![allow(dead_code)]` removed from `setter_value.rs`                         |   ☑   |
| `#![allow(dead_code)]` removed from `walker.rs`                               |   ☑   |
| Genuinely unused items deleted (incl. `SetterToken::raw_value` if confirmed)  |   ☑   |
| Topic doc paragraph at line 359-363 corrected (finding 1)                     |   ☑   |
| Topic doc paragraph at line 342-343 enumerates 7 peers (finding 7)            |   ☑   |
| `cargo clippy -p claudine -p claudine-cli --all-targets --all-features -- -D warnings` clean |   ☑   |
| `cd claudine && just test && just lint` green                                 |   ☑   |

**Phase 5 notes:**
- Removed all six crate-level `#![allow(dead_code)]` attributes from
  `composition.rs`, `frontmatter.rs`, `fuzzy.rs`, `scopes.rs`,
  `setter_value.rs`, and `walker.rs`.
- Clippy flagged three items after the blanket allows came off; triage:
  - `fuzzy.rs::fuzzy_score` — **deleted**. The function had no
    production callers; only its own unit tests exercised it. Removed
    the function plus three associated tests
    (`fuzzy_score_empty_query_is_zero`,
    `fuzzy_score_contiguous_match_is_smaller_than_spread`,
    `fuzzy_score_returns_none_on_mismatch`). Updated the module-level
    rustdoc to drop the score-related paragraph since the matcher now
    is a pure boolean predicate.
  - `fuzzy.rs::PartialLen::repo_directories_allowed` — **deleted**. The
    method's rustdoc cross-referenced `dir_match_mode`, which is the
    actually-used entry point in production. Removed the method plus
    its test (`partial_len_repo_directories_allowed_at_short_and_long`)
    and updated `directories_allowed`'s rustdoc cross-reference to
    point only at `dir_match_mode`.
  - `composition.rs::PartialKind::active_segment` — **gated with
    `#[cfg(test)]`**. Production callers extract the active segment
    inline at the pipeline branch site; the method exists solely to
    drive the `active_segment_varies_by_kind` regression test. Kept
    the method (and its test) for variant-classification coverage.
- `SetterToken::raw_value` — audit cleared. The field is used by
  `run()` (line 97 — `strip_leading_quote(parsed.raw_value)`) so it is
  not dead. Tracking entry covers this case.
- Topic doc updated:
  - `inline-compose` "Scope extras" enumerates all seven agent-skill
    peer directories (`.claude`, `.codex`, `.gemini`, `.opencode`,
    `.goose`, `.qwen`, `.kimi`) in spec order with a pointer to
    `cli/src/completion/scopes.rs::SKILL_PEER_DIRS` as source of
    truth. The `sequence` extras still says "same as `inline-compose`"
    so the seven-peer enumeration is single-sourced.
  - The "sequence" "Why" paragraph replaces the (incorrect) claim that
    the validator resolves external references with the corrected
    presence-only explanation per finding 1's option (a). The
    "Frontmatter gate" line was also updated to match —
    "presence-only — the validator does not resolve external
    references" — so the gate description and rationale now agree.
- Tests verified: `cargo test -p claudine` (lib + doc tests green);
  `cargo test -p claudine-cli` (104 tests; 2 flaky pre-existing
  `wrap_commands` tests under contention pass on isolated re-run —
  same flake noted in Phase 3 notes); `cargo clippy -p claudine -p
  claudine-cli --all-targets -- -D warnings` clean; `cargo clippy
  ... --all-features -- -D warnings` clean; `cargo fmt -p claudine
  -p claudine-cli --check` clean (one pre-existing fmt drift in the
  Phase 3 `gather_repo_dirs` call site auto-corrected by `cargo
  fmt`); `cd claudine && just test` green on retry; `cd claudine
  && just lint` clean.
- No new tests added — Phase 5 is a code-hygiene + docs sweep; the
  existing test suite is the regression guard.
- Commit hash: _set by orchestrator_.

---

## Cross-cutting concerns / risks

1. **Helper extraction churn (Phase 1).** Six test files change in a
   single phase. Risk: a stray import or a renamed helper drops a
   compile. Mitigation: lift one helper at a time; run
   `cargo test -p claudine-cli --no-run` after each helper move.

2. **Magic-mode directory parity is a behavior change (Phase 3).** The
   spec is silent, so this is technically not "spec-violating" today,
   but users with the Phase 3 build will see different output than
   users on the pre-Phase-3 build. Document the change in the topic doc
   inline with the behavior change so the doc and code ship together.

3. **First-hit-wins vs directory walk independence.** Phase 3 must NOT
   undo the file-tier shadowing rule from review-plan-2 Phase 2. The
   directory walk runs independently of the file-tier decision. This is
   tested explicitly by
   `compose_magic_dirs_independent_of_file_tier_shadow`.

4. **Dead-code sweep risk (Phase 5).** Removing items eagerly might
   delete something a downstream module silently relied on. Mitigation:
   the test suite is the contract — every `cargo test` must stay green
   after each individual deletion. Delete in small batches; revert
   anything that breaks tests rather than chase the failure.

5. **Bootstrap unit-test brittleness (Phase 4).** The new tests assert
   the rendered script body contains specific substrings. This is a
   tradeoff: too-strict matching brittlely breaks on cosmetic changes;
   too-loose matching does not catch real regressions. Strike the
   balance by asserting the **conditional structure** (`if ...
   CLAUDINE_COMPLETION_DEBUG`, `else`, `2>/dev/null`) rather than exact
   line-by-line equality.

6. **`SetterToken::raw_value` may not actually be dead.** Review-3 lists
   it as a likely candidate, but the plan's Phase 5 mandates running
   clippy and triaging — do not pre-delete. If clippy flags it, delete;
   if it does not, leave it and note in the tracking table that the
   audit cleared it.

7. **No scope creep.** Every item in this plan traces to a specific
   review-3 finding. Drive-by typo fixes, doc cosmetic changes outside
   findings 1 and 7, and "while we're here" refactors in `composition.rs`
   are deferred.

---

## Closure criteria

- [x] **Phase 1:** `cli/tests/common/completion.rs` exists with all six
      shared helpers; six integration files use them; tests + clippy
      green.
- [x] **Phase 2:** nested-feature-dir + plain-git setter-value
      regression tests added and passing; tests + clippy green.
- [x] **Phase 3:** `gather_magic` surfaces directories at Short and Long
      prefix lengths via the shared `gather_repo_dirs` helper; new unit
      and integration tests pinning the parity (incl. independence from
      file-tier shadow) all pass; topic doc magic section updated.
- [x] **Phase 4:** bash, zsh, and fish bootstrap scripts respect
      `CLAUDINE_COMPLETION_DEBUG=1`; six new bootstrap tests pass; tests
      + clippy green.
- [x] **Phase 5:** every `#![allow(dead_code)]` in
      `cli/src/completion/` is gone; topic doc paragraphs at lines
      ~359–363 and ~342–343 are corrected per findings 1 and 7;
      `cd claudine && just test && just lint` green;
      `cargo clippy -p claudine -p claudine-cli --all-targets
      --all-features -- -D warnings` green.
- [x] All eight findings from `review-3.md` are demonstrably closed.
