---
phases: 5
created: 2026-04-24
source_feature: 2026-04-24-improved-shell-completions
review_source: ./review-1.md
package_area: claudine
packages:
  - claudine-cli
findings_addressed:
  - finding-1: Setter-value completion returns non-Markdown files
  - finding-2: Directory suggestions scoped to high-profile roots only
  - finding-3: One- and two-character directory completion is missing
  - finding-4: '@ magic paths do not support typed path-shaped magic prefixes'
  - finding-5: Magic-path priority for inline/sequence extras
  - finding-6: Compose accepts oversized/unreadable Markdown candidates
---

# Review-Plan 1: Improved Shell Completions

This plan addresses the six production-blocking findings in
[`review-1.md`](./review-1.md) for the `2026-04-24-improved-shell-completions`
feature. The original implementation plan ([`plan.md`](./plan.md)) shipped all
six feature phases; this plan is a follow-up **revision** targeted at closing
the specific gaps the review identified plus the listed test-coverage gaps.

## Validation baseline (run before starting)

Run these once before Phase 1 to establish a clean baseline:

```bash
cargo test -p claudine-cli
cargo clippy -p claudine-cli -p claudine-lib --all-targets -- -D warnings
```

Every phase ends with the same two commands; if the baseline is already red
those failures must be triaged before proceeding so regressions are not
attributed to this plan.

## Dependency map (findings ↔ phases)

| Finding | Module(s) touched                                        | Phase |
|--------:|----------------------------------------------------------|------:|
| 1       | `completion/setter_value.rs`                             | 1     |
| 6       | `completion/frontmatter.rs`                              | 1     |
| 4       | `completion/composition.rs`                              | 2     |
| 5       | `completion/scopes.rs`, `completion/composition.rs`      | 2     |
| 2       | `completion/composition.rs`, `completion/scopes.rs`      | 3     |
| 3       | `completion/fuzzy.rs`, `completion/composition.rs`       | 3     |

Phases 1 and 2 are almost independent (different modules) and could run in
parallel if two executors are available; Phase 3 builds on both so must run
after. Phase 4 is the docs refresh, and Phase 5 is the final sweep / clippy
gate.

---

## Phase 1 — Extension & Size Gates (findings 1, 6)

**Goal:** Enforce the Markdown-only and size-bounded contracts that the spec
describes but the implementation currently loosens.

**Scope:**

- `claudine/cli/src/completion/setter_value.rs`
- `claudine/cli/src/completion/frontmatter.rs`
- `claudine/cli/tests/completion_setter.rs`

**Code changes:**

1. **Setter-value extension gate (finding 1).** In
   `gather_value_candidates` (setter_value.rs, around the
   `for entry in walker::walk_scope(scope)` loop), add a case-insensitive
   Markdown extension check immediately after the `is_dir()` skip and
   **before** the fuzzy match. Reuse the same extension list that
   `frontmatter::has_markdown_extension` uses (`md`, `markdown`,
   case-insensitive via `to_ascii_lowercase`). The cleanest implementation
   is a free function in `setter_value.rs`:

   ```rust
   fn has_markdown_extension(path: &Path) -> bool {
       path.extension()
           .and_then(|ext| ext.to_str())
           .map(|ext| matches!(ext.to_ascii_lowercase().as_str(), "md" | "markdown"))
           .unwrap_or(false)
   }
   ```

   (Do **not** promote `has_markdown_extension` out of `frontmatter.rs` —
   it is `pub(super)`-scoped there and setter_value does not need to depend
   on frontmatter for anything else. A small local copy keeps module
   boundaries clean.)

   Add an early `continue` for any entry that fails the gate, **regardless
   of the user's partial_len classification**. A filename that is not
   Markdown is never a valid setter-value candidate.

2. **Compose size-failure behavior (finding 6).** In `is_valid_compose`
   (frontmatter.rs, lines 63–77), replace:

   ```rust
   let Some(text) = read_text_within_size_cap(path) else {
       // Oversized or unreadable files are accepted for compose because
       // the extension gate already passed and the runtime handles
       // frontmatter-less files uniformly.
       return true;
   };
   ```

   with a **rejection** on failure:

   ```rust
   let Some(text) = read_text_within_size_cap(path) else {
       // Oversized, unreadable, or non-UTF-8 files are rejected so
       // expensive or noisy candidates never surface in compose output.
       // This mirrors the behavior of `is_valid_inline_compose` and
       // `is_valid_sequence` for a single uniform size/read contract.
       return false;
   };
   ```

   Update the rustdoc on `is_valid_compose` to match (delete the "An
   unreadable file is treated as 'no frontmatter' and accepted" sentence,
   replace with a statement that every size/read failure is a rejection —
   matching the inline/sequence paths).

