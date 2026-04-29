---
phases: 6
created: 2026-04-17
start_phase: 4
source_files_during_phase_0: []
docs_updated_during_phase_0: []
docs_created_during_phase_0:
  - claudine/features/2026-04-17-file-completion/phase-0-lock.md
skills_files_updated_during_phase0: []
source_files_during_phase_1:
  - claudine/cli/Cargo.toml
  - claudine/cli/src/main.rs
  - claudine/cli/src/completion/mod.rs
  - claudine/cli/src/completion/command_factory.rs
  - claudine/cli/src/completion/file_reference.rs
  - claudine/cli/src/completion/validate.rs
  - claudine/cli/src/completion/bootstrap.rs
  - claudine/cli/tests/argv_normalization.rs
docs_updated_during_phase_1: []
docs_created_during_phase_1: []
skills_files_updated_during_phase1: []
source_files_during_phase_2:
  - claudine/cli/src/completion/file_reference.rs
  - claudine/cli/src/completion/command_factory.rs
docs_updated_during_phase_2: []
docs_created_during_phase_2: []
skills_files_updated_during_phase2: []
source_files_during_phase_3:
  - claudine/cli/src/completion/validate.rs
  - claudine/cli/src/completion/command_factory.rs
  - claudine/cli/src/completion/file_reference.rs
docs_updated_during_phase_3: []
docs_created_during_phase_3: []
skills_files_updated_during_phase3: []
source_files_during_phase_4:
  - claudine/cli/src/commands/completions.rs
  - claudine/cli/src/completion/bootstrap.rs
  - claudine/cli/tests/command_routing.rs
  - claudine/cli/tests/completion_cli.rs
docs_updated_during_phase_4:
  - claudine/docs/topics/composition.md
docs_created_during_phase_4:
  - claudine/docs/shell-completions.md
skills_files_updated_during_phase4: []
source_files_during_phase_5:
  - claudine/cli/tests/completion_cli.rs
docs_updated_during_phase_5:
  - claudine/docs/shell-completions.md
docs_created_during_phase_5: []
skills_files_updated_during_phase5: []
packages:
  - claudine-cli
---
# Execution Plan — File Completion

Source: [`spec.md`](./spec.md) and [`tech-design.md`](./tech-design.md)

Validated against the current implementation in:

- `claudine/cli/src/main.rs`
- `claudine/cli/src/args.rs`
- `claudine/cli/src/commands/completions.rs`
- `claudine/cli/src/commands/compose.rs`
- `claudine/cli/src/commands/sequence.rs`
- `claudine/cli/Cargo.toml`
- `claudine/cli/tests/command_routing.rs`
- `claudine/lib/src/composition/{mod,resolve,sequence}.rs`
- `sniff/lib/src/filesystem/repo/types.rs`

Current implementation snapshot:

- `claudine completions <shell>` still emits static scripts through `clap_complete::generate(...)`.
- `main.rs` does not yet run an early dynamic completion hook.
- `argv.rs` already treats `COMPLETE` as a no-op guard, which protects the pre-clap normalizer from mutating completion subprocess argv.
- `compose`, `inline-compose`, and `sequence` already share the same variadic `args: Vec<String>` positional, which is the right attachment point for dynamic file completion.

## Phase Index

| Phase | Outcome | Depends on |
|---|---|---|
| 0 | The implementation seam, completion contract, and regression surfaces are locked | none |
| 1 | Dynamic completion is wired into the binary and command graph | 0 |
| 2 | File-reference token classification, scope discovery, and bounded walking work | 1 |
| 3 | Per-command validity filtering matches Claudine's composition rules | 2 |
| 4 | Bootstrap output, docs, and automated completion tests match the new flow | 1, 3 |
| 5 | Latency, failure modes, and package-level acceptance are verified | 0-4 |

## Phase 0 — Lock The Real Seams

1. Confirm the file set that will own the change before editing: add a new `claudine/cli/src/completion/` area, update `main.rs`, `commands/completions.rs`, `commands/mod.rs`, `Cargo.toml`, and add one new CLI integration test file rather than pushing completion logic into `claudine/lib`. Observable result: the write scope is explicit and does not drift into unrelated crates.
2. Reconfirm the current runtime boundaries from code, not only from the design: `main.rs` currently reads argv once, `argv.rs` already has the `COMPLETE` escape hatch, and the three composition commands expose the shared `args` positional that completion must target. Observable result: the plan starts from the actual tree, not a stale mental model.
3. Lock the test homes up front: unit coverage should live under the new completion module, end-to-end subprocess coverage should live in `claudine/cli/tests/completion_cli.rs`, and `command_routing.rs` remains the regression backstop for `claudine completions <shell>`. Observable result: every acceptance criterion has a destination before code moves.
4. Freeze the emitted-value contract for the empty-partial landing menu before implementation: bare suggestions stay bare, repo-scoped suggestions are emitted with `@...`, and package-area suggestions are emitted with `!...`. Observable result: no ambiguity remains about what text the shell will insert for mixed-scope suggestions.

