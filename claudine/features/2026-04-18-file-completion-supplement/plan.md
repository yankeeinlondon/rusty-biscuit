---
phases: 6
created: 2026-04-18
start_phase: 3
source_files_during_phase_1: []
docs_updated_during_phase_1:
  - claudine/features/2026-04-18-file-completion-supplement/plan.md
docs_created_during_phase_1: []
skills_files_updated_during_phase1: []
source_files_during_phase_2:
  - biscuit-file/lib/src/file_reference/mod.rs
  - biscuit-file/lib/src/file_reference/context.rs
  - biscuit-file/lib/src/file_reference/resolve.rs
  - biscuit-file/lib/src/lib.rs
  - biscuit-file/lib/tests/implicit_relative.rs
docs_updated_during_phase_2: []
docs_created_during_phase_2: []
skills_files_updated_during_phase2: []
packages_during_phase_2: [biscuit-file]
source_files_during_phase_3:
  - sniff/lib/src/filesystem/docs.rs
  - sniff/lib/src/filesystem/mod.rs
docs_updated_during_phase_3: []
docs_created_during_phase_3: []
skills_files_updated_during_phase3: []
packages_during_phase_3: [sniff]
source_files_during_phase_4:
  - claudine/cli/src/args.rs
  - claudine/cli/src/main.rs
  - claudine/cli/src/argv.rs
  - claudine/cli/src/commands/completions.rs
  - claudine/cli/src/completion/mod.rs
  - claudine/cli/src/completion/supplement.rs
  - claudine/cli/src/telemetry.rs
docs_updated_during_phase_4: []
docs_created_during_phase_4: []
skills_files_updated_during_phase4: []
packages_during_phase_4: [claudine-cli]
source_files_during_phase_5:
  - claudine/cli/src/commands/completions.rs
  - claudine/cli/src/completion/bootstrap.rs
  - claudine/cli/tests/completion_cli.rs
  - claudine/cli/tests/command_routing.rs
docs_updated_during_phase_5:
  - claudine/docs/shell-completions.md
  - claudine/docs/topics/composition.md
docs_created_during_phase_5: []
skills_files_updated_during_phase5: []
packages_during_phase_5: [claudine-cli]
source_files_during_phase_6:
  - claudine/features/2026-04-18-file-completion-supplement/plan.md
docs_updated_during_phase_6:
  - claudine/features/2026-04-18-file-completion-supplement/plan.md
docs_created_during_phase_6: []
skills_files_updated_during_phase6: []
packages: [biscuit-file, sniff, claudine-cli]
---

# File Completion Supplement Execution Plan

Source: [`spec.md`](./spec.md)

Validated against the current implementation in:

- `claudine/cli/src/main.rs`
- `claudine/cli/src/args.rs`
- `claudine/cli/src/argv.rs`
- `claudine/cli/src/commands/completions.rs`
- `claudine/cli/src/completion/{mod,command_factory,file_reference,validate,bootstrap}.rs`
- `claudine/cli/tests/{completion_cli,command_routing}.rs`
- `biscuit-file/lib/src/file_reference/{mod,resolve}.rs`
- `sniff/lib/src/filesystem/{docs,repo/types}.rs`

## Current Implementation Snapshot

- Claudine's shipped completion path is still driven by `clap_complete::CompleteEnv` in `main.rs`, not by a hidden `__complete` subcommand.
- The current completer is attached only to `compose`, `inline-compose`, and `sequence` positionals via `ArgValueCompleter`; it does not cover `--append-system-prompt` / `--replace-system-prompt` on those commands or on wrapper subcommands.
- The current discovery logic is the older `2026-04-17-file-completion` model: directory-forward traversal, `!` package-area completion, `./` / `../`, and mode-specific validation for `inline-compose` and `sequence`.
- The supplement spec changes that contract materially: only markdown files at markdown-expecting positions, substring matching on filename, typed-length scoped search, `FileReference`-aligned root expansion, and bash/zsh/fish scripts that shell out to `claudine __complete`.
- `biscuit-file::FileReference` currently exposes parse + resolve APIs only; it does not expose any partial-token completion or root-expansion API.
- `sniff` already has the authoritative `.gitignore`-aware markdown walker, but `collect_markdown_files(...)` is private and currently returns parsed metadata rather than completion-ready path results.