**New / updated tests:**

Add to `setter_value.rs` `#[cfg(test)] mod tests` block:

- `run_excludes_txt_files`: seed `docs/spec.txt` alongside `docs/spec.md`,
  run `spec=@s`, assert `.md` surfaces and `.txt` does not.
- `run_excludes_yaml_files`: seed `docs/plan.yaml`; run `plan=@p`; assert
  no yaml candidate.
- `run_excludes_extensionless_files`: seed `docs/notes`; run `notes=@n`;
  assert no candidate.
- `run_accepts_uppercase_md_extension`: seed `docs/PLAN.MD`; assert it
  surfaces.
- `run_accepts_markdown_long_extension_case_insensitive`: seed
  `docs/README.MARKDOWN`; assert it surfaces.

Add to `frontmatter.rs` `#[cfg(test)] mod tests`:

- `compose_rejects_oversized_file`: seed an `.md` whose body exceeds
  `MAX_FRONTMATTER_BYTES`; assert `valid_for_mode(path, Compose)` returns
  `false`.
- `compose_rejects_non_utf8_markdown`: seed an `.md` whose body is
  deliberately non-UTF-8 (e.g., `fs::write(&path, [0xff, 0xfe, 0x00, 0x00])`);
  assert `valid_for_mode(path, Compose)` returns `false`. Note: write as a
  `&[u8]`, not via the write helper.

Add to `claudine/cli/tests/completion_setter.rs` (integration):

- `setter_excludes_non_markdown_files`: seed `docs/spec.md`,
  `docs/plan.txt`, `docs/notes.yaml`, `docs/extless`; run
  `compose foo.md spec=@`; assert only `spec='docs/spec.md'` appears; the
  non-markdown entries must NOT appear.
- `setter_accepts_uppercase_md_and_markdown`: seed `docs/PLAN.MD`,
  `docs/README.MARKDOWN`; run `compose foo.md ref=@`; assert both surface.

Add to `claudine/cli/tests/completion_compose.rs` (integration):

- `compose_rejects_oversized_markdown_candidate`: seed
  `prompts/huge.md` with body >1 MiB; run `compose` with empty partial;
  assert `huge.md` does NOT appear.
- `compose_rejects_non_utf8_markdown_candidate`: seed
  `prompts/bad.md` with non-UTF-8 bytes; run `compose` with empty partial;
  assert `bad.md` does NOT appear.

**Validation:**

```bash
cargo test -p claudine-cli setter_value
cargo test -p claudine-cli frontmatter
cargo test -p claudine-cli --test completion_setter
cargo test -p claudine-cli --test completion_compose
cargo clippy -p claudine-cli -p claudine-lib --all-targets -- -D warnings
```

All must pass with zero warnings. Success criterion: the new `.txt`,
`.yaml`, extensionless, uppercase-case, oversized, and non-UTF-8 tests all
pass; all existing tests still pass.

---

## Phase 2 — Magic-Path Semantics (findings 4, 5)

**Goal:** Make `@`-prefixed partials handle path-shaped forms
(`@prompts/plan`) and give repo-local extras priority over `user_claudine`.

**Scope:**

- `claudine/cli/src/completion/composition.rs`
- `claudine/cli/src/completion/scopes.rs`
- `claudine/cli/tests/completion_compose.rs`
- `claudine/cli/tests/completion_inline_compose.rs`
- `claudine/cli/tests/completion_sequence.rs`

**Code changes:**

