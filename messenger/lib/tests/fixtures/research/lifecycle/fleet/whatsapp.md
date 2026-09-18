---
# Accepted-scope fleet fixture for whatsapp: every roster interface, a complete
# coverage matrix resting on one investigated gap, and a few researched
# constraints. Used as a publication baseline; not research. Expect: valid
# in Accepted scope.
$schema: ../../../../../../docs/research/platforms/_schema.yaml
schema_version: 1
platform_id: whatsapp
created: 2026-09-01
last_updated: 2026-09-10
agent: fixture
model: none
sources:
- id: src.whatsapp.docs
  kind: official_docs
  url: https://docs.example.com/whatsapp/messages
  locator: Limits
  retrieved: 2026-09-10
interfaces:
- interface_id: whatsapp_cloud_api
  role: sending_adapter
  api_identity: Fixture whatsapp_cloud_api
  classification: official
  direction: send_only
  adapters:
  - whatsapp
  operations:
  - send_message
- interface_id: whatsapp_webhooks
  role: research_only
  api_identity: Fixture whatsapp_webhooks
  classification: official
  direction: receive_only
  adapters: []
  operations: []
  relationships:
  - kind: receives_events_for
    target: whatsapp_cloud_api
coverage:
- interface: whatsapp_cloud_api
  categories:
    versions:
      status: gap
      gap: gap.whatsapp.scope
    constraints:
      status: gap
      gap: gap.whatsapp.scope
    formatting:
      status: gap
      gap: gap.whatsapp.scope
    text_bindings:
      status: gap
      gap: gap.whatsapp.scope
    images:
      status: gap
      gap: gap.whatsapp.scope
    attachments:
      status: gap
      gap: gap.whatsapp.scope
    addressing:
      status: gap
      gap: gap.whatsapp.scope
    receipts:
      status: gap
      gap: gap.whatsapp.scope
    attribution:
      status: gap
      gap: gap.whatsapp.scope
    location:
      status: gap
      gap: gap.whatsapp.scope
    expression:
      status: gap
      gap: gap.whatsapp.scope
    interactivity:
      status: gap
      gap: gap.whatsapp.scope
    delivery_controls:
      status: gap
      gap: gap.whatsapp.scope
    eligibility:
      status: gap
      gap: gap.whatsapp.scope
    rate_limits:
      status: gap
      gap: gap.whatsapp.scope
    errors:
      status: gap
      gap: gap.whatsapp.scope
- interface: whatsapp_webhooks
  categories:
    versions:
      status: gap
      gap: gap.whatsapp.scope
    constraints:
      status: gap
      gap: gap.whatsapp.scope
    formatting:
      status: gap
      gap: gap.whatsapp.scope
    text_bindings:
      status: gap
      gap: gap.whatsapp.scope
    images:
      status: gap
      gap: gap.whatsapp.scope
    attachments:
      status: gap
      gap: gap.whatsapp.scope
    addressing:
      status: gap
      gap: gap.whatsapp.scope
    receipts:
      status: gap
      gap: gap.whatsapp.scope
    attribution:
      status: gap
      gap: gap.whatsapp.scope
    location:
      status: gap
      gap: gap.whatsapp.scope
    expression:
      status: gap
      gap: gap.whatsapp.scope
    interactivity:
      status: gap
      gap: gap.whatsapp.scope
    delivery_controls:
      status: gap
      gap: gap.whatsapp.scope
    eligibility:
      status: gap
      gap: gap.whatsapp.scope
    rate_limits:
      status: gap
      gap: gap.whatsapp.scope
    errors:
      status: gap
      gap: gap.whatsapp.scope
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
- id: gap.whatsapp.scope
  kind: research
  status: investigated
  question: Categories outside this fixture's subject are deliberately not researched.
  facts: []
  searches:
  - 'none: fixture scope'
  inspected_sources:
  - src.whatsapp.docs
  unresolved_reason: The fixture exercises publication, not research.
  blocked_decision: None; fixtures are not research.
  next_investigation: None.
requires_messenger_update: false
---

# Whatsapp fleet fixture

Publication fixture; not research.
