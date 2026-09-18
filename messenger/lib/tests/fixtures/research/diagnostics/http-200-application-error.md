---
# An application failure inside an HTTP 200 response, an unknown code in the
# same envelope, and a successful send carrying a warning (accepted, not
# failed). Criteria 12, 13. Expect: valid.
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
envelopes:
- id: env.fx.slack
  interface: slack_web_api
  operations:
  - chat_post_message
  origin: service
  body_format: json
  success_discriminator: /ok
  code_locator: /error
  warnings_locator: /response_metadata/warnings
  retry_after_locator: header:Retry-After
  retry_after_unit: seconds
  knowledge:
    state: known
    evidence:
    - src.fx.docs
errors:
- id: err.fx.channel_not_found
  interface: slack_web_api
  operations:
  - chat_post_message
  envelope: env.fx.slack
  origin: service
  phase: response
  outcome: failure
  category: invalid_destination
  match:
    http_status: 200
    discriminator_locator: /ok
    discriminator_equals: 'false'
    native_code_string: channel_not_found
  diagnostic_fields:
  - /error
  fixtures:
  - fx.fx.http200
  - fx.fx.http200_other
  delivery_certainty: rejected
  recovery: after_correction
  replay_safety: unknown
  remediation: correct_destination
  knowledge:
    state: known
    evidence:
    - src.fx.docs
- id: err.fx.warning
  interface: slack_web_api
  operations:
  - chat_post_message
  envelope: env.fx.slack
  origin: service
  phase: response
  outcome: warning
  category: invalid_content
  match:
    http_status: 200
    discriminator_locator: /ok
    discriminator_equals: 'true'
    native_code_string: missing_charset
  diagnostic_fields:
  - /response_metadata/warnings
  fixtures:
  - fx.fx.warning
  remediation_note: The send succeeded; the warning describes a request-quality issue.
  delivery_certainty: accepted
  recovery: do_not_retry
  replay_safety: unsafe
  remediation: change_format
  knowledge:
    state: known
    evidence:
    - src.fx.docs
error_fixtures:
- id: fx.fx.http200
  envelope: env.fx.slack
  operation: chat_post_message
  provenance: constructed_from_docs
  expect: match
  expected_error: err.fx.channel_not_found
  http_status: 200
  body: '{"error": "channel_not_found", "ok": false}'
- id: fx.fx.http200_other
  envelope: env.fx.slack
  operation: chat_post_message
  provenance: constructed_from_docs
  expect: unknown
  http_status: 200
  body: '{"error": "some_new_code", "ok": false}'
  note: 'Unknown code inside an HTTP-200 failure: classification stays unknown.'
- id: fx.fx.warning
  envelope: env.fx.slack
  operation: chat_post_message
  provenance: constructed_from_docs
  expect: match
  expected_error: err.fx.warning
  http_status: 200
  body: '{"ok": true, "response_metadata": {"warnings": ["missing_charset"]}, "ts": "TS"}'
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
