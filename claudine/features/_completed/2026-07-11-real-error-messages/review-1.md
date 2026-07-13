---
$schema: review.yaml
ready: false
implemented: true
agent: unknown/default
created: 2026-07-11T20:24:01-07:00
---

# Review: Real Error Messages (Iteration 1)

## Verdict

Not ready for production. The implementation correctly propagates structured
`error_message` values, centralizes message construction in the library, and
removes the misleading first-attempt suffix. However, the promised hygiene
invariant is not enforced for every message source, and the feature's primary
structured-provider-error path has no end-to-end verification.

## Findings

### High: `err.msg` is not always sanitized, single-line, or capped

The specification's success criterion 6 requires every `err.msg` to be a
single sanitized line of at most approximately 240 characters because the
value can feed TTS, outbound messaging, notifications, and stderr.

`failure_message` applies `headline` only to `error_message` and `stderr_text`
at `claudine/lib/src/harness/runtime.rs:112-134`. The guard-context branch
returns `guard_message` directly. In particular, the exit-expression `pattern`
and `scope` are interpolated verbatim at `runtime.rs:146-153`; a configured
pattern containing a newline, ANSI/OSC sequence, or hundreds of characters
therefore reaches `err.msg` unchanged.

The cap is also not a cap on the final message. `clamp_chars` takes 240
characters and then appends an ellipsis, producing 241 characters, and
`failure_message` appends ` (attempt N)` afterward at `runtime.rs:103-109`.
Thus even provider-derived text can exceed the advertised maximum.

Recommended fix: build the complete message, including any attempt suffix,
then pass every cascade result through one final escape-strip, single-line,
character-budget function. Reserve space for the ellipsis and suffix so the
final returned string respects the declared limit. Add tests with multiline,
ANSI/OSC, and oversized guard patterns, plus an oversized retry message.

**Verification level:** Level 1 unit coverage exists for provider-message ANSI
stripping and truncation, but there is no test for guard-context hygiene or the
length of the final suffixed message. This is a user-observable messaging/TTS
contract; Level 1 is the appropriate minimum because terminal encoding and
rendering are not involved, but the required branches are currently untested
and broken.

### High: No integration test proves a real structured provider error becomes `err.msg`

The primary success criterion is that a provider stream's structured
`error_message` reaches the lifecycle `failure` event. The implementation
copies `summary.error_message` into `AttemptOutcome` at
`claudine/cli/src/commands/wrap/harness_orch/attempt.rs:310-321`, and isolated
parser and builder unit tests cover their respective ends of the pipeline.
No test crosses that seam.

The two existing Level 2 lifecycle assertions at
`claudine/cli/tests/level2_lifecycle_dispatch.rs:568-597` and
`:744-755` stage a provider that only exits 99 and therefore verify the generic
exit-code fallback. They do not emit a provider protocol error event and do
not verify `Too many requests`, `Insufficient credits`, or another structured
message in either `failure.err.msg` or `finalize.err.msg`. Consequently, a
regression that drops `summary.error_message` in `execute_harness_attempt`
would leave all current tests green while restoring the original user-facing
bug.

Recommended fix: add a subprocess integration fixture for at least one
structured provider protocol which emits a real error event and exits
non-zero, then assert the exact sanitized provider message reaches both the
failure lifecycle action and failed-finalize payload. This behavior needs
Level 1 process integration, not Level 2 terminal capture: the observable is
lifecycle data written to a file, not terminal glyphs or styling.

### Medium: The specified cascade and suffix matrix is only partially tested

The unit suite covers the attempt suffix for a provider message and generic
fallback only. It does not demonstrate the stated uniform policy for guard,
timeout, stderr, launch-failure, and context-less-abort results. Stderr also
lacks explicit ANSI and truncation cases, despite being a distinct line-pick
branch (last meaningful line rather than first).

The implementation is centralized, so these cases likely share the intended
behavior today, but the specification explicitly asks for every cascade step,
hygiene branch, and suffix policy to be table-tested. Convert the current
examples into a compact case table that asserts source precedence, expected
headline, attempt-1 output, attempt-2 output, single-line output, and final
length.

**Verification level:** Level 1 is appropriate and currently incomplete. No
requirement in this feature needs Level 3 OS keyboard injection. Level 2 is
only incidental in the existing lifecycle tests; the feature defines message
data, not terminal-emulator rendering.

## Requirement Coverage

| Requirement | Strongest verification | Status |
|---|---:|---|
| Structured stream `error_message` is copied into `AttemptOutcome` | Code review + disconnected Level 1 units | Gap: no process integration |
| Provider message outranks stderr | Level 1 unit | Ready |
| Guard/timeout/fallback cascade | Level 1 unit | Ready except global hygiene |
| Attempt-1 suffix omitted; retry suffix included | Level 1 unit | Partially covered |
| One library builder serves lifecycle and stderr banner | Code review + Level 2 generic fallback | Ready |
| Every `err.msg` is sanitized, single-line, and capped | Level 1 partial unit | Broken |
| Generic fallback includes exit code | Level 1 unit + Level 2 tmux lifecycle | Ready |

## Verification

- `just test` from the `claudine/` package area:
  - `claudine-catalog-types`: 21 passed.
  - `claudine`: 3,394 passed, 7 skipped.
  - `claudine-contract`: 47 passed, 5 skipped.
  - `claudine-cli`: compilation was still progressing after the bounded
    non-interactive wait and was interrupted; the full package recipe did not
    complete.
- Static review confirmed the propagation sites, single builder call site,
  updated Level 2 fallback assertions, and corrected `error_kind` rustdoc.

The passing suites do not resolve the two high-severity gaps above, so the
feature is not production-ready in this iteration.
