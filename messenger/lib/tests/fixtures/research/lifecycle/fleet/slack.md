---
# Accepted-scope fleet fixture for slack: every roster interface, a complete
# coverage matrix resting on one investigated gap, and a few researched
# constraints. Used as a publication baseline; not research. Expect: valid
# in Accepted scope.
$schema: ../../../../../../docs/research/platforms/_schema.yaml
schema_version: 1
platform_id: slack
created: 2026-09-01
last_updated: 2026-09-10
agent: fixture
model: none
sources:
- id: src.slack.docs
  kind: official_docs
  url: https://docs.example.com/slack/messages
  locator: Limits
  retrieved: 2026-09-10
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
- interface_id: slack_socket_mode
  role: research_only
  api_identity: Fixture slack_socket_mode
  classification: official
  direction: receive_only
  adapters: []
  operations: []
  relationships:
  - kind: receives_events_for
    target: slack_web_api
- interface_id: slack_interactivity
  role: research_only
  api_identity: Fixture slack_interactivity
  classification: official
  direction: callback_only
  adapters: []
  operations: []
  relationships:
  - kind: responds_to_interactions_from
    target: slack_web_api
coverage:
- interface: slack_web_api
  categories:
    versions:
      status: gap
      gap: gap.slack.scope
    constraints:
      status: researched
    formatting:
      status: gap
      gap: gap.slack.scope
    text_bindings:
      status: gap
      gap: gap.slack.scope
    images:
      status: gap
      gap: gap.slack.scope
    attachments:
      status: gap
      gap: gap.slack.scope
    addressing:
      status: gap
      gap: gap.slack.scope
    receipts:
      status: gap
      gap: gap.slack.scope
    attribution:
      status: gap
      gap: gap.slack.scope
    location:
      status: gap
      gap: gap.slack.scope
    expression:
      status: gap
      gap: gap.slack.scope
    interactivity:
      status: gap
      gap: gap.slack.scope
    delivery_controls:
      status: gap
      gap: gap.slack.scope
    eligibility:
      status: gap
      gap: gap.slack.scope
    rate_limits:
      status: gap
      gap: gap.slack.scope
    errors:
      status: gap
      gap: gap.slack.scope
- interface: slack_incoming_webhook
  categories:
    versions:
      status: gap
      gap: gap.slack.scope
    constraints:
      status: gap
      gap: gap.slack.scope
    formatting:
      status: gap
      gap: gap.slack.scope
    text_bindings:
      status: gap
      gap: gap.slack.scope
    images:
      status: gap
      gap: gap.slack.scope
    attachments:
      status: gap
      gap: gap.slack.scope
    addressing:
      status: gap
      gap: gap.slack.scope
    receipts:
      status: gap
      gap: gap.slack.scope
    attribution:
      status: gap
      gap: gap.slack.scope
    location:
      status: gap
      gap: gap.slack.scope
    expression:
      status: gap
      gap: gap.slack.scope
    interactivity:
      status: gap
      gap: gap.slack.scope
    delivery_controls:
      status: gap
      gap: gap.slack.scope
    eligibility:
      status: gap
      gap: gap.slack.scope
    rate_limits:
      status: gap
      gap: gap.slack.scope
    errors:
      status: gap
      gap: gap.slack.scope
- interface: slack_events_api
  categories:
    versions:
      status: gap
      gap: gap.slack.scope
    constraints:
      status: gap
      gap: gap.slack.scope
    formatting:
      status: gap
      gap: gap.slack.scope
    text_bindings:
      status: gap
      gap: gap.slack.scope
    images:
      status: gap
      gap: gap.slack.scope
    attachments:
      status: gap
      gap: gap.slack.scope
    addressing:
      status: gap
      gap: gap.slack.scope
    receipts:
      status: gap
      gap: gap.slack.scope
    attribution:
      status: gap
      gap: gap.slack.scope
    location:
      status: gap
      gap: gap.slack.scope
    expression:
      status: gap
      gap: gap.slack.scope
    interactivity:
      status: gap
      gap: gap.slack.scope
    delivery_controls:
      status: gap
      gap: gap.slack.scope
    eligibility:
      status: gap
      gap: gap.slack.scope
    rate_limits:
      status: gap
      gap: gap.slack.scope
    errors:
      status: gap
      gap: gap.slack.scope
- interface: slack_socket_mode
  categories:
    versions:
      status: gap
      gap: gap.slack.scope
    constraints:
      status: gap
      gap: gap.slack.scope
    formatting:
      status: gap
      gap: gap.slack.scope
    text_bindings:
      status: gap
      gap: gap.slack.scope
    images:
      status: gap
      gap: gap.slack.scope
    attachments:
      status: gap
      gap: gap.slack.scope
    addressing:
      status: gap
      gap: gap.slack.scope
    receipts:
      status: gap
      gap: gap.slack.scope
    attribution:
      status: gap
      gap: gap.slack.scope
    location:
      status: gap
      gap: gap.slack.scope
    expression:
      status: gap
      gap: gap.slack.scope
    interactivity:
      status: gap
      gap: gap.slack.scope
    delivery_controls:
      status: gap
      gap: gap.slack.scope
    eligibility:
      status: gap
      gap: gap.slack.scope
    rate_limits:
      status: gap
      gap: gap.slack.scope
    errors:
      status: gap
      gap: gap.slack.scope
- interface: slack_interactivity
  categories:
    versions:
      status: gap
      gap: gap.slack.scope
    constraints:
      status: gap
      gap: gap.slack.scope
    formatting:
      status: gap
      gap: gap.slack.scope
    text_bindings:
      status: gap
      gap: gap.slack.scope
    images:
      status: gap
      gap: gap.slack.scope
    attachments:
      status: gap
      gap: gap.slack.scope
    addressing:
      status: gap
      gap: gap.slack.scope
    receipts:
      status: gap
      gap: gap.slack.scope
    attribution:
      status: gap
      gap: gap.slack.scope
    location:
      status: gap
      gap: gap.slack.scope
    expression:
      status: gap
      gap: gap.slack.scope
    interactivity:
      status: gap
      gap: gap.slack.scope
    delivery_controls:
      status: gap
      gap: gap.slack.scope
    eligibility:
      status: gap
      gap: gap.slack.scope
    rate_limits:
      status: gap
      gap: gap.slack.scope
    errors:
      status: gap
      gap: gap.slack.scope
api_versions: []
chronology: []
constraints:
- id: c.slack.text.truncation
  interface: slack_web_api
  operation: chat_post_message
  surface: body
  native_locator: text
  kind: hard_max
  value: 40000
  unit: unspecified_characters
  measurement_stage: field_value
  enforced_by: service
  overflow_behavior: truncate
  applies_when: []
  knowledge:
    state: known
    evidence:
    - src.slack.docs
    explanation: Text beyond the threshold is truncated by the service.
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
question_bindings: []
form_bindings: []
interaction_fixtures: []
changes: []
gaps:
- id: gap.slack.scope
  kind: research
  status: investigated
  question: Categories outside this fixture's subject are deliberately not researched.
  facts: []
  searches:
  - 'none: fixture scope'
  inspected_sources:
  - src.slack.docs
  unresolved_reason: The fixture exercises publication, not research.
  blocked_decision: None; fixtures are not research.
  next_investigation: None.
requires_messenger_update: false
---

# Slack fleet fixture

Publication fixture; not research.