1. **Path-shaped magic prefixes (finding 4).** Today
   `PartialKind::classify` classifies `@prompts/plan` as
   `Magic("prompts/plan")` and `gather_magic` matches that whole string
   against each candidate basename (it wins only against a file literally
   named `prompts/plan.md`, which never exists).

   Fix by extending `PartialKind::Magic` to carry a structured path:

   ```rust
   pub(crate) enum PartialKind {
       ...
       /// `@...` magic path. `dir` is the path portion before the last `/`
       /// (scope-relative); `active` is the fuzzy-match segment after the
       /// last `/`. For a bare `@plan`, `dir` is empty and `active` is
       /// `"plan"`.
       Magic { dir: String, active: String },
       ...
   }
   ```

   Update `PartialKind::classify`:

   ```rust
   if let Some(rest) = token.strip_prefix('@') {
       if let Some((dir, active)) = rest.rsplit_once('/') {
           return Self::Magic {
               dir: dir.to_string(),
               active: active.to_string(),
           };
       }
       return Self::Magic {
           dir: String::new(),
           active: rest.to_string(),
       };
   }
   ```

   Update `active_segment()` to return `active` for the `Magic` variant.

   Update `gather_magic` to:

   - Construct a `walk_root` per scope by joining `scope.path` with `dir`
     when `dir` is non-empty. If the joined path is not a directory, skip
     that scope. Otherwise walk it (not the raw scope.path).
   - Match the final segment against `active` (same fuzzy rules).
   - Render the inserted token as scope-relative (still `prompts/plan.md`
     when matched under `<repo>/prompts/`), **including** the `dir`
     portion. Reuse `render_magic_insert` — but when `dir` is non-empty
     the returned path already includes `dir` because the walker emits
     the fully-qualified entry path. Verify by tracing: `scope.path =
     /repo/prompts`, `dir = ""`, entry = `/repo/prompts/plan.md` →
     `prompts/plan.md`. With `dir = "plan"`, walking
     `/repo/prompts/plan/…` still emits entries prefixed with
     `/repo/prompts/plan/...`, rendered as `prompts/plan/...`. So the
     existing renderer handles both cases — no new logic needed there.

   Update the call-sites in `run()` to pattern-match
   `PartialKind::Magic { dir, active }` instead of `Magic(active)`.

   Update the existing `classify_magic_strips_at_sigil` unit test to the
   new shape:

   ```rust
   assert_eq!(
       PartialKind::classify("@plan"),
       PartialKind::Magic {
           dir: String::new(),
           active: "plan".to_string(),
       }
   );
   assert_eq!(
       PartialKind::classify("@prompts/plan"),
       PartialKind::Magic {
           dir: "prompts".to_string(),
           active: "plan".to_string(),
       }
   );
   assert_eq!(
       PartialKind::classify("@"),
       PartialKind::Magic {
           dir: String::new(),
           active: String::new(),
       }
   );
   ```

   Update the `active_segment_varies_by_kind` test to reflect the new
   shape.

2. **Magic-path iteration order for extras (finding 5).** Today
   `ScopeSet::iter_scopes()` chains scopes as repo → area → package →
   repo_claudine → user_claudine → extras. For inline-compose / sequence
   magic resolution, the spec explicitly orders repo-local extras
   (`docs/`, skill peers) before `user_claudine`.

   Add a dedicated iterator on `ScopeSet`:

   ```rust
   /// Iteration order for magic-path resolution (spec §5.5).
   ///
   /// Repo-local scopes — including mode-specific `docs/` and skill
   /// extras — precede `user_claudine`. This is stricter than
   /// [`iter_scopes`] so project-specific prompts win over global
   /// prompts in the `@` search priority.
   pub(crate) fn iter_magic_scopes(&self) -> impl Iterator<Item = &Scope> {
       self.repo
           .iter()
           .chain(self.package_area.iter())
           .chain(self.package.iter())
           .chain(self.repo_claudine.iter())
           .chain(self.extras.iter())
           .chain(self.user_claudine.iter())
   }
   ```

   In `composition::gather_magic`, replace `set.iter_scopes()` with
   `set.iter_magic_scopes()`. Leave `iter_scopes()` alone — the non-magic
   `gather_empty_or_word` path still uses it with the original order.

**New / updated tests:**

Add to `composition.rs` unit tests:

- `classify_magic_path_shaped`: asserts the `PartialKind::Magic { dir,
  active }` shape for `@prompts/plan`, `@a/b/c`, `@/x`, and bare `@plan`
  (the last two are edge cases worth pinning).
- `compose_magic_path_shaped_resolves_scope_relative`: seed
  `prompts/plan.md`; run `@prompts/plan`; assert the emitted candidate is
  `prompts/plan.md`.
- `compose_magic_nested_path_shaped_resolves`: seed
  `prompts/drafts/plan.md`; run `@prompts/drafts/plan`; assert
  `prompts/drafts/plan.md`.
