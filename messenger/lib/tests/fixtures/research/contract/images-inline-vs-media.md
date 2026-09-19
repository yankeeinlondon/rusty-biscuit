---
# Inline versus standalone media, several submission mechanisms for one slot,
# exact versus approximate role mappings, and explicit unsupported roles.
# Criterion 16. Expect: valid.
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
  - send_photo
  - send_media_group
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
      status: researched
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
      status: gap
      gap: gap.fx.scope
api_versions: []
chronology: []
constraints: []
format_profiles: []
text_bindings: []
image_bindings:
- id: ib.fx.photo
  interface: telegram_bot_api
  operation: send_photo
  role: attachment
  fidelity: exact
  native_locator: photo
  placement: standalone_media
  placement_control: explicit
  submissions:
  - multipart_part
  - request_field
  sources:
  - upload_bytes
  - remote_url
  - provider_media_id
  collection: single
  multiple_images: not_supported
  ordering: unknown
  atomic: unknown
  receipts: single
  supplied_by: caller_supplied
  caption_scope: per_item
  mime_types:
  - image/jpeg
  - image/png
  transformations: Recompressed by the service.
  knowledge:
    state: known
    evidence:
    - src.fx.docs
- id: ib.fx.inline_link
  interface: telegram_bot_api
  operation: send_message
  role: inline_content
  fidelity: approximate
  native_locator: text (HTML <a> to image)
  placement: inline_anchor
  placement_control: explicit
  submissions:
  - markup_reference
  sources:
  - remote_url
  collection: single
  multiple_images: not_supported
  ordering: unknown
  atomic: unknown
  receipts: single
  supplied_by: caller_supplied
  knowledge:
    state: known
    evidence:
    - src.fx.docs
role_coverage:
- interface: telegram_bot_api
  roles:
    inline_content:
      state: known
      support: conditional
      fidelity: approximate
      bindings:
      - ib.fx.inline_link
    attachment:
      state: known
      support: supported
      fidelity: exact
      bindings:
      - ib.fx.photo
    primary:
      state: unknown
      bindings: []
      gap: gap.fx.scope
    thumbnail:
      state: unknown
      bindings: []
      gap: gap.fx.scope
    accessory:
      state: unknown
      bindings: []
      gap: gap.fx.scope
    author_icon: &id002
      state: known
      support: unsupported
      bindings: []
    footer_icon: *id002
    link_preview:
      state: unknown
      bindings: []
      gap: gap.fx.scope
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
