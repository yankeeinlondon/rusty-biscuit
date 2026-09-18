---
# Schema-pilot telegram record ported to schema version 1 (Phase 2 freeze gate).
# Not research: values come from the Phase 1 pilot. Categories the pilot did
# not exercise point at `gap.telegram.pilot_scope`.
$schema: ../../../../../docs/research/platforms/_schema.yaml
schema_version: 1
platform_id: telegram
created: 2026-09-17
last_updated: 2026-09-17
agent: claude-code
model: claude-opus-5
sources:
- id: src.telegram.bot_api
  kind: official_docs
  url: https://core.telegram.org/bots/api#messageentity
  locator: MessageEntity; sendPhoto caption
  retrieved: 2026-09-17
  note: spec spot-check
- id: src.telegram.prose
  kind: secondary
  location: messenger/docs/research/platforms/telegram.md
  locator: API Capabilities; Gotchas 1, 2, 4, 6
  retrieved: 2026-03-09
interfaces:
- interface_id: telegram_bot_api
  role: sending_adapter
  api_identity: Telegram Bot API
  endpoint_template: POST https://api.telegram.org/bot{token}/{method}
  api_version: '9.5'
  classification: official
  direction: bidirectional
  adapters:
  - telegram
  applies_when:
  - kind: hosting_mode
    equals: hosted
  operations:
  - send_document
  - send_media_group
  - send_message
  - send_photo
  - send_poll
coverage:
- interface: telegram_bot_api
  categories:
    versions:
      status: researched
    constraints:
      status: researched
    formatting:
      status: researched
    text_bindings:
      status: researched
    images:
      status: researched
    attachments:
      status: gap
      gap: gap.telegram.pilot_scope
    addressing:
      status: gap
      gap: gap.telegram.pilot_scope
    receipts:
      status: gap
      gap: gap.telegram.pilot_scope
    attribution:
      status: gap
      gap: gap.telegram.pilot_scope
    location:
      status: researched
    expression:
      status: gap
      gap: gap.telegram.pilot_scope
    interactivity:
      status: researched
    delivery_controls:
      status: gap
      gap: gap.telegram.pilot_scope
    eligibility:
      status: gap
      gap: gap.telegram.pilot_scope
    rate_limits:
      status: gap
      gap: gap.telegram.pilot_scope
    errors:
      status: researched
api_versions:
- id: ver.telegram.bot_api
  interface: telegram_bot_api
  versioning: versioned
  latest_stable: '9.5'
  previews: []
  knowledge:
    state: known
    evidence:
    - src.telegram.prose
    confidence: low
    gap: gap.telegram.versions
  subject: provider_api
chronology:
- id: chr.telegram.bot_api.9_5
  interface: telegram_bot_api
  subject: provider_api
  version: '9.5'
  stability: stable
  release_date_state: unknown
  knowledge:
    state: known
    evidence:
    - src.telegram.prose
    confidence: low
    explanation: Prose gives only 'March 2026'; month precision cannot be stored in a date field.
    gap: gap.telegram.versions
constraints:
- id: c.telegram.send_message.text
  interface: telegram_bot_api
  operation: send_message
  surface: body
  native_locator: text
  kind: hard_max
  value: 4096
  unit: unspecified_characters
  measurement_stage: unknown
  enforced_by: service
  overflow_behavior: unknown
  applies_when: []
  knowledge:
    state: known
    evidence:
    - src.telegram.prose
    confidence: low
    explanation: Prose says 1-4096 characters; a lower bound has no constraint kind; stage not established.
    gap: gap.telegram.text_stage
- id: c.telegram.caption
  interface: telegram_bot_api
  operation: send_photo
  surface: caption
  native_locator: caption
  kind: hard_max
  value: 1024
  unit: unspecified_characters
  measurement_stage: parsed_text
  enforced_by: service
  overflow_behavior: unknown
  applies_when:
  - kind: media_kind
    one_of:
    - photo
    - video
    - document
    - audio
  knowledge:
    state: known
    evidence:
    - src.telegram.bot_api
    confidence: medium
    explanation: Counted after entity parsing; the character unit itself is not established by the entity offset unit.
