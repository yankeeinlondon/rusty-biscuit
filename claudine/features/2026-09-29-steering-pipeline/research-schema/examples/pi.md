---
$schema: ../_schema.yaml
schema_revision: 5
provider: pi
created: 2026-09-08
last_updated: 2026-09-08
agent: codex
model: gpt-6-luna
reasoning_effort: high
versions_examined:
- 0.84.4
channels:
- id: rpc-stdio
  profile_ids:
  - retained-rpc
  os:
  - macos
  - linux
  - windows
  carrier: child_stdio
  envelope: plain_json
  events: same_channel
  endpoint:
    source: child_pipes
  credential:
    kind: channel_ownership
    source: not_applicable
    handoff: not_applicable
  correlation:
    request_id_field: /id
    response_id_field: /id
    response_when:
    - field: /type
      test: equals
      value: response
    success_when:
    - field: /success
      test: equals
      value: true
    error_field: /error
  setup: []
  state:
    source: request_and_events
    request:
      verb: write
      body:
        id: "<request_id>"
        type: get_state
      awaits_response: true
    working_when:
    - field: /data/isStreaming
      test: equals
      value: true
    idle_when:
    - field: /data/isStreaming
      test: equals
      value: false
    - field: /data/isCompacting
      test: equals
      value: false
    - field: /data/pendingMessageCount
      test: equals
      value: 0
    conversation_field: /data/sessionId
    events:
    - match:
      - field: /type
        test: equals
        value: agent_start
      effect: working
    - match:
      - field: /type
        test: equals
        value: agent_settled
      effect: idle
  provider_requests:
  - match:
    - field: /type
      test: equals
      value: extension_ui_request
    meaning: cancel
    answer:
      type: extension_ui_response
      id: "<provider_request_id>"
      cancelled: true
  evidence_ids:
  - live-rpc-fixture-0844
mechanisms:
- id: rpc-steer
  channel_id: rpc-stdio
  interface_status: documented
  maturity: stable
  operation_intent: steer_active_turn
  conversation_effect: preserve_running_turn
  delivery_boundary: end_of_tool_batch
  tool_batch_behavior: continue_all
  message_interpretation: skills_templates
  sender_message_id: unsupported
  retry_policy: never_retry
  target_guard: none
  send:
    verb: write
    body:
      id: "<request_id>"
      type: steer
      message: "<message>"
    awaits_response: true
  acceptance:
    rule: correlated_success
    proves: queued
  stop_proof:
    rule: not_applicable
  startup_requirements:
  - retained pipes
  - registered expected session
  target_preconditions:
  - expected session
  - streaming
  - not compacting
  long_tool_behavior: Waits through long tool and entire batch; hangs indefinitely if boundary never arrives.
  queue_behavior: In-memory FIFO; all or one-at-a-time drain; clear_queue removes pending text.
  interruption_partial_failure: Not applicable.
  ordering: FIFO; concurrent sender ordering untested.
  duplicate_handling: No suppression; request id is correlation, not idempotency.
  limits: No established size/count/expiry/persistence bounds; extension commands rejected.
  evidence_ids:
  - official-rpc
  - source-rpc
  - source-session
  - source-loop
  - live-rpc-switch-crash-0844
- id: rpc-idle-prompt
  channel_id: rpc-stdio
  interface_status: documented
  maturity: stable
  operation_intent: start_idle_turn
  conversation_effect: resume_same_conversation
  delivery_boundary: idle_turn_start
  tool_batch_behavior: not_applicable
  message_interpretation: input_extension
  sender_message_id: unsupported
  retry_policy: never_retry
  target_guard: none
  send:
    verb: write
    body:
      id: "<request_id>"
      type: prompt
      message: "<message>"
    awaits_response: true
  acceptance:
    rule: correlated_success
    proves: accepted
  stop_proof:
    rule: not_applicable
  startup_requirements:
  - retained RPC
  - stdout reader
  target_preconditions:
  - expected session
  - isStreaming false
  - not compacting
  long_tool_behavior: Not applicable if idle.
  queue_behavior: Starts a turn; streamingBehavior can delegate to steer/follow-up.
  interruption_partial_failure: Not applicable.
  ordering: Concurrent idle submissions unverified.
  duplicate_handling: No idempotency.
  limits: Extensions may handle/transform; skills/templates expand; size unknown.
  evidence_ids:
  - official-rpc
  - source-rpc
  - source-session
- id: rpc-abort-submit
  channel_id: rpc-stdio
  interface_status: documented
  maturity: stable
  operation_intent: interrupt_then_submit
  conversation_effect: cancel_turn_same_conversation
  delivery_boundary: next_turn
  tool_batch_behavior: stop_remaining
  message_interpretation: input_extension
  sender_message_id: unsupported
  retry_policy: never_retry
  target_guard: none
  send:
    verb: write
    body:
      id: "<request_id>"
      type: prompt
      message: "<message>"
    awaits_response: true
  acceptance:
    rule: correlated_success
    proves: accepted
  cancel:
    verb: write
    body:
      id: "<request_id>"
      type: abort
    awaits_response: true
  stop_proof:
    rule: poll_state_idle
  startup_requirements:
  - manual approval
  - fresh state before both phases
  target_preconditions:
  - expected working session
  - same idle session before replacement
  long_tool_behavior: Abort signals current run; macOS 0.84.4 fixture confirms one built-in bash external process
    stops. Provider kill leaves that process alive; other trees/platforms unverified.
  queue_behavior: Abort retains queues; clear only under explicit policy.
  interruption_partial_failure: Abort may succeed while prompt fails; original stays canceled and cleared queues
    stay cleared.
  ordering: Never pipeline phases.
  duplicate_handling: No idempotency.
  limits: Manual only; forbidden for automatic warnings.
  evidence_ids:
  - official-rpc
  - source-rpc
  - source-session
  - live-rpc-bash-cleanup-0844
evidence: []
interface_inventory: []
launch_profiles: []
discovery: []
discovery_gaps: []
access_findings: []
delivery_states: []
receipt_guarantees: []
receipt_observations: []
compatibility: []
verification: []
cases: []
gaps: []
changes:
- "Worked example: channels and mechanisms transcribed from the shipped adapter."
requires_claudine_update: false
reason: Worked example for the draft contract; not research output.
---
# Worked example: pi

This shows the `channels` and `mechanisms` sections of revision 5 filled in for
pi. The typed values are transcribed from the adapter that ships today, so they
are known to be correct. The other collections are left empty; they are not
research output. The `agent`, `model`, and `reasoning_effort` values are
placeholders that satisfy the contract: no research run produced this file.
