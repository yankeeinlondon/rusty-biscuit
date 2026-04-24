---
phases: 6
created: 2026-04-24
start_phase: 1
source_files_during_phase_1:
  - claudine/cli/src/completion/engine.rs
  - claudine/cli/src/completion/root_menu.rs
  - claudine/cli/src/completion/mod.rs
  - claudine/cli/src/commands/completions.rs
docs_updated_during_phase_1: []
docs_created_during_phase_1: []
skills_files_updated_during_phase_1: []
source_files_during_phase_2:
  - claudine/cli/Cargo.toml
  - claudine/cli/src/completion/mod.rs
  - claudine/cli/src/completion/scopes.rs
  - claudine/cli/src/completion/walker.rs
docs_updated_during_phase_2: []
docs_created_during_phase_2: []
skills_files_updated_during_phase_2: []
source_files_during_phase_3:
  - claudine/cli/src/completion/mod.rs
  - claudine/cli/src/completion/engine.rs
  - claudine/cli/src/completion/scopes.rs
  - claudine/cli/src/completion/frontmatter.rs
  - claudine/cli/src/completion/fuzzy.rs
  - claudine/cli/src/completion/composition.rs
  - claudine/cli/tests/completion_cli.rs
  - claudine/cli/tests/completion_compose.rs
  - claudine/cli/tests/completion_inline_compose.rs
  - claudine/cli/tests/completion_sequence.rs
docs_updated_during_phase_3: []
docs_created_during_phase_3: []
skills_files_updated_during_phase_3: []
packages:
  - claudine-cli
---

# Execution Plan: Improved Shell Completions

## Phase 1 — Scaffolding + Classifier Rewrite

**Goal:** Establish the new completion engine skeleton and root-level menu behavior.

**Steps:**

1. **Create `engine.rs`** — Implement `classify_completion_target()` that dispatches to the correct completer based on argv position and current token shape. Wire into `maybe_complete()` in `mod.rs`.
2. **Create `root_menu.rs`** — Implement the fixed root menu candidate list (composition → wrappers → shared resources → hooks/actions → administration → init) with the `init` visibility rule (`should_offer_init`). Implement `--help` as the sole flag candidate.
3. **Update `mod.rs`** — Re-export the new engine; retain `maybe_complete()` for PowerShell/Elvish legacy path. Update module-level doc comments.
4. **Update `cli/src/commands/completions.rs`** — Route `run_complete` to `engine::run` instead of `supplement::run`. The CLI contract (`--current <INDEX> -- <argv>`) stays unchanged.
5. **Golden tests** — Add test asserting root menu composition for: no config, user-only config, repo-only config, both configs present; `--help` partial variants; global-flag-interspersed argv.

**Validation Checkpoint:**
- `cargo test -p claudine-cli root_menu` passes.
- `claudine __complete --current 1 -- claudine ` emits the curated root menu in spec order.
- `claudine __complete --current 1 -- claudine --h` emits `--help` only.
- Old code paths (`supplement.rs`, `command_factory.rs`) are still compiled but unreachable.

**Parallelizable:** Steps 1 and 2 can be drafted in parallel by different agents; step 3 depends on both.

---

## Phase 2 — Scope Resolution + Walker

**Goal:** Build the directory discovery and filesystem traversal infrastructure.

**Steps:**

1. **Create `scopes.rs`** — Implement `resolve_compose_scopes(ctx, mode)` using `sniff::detect_repo_structure` (called once per `__complete` invocation). Return `ScopeSet` with repo, package-area, package, repo-claudine, user-claudine, and extras (docs/, skills/). Handle `"root"` pseudo-area elision.
2. **Create `walker.rs`** — Build a `.gitignore`-aware walker on `ignore::WalkBuilder` with per-scope `follow_links` flag (false for agent-skill scopes, true otherwise). Implement `MAX_CANDIDATES = 500` budget. Implement skip list (`.git`, `target`, `node_modules`, etc.). Ensure `_`-prefixed files and directories are elided.
3. **Unit tests for `scopes.rs`** — Test scope composition for: cwd at repo root, inside package area, inside discrete package, outside any repo.
4. **Unit tests for `walker.rs`** — Test `.gitignore` honored at every depth, symlinks followed per flag, skip list honored, candidate budget honored.

**Validation Checkpoint:**
- `cargo test -p claudine-cli scopes walker` passes.
- Walker against a fixture repo with `.gitignore` correctly excludes ignored paths.
- Walker against a fixture with symlinks in `.claude/skills/` does not follow them.
- `sniff` is invoked at most once per `__complete` run (verified by mock or tracing).