- id: c.telegram.media_group.count
  interface: telegram_bot_api
  operation: send_media_group
  surface: media_group
  native_locator: media
  kind: item_count_max
  value: 10
  unit: items
  measurement_stage: field_value
  enforced_by: service
  overflow_behavior: unknown
  applies_when: []
  knowledge:
    state: known
    evidence:
    - src.telegram.prose
    confidence: low
- id: c.telegram.upload.hosted
  interface: telegram_bot_api
  operation: send_document
  surface: attachments
  native_locator: document (multipart)
  kind: payload_bytes_max
  unit: bytes
  measurement_stage: serialized_payload
  enforced_by: service
  overflow_behavior: unknown
  applies_when:
  - kind: hosting_mode
    equals: hosted
  knowledge:
    state: unknown
    evidence:
    - src.telegram.prose
    confidence: low
    explanation: Prose says 50 MB; decimal versus binary megabytes is unresolved so no byte value is recorded.
    gap: gap.telegram.upload
- id: c.telegram.upload.local_server
  interface: telegram_bot_api
  operation: send_document
  surface: attachments
  native_locator: document (multipart)
  kind: payload_bytes_max
  unit: unknown
  measurement_stage: unknown
  overflow_behavior: unknown
  applies_when:
  - kind: hosting_mode
    equals: local_bot_api_server
  knowledge:
    state: unknown
    evidence:
    - src.telegram.prose
    confidence: low
    explanation: Prose says the local server 'removes these limits'; an undocumented bound is not unlimited.
    gap: gap.telegram.upload
- id: c.telegram.callback_data
  interface: telegram_bot_api
  operation: send_message
  surface: callback_data
  native_locator: reply_markup.inline_keyboard[][].callback_data
  kind: hard_max
  unit: utf8_bytes
  measurement_stage: field_value
  overflow_behavior: unknown
  applies_when: []
  knowledge:
    state: unknown
    evidence: []
    gap: gap.telegram.interactivity
format_profiles:
- id: fmt.telegram.plain
  family: plain_text
  malformed_behavior: literal
  constructs:
    emphasis:
      state: known
      support: unsupported
    strong:
      state: known
      support: unsupported
    strikethrough:
      state: known
      support: unsupported
    underline:
      state: known
      support: unsupported
    inline_code:
      state: known
      support: unsupported
    fenced_code:
      state: known
      support: unsupported
    links:
      state: unknown
      restriction: automatic link detection not researched
    images:
      state: known
      support: unsupported
    headings:
      state: known
      support: unsupported
    lists:
      state: known
      support: unsupported
    block_quotes:
      state: known
      support: unsupported
    tables:
      state: known
      support: unsupported
    spoilers:
      state: known
      support: unsupported
    mentions:
      state: unknown
  knowledge:
    state: known
    evidence:
    - src.telegram.prose
    confidence: low
  auto_interpretation: unknown
- id: fmt.telegram.html
  family: html
  dialect: Telegram Bot API HTML style (subset)
  escaping: Escape <, > and & as HTML entities.
  malformed_behavior: unknown
  constructs:
    emphasis:
      state: known
      support: supported
      syntax: <i>
    strong:
      state: known
      support: supported
      syntax: <b>
    strikethrough:
      state: unknown
    underline:
      state: unknown
    inline_code:
      state: known
      support: supported
      syntax: <code>
    fenced_code:
      state: unknown
    links:
      state: known
      support: supported
      syntax: <a href>
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
  knowledge:
    state: known
    evidence:
    - src.telegram.prose
    confidence: low
    gap: gap.telegram.formatting
  auto_interpretation: unknown
- id: fmt.telegram.markdown_v2
  family: provider_markup
  dialect: MarkdownV2
  escaping: 'Escape _ * [ ] ( ) ~ ` > # + - = | { } . ! outside entities.'
  malformed_behavior: unknown
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
  knowledge:
    state: known
    evidence:
    - src.telegram.prose
    confidence: low
    gap: gap.telegram.formatting
  auto_interpretation: unknown
text_bindings:
- id: tb.telegram.send_message.markup
  interface: telegram_bot_api
  operation: send_message
  surface: body
  native_locator: text
  representation: markup
  content_role: primary
  profiles:
  - fmt.telegram.plain
  - fmt.telegram.html
  - fmt.telegram.markdown_v2
  selector:
    field: parse_mode
    values:
    - HTML
    - MarkdownV2
    - Markdown
    default: '(absent: plain text)'
    default_state: documented
  relationships:
  - kind: mutually_exclusive
    target: tb.telegram.send_message.entities
  constraints:
  - c.telegram.send_message.text
  knowledge:
    state: known
    evidence:
    - src.telegram.prose
    confidence: low
  packaging: unknown
  visibility: visible_content
