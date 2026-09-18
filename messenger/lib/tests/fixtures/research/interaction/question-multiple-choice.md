---
# Multiple choice through anonymous (aggregate-only) and non-anonymous polls;
# a retracted vote is `no_answer`, never an empty selection. Criterion 21.
# Expect: valid.
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
  direction: bidirectional
  adapters:
  - telegram
  operations:
  - send_message
  - send_poll
  - answer_callback_query
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
- id: qb.fx.poll
  interface: telegram_bot_api
  kind: multiple_choice
  native: native
  mechanism: native_poll
  operation: send_poll
  native_locator: options
  answer_type: option_id_array
  cancellation: unknown
  responses: aggregate_only
  validation: provider
  visibility: public
  fidelity: approximate
  options: static
  min_selections: 1
  max_selections: 3
  answer_event: poll
  answer_locator: /poll/options
  knowledge:
    state: known
    evidence:
    - src.fx.docs
- id: qb.fx.poll_named
  interface: telegram_bot_api
  kind: multiple_choice
  native: native
  mechanism: native_poll
  operation: send_poll
  native_locator: options
  answer_type: option_id_array
  cancellation: unknown
  responses: attributable
  validation: provider
  visibility: public
  fidelity: exact
  options: static
  min_selections: 0
  max_selections: 3
  answer_event: poll_answer
  answer_locator: /poll_answer/option_ids
  responder_locator: /poll_answer/user/id
  applies_when:
  - kind: message_form
    equals: non_anonymous_poll
  knowledge:
    state: known
    evidence:
    - src.fx.docs
form_bindings: []
interaction_fixtures:
- id: ix.fx.aggregate
  binding: qb.fx.poll
  provenance: constructed_from_docs
  payload: '{"poll": {"options": [{"text": "LABEL", "voter_count": 2}]}}'
  expect: aggregate_only
  note: Anonymous poll counts never become attributable answers.
- id: ix.fx.multi
  binding: qb.fx.poll_named
  provenance: constructed_from_docs
  payload: '{"poll_answer": {"option_ids": [0, 2], "user": {"id": "USER_ID"}}}'
  expect: answer
  expected_answer: '["0", "2"]'
- id: ix.fx.retracted
  binding: qb.fx.poll_named
  provenance: constructed_from_docs
  payload: '{"poll_answer": {"option_ids": [], "user": {"id": "USER_ID"}}}'
  expect: no_answer
  note: A retracted vote is no answer, not an empty selection.
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