**Parallelizable:** Steps 1 and 2 are independent; tests (3, 4) depend on their respective modules.

---

## Phase 3 — Composition Completer

**Goal:** Implement the full completion pipeline for `compose`, `inline-compose`, and `sequence`.

**Steps:**

1. **Create `frontmatter.rs`** — Implement `valid_for_mode(path, mode)` using `darkmatter::markdown::Markdown` for `.md` files and `serde_yaml_ng` for `.yaml`/`.yml`. Apply the file-size guard (`MAX_FRONTMATTER_BYTES = 1 MiB`). Case-sensitive key matching (`prompt`, `sequence`).
2. **Create `fuzzy.rs`** — Integrate `fuzzy_matcher::skim::SkimMatcherV2` (or hand-rolled subsequence matcher if dependency is rejected). Implement prefix vs. fuzzy matching rules per prefix length.
3. **Create `composition.rs`** — Implement the shared pipeline parameterized by `ComposeMode`:
   - Classify partial into `Empty`, `ShortPrefix`, `LongPrefix`, `Magic`, `CommittedDir`.
   - Resolve high-profile scopes per mode.
   - Walk scopes with prefix-length-gated behavior (0 chars: files only; 1–2 chars: fuzzy files; 3+: fuzzy files + directories).
   - Filter by mode contract via `frontmatter.rs`.
   - Apply hidden-file + `.gitignore` filter.
   - Dedup + sort by `source_rank`.
   - Render candidates (magic paths resolve to relative paths on selection).
4. **Integration tests** — One test file per mode (`compose`, `inline-compose`, `sequence`) against seeded temp-directory fixtures. Assert stdout lines match expected candidates for: empty prefix, short prefix, long prefix, magic path, committed directory.

**Validation Checkpoint:**
- `cargo test -p claudine-cli completion_compose` passes.
- `claudine compose <TAB>` in a fixture repo shows only `.md` files without `prompt` frontmatter from high-profile scopes.
- `claudine sequence <TAB>` shows `.md`/`.yaml` files with `sequence` frontmatter plus docs/ and skills/.
- `claudine compose @plan<TAB>` resolves `@prompts/plan.md` to relative path.
- Prefix-length progression verified: 0 chars → no dirs; 3+ chars → dirs included.

**Parallelizable:** `frontmatter.rs` and `fuzzy.rs` are independent; `composition.rs` depends on both.

---

## Phase 4 — Setter-Value Completer

**Goal:** Implement `@`-prefixed setter value completion after the file reference.

**Steps:**

1. **Create `setter_value.rs`** — Implement detection of setter pattern `^[A-Za-z_][A-Za-z0-9_-]*=`. Classify value by first non-quote character (`@` triggers file completion, all others → zero candidates). Strip leading quotes for classification; normalize opening `"` to `'` in emitted candidate.
2. **Scope resolution** — Resolve `docs/`, `features/`, `fixes/`, `reviews/` at repo root, package-area, and package levels (reusing `scopes.rs` helpers).
3. **Integration tests** — Test: `@` trigger, quote normalization (`"` → `'` ), scope resolution with/without package-area/package, non-`@` values return empty.

**Validation Checkpoint:**
- `cargo test -p claudine-cli completion_setter` passes.
- `claudine compose foo.md spec=@s<TAB>` emits `spec='docs/...'` with single quotes.
- `claudine compose foo.md spec="@s<TAB>` emits same but with `"` replaced by `'`.
- `claudine compose foo.md spec=bar<TAB>` returns zero candidates.

**Parallelizable:** None — this is a single focused module.

---

## Phase 5 — Delete Legacy + Documentation + Help Defect Fix

**Goal:** Remove old code, fix the help system defect, and rewrite the user-facing documentation.

**Steps:**

1. **Delete legacy files** — Remove `cli/src/completion/supplement.rs`, `command_factory.rs`, `file_reference.rs`, `validate.rs`.
2. **Fix `cli/src/commands/help.rs`** — Add `sequence` to the Composition group (one-line change). Verify description string matches clap doc comment.
3. **Golden regression test** — Assert `help.rs::run()` Composition group contains `compose`, `inline-compose`, `sequence` in that order.
4. **Rewrite `docs/topics/shell-completions.md`** — Cover: overview, installation, root-level menu, composition commands (per-mode), setter values, other commands, performance optimization, legacy shells, examples, architecture diagram. Every non-obvious rule must include a "Why" paragraph.
5. **Update `bootstrap.rs` doc comments** — Reference the new engine path.
6. **Update `CLAUDINE_SKILL.md`** — Link to the rewritten topic file under "deeper topic references."