## Outcome

Ship the supplement without destabilizing the already-shipped completion path by:

1. adding the missing `biscuit-file` partial-completion primitive first
2. exposing a `sniff` markdown-walk adapter that reuses the existing ignore policy
3. implementing a new hidden `claudine __complete` path for the supplement contract
4. generating bash/zsh/fish completion scripts that call `__complete` only at the documented argument positions
5. keeping the legacy `COMPLETE=...` path intact for stale installed scripts rather than trying to migrate them implicitly

## Phase Overview

| Phase | Goal | Depends on | Parallelizable |
| --- | --- | --- | --- |
| 1 | Lock the migration seam and freeze the old vs new contract | none | limited |
| 2 | Add the prerequisite partial-token API in `biscuit-file` | 1 | no |
| 3 | Expose the required repo/package/markdown-walk adapters from `sniff` | 1 | yes, partly with 2 |
| 4 | Build the new `claudine __complete` engine around the supplement rules | 2, 3 | no |
| 5 | Replace generated bash/zsh/fish scripts and update docs/tests | 4 | limited |
| 6 | Run acceptance validation and record residual risk | 1-5 | no |

## Phase 1 - Lock The Migration Seams

| Step | Action | Depends on | Parallelizable | Observable completion |
| --- | --- | --- | --- | --- |
| 1.1 | Audit the existing completion surface in `main.rs`, `completion/mod.rs`, `completion/command_factory.rs`, `completion/file_reference.rs`, `completion/validate.rs`, and the current integration tests. | none | yes | A short list exists of the behaviors that must be preserved only for the legacy `COMPLETE` path versus the behaviors that must change for the new supplement path. |
| 1.2 | Freeze the trigger matrix directly from code and spec: `compose`, `inline-compose`, `sequence` positionals plus `--append-system-prompt` / `--asp` and `--replace-system-prompt` / `--rsp` on those commands and on every wrapper subcommand. | none | yes | The implementation has one authoritative list of completion-taking commands and flags, derived from the real clap surface. |
| 1.3 | Lock the compatibility decision up front: keep the current `CompleteEnv` path as a legacy shim for already-installed bootstrap lines, but make all new scripts and all new acceptance tests target `claudine __complete`. | 1.1 | no | There is no ambiguity about whether this feature is a rewrite-in-place or a parallel new path. |
| 1.4 | Freeze the semantic deltas from the 2026-04-17 feature so they do not leak back in during implementation: no directory candidates, no `!` package completion UI, no `./` / `../` traversal UI, and no mode-specific frontmatter validation for `inline-compose` or `sequence` in the new engine. | 1.1, 1.2 | no | The new path is explicitly markdown-only and `FileReference`-aligned, not a patch on the old directory-centric engine. |

Validation checkpoint:

- `cargo test -p claudine-cli --test completion_cli`
- `cargo test -p claudine-cli --test command_routing`

### Phase 1 Outputs — Frozen Contracts

The outputs below are the authoritative hand-off from the audit steps above to Phases 2-5. They are derived from the current clap surface (`claudine/cli/src/args.rs`, `cli/src/commands/wrap/mod.rs`, `cli/src/commands/compose.rs`, `cli/src/commands/wrap/sequence.rs`) and from the current completion modules (`cli/src/completion/{mod,command_factory,file_reference,validate,bootstrap}.rs`).

#### 1.1 Preserve vs change

The current completion surface is the `2026-04-17-file-completion` engine — a directory-centric walker with mode-specific frontmatter validation. It is reached via `clap_complete::CompleteEnv` wired up in `completion/mod.rs::maybe_complete` and invoked from `main.rs::main` before argv normalization. The supplement does not modify this engine in place; it lands a parallel path.

Behaviors to **preserve** only for the legacy `COMPLETE=<shell> claudine …` path (already-installed bootstrap snippets rely on them):

