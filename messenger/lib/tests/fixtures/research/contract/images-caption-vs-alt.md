---
# Captions, alt text, and filenames are separate bindings; one message
# combines a primary slot, a thumbnail, and an ordered media list. Criterion 16.
# Expect: valid.
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
      status: researched
    text_bindings:
      status: researched
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
format_profiles:
- id: fmt.fx.plain
  family: plain_text
  constructs:
    emphasis:
      state: unknown
    strong:
      state: unknown
    strikethrough:
      state: unknown
    underline:
      state: unknown
    inline_code:
      state: unknown
    fenced_code:
      state: unknown
    links:
      state: unknown
    images:
      state: unknown
    headings:
      state: unknown
    lists:
      state: unknown
    block_quotes:
      state: unknown
    tables:
      state: unknown
    spoilers:
      state: unknown
    mentions:
      state: unknown
  auto_interpretation: unknown
  malformed_behavior: unknown
  knowledge:
    state: known
    evidence:
    - src.fx.docs
text_bindings:
- id: tb.fx.alt
  interface: discord_bot_api
  operation: create_message
  surface: alt_text
  native_locator: attachments[].description
  representation: text
  content_role: accessibility_fallback
  packaging: plain_only
  visibility: accessibility_only
  profiles:
  - fmt.fx.plain
  knowledge:
    state: known
    evidence:
    - src.fx.docs
- id: tb.fx.caption
  interface: discord_bot_api
  operation: create_message
  surface: body
  native_locator: content
  representation: text
  content_role: primary
  packaging: plain_only
  visibility: visible_content
  profiles:
  - fmt.fx.plain
  knowledge:
    state: known
    evidence:
    - src.fx.docs
image_bindings:
- id: ib.fx.file
  interface: discord_bot_api
  operation: create_message
  role: attachment
  fidelity: exact
  native_locator: files[n]
  placement: standalone_media
  placement_control: explicit
  submissions:
  - multipart_part
  sources:
  - upload_bytes
  collection: ordered_list
  multiple_images: repeated_objects
  ordering: 'yes'
  atomic: unknown
  receipts: single
  supplied_by: caller_supplied
  caption_binding: tb.fx.caption
  caption_scope: shared
  alt_text_binding: tb.fx.alt
  knowledge:
    state: known
    evidence:
    - src.fx.docs
- id: ib.fx.embed_image
  interface: discord_bot_api
  operation: create_message
  role: primary
  fidelity: exact
  native_locator: embeds[].image.url
  placement: card_main
  placement_control: explicit
  submissions:
  - request_field
  - upload_then_reference
  sources:
  - remote_url
  - upload_bytes
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
- id: ib.fx.embed_thumb
  interface: discord_bot_api
  operation: create_message
  role: thumbnail
  fidelity: exact
  native_locator: embeds[].thumbnail.url
  placement: card_thumbnail
  placement_control: explicit
  submissions:
  - request_field
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
- interface: discord_bot_api
  roles:
    inline_content:
      state: unknown
      bindings: []
      gap: gap.fx.scope
    attachment:
      state: known
      support: supported
      fidelity: exact
      bindings:
      - ib.fx.file
    primary:
      state: known
      support: supported
      fidelity: exact
      bindings:
      - ib.fx.embed_image
    thumbnail:
      state: known
      support: supported
      fidelity: exact
      bindings:
      - ib.fx.embed_thumb
    accessory:
      state: unknown
      bindings: []
      gap: gap.fx.scope
    author_icon:
      state: unknown
      bindings: []
      gap: gap.fx.scope
    footer_icon:
      state: unknown
      bindings: []
      gap: gap.fx.scope
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
