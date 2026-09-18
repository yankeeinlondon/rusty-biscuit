---
# A supported combination: the Web API sends and a research-only Events API
# companion receives; the incoming webhook stays one-way. No capability is
# inherited from the platform name. Criterion 20. Expect: valid.
$schema: ../../../../../docs/research/platforms/_schema.yaml
schema_version: 1
platform_id: slack
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
- interface_id: slack_web_api
  role: sending_adapter
  api_identity: Fixture slack_web_api
  classification: official
  direction: send_only
  adapters:
  - slack
  operations:
  - chat_post_message
  relationships:
  - kind: requires_companion_interface
    target: slack_events_api
- interface_id: slack_incoming_webhook
  role: sending_adapter
  api_identity: Fixture slack_incoming_webhook
  classification: official
  direction: send_only
  adapters:
  - slack-webhook
  operations:
  - post_webhook
- interface_id: slack_events_api
  role: research_only
  api_identity: Fixture slack_events_api
  classification: official
  direction: receive_only
  adapters: []
  operations: []
  relationships:
  - kind: receives_events_for
    target: slack_web_api
    prerequisites:
    - event subscription
    - channels:history scope
coverage:
- interface: slack_web_api
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
- interface: slack_incoming_webhook
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
- interface: slack_events_api
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
inbound_bindings:
- id: ib.fx.webhook_none
  interface: slack_incoming_webhook
  event: none
  mechanism: none
  reach:
  - none
  content: none
  reuses_send_identity: 'no'
  knowledge:
    state: known
    evidence:
    - src.fx.docs
- id: ib.fx.events
  interface: slack_events_api
  event: message.channels
  mechanism: push_webhook
  reach:
  - groups
  - mentions
  content: full_text
  reuses_send_identity: 'no'
  prerequisites:
  - app installed in the channel
  ack_deadline_ms: 3000
  redelivery: 'yes'
  ordering: 'no'
  knowledge:
    state: known
    evidence:
    - src.fx.docs
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
