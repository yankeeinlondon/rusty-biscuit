---
ready: false
agent: codex
model: ""
---

# Review: Prose Cross-Target Rendering

## Findings

### Critical: Core cross-target Prose implementation is absent

The spec requires `Prose` to implement both `BrowserRenderable` and `MarkdownRenderable` (FR-1/FR-2; acceptance criteria in `renderable/features/2026-05-17-prose-cross-target/spec.md:300-301` and `:347`). The implementation still only has `TerminalRenderable for Prose` in `biscuit-terminal/lib/src/components/prose/render.rs:11`; searches found no `BrowserRenderable for Prose`, no `MarkdownRenderable for Prose`, and no `ProseDocument` / `ProseNode` IR implementation.

Impact: the feature cannot render Prose to Browser, Markdown, or MarkdownPlus, so FR-1, FR-2, FR-4, FR-5, FR-6, and the Browser/Markdown acceptance tests are unmet.

Verification level present: none for Browser/Markdown Prose behavior. Required: target tests for Browser/Markdown output, plus structural escaping assertions.

### Critical: Atomic-token grammar is still live

The spec explicitly requires removing the atomic grammar and `atomic_token_*` tables (`spec.md:285-293`, acceptance at `:362-363`). The parser still recognizes `{{token}}` in `biscuit-terminal/lib/src/components/prose/tokens.rs:110-160`, the atomic lookup helpers still exist in `biscuit-terminal/lib/src/components/prose/styles.rs:117-178`, and the Prose unit tests still assert atomic styling behavior in `biscuit-terminal/lib/src/components/prose/mod.rs:31-87`.

Impact: former atomic syntax does not become ordinary text, so FR-7 is violated for known atomic tokens. This also keeps the overlapping-range grammar that the spec is designed to eliminate.

Verification level present: existing Level 1/unit coverage proves the old behavior still works, which is the opposite of the new requirement. Required: parser/unit tests asserting `{{bold}}`, `{{reset}}`, and other former known tokens parse/render as literal text.

### High: Call-site migration is incomplete

The migration section scopes live call sites to Claudine hook/action UI code (`spec.md:276-279`). The hook views were partly migrated, but `claudine/cli/src/commands/actions.rs:60` and `claudine/cli/src/commands/actions.rs:180` still contain `{{dim}}`, `{{blue}}`, and `{{reset}}` in user-facing output. There are also remaining non-test examples/docs under `biscuit-terminal` that still present atomic syntax, including the `bt prose` no-content hint at `biscuit-terminal/cli/src/commands/prose.rs:34`.

Impact: once the grammar is removed, these user-facing paths will display raw style tokens. Until they are migrated, the spec's required sequencing is not complete.

Verification level present: Level 0/ordinary integration coverage only for `claudine hooks` output in `claudine/cli/tests/hooks_cli.rs`; no test covers `claudine actions` after token removal. Required: at least Level 1 CLI tests for migrated action output and a repository-wide negative token check scoped to live code/docs.

### High: User-visible terminal styling is not verified at the required level

The new hook tests intentionally run the real binary with `NO_COLOR=1` and assert plain stdout (`claudine/cli/tests/hooks_cli.rs:1-6`, `:32-41`). That catches literal token leaks, but it cannot verify that nested bracketed tags preserve dim re-entry, foreground colors, strikethrough, emoji widths, table alignment, or real terminal rendering after the migration.

Impact: the migration changes terminal styling syntax in dense tables and legends, but the strongest new tests do not exercise ANSI styling or a real terminal emulator. Per the requested rigor model, styling/color/width requirements need Level 2 capture for production readiness.

Verification level present: below Level 1 for styling because color is disabled and no PTY/terminal capture is used. Required: Level 2 capture for representative `claudine hooks --support`, `--variables`, `--capture-method`, and an invalid-effect table case; Level 1 ANSI assertions are also useful for local regressions.

### Medium: `bt prose` help/docs still advertise removed syntax

The spec requires updating `bt prose` help/examples and `prose.md` docs (`spec.md:289`). The CLI still reports `bt prose "Hello {{bold}}world{{reset}}!"` at `biscuit-terminal/cli/src/commands/prose.rs:34`, and the docs still describe atomic tokens as a supported grammar in `biscuit-terminal/docs/components/prose.md:6-20`.

Impact: users will keep authoring syntax that is intended to become literal text.

## Test Coverage Assessment

- Parser IR tests required by `spec.md:321-328`: not present, because the IR is not implemented.
- Browser/Markdown target tests required by `spec.md:330-335` and `:349-361`: not present for Prose.
- Terminal parity required by `spec.md:337-340`: old terminal tests still exist, but there is no old-vs-new IR-backed parity path because the new path is absent.
- Claudine hook migration tests: present and passing, but they are plain stdout assertions with `NO_COLOR=1`, not Level 1/2 terminal behavior verification.

## Verification Performed

- `cargo test --color=never -p claudine-cli --test hooks_cli` passed: 4 tests, 0 failed.
- `cargo check --color=never -p claudine-cli` was started but terminated after it exceeded the non-interactive review window; no result should be inferred from that command.

## Production Readiness

Not ready. The current changes are a useful first slice of the prerequisite call-site migration, but the primary feature described by the spec is not implemented yet, atomic parsing remains active, live call sites still use atomic tokens, and the terminal-rendering verification level is below the stated bar.
