---
phases: 5
start_phase: 4
created: 2026-04-25
source_feature: 2026-04-24-improved-shell-completions
review_source: ./review-2.md
package_area: claudine
packages:
  - claudine
  - claudine-cli
findings_addressed:
  - finding-1: Non-magic prompt completions render wrong paths for repo .claudine, user-global, package, and package-area scopes
  - finding-2: "'@' magic lookup emits lower-priority tiers instead of first-hit-wins results"
  - finding-3: Fish completions do not fall back to native file completion on non-targeted slots
  - finding-4: Plain git checkouts are only partially handled because committed paths and magic rendering ignore git_root
---

# Review-Plan 2: Improved Shell Completions

This plan addresses every production-readiness finding in
[`review-2.md`](./review-2.md). The changes are intentionally narrow: fix
candidate rendering semantics, enforce magic priority, restore fish file
fallback behavior, make git-root handling consistent, and add regression
coverage for the exact scope tiers the review found weak.

## Validation baseline

Run these before editing so any existing failure is known:

```bash
cargo test -p claudine-cli --test completion_compose --test completion_inline_compose --test completion_sequence --test completion_setter --test completion_cli --test completion_perf -- --nocapture
cargo test -p claudine-cli completion:: -- --nocapture
cd claudine && just test
cd claudine && just lint
```

The final gate repeats the focused completion tests plus the full claudine
area test/lint commands. `just lint` must finish without warnings or errors.

## Dependency map

| Finding | Primary modules | Phase |
| ---: | --- | ---: |
| 1 | `completion/scopes.rs`, `completion/composition.rs`, completion integration tests | 1 |
| 2 | `completion/composition.rs`, compose/inline/sequence integration tests | 2 |
| 3 | `completion/bootstrap.rs`, bootstrap unit tests, command-routing smoke tests | 3 |
| 4 | `completion/scopes.rs`, `completion/composition.rs`, plain-git integration tests | 4 |

Phase 1 creates the rendering vocabulary Phase 4 also uses, so it must land
first. Phases 2 and 3 can proceed independently after Phase 1. Phase 5 is the
documentation and validation closeout.

---

## Phase 1 - Explicit Scope Rendering (finding 1)

**Goal:** Stop inferring inserted paths from the final directory component of a
scope. Every completion candidate should render to the path a user can actually
run from the shell.

**Scope:**

- `claudine/cli/src/completion/scopes.rs`
- `claudine/cli/src/completion/composition.rs`
- `claudine/cli/tests/completion_compose.rs`
- Related inline/sequence tests only if their shared helper needs adjustment.

**Code changes:**

1. Add explicit scope semantics in `scopes.rs`.

   Introduce a small enum carried by `Scope`:

   ```rust
   pub(crate) enum ScopeKind {
       RepoPrompts,
       PackageAreaPrompts,
       PackagePrompts,
       RepoClaudinePrompts,
       UserClaudinePrompts,
       RepoDocs,
       AgentSkills,
       RepoDirWalk,
       CommittedDir,
   }
   ```

   Add `kind: ScopeKind` to `Scope` and populate it everywhere scopes are
   created, including tests. Keep `path` and `follow_links` unchanged.

2. Add one helper for the repo-relative base:

   ```rust
   fn effective_repo_root(ctx: &ScopeContext) -> Option<&Path> {
       ctx.repo_info
           .as_ref()
           .map(|info| info.root.as_path())
           .or(ctx.git_root.as_deref())
   }
   ```

   Put this in `scopes.rs` as `pub(crate)` so `composition.rs` and
   `setter_value.rs` can share the same root decision instead of each module
   reimplementing it.

3. Replace `format_relative_insert(scope_root, entry)` with a renderer that
   accepts `&Scope` and `&ScopeContext`.

   Rendering contract:

   - Any repo-rooted scope under `effective_repo_root(ctx)` renders
     repo-relative: `prompts/plan.md`, `.claudine/prompts/plan.md`,
     `pkg/prompts/plan.md`, `claudine/cli/prompts/plan.md`,
     `docs/guide.md`, etc.
   - `UserClaudinePrompts` renders home-relative:
     `~/.claudine/prompts/plan.md`.
   - Fallback for no repo/home match is scope-relative using the old leaf
     behavior, but this should be a defensive path, not the normal case.
   - Directory candidates still append exactly one trailing `/`.

