---
spec: "/Volumes/coding/wt/rusty-biscuit/fix-magic-globs/darkmatter/fixes/2026-09-30-file-refs-use-magic/spec.md"
plan: "darkmatter/fixes/2026-09-30-file-refs-use-magic/plan.md"
implemented_by: "claude/opus"
started_phase: "1"
source_files_during_phase_1:
    - darkmatter/lib/src/markdown/compose/context/options.rs
    - darkmatter/lib/src/markdown/compose/pipeline/mod.rs
    - darkmatter/lib/src/markdown/compose/preflight/collect.rs
    - darkmatter/lib/tests/l1/main.rs
    - darkmatter/lib/tests/l1/preflight_repository_sigils.rs
    - darkmatter/cli/tests/l1/compose_transclusion.rs
docs_updated_during_phase_1:
    - darkmatter/docs/inline/preflight-checks.md
docs_created_during_phase_1: []
skills_files_updated_during_phase_1:
    - .claude/skills/darkmatter/SKILL.md
    - .claude/skills/darkmatter/compose.md
packages:
    - darkmatter
    - darkmatter-cli
---

# Implementation Log for 2026-09-30-file-refs-use-magic (8 phases)

## Phase 1

### Wave 1

#### Reproduce Incident 1

From `claudine/` with the installed `md` (`md 0.1.0`, `~/.cargo/bin/md`):

```text
$ md compose docs/use-claudine/SKILL.md
⤫ TransclusionError: file reference failure
┃
┃ `^` repository reference requires a repository containing reference CWD
┃ `/Volumes/coding/wt/rusty-biscuit/fix-magic-globs/claudine/docs/use-
┃ claudine`
┃
┃ Check sigil usage: `@` magic, `&` repository root, `^` repository-scoped.
```

#### Spike S1: builder-failure inputs (recorded as R11 in the plan)

Ran with a throwaway crate (`/tmp/s1-spike`, path dependencies on
`biscuit-file` and `sniff`; nothing added to the repository).

- (a) `FileResolutionContext::from_snapshot("relative/dir", ..)` fails
  `validate()` with `RelativeContextDirectory { anchor: RequestDirectory }`.
- (b) A request directory inside a repository plus an opening reference
  through `~` or `{{VAR}}` that resolves outside it fails
  `validate()` after `for_source_reference` with
  `RepositoryRootNotContainingSource`. A request directory in `VAULT` with
  the repository root elsewhere fails the same way. With no repository the
  `~` anchor becomes the tree and validation passes.
- (c) A syntactically corrupt `.git/config` (`[core` unterminated) makes
  `GitRepo::discover` and `find_git_root` return `Err`. Rewriting the config
  as valid repairs it. Every other broken state tried (a `.git` file naming
  a missing or empty gitdir, a garbage `.git` file, an empty `.git`
  directory, a missing `HEAD`, an unreadable `.git` on Unix) is reported as
  **no repository**, not an error. A malformed `HEAD`, format version 99, and
  an unknown extension all discover successfully.
- Phase 4 consequence: DMLS must see a `.git/config` change to drop the
  failed entry (see `message_to_agent` in the spec).

#### Spike S2: matrix and guard topology (recorded under R6 in the plan)

Ran with temporary edits to `claudine/cli` (`Cargo.toml`
`source-inputs`, `tests/l1/s2_spike.rs`) and `darkmatter/dmls`
(`tests/l1/s2_spike.rs`), all reverted afterward.

- A `#[path]`-included copy of `darkmatter/cli/tests/common/source_scan.rs`
  compiled and ran in both `dmls::l1` and `claudine-cli::l1`. The darkmatter
  `l1` binary already does this (`semantic_results_never_persist.rs`).
- With only the `#[path]` include, the planner **refused** the declaration:
  `RealWorkspaceTestInputTests.test_every_declared_source_input_is_read_by_its_declarer`
  failed with "claudine-cli declares darkmatter/cli/tests/common/source_scan.rs
  but no L1 test of it names the path". The index walks `#[path]` modules but
  does not count them as references.
- Adding `include_str!("<same path>")` makes the test pass. At module level
  the planned cell is `claudine-cli ubuntu-latest L1 (binary_id(claudine-cli::l1))`.
  Inside the test function it narrows to
  `(binary_id(claudine-cli::l1) & test(=s2_spike::s2_spike_path_include_compiles))`.
  The R6 alternative: each declaring test spells the shared file with an
  in-function `include_str!` next to the `#[path]` include.
- `claudine-cli::l1` holds both completion and composition tests. Several
  `compose_*` modules there are `#[cfg(unix)]`.

### Wave 2

#### Regression tests (written first, confirmed red before the fix)

- `darkmatter/cli/tests/l1/compose_transclusion.rs::test_compose_repository_sigils_from_a_nested_document`.
  This is a git repository built with `CliProcessFixture::initialize_repository_at`.
  `md` is launched from `repo/pkg` (`ambient_context`) with the relative
  argument `docs/guide/doc.md`. The document holds `::file &amp-target.md`,
  `::file ^caret-target.md`, and `::file ^pkg/docs/cli/index.md`, which is
  the shape of the original report. The test asserts exit 0, all three
  bodies in stdout, no leaked `::file`, and no "requires a repository
  containing reference CWD" message. Before the fix it failed with exactly
  that message for `&`.
