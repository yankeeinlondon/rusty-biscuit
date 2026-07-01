---
ready: true
agent: codex/default
created: 2026-06-30T17:55:16
---

# Review: File Property Rewrite

## Findings

No blocking findings.

The implementation matches the feature contract for the core behavior:

- eager `file(eager)` / `format: darkmatter-file` values are normalized after validation succeeds;
- bare lazy `file`, `string`, absent/null, pending, remote, and unresolvable values are left verbatim;
- normalization is exposed through an explicit read-only library API and compose opts into write-back;
- `validate` / `validate_with_positions` keep their no-mutation contract;
- nested inline-object properties, arrays, root unions, and property unions are covered by the rewrite walk;
- compose excludes `$schema` and caller-excluded keys from validation and rewrite write-back;
- path projection stores `/` separators for persisted Markdown.

## Test Rigor

Level 1 is the correct verification level for this feature. The behavior is schema parsing, JSON-value normalization, filesystem path resolution, compose-time frontmatter write-back, and CLI process output. It does not require a real terminal renderer, terminal input encoder, OS keyboard injection, mouse handling, paste, IME, styling capture, scrolling, or glyph-width verification, so Level 2 and Level 3 coverage would not add relevant signal.

Requirement-to-level mapping:

- `file(eager)` rewrites to repo-relative resolved path: Level 1 unit and CLI integration coverage present.
- lazy `file`, `string`, absent/null, remote, pending, and failed-resolution values are not rewritten: Level 1 unit coverage present.
- normalization is idempotent: Level 1 unit and CLI integration coverage present.
- nested inline-object and array eager-file values are rewritten: Level 1 unit coverage present.
- root/property unions follow the accepted-arm/no-guessing rules: Level 1 unit coverage present.
- compose write-back feeds downstream frontmatter and is a fixpoint: Level 1 compose-stage unit and CLI integration coverage present.
- validation-only APIs do not mutate caller input: Level 1 library API coverage present.
- CWD independence and launch-area fallback behavior: Level 1 unit coverage present.
- Windows separator stability: Level 1 projection coverage present; I did not run tests on Windows in this macOS session.

## Verification

Focused checks run:

- `cargo nextest run --package darkmatter --lib rewrite:: schema_validation:: --color=never` passed: 85 tests run, 85 passed, 4424 skipped.
- `cargo nextest run --package darkmatter-cli --test compose_schema_file_rewrite --color=never` passed: 3 tests run, 3 passed.

The library command matched more tests than intended because of broad name filtering, but it still exercised the new rewrite and compose schema-validation paths.

## Production Readiness

Ready for production. The implemented behavior satisfies the spec, and the strongest required verification level for each user-observable requirement is present.