4. Thread the new renderer through `gather_empty_or_word`.

   `WordRenderCtx` should carry `scope: &Scope`, not just `scope_root: &Path`.
   This removes the leaf-based bug for all non-magic high-profile candidates.

5. Add integration coverage for every reviewed broken tier.

   In `completion_compose.rs`, add or expose `run_complete_with_home` so tests
   can seed a fake home. Add tests:

   - `compose_empty_partial_renders_repo_claudine_scope`: only
     `<repo>/.claudine/prompts/plan.md` exists; `claudine compose <TAB>`
     emits `.claudine/prompts/plan.md`, not `prompts/plan.md`.
   - `compose_word_partial_renders_repo_claudine_scope`: same fixture,
     `claudine compose plan<TAB>` emits `.claudine/prompts/plan.md`.
   - `compose_empty_partial_renders_user_global_scope`: only
     `<fake-home>/.claudine/prompts/plan.md` exists; output is
     `~/.claudine/prompts/plan.md`.
   - `compose_word_partial_renders_user_global_scope`: same fixture with
     partial `plan`.
   - `compose_package_prompt_renders_repo_relative_path`: cwd inside `pkg/`,
     only `pkg/prompts/plan.md` exists; output is `pkg/prompts/plan.md`, not
     `prompts/plan.md`.

   If `sniff` fixture setup can cheaply produce a package-area scope, also add
   a package-area prompt test. If it cannot, add a unit test for the renderer
   with a synthetic `ScopeKind::PackageAreaPrompts` and a repo root.

**Phase checks:**

```bash
cargo test -p claudine-cli --test completion_compose -- --nocapture
cargo test -p claudine-cli completion::composition completion::scopes -- --nocapture
cargo clippy -p claudine-cli --all-targets -- -D warnings
```

---

## Phase 2 - Magic First-Hit Priority (finding 2)

**Goal:** Make `@...` a prioritized search, not a merged result set. For any
magic query, the first scope tier that produces valid candidates wins and all
lower-priority tiers are suppressed.

**Scope:**

- `claudine/cli/src/completion/composition.rs`
- `claudine/cli/tests/completion_compose.rs`
- `claudine/cli/tests/completion_inline_compose.rs`
- `claudine/cli/tests/completion_sequence.rs`
- `claudine/docs/topics/shell-completions.md` if examples currently show
  lower-priority results after a higher-priority hit.

**Code changes:**

1. Change `gather_magic` from "append every scope" to "collect one tier".

   For each scope from `set.iter_magic_scopes()`:

   - Resolve its walk root.
   - Walk and filter entries exactly as today.
   - Accumulate matches in a `scope_candidates` vector with a fresh per-scope
     `seen` set.
   - If `scope_candidates` is non-empty, return it immediately.
   - If it is empty, continue to the next scope.

   Keep multiple candidates from the winning scope. The "first hit wins" rule
   suppresses lower tiers, not sibling files within the winning tier.

2. Preserve stable ordering inside the winning tier.

   Reuse `finalize` for lexical sorting/dedup after `gather_magic` returns.
   Keep source rank meaningful, but it should no longer be relied on to make a
   lower-priority tier look correct after it has already been wrongly emitted.

3. Use the Phase 1 renderer for magic output too.

   `render_magic_insert` should either call the new `render_scope_insert` or be
   deleted in favor of that helper. This prevents magic and non-magic rendering
   from diverging again.

4. Replace tests that currently expect "repo result first, global result
   second".

   Add compose tests:

   - `compose_magic_repo_prompts_shadows_repo_claudine_and_user`: seed
     `prompts/plan.md`, `.claudine/prompts/plan.md`, and
     `<home>/.claudine/prompts/plan.md`; `@plan` emits only
     `prompts/plan.md`.
   - `compose_magic_repo_claudine_shadows_user_when_repo_prompts_absent`: seed
     repo `.claudine` and user global only; `@plan` emits only
     `.claudine/prompts/plan.md`.
   - `compose_magic_user_global_emits_when_no_repo_tier_matches`: seed only
     user global; `@plan` emits only `~/.claudine/prompts/plan.md`.

   Update existing inline/sequence magic-priority tests so repo `docs/` or
   skill extras shadow user-global prompts instead of appearing before them.