- `CompleteEnv::with_factory(completion_command)` is still invoked from `main.rs` ahead of everything else.
- `completion_command()` still attaches the mode-tagged `ArgValueCompleter` to `compose`, `inline-compose`, `sequence`.
- `completion/file_reference.rs` discovery semantics (bare landing menu, `./`/`../`, `@`, `!`, skip list, depth cap, rank-and-dedup) remain unchanged.
- `completion/validate.rs` `is_valid_for_mode` still gates by `ComposeMode` with `compose` = extension-only, `inline-compose` = non-empty string `prompt:` frontmatter, `sequence` = `resolve_sequence_plan` succeeds with ≥1 step.
- `completion/bootstrap.rs` still emits the existing one-line `source <(COMPLETE=<shell> claudine)` / fish / powershell / elvish snippets. The `completions_emit_dynamic_bootstrap_snippets` test in `tests/command_routing.rs` still passes against the legacy output.
- The existing integration tests in `cli/tests/completion_cli.rs` continue to exercise the `COMPLETE`-prefixed subprocess path; none of them are migrated by Phase 1.

Behaviors that must **change** for the new supplement path (net-new work in Phases 2-5):

- Completion candidates are markdown-file-only at markdown-expecting positions; no directory candidates, no `!` package candidates, no `./`/`../` traversal UI.
- Candidate scope is driven by "meaningful query characters" (`@` sigil excluded, count resets after every `/`); 0-2 chars = curated scope only, 3+ chars = curated scope + `.gitignore`-aware repo walk.
- Matching is case-insensitive substring on filename with `.md` stripped for matching only (not for the inserted candidate).
- Candidate enumeration routes through a yet-to-build `biscuit-file::FileReference` partial-completion API (Phase 2) plus a `sniff` markdown-walk adapter (Phase 3).
- No mode-specific frontmatter validation at completion time for `inline-compose` or `sequence`; all three composition subcommands share the same markdown-only contract.
- The new path is driven by a hidden `claudine __complete` subcommand, not by `ArgValueCompleter` attached to the running clap tree.
- `claudine completions bash|zsh|fish` emits full shell-specific scripts (not one-line bootstraps) that shell out to `claudine __complete` at the documented argument positions.

Behaviors that stay shared across both paths:

- The CLI is still parsed by the two-pass `parse_cli_from` in `main.rs` (strict for non-wrapper subcommands, lenient via cloned `ignore_errors(true)` Command for wrapper subcommands). The new engine must still classify completion context correctly on wrapper argv.
- Pre-clap argv normalization (`argv::normalize`) remains a strict no-op under `COMPLETE` and is itself a no-op for the new `__complete` subcommand in the sense that `__complete` will not be a composition subcommand subject to Rules 1, 3, or 4.

#### 1.2 Trigger matrix (frozen)

One authoritative list of completion-taking argument positions, derived directly from the clap surface. Anything outside this matrix keeps whatever static completion clap derives by default.

Positional `args` (index 0 on each subcommand):

| Subcommand | Source | Flag group |
| --- | --- | --- |
| `compose` | `cli/src/commands/compose.rs::ComposeArgs` | composition |
| `inline-compose` | `cli/src/commands/compose.rs::InlineComposeArgs` | composition |
| `sequence` | `cli/src/commands/wrap/sequence.rs::SequenceArgs` | composition |

File-taking flag values (the long form, the aliases, and the `=`-joined form all trigger completion):

| Flag | Aliases | Subcommands | Source |
| --- | --- | --- | --- |
| `--append-system-prompt <FILE>` | `--asp` | `compose`, `inline-compose`, `sequence`, `claude`, `codex`, `gemini`, `goose`, `kimi`, `opencode`, `qwen` | `commands/compose.rs`, `commands/wrap/mod.rs` (`WrapperArgs`) |
| `--replace-system-prompt <FILE>` | `--rsp` | same 10 subcommands as `--append-system-prompt` | same sources |

Wrapper-subcommand coverage is anchored to `argv::WRAPPER_SUBCOMMANDS = ["claude", "codex", "gemini", "kimi", "qwen", "opencode", "goose"]`. The flag pair is defined directly on `WrapperArgs` (not on a shared SystemPromptArgs struct), so the clap surface across all 7 wrappers is identical for these two flags.

Intentionally **out of scope** for the supplement trigger matrix:

- Any other `<FILE>` / `<PATH>` flag on composition subcommands (e.g. `--output`).
- Any flag on `claudine` root, `claudine handle`, `claudine completions`, `claudine config`, `claudine mcp`, etc.
- Wrapper positional passthrough tokens (they go to the wrapped child CLI).
- PowerShell and elvish shell scripts — the supplement's acceptance matrix is bash/zsh/fish only (see step 5.2).

