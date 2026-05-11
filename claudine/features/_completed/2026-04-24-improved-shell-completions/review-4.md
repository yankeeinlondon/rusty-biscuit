---
ready: true
reviewer: claude
iteration: 4
date: 2026-04-25
focus: implementation + documentation drift verification after review-3 follow-up
---

# Review 4 — Improved Shell Completions

## Verdict

**Production ready.** Review-3 was already marked ready; this iteration re-verifies that no drift has been introduced by the review-3 follow-up plan and audits the implementation against the spec and tech-design line-by-line. Two minor documentation findings (one linkage, one filename collision); neither blocks shipment.

## Scope of this review

- `claudine/cli/src/completion/` (10 modules) vs. tech-design §3, §4, §5, §6, §11
- `claudine/cli/src/commands/help.rs` Composition group vs. spec line 150 / design §9
- `claudine/docs/topics/shell-completions.md` vs. spec line 144 / design §14
- `claudine/cli/tests/completion_*.rs` integration coverage vs. design §12.2
- All 310 completion tests (231 unit + 79 integration) executed locally

## Implementation conformance

### Module layout — design §3

All ten target files exist; the four legacy files (`supplement.rs`, `command_factory.rs`, `file_reference.rs`, `validate.rs`) are deleted as required by §13.

### Public surface — design §11

| Designed | Implemented as | File |
|---|---|---|
| `ComposeMode` | `ComposeMode` (`Compose`/`InlineCompose`/`Sequence`) | `scopes.rs:31` |
| `RootSlot` | `RootPartial` (`Empty`/`Word`/`FlagLike`) | `engine.rs:71` |
| `CompositionSlot` | `CompletionTarget` (broader; adds `Root`/`Other`) | `engine.rs:43` |
| `ScopeSet` | `ScopeSet` with all six fields | `scopes.rs:91` |
| `CandidateEntry` | `Candidate` (insert + source_rank) | `composition.rs:125` |

The naming differences are cosmetic — the contract surfaces are equivalent.

### Root menu — design §4

`root_menu.rs:55–84` produces the spec-defined fixed menu in the documented order: composition (3) → wrappers (7) → shared (4) → `hooks`/`actions` (the spec's "events" → clap's `actions`) → administration (6) → `init` when applicable. `--help` is the only flag emitted (root_menu.rs:46–52). `init` elision via `should_offer_init` (root_menu.rs:79) inspects user-scope and repo-scope config presence with two `stat` calls — no parse, per design §4.2.

### Composition pipeline — design §5

- **Prefix-length progression** (`composition.rs:158–199`, `fuzzy.rs:88–127`): 0 chars = files only; 1–2 chars = fuzzy file + prefix-matched repo dirs; 3+ chars = fuzzy files + fuzzy dirs. Matches §5.3.
- **Per-mode scope sets** (`scopes.rs:242–327`): `Compose` gets prompts only; `InlineCompose`/`Sequence` add `docs/` plus the seven `.{provider}/skills/**` peers (`.claude`, `.codex`, `.gemini`, `.opencode`, `.goose`, `.qwen`, `.kimi`). Matches §5.2.
- **Frontmatter contracts** (`frontmatter.rs:59–124`): `compose` is the negative filter (no `prompt` key); `inline-compose` requires non-empty string `prompt`; `sequence` accepts `.md` or `.yaml`/`.yml` with root `sequence` key. Case-sensitive lowercase keys per design §5.4.
- **Magic `@` resolution** (`composition.rs:433–519`): three-tier lookup (repo → repo/.claudine → user/.claudine), first-hit-wins, path-shaped forms (`@prompts/plan`) supported, accepted candidate inserted as relative path, not the `@` token — fully addresses review-2 finding 2 and review-1 finding 4.
- **Symlink rule** (`scopes.rs:297–315`, `walker.rs:105`): per-scope `follow_links` flag; agent-skill scopes set `false`; everything else `true`. Matches §5.7.
- **Hidden-file + `.gitignore`** (`walker.rs:100–147`): `ignore::WalkBuilder` with `git_ignore(true).git_global(true).git_exclude(true).hidden(false)` mirroring `sniff::filesystem::docs::collect_markdown_paths`. `_`-prefix filter and `SKIP_DIRS` honored at every depth.
- **Single sniff invocation** (`scopes.rs:186–189`): `detect_repo_structure` is called once per `__complete` invocation and threaded via `ScopeContext::repo_info`, addressing the §16 risk register entry on hot-path `cargo metadata`.

