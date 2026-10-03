---
$schema: ../_schema.yaml
schema_revision: 5
provider: codex
created: 2026-09-08
last_updated: 2026-09-28
agent: codex
model: gpt-6-luna
reasoning_effort: high
versions_examined:
- 0.157.1
channels:
- id: app-server-stdio
  profile_ids:
  - managed-app-server
  os:
  - macos
  - linux
  - windows
  carrier: child_stdio
  envelope: json_rpc
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
    - field: /id
      test: present
    - field: /method
      test: absent
    success_when:
    - field: /result
      test: present
    error_field: /error/message
  setup:
  - id: initialize
    request:
      verb: write
      body:
        id: "<request_id>"
        method: initialize
        params:
          clientInfo:
            name: claudine
            title: Claudine
            version: "<claudine_version>"
          capabilities:
            experimentalApi: true
            requestAttestation: false
      awaits_response: true
    binds: provider_version
    field: /result/userAgent
    extract: first_version_number
  - id: initialized
    request:
      verb: write
      body:
        method: initialized
      awaits_response: false
    binds: nothing
    extract: not_applicable
  - id: thread-start
    request:
      verb: write
      body:
        id: "<request_id>"
        method: thread/start
        params:
          approvalPolicy: never
          cwd: "<directory>"
      awaits_response: true
    binds: conversation
    field: /result/thread/id
    extract: whole_value
  state:
    source: request_and_events
    request:
      verb: write
      body:
        id: "<request_id>"
        method: thread/read
        params:
          threadId: "<conversation>"
          includeTurns: false
      awaits_response: true
    working_when:
    - field: /result/thread/status/type
      test: equals
      value: active
    idle_when:
    - field: /result/thread/status/type
      test: equals
      value: idle
    conversation_field: /result/thread/id
    events:
    - match:
      - field: /method
        test: equals
        value: turn/started
      effect: working
      operation_field: /params/turn/id
      conversation_field: /params/threadId
    - match:
      - field: /method
        test: equals
        value: turn/completed
      effect: idle
      operation_field: /params/turn/id
      conversation_field: /params/threadId
      status_field: /params/turn/status
  provider_requests:
  - match:
    - field: /id
      test: present
    - field: /method
      test: equals
      value: mcpServer/elicitation/request
    meaning: cancel
    answer:
      id: "<provider_request_id>"
      result:
        action: cancel
        content: null
        _meta: null
  - match:
    - field: /id
      test: present
    - field: /method
      test: present
    meaning: refuse
    answer:
      id: "<provider_request_id>"
      error:
        code: -32000
        message: "`<provider_request_method>` is not supported in a managed Claudine run"
  evidence_ids:
  - official-app-server
mechanisms:
- id: app-server-steer
  channel_id: app-server-stdio
  interface_status: documented
  maturity: experimental
  operation_intent: steer_active_turn
  conversation_effect: preserve_running_turn
  delivery_boundary: end_of_tool_batch
  tool_batch_behavior: continue_all
  message_interpretation: provider_defined
  sender_message_id: supported
  retry_policy: unsafe_possible_duplicate
  target_guard: provider_checks_operation
  send:
    verb: write
    body:
      id: "<request_id>"
      method: turn/steer
      params:
        threadId: "<conversation>"
        expectedTurnId: "<operation>"
        input:
        - type: text
          text: "<message>"
          text_elements: []
        clientUserMessageId: "<message_id>"
    awaits_response: true
  acceptance:
    rule: correlated_success
    operation_echo_field: /result/turnId
    proves: queued
  stop_proof:
    rule: not_applicable
  startup_requirements:
  - explicit app-server or managed daemon
  - initialize/initialized handshake
  target_preconditions:
  - explicit app-server or managed daemon
  - registered reachable transport
  - initialize/initialized handshake
  - known loaded threadId
  - fresh exact expectedTurnId
  - experimental API capability where required by the installed protocol
  long_tool_behavior: "Observed in 0.157.1: while a tool runs the steer is answered at once and reaches the model\
    \ at the turn's next request, after every tool in the running batch finishes; the tool is not cancelled. While\
    \ the model is generating, the steer is also answered at once and reaches the model when that response ends;\
    \ the turn then continues with the steer even if the response was a final message. A pending approval after\
    \ the user message is accepted is execution state, not a delivery hold."
  queue_behavior: Same-turn pending input, drained in submission order at the turn's next model request (observed
    in 0.157.1); not the separate thread/queue next-turn queue.
  interruption_partial_failure: Not applicable.
  ordering: Submission ordering follows the app-server/core queue, but concurrent-client ordering and behavior when
    multiple steers race are not documented sufficiently for activation.
  duplicate_handling: clientUserMessageId provides correlation only. Observed in 0.157.1, two steers with the same
    clientUserMessageId were both delivered, so a resend is never safe.
  limits: Regular directly controlled active turns only; review/manual-compaction turns and parent-owned Multi-Agent
    V2 subagents reject steering. Requires exact active-turn identity and a managed app-server profile. Ingress
    overload returns JSON-RPC -32001 and is retryable with backoff, but retry safety for a possibly accepted request
    remains unproven.
  evidence_ids:
  - official-app-server
  - local-schema-0-153-4
  - final-race-inference
  - local-schema-0-157-1
  - live-app-server-0-157-1