- `compose_magic_path_shaped_misses_when_dir_absent`: seed
  `prompts/plan.md` but NOT `prompts/drafts`; run `@prompts/drafts/plan`;
  assert empty (the dir join fails).
- `magic_scope_iter_orders_extras_before_user_claudine`: construct a
  synthetic `ScopeSet` with values in every slot and verify
  `iter_magic_scopes()` yields: repo, area, package, repo_claudine,
  extras (in insertion order), user_claudine.

Add to `claudine/cli/tests/completion_compose.rs`:

- `compose_magic_path_shaped_prompts_slash_plan_resolves`: seed
  `prompts/plan.md`; run `compose @prompts/plan`; assert emitted
  candidate is `prompts/plan.md`.
- `compose_magic_path_shaped_claudine_prompts_resolves`: seed
  `.claudine/prompts/plan.md`; run `compose @.claudine/prompts/plan`;
  assert emitted candidate is `.claudine/prompts/plan.md`.

Add to `claudine/cli/tests/completion_inline_compose.rs`:

- `inline_compose_magic_prefers_repo_docs_over_user_global`: seed BOTH
  `docs/plan.md` (with `prompt:` frontmatter) in the fake repo AND
  `prompts/plan.md` (with `prompt:` frontmatter) in the fake `$HOME`
  (`~/.claudine/prompts/plan.md`). Run `inline-compose @plan`. Assert
  the repo-local `docs/plan.md` sorts **before** the
  `~/.claudine/prompts/plan.md` entry. Note: both must appear (the
  iterator emits all matches); the assertion is about relative ordering.

Add to `claudine/cli/tests/completion_sequence.rs`:

- Equivalent `sequence_magic_prefers_repo_docs_over_user_global` using
  `sequence:` frontmatter on both files.

**Validation:**

```bash
cargo test -p claudine-cli composition::tests
cargo test -p claudine-cli scopes::tests
cargo test -p claudine-cli --test completion_compose
cargo test -p claudine-cli --test completion_inline_compose
cargo test -p claudine-cli --test completion_sequence
cargo clippy -p claudine-cli -p claudine-lib --all-targets -- -D warnings
```

Success criterion: the five new unit + integration tests all pass; all
existing tests still pass.

---

## Phase 3 — Repo-Wide Directory Discovery (findings 2, 3)

**Goal:** Surface directory candidates from the **repo / CWD root** (not
just from high-profile scopes) and allow 1–2 character directory matches
via case-insensitive prefix matching.

This is the trickiest phase because it introduces a new scan axis and
interacts with several existing assertions (including one that the review
called out as needing updating). Implement it last so the simpler phases
reduce the surface area of in-flight change.

**Scope:**

- `claudine/cli/src/completion/fuzzy.rs`
- `claudine/cli/src/completion/scopes.rs` (export helpers)
- `claudine/cli/src/completion/composition.rs`
- `claudine/cli/tests/completion_compose.rs`
- `claudine/cli/tests/completion_inline_compose.rs` (light touch)
- `claudine/cli/tests/completion_sequence.rs` (light touch)

**Code changes:**

1. **`PartialLen` directory-matching regime (finding 3).** Add two new
   methods to `PartialLen`:

   ```rust
   /// Whether directories should be surfaced at this prefix length.
   ///
   /// Differs from [`directories_allowed`] — the latter only applied to
   /// high-profile scopes. With the repo-wide directory walk added in
   /// the review-plan, directories are emitted at every non-empty
   /// prefix length.
   pub(crate) fn repo_directories_allowed(self) -> bool {
       matches!(self, Self::Short | Self::Long)
   }

   /// Directory match mode: prefix for short partials, fuzzy for long.
   #[derive(Debug, Clone, Copy, PartialEq, Eq)]
   pub(crate) enum DirMatchMode { Prefix, Fuzzy, None }

   pub(crate) fn dir_match_mode(self) -> DirMatchMode {
       match self {
           Self::Empty => DirMatchMode::None,
           Self::Short => DirMatchMode::Prefix,
           Self::Long  => DirMatchMode::Fuzzy,
       }
   }
   ```

   (Place `DirMatchMode` as a top-level enum in `fuzzy.rs`, public to the
   crate, not nested inside `PartialLen`.)

   Keep `directories_allowed()` alive for the legacy high-profile path —
   the `gather_magic` and `gather_committed` paths should continue to use
   it so those paths' existing contract is preserved. Only the new
   repo-wide directory walk uses `repo_directories_allowed` /
   `dir_match_mode`.