5. Update docs examples if needed.

   `shell-completions.md` should say "the first matching tier wins" and should
   not show both a repo-local hit and a user-global hit for the same `@plan`
   query.

**Phase checks:**

```bash
cargo test -p claudine-cli --test completion_compose --test completion_inline_compose --test completion_sequence -- --nocapture
cargo test -p claudine-cli completion::composition completion::scopes -- --nocapture
cargo clippy -p claudine-cli --all-targets -- -D warnings
```

---

## Phase 3 - Fish File Fallback (finding 3)

**Goal:** Make fish behave like bash and zsh for slots Claudine's completion
engine does not own: when `__complete` emits no candidates, fish should still
offer normal file completion.

**Scope:**

- `claudine/cli/src/completion/bootstrap.rs`
- `claudine/cli/tests/command_routing.rs` if it has bootstrap assertions that
  need to recognize the new script body.

**Code changes:**

1. Keep one dynamic fish rule with `-f`, but make the function emit file
   candidates itself when the engine is empty.

   Use a fish-side branch like:

   ```fish
   function __claudine_complete
       set -l tokens (commandline -opc)
       set -l current_partial (commandline -ct)
       set -l argv_all claudine $tokens[2..] $current_partial
       set -l idx (math (count $argv_all) - 1)
       set -l candidates (command claudine __complete --current $idx -- $argv_all 2>/dev/null)
       if test (count $candidates) -gt 0
           printf '%s\n' $candidates
       else
           __fish_complete_path $current_partial
       end
   end
   ```

   This keeps curated root/composition candidates controlled by Claudine while
   still restoring native path candidates for `claudine hooks <TAB>`, wrapper
   flag value slots, and other "Other" classifications.

2. Delete or rewrite the stale comment claiming a `--force-files` retry exists.

   The comment should name the actual mechanism: fish invokes
   `__fish_complete_path` when the dynamic engine returns no candidates.

3. Add bootstrap-level assertions.

   Add `fish_script_falls_back_to_files_on_empty_candidates` asserting that
   the rendered fish script:

   - captures engine output in a `candidates` variable,
   - branches on `count $candidates`,
   - calls `__fish_complete_path`,
   - still registers `complete -c claudine -f -a '(__claudine_complete)'`.

   Keep existing tests that verify bash/zsh fallback behavior.

4. Optional smoke check.

   If fish is installed locally, manually run a shell-level check after
   building the binary. This should not be a required automated test because
   CI may not have fish.

**Phase checks:**

```bash
cargo test -p claudine-cli completion::bootstrap -- --nocapture
cargo test -p claudine-cli --test command_routing -- --nocapture
cargo clippy -p claudine-cli --all-targets -- -D warnings
```

---

## Phase 4 - Plain Git Root Consistency (finding 4)

**Goal:** Treat a sniff-less git checkout as a repo everywhere, not only during
initial scope resolution.

**Scope:**

- `claudine/cli/src/completion/scopes.rs`
- `claudine/cli/src/completion/composition.rs`
- `claudine/cli/tests/completion_compose.rs`
- `claudine/cli/tests/completion_inline_compose.rs` or
  `completion_sequence.rs` only if the shared helper is moved.

**Code changes:**

1. Use `scopes::effective_repo_root(ctx)` everywhere a repo-relative path is
   walked or rendered.

   Required call sites:

   - `gather_committed`: base is `effective_repo_root(ctx).unwrap_or(&ctx.cwd)`,
     not `repo_info.root.unwrap_or(cwd)`.
   - Magic rendering: repo-relative rendering should work when only
     `ctx.git_root` exists.
   - Any fallback renderer created in Phase 1 should prefer
     `repo_info.root`, then `git_root`, then `cwd`.

