---
ready: false
agent: codex/default
created: 2026-06-20T13:59:58
implemented: true
---

# Review 5

Not ready for production. The Darkmatter fixes for the review-4 preflight dependency hole are covered by focused Level 1 tests and those tests pass, but Claudine's own regression for the motivating prompt currently fails. That leaves the original user-facing regression path unverified and means the requested package tests are not green.

## Findings

### High: Claudine regression for the motivating prompt is failing

`claudine/lib/src/composition/prepare.rs:865` defines `implement_suggestions_prompt_rejects_malformed_spec_path`, the Claudine-side regression required by the spec. It currently fails. The test expects composition of `prompts/implement-suggestions.md` with a review override to abort with an interpolation parse diagnostic naming `spec_path`, but the actual error is:

```text
compose failed: Schema validation failed for ".../prompts/implement-suggestions.md": frontmatter did not satisfy the schema
```

This appears to be caused by the shipped prompt no longer matching the original reproduction shape. `prompts/implement-suggestions.md` now declares `$schema.spec: string(required)` and no longer has the malformed `spec_path: "{{ dirname(review) + '/spec.md') }}"` value. The regression test still passes only a `review` override, so schema validation fails before the invalid whole-value interpolation path can be exercised.

That creates two production blockers:

- Acceptance criterion 4 is not verified against the original Claudine reproduction; the prompt now fails for a missing `spec`, not for the malformed `spec_path` leak.
- Acceptance criterion 7 is not met for the touched Claudine package because a targeted `cargo nextest run -p claudine ...` run fails.

Recommended fix: keep a stable minimal Claudine fixture with the malformed `spec_path` shape instead of relying on the mutable shipped prompt, or update the shipped-prompt regression so it intentionally exercises a malformed whole-value interpolation after satisfying the prompt schema. The test should assert the error names the offending key and reports an interpolation parse failure, not a generic schema failure.

Verification level: Level 1 is the correct tier. This is composition and CLI preparation behavior, not terminal rendering or keyboard-input encoding.

## Requirement Coverage

| Requirement | Strongest verification observed | Status |
| --- | --- | --- |
| Whole-value `{{ ... }}` parse/evaluation failures are fatal even with `fail_fast = false` | Level 1 Darkmatter unit tests | OK |
| Typed whole-value `{{ ... }}` results are preserved | Level 1 Darkmatter unit tests | OK |
| Mixed malformed interpolation remains lenient | Level 1 Darkmatter unit test | OK |
| Whole-value `$()` parse/expansion failures are fatal when shell expansion is enabled | Level 1 Darkmatter unit tests from prior implementation | OK |
| Raw expansion syntax does not appear in successful effective frontmatter | Level 1 Darkmatter tests; Claudine motivating-prompt regression is currently failing | Gap |
| Preflight approval set remains a faithful superset of real execution | Level 1 Darkmatter and Claudine preflight tests | OK |
| Required package tests pass | Focused Darkmatter tests pass; focused Claudine regression run fails | Gap |

No Level 2 or Level 3 tests are required for this spec. The observable behavior is compose preparation, shell-command preflight, diagnostics, and effective frontmatter content; Level 1 is the appropriate verification tier.

## Verification Performed

- `cargo nextest run -p darkmatter rejects_command_depending_on_context_requiring_sibling_key still_collects_command_depending_on_context_free_sibling_key best_effort_does_not_finalize_command_depending_on_errored_key best_effort_defers_transitive_dependent_of_errored_key collects_resolved_command_despite_context_requiring_sibling_key whole_value_parse_failure_is_fatal_without_fail_fast --color=never` passed: 6 tests run, 6 passed.
- `cargo nextest run -p claudine implement_suggestions_prompt_rejects_malformed_spec_path full_flow_shell_pending_dir_with_context_requiring_sibling_key --color=never` failed: `full_flow_shell_pending_dir_with_context_requiring_sibling_key` passed, but `implement_suggestions_prompt_rejects_malformed_spec_path` failed on all retries with the schema-validation error above.