2. **Repo/CWD directory walk (finding 2).** Add a new helper to
   `scopes.rs`:

   ```rust
   /// Single-scope resolver for the repo / CWD directory walk.
   ///
   /// Separate from [`resolve_compose_scopes`] because the repo-wide
   /// directory walk is independent of the high-profile file scope
   /// set (spec §5.3, and review-1 finding 2).
   pub(crate) fn resolve_repo_dir_walk_root(ctx: &ScopeContext) -> Scope {
       let root = ctx
           .repo_info
           .as_ref()
           .map(|info| info.root.clone())
           .or_else(|| ctx.git_root.clone())
           .unwrap_or_else(|| ctx.cwd.clone());
       Scope {
           path: root,
           follow_links: true,
       }
   }
   ```

   In `composition::gather_empty_or_word`, after the existing
   high-profile scope loop, add a second pass that walks the repo-wide
   root and emits **directory-only** candidates (skipping files
   entirely — files continue to come from high-profile scopes):

   - Skip if `partial_len.dir_match_mode() == DirMatchMode::None`.
   - Walk `resolve_repo_dir_walk_root(ctx)` once. The walker already
     honors `.gitignore`, `_`-prefix, and the skip list.
   - For each entry where `is_dir()` is `true`:
     - Compute the entry's path relative to the walk root. Skip the root
       itself (it appears as empty-relative).
     - Skip directories that are already rooted under a high-profile
       scope emitted above (compare canonical paths via the existing
       `seen` set — just extend `seen` to cover both passes).
     - Compute the match target: the final path component
       (`entry.file_name()`). Apply `prefix_match` when
       `DirMatchMode::Prefix`, `fuzzy_match` when `DirMatchMode::Fuzzy`.
     - Render the candidate as the relative path with a trailing `/`.
     - Emit with a dedicated `source_rank` **after** all existing ranks
       so high-profile-rooted candidates still sort first. A concrete
       choice: `source_rank = 10` (any integer larger than the highest
       existing rank). Use a named constant
       `REPO_DIR_WALK_RANK: u8 = 10` in `composition.rs`.

   Care points:

   - The walker yields every directory inside the walk root (including
     deep nesting). The spec says directories are matched by **name**,
     so the target is always the leaf component, even for nested
     entries. That means `src/` and `components/` both match on `co`
     regardless of their depth. This is intentional.
   - Do not emit directories whose **name** starts with `_` (the
     walker already skips them) or that match the skip list (walker
     handles this too).
   - The walker prunes `.git` etc. but does NOT otherwise gate on
     depth. That is fine for monorepos because `.gitignore` does most
     of the pruning and the candidate budget (MAX_CANDIDATES = 500)
     caps runaway walks. If the rusty-biscuit perf harness later
     shows p95 regression from this walk, add a depth cap as a
     follow-up — do not pre-optimize here.
   - A directory that also appears as a high-profile scope leaf (e.g.
     `prompts/` when cwd is at the repo root) would otherwise appear
     twice. Dedup via the canonical-path `seen` set that spans both
     passes.

3. **Update the existing "short prefix matches filenames no dirs" test.**
   In `claudine/cli/tests/completion_compose.rs`, the test
   `compose_short_prefix_matches_filenames_no_dirs` (lines 193–210)
   currently asserts that directories are suppressed for short prefixes.
   The spec actually requires short-prefix directory matches.

   Replace the old assertion with a test that reflects the new contract:

   - Rename to `compose_short_prefix_matches_filenames_and_dirs_with_prefix`.
   - Seed `prompts/plan.md` and a directory `planning/` at the repo root
     (NOT inside `prompts/`). Run `compose pl`.
   - Assert `prompts/plan.md` appears (file match via high-profile scope).
   - Assert `planning/` appears (directory match via repo-wide walk, with
     trailing `/`).
   - Assert that a non-matching directory (e.g. `src/`) does NOT appear
     — the prefix gate must still filter.

   Also delete any other test assertions elsewhere that depend on the
   "short prefix = no dirs" behavior. Grep for
   `short prefix must NOT include directories`, `no directory candidate
   for empty prefix`, and `short prefix` strings across
   `cli/tests/completion_*` to locate them. The `empty` prefix test
   remains valid (Empty → no dirs) — only the Short prefix assertion
   changes.

   Also update the analogous unit test in `composition.rs`
   (`compose_short_prefix_fuzzy_matches_filenames_only`) with the same
   new contract: both a matching file and a matching dir must surface.

