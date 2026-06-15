---
agent: codex/
phases: 6
created: 2026-06-15
start_phase: 1
yolo: true
source_files_during_phase_1: []
docs_updated_during_phase_1:
- darkmatter/features/2026-06-15-grammar/plan.md
docs_created_during_phase_1: []
skills_files_updated_during_phase_1: []
source_files_during_phase_2:
- darkmatter/lib/src/markdown/language_grammar.rs
- darkmatter/lib/src/markdown/code_block.rs
- darkmatter/cli/src/args.rs
docs_updated_during_phase_2:
- darkmatter/features/2026-06-15-grammar/plan.md
- darkmatter/docs/topics/yamlblock-migration.md
docs_created_during_phase_2: []
skills_files_updated_during_phase_2:
- .claude/skills/darkmatter/SKILL.md
source_files_during_phase_3:
- darkmatter/lib/src/markdown/code_block.rs
- darkmatter/lib/src/markdown/highlighting/mod.rs
- darkmatter/lib/src/markdown/language_grammar.rs
- darkmatter/lib/src/markdown/output/code_block.rs
- darkmatter/lib/src/markdown/output/terminal.rs
- darkmatter/lib/src/markdown/render_tree/code_renderer.rs
- darkmatter/cli/src/commands/schema/about.rs
docs_updated_during_phase_3:
- darkmatter/features/2026-06-15-grammar/plan.md
docs_created_during_phase_3: []
skills_files_updated_during_phase_3: []
source_files_during_phase_4:
- darkmatter/lib/src/markdown/compose/transclusion/code.rs
docs_updated_during_phase_4:
- darkmatter/features/2026-06-15-grammar/plan.md
docs_created_during_phase_4: []
skills_files_updated_during_phase_4: []
source_files_during_phase_5:
- darkmatter/lib/src/markdown/language_grammar.rs
- darkmatter/lib/src/markdown/output/code_block.rs
docs_updated_during_phase_5:
- darkmatter/features/2026-06-15-grammar/plan.md
- darkmatter/features/2026-06-14-summary-and-suggest/plan.md
docs_created_during_phase_5: []
skills_files_updated_during_phase_5:
- .claude/skills/darkmatter/SKILL.md
source_code:
- darkmatter/lib/src/markdown/language_grammar.rs
- darkmatter/lib/src/markdown/code_block.rs
- darkmatter/cli/src/args.rs
- darkmatter/lib/src/markdown/highlighting/mod.rs
- darkmatter/lib/src/markdown/output/code_block.rs
- darkmatter/cli/src/commands/schema/about.rs
- darkmatter/lib/src/markdown/compose/transclusion/code.rs
documentation:
- darkmatter/features/2026-06-15-grammar/plan.md
- darkmatter/docs/topics/yamlblock-migration.md
- darkmatter/features/2026-06-14-summary-and-suggest/plan.md
packages:
- darkmatter
hash: 31f02fde747ff89b-953097b15adb7537
last_updated: 2026-06-15
---

# Language Grammar Resolution Plan

## Success Criteria

- `LanguageGrammar` is the only production grammar lookup path in Darkmatter.
- Public construction covers fallible, lossy, token-only, extension, name, and filename inputs.
- Code-block rendering, YAML highlighting, and code transclusion route syntax resolution through `LanguageGrammar`.
- Direct production `SyntaxSet::find_syntax_by_*` calls remain only inside `language_grammar.rs`.
- Documentation, rustdoc, and the darkmatter skill describe the single-authority rule.

## Phase 1: Baseline And Scope Confirmation

- [x] Confirm the working tree state with `GIT_TERMINAL_PROMPT=0 git status --short` and record unrelated dirty files before editing.
- [x] Reconfirm relevant workspace packages with `cargo metadata --no-deps --format-version 1`, verifying the target crates are `darkmatter` and `darkmatter-cli`.
- [x] Inspect `darkmatter/lib/src/markdown/language_grammar.rs`, `darkmatter/lib/src/markdown/code_block.rs`, `darkmatter/lib/src/markdown/output/code_block.rs`, `darkmatter/lib/src/markdown/highlighting/mod.rs`, and `darkmatter/lib/src/markdown/compose/transclusion/code.rs` to map current call sites and tests.
- [x] Search for direct grammar lookups with `rg -n "find_syntax_by_|from_fence_token|SyntaxSet::load_defaults_newlines|find_syntax\\(" darkmatter -S` and classify each result as production, test-only, docs, or historical feature text.
- [x] Identify docs that mention old behavior, including `darkmatter/docs/topics/yamlblock-migration.md`, `darkmatter/cli/src/args.rs`, and `.claude/skills/darkmatter/SKILL.md`.