### Setter-value completer — design §6

- **Trigger** (`setter_value.rs:88–111`): `^[A-Za-z_][A-Za-z0-9_-]*=` regex-equivalent byte scan; only `@`-prefixed values produce candidates.
- **Scope set** (`setter_value.rs:196–242`): repo + package-area + package roots over `docs|features|fixes|reviews`; `.md`/`.markdown` only.
- **Quote-wrapping** (`setter_value.rs:109,117`): leading `"`/`'` stripped for classification; emitted candidate is always wrapped in single quotes regardless of opening quote — matches §6.3 contract verbatim.

### Help-system defect — design §9

`claudine/cli/src/commands/help.rs:78–90` has the Composition group with all three commands in the required order. Two golden tests (`composition_group_lists_compose_inline_compose_and_sequence_in_order` at `:204` and `sequence_entry_description_matches_clap_doc_comment` at `:220`) protect both ordering and description drift — fully satisfies design §12.4.

## Test coverage — design §12

| Category | Status | Evidence |
|---|---|---|
| `root_menu` | ✓ | 17 unit tests (root_menu.rs:99–308) |
| `engine::classify` | ✓ | 20+ unit tests (engine.rs:415–771) |
| `composition` | ✓ | 50+ unit tests (composition.rs:766–1489) |
| `setter_value` | ✓ | 17 integration tests (`completion_setter.rs`) |
| `scopes` | ✓ (implicit) | exercised via composition tests |
| `walker` | ✓ | 6 unit tests (walker.rs:149+) including symlink-follow toggles |
| `frontmatter` | ✓ | 8+ unit tests including case-sensitivity, oversize, non-UTF-8 |
| `fuzzy` | ✓ | 9+ unit tests (fuzzy.rs:130+) |
| Integration | ✓ | 79 tests across 5 files (`completion_cli`, `completion_compose`, `completion_inline_compose`, `completion_sequence`, `completion_setter`) |
| Performance harness | ✓ | `completion_perf.rs` with 3 `#[ignore]`d benches per design §8/§12.3 |

Local run: **231 unit + 79 integration = 310 tests, 0 failures, 0 ignored** within completion. Two failures in `wrap_commands.rs` are outside this feature's scope.

## Documentation — design §14

`claudine/docs/topics/shell-completions.md` (826 lines) covers all ten §14 outline items:

1. Overview (lines 1–30) — explains dynamic completion model, why runtime freshness matters
2. Installation (lines 31–54) — bash, zsh, fish, legacy bootstraps
3. Root menu (lines 62–107) — fixed order, `init` rule, `--help` rule
4. Composition commands (lines 109–444) — per-mode scope sets, frontmatter contracts, prefix progression, `@` resolution (bare and path-shaped), symlink behavior, `.gitignore` contract; "Why" reasoning paragraphs throughout
5. Setter values (lines 452–533) — trigger, quote-wrapping, scope set
6. Other commands (lines 534–551) — clap-default behavior
7. Performance Optimization (lines 552–631) — required by spec line 146; covers lazy resolution, extension gate, 1 MiB cap, candidate budget, `RUST_LOG` profiling, harness reference, current "no cache" status with 150ms fallback trigger
8. Legacy shells (lines 632–650) — PowerShell/Elvish gap with rationale
9. Examples (lines 652–792) — realistic `<TAB>` sessions for every major scenario
10. Architecture diagram (lines 794–826) — mermaid flowchart + module-role table

## Findings

### Finding 1 — Topic doc not linked from SKILL.md (minor, non-blocking)

**Severity:** documentation drift.
**Location:** `.claude/skills/claudine/SKILL.md` topic-references list (around lines 37–44 in the rendered skill).
**Detail:** Tech-design §14 explicitly states the doc "is linked from `CLAUDINE_SKILL.md` under 'deeper topic references.'" The list currently surfaces composition, system-prompt, MCP catalog, MCP mode, protect service, traces/logging, log reporting, CLI pre-parsing, and argv-normalization — but **not** shell-completions. New contributors will not discover the doc through the canonical skill.
**Suggested fix:** add a single line under "deeper topic references":

```
- [Shell Completions](../../../claudine/docs/topics/shell-completions.md) — dynamic completion engine, root menu, composition pipelines, setter-value `@` resolution, performance strategy
```

