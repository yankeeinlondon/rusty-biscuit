---
review: 1
reviewer: Claude (Opus 4.7)
reviewed: 2026-04-18
ready: false
spec: ./spec.md
plan: ./plan.md
---

# Review 1 — File Completion Supplement

## Summary

The implementation closely tracks the spec across the three packages
(`biscuit-file`, `sniff`, `claudine-cli`), and the test suite is comprehensive
at the unit level. All 14 supplement integration tests and 22 supplement unit
tests pass. The acceptance matrix from the spec is broadly covered.

However, there are a number of **functional, ergonomic, and coverage gaps**
that should be resolved before sign-off — most notably a fish completion
script that contradicts its own documentation (no fallback to default file
completion), and several spec-listed behaviors that have no end-to-end test
coverage.

`ready: false` — not blocking, but the fish bug (#1) and the wrapper
coverage gap (#2) should land before this is shipped to users.

---

## High-Priority Findings

### 1. Fish completion script disables fallback contrary to its own docs (functional bug)

**Files:** `claudine/cli/src/completion/bootstrap.rs:154-171`,
`claudine/docs/shell-completions.md`

The fish script is registered as:

```fish
complete -c claudine -f -a '(__claudine_complete)'
```

The `-f` flag tells fish **never** to fall back to file completion. But the
doc comment in `bootstrap.rs:159-161` claims:

> When it has no candidates (non-targeted positions, errors) fish falls back
> to default file completion via the `--force-files` retry.

There is no `--force-files` retry. The script never re-attempts completion
when `__claudine_complete` returns nothing, so fish users get **no
completion at all** at non-targeted argument positions
(`claudine hooks <TAB>`, `claudine claude /tmp/<TAB>`, etc.) — a regression
vs. the spec's "non-targeted positions stay on default" intent.

bash uses `-o bashdefault -o default` and zsh falls back to `_files`; only
fish is broken.

**Fix:** Either drop `-f` so fish falls back to its native file completion,
or add a wrapper function that retries with `__fish_complete_path` (or
similar) when the engine returns empty. The doc comment must agree with
whatever ships.

**Acceptance criterion** the spec's framing relies on (criterion 6 cross-
references "shell-agnostic" parity) is silently violated for fish.

---

### 2. Five of seven wrapper subcommands have no end-to-end coverage for `--asp` / `--rsp`

**File:** `claudine/cli/tests/completion_cli.rs`

Acceptance criterion 6 requires `--append-system-prompt` /
`--replace-system-prompt` (and their aliases) to fire on **every wrapped
provider subcommand**: `claude`, `codex`, `gemini`, `goose`, `kimi`,
`opencode`, `qwen`. The integration suite only tests `claude` and `codex`.

The unit-level classifier test
(`classifier_detects_file_flag_value_slot_on_wrapper`) only tests `claude`.

If a future refactor accidentally drops `gemini` from
`FILE_FLAG_SUBCOMMANDS`, no test will fail.

**Fix:** Add a parameterized integration test (or a unit test on
`classify_completion_target`) that walks every entry in
`FILE_FLAG_SUBCOMMANDS` and asserts the `FileFlag` target classifies for
both `--asp` and `--rsp`.

---

### 3. `FILE_FLAG_SUBCOMMANDS` and friends silently drift from the clap surface

**File:** `claudine/cli/src/completion/supplement.rs:46-95`

Four constants — `COMPOSITION_SUBCOMMANDS`, `FILE_FLAG_SUBCOMMANDS`,
`FILE_FLAGS`, `VALUE_BEARING_FLAGS` — are hard-coded duplicates of values
that already exist in `argv.rs` or in clap-derived `WrapperArgs` /
`SharedComposeArgs`. The plan explicitly chose to "keep the supplement
engine self-contained," but `argv.rs` has a drift-detection test
(`composition_flags_with_value_matches_clap_surface`) for exactly this
risk; the supplement does not.

If a new wrapper (`--mistral`, etc.) is added or a value-bearing flag
changes name, the supplement constants will silently fall out of sync. The
classifier will then either ignore the new subcommand entirely or
mis-classify a positional as a setter.

**Fix:** Either:

- (a) Have the supplement consume the same constants from `argv.rs`
  (`argv::WRAPPER_SUBCOMMANDS`, `argv::COMPOSITION_SUBCOMMANDS`,
  `argv::COMPOSITION_FLAGS_WITH_VALUE`) — they are already `pub(crate)`.
- (b) Add a drift test that builds the clap command tree and asserts
  every wrapper / value-bearing flag is present in the supplement's
  static list.

Option (a) is cheaper.

---

### 4. `detect_repo_structure` runs `cargo metadata` on every keypress

**Files:** `claudine/cli/src/completion/supplement.rs:393-396`,
`sniff/lib/src/filesystem/repo/types.rs:530-532`,
`biscuit-file/lib/src/file_reference/context.rs:82-147`

`emit_candidates` calls `detect_repo_structure(repo_root)` for every
completion attempt. That function shells out to `cargo metadata --no-deps`
internally (via `MetadataCommand`). On the rusty-biscuit monorepo (48
workspace members), that call alone takes hundreds of milliseconds — every
single `<TAB>`.

The plan's Phase 3.2 explicitly anticipated this: "Only add a
lighter-weight `cwd → package roots` helper if profiling shows `RepoInfo`
construction is materially too slow." Profiling was not done; the slow
path is shipped.

This is in the spec's **Open Questions** ("Performance budget") so it does
not block correctness, but real users on a real monorepo will feel it. At
minimum, the implementation should:

- Be measured against a representative repo before declaring "ready".
- Either accept the latency explicitly (with a recorded note for users),
  or add a one-shot in-process cache keyed on `(cwd, repo_root)` so a
  rapid sequence of `<TAB>` presses doesn't re-spawn `cargo metadata`
  each time.
- Consider gating the package-root / area-root walk behind 3+ chars so
  the cheapest case (empty input) doesn't pay for it.

---

## Medium-Priority Findings

### 5. The biscuit-file partial-completion API is parsed but its `roots()` are unused

**Files:** `biscuit-file/lib/src/file_reference/resolve.rs:300-336`,
`claudine/cli/src/completion/supplement.rs:282-292`

The Phase 2 deliverable was a `FileReference::complete_partial` API that
returns "the absolute roots a completion consumer should enumerate". The
supplement engine consumes the API, but **only reads `entry_form()`,
`active_segment()`, and `rendered_prefix()`** — it ignores
`completion.roots()` and computes its own roots in `curated_roots`.

The duplication isn't strictly a bug (the supplement adds package-root,
package-area-root, and `~/.claudine/`, which the biscuit-file API
deliberately does not know about), but the code is harder to reason about
because two functions independently compute "where to enumerate" with
overlapping but non-identical logic.

