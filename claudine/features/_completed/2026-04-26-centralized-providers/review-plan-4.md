---
ready: false
agent: ""
model: ""
review: review-4.md
---

# Centralized Providers — Review 4 Closure Plan

This plan addresses every finding in
[`review-4.md`](./review-4.md). The three findings are largely independent
and can be implemented in any order, so the plan ships as **two
implementation phases plus a final verification gate**:

- Phase 1 bundles findings 1 and 2 (the two production-code gaps). Both
  touch `claudine/lib/src/provider/`, so co-locating them keeps the test
  changes in a single test module and avoids a redundant intermediate
  green build.
- Phase 2 handles finding 3 (a docs-only fix to
  `building-an-agent-wrapper.md`).
- Phase 3 is the verification gate — full `cargo test` and
  `cargo clippy -D warnings` across the claudine crates.

The findings ranked by severity:

| # | Severity | Issue |
|---|---|---|
| 1 | High | `providers --describe --format json` omits `capabilities` and `resource_support` because the accessors are `#[serde(skip)]` `fn` pointers. The serializable surface is the central feature deliverable, so this is a functional gap. |
| 2 | Medium | `Provider::detect_from_payload` (`provider/methods.rs:141`) still hard-codes per-provider shape checks instead of delegating to `AdapterBehavior::detect` overrides. The behavior trait is the documented dispatch surface. |
| 3 | Low | `building-an-agent-wrapper.md:190` claims Phase 8 removed the `AgentId` alias and `events::Provider` re-export. They were restored by review-plan-3 and the doc must reflect that. |

Phase mapping:

| Phase | Closes | Description |
|---|---|---|
| 1 | Issues 1 + 2 | Make `capabilities` / `resource_support` part of the serializable JSON surface and move per-provider payload detection into `AdapterBehavior::detect` overrides. Strengthen tests to lock both surfaces. |
| 2 | Issue 3 | Update `building-an-agent-wrapper.md:190` to describe the post-Phase-8 cleanup release window. |
| 3 | All | Final verification gate — `cargo test` and `cargo clippy -D warnings` across the claudine package areas. |

## Assumptions

- `claudine/lib` and `claudine/cli` are the in-repo claudine crates; the
  final lint/test gate covers both. `cargo metadata --no-deps
  --format-version 1` is the source of truth at gate time.
- `AgentCapabilities` and `ProviderCapabilities` already implement
  `Serialize` (every nested struct in `agents/model.rs` and
  `linking/capabilities.rs` derives `Serialize` for the existing matrix
  output). Phase 1 verifies this assumption with a serialization smoke
  test before touching `ProviderInfo`.
- `provider/methods.rs` `detect_from_payload` may still need a small
  cross-cutting tie-breaker for ambiguous payloads (e.g. `hook_event_name`
  shared by Claude/Gemini), but it must not contain provider-specific
  shape checks. The Phase 1 redesign moves the per-provider rules into
  the relevant `AdapterBehavior::detect` overrides and leaves a
  data-driven tie-breaker behind, guarded by an updated drift test.
- Existing `provider::tests::detect_from_payload_recognizes_known_shapes`
  asserts the public detection surface; new tests added in Phase 1 must
  preserve every assertion in that test.
- `clippy` baseline across claudine crates is currently zero-warning per
  review-plan-3 Phase 4. Any new warnings introduced in this plan must be
  fixed before Phase 3 closes.

---

## Phase 1 — Serialize the full catalog and route detection through `AdapterBehavior`

### Scope (Closes Issues 1 + 2)

Two changes to `claudine/lib/src/provider/`:

1. **Issue 1 — full catalog serialization.** Replace the `#[serde(skip)]`
   on `agent_capabilities_fn` and `resource_support_fn`
   (`claudine/lib/src/provider/mod.rs:150`,
   `claudine/lib/src/provider/mod.rs:168`) with a serializable surface so
   `claudine providers --describe --format json` round-trips
   `capabilities` and `resource_support`. Two acceptable shapes (Phase 1
   selects the first):
     - **Selected:** add two `#[serde(serialize_with = "...")]` fn-pointer
       fields that call the accessor and serialize the resulting
       `&'static AgentCapabilities` /
       `&'static ProviderCapabilities`. Keeps the existing accessor-based
       data flow.
     - Alternative (rejected for Phase 1): introduce a `ProviderInfoView`
       DTO assembled in `cli/src/commands/providers.rs::run_describe`.
       Rejected because it duplicates the catalog source of truth and
       leaves a runtime translation step between `ProviderInfo` and the
       JSON output.
2. **Issue 2 — per-provider detection.** Move the cross-cutting payload
   shape checks out of
   `Provider::detect_from_payload` (`claudine/lib/src/provider/methods.rs:141`)
   and into per-provider `AdapterBehavior::detect`
   (`claudine/lib/src/provider/behavior.rs:142`) overrides. The post-fix
   `Provider::detect_from_payload` body must contain only the
   `for provider in PROVIDERS_DISPLAY_ORDER` walk; no `Provider::Gemini`,
   `Provider::Claude`, `Provider::Codex`, `Provider::OpenCode`,
   `Provider::KimiCode` literals; no `looks_like_codex_payload` helper.

### Tasks

#### 1.1 — Confirm serializability of catalog data (probe)

Add a single-shot test (or `dbg!` walkthrough; not committed) to
`provider/tests.rs` that calls
`serde_json::to_value(provider_info(p).agent_capabilities())` and
`serde_json::to_value(provider_info(p).resource_support())` for every
provider in `PROVIDERS_DISPLAY_ORDER`. Confirms no
`Serialize`-not-implemented errors before the schema changes land.

If any nested struct does not derive `Serialize` (e.g. `AgentDocs` or
`SubagentCapabilities`), add the derive at the original site
(`claudine/lib/src/agents/*.rs` /
`claudine/lib/src/linking/capabilities.rs`). Each derive must be paired
with a matching unit test in the same file confirming the new derive
output is well-formed.

The probe test is deleted before phase closes — it exists only to surface
missing derives.

**Files:**
- `claudine/lib/src/provider/tests.rs`
- (potentially) `claudine/lib/src/agents/model.rs`
- (potentially) `claudine/lib/src/agents/*.rs` (per-provider files
  consumed by the probe; only if a derive is missing)
- (potentially) `claudine/lib/src/linking/capabilities.rs`

**Tests:** local probe (deleted before phase closes) + any newly added
`Serialize`-derive smoke tests next to the new derives.

**Acceptance criteria:**
- Probe runs green with no panics or compile errors for every
  `PROVIDERS_DISPLAY_ORDER` variant.
- If derives were added: every new `Serialize` derive has a paired unit
  test.

#### 1.2 — Wire `capabilities` and `resource_support` into the JSON surface

Replace the two `#[serde(skip)]` accessors on
`ProviderInfo` (`claudine/lib/src/provider/mod.rs:150` and `mod.rs:168`)
with `serialize_with`-driven fn-pointer fields:

```rust
#[serde(serialize_with = "serialize_agent_capabilities")]
pub agent_capabilities_fn: fn() -> &'static AgentCapabilities,

#[serde(serialize_with = "serialize_resource_support")]
pub resource_support_fn: fn() -> &'static ProviderCapabilities,
```

Implement the helpers at the top of
`claudine/lib/src/provider/mod.rs`. They call the fn pointer and forward
the static reference to the supplied serializer. Rename the JSON keys via
`#[serde(rename = "capabilities")]` and
`#[serde(rename = "resource_support")]` so the JSON shape matches the
spec (spec §"Central type: ProviderInfo (data) + ProviderBehavior
(trait)" describes `capabilities` and `resource_support` as the catalog
field names).

If a wrapping helper is needed because `serialize_with` cannot easily
deref a fn-pointer, define a small private `CapabilitiesView` /
`ResourceSupportView` `serde::Serialize` proxy struct at the top of
`provider/mod.rs`.