- id: tb.telegram.send_message.entities
  interface: telegram_bot_api
  operation: send_message
  surface: body
  native_locator: text + entities[]
  representation: entities
  content_role: primary
  profiles:
  - fmt.telegram.plain
  entity_offset_unit: utf16_code_units
  relationships:
  - kind: mutually_exclusive
    target: tb.telegram.send_message.markup
  constraints:
  - c.telegram.send_message.text
  knowledge:
    state: known
    evidence:
    - src.telegram.bot_api
    - src.telegram.prose
    confidence: medium
    explanation: Entity offsets are UTF-16 code units; this does not establish the text length unit.
  packaging: unknown
  visibility: visible_content
- id: tb.telegram.send_photo.caption
  interface: telegram_bot_api
  operation: send_photo
  surface: caption
  native_locator: caption
  representation: markup
  content_role: primary
  profiles:
  - fmt.telegram.plain
  - fmt.telegram.html
  - fmt.telegram.markdown_v2
  selector:
    field: parse_mode
    values:
    - HTML
    - MarkdownV2
    - Markdown
    default_state: unknown
  constraints:
  - c.telegram.caption
  knowledge:
    state: known
    evidence:
    - src.telegram.bot_api
    confidence: medium
  packaging: unknown
  visibility: visible_content
image_bindings:
- id: img.telegram.send_photo
  interface: telegram_bot_api
  operation: send_photo
  role: attachment
  fidelity: exact
  native_locator: photo
  placement: standalone_media
  placement_control: provider_selected
  submissions:
  - multipart_part
  - request_field
  sources:
  - upload_bytes
  - remote_url
  - provider_media_id
  collection: single
  caption_binding: tb.telegram.send_photo.caption
  supplied_by: caller_supplied
  knowledge:
    state: known
    evidence:
    - src.telegram.prose
    confidence: low
  multiple_images: unknown
  ordering: unknown
  atomic: unknown
  receipts: unknown
- id: img.telegram.media_group
  interface: telegram_bot_api
  operation: send_media_group
  role: attachment
  fidelity: exact
  native_locator: media[] (InputMediaPhoto)
  placement: standalone_media
  placement_control: provider_selected
  submissions:
  - multipart_part
  - request_field
  sources:
  - upload_bytes
  - remote_url
  - provider_media_id
  collection: album
  max_items: 10
  shared_constraints:
  - c.telegram.media_group.count
  supplied_by: caller_supplied
  knowledge:
    state: known
    evidence:
    - src.telegram.prose
    confidence: low
    explanation: Per-item versus shared caption and receipt count not researched.
    gap: gap.telegram.images
  multiple_images: unknown
  ordering: unknown
  atomic: unknown
  receipts: unknown
- id: img.telegram.link_preview
  interface: telegram_bot_api
  operation: send_message
  role: link_preview
  native_locator: link_preview_options
  placement: preview
  placement_control: provider_selected
  submissions:
  - request_field
  sources:
  - remote_url
  collection: provider_selected
  supplied_by: provider_derived
  knowledge:
    state: unknown
    evidence:
    - src.telegram.prose
    gap: gap.telegram.images
  fidelity: exact
  multiple_images: unknown
  ordering: unknown
  atomic: unknown
  receipts: unknown
role_coverage:
- interface: telegram_bot_api
  roles:
    inline_content:
      state: unknown
      bindings: []
      gap: gap.telegram.images
    attachment:
      state: known
      support: supported
      bindings:
      - img.telegram.send_photo
      - img.telegram.media_group
    primary:
      state: unknown
      bindings: []
      gap: gap.telegram.images
    thumbnail:
      state: unknown
      bindings: []
      gap: gap.telegram.images
    accessory:
      state: unknown
      bindings: []
      gap: gap.telegram.images
    author_icon:
      state: unknown
      bindings: []
      gap: gap.telegram.images
    footer_icon:
      state: unknown
      bindings: []
      gap: gap.telegram.images
    link_preview:
      state: unknown
      bindings:
      - img.telegram.link_preview
      gap: gap.telegram.images