Parallelizable work:

- Steps 0.1 and 0.2 can proceed together.
- Step 0.4 should finish before any completer code is written.

Validation checkpoint:

- `cargo test -p claudine-cli --test command_routing`
- `cargo test -p claudine-cli --test argv_normalization`

## Phase 1 — Add The Dynamic Completion Entry Path

1. Enable `clap_complete`'s `unstable-dynamic` feature in `claudine/cli/Cargo.toml` and scaffold `claudine/cli/src/completion/{mod,command_factory,file_reference,validate,bootstrap}.rs` plus `mod completion;` in `main.rs`. Observable result: the crate can compile against the dynamic completion API and has a dedicated home for the feature.
2. Implement `completion::command_factory::completion_command()` starting from `Cli::command()`, preserve `ignore_errors(true)` for wrapper subcommands just like the current lenient wrapper parse, and attach mode-specific `ArgValueCompleter`s to `compose.args`, `inline-compose.args`, and `sequence.args`. Observable result: only the three composition commands gain dynamic completion behavior and wrapper parsing does not regress.
3. Add `completion::maybe_complete()` and call it immediately after `color_eyre::install()` in `main.rs`, before argv normalization, telemetry initialization, config checks, or normal clap parsing. Observable result: a completion subprocess exits before any normal CLI startup side effects run.
4. Keep the existing `argv.rs` `COMPLETE` guard in place and add a focused regression assertion that completion subprocesses do not exercise the normalizer or wrapper launch path. Observable result: both the early hook and the argv-layer no-op guarantee remain true.

Parallelizable work:

- Steps 1.1 and 1.2 can move in parallel once the module boundaries are created.
- Step 1.4 can be written as soon as `maybe_complete()` is callable.

Validation checkpoint:

- `cargo check -p claudine-cli`
- `cargo test -p claudine-cli --test argv_normalization`

## Phase 2 — Build Token Classification And Candidate Discovery

1. Implement token classification in `completion/file_reference.rs` for `SetterPartial`, `Bare`, `DotRelative`, `DotDotRelative`, `Magic`, `Package`, and `Unsupported`, using the spec's strict setter regex `^[A-Za-z_][A-Za-z0-9_]*=` and returning zero candidates immediately for unsupported prefixes. Observable result: completion decisions are deterministic from the current token alone.
2. Build lightweight repo context with `sniff::filesystem::repo::detect_repo_structure(...)` and `RepoInfo::package_area_for_dir(...)`; derive the package-area root from the returned area name and repo root instead of shelling out or hard-coding package areas. Observable result: `!` completion resolves against the same monorepo model as the rest of the repo tooling.
3. Implement scope-specific candidate discovery:
   bare partial uses the curated landing menu,
   `./` and `../` list immediate filesystem children,
   `@` walks repo root plus `~/.claudine/prompts` and `~/.claudine/sequences`,
   `!` walks only the current package area.
   Observable result: each prefix produces candidates from exactly the documented scope.
4. Add the bounded walker with named constants for max recursion depth, max candidates, max frontmatter bytes, and skip-list behavior; never follow symlinked directories, skip unreadable paths silently, append `/` to directory candidates, and deduplicate by emitted value with deterministic source ranking. Observable result: completion remains bounded and stable even on large or messy trees.
5. Add unit tests for token classification, setter suppression, bare-menu rendering, `@` and `!` emission format, skip-list enforcement, depth and candidate caps, and symlink non-recursion. Observable result: discovery logic is locked before YAML-aware validation is layered on top.

Parallelizable work:

- Steps 2.1 and 2.2 can proceed in parallel.
- Step 2.5 can be written alongside Steps 2.3 and 2.4 once helper signatures are stable.

Validation checkpoint:

- `cargo test -p claudine-cli`

## Phase 3 — Add Mode-Specific File Validation

