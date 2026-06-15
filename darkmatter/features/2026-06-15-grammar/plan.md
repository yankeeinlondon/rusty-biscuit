---
agent: codex/
phases: 6
created: 2026-06-15
start_phase: 1
yolo: true
---

# Language Grammar Resolution Plan

## Success Criteria

- `LanguageGrammar` is the only production grammar lookup path in Darkmatter.
- Public construction covers fallible, lossy, token-only, extension, name, and filename inputs.
- Code-block rendering, YAML highlighting, and code transclusion route syntax resolution through `LanguageGrammar`.
- Direct production `SyntaxSet::find_syntax_by_*` calls remain only inside `language_grammar.rs`.
- Documentation, rustdoc, and the darkmatter skill describe the single-authority rule.

## Phase 1: Baseline And Scope Confirmation

- [ ] Confirm the working tree state with `GIT_TERMINAL_PROMPT=0 git status --short` and record unrelated dirty files before editing.
- [ ] Reconfirm relevant workspace packages with `cargo metadata --no-deps --format-version 1`, verifying the target crates are `darkmatter` and `darkmatter-cli`.
- [ ] Inspect `darkmatter/lib/src/markdown/language_grammar.rs`, `darkmatter/lib/src/markdown/code_block.rs`, `darkmatter/lib/src/markdown/output/code_block.rs`, `darkmatter/lib/src/markdown/highlighting/mod.rs`, and `darkmatter/lib/src/markdown/compose/transclusion/code.rs` to map current call sites and tests.
- [ ] Search for direct grammar lookups with `rg -n "find_syntax_by_|from_fence_token|SyntaxSet::load_defaults_newlines|find_syntax\\(" darkmatter -S` and classify each result as production, test-only, docs, or historical feature text.
- [ ] Identify docs that mention old behavior, including `darkmatter/docs/topics/yamlblock-migration.md`, `darkmatter/cli/src/args.rs`, and `.claude/skills/darkmatter/SKILL.md`.

### Checkpoint 1

- [ ] Produce a short implementation note listing every production migration target and every intentionally allowed test-only direct syntect lookup.

## Phase 2: Build The Authoritative `LanguageGrammar` API

- [ ] Add or update the `PlainText` variant so `LanguageGrammar::PlainText` represents the fallback grammar and `Display` returns an empty string for it.
- [ ] Implement `LanguageGrammar::plain_text()` and `LanguageGrammar::text()` as infallible convenience constructors that return `PlainText`.
- [ ] Implement named infallible constructors `yaml()`, `rust()`, `markdown()`, `json()`, and `toml()` without runtime lookup.
- [ ] Replace `from_fence_token` with `from_token`, `from_token_or_plain_text`, `from_lossy`, `from_extension`, `from_name`, and `from_filename`.
- [ ] Implement `TryFrom<&str>`, `TryFrom<String>`, and `FromStr` for `LanguageGrammar`; do not implement `From<&str>` or `From<String>`.
- [ ] Add token normalization that trims input, reads quoted tokens through the matching closing quote, and ignores unquoted ASCII-whitespace metadata after the first token.
- [ ] Add filename/path detection for the full fallible and `from_lossy` paths, skipping filename detection when unquoted whitespace indicates Markdown fence metadata.
- [ ] Centralize aliases and resolution order in one private resolver: explicit aliases, extension lookup, exact name lookup, case-insensitive name lookup, token validation, then `UnknownGrammar`.
- [ ] Preserve explicit plain-text tokens such as `txt`, `text`, `plain`, `plaintext`, and `plain-text` as token-preserving dynamic variants that resolve to syntect Plain Text.
- [ ] Add `resolve_default()` using Darkmatter's shared two-face syntax set and keep `resolve(&self, &SyntaxSet)` for custom syntax sets.
- [ ] Update `LanguageGrammarError::UnknownGrammar` usage so empty normalized fallible input returns `UnknownGrammar(String::new())`.

### Parallelizable Work

- [ ] In parallel after the API shape is settled, one implementer can add parsing and alias tests while another updates rustdoc examples in `language_grammar.rs`.

### Checkpoint 2

- [ ] Run `cargo test -p darkmatter language_grammar --color=never`.
- [ ] Verify acceptance cases for `try_from("rust title=\"hi\"")`, quoted custom grammar tokens, `src/main.rs`, `config.yml`, `Dockerfile`, unknown language errors, lossy fallback, token-only filename rejection, and explicit plain-text token preservation.

## Phase 3: Migrate Rendering And Highlighting Call Sites