### Checkpoint 1

- [x] Produce a short implementation note listing every production migration target and every intentionally allowed test-only direct syntect lookup.

**Implementation note — Phase 1 baseline**

Production migration targets (direct `SyntaxSet::find_syntax_by_*` or `SyntaxSet::load_defaults_newlines` outside `language_grammar.rs`, plus `from_fence_token` call sites):

1. `darkmatter/lib/src/markdown/output/code_block.rs`
   - `find_syntax` helper (lines 330–372) does extension/name/case/alias lookup directly.
   - Called by `render_terminal_code_block` (line 57) and `render_html_code_block` (line 230).
   - Plan: delete helper; make render helpers accept `&LanguageGrammar` and call `LanguageGrammar::resolve`.

2. `darkmatter/lib/src/markdown/highlighting/mod.rs`
   - `highlight_yaml_lines_with_theme` directly calls `find_syntax_by_extension("yaml")` / `find_syntax_by_name("YAML")` (lines 158–162).
   - Plan: resolve via `LanguageGrammar::yaml().resolve(...)`.

3. `darkmatter/lib/src/markdown/compose/transclusion/code.rs`
   - Private `lazy_static! SYNTAX_SET = SyntaxSet::load_defaults_newlines()` (line 8).
   - `infer_language` calls `SYNTAX_SET.find_syntax_by_extension(ext)` (line 17).
   - Plan: remove `SYNTAX_SET`; validate with `LanguageGrammar::from_filename(path)`.

4. `darkmatter/cli/src/commands/schema/about.rs`
   - `one_half_yaml_color` calls `find_syntax_by_extension("yaml")` (lines 519–522).
   - Plan: route through `LanguageGrammar::yaml().resolve(...)`.

5. `darkmatter/lib/src/markdown/code_block.rs`
   - `with_fence_language` (line 118), typed constructors `yaml/rust/json/toml` (lines 147, 152, 157, 162), `from_source_file` (line 182), and `from_fence_parts` (line 201) all call `LanguageGrammar::from_fence_token`.
   - Plan: replace with new `LanguageGrammar` constructors (`from_token`, `from_extension`, `from_name`, `from_filename`, `from_lossy`, `from_token_or_plain_text`, and infallible `yaml/rust/json/toml/markdown`).

Intentionally allowed test-only direct syntect lookups (all inside `#[cfg(test)]` or test files):

1. `darkmatter/lib/src/markdown/output/code_block.rs` test module — `find_syntax("rust", ...)` and `find_syntax("unknown_language_xyz", ...)` (lines 623, 634). These validate the helper that will be deleted; equivalent coverage should move into `language_grammar.rs`.
2. `darkmatter/lib/src/markdown/output/terminal.rs` test module — multiple `find_syntax(...)` calls (lines 1653–1739). Coverage of alias/name/extension resolution; to be moved/rewritten into `language_grammar.rs`.
3. `darkmatter/lib/src/markdown/highlighting/grammars.rs` — doc example and syntax-set loading tests (lines 30, 50, 58, 65, 73, 80). These test the grammar loader itself, not application lookup logic.
4. `darkmatter/lib/src/markdown/highlighting/mod.rs` test module — `find_syntax_by_extension("rs")` (line 223). Smoke test that the syntax set is populated.
5. `darkmatter/cli/tests/level2_schema_about.rs` — `find_syntax_by_extension("yaml")` (line 120). Test-only schema-about assertion.
6. `darkmatter/lib/src/markdown/render_tree/entrypoints.rs` test module — `find_syntax_by_extension("yaml")` (line 818). Test helper.
7. `darkmatter/lib/src/markdown/render_tree/code_renderer.rs` test module — `find_syntax_by_extension("yaml")` (line 426). Test helper.

Docs/historical references to update (Phase 5):