#### 1.3 Compatibility decision (locked)

- This is **not** a rewrite-in-place. The `CompleteEnv`-driven legacy path and its supporting modules (`completion/command_factory.rs`, `completion/file_reference.rs`, `completion/validate.rs`, `completion/bootstrap.rs`) remain compilable and behaviorally unchanged through Phase 5.
- The new supplement path is a parallel, hidden `Commands::__Complete(...)` subcommand that Phases 4-5 build from scratch. Newly generated bash/zsh/fish scripts shell out to it; stale installed bootstrap lines continue to reach the legacy path.
- No implicit migration is performed for users who have already sourced a `COMPLETE=<shell> claudine` bootstrap. They get the new behavior only after running `claudine completions <shell>` and reinstalling the output (documented in Phase 5.5).
- No deprecation warning is emitted from the legacy path for this feature. If a deprecation/telemetry story is desired later, it is deferred to a follow-up feature (recorded in step 6.3 residual-risk notes).

#### 1.4 Semantic deltas from `2026-04-17-file-completion` (frozen out of the new engine)

The supplement's new engine must never re-introduce the following older-engine concepts. Each item is a behavior from the legacy path that is deliberately absent from the supplement contract.

- No **directory candidates**. The older engine emits directory entries with a trailing `/` so `<TAB>` can descend. The new engine emits only `*.md` file candidates; directory descent is implicit in how `FileReference` enumerates under a path-separator reset, not via directory UI.
- No **`!` package sigil**. The older engine exposes a dedicated `!`-prefixed package-area walker with its own candidate rank. The new engine does not recognize `!`; users only have `@` (magic) and implicit-relative entry forms.
- No **`./` / `../` traversal UI**. The older engine has dedicated classifiers and walkers for `DotRelative` and `DotDotRelative`. The new engine does not special-case these prefixes; whatever `FileReference` does with them is the contract.
- No **mode-specific frontmatter validation**. The older engine's `validate.rs` opens each candidate to confirm `prompt:` (for `inline-compose`) or a resolvable `sequence:` plan (for `sequence`). The new engine treats the three composition subcommands identically — they all offer the same markdown-only candidate set.
- No **`Unsupported` classifier for `vault:` / absolute `/` / `%` / `{{…}}`**. The supplement has exactly two supported entry forms: `@…` magic paths and implicit-relative paths. Anything else either returns an empty result or is not reached because the `FileReference` partial-completion API rejects it.
- No **setter partial (`^[A-Za-z_][A-Za-z0-9_]*=`) suppression**. The supplement trigger matrix only fires on positionals at index 0 and on two named flag values, neither of which ever contain a `key=value` setter. Setter handling stays a shell-level concern.
- No **bounded-walk caps tied to the older engine** (`MAX_RECURSION_DEPTH = 4`, `MAX_CANDIDATES = 500`, `SKIP_DIR_NAMES`). The new engine's broad scan inherits its exclusion policy from `sniff`'s `.gitignore`-aware walker (Phase 3), not from the hand-rolled skip list.
- No **claudine-specific shadow-dir filter** (`is_claudine_shadow`). The supplement defers to `sniff`'s walker for broad-scan exclusion, and the curated-scope enumeration lists fixed directories that never contain a Claudine shadow tree.

If any of these behaviors are found creeping into the Phase 4 engine, treat it as a spec violation and remove it before sign-off.

## Phase 2 - Add The `biscuit-file` Partial Completion Primitive

| Step | Action | Depends on | Parallelizable | Observable completion |
| --- | --- | --- | --- | --- |
| 2.1 | Add a new completion-oriented API to `biscuit-file::FileReference` that accepts a partial token plus an explicit base directory and returns the expansion roots and rendered prefixes implied by `FileReference` parsing. Keep parsing, `@` semantics, repo-root resolution, home-root resolution, and implicit-relative semantics in `biscuit-file`; keep markdown filtering, typed-length decisions, and ranking out of it. | 1 | no | Claudine can ask `biscuit-file` "what roots and emitted prefixes does this partial imply?" without re-implementing grammar or root expansion. |
| 2.2 | Ensure the new API handles the two supplement-supported entry forms only: `@...` magic paths and implicit-relative paths. Unsupported forms may return an empty result or a typed "not completable" response, but the API must not silently reinterpret them. | 2.1 | no | The completion consumer can branch cleanly without second-guessing `FileReference` grammar. |
| 2.3 | Make the API aware of path-separator resets and partial segments so callers can enumerate inside `@prompts/` or `prompts/` without re-parsing the token text manually in Claudine. | 2.1 | no | The returned result identifies the active directory scope and the active segment being matched. |
| 2.4 | Add unit coverage in `biscuit-file` for repo-root + home-root magic expansion, implicit-relative repo-root expansion, no-repo fallback behavior, and `/` segment reset behavior. | 2.1, 2.2, 2.3 | yes | The prerequisite API is locked by tests before Claudine consumes it. |

