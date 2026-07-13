---
ready: false
agent: codex/default
created: 2026-07-10T09:15:49
implemented: true
---

# Review: Suggested Values for SimplifiedSchema (Iteration #1)

## Summary

The implementation has a sound core: eligible grammar forms parse, numeric
normalization is string-based, generated metadata remains non-validating,
linting is library-owned, standalone envelopes resolve through a shared path,
and raw JSON Schema remains distinct. The focused feature suites pass.

The feature is not production-ready, however. Four focused review regressions
exposed failures in explicit acceptance criteria: root-union completion can
return invalid candidates or no candidates, inline diagnostics can point at an
unrelated frontmatter value, and numeric completion filters canonical text
instead of decoded authoring text.

## Findings

### High: root-union completion selects an atom and its lint provenance through different rules

In `simplified/query.rs:93-117`, `suggestions_for_path` first obtains the
suggestion-bearing atom and then independently calls `locate_atom` to determine
the root/property arm used to match lint problems. The two searches are not
equivalent:

- `suggestion_atom_for_path` skips a top-level property definition without a
  `suggest(...)` arm, but `locate_atom` returns that first definition anyway
  (`query.rs:189-195`).
- While searching a root union for a nested path, `descend(shape, ancestors)?`
  at `query.rs:135` returns from the entire function when one earlier arm lacks
  the ancestor instead of continuing to later arms.

The resulting observable failures are:

1. Given a first root arm with `value: string` and a later arm with
   `value: number(suggest(1, many, 2))`, completion returns `many`; the selected
   atom is from the later arm, but lint filtering looks for problems from the
   first arm.
2. If only a later root arm contains
   `settings.mode: string(suggest(fast, slow))`, completion for
   `settings.mode` returns nothing.

This violates acceptance criteria 7, 18, 19, and 20. Return the selected atom
and its root/property-arm provenance from one traversal, and make a missing
ancestor continue to the next root arm. Add Level 1 library and in-memory LSP
regressions for both cases, including an invalid candidate in the selected
later arm.

### High: inline diagnostic span projection is not scoped to the `$schema` value

`project_suggestion_spans` scans every scalar in the supplied YAML source and
chooses the next scalar whose parsed candidates equal the AST candidates
(`simplified/source.rs:39-44`, `101-109`, and `125-142`). DMLS supplies the
entire frontmatter YAML block while passing only the parsed `$schema` value
(`dmls/src/overlay/suggestions.rs:45-61`).

Consequently, if an ordinary frontmatter field before `$schema` contains the
same type-expression text, an invalid-suggestion warning is ranged against the
ordinary field instead of the authored schema candidate. A focused regression
with identical `number(suggest(1, many, 2))` text before and inside `$schema`
produced `24..28` for the first `many`; the correct `$schema` range was
`70..74`.

This breaks the exact-authoring-range contract in acceptance criteria 12 and
14. Projection should start from the parser's structural YAML node/span for the
actual `$schema` payload and property value, not recover identity by candidate
equality across the document. Add a Level 1 DMLS diagnostic test with a decoy
expression before `$schema` and assert the exact LSP range.

### High: numeric prefix filtering uses canonical labels instead of decoded candidate text

The spec requires prefix filtering against decoded candidate values, while
numeric insertion uses canonical decimal text. `SuggestionItem` retains both,
but DMLS filters with `suggestion.label.starts_with(prefix)` at
`providers/frontmatter.rs:252-256`; `label` is canonicalized for numbers by
`simplified/query.rs:267-272`. The public `matches_prefix` helper has the same
problem at `query.rs:318-320`.

For `number(suggest(003.5))`, typing prefix `00` yields no completion even
though decoded text is `003.5`; the eventual insertion should still be `3.5`.
This violates acceptance criteria 18 and 19. Filter on `decoded`, keep the
canonical label/insertion text, and add Level 1 library plus in-memory LSP
coverage for leading-zero and trailing-fractional-zero spellings.

### Medium: the acceptance matrix overstates both coverage and test level

The `level2_suggest_*` tests use `Connection::memory()` and an in-process server
thread (`dmls/tests/suggest_constraint_phase1.rs:1-35`). They do not use a real
terminal or terminal harness, so under the repository taxonomy they are Level
1 integration tests, not Level 2. Level 1 is the appropriate level for this
LSP protocol behavior, but naming and filtering them as `level2_` removes them
from `just test` and makes the validation record inaccurate.

The matrix also assigns behavior that the named tests do not exercise:

- `level2_suggest_phase1_standalone_ranges_and_completion` checks standalone
  diagnostics only; it performs no completion request.
- The completion-position test covers an inline schema, not completion from
  either referenced standalone envelope.
- No end-to-end test covers invalid-sibling omission, decoded numeric-prefix
  matching, a nested property supplied only by a later root arm, or a bare
  block-array dash (`-`) before the trailing space exists.
- `suggest_phase1_standalone_envelopes_resolve_consistently` remains ignored,
  despite the matrix's statement that phase scaffolds are enabled as their
  implementation lands. Later Phase 4 tests cover similar resolution behavior,
  so this is record drift rather than an additional functional blocker.

Rename the in-memory sessions as ordinary Level 1 integration tests, run them
under `just test`, and update the matrix to identify the actual executable
owner of each criterion. Add the missing cases above before claiming criteria
15, 18, 19, 20, and 21 are fully verified.

## Test Rigor Assessment

| User-facing requirement | Strongest observed verification | Assessment |
|---|---:|---|
| Grammar, eligibility, cardinality, interpretation, normalization, duplicates, generated metadata, and non-validating behavior (AC 1-13, 23) | Level 1 unit/integration/snapshot | Appropriate, except later-root invalid filtering fails as described above. |
| Exact inline and standalone warning severity/source/code/range (AC 14-15) | Level 1 in-memory LSP session | Appropriate level; inline range scoping is functionally broken and standalone completion is not covered by the named test. |
| Standalone whole-file/import behavior and raw-schema separation (AC 16-17, 22) | Level 1 temp-file resolver tests plus in-memory LSP session | Appropriate. |
| Scalar, nested, block-array, and flow-array completion; insertion and unions (AC 18-21) | Level 1 in-memory LSP session | Appropriate level but incomplete; root-union and decoded-prefix failures remain, and referenced-envelope completion is untested. |
| Cross-platform path/newline assumptions (AC 24) | Level 1 CRLF/UTF-8/source-map and temp-file tests on macOS | Reasonable host verification; no OS-specific API was introduced, but Windows/Linux were not executed in this review. |

No requirement needs Level 2 real-terminal capture or Level 3 OS input
injection. This feature exposes library and LSP protocol behavior; it does not
specify terminal rendering, keyboard, mouse, paste, or IME behavior.

## Verification

- Focused Darkmatter feature binaries: 32 passed, 1 ignored.
- Focused DMLS suggestion tests: 30 passed.
- Four temporary review regressions: 4 failed, confirming the three functional
  findings above; the temporary test file was removed afterward.
- The full package-area `just test` run was stopped after exceeding the
  non-interactive command budget; 2,037 tests had passed with no failure before
  interruption.
- `just lint` could not start within the command budget because another Cargo
  process held the shared build lock; it was stopped without a lint result.

## Verdict

Not ready for production. Fix the root-union selection/provenance traversal,
scope source projection structurally to `$schema`, and filter numeric prefixes
against decoded text. Then add Level 1 integration regressions for those paths
and correct the test-tier/matrix claims.