2. Keep `resolve_repo_dir_walk_root` behavior aligned with the helper.

   The existing order already matches `repo_info -> git_root -> cwd`; update it
   to call the helper if doing so avoids duplicate logic.

3. Add plain-git integration fixtures.

   Add a small helper:

   ```rust
   fn seed_plain_git_repo(root: &Path) {
       fs::create_dir_all(root.join(".git")).unwrap();
   }
   ```

   Do not write `Cargo.toml`; these tests specifically assert behavior when
   `sniff::detect_repo_structure` returns `None`.

4. Add tests from a nested cwd.

   Use `let nested = ws.path().join("nested").join("child")` and run
   completion with `current_dir(nested)`.

   Required tests:

   - `compose_plain_git_committed_dir_uses_git_root`: seed
     `<git-root>/prompts/planning/deep.md`; from nested cwd,
     `claudine compose prompts/planning/<TAB>` emits
     `prompts/planning/deep.md`.
   - `compose_plain_git_magic_renders_repo_claudine_relative`: seed
     `<git-root>/.claudine/prompts/plan.md`; from nested cwd,
     `claudine compose @.claudine/prompts/plan<TAB>` emits
     `.claudine/prompts/plan.md`, not `prompts/plan.md`.
   - `compose_plain_git_non_magic_renders_repo_claudine_relative`: same
     fixture with `claudine compose plan<TAB>` emits
     `.claudine/prompts/plan.md`.

   If Phase 1 already added the last non-magic case at repo root, keep both:
   repo-root and nested sniff-less git are distinct regressions.

**Phase checks:**

```bash
cargo test -p claudine-cli --test completion_compose -- --nocapture
cargo test -p claudine-cli completion::composition completion::scopes -- --nocapture
cargo clippy -p claudine-cli --all-targets -- -D warnings
```

---

## Phase 5 - Documentation, Performance, and Final Gate

**Goal:** Ensure the implemented behavior, topic documentation, performance
expectations, tests, and lints are all aligned before the feature is marked
ready.

**Documentation checks:**

- Review `claudine/docs/topics/shell-completions.md`.
- Correct all examples that conflict with first-hit-wins magic behavior.
- Ensure fish fallback text describes the real mechanism after Phase 3.
- Ensure examples for repo `.claudine`, package/package-area prompts, and
  user-global prompts show runnable inserted paths.
- Keep the required `## Performance Optimization` section intact.

**Performance checks:**

- Run the focused perf harness in its default mode:

  ```bash
  cargo test -p claudine-cli --test completion_perf -- --nocapture
  ```

- Run ignored perf tests locally at least once before signoff:

  ```bash
  cargo test -p claudine-cli --test completion_perf -- --ignored --nocapture
  ```

- If ignored perf results show p95 above 150ms on the rusty-biscuit-scale
  fixture, do not hide it in this review pass. Either implement the fallback
  cache from `tech-design.md` section 8.3 or record a follow-up blocker in the
  feature notes before claiming production readiness.

**Final validation commands:**

```bash
cargo fmt --all -- --check
cargo test -p claudine-cli --test completion_compose --test completion_inline_compose --test completion_sequence --test completion_setter --test completion_cli --test completion_perf -- --nocapture
cargo test -p claudine-cli completion:: -- --nocapture
cd claudine && just test
cd claudine && just lint
```

If `just lint` reports any clippy/rustc warning in either `claudine` or
`claudine-cli`, fix it in this pass. The feature is not ready while the
claudine package area has lint warnings or errors.

## Acceptance checklist

- Non-magic candidates from repo `.claudine`, fake-home `.claudine`, package,
  and package-area scopes render to runnable inserted paths.
- `@` magic returns only the first matching priority tier and suppresses lower
  tiers for the same query.
- Fish completion script produces path candidates on empty engine output.
- Plain git checkouts without a sniff-detected workspace use `git_root` for
  committed directories and repo-relative rendering.
- Focused completion tests pass.
- Full `cd claudine && just test` passes.
- Full `cd claudine && just lint` passes with zero warnings/errors.