- `darkmatter/cli/src/args.rs` doc comment references `LanguageGrammar::from_fence_token`.
- `darkmatter/docs/topics/yamlblock-migration.md` references `LanguageGrammar::from_fence_token`.
- `.claude/skills/darkmatter/SKILL.md` needs the single-authority rule added.
- Historical feature specs under `darkmatter/features/_completed/` mention `from_fence_token` intentionally; leave as historical record.

## Phase 2: Build The Authoritative `LanguageGrammar` API

- [x] Add or update the `PlainText` variant so `LanguageGrammar::PlainText` represents the fallback grammar and `Display` returns an empty string for it.
- [x] Implement `LanguageGrammar::plain_text()` and `LanguageGrammar::text()` as infallible convenience constructors that return `PlainText`.
- [x] Implement named infallible constructors `yaml()`, `rust()`, `markdown()`, `json()`, and `toml()` without runtime lookup.
- [x] Replace `from_fence_token` with `from_token`, `from_token_or_plain_text`, `from_lossy`, `from_extension`, `from_name`, and `from_filename`.
- [x] Implement `TryFrom<&str>`, `TryFrom<String>`, and `FromStr` for `LanguageGrammar`; do not implement `From<&str>` or `From<String>`.
- [x] Add token normalization that trims input, reads quoted tokens through the matching closing quote, and ignores unquoted ASCII-whitespace metadata after the first token.
- [x] Add filename/path detection for the full fallible and `from_lossy` paths, skipping filename detection when unquoted whitespace indicates Markdown fence metadata.
- [x] Centralize aliases and resolution order in one private resolver: explicit aliases, extension lookup, exact name lookup, case-insensitive name lookup, token validation, then `UnknownGrammar`.
- [x] Preserve explicit plain-text tokens such as `txt`, `text`, `plain`, `plaintext`, and `plain-text` as token-preserving dynamic variants that resolve to syntect Plain Text.
- [x] Add `resolve_default()` using Darkmatter's shared two-face syntax set and keep `resolve(&self, &SyntaxSet)` for custom syntax sets.
- [x] Update `LanguageGrammarError::UnknownGrammar` usage so empty normalized fallible input returns `UnknownGrammar(String::new())`.

### Parallelizable Work

- [x] In parallel after the API shape is settled, one implementer can add parsing and alias tests while another updates rustdoc examples in `language_grammar.rs`.

### Checkpoint 2

- [x] Run `cargo test -p darkmatter language_grammar --color=never`.
- [x] Verify acceptance cases for `try_from("rust title=\"hi\"")`, quoted custom grammar tokens, `src/main.rs`, `config.yml`, `Dockerfile`, unknown language errors, lossy fallback, token-only filename rejection, and explicit plain-text token preservation.

## Phase 3: Migrate Rendering And Highlighting Call Sites

- [x] Update `CodeBlock::with_fence_language`, `CodeBlock::rust`, `CodeBlock::yaml`, `CodeBlock::json`, `CodeBlock::toml`, and `CodeBlock::from_source_file` to use the new constructors instead of `from_fence_token`.
- [x] Preserve separate display-label flow for Markdown fence emission, HTML `language-*` classes, and change detection while routing syntax resolution through the stored `LanguageGrammar`.
- [x] Change `render_terminal_code_block` and `render_html_code_block` in `darkmatter/lib/src/markdown/output/code_block.rs` to accept `&LanguageGrammar` for syntax resolution.
- [x] Delete the local `find_syntax` helper in `output/code_block.rs`.
- [x] Move or rewrite the deleted helper's alias and fallback tests into `language_grammar.rs`.
- [x] Update YAML highlighting in `darkmatter/lib/src/markdown/highlighting/mod.rs` so `highlight_yaml_lines_with_theme` resolves with `LanguageGrammar::yaml().resolve(...)`.
- [x] Confirm `LanguageGrammar::PlainText` keeps unknown-token terminal and HTML rendering on the plain-text syntax path while emitting an empty display label.
- [x] Confirm explicit plain-text tokens such as `txt` keep their display token for Markdown and HTML class output while resolving to Plain Text.

### Parallelizable Work

- [x] In parallel after Phase 2, one implementer can migrate `CodeBlock` construction, another can migrate render helpers, and a third can migrate YAML highlighting, as long as they coordinate on the final helper signatures.