**Fix:** Either:

- Drop `PartialCompletion::roots` from the public API since claudine is
  the only consumer and doesn't use it. Replace with a smaller
  `(form, scope, active)` returner.
- Or have `curated_roots` start from `completion.roots()` and **add**
  the package/area/~/.claudine/ roots, so the biscuit-file roots are
  the authoritative source for repo+home expansion.

The current state is a net-zero — Phase 2 and Phase 4 each implemented the
same logic.

---

### 6. `home_dir()` source diverges between supplement and biscuit-file

**Files:** `claudine/cli/src/completion/supplement.rs:291`,
`biscuit-file/lib/src/file_reference/context.rs:150-152`

- `supplement.rs` uses `dirs::home_dir()`, which on Unix falls back to
  `getpwuid_r` when `$HOME` is unset.
- `biscuit-file::context::home_dir()` is `std::env::var_os("HOME").map(...)`.

When `$HOME` is unset (rare but possible: cron, system services, tests),
`complete_partial` returns no home root, but `curated_roots` still
enumerates `~/prompts/` etc. via `dirs`. The two halves of the same
completion call have inconsistent views of "home".

**Fix:** Use the biscuit-file path consistently — read `$HOME` directly
and skip user-scope when unset (matches the spec's open question:
"Behavior when `HOME` is unset … presumably 'skip silently'").

---

### 7. Acceptance criterion 7 (no-repo fallback, user scope still applies) is not asserted

**File:** `claudine/cli/src/completion/supplement.rs:759-769`

The spec's criterion 7 has two parts:

> When the cwd is not inside any git repository:
> - The curated **user-scope** directories still apply.
> - The repo-scope directories do not exist and are skipped.
> - The "3+ meaningful chars extends to enclosing repo" behavior **does
>   not activate**.

The unit test `no_repo_fallback_skips_broad_scan` only asserts the third
bullet. Neither the unit nor integration suite asserts that user-scope
still produces matches when there is no git repo.

**Fix:** Add a test that seeds `$HOME/prompts/foo.md`, points cwd at a
non-repo temp dir, and asserts `@<TAB>` returns `@prompts/foo.md`.

---

### 8. Acceptance criterion 11 (multi-crate area dedup) has no functional test

**Files:** spec § Acceptance Criteria 11; `claudine/cli/src/completion/supplement.rs`

