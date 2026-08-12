---
ready: false
agent: "codex/default"
created: "2026-06-29T03:34:38"
implemented: true
---

# Review 4

## Findings

### High - `Diagnostic::detail()` does not consistently satisfy the locked registry schemas

The error catalog makes `detail` part of the public, handleable contract: every field listed by a code must be reachable as `err.detail.*`, with absent optional values projecting as `null`, not by omitting the shape entirely. The implementation still has several code/detail mismatches.

Examples:

- `provider.unavailable` declares `provider` and `path` in the registry (`claudine/lib/src/diagnostics/registry.rs`:115-120), but `ClaudineError::ProviderNotAvailable` only emits `provider` (`claudine/lib/src/error.rs`:325-328).
- `io.read_failed` and `io.permission_denied` declare `path` (`claudine/lib/src/diagnostics/registry.rs`:313-335), but many `ClaudineError::Io`, `Sqlite`, and parse/config variants that map to those codes fall through to `Value::Null` (`claudine/lib/src/error.rs`:261-268 and `:352-355`).
- `composition.failed` declares `source_path` (`claudine/lib/src/diagnostics/registry.rs`:254-260), but the catch-all `CompositionError` path returns `Value::Null` (`claudine/lib/src/composition/error.rs`:2818-2819 and `:2875`). `ClaudineError::SystemPromptComposition` also maps to `composition.failed` (`claudine/lib/src/error.rs`:297-298) while its detail path falls through to null.

Impact: lifecycle handlers and API callers can see a stable `err.code` but cannot rely on the corresponding `err.detail.*` schema advertised by `claudine errors`. That violates the handleability axis of the feature: authors still have to special-case missing detail or fall back to prose.

Verification level: Level 1 is appropriate. Add an executable conformance test that exercises representative `Diagnostic` values for every code family and asserts every field listed in `code_spec(err.code()).detail` is present, using `null` for unavailable optional values.

### High - The Claudine Level 2 reference-report test still does not cover the full flagship requirement

The spec requires the reference failure to render identically through `md compose` and `claudine compose`, including the root-cause headline, receiving key, OSC8-linked prompt file, focused `$schema` / `spec` / `iteration` excerpt, and likely-file suggestions (`claudine/features/2026-06-28-real-errors/spec.md`:170-173).

The new Claudine Level 2 test is valuable, but its fixture moved the involved keys to top-level frontmatter and explicitly avoids `$schema` (`claudine/cli/tests/level2_invalid_file_reference_capture.rs`:32-42). The assertions then check only that `iteration` appears somewhere and that `spec:` appears somewhere (`claudine/cli/tests/level2_invalid_file_reference_capture.rs`:156-164); they do not verify a `$schema:` parent, a focused `iteration:` excerpt line, exclusion of unrelated keys, or a `Did you mean?` section. The Darkmatter Level 2 suite now covers suggestions (`darkmatter/cli/tests/level2_errors.rs`:419-463), but there is no equivalent Claudine Level 2 suggestion capture, so the cross-binary parity requirement is still weaker than the spec asks for.

Verification level mismatch: this is terminal-rendered, user-observable behavior. Per the review rubric, the strongest verification for the full Claudine reference report must be Level 2. Add a Claudine Level 2 near-sibling fixture that asserts the suggestions section, and either use a `$schema`-parent shape where the product supports it or update the spec/design to stop requiring `$schema` in the flagship reference report.

## Summary

Review 4 confirms real progress since review 3: `claudine errors` exists, promoted lifecycle fields are wired, a targeted boundary lint exists, schema parse has Claudine Level 2 coverage, and the scoped compile check passed.

The feature is still not production-ready. The remaining blockers are the incomplete `err.detail.*` contract and the fact that the Claudine Level 2 flagship test still does not verify the full specified report shape.

Production ready: **no**.
