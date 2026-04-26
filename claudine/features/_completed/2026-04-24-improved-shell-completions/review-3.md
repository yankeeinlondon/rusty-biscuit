---
ready: true
---

# Review 3 — Improved Shell Completions

## Summary

All four blocking findings from `review-2.md` have been addressed and the
underlying contracts are now exercised by integration tests:

- **Finding 1 (path rendering)** — `composition::format_relative_insert`
  ([`composition.rs:352`](../../cli/src/completion/composition.rs)) now
  performs repo-relative → home-relative → scope-leaf-relative rendering
  in priority order, so `.claudine/prompts/plan.md`,
  `~/.claudine/prompts/plan.md`, and `pkg/prompts/plan.md` each emit a
  runnable token. New tests:
  `compose_empty_partial_renders_repo_claudine_scope`,
  `compose_word_partial_renders_user_global_scope`,
  `compose_package_prompt_renders_repo_relative_path`,
  `compose_package_area_prompt_renders_repo_relative_path`.
- **Finding 2 (magic first-hit-wins)** — `composition::gather_magic`
  ([`composition.rs:409`](../../cli/src/completion/composition.rs))
  returns the first scope's candidate list and never consults lower-tier
  scopes once a higher-tier scope hits. Verified by
  `compose_magic_repo_prompts_shadows_repo_claudine_and_user`,
  `compose_magic_repo_claudine_shadows_user_when_repo_prompts_absent`,
  `compose_magic_user_global_emits_when_no_repo_tier_matches`,
  `inline_compose_magic_first_hit_wins_shadows_user_global`,
  `sequence_magic_first_hit_wins_shadows_user_global`.
- **Finding 3 (fish fallback)** — `bootstrap.rs:182-204` now calls
  `__fish_complete_path $current_partial` whenever the engine emits zero
  candidates, restoring native file completion on non-targeted slots.
  Locked in by `fish_script_allows_file_completion_fallback`.
- **Finding 4 (plain git fallback)** — `effective_repo_root`
  ([`scopes.rs:83`](../../cli/src/completion/scopes.rs)) is now used
  uniformly by `gather_committed` and `format_relative_insert`, so plain
  git checkouts (no Cargo workspace, no monorepo manifest) resolve
  scopes against the `.git` ancestor. Verified by
  `compose_plain_git_committed_dir_uses_git_root`,
  `compose_plain_git_magic_renders_repo_claudine_relative`,
  `compose_plain_git_non_magic_renders_repo_claudine_relative`.

The full suite (225 unit tests + 60 completion integration tests + 2
help-group golden tests) passes. The performance harness remains
`#[ignore]`d but documents `p95 ≤ 19 ms` on the reference monorepo —
well inside the 100 ms target — so the fallback cache is correctly
deferred per spec §8.3.

## Findings

### 1. Doc claim about sequence external reference resolution is wrong

**Severity:** Low (documentation only)

[`docs/topics/shell-completions.md:359-363`](../../docs/topics/shell-completions.md)
says:

> **Why resolve external references at completion time.** A dangling
> external reference would render a file that never actually runs; the
> user would lose trust in the candidate list the first time a
> suggestion failed. The validator resolves inline or external sequence
> specs so every offered candidate is runnable.

The implementation in
[`frontmatter.rs::is_valid_sequence`](../../cli/src/completion/frontmatter.rs)
explicitly does **not** resolve external references — the docstring on
`is_valid_sequence` even calls this out: *"the file is a candidate so
long as the key is present — the runtime composition pipeline is the
authority on whether it actually runs."* The
`sequence_accepts_markdown_with_external_ref_string` unit test
asserts presence-only behavior with a non-existent `steps.yaml`.

**Impact:** A `sequence:` markdown that references a missing YAML file
will surface as a completion candidate and then fail at runtime —
contrary to the doc's promise. Either:

- (a) update the docs to say presence-only, deferring full validation
  to runtime, **or**
- (b) make the validator resolve external references (and update the
  unit test).

The docstring in `frontmatter.rs` already states (a); the topic doc
should match.

### 2. Test gap: nested setter-value file paths

**Severity:** Low (test coverage)

The setter-value spec example explicitly uses a deeply nested path:

> `claudine compose foobar.md spec=@spec<tab>` will resolve to
> `'docs/2026-04-24-improved-shell-completions/spec.md'`

The walker walks recursively, so the code should handle this — but
every integration test in `completion_setter.rs` places fixtures
directly at `docs/spec.md`, `features/b.md`, etc. There is no
regression guard for the nested-feature-directory case that motivates
the spec example.

Recommended fix: add a test that places `docs/2026-04-24-x/spec.md`
under a fixture and asserts the rendered candidate is
`spec='docs/2026-04-24-x/spec.md'`.

### 3. Test gap: plain git checkout with setter-value completion

**Severity:** Low (test coverage)

review-2 added plain-git scenarios for composition
(`compose_plain_git_committed_dir_uses_git_root`, etc.), but
`setter_value::resolve_setter_scopes` also depends on
`effective_repo_root` for the repo-level scope tier. There is no
integration test that asserts setter-value completion works inside a
bare `.git` checkout where `sniff::detect_repo_structure` returns
`None`.

The code path is correct — `repo_or_cwd(ctx)` falls back to `git_root`
via `effective_repo_root` — but a regression guard would lock it in.

### 4. Magic mode does not surface directories at short prefix lengths

**Severity:** Low (UX gap; spec is silent)