**New / updated tests:**

Add to `fuzzy.rs` unit tests:

- `partial_len_repo_directories_allowed_at_short_and_long`.
- `partial_len_dir_match_mode_progression`: Empty → None, Short → Prefix,
  Long → Fuzzy.

Add to `composition.rs` unit tests:

- `compose_one_char_prefix_surfaces_matching_repo_dir`: seed
  `claudine/` dir at repo root; run `c`; assert `claudine/` appears.
- `compose_two_char_prefix_surfaces_matching_repo_dir`: seed
  `docs/` dir; run `do`; assert `docs/` appears.
- `compose_short_prefix_directory_match_is_starting_substring`: seed
  `docs/` and `widgets/` dirs; run `do`; assert `docs/` appears and
  `widgets/` does NOT (prefix-only, not fuzzy, for short prefix).
- `compose_long_prefix_directory_match_is_fuzzy`: seed
  `documentation/` dir; run `dom`; assert `documentation/` appears
  (fuzzy subsequence d-o-m hits d-o-c-u-m — well, d-o-m does not appear
  as a subsequence of `documentation`; use `dcm` or `dtn` to construct
  a genuine fuzzy match). Final match target: pick a query-dir pair
  that proves fuzzy matching fires, e.g.
  `compose_long_prefix_directory_match_is_fuzzy` with dir `foo-bar-baz`
  and query `fbb`.
- `compose_repo_dir_walk_skips_high_profile_roots_once`: seed
  `prompts/plan.md`; run `pro`; assert `prompts/` appears exactly
  once in output (dedup across both passes).

Add to `claudine/cli/tests/completion_compose.rs`:

- `compose_one_char_prefix_surfaces_repo_directories`: seed `docs/` and
  `features/` at the repo root (NOT under `prompts/`); run `compose d`;
  assert both `docs/` and `features/` appear (with trailing `/`).

    Note: In this fixture, `docs/` is NOT a compose scope (compose doesn't
    include `docs/` in extras), so the only way `docs/` can surface is
    via the repo-wide directory walk — which is exactly what we want to
    prove.

- `compose_two_char_prefix_surfaces_repo_directories`: seed `claudine/`
  area and `biscuit-speaks/` area; run `compose cl`; assert `claudine/`
  appears and `biscuit-speaks/` does NOT (starting substring, not
  subsequence).

- `compose_three_char_prefix_fuzzy_matches_directory_names`: seed
  `documentation/` at repo root; run `compose dcm`; assert
  `documentation/` appears (fuzzy dir match at Long).

- `compose_empty_partial_still_does_not_surface_repo_dirs`: regression
  guard that `Empty` remains directory-free.

Add to `completion_inline_compose.rs` / `completion_sequence.rs`: one
each — `inline_compose_one_char_prefix_surfaces_repo_dirs`,
`sequence_one_char_prefix_surfaces_repo_dirs` — so all three modes are
pinned.

**Validation:**

```bash
cargo test -p claudine-cli fuzzy::tests
cargo test -p claudine-cli composition::tests
cargo test -p claudine-cli --test completion_compose
cargo test -p claudine-cli --test completion_inline_compose
cargo test -p claudine-cli --test completion_sequence
cargo clippy -p claudine-cli -p claudine-lib --all-targets -- -D warnings
```

Success criterion: every new test passes; the pre-existing
"short prefix = no dirs" tests have been updated to the new contract
and still pass; no test exists asserting the old (spec-violating)
behavior; zero clippy warnings.

---

## Phase 4 — Documentation Refresh

**Goal:** Fold the new behaviors into the feature's topic doc so the
spec-to-doc chain stays accurate.

**Scope:**

- `claudine/docs/topics/shell-completions.md`

**Code changes:**

1. In the `compose` / `inline-compose` / `sequence` sections, update the
   prefix-progression description:

   - Update "0 characters" to state: no fuzzy matching, no directory
     suggestions.
   - Update "1–2 characters" to state: files fuzzy-matched in
     high-profile scopes; **directories** prefix-matched across the
     repo (or CWD fallback), ignoring high-profile-scope gating.
   - Update "3+ characters" to state: files remain fuzzy-matched in
     high-profile scopes; directories are fuzzy-matched across the
     repo (or CWD fallback).
   - Add a "Why" paragraph: the repo-wide dir scan lets users drill
     into any directory before switching to committed-directory mode;
     restricting it to high-profile scopes would make short partials
     useless for anything except the canonical prompts roots.

