---
phases: 5
---
# Execution Plan — `--edit` Wrapper Support

## Phase 0 — Confirm Scope and Test Surfaces

1. Reconfirm the implementation targets named in the spec:
   `darkmatter/lib/src/lib.rs`, `darkmatter/cli/src/commands.rs`,
   `claudine/cli/src/commands/wrap/mod.rs`, and existing prompt parsing in
   `claudine/cli/src/commands/wrap/profile.rs`.
   Observable result: the change list maps cleanly onto existing code seams and no extra crates are needed.
2. Decide the test homes before editing:
   `darkmatter/lib/src/editor/` unit tests for reusable editor logic,
   existing `darkmatter/cli/src/commands.rs` tests for `md edit` regression coverage,
   `claudine/cli/src/commands/wrap/mod.rs` unit tests for passthrough extraction,
   and `claudine/cli/tests/wrap_commands.rs` for end-to-end CLI behavior.
   Observable result: every spec scenario has a destination before implementation starts.

Validation checkpoint:
`cargo check -p darkmatter -p darkmatter-cli -p claudine-cli`

## Phase 1 — Extract Reusable Editor Logic Into `darkmatter::editor`

1. Add `pub mod editor;` to `darkmatter/lib/src/lib.rs`.
   Observable result: `darkmatter::editor` becomes importable from other crates.
2. Create `darkmatter/lib/src/editor/mod.rs` with:
   `DEFAULT_EDITOR_PRIORITY`, `resolve_editor_command()`,
   `wait_args_for_editor()`, `launch_editor_on_path()`, `edit_text()`, and
   `EditorError`.
   Observable result: Darkmatter library exposes the full reusable editor API described in the spec.
3. Implement `edit_text()` with `tempfile::Builder::new().suffix(".md")`,
   seed write, flush/sync, blocking editor launch, UTF-8 reread, `trim_end()`,
   `Ok(None)` for empty buffers, and `EditorError::Missing` when the temp file disappears.
   Observable result: the reusable temp-file prompt workflow exists in one place.
4. Move editor-resolution and wait-flag policy out of `darkmatter-cli` and refactor
   `darkmatter/cli/src/commands.rs::run_edit()` to delegate to
   `darkmatter::editor::launch_editor_on_path()` while preserving existing file-path
   resolution and success output.
   Observable result: `md edit` behavior is unchanged, but the policy no longer lives only in the CLI crate.

Parallelizable work:
- Steps 1.2 and 1.3 are the critical path.
- Step 1.4 can start once the public API signatures are stable.

Validation checkpoint:
- `cargo test -p darkmatter`
- `cargo test -p darkmatter-cli`

## Phase 2 — Add `--edit` Flag Plumbing To Claudine Wrappers

1. Add `edit: bool` to `WrapperArgs` in `claudine/cli/src/commands/wrap/mod.rs`
   with `#[arg(long, conflicts_with = "interactive")]`.
   Observable result: every wrapper subcommand accepts `--edit`, and clap rejects `--edit` with `--interactive`.
2. Extend `ExtractedWrapperFlags` with `edit: bool`.
   Observable result: the wrapper pipeline can detect `--edit` whether clap parsed it directly or it arrived in passthrough.
3. Teach `extract_wrapper_flags_from_passthrough_with_boundary()` to consume pre-boundary `--edit` and leave post-`--` `--edit` untouched.
   Observable result: Claudine owns pre-`--` `--edit`, and provider passthrough keeps the post-`--` escape hatch.
4. OR-merge `args.edit || extracted.edit` inside `run_provider_wrapper_inner()`.
   Observable result: a single `edit_requested` branch controls downstream behavior.

Parallelizable work:
- Steps 2.1 and 2.2 can be prepared together.
- Step 2.3 depends on the updated extracted-flags struct.

Validation checkpoint:
- `cargo test -p claudine-cli extract_wrapper_flags`
- `cargo test -p claudine-cli --test wrap_commands`

## Phase 3 — Insert Editor Flow Into The Wrap Pipeline

1. In `run_provider_wrapper_inner()`, insert the `--edit` flow immediately after
   `extract_prompt_source_from_passthrough()` and before `has_prompt` is computed.
   Observable result: editing happens before any downstream session-mode decisions.
2. Add the TTY preflight using `std::io::IsTerminal` for both stdin and stdout, and
   reject `PromptSource::InheritStdin` defensively with `--edit requires an interactive terminal`.
   Observable result: non-interactive sessions fail before launching an editor.
3. Seed the editor from `PromptSource::Inline(_)` or an empty string, print the
   pre-session status line on stderr unless `--silent`, call
   `darkmatter::editor::edit_text(&seed, ".md")`, and replace the prompt source with
   `PromptSource::Inline(edited_text)` on success.
   Observable result: edited text becomes the effective prompt without changing the rest of the pipeline.
4. Treat `Ok(None)` from `edit_text()` as a clean abort: print `prompt empty; aborted`
   unless `--silent`, return `Ok(())`, and do not invoke the provider.
   Observable result: empty buffers exit `0` and stop before preview/provider execution.
5. Preserve existing semantics for `--dry-run`, `--quiet`, system prompt flags,
   MCP/tag extraction, and provider timeout behavior.
   Observable result: only prompt acquisition changes; the rest of the wrap pipeline stays stable.

Dependency note:
- Phase 3 depends on Phase 1 for `darkmatter::editor`.
- Phase 3 depends on Phase 2 for `edit_requested` and clap conflict behavior.

Validation checkpoint:
- `cargo test -p claudine-cli --test wrap_commands`
- `cargo test -p claudine-cli`

## Phase 4 — Help Text, Completions, and Regression Coverage

1. Update `print_wrapper_help()` to list `--edit` in the wrapper options output.
   Observable result: manual help and generated clap help both mention the flag.
2. Add or update `darkmatter` unit tests for editor resolution priority, wait-flag lookup,
   edit success, empty-buffer abort, and non-zero editor exit.
   Observable result: the reusable library behavior is locked down independent of Claudine.
3. Add or update `claudine` tests for:
   non-TTY rejection, `--edit`/`--interactive` conflict, seeded edits,
   empty-buffer clean abort, `--dry-run` preview using edited text, and post-`--` passthrough.
   Observable result: each spec scenario has an automated assertion.
4. Regenerate or verify completion output via the package recipe once the clap surface is final.
   Observable result: shell completion output reflects the new `--edit` flag.

Parallelizable work:
- Darkmatter library tests and Claudine CLI tests can proceed in parallel once Phase 3 code compiles.
- Help/completion verification can happen after the clap-facing changes in Phases 2 and 3 land.

Validation checkpoint:
- `cd darkmatter && just test`
- `cd claudine && just test`
- `cd claudine && just build`

## Phase 5 — Final Acceptance Pass

1. Run focused checks first, then the package-level recipes:
   `cargo test -p darkmatter -p darkmatter-cli -p claudine-cli`,
   followed by `cd darkmatter && just test` and `cd claudine && just test`.
   Observable result: both the narrow and package-preferred validation paths pass.
2. Manually smoke-test at least these flows with a harmless editor command or temp script:
   `claudine claude --edit --dry-run`,
   `claudine claude "seed text" --edit --dry-run`,
   `claudine claude -- --edit`.
   Observable result: editor launch, seed carry-through, clean abort, and provider passthrough all match the spec.
3. Review stderr ordering to confirm the editor status line appears before the execution header and that `--silent` suppresses it.
   Observable result: output ordering matches the documented CLI contract.

Release gate:
- Do not consider the feature complete until the reusable Darkmatter API, Claudine wrapper integration, automated tests, and completion/help output all pass together.
