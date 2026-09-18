---
# Accepted-scope fleet fixture for signal: every roster interface, a complete
# coverage matrix resting on one investigated gap, and a few researched
# constraints. Used as a publication baseline; not research. Expect: valid
# in Accepted scope.
$schema: ../../../../../../docs/research/platforms/_schema.yaml
schema_version: 1
platform_id: signal
created: 2026-09-01
last_updated: 2026-09-10
agent: fixture
model: none
sources:
- id: src.signal.docs
  kind: official_docs
  url: https://docs.example.com/signal/messages
  locator: Limits
  retrieved: 2026-09-10
interfaces:
- interface_id: signal_cli_jsonrpc
  role: sending_adapter
  api_identity: Fixture signal_cli_jsonrpc
  classification: official
  direction: send_only
  adapters:
  - signal
  operations:
  - send
coverage:
- interface: signal_cli_jsonrpc
  categories:
    versions:
      status: gap
      gap: gap.signal.scope
    constraints:
      status: gap
      gap: gap.signal.scope
    formatting:
      status: gap
      gap: gap.signal.scope
    text_bindings:
      status: gap
      gap: gap.signal.scope
    images:
      status: gap
      gap: gap.signal.scope
    attachments:
      status: gap
      gap: gap.signal.scope
    addressing:
      status: gap
      gap: gap.signal.scope
    receipts:
      status: gap
      gap: gap.signal.scope
    attribution:
      status: gap
      gap: gap.signal.scope
    location:
      status: gap
      gap: gap.signal.scope
    expression:
      status: gap
      gap: gap.signal.scope
    interactivity:
      status: gap
      gap: gap.signal.scope
    delivery_controls:
      status: gap
      gap: gap.signal.scope
    eligibility:
      status: gap
      gap: gap.signal.scope
    rate_limits:
      status: gap
      gap: gap.signal.scope
    errors:
      status: gap
      gap: gap.signal.scope
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
- id: gap.signal.scope
  kind: research
  status: investigated
  question: Categories outside this fixture's subject are deliberately not researched.
  facts: []
  searches:
  - 'none: fixture scope'
  inspected_sources:
  - src.signal.docs
  unresolved_reason: The fixture exercises publication, not research.
  blocked_decision: None; fixtures are not research.
  next_investigation: None.
requires_messenger_update: false
---

# Signal fleet fixture

Publication fixture; not research.
