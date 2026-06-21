---
ready: false
agent: codex
model: ""
created: "2026-06-19T16:32:29"
---

# Review 1 — Comprehensive Review Remediation

## Findings

### High — `paths[]` protect extraction only scans the first path

`claudine/lib/src/protect/observe.rs:170` accepts the `paths` key, but the array branch at `claudine/lib/src/protect/observe.rs:174` returns only `arr.first()`. The regression test at `claudine/lib/src/protect/observe.rs:348` locks this in by expecting `["/tmp/a", "/tmp/b"]` to produce only `/tmp/a`.

This does not satisfy P2.2's requirement that write-like tools with `paths[]` are scanned. A payload such as:

```json
{ "paths": ["src/generated.txt", "~/.ssh/config"] }
```

would evaluate only the benign first path and never pass the sensitive second path to `ProtectService::evaluate_write_path`. This preserves a fail-open bypass for the exact multi-path tool shape the spec calls out.

**Verification level:** Level 1 is appropriate because this is pure extraction/dispatch behavior with no terminal encoder/renderer dependency. Current Level 1 coverage is present but asserts the wrong behavior. Production readiness is blocked.

### Medium — key-value `statusCode=` still accepts four-digit prefixes

P7.2 requires status-code handling to avoid lossy/bogus external status values. The JSON regex has a boundary guard, but the key-value regex at `claudine/lib/src/stream/logs/opencode/errors.rs:26` is still `statusCode=(\d{3})`, so `extract_status_code` at `claudine/lib/src/stream/logs/opencode/errors.rs:674` reads `statusCode=4299` as `429`.

The current test explicitly documents this as intentional at `claudine/lib/src/stream/logs/opencode/errors.rs:1686`, expecting `statusCode=9999` to return `Some(999)`. That means malformed provider text can still be misclassified as a real 3-digit HTTP status, including false rate-limit detection for `4299`.

**Verification level:** Level 1 is appropriate. I ran:

```bash
cargo nextest run -p claudine extract_status_code_returns_none_for_missing_code --no-tests=pass
```

It passed, confirming the suite does not catch this gap.

### Low — lifecycle undefined-variable docs still describe the old ternary behavior

The P5.1 implementation descends ternary conditions at `claudine/lib/src/composition/lifecycle.rs:785`, and the tests at `claudine/lib/src/composition/lifecycle.rs:1807` and `claudine/lib/src/composition/lifecycle.rs:1827` cover that behavior. However, the public rustdoc still says ternary subtrees are skipped wholesale at `claudine/lib/src/composition/lifecycle.rs:705` and again at `claudine/lib/src/composition/lifecycle.rs:771`.

Per the repo's comment-quality rule, this drift should be fixed in the same behavior-changing change. The code appears correct; the comments are stale.

**Verification level:** Level 1 is sufficient; this is API/documentation accuracy for an in-process validator.

## Coverage Notes

Most requirements in this remediation are in-process parsing, policy, persistence, or process-management behavior, so Level 1 unit/integration tests are generally the right verification level. I did not find a user-observable terminal rendering or OS keyboard-input requirement in this spec that would require Level 2 or Level 3 coverage under the provided taxonomy.

I did not run the full `just test` or `just test-l2` suites. The targeted nextest command above passed.

## Production Readiness

Not ready. The remaining `paths[]` extraction bypass is a High-severity miss against the protect hardening acceptance criteria, and the status-code parser still encodes a known false-positive behavior.
