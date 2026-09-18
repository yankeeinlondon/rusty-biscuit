---
# Accepted-scope fleet fixture for telegram: every roster interface, a complete
# coverage matrix resting on one investigated gap, and a few researched
# constraints. Used as a publication baseline; not research. Expect: valid
# in Accepted scope.
$schema: ../../../../../../docs/research/platforms/_schema.yaml
schema_version: 1
platform_id: telegram
created: 2026-09-01
last_updated: 2026-09-10
agent: fixture
model: none
sources:
- id: src.telegram.docs
  kind: official_docs
  url: https://docs.example.com/telegram/messages
  locator: Limits
  retrieved: 2026-09-10
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
      gap: gap.telegram.scope
    constraints:
      status: gap
      gap: gap.telegram.scope
    formatting:
      status: gap
      gap: gap.telegram.scope
    text_bindings:
      status: gap
      gap: gap.telegram.scope
    images:
      status: gap
      gap: gap.telegram.scope
    attachments:
      status: gap
      gap: gap.telegram.scope
    addressing:
      status: gap
      gap: gap.telegram.scope
    receipts:
      status: gap
      gap: gap.telegram.scope
    attribution:
      status: gap
      gap: gap.telegram.scope
    location:
      status: gap
      gap: gap.telegram.scope
    expression:
      status: gap
      gap: gap.telegram.scope
    interactivity:
      status: gap
      gap: gap.telegram.scope
    delivery_controls:
      status: gap
      gap: gap.telegram.scope
    eligibility:
      status: gap
      gap: gap.telegram.scope
    rate_limits:
      status: gap
      gap: gap.telegram.scope
    errors:
      status: gap
      gap: gap.telegram.scope
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
question_bindings: []
form_bindings: []
interaction_fixtures: []
changes: []
gaps:
- id: gap.telegram.scope
  kind: research
  status: investigated
  question: Categories outside this fixture's subject are deliberately not researched.
  facts: []
  searches:
  - 'none: fixture scope'
  inspected_sources:
  - src.telegram.docs
  unresolved_reason: The fixture exercises publication, not research.
  blocked_decision: None; fixtures are not research.
  next_investigation: None.
requires_messenger_update: false
---

# Telegram fleet fixture

Publication fixture; not research.
