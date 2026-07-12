---
$schema: "@.claudine/schemas/review.yaml"
ready: false
agent: unknown/default
created: 2026-07-11T20:58:05-07:00
implemented: true
---

# Review: Real Error Messages (Iteration 2)

## Verdict

Not ready for production. The implementation now satisfies the feature's
behavioral requirements and closes both high-severity findings from iteration
1, but the canonical Level 1 package suite is red because the committed
provider dispatch inventory was not regenerated after the CLI source moved.
A feature should not be released while its required test recipe fails.

## Findings

### High: The canonical Level 1 suite fails its dispatch drift guard

Running `just test` from the `claudine` package area fails in
`claudine-cli::dispatch_inventory::dispatch_inventory_matches_committed_file`.
The test retried four times and reported the first divergence at line 1033 of
`claudine/docs/providers/dispatch-inventory.json`: the committed entry records
CLI source line 1166 while the generated inventory records line 1167. Nextest
then canceled 1,671 remaining CLI tests under fail-fast.

The staged implementation changes dispatch-bearing CLI source but does not
update the generated inventory. This is exactly the drift that the repository's
dispatch guard is intended to reject.

Recommended fix: regenerate the inventory with the command printed by the
guard, inspect and stage the resulting generated-file change, then rerun
`just test` to completion. Do not merely adjust the recorded line manually.

**Verification level:** Level 1 is the appropriate level for this source-to-
inventory consistency contract. It currently fails, so the release gate is
not satisfied.

## Prior Findings Rechecked

### Message hygiene is now complete

`failure_message` applies one final hygiene pass after choosing every cascade
branch. It strips terminal escapes, folds control-separated content to one
line, reserves room for the retry suffix, counts the ellipsis inside the
240-character budget, and clamps the complete result. The Level 1 unit matrix
now covers provider messages, each guard shape, both timeout modes, stderr,
launch failure, context-less abort, generic fallback, ANSI/OSC removal,
multiline input, oversized guard input, oversized stderr, and a retained
attempt-2 suffix under truncation.

This closes iteration 1's first high-severity finding.

### Structured provider errors now cross the full runtime seam

The new Unix Level 1 process-integration test stages an OpenCode-compatible
subprocess that emits a structured `api_timeout` event with `error_message:
"upstream timeout"`, exits non-zero, and asserts that the exact provider text
reaches both `failure.err.msg` and failed-`finalize.err.msg` instead of the
generic exit-code fallback. I ran this test independently and it passed.

This closes iteration 1's second high-severity finding. The Unix-only fixture
is acceptable for this regression because the underlying propagation and
builder are platform-neutral Rust, while the shell fixture is only the
subprocess producer. The test is compile-gated off on Windows rather than
pretending a POSIX shell fixture is portable.

## Requirement-to-Verification Matrix

| User-facing requirement | Strongest relevant evidence | Assessment |
|---|---|---|
| Structured provider text becomes `failure.err.msg` and failed-`finalize.err.msg` | Level 1 real subprocess integration | Appropriate and passing |
| Guard trips name their guard and key parameters | Level 1 pure builder matrix plus structured termination tests | Appropriate and passing |
| Attempt suffix appears only after attempt 1 for every cascade source | Level 1 table-driven unit matrix | Appropriate and passing |
| Provider/guard message precedence and generic exit-code fallback | Level 1 unit and process integration tests | Appropriate and passing |
| Single-line, escape-free, at-most-240-character message | Level 1 unit tests over all externally sourced branches | Appropriate and passing |
| Existing lifecycle fallback behavior | Existing Level 2 lifecycle coverage | More than required for message data; assertions were updated |

No requirement in this feature depends on terminal-emulator rendering or input
encoding. Level 2 and Level 3 evidence is therefore not required for the new
message-data behavior. The existing Level 2 lifecycle tests are supplementary;
there is no keyboard, paste, IME, mouse, glyph-width, SGR, or scrolling contract
in this specification that would justify Level 3 or additional Level 2 tests.

## Verification Performed

- `just test`: failed in `claudine-cli` at the dispatch-inventory drift guard;
  `claudine-catalog-types` passed 21 tests, `claudine` passed 3,395 tests, and
  `claudine-contract` passed 47 tests before the CLI failure stopped the recipe.
- `cargo nextest run -p claudine-cli --test level1_structured_error_message --color never`:
  passed 1 test.

The implementation should be considered production-ready once the dispatch
inventory is regenerated and the full canonical Level 1 recipe completes
successfully.
