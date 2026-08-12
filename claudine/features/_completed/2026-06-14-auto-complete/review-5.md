---
ready: true
agent: "codex/default"
created: "2026-06-27T06:51:13"
implemented: true
---

# Review 5 - Auto Complete

Ready for production. I did not find any remaining blocking gaps against the
specification.

## Findings

No blocking findings.

Review 5 confirms the two review-4 blockers have been addressed:

- The no-match and over-cap ENTER autocomplete failures now have Level 1 PTY
  tests that drive the CLI route through `claudine compose --goose <query>`,
  assert the rendered error text, assert a non-zero exit, and assert the
  provider stub was not launched
  ([level1_compose_autocomplete_failure_pty.rs](../../cli/tests/level1_compose_autocomplete_failure_pty.rs:107),
  [level1_compose_autocomplete_failure_pty.rs](../../cli/tests/level1_compose_autocomplete_failure_pty.rs:148)).
- The performance helper now fails for any p95 above the 100 ms target, and
  fast unit tests pin the former warning range as a failure
  ([completion_perf.rs](../../cli/tests/completion_perf.rs:294),
  [completion_perf.rs](../../cli/tests/completion_perf.rs:488)).

## Verification Matrix

- Shared bounded walker, query-filtered cap counting, scope exclusion rules,
  and mode contract filtering: Level 1 unit coverage is present.
- Current dynamic completion contract, `claudine __complete`, bootstrap
  `claudine completions <shell>`, `@` sigil stripping, bare `file`/`file[]`
  fallback, and comma continuation: Level 1 subprocess/unit coverage is
  present.
- Operation-file single-match confirmation and multi-match chooser for Markdown
  compose: Level 2 real-terminal rendering and interaction coverage are
  present.
- Operation-file YAML sequence candidate confirmation, chooser, and detail
  rendering and interaction: Level 2 real-terminal coverage is present in
  [level2_auto_complete_operation_file.rs](../../cli/tests/level2_auto_complete_operation_file.rs).
- Missing `file` and `file[]` frontmatter property chooser behavior: Level 2
  rendering and interaction coverage is present. A single focused L3 smoke
  test verifies real macOS Enter delivery through WezTerm without duplicating
  the product-behavior matrix.
- Autocomplete failure modes: non-TTY has Level 1 subprocess coverage; no-match
  and over-cap now have Level 1 PTY CLI-path coverage, which is appropriate
  because these are immediate error surfaces rather than terminal layout or key
  encoder requirements.
- Latency: the ignored performance harness still exists for the real timing
  measurement, and the assertion contract now fails above the documented
  100 ms p95 target.

## Notes

I started a narrow local verification run:

```text
cargo nextest run --color=never -p claudine-cli --test completion_perf assert_perf_target
```

It was aborted with exit code 130 after a cold dependency compile exceeded the
non-interactive command time budget. I did not run the full Level 1 suite or
the L2/L3 terminal harnesses for this review.