2. Under the `@` magic-path section, document two new pieces:

   - **Path-shaped forms.** `@prompts/plan` and `@a/b/c` are supported;
     the portion before the last `/` selects the scope-relative walk
     root, and the portion after is the fuzzy-match segment.
   - **Priority for inline-compose / sequence extras.** Repo-local
     `docs/` and skill peers outrank `~/.claudine/prompts/...` in the
     magic priority order. State the full order:
     repo → area → package → repo `.claudine` → extras (docs, skills)
     → user `.claudine`. Add a "Why" paragraph: project-specific prompts
     should win over global prompts because the user's intent on TAB is
     nearly always "the thing in my current project."

3. In the setter-value section, explicitly state that **only** `.md` /
   `.markdown` (case-insensitive) files are surfaced; `.txt`, `.yaml`,
   and extensionless files are rejected regardless of basename match.
   Why: the setter-value slot is a Markdown-document reference,
   consistent with the compose/inline-compose file contract.

4. In the Performance Optimization section, add a bullet for the
   compose size-rejection uniformity: oversized and unreadable files
   are rejected uniformly across `compose`, `inline-compose`, and
   `sequence`. No mode is permissive on read failures.

**New / updated tests:** None — docs-only phase.

**Validation:**

```bash
# Markdown render / lint if the repo has one; otherwise a visual read.
# (Claudine's repo does not run a markdown linter in CI; manual read is
# the contract here.)
cargo test -p claudine-cli            # regression only (should still pass)
cargo clippy -p claudine-cli -p claudine-lib --all-targets -- -D warnings
```

Success criterion: the four items above are present in the topic doc;
no code regressions.

---

## Phase 5 — Full Sweep + Lint Gate

**Goal:** Final verification that every finding and every new test passes
in aggregate, with zero clippy warnings across the claudine package area.

**Scope:** None (verification only).

**Validation commands:**

```bash
cd claudine
cargo test -p claudine-cli
cargo test -p claudine-lib
cargo clippy -p claudine-cli -p claudine-lib --all-targets --all-features -- -D warnings
cargo fmt -p claudine-cli -p claudine-lib --check
```

Also run the smoke test manually against a real repo:

```bash
# From inside the rusty-biscuit monorepo:
target/debug/claudine __complete --current 1 -- claudine ''
target/debug/claudine __complete --current 2 -- claudine compose 'c'
target/debug/claudine __complete --current 2 -- claudine compose '@prompts/plan'
target/debug/claudine __complete --current 3 -- claudine compose foo.md 'spec=@s'
```

Expected observations:

- Root menu lists composition/wrappers/shared/hooks/admin/`init` per the
  `init`-visibility rule.
- `compose c` surfaces directories under the repo root that start with
  `c` (e.g., `claudine/`), alongside any `prompts/` file matches.
- `compose @prompts/plan` resolves against `rusty-biscuit/prompts/` if a
  matching Markdown file exists there.
- `compose foo.md spec=@s` restricts to `.md` / `.markdown` files under
  `docs/`, `features/`, `fixes/`, `reviews/`.

**Success criterion:** all six review findings demonstrably closed; zero
new clippy warnings; no regressions in existing tests.

---

## Cross-cutting concerns / risks

1. **Review update for test expectations.** The existing
   `compose_short_prefix_matches_filenames_no_dirs` integration test
   encodes a **spec-violating** expectation. Phase 3 mandates its update.
   Do not fix this in Phase 1 or 2 — it will flap with every Phase 3
   change otherwise.

2. **PartialKind::Magic tuple → struct is a breaking shape change**
   (internal only). Every caller of `PartialKind::Magic(..)` must change
   to `PartialKind::Magic { dir, active }` in the same commit. Grep for
   `PartialKind::Magic(` before and after to confirm no stray call-site
   survives.

3. **Repo-wide directory walk budget.** The Phase 3 change introduces a
   second `walker::walk_scope` pass per completion on the repo root. The
   existing `MAX_CANDIDATES = 500` cap applies per call, but the total
   candidate count across both passes is bounded by the dedup set plus
   the cap. Performance is an observable risk on very large repos —
   if the rusty-biscuit smoke test shows noticeable latency, add a
   depth cap or a small max-dir budget in a follow-up; do not attempt
   to pre-optimize here.