Criterion 11 specifically describes "When cwd is inside a multi-crate
area (e.g. `claudine/cli/src/`), both `<package-root>/prompts/` and
`<package-area-root>/prompts/` are walked" with dedup by canonical path.

The only dedup test in the suite (`allows_canonical_path_deduplication`
in `sniff/.../docs.rs`) builds a single `prompts/` directory and walks it
twice via the same path — not a multi-crate area scenario. The supplement
engine itself has no test that exercises both `package_for_dir` and
`package_area_for_dir` returning distinct directories with overlapping
markdown.

**Fix:** Add an integration test that seeds a Cargo workspace with two
crates under one area (`area/lib/Cargo.toml`, `area/cli/Cargo.toml`),
puts a file at `area/prompts/shared.md`, points cwd at `area/lib/src/`,
and asserts the file appears exactly once.

---

### 9. Symlink behavior inside curated scopes is unspecified and untested

The spec lists symlinked directories inside curated scopes as an **open
question**. The implementation makes a silent choice: `WalkBuilder` does
not follow symlinks by default, but `std::fs::canonicalize(...)` resolves
them for the dedup key. This means a symlink to a real markdown file
*outside* the walked directory gets deduped against… nothing, because
nothing else is walked.

The risk is small, but if a curated `~/prompts/` is a symlink to
`~/Documents/Prompts/`, candidates render relative to `~/prompts/` while
the canonical path is the symlink target. If both `~/prompts/` and
`~/Documents/Prompts/` happen to be enumerated (e.g., one through a
package-area root and one through `$HOME`), the same file might be
rendered twice with different prefixes — but the canonical-path dedup
catches that.

**Fix:** Either land an explicit unit test confirming the chosen
behavior (symlink to file is followed, symlink to dir is not, and dedup
collapses cross-root duplicates), or add a doc comment in `supplement.rs`
saying "symlink behavior is deliberately unspecified — relies on
`WalkBuilder` defaults".

---

### 10. `find_enclosing_repo` doesn't canonicalize the cwd

**File:** `claudine/cli/src/completion/supplement.rs:427-435`

```rust
fn find_enclosing_repo(cwd: &Path) -> Option<PathBuf> {
    for ancestor in cwd.ancestors() {
        let dot_git = ancestor.join(".git");
        if dot_git.is_dir() || dot_git.is_file() {
            return Some(ancestor.to_path_buf());
        }
    }
    None
}
```

If the shell starts the completion subprocess with a cwd containing `.`
or `..` components, the returned repo root keeps them too — and then
flows into `detect_repo_structure(repo_root)` and into
`std::fs::canonicalize` later. This usually works, but mixed-form paths
make the dedup hash less effective and complicate downstream
`strip_prefix(...)` calls.

**Fix:** Canonicalize cwd at the top of `run()` (or the top of
`emit_candidates`) before walking. Cheap, eliminates a class of
edge-case bugs.

---

## Low-Priority / Polish

### 11. `completion/mod.rs` doc references the wrong feature

**File:** `claudine/cli/src/completion/mod.rs:11-15`

The module doc still references "Phases 2 and 3 of the
`2026-04-17-file-completion` feature." That feature is the
**predecessor**; this is a comment from the old plan that survived the
supplement landing. Should mention `2026-04-18-file-completion-supplement`
or remove the phase reference.

### 12. `completion/file_reference.rs` and `completion/validate.rs` are now legacy-only

The module doc and pub(crate) visibility on these files date from the
`2026-04-17-file-completion` engine. They are still wired into the
`CompleteEnv` legacy path for stale installed scripts, but no test in the
new supplement suite exercises them. A comment at the top of each file
making "legacy path only — not used by `__complete`" explicit would help
future readers avoid breaking the legacy contract while modifying the
supplement engine.

### 13. `compose --asp` test exercises an unrelated rule-3 edge

**File:** `claudine/cli/tests/completion_cli.rs:367-381`

`compose_asp_flag_value_slot_emits_curated_candidates` uses argv
`["compose", "file.md", "--asp", ""]`. Because the engine receives raw
argv (not normalized), this argv classifies fine. But it's worth a
matching test where the cursor is on the `--asp` value slot **before**
any positional, so the classifier's `seen_positional` branch isn't the
one being tested.

### 14. `is_global_bool_flag` doesn't accept `-vvvv` and longer counts

**File:** `claudine/cli/src/completion/supplement.rs:235-240`

`-v`, `-vv`, `-vvv` are listed; `-vvvv` is not. clap allows the count to
go arbitrarily high (`ArgAction::Count`). If a user types
`claudine -vvvv compose <TAB>`, the classifier returns None and no
completion fires. Edge case, but the bound is arbitrary — either accept
any `-v+` token or document the cap.