attachment_bindings: []
addressing: []
receipts: []
attribution_bindings: []
location_bindings: []
author_geolocation:
- id: geo.telegram.bot_api.author
  interface: telegram_bot_api
  exposure: unknown
  knowledge:
    state: unknown
    evidence: []
    gap: gap.telegram.location
expression_bindings: []
delivery_controls: []
eligibility: []
rate_limits: []
envelopes:
- id: env.telegram.bot_api.json
  interface: telegram_bot_api
  operations:
  - send_message
  - send_photo
  - send_media_group
  - send_document
  origin: service
  body_format: json
  success_discriminator: /ok
  code_locator: /error_code
  message_locator: /description
  retry_after_locator: /parameters/retry_after
  retry_after_unit: seconds
  knowledge:
    state: known
    evidence:
    - src.telegram.prose
    confidence: low
    gap: gap.telegram.errors
errors:
- id: err.telegram.rate_limited
  interface: telegram_bot_api
  operations:
  - send_message
  - send_photo
  - send_media_group
  - send_document
  envelope: env.telegram.bot_api.json
  origin: service
  phase: response
  outcome: failure
  category: rate_limited
  match:
    http_status: 429
    native_code_number: 429
  delivery_certainty: rejected
  recovery: retry_candidate
  replay_safety: unknown
  remediation: wait
  remediation_note: Prose says the wait blocks every call for the bot, not one chat.
  diagnostic_fields:
  - error_code
  - parameters.retry_after
  fixtures:
  - fx.telegram.429
  - fx.telegram.unknown_409
  knowledge:
    state: known
    evidence:
    - src.telegram.prose
    confidence: low
- id: err.telegram.forbidden
  interface: telegram_bot_api
  operations:
  - send_message
  envelope: env.telegram.bot_api.json
  origin: service
  phase: response
  outcome: failure
  category: permission
  match:
    http_status: 403
    native_code_number: 403
  delivery_certainty: rejected
  recovery: do_not_retry
  replay_safety: unknown
  remediation: contact_operator
  remediation_note: Blocked bot, never-started chat and removed-from-group share this code; the description text is not an executable discriminator.
  diagnostic_fields:
  - error_code
  knowledge:
    state: known
    evidence:
    - src.telegram.prose
    confidence: low
error_fixtures:
- id: fx.telegram.429
  envelope: env.telegram.bot_api.json
  provenance: constructed_from_docs
  expect: match
  expected_error: err.telegram.rate_limited
  http_status: 429
  body: '{"ok":false,"error_code":429,"description":"Too Many Requests: retry after 35","parameters":{"retry_after":35}}'
  operation: send_message
- id: fx.telegram.unknown_409
  envelope: env.telegram.bot_api.json
  provenance: constructed_from_docs
  expect: no_match
  http_status: 409
  body: '{"ok":false,"error_code":409,"description":"Conflict"}'
  note: unknown code keeps an unknown classification
  operation: send_message
inbound_bindings:
- id: ib.telegram.get_updates
  interface: telegram_bot_api
  event: message update
  mechanism: long_poll
  reach:
  - direct
  - groups
  - mentions
  - replies
  - commands_only
  content: full_text
  prerequisites:
  - group privacy mode narrows group reach
  - bots never see other bots
  reuses_send_identity: 'yes'
  knowledge:
    state: known
    evidence:
    - src.telegram.prose
    confidence: low
    explanation: Reach is conditional on privacy mode; no typed condition kind exists for it.
- id: ib.telegram.webhook
  interface: telegram_bot_api
  event: message update
  mechanism: push_webhook
  reach:
  - direct
  - groups
  - mentions
  - replies
  - commands_only
  content: full_text
  prerequisites:
  - valid TLS certificate
  - port 443, 80, 88 or 8443
  - mutually exclusive with getUpdates
  reuses_send_identity: 'yes'
  knowledge:
    state: known
    evidence:
    - src.telegram.prose
    confidence: low
