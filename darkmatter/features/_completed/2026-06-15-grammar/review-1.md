---
agent: codex
model: ""
ready: false
---

# Review: Language Grammar Resolution

## Findings

### High: explicit filename resolution does not support well-known extensionless files

The spec requires `LanguageGrammar::from_filename` to resolve extensionless well-known filenames such as `Makefile` and `Dockerfile` through the same alias table used by token lookup. The implementation detects those names as filename-like, but the explicit filename constructor only calls `grammar_from_alias` for extensionless basenames. That alias table omits `dockerfile`, `makefile`, and `make`, so `LanguageGrammar::from_filename("Dockerfile")` and `LanguageGrammar::from_filename("Makefile")` return `UnknownGrammar`.

This matters because `compose/transclusion/code.rs` now uses `from_filename` as the validation gate. As a result, `infer_language(Path::new("Makefile"), "txt")` returns the fallback token, and the test currently asserts that fallback behavior. That contradicts the API contract in the spec and leaves a user-facing transclusion gap for supported extensionless source files.

References:

- `darkmatter/lib/src/markdown/language_grammar.rs:237` documents that `from_filename` resolves `Makefile` and `Dockerfile`.
- `darkmatter/lib/src/markdown/language_grammar.rs:266` calls `grammar_from_alias` for extensionless filenames.
- `darkmatter/lib/src/markdown/language_grammar.rs:497` omits the required `dockerfile`, `makefile`, `make`, `c++`, and `cpp` aliases from that shared alias table.
- `darkmatter/lib/src/markdown/language_grammar.rs:616` has a separate alias mapping for `OtherByToken`, which is why `TryFrom` happens to pass for `Dockerfile`/`Makefile` after `from_filename` fails.
- `darkmatter/lib/src/markdown/compose/transclusion/code.rs:16` gates transclusion inference on `from_filename`.
- `darkmatter/lib/src/markdown/compose/transclusion/code.rs:80` locks in the wrong `Makefile` fallback behavior.

Suggested fix: make the explicit filename path use the same extensionless filename aliases as token resolution, then update/add tests for `LanguageGrammar::from_filename("Dockerfile")`, `from_filename("Makefile")`, and `infer_language(Path::new("Makefile"), "txt")`.

### Medium: explicit extension lookup is not fully case-insensitive for dynamic extensions

`from_extension` documents and the spec requires case-insensitive extension lookup. The implementation lowercases the input only for `grammar_from_alias`, then passes the original spelling to `find_syntax_by_extension`. Named aliases like `RS` work because `rs` is in the alias table, but non-aliased two-face/syntect extensions can still fail when supplied in uppercase or mixed case.

References:

- `darkmatter/lib/src/markdown/language_grammar.rs:187` documents case-insensitive extension lookup.
- `darkmatter/lib/src/markdown/language_grammar.rs:197` computes `lower`.
- `darkmatter/lib/src/markdown/language_grammar.rs:204` performs dynamic syntax lookup with the original `ext`, not `lower`.
- `darkmatter/lib/src/markdown/language_grammar.rs:988` tests only lowercase and dotted lowercase extension inputs.

Suggested fix: use the normalized lowercase extension for dynamic lookup, or try both original and lowercase while returning a canonical stored spelling. Add a non-aliased mixed-case extension test so the contract is covered beyond the named alias map.

## Test Rigor Classification

This feature is mostly pure parsing/resolution behavior. The appropriate verification level is Level 1 for constructor semantics, resolver behavior, code-block render helper routing, YAML highlighting, and transclusion fence token inference. No requirement depends on real terminal encoder behavior or OS keyboard injection, so Level 2/Level 3 coverage is not required for readiness.

Observed Level 1 coverage:

- `cargo test -p darkmatter language_grammar --color=never` passed: 41 grammar-focused unit tests.
- `cargo test -p darkmatter compose::transclusion::code --color=never` passed: 7 transclusion helper unit tests.
- `cargo test -p darkmatter from_filename --color=never` passed: 2 filtered tests, but this exposed that `from_filename` coverage does not include the spec-required extensionless filename cases.

Coverage gaps:

- No Level 1 test directly exercises `LanguageGrammar::from_filename("Dockerfile")` or `LanguageGrammar::from_filename("Makefile")`.
- The transclusion Level 1 test for `Makefile` currently expects fallback instead of the spec-required recognized filename behavior.
- No Level 1 test verifies case-insensitive lookup for dynamic, non-aliased extensions.

## Ready For Production

Not ready. The central migration is largely complete and direct production syntect lookup calls appear contained inside `language_grammar.rs`, but the explicit filename API misses a required user-facing behavior and transclusion currently preserves that miss.
