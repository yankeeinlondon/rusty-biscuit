---
# Confirmation with an explicit affirmative/negative mapping; a timeout is a
# distinct outcome, never `false`. Sanitized placeholders replace IDs.
# Criterion 21. Expect: valid.
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
- interface_id: discord_bot_api
  role: sending_adapter
  api_identity: Fixture discord_bot_api
  classification: official
  direction: send_only
  adapters:
  - discord
  operations:
  - create_message
- interface_id: discord_interactions
  role: research_only
  api_identity: Fixture discord_interactions
  classification: official
  direction: callback_only
  adapters: []
  operations:
  - interaction_callback
  relationships:
  - kind: responds_to_interactions_from
    target: discord_bot_api
    prerequisites:
    - application public key
    authorization: Ed25519 request signature
coverage:
- interface: discord_bot_api
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
      status: researched
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
      status: gap
      gap: gap.fx.scope
- interface: discord_interactions
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
      status: researched
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
      status: gap
      gap: gap.fx.scope
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
rate_limits: []
envelopes: []
errors: []
error_fixtures: []
inbound_bindings: []
question_bindings:
- id: qb.fx.confirm
  interface: discord_bot_api
  kind: confirmation
  native: native
  mechanism: buttons
  operation: create_message
  native_locator: components[].components[] (button)
  answer_type: boolean
  cancellation: not_available
  responses: attributable
  validation: application
  visibility: conditional
  fidelity: exact
  companion_interface: discord_interactions
  answer_event: INTERACTION_CREATE
  answer_locator: /data/custom_id
  confirmation_mapping:
    affirmative: confirm.yes
    negative: confirm.no
  responder_locator: /member/user/id
  correlation_locator: /message/id
  lifecycle:
    ack_deadline_ms: 3000
    response_token_seconds: 900
    closable: 'yes'
  knowledge:
    state: known
    evidence:
    - src.fx.docs
form_bindings: []
interaction_fixtures:
- id: ix.fx.yes
  binding: qb.fx.confirm
  provenance: constructed_from_docs
  payload: '{"data": {"custom_id": "confirm.yes"}, "member": {"user": {"id": "USER_ID"}}, "message": {"id": "MESSAGE_ID"}, "type": 3}'
  expect: answer
  expected_answer: 'true'
- id: ix.fx.no
  binding: qb.fx.confirm
  provenance: constructed_from_docs
  payload: '{"data": {"custom_id": "confirm.no"}, "member": {"user": {"id": "USER_ID"}}, "message": {"id": "MESSAGE_ID"}, "type": 3}'
  expect: answer
  expected_answer: 'false'
- id: ix.fx.timeout
  binding: qb.fx.confirm
  provenance: constructed_from_docs
  payload: '{}'
  expect: timeout
  note: 'No interaction before the token expired: never an implicit false.'
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