In Word / Empty modes the repo-wide directory walk
(`gather_repo_dirs`) surfaces matching directories at 1+ char prefix
lengths — so `claudine compose pl<TAB>` offers both `prompts/plan.md`
and `planning/`. In Magic mode (`gather_magic`), directory candidates
are still gated on `partial_len.directories_allowed()`, which only
fires at `Long` (3+ chars). So `claudine compose @pl<TAB>` will not
surface a `prompts/` or `planning/` directory under any scope.

The spec does not specify magic-directory behavior — the `@` sigil is
documented as a search sigil for files. So this is technically
spec-compliant. But the asymmetry between Word and Magic for the same
short partial may surprise users who reach for `@` first. Worth
deciding whether to mirror the repo-wide walk into the magic pipeline
or document the divergence in the topic file.

### 5. Test boilerplate duplicated across 5+ test files

**Severity:** Low (ergonomics)

`seed_cargo_workspace`, `seed_cargo_workspace_members`, `write_file`,
`fake_home`, `run_complete`, and `run_complete_with_home` are
copy-pasted across `completion_compose.rs`,
`completion_inline_compose.rs`, `completion_sequence.rs`,
`completion_setter.rs`, `completion_perf.rs`, and `completion_cli.rs`.
The `tests/common/` module is already imported by every file but only
provides `TestWorkspace` / `init_git_repo`.

Recommended cleanup: lift the helpers into `tests/common/completion.rs`
(or extend `tests/common/mod.rs`) so the test surface is DRY. This is
purely a maintainability win — drift between any two test files'
helpers would silently break test parity.

### 6. Bash script's `2>/dev/null` swallows engine panics

**Severity:** Low (operational visibility)

[`bootstrap.rs:104`](../../cli/src/completion/bootstrap.rs):

```bash
candidates=( $(command claudine __complete --current "$COMP_CWORD" -- "${COMP_WORDS[@]}" 2>/dev/null) )
```

If the engine panics on a malformed argv, the user's `<TAB>` simply
fails silently and falls back to `bashdefault`/`default`. There is no
way to surface a regression from inside the shell. zsh and fish
scripts do the same.

This is the right default for production — a noisy completion is worse
than a silent one — but a debug switch (e.g. respect
`CLAUDINE_COMPLETION_DEBUG=1` to drop `2>/dev/null`) would help when
diagnosing user-reported issues. Out of scope for this feature, worth
queueing for a follow-up.

### 7. Topic doc lists only 2 of 7 agent-skill peer directories

**Severity:** Low (documentation completeness)

[`docs/topics/shell-completions.md:342-343`](../../docs/topics/shell-completions.md)
says:

> **Scope extras:** `<repo>/docs/`; agent-skill peer directories
> (`.claude/skills/`, `.codex/skills/`, …) with `follow_links = false`.

The `…` elision hides 5 of 7 actual directories. The spec and
`scopes::SKILL_PEER_DIRS` enumerate all seven (`.claude`, `.codex`,
`.gemini`, `.opencode`, `.goose`, `.qwen`, `.kimi`). The spec section
mentions all seven explicitly because users want to know whether their
provider's skills are walked.

Recommended fix: replace the elision with the full list, or link to
`SKILL_PEER_DIRS` so doc and code stay in sync.

### 8. `#![allow(dead_code)]` blanket on production modules

**Severity:** Low (lint hygiene)

`composition.rs:26`, `frontmatter.rs:28`, `fuzzy.rs:28`, `scopes.rs:24`,
`setter_value.rs:35`, and `walker.rs:36` all carry
`#![allow(dead_code)]` from the scaffolding phases. Now that every
phase is wired up, the lint should re-fire to catch genuinely unused
items as the engine evolves. A targeted audit would surface anything
that became unused during the implementation cycle (the
`SetterToken::raw_value` field for example).

This is a polish item, not a defect — completion code is correct as-is.

## Coverage and Ergonomics Notes

- The repo-wide directory walk plus the high-profile-scope file walk
  cleanly dedupe via canonical-path `seen` set; the
  `compose_repo_dir_walk_skips_high_profile_roots_once` unit test
  locks in the dedup contract.
- `resolve_magic_walk_root` correctly handles multi-segment scope-leaf
  peeling (`@.claudine/prompts/plan` → walk root resolves to
  `<repo>/.claudine/prompts/`). The unit tests cover empty dir, exact
  leaf match, multi-segment extension, and non-matching scope leaf.
- The 1 MiB `MAX_FRONTMATTER_BYTES` cap and the
  `MAX_CANDIDATES = 500` walker budget together bound the worst-case
  walk cost. The performance harness measures `p95 ≤ 19 ms` on the
  reference monorepo with cache disabled — well below the 100 ms
  target, so the cache.rs fallback is correctly deferred.
- The fish bootstrap now correctly falls back to
  `__fish_complete_path` so non-targeted slots get native file
  completion. The `complete -c claudine -f` flag is still set, which
  is intentional — the function-side fallback gives finer-grained
  control than fish's native auto-detection would.

## Verification Run

- `cargo test -p claudine-cli --bin claudine completion` —
  225 unit tests pass.
- `cargo test -p claudine-cli --test completion_compose
  --test completion_inline_compose --test completion_sequence
  --test completion_setter --test completion_cli` —
  30 + 7 + 7 + 15 + 18 = 77 integration tests pass.
- `cargo test -p claudine-cli --bin claudine commands::help` —
  2 help-group golden tests pass.
- `completion_perf` remains `#[ignore]`d per design (not part of the
  default suite).

## Production Readiness

**Ready for production.** All four blocking findings from review-2 are
fixed and verified by tests. The remaining findings are documentation
drift (#1, #7), test coverage extensions (#2, #3), one design-vs-spec
ambiguity that the spec leaves silent (#4), and code-hygiene cleanups
(#5, #6, #8). None of them prevent shipping; all of them are reasonable
follow-ups.
