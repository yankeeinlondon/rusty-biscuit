---
$schema: "@.claudine/schemas/review.yaml"
ready: true
implemented: true
agent: codex/default
created: 2026-07-11T10:44:10
---

# Review 6: Schema Triggers

## Findings

No production-blocking findings.

## Review-5 Closure

The open-envelope diagnostic publication gap is closed. After refreshing the
open-document set, the router now coalesces trigger diagnostic transitions by
envelope path and republishes every affected envelope from the final settled
registry state. Open envelopes are recomputed through the ordinary versioned
diagnostic path; closed envelopes retain file-level publication with no
version. This removes the prior dependence on open-document iteration order.

The new Level-1 LSP regressions cover immediate clearing after payload repair,
failure ownership transfer from an open envelope to a closed envelope, and both
relative open orders of the envelope and consumer. They also pin the expected
versioned versus file-level publication contract.

## Test Rigor

This feature's observable behavior consists of filesystem discovery, schema
parsing and matching, CLI activation and trace output, and LSP state and
diagnostic publication. Level 1 is appropriate for each requirement. No
requirement depends on rendering through a real terminal emulator or on a
terminal's input encoder, so Levels 2 and 3 are not required.

| Requirement | Strongest verification present | Assessment |
| --- | --- | --- |
| Match grammar, combinators, arms, path matching, forbidden constraints, and vacuous lint | Level 1 unit tests | Appropriate |
| Ancestor discovery, inclusive boundaries, ordering, shadowing, extensions, symlink exclusion, and case collisions | Level 1 filesystem tests | Appropriate |
| Envelope/payload separation, cycle rejection, merge compatibility, precedence, origins, and dependencies | Level 1 unit and integration tests | Appropriate |
| CLI compose/validate activation, assignment and shell re-resolution, raw mode, opt-out, and trace output | Level 1 binary integration tests | Appropriate |
| CLI/DMLS activation parity, last-good frontmatter, and transactional registry retention | Level 1 fixture and LSP integration tests | Appropriate |
| Trigger create/change/delete and payload dependency invalidation | Level 1 state and LSP integration tests | Appropriate |
| Closed-envelope diagnostic publication and clearing in both watcher modes | Level 1 LSP integration tests | Appropriate |
| Open-envelope repair and failure ownership transfer, independent of open order | Level 1 LSP integration tests | Appropriate |
| Generated schema-about trigger documentation and match-safe catalog agreement | Level 1 unit and CLI integration tests | Appropriate |

## Verification

- Targeted Level-1 nextest run: 2 passed, 0 failed.
- Full `just test` package-area Level-1 suite: passed for `darkmatter`,
  `darkmatter-cli`, and `dmls`; the DMLS suite reported 412 passed.
- `just lint`: passed for `darkmatter`, `darkmatter-cli`, and `dmls`.
- Level 2 and Level 3 were not run because the feature has no real-terminal
  rendering or OS-keyboard-input requirements.

## Production Readiness

Ready for production. The final review-5 blocker is repaired and directly
regression-tested, the acceptance criteria have verification at the appropriate
level, and the full Level-1 package-area test and lint gates pass.