### Checkpoint 3

- [x] Run focused tests for code blocks and highlighting with `cargo test -p darkmatter code_block --color=never` and `cargo test -p darkmatter highlighting --color=never`.
- [x] Re-run the grammar lookup search and confirm production direct syntect lookups outside `language_grammar.rs` have decreased to only remaining planned migration targets.

## Phase 4: Migrate Code Transclusion

- [x] Remove the private `lazy_static!` `SyntaxSet::load_defaults_newlines()` from `darkmatter/lib/src/markdown/compose/transclusion/code.rs`.
- [x] Change `infer_language(path, fallback)` to validate support with `LanguageGrammar::from_filename(path)`.
- [x] Keep the returned Markdown fence token as the lowercase file extension when a filename resolves successfully, preserving existing output for extensions supported by both old and new grammar sets.
- [x] Add or update tests showing `.ts` or another two-face-only extension now emits the real extension token instead of the fallback.
- [x] Update transclusion golden or snapshot fixtures that assumed the narrower syntect-default grammar set.
- [x] Confirm fallback behavior remains unchanged for unsupported extensions and extensionless files that do not match known filenames.

### Checkpoint 4

- [x] Run focused compose/transclusion tests with `cargo test -p darkmatter transclusion --color=never`.
- [x] Inspect fixture diffs and verify any changed Markdown fence info strings are expected two-face grammar widening.

## Phase 5: Documentation, Skill, And Coordination Updates

- [x] Update rustdoc on `LanguageGrammar` to state it is Darkmatter's authoritative grammar lookup API.
- [x] Document when callers should use `from_token`, `from_extension`, `from_name`, `from_filename`, `from_lossy`, and `from_token_or_plain_text`.
- [x] Remove or rewrite comments and docs that refer to `from_fence_token` or duplicated resolver behavior, treating code as correct where comments drift.
- [x] Update `darkmatter/docs/topics/yamlblock-migration.md` and CLI argument docs so examples use the new constructors and behavior.
- [x] Update `.claude/skills/darkmatter/SKILL.md` with the rule: production code must use `LanguageGrammar` and must not call direct syntect lookup APIs outside `language_grammar.rs`.
- [x] Update `darkmatter/features/2026-06-14-summary-and-suggest/plan.md` so it assumes this grammar feature lands first and includes a guard against reintroducing direct production syntect lookup.
- [x] Note the intended code-transclusion behavior change for two-face-only extensions so downstream composed Markdown diffs are not misread as regressions.

### Parallelizable Work

- [x] Documentation updates can proceed in parallel with Phase 4 after the final public API names and behavior are stable.

### Checkpoint 5

- [x] Run `rg -n "from_fence_token|find_syntax_by_|SyntaxSet::load_defaults_newlines" darkmatter .claude/skills/darkmatter/SKILL.md -S` and verify all remaining hits are either inside `language_grammar.rs`, test-only, historical completed feature docs, or intentionally updated references explaining the old behavior.

## Phase 6: Full Verification And Handoff

- [ ] Run `cargo test -p darkmatter --color=never`.
- [ ] Run `cargo test -p darkmatter-cli --color=never` if CLI docs, schema-about tests, or CLI argument surfaces changed.
- [ ] Run any area recipe used by the darkmatter package if available, such as `just test`, only if it is already established for this package area and expected to complete non-interactively.
- [ ] Re-run the production lookup guard with `rg -n "find_syntax_by_|from_fence_token|SyntaxSet::load_defaults_newlines" darkmatter -S`.
- [ ] Manually classify remaining direct `find_syntax_by_*` hits and confirm production hits outside `language_grammar.rs` are gone; allowed test-only hits include YAML color helpers and syntax-set loading tests named in the spec.
- [ ] Review `git diff --check` and inspect final diffs for unrelated formatting churn.
- [ ] Prepare the implementation handoff summary with changed files, behavior changes, validation commands, and any skipped tests.

### Final Acceptance Checkpoint

- [ ] All spec acceptance criteria pass or have a documented blocker.
- [ ] No in-tree caller references `from_fence_token`.
- [ ] `compose/transclusion/code.rs` no longer owns a private default `SyntaxSet`.
- [ ] Public docs and the darkmatter skill make the single grammar-authority rule discoverable.