**Files:**
- `claudine/lib/src/provider/mod.rs:150` (replace `#[serde(skip)]`)
- `claudine/lib/src/provider/mod.rs:168` (replace `#[serde(skip)]`)

**Tests added in this task** (new tests, all in
`claudine/lib/src/provider/tests.rs`):

- `provider_info_json_includes_capabilities_and_resource_support`:
  walks `PROVIDERS_DISPLAY_ORDER`, deserializes
  `serde_json::to_value(provider_info(p))`, asserts that
  `json["capabilities"].is_object()` and
  `json["resource_support"].is_object()` for every provider.
- `provider_info_json_capabilities_includes_nested_runtime_data`:
  asserts that the serialized `capabilities` for `Provider::Claude`
  contains representative non-interactive capabilities — at minimum
  `capabilities.runtime.non_interactive.entrypoints` is a non-empty
  array, `capabilities.runtime.non_interactive.output_formats` is a
  non-empty array, and
  `capabilities.runtime.non_interactive.structured_output_supported` is
  `true`.
- `provider_info_json_resource_support_includes_skills_commands_agents_scripts`:
  asserts that the serialized `resource_support` for `Provider::Claude`
  contains `skills`, `commands`, `agents`, and `scripts` as objects with
  a `level` string key (the existing `ResourceSupport` shape).