1. Implement `completion/validate.rs` with one shared markdown-extension helper and one shared size/UTF-8 gate so every validator applies the same `.md` / `.markdown` and max-bytes policy before deeper parsing. Observable result: file-type and file-size filtering are consistent across modes.
2. Implement the `compose` validator as extension-only with no frontmatter parsing. Observable result: `compose` completion mirrors `resolve_composition_source(...)` without adding unnecessary I/O.
3. Implement the `inline-compose` validator by reading the candidate once, constructing `darkmatter::markdown::Markdown`, and requiring a non-empty string `prompt:` frontmatter value after `trim()`. Observable result: files with missing, non-string, empty, or whitespace-only prompts are omitted from completion.
4. Implement the `sequence` validator by constructing a `claudine::composition::ResolvedCompositionSource` from the candidate file and calling `claudine::composition::resolve_sequence_plan(...)`, treating any I/O or parse failure as "not a candidate". Observable result: sequence completion stays aligned with the real sequence parser, including external YAML references.
5. Thread validators into the completers after prefix narrowing so directories always pass through, file parsing only happens on narrowed candidates, and all failures are silent omissions rather than shell-visible errors. Observable result: the completion path remains fail-closed and quiet.
6. Add unit tests for extension filtering, oversized-file skipping, malformed frontmatter omission, empty-prompt rejection, inline-sequence acceptance, and external-sequence acceptance. Observable result: the validator layer matches the spec's acceptance and failure-mode matrix.

Parallelizable work:

- Steps 3.2, 3.3, and 3.4 can be implemented in parallel once the shared helpers in Step 3.1 exist.
- Step 3.6 can be split across compose/inline/sequence validator tests.

Validation checkpoint:

- `cargo test -p claudine-cli`

## Phase 4 — Replace The Install Flow And Update Regression Surfaces

1. Rewrite `claudine/cli/src/commands/completions.rs` to emit dynamic bootstrap snippets for `bash`, `zsh`, `fish`, `powershell`, and `elvish`, and remove the old static `clap_complete::generate(...)` path and zsh post-processing logic. Observable result: `claudine completions <shell>` becomes a one-time bootstrap command instead of a static script generator.
2. Update the command help examples and docs in `claudine/docs/shell-completions.md` and `claudine/docs/topics/composition.md` to explain the new install model, binary-owned updates, command-specific validation, and intentionally unsupported prefixes such as `vault:` and absolute paths. Observable result: user-facing docs match the shipped behavior.
3. Add `claudine/cli/tests/completion_cli.rs` to exercise real completion subprocesses with `COMPLETE=<shell>` and the relevant Clap completion env vars, covering:
   `compose @...`,
   `inline-compose @...`,
   `sequence @...`,
   setter suppression,
   unsupported prefixes,
   and empty-package-area `!` behavior.
   Observable result: the dynamic completion protocol is verified end to end.
4. Update `claudine/cli/tests/command_routing.rs` so it asserts the new bootstrap snippets rather than the old static-script markers. Observable result: routing regressions track the new completion contract, not the retired one.

Parallelizable work:

- Steps 4.2 and 4.3 can proceed in parallel once Step 4.1 compiles.
- Step 4.4 can be updated independently from the new end-to-end tests.

Validation checkpoint:

- `cargo test -p claudine-cli --test completion_cli`
- `cargo test -p claudine-cli --test command_routing`

## Phase 5 — Measure, Harden, And Close The Feature

1. Run the focused CLI test suites first, then the package-level validation path for the area: `cargo test -p claudine-cli`, `cd claudine && just test`, and `cd claudine && just build`. Observable result: the feature passes both narrow and area-preferred validation paths.
2. Manually smoke-test the key acceptance flows on a real repo checkout:
   `claudine compose @pro<TAB>`,
   `claudine compose ./<TAB>`,
   `claudine compose !<TAB>`,
   `claudine inline-compose @<TAB>`,
   `claudine sequence @<TAB>`,
   and `claudine compose topic=<TAB>`.
   Observable result: the interactive shell behavior matches the spec, not just the subprocess fixtures.
3. Explicitly test the failure-mode cases from the spec on representative fixtures:
   malformed frontmatter,
   unreadable files,
   oversized files,
   symlink cycles,
   missing `prompts/` or `sequences/` directories,
   `vault:<TAB>`,
   and `/abs<TAB>`.
   Observable result: the completion path returns cleanly with omissions or zero candidates and never surfaces diagnostics.
4. Measure cold-cache completion latency for at least one repo-root `@` request and one frontmatter-aware request, record the observed numbers in the implementation notes or release notes, and decide whether the initial depth/result/file-size constants need tuning before merge. Observable result: the feature closes with real latency data rather than a placeholder budget.
5. Do not call the feature complete until the binary hook, bounded walker, validators, bootstrap output, docs, automated tests, and manual acceptance pass all agree. Observable result: rollout is gated on the full feature contract, not on partial compilation.

Validation checkpoint:

- `cargo test -p claudine-cli`
- `cd claudine && just test`
- `cd claudine && just build`
