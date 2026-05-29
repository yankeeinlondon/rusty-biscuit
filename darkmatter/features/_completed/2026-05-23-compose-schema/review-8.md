---
ready: false
agent: codex
model: ""
---

# Review 8

## Findings

### Medium: Shell-dependent schema problems are deferred even when frontmatter shell expansion is disabled

`schema_validation::run` defers every validation problem whose top-level frontmatter value contains `$(`:

- `darkmatter/lib/src/markdown/compose/schema_validation.rs:76`
- `darkmatter/lib/src/markdown/compose/schema_validation.rs:83`
- `darkmatter/lib/src/markdown/compose/schema_validation.rs:91`

That is correct only when `ComposeOperation::FrontmatterShellExpansion` will actually run. The compose pipeline always invokes schema validation, but it conditionally skips frontmatter shell expansion:

- `darkmatter/lib/src/markdown/compose/mod.rs:540`
- `darkmatter/lib/src/markdown/compose/mod.rs:549`

So a caller can run `ComposeOptions::new().only(&[ComposeOperation::Interpolation])` against a document like:

```markdown
---
$schema:
  spec: 'string(min(1); required)'
spec: "$(printf '')"
---
Body
```

The schema stage sees the `min(1)` failure, classifies it as shell-dependent, and returns `Ok(())`; no later stage expands or re-validates `spec`, so the schema violation is silently accepted. That undercuts the spec's "always-on and always strict" compose validation contract for custom operation sets. The deferral predicate needs to be gated on `options.is_enabled(ComposeOperation::FrontmatterShellExpansion)`, or the stage needs an explicit post-shell revalidation path when shell expansion is enabled.

Recommended test: add a Level 1 compose test that disables `FrontmatterShellExpansion`, keeps a `$(` value that violates a schema constraint, and asserts `MarkdownError::SchemaValidationFailed`. Level 1 is sufficient because this is an in-process API/pipeline contract, not terminal rendering behavior.

## Test Rigor Notes

Most requirements are covered at the right level:

- Level 1: document `$schema`, no-schema no-op, baseline merge, document-overrides-baseline, post-override validation, child validation with `set=`, fail-fast before shell expansion, and cache invalidation by baseline schema.
- Level 1 CLI process tests: `md compose` fail-fast regression, schema validate parity, source path in rendered error, and preparation-failure diagnostic.
- Level 2: `SchemaValidationFailed` terminal rendering verifies OSC8 source link, visible property text, dim/italic description styling, red category label, and inverse property label in a real terminal capture.

I found no Level 3 requirement in this feature. The spec does not define OS keyboard, paste, IME, mouse, or modifier-key behavior.

The gap above is also a test gap: there is no Level 1 case for shell-dependent validation problems when `FrontmatterShellExpansion` is disabled.

## Verification

I attempted focused Cargo verification with:

```bash
cargo test -p darkmatter schema_validation_fails_fast_before_shell_expansion --color=never
```

The command was still compiling dependencies after 60 seconds, which exceeds the non-interactive session limit, so I terminated it. No test failure was observed; verification is inconclusive.

## Recommendation

Not ready for production until schema problem deferral is tied to an enabled shell-expansion path, or until compose performs the promised revalidation before returning success.
