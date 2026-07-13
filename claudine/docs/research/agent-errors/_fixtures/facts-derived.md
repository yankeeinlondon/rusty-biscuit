---
$schema: ../_schema.yaml
created: 2026-07-12
last_updated: 2026-07-12
agent: opencode
model: kimi-for-coding/k2p7
docs: https://github.com/openai/codex
# A facts-derived shape: every seeded needle re-stated as `evidence: seed`,
# repeated `kind` (the late-ApiRemote second pass), a code row, and an explicit
# capacity gap. This is what a first research pass looks like before any
# documented additions land.
kind_buckets:
  - kind: api_remote
    needles:
      - text: rate
        evidence: seed
      - text: quota
        evidence: seed
      - text: billing
        evidence: seed
  - kind: configuration
    needles:
      - text: auth
        evidence: seed
      - text: config
        evidence: seed
  - kind: interrupted
    needles:
      - text: interrupt
        evidence: seed
      - text: abort
        evidence: seed
  - kind: api_remote
    needles:
      - text: api
        evidence: seed
      - text: upstream
        evidence: seed
      - text: server
        evidence: seed
msg_buckets:
  - kind: api_remote
    needles:
      - text: rate limit
        evidence: seed
      - text: api error
        evidence: seed
  - kind: configuration
    needles:
      - text: api key
        evidence: seed
      - text: permission denied
        evidence: seed
code_buckets:
  - kind: configuration
    codes:
      - code: -32001
        name: AUTH_EXPIRED
        evidence: seed
gaps:
  - area: capacity
    notes: >-
      Could not confirm the exact "at capacity" / 503 overload phrasing in the
      current CLI source; needs a documented citation before it becomes a needle.
changes: []
requires_claudine_update: false
---

# Error Vocabulary Fixture — Facts-Derived Shape

This is a schema-validation fixture (not a real provider document). It exercises
the facts-derived shape: repeated kinds, a code row, message-only optional
branches, and an explicit `capacity` gap.
