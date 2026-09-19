---
# Nested field errors: a specific signature (extra predicate) outranks the
# general one, and the content-limit error resolves to its constraint fact.
# Criteria 12, 13. Expect: valid.
$schema: ../../../../../docs/research/platforms/_schema.yaml
schema_version: 1
platform_id: discord
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
- interface_id: discord_webhook
  role: sending_adapter
  api_identity: Fixture discord_webhook
  classification: official
  direction: send_only
  adapters:
  - discord-webhook
  operations:
  - execute_webhook
coverage:
- interface: discord_webhook
  categories:
    versions:
      status: gap
      gap: gap.fx.scope
    constraints:
      status: researched
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
      status: gap
      gap: gap.fx.scope
    errors:
      status: researched
api_versions: []
chronology: []
constraints:
- id: c.fx.webhook.content
  interface: discord_webhook
  operation: execute_webhook
  surface: body
  native_locator: content
  kind: hard_max
  value: 2000
  unit: unspecified_characters
  measurement_stage: field_value
  enforced_by: service
  overflow_behavior: reject
  applies_when: []
  overflow_errors:
  - err.fx.content_field
  knowledge:
    state: known
    evidence:
    - src.fx.docs
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
rate_limits: []
envelopes:
- id: env.fx.discord
  interface: discord_webhook
  operations:
  - execute_webhook
  origin: service
  body_format: json
  code_locator: /code
  message_locator: /message
  field_errors_locator: /errors
  retry_after_locator: /retry_after
  retry_after_unit: seconds
  knowledge:
    state: known
    evidence:
    - src.fx.docs
errors:
- id: err.fx.content_field
  interface: discord_webhook
  operations:
  - execute_webhook
  envelope: env.fx.discord
  origin: service
  phase: response
  outcome: failure
  category: content_too_large
  match:
    http_status: 400
    native_code_number: 50035
    discriminator_locator: /errors/content/_errors/0/code
    discriminator_equals: BASE_TYPE_MAX_LENGTH
  related_facts:
  - c.fx.webhook.content
  affected_fields:
  - content
  diagnostic_fields:
  - /code
  - /errors/content/_errors/0/code
  fixtures:
  - fx.fx.nested
  - fx.fx.nested_other_field
  delivery_certainty: rejected
  recovery: after_correction
  replay_safety: safe
  remediation: shorten_content
  knowledge:
    state: known
    evidence:
    - src.fx.docs
- id: err.fx.form_body
  interface: discord_webhook
  operations:
  - execute_webhook
  envelope: env.fx.discord
  origin: service
  phase: response
  outcome: failure
  category: invalid_content
  match:
    http_status: 400
    native_code_number: 50035
  fixtures:
  - fx.fx.nested_other_field
  delivery_certainty: rejected
  recovery: after_correction
  replay_safety: unknown
  remediation: unknown
  knowledge:
    state: known
    evidence:
    - src.fx.docs
    explanation: The general form-body error; the content-field signature adds a predicate and takes precedence.
error_fixtures:
- id: fx.fx.nested
  envelope: env.fx.discord
  operation: execute_webhook
  provenance: constructed_from_docs
  expect: match
  expected_error: err.fx.content_field
  http_status: 400
  body: '{"code": 50035, "errors": {"content": {"_errors": [{"code": "BASE_TYPE_MAX_LENGTH", "message": "Must be 2000 or fewer in length."}]}}, "message": "Invalid Form Body"}'
- id: fx.fx.nested_other_field
  envelope: env.fx.discord
  operation: execute_webhook
  provenance: constructed_from_docs
  expect: match
  expected_error: err.fx.form_body
  http_status: 400
  body: '{"code": 50035, "errors": {"embeds": {"_errors": [{"code": "BASE_TYPE_REQUIRED"}]}}, "message": "Invalid Form Body"}'
  note: 'Only the general signature matches: the specific one needs a content field error code.'
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