question_bindings:
- id: qb.telegram.inline_keyboard
  interface: telegram_bot_api
  kind: single_choice
  mechanism: buttons
  operation: send_message
  native_locator: reply_markup.inline_keyboard[][]
  answer_event: callback_query update
  answer_locator: /callback_query/data
  answer_type: option_id
  max_selections: 1
  correlation_locator: /callback_query/message/message_id
  opaque_state_constraint: c.telegram.callback_data
  validation: application
  visibility: unknown
  fidelity: exact
  knowledge:
    state: unknown
    evidence:
    - src.telegram.prose
    confidence: low
    gap: gap.telegram.interactivity
  native: native
  cancellation: unknown
  responses: attributable
- id: qb.telegram.poll
  interface: telegram_bot_api
  kind: multiple_choice
  mechanism: native_poll
  operation: send_poll
  native_locator: options
  answer_type: option_id_array
  validation: provider
  visibility: conditional
  fidelity: approximate
  knowledge:
    state: unknown
    evidence:
    - src.telegram.prose
    confidence: low
    explanation: Anonymous polls cannot yield attributable answers.
    gap: gap.telegram.interactivity
  native: native
  cancellation: unknown
  responses: aggregate_only
form_bindings:
- id: fb.telegram.sequential
  interface: telegram_bot_api
  container: sequential_application
  question_kinds:
  - single_choice
  - text_input
  submission: sequential_application
  knowledge:
    state: unknown
    evidence: []
    gap: gap.telegram.interactivity
  lifecycle:
    requires_user_action: unknown
  absent_vs_empty: unknown
  cancellation: unknown
  validation: unknown
  navigation: unknown
interaction_fixtures: []
changes:
- id: chg.telegram.pilot
  kind: added
  facts:
  - c.telegram.caption
  - tb.telegram.send_message.entities
  summary: Pilot records only.
gaps:
- id: gap.telegram.versions
  status: open
  question: What is the exact release date of Bot API 9.5?
  facts:
  - chr.telegram.bot_api.9_5
  next_investigation: Read the Bot API changelog.
  kind: research
- id: gap.telegram.text_stage
  status: open
  question: Is the 4096 text limit measured before or after entity parsing?
  facts:
  - c.telegram.send_message.text
  next_investigation: Read the sendMessage reference.
  kind: research
- id: gap.telegram.upload
  status: open
  question: What are the exact byte limits for hosted and local Bot API servers?
  facts:
  - c.telegram.upload.hosted
  - c.telegram.upload.local_server
  next_investigation: Read the local Bot API server documentation.
  kind: research
- id: gap.telegram.formatting
  status: open
  question: Which constructs do HTML and MarkdownV2 support?
  facts:
  - fmt.telegram.html
  - fmt.telegram.markdown_v2
  next_investigation: Read the formatting options section.
  kind: research
- id: gap.telegram.images
  status: open
  question: How are media-group captions and link-preview images controlled?
  facts:
  - img.telegram.media_group
  - img.telegram.link_preview
  next_investigation: Read sendMediaGroup and LinkPreviewOptions.
  kind: research
- id: gap.telegram.errors
  status: open
  question: Does every failure use the ok/error_code/description envelope?
  facts:
  - env.telegram.bot_api.json
  next_investigation: Read the Making requests section.
  kind: research
- id: gap.telegram.location
  status: open
  question: Is author geolocation ever attached automatically?
  facts:
  - cap.telegram.location.author_geolocation
  next_investigation: Read sendLocation and Message.location.
  kind: research
- id: gap.telegram.interactivity
  status: open
  question: What are callback_data limits and callback answer deadlines?
  facts:
  - c.telegram.callback_data
  - qb.telegram.inline_keyboard
  - qb.telegram.poll
  - fb.telegram.sequential
  next_investigation: Read InlineKeyboardButton and answerCallbackQuery.
  kind: research
- id: gap.telegram.pilot_scope
  kind: research
  status: open
  question: Categories outside the schema pilot's scope have not been researched for this platform.
  facts: []
  next_investigation: Research in the Phase 7 baseline; this fixture only exercises the contract.
requires_messenger_update: false
---

# Telegram (schema pilot, v1 port)

Pilot record only; not research. It contrasts a caption bound measured on
`parsed_text` with an entity representation indexed in `utf16_code_units`, a
parse-mode selector that is mutually exclusive with pre-parsed entities, an
album collection with a shared count budget, hosting-mode conditions, and a
structured JSON error envelope carried in both the HTTP status and `error_code`.