**Validation Checkpoint:**
- `cargo build -p claudine-cli` succeeds with zero warnings.
- `cargo test -p claudine-cli` passes (all completion + help tests).
- `claudine help` shows `sequence` under Composition.
- `docs/topics/shell-completions.md` renders correctly and covers all spec rules.

**Parallelizable:** Step 2 (help fix) and step 4 (docs) are independent. Step 1 must precede step 6 (build check).

---

## Phase 6 — Performance Profiling + Optional Cache

**Goal:** Validate the sub-100ms target and implement the fallback cache only if needed.

**Steps:**

1. **Performance harness** — Add `cli/tests/completion_perf.rs` (behind `#[ignore]`) that times `__complete` against a fixture mirroring rusty-biscuit scale (~48 packages, ~2000 markdown files). Use `tracing::span` under `RUST_LOG=claudine::completion=trace` with `CLAUDINE_COMPLETION_PROFILE=1`.
2. **Run harness** — Execute on CI reference hardware (or local equivalent). Record p95 latency.
3. **Decision gate**:
   - **If p95 ≤ 100ms:** Mark cache section in `docs/topics/shell-completions.md` as "not currently active; reserved for future activation." Skip to closure.
   - **If p95 > 150ms:** Proceed to step 4.
4. **Implement `cache.rs`** (only if triggered):
   - Cache location: `~/.cache/claudine/completions/<repo-hash>.json`.
   - Payload: `repo_git_head`, `youngest_mtime`, `scanned_at`, candidates.
   - Read path: return cached results immediately; spawn background thread for refresh.
   - Staleness check: compare `repo_git_head` vs. current HEAD; compare directory mtimes vs. `scanned_at`.
   - Atomic write: tempfile + rename.
   - Version gate: schema version mismatch forces full scan.
5. **Cache tests** — Test: stale payload returned immediately; background refresh updates atomically; corrupt payload falls through to sync scan.

**Validation Checkpoint:**
- Performance harness reports p95 ≤ 100ms (or cache is implemented and re-run passes).
- If cache implemented: `cargo test -p claudine-cli cache` passes.
- Final `cargo test -p claudine-cli` is green.

**Parallelizable:** Step 1 (harness) can be drafted during Phase 5. Step 4 (cache) is only needed if profiling triggers it.

---

## Dependency Graph

```
Phase 1 ──► Phase 2 ──► Phase 3 ──► Phase 4 ──► Phase 5 ──► Phase 6
  │           │           │           │           │           │
  └───────────┴───────────┴───────────┴───────────┴───────────┘
                        (all depend on prior phases)
```

Within each phase, independent steps are flagged as parallelizable.

## Risk Mitigation Checkpoints

| Risk | Checkpoint Location | Mitigation Action |
|---|---|---|
| `sniff` shells out to `cargo metadata` on hot path | Phase 2 validation | Verify single invocation per `__complete`; fallback cache ready in Phase 6 |
| Frontmatter parse dominates on large files | Phase 3 validation | Confirm 1 MiB cap is active; extension gate fires before parse |
| Users type `--help` and expect clap output | Phase 1 validation | `--help` is sole candidate; shell inserts it correctly |
| Agent-skill symlinks leak duplicates | Phase 2 validation | Walker test with symlinked `.claude/skills/` confirms no follow |
| `@` magic path inserts unexpected token | Phase 3 validation | Fixture test maps every `@` form to resolved relative path |
| Cache correctness diverges from filesystem | Phase 6 validation (if cache) | Staleness keyed on git HEAD + mtime; background refresh on every read |
| Fallback cache file corruption | Phase 6 validation (if cache) | All reads Result-wrapped; corrupt → sync scan + overwrite |

## Closure Criteria

- [ ] `cargo test -p claudine-cli` passes with zero failures.
- [ ] `cargo build -p claudine-cli` succeeds with zero warnings.
- [ ] `claudine help` shows `sequence` under the Composition group.
- [ ] `docs/topics/shell-completions.md` is rewritten and reviewed.
- [ ] Performance harness reports p95 ≤ 100ms (or cache is implemented and passes).
- [ ] All legacy completion files (`supplement.rs`, `command_factory.rs`, `file_reference.rs`, `validate.rs`) are deleted.
- [ ] `CLAUDINE_SKILL.md` links to the new topic file.
