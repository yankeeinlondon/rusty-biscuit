---
$schema: ../_schema.yaml
created: 2026-07-12
last_updated: 2026-07-12
agent: opencode
model: kimi-for-coding/k2p7
docs: https://ai.google.dev/gemini-api/docs
# A research-shaped document: a message-only classifier (no `kind_buckets`),
# seeded needles retained, a documented capacity addition with a citation, and
# no code buckets. This is what a document with real additions looks like.
msg_buckets:
  - kind: configuration
    needles:
      - text: api key
        evidence: seed
      - text: authentication
        evidence: seed
  - kind: api_remote
    needles:
      - text: rate limit
        evidence: seed
      - text: overloaded
        evidence: documented
        source: https://ai.google.dev/gemini-api/docs/troubleshooting#error-503
      - text: resource_exhausted
        evidence: source_code
        source: https://github.com/google-gemini/gemini-cli/blob/v0.1.0/packages/core/src/errors.ts#L40
gaps: []
changes: []
requires_claudine_update: true
reason: >-
  A documented `overloaded` / `resource_exhausted` capacity vocabulary is
  proposed for the message branch; graduating it would add needles the current
  seed lacks. The addition is adjudicated in Phase C, not here.
---

# Error Vocabulary Fixture — Research-Shaped

This is a schema-validation fixture (not a real provider document). It exercises
a message-only classifier with the optional `kind_buckets`/`code_buckets`
branches absent, seeded needles preserved, and documented capacity additions
carrying stable citations.
