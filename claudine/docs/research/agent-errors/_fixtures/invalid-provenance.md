---
$schema: ../_schema.yaml
created: 2026-07-12
last_updated: 2026-07-12
agent: opencode
model: kimi-for-coding/k2p7
# INTENTIONALLY INVALID schema fixture: the `evidence` value `guessed` is not a
# member of the provenance enum. `md schema validate` MUST reject this, proving
# the sidecar constrains provenance. (The deterministic gate enforces the
# *conditional* source rule the enum cannot express — see gen/src/
# agent_errors_check.rs.)
msg_buckets:
  - kind: api_remote
    needles:
      - text: rate limit
        evidence: guessed
changes: []
requires_claudine_update: false
---

# Error Vocabulary Fixture — Invalid Provenance (must fail validation)

A negative fixture: schema validation must fail on the `guessed` provenance.