Validation checkpoint:

- `cargo test -p biscuit-file`

## Phase 3 - Expose The `sniff` Adapters The Supplement Needs

| Step | Action | Depends on | Parallelizable | Observable completion |
| --- | --- | --- | --- | --- |
| 3.1 | Add a small public adapter in `sniff` that reuses the existing `collect_markdown_files(...)` walker configuration but returns completion-friendly path results instead of parsed document metadata. Do not create a second ignore policy. | 1 | yes | Claudine can request "all repo markdown files under the enclosing repo, honoring `.gitignore`" through `sniff`, and the result is path-oriented. |
| 3.2 | Reuse `RepoInfo::package_for_dir(...)` and `RepoInfo::package_area_for_dir(...)` for the curated package/package-area directories first. Only add a lighter-weight `cwd -> package roots` helper if profiling shows `RepoInfo` construction is materially too slow for completion. | 1 | yes | Package-root and package-area-root resolution remain owned by `sniff`, with the API expansion gated on real need rather than anticipation. |
| 3.3 | Confirm that the new adapter works correctly when package-root and package-area-root coincide, and that callers can deduplicate by canonical path without needing extra `sniff` state. | 3.1, 3.2 | yes | Single-crate areas and symlink-equivalent duplicates can be handled deterministically by the consumer. |

Validation checkpoint:

- `cargo test -p sniff filesystem::docs`
- `cargo test -p sniff filesystem::repo`

## Phase 4 - Build The New `claudine __complete` Engine

| Step | Action | Depends on | Parallelizable | Observable completion |
| --- | --- | --- | --- | --- |
| 4.1 | Add a hidden `Commands::__Complete(...)` entry and route it in `main.rs` without config bootstrapping. Its interface should accept the current shell context needed to answer completion requests deterministically: the completion target index and the argv being completed. | 2, 3 | no | `claudine __complete ...` runs as a normal hidden CLI command and returns candidates on stdout with no unrelated startup work. |
| 4.2 | Replace the supplement path's context detection with a full-argv classifier rather than `ArgValueCompleter` attachment. It must identify whether the cursor is on a targeted positional or on a targeted file-taking flag value, including wrapper subcommands that still use `ignore_errors(true)`. | 4.1 | no | The engine knows when to take over and when to return no custom candidates. |
| 4.3 | Rework `completion/file_reference.rs` for the new engine so it consumes the `biscuit-file` API plus `sniff` adapters, computes meaningful-character length, selects curated scope for 0-2 characters, extends to repo-wide scan at 3+, and applies the path-separator reset rules exactly as written in the spec. | 4.1, 4.2 | no | Candidate scope matches the supplement's search budget rules rather than the earlier directory walker. |
| 4.4 | Replace the old mode-specific validation path in the new engine with one markdown-only filter: emit `*.md` candidates only, match case-insensitive substring on filename with `.md` stripped for matching, and leave ordering intentionally unspecified. | 4.3 | no | `compose`, `inline-compose`, `sequence`, `--asp`, and `--rsp` all share one markdown-only completion contract. |
| 4.5 | Deduplicate candidates by canonical resolved path before emission, while preserving the shell-inserted string implied by the `FileReference` form (`@...` vs implicit-relative). | 4.3, 4.4 | yes | Multi-root duplicates appear at most once without rewriting the user's chosen reference form. |
| 4.6 | Keep the legacy `CompleteEnv` modules compiling and behaviorally unchanged for stale installed scripts; do not try to make the old path fully emulate the new spec in this feature. | 1.3, 4.1 | yes | The supplement lands without breaking users who have not regenerated their completion scripts. |

Validation checkpoint:

- `cargo test -p claudine-cli`

## Phase 5 - Replace Generated Shell Scripts And Flip The Test Surface

| Step | Action | Depends on | Parallelizable | Observable completion |
| --- | --- | --- | --- | --- |
| 5.1 | Rewrite `claudine completions bash|zsh|fish` so it generates shell-specific scripts, not one-line bootstrap snippets, and injects a callback that shells out to `claudine __complete` at the documented argument positions only. Use clap-generated static completion for the rest of the command graph instead of re-implementing generic completion behavior. | 4 | no | Freshly generated bash/zsh/fish completions call `claudine __complete` only where the supplement says they should. |
| 5.2 | Keep the current behavior for `powershell` and `elvish` unless a follow-up spec expands the supplement to those shells. The supplement acceptance matrix is bash/zsh/fish only. | 5.1 | yes | Scope is explicit and implementation does not invent unsupported shell behavior. |
| 5.3 | Replace `completion_cli.rs` coverage that currently depends on `COMPLETE=...` with subprocess tests that invoke `claudine __complete` directly and assert the acceptance scenarios from the supplement spec. | 5.1 | yes | End-to-end tests now exercise the real supplement engine rather than the legacy bootstrap path. |
| 5.4 | Update `command_routing.rs` and `commands/completions.rs` assertions to reflect the new generated-script contract for bash/zsh/fish and the retained legacy output for any unchanged shells. | 5.1, 5.2 | yes | Routing tests pin the new install surface without conflating it with the old bootstrap one-liners. |
| 5.5 | Update user-facing docs in `claudine/docs/shell-completions.md` and `claudine/docs/topics/composition.md` to describe the new install/regeneration model, the supported shells, and the fact that stale installed scripts remain on the old path until reinstalled. | 5.1, 5.2 | yes | Docs match the shipped completion surface and the spec's non-goal around retroactive upgrades. |

Validation checkpoint:

- `cargo test -p claudine-cli --test completion_cli`
- `cargo test -p claudine-cli --test command_routing`

## Phase 6 - Acceptance Validation And Closeout

| Step | Action | Depends on | Parallelizable | Observable completion |
| --- | --- | --- | --- | --- |
| 6.1 | Run the supplement acceptance matrix against the hidden command and generated bash/zsh/fish scripts: empty input, 2-char `@` match, 3-char broad scan, path reset, implicit-relative path, wrapper `--asp` / `--rsp`, no-repo fallback, non-markdown exclusion, gitignored broad-scan exclusion, mid-filename substring match, and multi-crate dedup. | 1-5 | no | Every numbered acceptance criterion in `spec.md` has a direct automated assertion or an explicitly recorded manual shell check. |
| 6.2 | Manually smoke-test bash, zsh, and fish with regenerated scripts in a real repo checkout to confirm the callback wiring is shell-correct, not only Rust-correct. | 6.1 | no | The generated scripts invoke `claudine __complete` successfully in all three supported shells. |
| 6.3 | Record residual open items explicitly rather than silently resolving them: ordering, caching, performance budget, `FileReference` API shape refinements, optional `sniff` lightweight helpers, HOME-unset behavior, curated-scope symlink policy, and observability. | 6.1, 6.2 | no | The feature closes with the implemented contract and a clean list of deferred design questions. |

Validation checkpoint:

- `cargo test -p biscuit-file`
- `cargo test -p sniff`
- `cargo test -p claudine-cli`
- `cd claudine && just test`

## Key Implementation Notes

- The supplement is not a small tweak to the current `ArgValueCompleter` flow. The main confidence gain in this plan is treating `__complete` as a parallel path, with `biscuit-file` and `sniff` prerequisites landed first.
- The new engine must deliberately remove the older per-command validation behavior. The supplement's contract is "markdown-expecting positions complete markdown files", not "inline-compose and sequence pre-validate frontmatter."
- Repo root detection for the supplement should come from `FileReference`-aligned logic, not from the current workspace-only `sniff::detect_repo_structure(...)` shortcut used by the older completer.
- The broad scan must reuse `sniff`'s ignore-aware markdown walk. Curated-scope enumeration must not turn into a second ad hoc recursive walker with a separate exclusion policy.
- Ordering remains unspecified. The plan should not invent sorting/ranking work beyond stable dedup unless a later decision is made explicitly.