- `darkmatter/lib/tests/l1/preflight_repository_sigils.rs` uses the same
  layout through `gix::init`. Each test runs two request shapes that carry
  no file-resolution context: `ComposeOptions::for_document(launch_dir, ..)`,
  which is what `md` builds (the source is inside the request repository),
  and `ComposeOptions::new()` (the source is outside the process's
  repository). The tests:
  - `compose_preflight_resolves_repository_sigils_from_a_nested_document`
    checks the approval set holds each target's command and the graph edges
    resolve to the three canonical target paths, in order;
  - `collect_shell_commands_resolves_repository_sigils_from_a_nested_document`
    checks the entries match the targets' commands, in order;
  - `compose_preflight_approvals_resolves_repository_sigils_from_a_nested_document`
    checks the pre-approved set and the discovered count;
  - `compose_preflight_reports_a_missing_repository_target_as_not_found`
    is the negative case: a missing `&` target still fails, names the
    target, and is no longer the missing-repository precondition.

  All four failed before the fix with "`&` repository reference requires a
  repository containing reference CWD".

#### Preflight fix

- `ComposeOptions::prepare_root(&mut self, &Markdown)` (`pub(crate)`,
  `compose/context/options.rs`) runs, in order, `extend_context_for`,
  `establish_repository_observation`, and `ensure_file_resolution_context`.
  `prepared_root(&self, ..)` is the borrowing form, which clones.
- `run_compose_pipeline` calls `prepare_root` in place of the three inline
  calls.
- Pre-flight calls `prepared_root` at the top of `collect_effects` and
  `collect_frontmatter_shell_commands`. **Departure from the plan's wording:**
  the plan lists `compose_preflight`, `compose_preflight_approvals`, and the
  public `collect_*` entries. Every one of those reaches the walk through
  `collect_effects` (`compose_preflight_approvals` goes through
  `compose_preflight`; `collect_shell_commands` goes through
  `collect_shell_commands_with_graph`), except
  `collect_frontmatter_shell_commands`, which is prepared separately. So two
  call sites cover all five entries, plus the pipeline's
  `validate_pre_approved` and `nested.rs`'s use of it.
- The ambient repository observation lives behind an `Arc<OnceLock>` that
  clones share, so preparing a clone in pre-flight does not add a second
  discovery when the pipeline later runs with the caller's options.
- Doc drift fixed: `establish_repository_observation`'s doc said the "root
  pipeline entry" calls it. It now names `prepare_root`.

#### Docs and skill

- `darkmatter/docs/inline/preflight-checks.md`: collection resolves
  transclusion targets as composition does, so `&` and `^` reach the same
  file from any launch directory.
- `.claude/skills/darkmatter/SKILL.md`: pre-flight is a root entry and
  prepares through `prepare_root`.
- `.claude/skills/darkmatter/compose.md`: the "fixed by the root pipeline
  entry" line now names `prepare_root` and both of its callers.

#### Incident check after the fix

Running `target/debug/md compose docs/use-claudine/SKILL.md` from `claudine/`
with the worktree build exits 0 and prints 88 lines with no unresolved
`::file`.

#### Gates

- `just test` (darkmatter): 8749 passed, 12 skipped, exit 0. The four new
  library tests and the new CLI test ran in it.
- `just lint` (darkmatter): exit 0.
- `just test` (claudine, a downstream caller of `compose_preflight`): 8072
  passed, 9 skipped, exit 0.
- No pre-existing failures seen.
- No cross-OS run in this phase. The change adds no path comparison and no
  `#[cfg]` code, and the tests compare canonicalized paths on both sides.
  Windows evidence is gathered in Phase 8 under the plan's Definition of
  Success.
- Not done, per the phase prompt: Commit 1 and `git verify-commit`. The plan's
  Checkpoint asks for them; they are left to the separate commit process.

#### Requirement-to-test mapping

| Requirement | Test |
|---|---|
| `md compose` resolves `&` from a nested directory | `compose_transclusion::test_compose_repository_sigils_from_a_nested_document` |
| `md compose` resolves `^` (bare and `^pkg/...`, the incident shape) | same test |
| `compose_preflight` collects `&`/`^` targets with no context (both request shapes) | `preflight_repository_sigils::compose_preflight_resolves_repository_sigils_from_a_nested_document` |
| `collect_shell_commands` likewise | `preflight_repository_sigils::collect_shell_commands_resolves_repository_sigils_from_a_nested_document` |
| `compose_preflight_approvals` likewise | `preflight_repository_sigils::compose_preflight_approvals_resolves_repository_sigils_from_a_nested_document` |
| A missing `&` target still fails, as not-found, not as the repository precondition | `preflight_repository_sigils::compose_preflight_reports_a_missing_repository_target_as_not_found` |

The Input Robustness Matrix does not apply (R15): no file-format reader was
added or changed.