These three tests close the test-coverage gap called out by review-4
("only assert a small typed subset exists, not that the full catalog
exists").

**CLI-side coverage** (also in this task) — add to
`claudine/cli/src/commands/providers.rs::tests::describe_json_serializes_all_providers`:
extend the loop's required-key list to include
`"capabilities"` and `"resource_support"`. The existing test loops over
every provider; the new keys are added alongside the existing
`"event_mapping"`, `"system_prompt"`, etc. checks.

**Acceptance criteria:**
- Every provider's JSON surface now contains `capabilities` and
  `resource_support` objects.
- All three new lib tests pass.
- The amended CLI test passes.
- The existing `provider_info_serializes_round_trip` test still passes
  (the negative key-list still excludes `behavior`, `mcp`, `adapter`,
  `configurator`).

#### 1.3 — Move per-provider detection into `AdapterBehavior::detect`

For each provider whose payload shape is currently hard-coded in
`Provider::detect_from_payload`
(`claudine/lib/src/provider/methods.rs:141`), add an
`AdapterBehavior::detect` override on the provider behavior implementor.
The detect logic moves out of the central function and into the
provider's own module:

| Provider | Behavior impl file | Shape rule moved out of `methods.rs` |
|---|---|---|
| Claude | `claudine/lib/src/provider/claude.rs` | `hook_event_name` matches `claude_table.event_from_native_name(name)` |
| Gemini | `claudine/lib/src/provider/gemini.rs` | `hook_event_name` matches `gemini_table.event_from_native_name(name)` AND not Claude; or `event_name` field present |
| Codex | `claudine/lib/src/provider/codex.rs` | `looks_like_codex_payload(raw)` (move helper into `provider/codex.rs` as a module-private fn) |
| OpenCode | `claudine/lib/src/provider/opencode.rs` | `event_type` or `eventType` field present |
| KimiCode | `claudine/lib/src/provider/kimi.rs` | `method` field present |

Each provider's `AdapterBehavior::detect(&self, raw: &Value) -> bool`
returns `true` iff its shape rule matches, otherwise `false`. The
implementations consult `provider_info(Provider::X).event_mapping`
directly when needed (Claude/Gemini use their own event tables).

Rewrite `Provider::detect_from_payload` in
`claudine/lib/src/provider/methods.rs:141` to:

```rust
pub fn detect_from_payload(raw: &Value) -> Option<Self> {
    PROVIDERS_DISPLAY_ORDER
        .into_iter()
        .find(|provider| provider_info(*provider).adapter.detect(raw))
}
```

No fallback `match`. No `Provider::Foo` literals. No
`looks_like_codex_payload` helper. The post-fix function must be a pure
walk of `PROVIDERS_DISPLAY_ORDER`.

**Cross-cutting tie-breaker.** The current implementation handles two
sequences where the same `hook_event_name` could match both Claude and
Gemini (Claude takes precedence unless the name is exclusively Gemini's).
That ordering is now expressed by `PROVIDERS_DISPLAY_ORDER`: Claude
appears before Gemini in display order, so Claude's `detect` returning
`true` short-circuits the walk. Gemini's `detect` must guard against
names that Claude also recognizes (assert
`!claude_table.event_from_native_name(name).is_some()` before claiming
the payload).

**Files:**
- `claudine/lib/src/provider/methods.rs:141` — rewrite
  `detect_from_payload` body
- `claudine/lib/src/provider/methods.rs:245-276` — delete
  `looks_like_codex_payload` helper (moved into `provider/codex.rs`)
- `claudine/lib/src/provider/claude.rs` — add `detect` override on
  `impl AdapterBehavior for ClaudeProvider`
- `claudine/lib/src/provider/codex.rs` — add `detect` override
- `claudine/lib/src/provider/gemini.rs` — add `detect` override
- `claudine/lib/src/provider/kimi.rs` — add `detect` override
- `claudine/lib/src/provider/opencode.rs` — add `detect` override

**Tests added** (in `claudine/lib/src/provider/tests.rs`):

- `adapter_detects_known_claude_payloads`:
  `provider_info(Provider::Claude).adapter.detect(...)` returns `true`
  for `{"hook_event_name":"Stop"}`,
  `{"hook_event_name":"PreToolUse"}`,
  `{"hook_event_name":"UserPromptSubmit"}`,
  `{"hook_event_name":"SessionStart"}`,
  `{"hook_event_name":"Notification"}`. Returns `false` for
  Gemini-only names (`{"hook_event_name":"BeforeAgent"}`).
- `adapter_detects_known_gemini_payloads`:
  `provider_info(Provider::Gemini).adapter.detect(...)` returns `true`
  for `{"hook_event_name":"BeforeAgent"}`,
  `{"hook_event_name":"AfterAgent"}`,
  `{"hook_event_name":"BeforeModel"}`,
  `{"hook_event_name":"BeforeTool"}`,
  `{"hook_event_name":"AfterTool"}`,
  `{"event_name":"BeforeAgent"}`. Returns `false` for Claude-shared
  names like `{"hook_event_name":"Stop"}`.
- `adapter_detects_known_codex_payloads`:
  `provider_info(Provider::Codex).adapter.detect(...)` returns `true`
  for `{"type":"turn.completed","thread_id":"t-1"}`,
  `{"type":"agent-turn-complete","thread-id":"t-1"}`,
  `{"session_id":"ses_123","hook_event":{"event_type":"after_tool_use"}}`.
- `adapter_detects_known_opencode_payloads`:
  `provider_info(Provider::OpenCode).adapter.detect(...)` returns `true`
  for `{"event_type":"session.idle"}` and
  `{"eventType":"session.idle"}`.
- `adapter_detects_known_kimi_payloads`:
  `provider_info(Provider::KimiCode).adapter.detect(...)` returns
  `true` for `{"method":"notification"}`.

Strengthen the existing
`detect_from_payload_exercise_all_providers` test (currently only proves
non-panic) to additionally assert that calling `.adapter.detect` on a
non-empty representative payload for each provider returns `true` — the
test becomes both a panic-guard and a baseline detection assertion.

Strengthen `adapter_detect_exercise_all_providers` similarly.

Verify
`detect_from_payload_recognizes_known_shapes`
(`claudine/lib/src/provider/methods.rs:789`) still passes unchanged. The
public surface must not regress — every existing
`Provider::detect_from_payload(...)` assertion in that test must continue
to hold.

#### 1.4 — Source guard against reintroducing per-provider dispatch in `methods.rs`

Add a new test
`detect_from_payload_has_no_provider_specific_branches` in
`claudine/lib/src/provider/tests.rs` (or extend the existing
`no_unauthorized_match_provider_in_lib` allow-list discipline). The test:

1. Reads the source of `claudine/lib/src/provider/methods.rs` via
   `std::fs::read_to_string`.
2. Strips comments using the existing `strip_comments` helper from
   `tests.rs:357`.
3. Asserts that the substring
   `pub fn detect_from_payload(` is followed (within the next ~600
   characters of source) by **none** of:
     - `Provider::Claude`
     - `Provider::Gemini`
     - `Provider::Codex`
     - `Provider::OpenCode`
     - `Provider::KimiCode`
     - `looks_like_codex_payload`
     - `hook_event_name`
     - `event_type` literal
     - `event_name` literal (other than as part of
       `event_from_native_name` calls — exclude that case explicitly)
     - `method` literal as a string

The test asserts a positive structural property: the body of
`detect_from_payload` is exactly the
`PROVIDERS_DISPLAY_ORDER.into_iter().find(...)` walk. A regex against the
post-strip source can match
`r"pub fn detect_from_payload\([^)]*\)\s*->\s*Option<Self>\s*\{[^}]*PROVIDERS_DISPLAY_ORDER[^}]*\}"`
and assert no other content fits between the braces.

This guard is conservative on purpose: review-4 explicitly required "an
invariant/source guard preventing reintroduction of provider-specific
dispatch in `provider/methods.rs`."

**Files:**
- `claudine/lib/src/provider/tests.rs` — new test

**Tests added in this task:** the guard above
(`detect_from_payload_has_no_provider_specific_branches`).

**Acceptance criteria:**
- New guard test passes.
- The pre-existing `no_unauthorized_match_provider_in_lib` test still
  passes (it allow-lists `provider/methods.rs`, but the guard works
  alongside it).

### Phase 1 Acceptance Gate

- `cargo test -p claudine --lib provider::` passes.
- `cargo test -p claudine --lib` passes.
- `cargo test -p claudine-cli providers` passes.
- `cargo test -p claudine-cli wrapper_registry_covers_every_provider_and_documents_exceptions`
  passes.
- `cargo clippy -p claudine -p claudine-cli --all-targets -- -D warnings`
  produces zero warnings (only the claudine package areas are gated; the
  rest of the workspace is out of scope for this plan).
- The new tests added in 1.2 — 1.4 are present and passing.
- `Provider::detect_from_payload` body contains zero `Provider::X`
  literals and zero string-shape literals.
- `claudine providers --describe --format json` emits objects under
  `capabilities` and `resource_support` for every provider (manual smoke
  check — output piped to `jq '.[] | {provider, has_caps:
  (.capabilities|type=="object"), has_res: (.resource_support|type=="object")}'`).

---

## Phase 2 — Update `building-an-agent-wrapper.md` to match the implemented compatibility window

### Scope (Closes Issue 3)

Update [`claudine/docs/topics/building-an-agent-wrapper.md`](../../docs/topics/building-an-agent-wrapper.md)
so the migration history matches the code: review-plan-3 restored the
`AgentId` alias, the `events::Provider` re-export, and
`events::PROVIDERS_DISPLAY_ORDER` for the deprecation window the spec
mandated. The doc currently says Phase 8 removed them.

### Tasks

#### 2.1 — Rewrite the Phase 8 bullet at line 190

Replace the existing bullet at
`claudine/docs/topics/building-an-agent-wrapper.md:190`:

> - **Phase 8** removed the deprecated `AgentId` alias and the
>   `crate::events::Provider` re-export, deleted the per-provider thin
>   facade structs in `agents/<name>.rs`, and consolidated the
>   `impl Provider` blocks (CLI aliases, sniff binding, payload
>   detection, slug, doc URLs, agent offset, event mapping accessors,
>   `Display`) into [`provider/methods.rs`](../../lib/src/provider/methods.rs).
>   All in-repo consumers now import from `crate::provider::*`.

with a corrected version that:

1. Describes Phase 8 as having consolidated `impl Provider` blocks and
   retired the per-provider thin facades in `agents/<name>.rs`.
2. Explicitly states that the `AgentId` alias and the
   `crate::events::Provider` / `events::PROVIDERS_DISPLAY_ORDER`
   re-exports remain in place as `#[deprecated]` re-exports until the
   post-Phase-8 cleanup release, matching design §6.1 and the
   `#[deprecated]` markers in `claudine/lib/src/events/provider.rs:9`
   and `claudine/lib/src/agents/mod.rs:15`.
3. Notes that `provider/methods.rs` is the canonical home for `impl
   Provider` and that callers in new code should import directly from
   `crate::provider::*`.

Suggested replacement bullet (committed text may differ slightly to
match surrounding tone, but every fact above must appear):

> - **Phase 8** consolidated the `impl Provider` blocks (CLI aliases,
>   sniff binding, payload detection, slug, doc URLs, agent offset,
>   event mapping accessors, `Display`) into
>   [`provider/methods.rs`](../../lib/src/provider/methods.rs) and
>   retired the per-provider thin facade structs that previously lived
>   in `agents/<name>.rs`. The `AgentId` alias and the
>   `crate::events::Provider` and `events::PROVIDERS_DISPLAY_ORDER`
>   re-exports remain in place as `#[deprecated]` shims and will be
>   removed in the post-Phase-8 cleanup release, matching the
>   deprecation window described in the design's §"Deprecation
>   Mechanics". All in-repo consumers now import from
>   `crate::provider::*`.

If the "Migration History" section has any other statements that imply
the deprecated surfaces are gone, audit them in the same edit and adjust
to match the current code state. Search for
`AgentId`, `events::Provider`, and `PROVIDERS_DISPLAY_ORDER` mentions in
the doc and ensure each one accurately describes whether it is
implemented today or deferred to the cleanup release.

**Files:**
- `claudine/docs/topics/building-an-agent-wrapper.md:190` (Phase 8
  bullet)
- (potentially) other Phase 8 / Migration History references in the same
  file if the audit surfaces them.

**Tests:** documentation-only change; no executable test asserts the
prose. The Phase 3 lint/test gate still applies (no rustdoc warnings, no
broken intra-doc links).

**Acceptance criteria:**
- Phase 8 bullet at line 190 references the deprecation window for
  `AgentId` / `events::Provider` / `events::PROVIDERS_DISPLAY_ORDER`.
- No remaining doc claims that the deprecated re-exports were removed.
- `cargo doc -p claudine --no-deps` (run as part of Phase 3) does not
  emit new warnings for this file.

---

## Phase 3 — Verification Gate

### Scope

Confirm every claudine package area is green at the end of the plan, on
the same checklist as
[`review-plan-3.md`](./review-plan-3.md) Phase 4. This phase MUST run
after Phases 1 and 2 land.

### Tasks

#### 3.1 — Enumerate the claudine package areas

Run `cargo metadata --no-deps --format-version 1` and select crates
whose manifest path is inside `claudine/`. The expected set is
`claudine` (lib) and `claudine-cli`. Pin the list before running the
gate so the verification scope is reproducible.

#### 3.2 — Run the test gate

For each crate in 3.1, run:

```sh
cargo test -p <crate> --all-targets --quiet
```

Plus the targeted suites already cited by review-4:

```sh
cargo test -p claudine --lib provider::tests --quiet
cargo test -p claudine-cli providers --quiet
cargo test -p claudine-cli wrapper_registry_covers_every_provider_and_documents_exceptions --quiet
```

Every command must exit 0 with no failing or ignored tests.

#### 3.3 — Run the lint gate

For each crate in 3.1, run:

```sh
cargo clippy -p <crate> --all-targets -- -D warnings
```

Every command must exit 0 with zero warnings and zero errors.

#### 3.4 — Run the doc gate

```sh
cargo doc -p claudine --no-deps
cargo doc -p claudine-cli --no-deps
```

Both must exit 0 with no `rustdoc::*` warnings (the doc edit in Phase 2
is the only doc churn, and it must not introduce broken intra-doc
links).

#### 3.5 — Smoke checks

- `cargo run -p claudine-cli -- providers --describe --format json | jq '.[0] | keys'`
  must list `capabilities` and `resource_support` among the keys.
- `cargo run -p claudine-cli -- providers --describe --format json | jq -r '.[] | "\(.provider) caps=\(.capabilities|type) res=\(.resource_support|type)"'`
  must report `caps=object res=object` for every provider.

### Phase 3 Acceptance Gate

- All commands in 3.2, 3.3, 3.4 exit 0.
- Smoke checks in 3.5 confirm the JSON surface fix.
- All review-4 findings are now closed:
  - Finding 1 (High): closed by Phase 1 task 1.2 + new tests.
  - Finding 2 (Medium): closed by Phase 1 tasks 1.3 + 1.4 + new tests.
  - Finding 3 (Low): closed by Phase 2 task 2.1.

---

## Out of Scope

- Adding new providers, new events, or new capability fields. This plan
  is purely closure of review-4 findings.
- Refactoring `Provider::detect_from_payload` to return `Result` or
  carry diagnostic context. The signature stays
  `fn(&Value) -> Option<Self>`.
- Replacing the fn-pointer accessor pattern
  (`agent_capabilities_fn`, `resource_support_fn`) with direct
  `&'static AgentCapabilities` / `&'static ProviderCapabilities` fields.
  That migration would touch every provider definition file and is
  unrelated to the review-4 surface gap; it can be revisited as a
  separate follow-up.
- Workspace-wide clippy or test runs. Phase 3 covers only the claudine
  package areas, mirroring review-plan-3.

## Summary of Test Coverage Additions

| Test name | Location | Purpose |
|---|---|---|
| `provider_info_json_includes_capabilities_and_resource_support` | `provider/tests.rs` | Closes Finding 1 — proves both top-level keys appear in JSON for every provider. |
| `provider_info_json_capabilities_includes_nested_runtime_data` | `provider/tests.rs` | Closes Finding 1 — proves non-interactive capabilities round-trip. |
| `provider_info_json_resource_support_includes_skills_commands_agents_scripts` | `provider/tests.rs` | Closes Finding 1 — proves skill/command/agent/script support round-trips. |
| `describe_json_serializes_all_providers` (extended) | `cli/src/commands/providers.rs` | Closes Finding 1 — CLI-side coverage for the new keys. |
| `adapter_detects_known_claude_payloads` | `provider/tests.rs` | Closes Finding 2 — Claude detection lives on `AdapterBehavior::detect`. |
| `adapter_detects_known_gemini_payloads` | `provider/tests.rs` | Closes Finding 2 — Gemini detection lives on `AdapterBehavior::detect`. |
| `adapter_detects_known_codex_payloads` | `provider/tests.rs` | Closes Finding 2 — Codex detection lives on `AdapterBehavior::detect`. |
| `adapter_detects_known_opencode_payloads` | `provider/tests.rs` | Closes Finding 2 — OpenCode detection lives on `AdapterBehavior::detect`. |
| `adapter_detects_known_kimi_payloads` | `provider/tests.rs` | Closes Finding 2 — Kimi detection lives on `AdapterBehavior::detect`. |
| `detect_from_payload_exercise_all_providers` (strengthened) | `provider/tests.rs` | Closes Finding 2 — proves detection works, not just that it doesn't panic. |
| `adapter_detect_exercise_all_providers` (strengthened) | `provider/tests.rs` | Closes Finding 2 — same. |
| `detect_from_payload_has_no_provider_specific_branches` | `provider/tests.rs` | Closes Finding 2 — source guard against reintroducing per-provider dispatch in `methods.rs`. |
| `provider_info_serializes_round_trip` (preserved) | `provider/tests.rs` | Existing test still passes after schema change. |
| `detect_from_payload_recognizes_known_shapes` (preserved) | `provider/methods.rs` | Existing public-surface test still passes. |