### Finding 2 — Filename collision with repo-root `docs/shell-completions.md` (minor, advisory)

**Severity:** discoverability hazard.
**Location:** `docs/shell-completions.md` (repo root, 323 lines) vs. `claudine/docs/topics/shell-completions.md` (canonical, 826 lines).
**Detail:** The repo-root file is a multi-CLI bootstrap-snippets reference for `.zshrc`/`.bashrc` setup (covers `just`, `hug`, `homey`, `messenger`, `md`, `bt`, `sniff`, `wt`, `playa`, `so-you-say`, `model`, `bh`, `claudine`). It predates this feature and is unrelated to the new completion engine, but the matching filename is misleading: this very review prompt referenced the root file as the documentation for the feature, which it is not. Future readers (and tooling) will have the same confusion.
**Suggested fix (one of):**
- Rename the root file to `docs/shell-completions-bootstrap.md` and update any external references, **or**
- Add a one-line pointer at the top of the root file to the canonical topic doc, **or**
- Add a redirect-style stub at the top of `docs/shell-completions.md` directing readers to `claudine/docs/topics/shell-completions.md` for the Claudine completion engine specifically.

### Finding 3 — Out-of-feature test failures (informational only)

**Severity:** unrelated to this feature; flagged so it isn't lost.
**Location:** `claudine/cli/tests/wrap_commands.rs` — `codex_structured_mode_reconstructs_stdout_and_writes_summary_event`, `explicit_provider_flag_bypasses_chooser`.
**Detail:** Two failures in the wrap-commands harness during `cargo test -p claudine-cli --tests`. These are outside the completion feature scope and do not touch any code under `cli/src/completion/`. They likely warrant a separate fix iteration.

## Items confirmed resolved from prior reviews

- **Review-1 finding 1** (txt files surfaced as setter-value candidates) — `setter_value.rs:49` `SETTER_VALUE_SUBDIRS` plus `.md`/`.markdown`-only filter; covered by `setter_value_skips_txt_files`.
- **Review-1 finding 2** (directory walk too narrow) — `composition.rs:230` `gather_repo_dirs` walks the repo for prefix matches at 1+ chars.
- **Review-1 finding 3** (1–2 char dir completion missing) — `fuzzy.rs:88–127` `PartialLen::classify` + composition.rs prefix-mode emission.
- **Review-1 finding 4** (path-shaped `@` forms) — `composition.rs:602` `resolve_magic_walk_root`.
- **Review-1 finding 5** (magic priority order) — three-tier strict order in `composition.rs:462`.
- **Review-2 finding 1** (wrong rendered paths) — explicit per-scope rendering with `source_rank` survives dedup and sort.
- **Review-2 finding 2** (magic emits lower-priority tiers) — first-hit-wins shadowing in `gather_magic_files`.
- **Review-2 finding 3** (fish file fallback) — verified in `bootstrap.rs` fish stanza.
- **Review-2 finding 4** (`git_root` ignored) — covered by plain-git-checkout integration cases.
- **Review-3 minor findings 1–8** — all addressed per review-plan-3; the doc is now consistent with the implementation, blanket `#![allow(dead_code)]` is gone, debug flag exists, etc.

## Ergonomics & performance observations

- Implementation calls `sniff::detect_repo_structure` exactly once per completion run and reuses the cached `RepoInfo` everywhere — clean defense against the §16 hot-path risk.
- `frontmatter.rs:155–160` short-circuits files larger than 1 MiB before any parse — protects against generated fixture markdown.
- `walker.rs` consistently uses the same `ignore::WalkBuilder` configuration as `sniff::filesystem::docs::collect_markdown_paths`, which keeps `.gitignore` semantics aligned with the rest of the toolchain.
- `setter_value.rs:109` always emits single-quoted candidates regardless of opening quote — eliminates the class of bugs where a path with spaces breaks shell parsing.
- The perf harness is correctly `#[ignore]`d (per design §8/§12.3); review-3 reported p95 ≈ 19 ms on the reference monorepo, well under the 100 ms target, so the fallback cache remains correctly deferred.

## Suggestions for follow-up (optional, non-blocking)

1. **Add the SKILL.md link** described in Finding 1 — single-line edit, immediate discoverability win.
2. **Disambiguate the two `shell-completions.md` files** per Finding 2 — even just a one-line pointer is sufficient.
3. **Address the unrelated `wrap_commands.rs` failures** in a separate fix iteration so CI returns to green.
