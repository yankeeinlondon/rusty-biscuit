---
ready: false
agent: "codex/default"
created: "2026-06-29T04:17:02"
implemented: true
---

# Review 5

## Findings

### High - `composition.shell_expansion` exposes the prompt path as `err.detail.command`

`CompositionError::ShellExpansionFailed` maps to the locked `composition.shell_expansion` code, whose detail schema declares a `command` field (`claudine/lib/src/composition/error.rs`:2798). The detail projection currently fills that field with `source_path` (`claudine/lib/src/composition/error.rs`:2921-2924), so lifecycle handlers see the Markdown file path where the authored shell command should be.

That is incorrect data, not just missing optional data. The source error already carries the command for every command-shaped shell failure (`darkmatter/lib/src/markdown/compose/shell_expansion/types.rs`:525-590), and parse/policy/preflight variants can still project `null` or a message where no command exists. As written, an author handling `err.code == "composition.shell_expansion"` cannot target the failed command without parsing prose, which violates the handleability contract.

The new conformance test misses this because it explicitly omits `CompositionError::ShellExpansionFailed` and treats `HarnessError::ShellCommandDenied` as coverage for the whole code family (`claudine/lib/tests/diagnostic_detail_conformance.rs`:153-156). This needs a representative `CompositionError::ShellExpansionFailed` test, even if the `SourceContext` fixture is a little heavier, and the projection should extract the command from the boxed `ShellExpansionError` where available.

Verification level: Level 1 is appropriate because this is an in-process API/lifecycle `err.detail.*` contract.

### High - Some lifecycle authoring errors still lose their typed diagnostic code/detail

`LifecycleMultipleLifecycleActions` is a lifecycle stack authoring error with the same shape as the other lifecycle validation errors: it carries the offending `property` and has a dedicated user-facing status block (`claudine/lib/src/composition/error.rs`:567-571, `claudine/lib/src/composition/error.rs`:2130-2142). The parser raises it when a stack item contains more than one lifecycle control action (`claudine/lib/src/composition/lifecycle.rs`:1624-1628).

However, `Diagnostic::code()` does not include this variant in the lifecycle family match, so it falls through to `composition.failed` (`claudine/lib/src/composition/error.rs`:2802-2819). Its `detail()` arm is missing as well, so the useful `property` is not projected as `err.detail.property` / `err.detail.message` (`claudine/lib/src/composition/error.rs`:2929-2978).

Impact: two lifecycle syntax mistakes that render similarly to humans become different, less handleable machine errors. A handler that targets `composition.lifecycle_invalid` or checks `err.detail.property == "success"` will work for `LifecycleActionOrder` but not for the adjacent "multiple lifecycle actions" case.

Verification level: Level 1 is appropriate. Add per-variant diagnostic conformance tests for lifecycle variants that carry `property`, not just one representative for the code family.

## Notes

The review-4 blockers are materially improved: Claudine now has Level 2 captures for the invalid-file-reference headline/excerpt/link path, the near-sibling "Did you mean?" path, and the `$schema` parent path. Those terminal-rendering requirements now have the right verification level.

I attempted to run `cargo nextest run -p claudine --test diagnostic_detail_conformance --color never`, but stopped it after about 60 seconds per the non-interactive session rule while it was still compiling dependencies. No completed test result is available from this review run.

Production ready: **no**.
