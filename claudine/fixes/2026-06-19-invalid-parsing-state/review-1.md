---
ready: false
implemented: true
agent: codex/default
created: "2026-06-20T07:24:06"
---

# Review 1

## Findings

### High: Padded malformed whole-value shell expressions can still survive enabled shell expansion

The spec defines the shell invariant on trimmed whole-value frontmatter: a value that trims to `$(...)` must parse and expand, or composition must fail when frontmatter shell expansion is enabled. The implementation only scans untrimmed strings for execution in `scan_frontmatter`, then the post-expansion guard only rejects values that parse cleanly as executable shell directives.

Evidence:

- `darkmatter/lib/src/markdown/compose/frontmatter_shell_expansion.rs:969` passes the raw string to `parse_shell_value`; a value like `"  $(echo ok"` is not scanned because it does not start with `$(`.
- `darkmatter/lib/src/markdown/compose/frontmatter_shell_expansion.rs:1575` trims before classification, but returns `false` when `parse_shell_value` returns an error. That means malformed trimmed whole-value shell expressions, invalid suffixes, and no-command diagnostics hidden behind leading whitespace are treated as non-candidates instead of hard errors.
- The tests cover padded valid values (`"   $(echo hi)  "`) and mixed/trailing literals, but do not cover padded malformed values such as `"  $(echo ok"` or padded no-command values such as `"  $(file_exists('x'))"`.

This misses the spec’s shell acceptance criteria: whole-value `$()` parse failures must be fatal when shell expansion is enabled, and raw expansion syntax must not leak into effective frontmatter.

### High: Required Level 1 package tests are not green

Acceptance criterion 7 requires `just test` to pass in the touched `darkmatter` and `claudine` package areas. I ran the requested package-level Level 1 recipes and both failed before completing all tests.

Observed failures:

- `just test darkmatter` failed in `darkmatter` at `darkmatter/lib/src/markdown/compose/schema_validation.rs:829`: `inline_object_uncoercible_value_left_alone` expected a problem under `/config/name` but got a root `/config` validation problem. `darkmatter-cli` and `dmls` passed afterward, but the package area run failed overall.
- `just test claudine-cli` failed in `claudine/cli/tests/command_routing.rs:309`: `force_color_enables_ansi_in_non_tty_context` expected ANSI output, but the error output was plain text.

The new Claudine regression `compose_dry_run_malformed_whole_value_spec_path_aborts_without_leaking` did pass before the `claudine-cli` run was cancelled, but the feature cannot be marked production-ready while the required package test commands fail.

## Requirement Coverage

| Requirement | Implementation | Strongest verification observed | Level assessment |
| --- | --- | --- | --- |
| Whole-value `{{ ... }}` frontmatter parse failures are fatal even with `fail_fast = false` | `interpolate_value` detects a single whole-value span and parses directly | Darkmatter unit test `whole_value_parse_failure_is_fatal_without_fail_fast`; Claudine CLI regression for malformed `spec_path` | Level 1 is appropriate; no real terminal behavior |
| Whole-value `{{ ... }}` evaluation failures are fatal | Same direct `eval_json` path returns `MarkdownError::Transform` | Existing remote-read tests were updated to expect hard errors for whole-value evaluation failures | Level 1 is appropriate |
| Whole-value `{{ ... }}` preserves typed values | Direct `eval_json` return is stored, including bool/number/null/array/object | Darkmatter tests for scalar and array preservation | Level 1 is appropriate |
| Mixed frontmatter/body interpolation remains lenient when `fail_fast = false` | Non-whole strings still route through `interpolate_text` | Darkmatter mixed malformed interpolation test | Level 1 is appropriate |
| Whole-value `$()` expands or fails when shell expansion is enabled | Normal unpadded values still parse through `scan_frontmatter`; post-expansion leak guard catches only clean surviving candidates | Darkmatter unit tests cover normal parse errors and clean surviving candidates, but miss padded malformed/no-command cases | Level 1 exists but is incomplete |
| Malformed `spec_path` reproduction fails instead of printing raw effective frontmatter | Claudine CLI dry-run regression asserts non-zero exit, key in stderr, parse diagnostic, no raw stdout leak, no provider launch | `compose_dry_run_malformed_whole_value_spec_path_aborts_without_leaking` passed during `just test claudine-cli` | Level 1 is appropriate |
| Documentation states the strict whole-value contract | Claudine skill, Claudine composition topic, and Darkmatter inline docs were updated | Static review | Level 1/doc review is appropriate |

No requirement in this spec asserts modifier-key behavior, terminal encoder behavior, hotkeys, paste/IME, mouse input, scrolling, or real-terminal styling. I do not see an L2 or L3 verification requirement for this fix. The relevant behavior is parse/compose semantics and CLI exit/output routing, so Level 1 coverage is the correct tier once the missing shell cases and failing tests are resolved.

## Notes

The interpolation side is generally stronger than the shell side: centralizing whole-value `{{ ... }}` strictness in `interpolate_value` preserves typed results and closes the original `spec_path` leak at the Darkmatter layer, which matches the spec’s preferred design. The remaining work is to make the shell leak guard treat trimmed parse failures as errors instead of "not a candidate", and to add regression tests for those padded malformed/no-command forms.