### 15. `unsupported_form_returns_empty` covers more cases than the spec mandates

**File:** `claudine/cli/src/completion/supplement.rs:771-778`

This unit test asserts that `!pkg`, `vault:x`, `./local`, `/abs/path`
all return empty. That matches the spec's "unsupported entry forms" list,
but the spec doesn't mention `%recursive` or `{{HOME}}` which the
integration suite *does* test in
`unsupported_prefix_returns_no_candidates`. Worth adding `%` and `{{...}}`
to the unit test for parity, since `complete_partial` rejects them too.

### 16. The `0-2 chars curated only` rule is not explicitly tested for 1 char

The spec has a clear table:

| Typed | Meaningful | Behavior |
|-------|------------|----------|
| `@p`  | 1          | curated only |
| `@pr` | 2          | curated only |
| `@pro`| 3          | curated + broad |

Tests cover the **boundary** transition (2 → 3) via
`magic_three_char_extends_to_broad_scan`, but not the 1-char curated case
explicitly. A small test that asserts `@p` (one meaningful char) does
NOT trigger the broad scan would lock in the lower boundary too.

### 17. `meaningful_char_count` is `pub(crate)` but never called from outside `supplement.rs`

**File:** `claudine/cli/src/completion/supplement.rs:270`

If nothing outside the module needs it, drop the visibility. If it's
intentionally exported for future testing or reuse, add a brief comment
saying so.

### 18. `emit_candidates` is `pub(crate)` for the same reason

**File:** `claudine/cli/src/completion/supplement.rs:281`

Same as #17 — `run` is the only call site outside this module. Tighten
visibility unless deliberate.

### 19. No telemetry on the `__complete` subcommand

The spec lists "Observability" as an open question and the docs say
tracing is deliberately off. That's fine for shell pipelines, but
diagnosing user-reported "completion isn't working" without **any**
on-disk trace is going to be painful. Consider:

- A `--debug` flag on `__complete` that writes to
  `~/.claudine/logs/completion.log` (off by default).
- Or honor the existing `RUST_LOG=trace` for that subcommand only and
  redirect to stderr (which fish/zsh/bash all swallow during completion).

This is explicitly in the spec's open questions, so not a blocker, but
worth a note in the residual-risk record.

---

## Spec Compliance Summary

| Criterion | Status | Notes |
|-----------|--------|-------|
| 1. Empty input curated scope | ✅ | Tested at unit + integration |
| 2. `@pr` 2-char substring | ✅ | Integration test passes |
| 3. `@pro` 3-char broad scan | ✅ | Integration test passes |
| 4. `@prompts/` path-reset | ✅ | Tested |
| 5. `prompts/` implicit-relative | ✅ | Tested |
| 6. Wrapper `--asp` / `--rsp` | ⚠️ | Only `claude` + `codex` tested; 5 wrappers untested (#2) |
| 7. No-repo fallback | ⚠️ | Only "broad scan suppressed" half tested (#7) |
| 8. Non-markdown excluded | ✅ | Tested |
| 9. Gitignored excluded | ✅ | Tested |
| 10. Mid-filename substring | ✅ | Tested |
| 11. Multi-crate dedup | ⚠️ | Only proxy unit test; no real multi-crate fixture (#8) |

---

## Ergonomics & Performance

- The hidden `__complete` subcommand spawns a fresh process on every
  keypress, then calls `cargo metadata` inside it. On a 48-member
  workspace this will be perceptibly laggy. See finding #4.
- The rendered candidate ordering is alphabetical via `BTreeSet`. The
  spec leaves this as an open question; the choice of "alphabetical with
  no curated-first ranking" should be recorded explicitly so future
  reviewers know it was a decision, not an oversight.
- `emit_candidates` runs `std::fs::canonicalize` once per discovered
  file. On a large repo, that's a per-file syscall — a HashSet of *path
  strings* would be cheaper for the common case where two roots don't
  collide on the same file. Canonicalize lazily only when the string
  dedup hits a duplicate, if at all.

---

## Suggested Closure Path

Before flipping `ready` to true:

1. Fix the fish script (#1) — small, clear, regression risk for
   non-targeted positions.
2. Add wrapper coverage (#2) — small, parameterized.
3. Add user-scope-without-repo and multi-crate-area integration tests
   (#7, #8).
4. Decide and document the `cargo metadata` cost (#4) — measurement,
   then either accept or cache.

Findings #3, #5, #6, #9-#19 are quality-of-life improvements that can
land in a follow-up.
