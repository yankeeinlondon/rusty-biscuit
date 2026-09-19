---
# A rate-limit error with its retry-after contract, an unknown code, a changed
# envelope field that must not produce a specific match, and an ambiguous
# timeout that stays unknown. Criteria 12, 13. Expect: valid.
$schema: ../../../../../docs/research/platforms/_schema.yaml
schema_version: 1
platform_id: telegram
created: &id001 2026-09-17
last_updated: *id001
agent: fixture
model: none
sources:
- id: src.fx.docs
  kind: official_docs
  url: https://docs.example.com/api/messages
  locator: Limits
  retrieved: *id001
- id: src.fx.sdk
  kind: sdk_validator
  location: fixtures/sdk/validate.rs
  locator: MAX_LEN
  revision: 0.17.0
  retrieved: *id001
- id: src.fx.code
  kind: source_code
  url: https://git.example.com/bridge/src/send.rs
  locator: send()
  revision: abc1234
  retrieved: *id001
- id: src.fx.observed
  kind: observed_fixture
  location: messenger/lib/tests/fixtures/research/diagnostics/observed.json
  retrieved: *id001
- id: src.fx.forum
  kind: secondary
  url: https://forum.example.com/t/limits
  retrieved: *id001
  note: community lead only
interfaces:
- interface_id: telegram_bot_api
  role: sending_adapter
  api_identity: Fixture telegram_bot_api
  classification: official
  direction: send_only
  adapters:
  - telegram
  operations:
  - send_message
coverage:
- interface: telegram_bot_api
  categories:
    versions:
      status: gap
      gap: gap.fx.scope
    constraints:
      status: gap
      gap: gap.fx.scope
    formatting:
      status: gap
      gap: gap.fx.scope
    text_bindings:
      status: gap
      gap: gap.fx.scope
    images:
      status: gap
      gap: gap.fx.scope
    attachments:
      status: gap
      gap: gap.fx.scope
    addressing:
      status: gap
      gap: gap.fx.scope
    receipts:
      status: gap
      gap: gap.fx.scope
    attribution:
      status: gap
      gap: gap.fx.scope
    location:
      status: gap
      gap: gap.fx.scope
    expression:
      status: gap
      gap: gap.fx.scope
    interactivity:
      status: gap
      gap: gap.fx.scope
    delivery_controls:
      status: gap
      gap: gap.fx.scope
    eligibility:
      status: gap
      gap: gap.fx.scope
    rate_limits:
      status: researched
    errors:
      status: researched
api_versions: []
chronology: []
constraints: []
format_profiles: []
text_bindings: []
image_bindings: []
role_coverage: []
attachment_bindings: []
addressing: []
receipts: []
attribution_bindings: []
location_bindings: []
author_geolocation: []
expression_bindings: []
delivery_controls: []
eligibility: []
rate_limits:
- id: rl.fx.tg
  interface: telegram_bot_api
  operations:
  - send_message
  scope: conversation
  basis: dynamic
  retry_after_locator: /parameters/retry_after
  retry_after_unit: seconds
  errors:
  - err.fx.429
  idempotency: unsupported
  knowledge:
    state: known
    evidence:
    - src.fx.docs
envelopes:
- id: env.fx.tg
  interface: telegram_bot_api
  operations:
  - send_message
  origin: service
  body_format: json
  success_discriminator: /ok
  code_locator: /error_code
  message_locator: /description
  retry_after_locator: /parameters/retry_after
  retry_after_unit: seconds
  knowledge:
    state: known
    evidence:
    - src.fx.docs
- id: env.fx.transport
  interface: telegram_bot_api
  operations:
  - send_message
  origin: transport
  body_format: empty
  knowledge:
    state: known
    evidence:
    - src.fx.docs
errors:
- id: err.fx.429
  interface: telegram_bot_api
  operations:
  - send_message
  envelope: env.fx.tg
  origin: service
  phase: response
  outcome: failure
  category: rate_limited
  match:
    http_status: 429
    native_code_number: 429
  related_facts:
  - rl.fx.tg
  diagnostic_fields:
  - /parameters/retry_after
  fixtures:
  - fx.fx.429
  - fx.fx.409
  - fx.fx.changed
  recovery_prerequisites: Wait retry_after seconds; replay safety is unknown because the API has no idempotency key.
  delivery_certainty: rejected
  recovery: retry_candidate
  replay_safety: unknown
  remediation: wait
  knowledge:
    state: known
    evidence:
    - src.fx.docs
- id: err.fx.timeout
  interface: telegram_bot_api
  operations:
  - send_message
  envelope: env.fx.transport
  origin: transport
  phase: response
  outcome: failure
  category: transport
  delivery_certainty: unknown
  recovery: unknown
  replay_safety: unknown
  remediation: check_service
  knowledge:
    state: unknown
    evidence: []
    gap: gap.fx.scope
    explanation: A timeout does not prove rejection; the message may have been accepted.
error_fixtures:
- id: fx.fx.429
  envelope: env.fx.tg
  operation: send_message
  provenance: constructed_from_docs
  expect: match
  expected_error: err.fx.429
  http_status: 429
  body: '{"description": "Too Many Requests: retry after 35", "error_code": 429, "ok": false, "parameters": {"retry_after": 35}}'
- id: fx.fx.409
  envelope: env.fx.tg
  operation: send_message
  provenance: constructed_from_docs
  expect: unknown
  http_status: 409
  body: '{"description": "Conflict", "error_code": 409, "ok": false}'
  note: Unknown code keeps an unknown classification with safe native context.
- id: fx.fx.changed
  envelope: env.fx.tg
  operation: send_message
  provenance: constructed_from_docs
  expect: unknown
  http_status: 429
  body: '{"code": 429, "ok": false}'
  note: 'Envelope field renamed: the code locator no longer resolves, so no specific match.'
- id: fx.fx.timeout
  envelope: env.fx.transport
  operation: send_message
  provenance: constructed_from_docs
  expect: unknown
  note: 'No response before the deadline: ambiguous delivery, never a known rejection or a retry authorization.'
inbound_bindings: []
question_bindings: []
form_bindings: []
interaction_fixtures: []
changes: []
gaps:
- id: gap.fx.scope
  kind: research
  status: investigated
  question: Categories outside this fixture's subject are deliberately not researched.
  facts: []
  searches:
  - 'none: fixture scope'
  inspected_sources:
  - src.fx.docs
  unresolved_reason: The fixture exercises one contract representation only.
  blocked_decision: None; fixtures are not research.
  next_investigation: None.
requires_messenger_update: false
---

# Fixture

Contract fixture; not research.
