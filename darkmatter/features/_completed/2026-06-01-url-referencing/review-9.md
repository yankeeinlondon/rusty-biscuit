---
ready: true
agent: codex
model: ""
---

# Review 9: URL Referencing

## Findings

No production-blocking findings.

The review-8 issues appear resolved:

- Eager expression discovery is now parser-driven and limited to actual
  interpolation expressions with exact read-side function identifiers, so prose,
  inline code, fenced code, and longer identifiers do not trigger remote fetch
  registration.
- Remote `prologue` / `epilogue` references are now registered before the
  prepared remote transclusion waits on them, and both allowed and denied cases
  have library and CLI coverage.
- The stale `remote_read_config` field docs were updated to describe the wired
  behavior.

## Test Rigor

Level 1 is the appropriate verification level for this feature's user-facing
requirements: URL classification, deny-all and allow-host policy behavior,
remote transclusion, read-side expression URL reads, cache freshness, CLI flags,
remote `prologue` / `epilogue`, and rendered-link preservation. These are
filesystem, HTTP policy, compose, and CLI behaviors; the spec does not require
terminal keyboard input, paste / IME / mouse behavior, modifier visibility,
terminal scrolling, or terminal rendering fidelity that would require Level 2
or Level 3 verification.

Requirement-to-level summary:

- HTTP(S) classification and local-vs-remote resolution: Level 1 biscuit-file
  unit tests.
- Shared fetch primitive, host allowlist, conditional headers, HTTP errors, and
  redirect blocking: Level 1 biscuit-file fetch integration tests with a mock
  HTTP server.
- Eager single-flight registration, concurrency cap, nested remote discovery,
  and persistent cache freshness: Level 1 darkmatter unit / integration tests
  with `wiremock`.
- `::file`, `::code`, `prologue`, `epilogue`, and read-side expression URL
  reads: Level 1 darkmatter compose tests plus Level 1 CLI tests.
- Rendered HTTP(S) links preserved and not fetched: Level 1 compose / CLI tests.

I ran:

```bash
GIT_TERMINAL_PROMPT=0 cargo test -p biscuit-file -p darkmatter -p darkmatter-cli remote --color=never
GIT_TERMINAL_PROMPT=0 cargo test -p biscuit-file --color=never
cargo test -p biscuit-file --features fetch --test fetch_integration --color=never
```

Results:

- Remote-filtered suite: passed. `darkmatter` ran 111 remote-filtered tests,
  `darkmatter-cli` ran 10 remote CLI tests, and one selected Level 2 terminal
  style test passed. The Level 2 test is not required by this feature's URL
  behavior, but it did pass.
- `biscuit-file` default suite: passed, including 262 unit tests, 26 integration
  tests, and 17 doctests.
- Feature-gated `biscuit-file` fetch integration suite: passed, 11 tests.

## Recommendation

Mark this feature production-ready.