4. **Non-UTF-8 test fixture portability.** The non-UTF-8 markdown test
   (Phase 1) writes raw bytes; some CI runners normalize line endings
   on text-mode writes. Use `fs::write(&path, &[0xFF, 0xFE, 0x00, 0x00])`
   (byte slice, not string) to bypass text-mode handling. On Windows
   CI, `fs::write` with a `&[u8]` still writes binary; this works.

5. **Magic-path rendering for `@prompts/plan`**. With the new
   `Magic { dir, active }` shape, the walker receives
   `scope.path.join(dir)` as the walk root. If `dir` includes the scope
   leaf (e.g. `scope.path = /repo/prompts`, `dir = prompts`), the join
   becomes `/repo/prompts/prompts` which will not exist → skip, zero
   candidates. That is the intended behavior (the user already typed
   the scope leaf, so we resolve scope-relative). For a scope whose
   leaf is something other than `prompts` (e.g. extras like `docs/`),
   `@prompts/plan` will also resolve empty for that scope because
   `/repo/docs/prompts` likely does not exist. The repo scope
   (`/repo/prompts`) with empty `dir` and a proper `@plan` query
   remains the only form that covers the `@prompts/plan` user
   intent — verify the new integration test covers this case (it
   does: `compose_magic_path_shaped_prompts_slash_plan_resolves`).

   **Design nuance:** when `dir` starts with the scope leaf, peel it off
   before joining so `scope.path = /repo/prompts` + `dir = "prompts"`
   resolves to `/repo/prompts/`. Implement this as a small helper:

   ```rust
   fn resolve_magic_walk_root(scope_root: &Path, dir: &str) -> Option<PathBuf> {
       if dir.is_empty() {
           return Some(scope_root.to_path_buf());
       }
       // Peel off a matching leading segment so `@prompts/plan`
       // resolves against `<scope>/prompts/` rather than
       // `<scope>/prompts/prompts/`.
       let leaf = scope_root.file_name().and_then(|n| n.to_str())?;
       let trimmed = dir.strip_prefix(leaf).and_then(|rest| rest.strip_prefix('/'));
       let sub = trimmed.unwrap_or(dir);
       Some(scope_root.join(sub))
   }
   ```

   Unit-test this helper on its own:

   - `("/r/prompts", "") → Some("/r/prompts")`
   - `("/r/prompts", "prompts") → Some("/r/prompts")`
   - `("/r/prompts", "prompts/drafts") → Some("/r/prompts/drafts")`
   - `("/r/prompts", "drafts") → Some("/r/prompts/drafts")`
   - `("/r/docs", "prompts/plan") → Some("/r/docs/prompts/plan")` (no
     leaf match; join raw — the expected miss).

6. **Scope of clippy lint gate.** `cargo clippy -p claudine-cli -p
   claudine-lib` is the strict contract. The broader workspace may have
   pre-existing warnings unrelated to these findings; leave those for
   their own cleanup pass. The review specifically demands zero warnings
   in the **claudine** package area, which this narrower invocation
   enforces.

7. **No scope creep.** Every change in this plan traces back to one of
   the six review findings or the listed test-coverage gaps. Do NOT
   take on unrelated refactors, drive-by typo fixes, or new features
   in the same phases — those land separately so this plan's scope
   stays reviewable against the review findings document.

---

## Closure Criteria

- [ ] Phase 1: setter-value extension gate + compose size-rejection
      uniformity shipped with new tests; `cargo test -p claudine-cli`
      and clippy pass.
- [ ] Phase 2: path-shaped magic + magic-scope iterator shipped; spec
      priority order (project-specific before user-global) verified in
      tests.
- [ ] Phase 3: repo-wide directory walk implemented; 1–2 char directory
      prefix matching enabled; existing spec-violating assertion
      updated; new tests cover each new regime.
- [ ] Phase 4: `docs/topics/shell-completions.md` updated for the new
      behaviors and "why" paragraphs.
- [ ] Phase 5: full sweep passes — `cargo test -p claudine-cli`,
      `cargo test -p claudine-lib`,
      `cargo clippy -p claudine-cli -p claudine-lib --all-targets
      --all-features -- -D warnings` all green.
- [ ] Manual smoke against the rusty-biscuit repo confirms the four
      scenarios listed in Phase 5.