- [ ] Update `CodeBlock::with_fence_language`, `CodeBlock::rust`, `CodeBlock::yaml`, `CodeBlock::json`, `CodeBlock::toml`, and `CodeBlock::from_source_file` to use the new constructors instead of `from_fence_token`.
- [ ] Preserve separate display-label flow for Markdown fence emission, HTML `language-*` classes, and change detection while routing syntax resolution through the stored `LanguageGrammar`.
- [ ] Change `render_terminal_code_block` and `render_html_code_block` in `darkmatter/lib/src/markdown/output/code_block.rs` to accept `&LanguageGrammar` for syntax resolution.
- [ ] Delete the local `find_syntax` helper in `output/code_block.rs`.
- [ ] Move or rewrite the deleted helper's alias and fallback tests into `language_grammar.rs`.
- [ ] Update YAML highlighting in `darkmatter/lib/src/markdown/highlighting/mod.rs` so `highlight_yaml_lines_with_theme` resolves with `LanguageGrammar::yaml().resolve(...)`.
- [ ] Confirm `LanguageGrammar::PlainText` keeps unknown-token terminal and HTML rendering on the plain-text syntax path while emitting an empty display label.
- [ ] Confirm explicit plain-text tokens such as `txt` keep their display token for Markdown and HTML class output while resolving to Plain Text.

### Parallelizable Work

- [ ] In parallel after Phase 2, one implementer can migrate `CodeBlock` construction, another can migrate render helpers, and a third can migrate YAML highlighting, as long as they coordinate on the final helper signatures.

### Checkpoint 3

- [ ] Run focused tests for code blocks and highlighting with `cargo test -p darkmatter code_block --color=never` and `cargo test -p darkmatter highlighting --color=never`.
- [ ] Re-run the grammar lookup search and confirm production direct syntect lookups outside `language_grammar.rs` have decreased to only remaining planned migration targets.

## Phase 4: Migrate Code Transclusion

- [ ] Remove the private `lazy_static!` `SyntaxSet::load_defaults_newlines()` from `darkmatter/lib/src/markdown/compose/transclusion/code.rs`.
- [ ] Change `infer_language(path, fallback)` to validate support with `LanguageGrammar::from_filename(path)`.
- [ ] Keep the returned Markdown fence token as the lowercase file extension when a filename resolves successfully, preserving existing output for extensions supported by both old and new grammar sets.
- [ ] Add or update tests showing `.ts` or another two-face-only extension now emits the real extension token instead of the fallback.
- [ ] Update transclusion golden or snapshot fixtures that assumed the narrower syntect-default grammar set.
- [ ] Confirm fallback behavior remains unchanged for unsupported extensions and extensionless files that do not match known filenames.

### Checkpoint 4

- [ ] Run focused compose/transclusion tests with `cargo test -p darkmatter transclusion --color=never`.
- [ ] Inspect fixture diffs and verify any changed Markdown fence info strings are expected two-face grammar widening.

## Phase 5: Documentation, Skill, And Coordination Updates

- [ ] Update rustdoc on `LanguageGrammar` to state it is Darkmatter's authoritative grammar lookup API.
- [ ] Document when callers should use `from_token`, `from_extension`, `from_name`, `from_filename`, `from_lossy`, and `from_token_or_plain_text`.
- [ ] Remove or rewrite comments and docs that refer to `from_fence_token` or duplicated resolver behavior, treating code as correct where comments drift.
- [ ] Update `darkmatter/docs/topics/yamlblock-migration.md` and CLI argument docs so examples use the new constructors and behavior.
- [ ] Update `.claude/skills/darkmatter/SKILL.md` with the rule: production code must use `LanguageGrammar` and must not call direct syntect lookup APIs outside `language_grammar.rs`.
- [ ] Update `darkmatter/features/2026-06-14-summary-and-suggest/plan.md` so it assumes this grammar feature lands first and includes a guard against reintroducing direct production syntect lookup.
- [ ] Note the intended code-transclusion behavior change for two-face-only extensions so downstream composed Markdown diffs are not misread as regressions.

### Parallelizable Work

- [ ] Documentation updates can proceed in parallel with Phase 4 after the final public API names and behavior are stable.

### Checkpoint 5

- [ ] Run `rg -n "from_fence_token|find_syntax_by_|SyntaxSet::load_defaults_newlines" darkmatter .claude/skills/darkmatter/SKILL.md -S` and verify all remaining hits are either inside `language_grammar.rs`, test-only, historical completed feature docs, or intentionally updated references explaining the old behavior.

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
