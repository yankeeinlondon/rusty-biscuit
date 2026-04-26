---
ready: false
---

# Review 2 - Improved Shell Completions

## Findings

### 1. Non-magic prompt completions render the wrong path for `.claudine` and user-global prompt scopes

**Severity:** High

`composition::format_relative_insert()` always renders candidates as `<scope leaf>/<relative path>`:

- `claudine/cli/src/completion/composition.rs:341`

That works for a repo prompt scope like `<repo>/prompts/plan.md`, because the inserted token becomes `prompts/plan.md`. It does not work for these required high-profile scopes:

- `<repo>/.claudine/prompts/plan.md` currently renders as `prompts/plan.md`
- `~/.claudine/prompts/plan.md` currently renders as `prompts/plan.md`

Those are not just display labels; they are the token the shell inserts. Selecting one of these candidates can point the runtime at the wrong file or at a file that does not exist. The magic-path path has special rendering for home and repo-relative paths, but the normal fuzzy/high-profile path does not.

**Expected:** non-magic candidates should render to a runnable path:

- repo `.claudine`: `.claudine/prompts/<file>`
- user global: `~/.claudine/prompts/<file>`
- package-area/package prompts: repo-relative package path, not just `prompts/<file>`, when that is what the runtime needs from the current shell cwd.

**Test gap:** add integration tests for `claudine compose <TAB>` and `claudine compose plan<TAB>` where the only valid prompt is under repo `.claudine/prompts` and where the only valid prompt is under fake-home `~/.claudine/prompts`.

### 2. `@` magic lookup is ordered, but it does not implement the designed "first hit wins" semantics

**Severity:** Medium

The spec says `@` resolution follows strict priority and "the first hit wins"; the tech design repeats that the first matching scope should win. The implementation loops over every magic scope and collects every matching candidate:

- `claudine/cli/src/completion/composition.rs:377`
- `claudine/cli/src/completion/composition.rs:419`
- `claudine/cli/src/completion/composition.rs:428`

The current tests only assert that repo-local results sort before user-global results, and in fact require both to appear. That locks in behavior that conflicts with the spec's priority contract. In practice, `@plan<TAB>` can show both `docs/plan.md` and `~/.claudine/prompts/plan.md` even though the repo-local hit should shadow the global one.

**Expected:** for a given magic query, once a higher-priority scope produces candidate(s), lower-priority scopes should not be consulted or emitted according to the first-hit rule. If the desired behavior is "show all matches, sorted by priority", the spec and design need to be changed explicitly.

**Test gap:** add tests where the same logical prompt exists in repo `prompts/`, repo `.claudine/prompts/`, and fake-home `.claudine/prompts/`, and assert only the winning tier is emitted.

### 3. Fish completions do not fall back to native file completion on non-targeted slots

**Severity:** Medium

The design says "empty stdout -> shell default" for slots the engine does not own. That is true for zsh (`_files`) and bash (`-o bashdefault -o default`), but fish registers:

- `complete -c claudine -f -a '(__claudine_complete)'`
- `claudine/cli/src/completion/bootstrap.rs:197`

The `-f` flag disables fish's default file completion, and the function only prints the engine candidates:

- `claudine/cli/src/completion/bootstrap.rs:189`
- `claudine/cli/src/completion/bootstrap.rs:194`

The comment mentions a `--force-files` retry, but no such retry exists. As a result, `claudine hooks <TAB>`, wrapper flag value slots, and other "Other" classifications produce no fish candidates instead of typical shell behavior. This violates the "Other Commands" requirement and makes fish materially worse than bash/zsh.

**Expected:** either remove `-f` where safe, or implement the documented fish fallback path using `--force-files`/a separate completion rule so empty dynamic output still allows native file completion.

**Test gap:** add a bootstrap-level assertion for the actual fallback mechanism, not just that the fish script registers a function.

### 4. Plain git checkouts are only partially handled; committed paths and magic rendering still ignore `git_root`

**Severity:** Medium

`ScopeContext` intentionally supports a `.git` root even when `sniff` cannot build a `RepoInfo`, and `resolve_compose_scopes()` uses that `git_root` for initial high-profile scopes:

- `claudine/cli/src/completion/scopes.rs:157`
- `claudine/cli/src/completion/scopes.rs:221`
- `claudine/cli/src/completion/scopes.rs:227`

But later paths fall back to `cwd` when `repo_info` is absent:

- `gather_committed()` uses `repo_info.root` or `ctx.cwd`, ignoring `ctx.git_root`: `claudine/cli/src/completion/composition.rs:520`
- `render_magic_insert()` renders repo-relative paths only when `repo_info` exists, otherwise it falls back to scope-leaf rendering: `claudine/cli/src/completion/composition.rs:500`

This breaks bare git repos and git worktrees that are not recognized by `sniff`. Example: from a nested directory in a plain git repo, initial completion can offer `prompts/`, but after accepting `prompts/`, the committed-dir pass resolves it relative to the nested cwd rather than the git root. Likewise, `@.claudine/prompts/plan` in a plain git repo can render as `prompts/plan.md` instead of `.claudine/prompts/plan.md`.

**Expected:** use the same `repo_info.root.or(git_root).unwrap_or(cwd)` base everywhere a repo-relative completion is rendered or walked.

**Test gap:** add integration tests for a temp repo with only `.git` and no Cargo workspace, with cwd nested below the repo root.

## Coverage and Ergonomics Notes

- The focused completion tests pass, but the current suite is strongest for repo-root Cargo workspace fixtures and weakest for alternative scope tiers, fish fallback behavior, and sniff-less git repos.
- The ignored performance harness exists and documents the target, but no non-ignored performance guard runs in the default suite. That is probably acceptable if CI cannot provide stable timing, but the release checklist should include the ignored perf tests or an equivalent local profile run.
- The code would be easier to keep correct if candidate rendering took a scope kind (`RepoPrompts`, `RepoClaudinePrompts`, `UserClaudinePrompts`, `PackagePrompts`, `Docs`, `Skills`) instead of inferring semantics from the scope root's final path component. The current leaf-based renderer is the source of finding 1.

## Verification Run

- `cargo test -p claudine-cli --test completion_compose --test completion_inline_compose --test completion_sequence --test completion_setter --test completion_cli --test completion_perf -- --nocapture`
- `cargo test -p claudine-cli completion:: -- --nocapture`

Both commands passed. `completion_perf`'s three performance tests were ignored by default.

## Production Readiness

Not ready for production. The implementation covers much of the designed surface, but the path-rendering bugs can insert unusable paths, and fish currently lacks the promised fallback behavior for non-composition completion slots.
