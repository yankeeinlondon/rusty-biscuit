---
agent: codex
model: ""
ready: false
---

# Review: Language Grammar Resolution

## Findings

### High: `make` does not resolve as a token alias

The spec requires `makefile` and `make` to resolve to the Makefile grammar by name/token (`spec.md:357`). Iteration #2 fixed the explicit filename path, so `LanguageGrammar::from_filename("make")` now resolves, but the central token constructor still does not consult `extensionless_alias`.

`LanguageGrammar::from_token("make")`, `LanguageGrammar::try_from("make")`, and `LanguageGrammar::from_lossy("make")` therefore return `UnknownGrammar` / `PlainText` instead of a token-preserving Makefile grammar. This is user-facing for fenced Markdown input such as:

````md
```make
target:
	echo hi
```
````

References:

- `darkmatter/features/2026-06-15-grammar/spec.md:357` requires `makefile`, `make` as Makefile token aliases.
- `darkmatter/lib/src/markdown/language_grammar.rs:536` defines `extensionless_alias("make") -> "Makefile"`.
- `darkmatter/lib/src/markdown/language_grammar.rs:568` implements `resolve_token`, but lines `581-588` only try direct extension/name/case-insensitive name lookup before returning `UnknownGrammar`; they never apply `extensionless_alias`.
- `darkmatter/lib/src/markdown/language_grammar.rs:712` tests `c++`, `dockerfile`, and `makefile`, but omits `make`.
- `darkmatter/lib/src/markdown/language_grammar.rs:1068` only covers `make` through `from_filename`, which is not the token path required by the alias table.

Suggested fix: have `resolve_token` apply `extensionless_alias(&lower)` before returning `UnknownGrammar`, validating the mapped extension/name against `load_syntax_set()` and returning `OtherByToken(token.to_string())` on success. Add Level 1 tests for `from_token("make")`, `try_from("make")`, and `from_lossy("make")`.

## Test Rigor Classification

This feature is pure parser/resolver/render-helper behavior. The appropriate verification level is Level 1 for constructor semantics, resolver behavior, code-block render routing, YAML highlighting, and transclusion language inference. No requirement depends on terminal emulator rendering, terminal input encoding, OS keyboard injection, mouse input, paste, IME behavior, or scrolling, so Level 2 and Level 3 coverage are not required for production readiness.

Observed Level 1 coverage:

- `cargo test -p darkmatter language_grammar --color=never` passed: 45 grammar-focused tests.
- `cargo test -p darkmatter compose::transclusion::code --color=never` passed: 8 transclusion helper tests.
- `cargo test -p darkmatter highlight_yaml_lines --color=never` passed: 5 YAML-highlighting tests.

Coverage gaps:

- No Level 1 test exercises the required token alias `LanguageGrammar::from_token("make")`.
- No Level 1 test exercises the same `make` alias through the general input paths `TryFrom<&str>` / `FromStr` / `from_lossy`.

## Ready For Production

Not ready. The previous review's `Dockerfile` / `Makefile` filename and dynamic-extension findings appear fixed, and production syntect lookup is confined to `language_grammar.rs`, but the required `make` token alias is still missing from the central public token path.