- id: app-server-turn-start
  channel_id: app-server-stdio
  interface_status: documented
  maturity: experimental
  operation_intent: start_idle_turn
  conversation_effect: resume_same_conversation
  delivery_boundary: idle_turn_start
  tool_batch_behavior: not_applicable
  message_interpretation: provider_defined
  sender_message_id: supported
  retry_policy: unsafe_possible_duplicate
  target_guard: none
  send:
    verb: write
    body:
      id: "<request_id>"
      method: turn/start
      params:
        threadId: "<conversation>"
        input:
        - type: text
          text: "<message>"
          text_elements: []
        clientUserMessageId: "<message_id>"
    awaits_response: true
  acceptance:
    rule: correlated_success
    binds_operation_field: /result/turn/id
    proves: accepted
  stop_proof:
    rule: not_applicable
  startup_requirements:
  - managed app-server
  - initialized connection
  target_preconditions:
  - managed app-server
  - initialized connection
  - known threadId
  - confirmed idle thread
  long_tool_behavior: Not applicable when the idle precondition is true. Starting while another regular turn is
    active needs explicit provider behavior testing and must not be treated as steering.
  queue_behavior: Starts an idle turn; concurrent idle admission behavior is unverified.
  interruption_partial_failure: Not applicable.
  ordering: Each accepted new turn has its own ID; concurrent admission and idle-to-busy races require testing.
  duplicate_handling: Optional clientUserMessageId correlates a user item but is not documented as an idempotency
    key.
  limits: Idle-thread delivery only for this research classification; an ordinary completed exec process is not
    a reachable idle app-server.
  evidence_ids:
  - official-app-server
  - local-schema-0-153-4
  - live-app-server-0-157-1
- id: app-server-interrupt-then-start
  channel_id: app-server-stdio
  interface_status: documented
  maturity: experimental
  operation_intent: interrupt_then_submit
  conversation_effect: cancel_turn_same_conversation
  delivery_boundary: next_turn
  tool_batch_behavior: provider_defined
  message_interpretation: provider_defined
  sender_message_id: supported
  retry_policy: unsafe_possible_duplicate
  target_guard: provider_checks_operation
  send:
    verb: write
    body:
      id: "<request_id>"
      method: turn/start
      params:
        threadId: "<conversation>"
        input:
        - type: text
          text: "<message>"
          text_elements: []
        clientUserMessageId: "<message_id>"
    awaits_response: true
  acceptance:
    rule: correlated_success
    binds_operation_field: /result/turn/id
    proves: accepted
  cancel:
    verb: write
    body:
      id: "<request_id>"
      method: turn/interrupt
      params:
        threadId: "<conversation>"
        turnId: "<operation>"
    awaits_response: true
  stop_proof:
    rule: event
    match:
    - field: /method
      test: equals
      value: turn/completed
    - field: /params/turn/id
      test: equals
      value: "<operation>"
    - field: /params/turn/status
      test: equals
      value: interrupted
  startup_requirements: []
  target_preconditions:
  - manual approval
  - fresh exact active turn ID
  - observe matching turn/completed status interrupted
  - revalidate thread idle
  - submit replacement turn/start
  long_tool_behavior: Requests cancellation of the active turn. Background terminals may survive and require separate
    cleanup; command/tool cleanup effects need live verification.
  queue_behavior: No replacement is queued atomically; wait for interrupted completion, then separately start.
  interruption_partial_failure: Interruption may complete while replacement submission fails, leaving the same conversation
    idle; background terminals may remain and no rollback is promised.
  ordering: Wait for terminal interruption and revalidate idle state before replacement; final-turn and new-turn
    races can otherwise target the wrong lifecycle.
  duplicate_handling: Repeated interrupt or replacement behavior is not an idempotency contract; clientUserMessageId
    only correlates replacement input.
  limits: Manual fallback only; cannot satisfy non-interrupting automatic loop-warning requirements.
  evidence_ids:
  - official-app-server
  - final-race-inference
  - live-app-server-0-157-1
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
# Worked example: codex

This shows the `channels` and `mechanisms` sections of revision 5 filled in for
codex. The typed values are transcribed from the adapter that ships today, so they
are known to be correct. The other collections are left empty; they are not
research output. The `agent`, `model`, and `reasoning_effort` values are
placeholders that satisfy the contract: no research run produced this file.
