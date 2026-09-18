---
# Optional caller-supplied coordinates (subject unspecified), a shared place, a
# live location stream, a text/link fallback, and the separate author-geolocation
# answer. Criterion 18. Expect: valid.
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
  - send_location
  - send_venue
  - edit_message_live_location
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
      status: researched
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
location_bindings:
- id: loc.fx.coordinates
  interface: telegram_bot_api
  operation: send_location
  role: shared_place
  subject: unspecified
  mode: static
  association: caller_asserted
  origin: caller_supplied
  inclusion: when_supplied
  representation: dedicated_operation
  delivery: separate_message
  presentation: map
  fields:
  - &id002
    name: latitude
    locator: latitude
    required: true
    unit: degrees
  - &id003
    name: longitude
    locator: longitude
    required: true
    unit: degrees
  - &id004
    name: accuracy
    locator: horizontal_accuracy
    required: false
    unit: meters
  coexists_with: []
  knowledge:
    state: known
    evidence:
    - src.fx.docs
- id: loc.fx.venue
  interface: telegram_bot_api
  operation: send_venue
  role: shared_place
  subject: place
  mode: static
  association: caller_asserted
  origin: caller_supplied
  inclusion: when_supplied
  representation: dedicated_operation
  delivery: separate_message
  presentation: place_card
  fields:
  - *id002
  - *id003
  - *id004
  - name: label
    locator: title
    required: true
  - name: address
    locator: address
    required: true
  knowledge:
    state: known
    evidence:
    - src.fx.docs
- id: loc.fx.live
  interface: telegram_bot_api
  operation: send_location
  role: live_location
  subject: unspecified
  mode: live
  association: caller_asserted
  origin: caller_supplied
  inclusion: when_supplied
  representation: dedicated_operation
  delivery: separate_message
  presentation: map
  fields:
  - *id002
  - *id003
  - *id004
  - name: live_duration
    locator: live_period
    required: true
    unit: seconds
  live_behavior: Updated with edit_message_live_location until the period expires or it is stopped.
  knowledge:
    state: known
    evidence:
    - src.fx.docs
- id: loc.fx.text
  interface: telegram_bot_api
  operation: send_message
  role: shared_place
  subject: place
  mode: static
  association: caller_asserted
  origin: caller_supplied
  inclusion: when_supplied
  representation: text_fallback
  delivery: same_message
  presentation: link
  knowledge:
    state: known
    evidence:
    - src.fx.docs
author_geolocation:
- id: geo.fx.author
  interface: telegram_bot_api
  exposure: unavailable
  knowledge:
    state: known
    evidence:
    - src.fx.docs
    explanation: Sending coordinates does not answer whether the author's location is exposed; it is not.
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
