# Unfolded Circle API -- Comprehensive Schematic Review

**Date:** 2026-03-05
**Scope:** All 4 Unfolded Circle API definitions in `schematic/definitions/src/unfolded_circle/`

This review evaluates the Schematic API definitions for completeness, ergonomics, test coverage, and potential integration with the `uc_api` (api-model-rs) crate.

---

## Executive Summary

The Unfolded Circle API definitions provide solid **transport infrastructure** -- typed envelopes with `Box<RawValue>` for deferred payload parsing, strongly typed discriminants, and clean auth flow modeling. However, significant gaps remain in domain-level coverage, type-level testing, and developer documentation.

| Dimension | Grade | Key Finding |
|-----------|-------|-------------|
| **api-model-rs Integration** | N/A | Do NOT add to `schematic-definitions`; integrate at consumer level (`homelab`) |
| **Completeness** | C | 14-60% coverage; transport done, domain operations missing |
| **Ergonomics & DX** | B+ | Strong foundation; nullability, KnownMessage enums, and docs need work |
| **Test Coverage** | D | 12 tests (lowest in project); zero serde/type tests |

---

## 1. api-model-rs Integration

### Recommendation: Integrate at Consumer Level, Not in Schematic

The `uc_api` crate (v0.16) and our Schematic definitions operate at complementary but distinct layers:

```
Wire bytes
  |
  v
+--------------------------------------------+
| Schematic envelope types                    |  <-- schematic-definitions
| IntegrationWsRequestEnvelope { msg_data }   |
| Uses Box<RawValue> for deferred parsing     |
+--------------------------------------------+
  |  deserialize msg_data based on `msg`
  v
+--------------------------------------------+
| uc_api payload types                        |  <-- uc_api crate
| EntityCommand, AvailableIntgEntity, etc.    |
+--------------------------------------------+
  |
  v
Application logic (homelab, integration drivers)
```

**Why NOT add `uc_api` to `schematic-definitions`:**
- Violates separation of concerns (transport vs. payload semantics)
- Introduces heavy transitive dependencies (`validator`, `strum`, `chrono`, `url`, `regex`)
- Couples release cadence to a pre-1.0 crate (v0.16)
- Breaks the Schematic pattern (no other definition module depends on a third-party payload crate)

**Our envelope types are strictly superior** to `uc_api`'s WS types:
- `Box<RawValue>` (zero-copy deferred) vs `Option<Value>` (full AST allocation)
- Strongly typed enum discriminants vs stringly typed `kind: String`

### Concrete Improvements (No New Dependencies)

1. **Add `parse_payload<T>()` helper methods** to all envelope types:

```rust
impl IntegrationWsRequestEnvelope {
    pub fn parse_payload<T: serde::de::DeserializeOwned>(
        &self,
    ) -> Result<T, serde_json::Error> {
        match &self.msg_data {
            Some(raw) => serde_json::from_str(raw.get()),
            None => Err(serde::de::Error::custom("msg_data is absent")),
        }
    }
}
```

2. **Expand `IntegrationWsKnownMessage`** from 3 auth-only variants to all ~25 well-known Integration API message names (requests, responses, events, setup flow).

3. **Wire `uc_api` in `homelab`** (or dedicated integration crate), not in `schematic-definitions`:

```rust
// homelab consumer code
match &envelope.msg {
    IntegrationWsMessageName::Known(IntegrationWsKnownMessage::EntityCommand) => {
        let cmd: uc_api::intg::EntityCommand = envelope.parse_payload()?;
        handle_entity_command(cmd)
    }
    // ...
}
```

### Risk Assessment

| Risk | Severity | Mitigation |
|------|----------|------------|
| `uc_api` version churn (pre-1.0) | Medium | Isolate to consumer crate; version pin |
| Transitive deps at consumer level | Low | `homelab` already uses most of these crates |
| Serde representation mismatches | None | Different layers, different serde contexts |
| NOT using `uc_api` (duplicate types) | High | Use `uc_api` at consumer level to avoid duplication |

---

## 2. Completeness

### Coverage Summary

| API | Defined | Official | Coverage | Assessment |
|-----|--------:|----------|---------:|------------|
| **Core REST** | 11 endpoints | ~80+ | ~14% | Minimal |
| **Core WS** | 13 messages / 4 endpoints | ~70+ / 4 | ~18% | Minimal |
| **Dock WS** | 6 messages / 1 endpoint | ~10+ / 1 | ~60% | Partial (infra) |
| **Integration WS** | 6 messages / 1 endpoint | ~15+ / 1 | ~40% | Partial (infra) |

The `Box<RawValue>` + `Known | Other(String)` pattern means all APIs are **technically usable today** via manual JSON construction. Typed domain operations can be added incrementally.

### Core REST Gaps (11 of ~80 endpoints)

Five of 11 defined endpoints are multipart file uploads. Standard JSON CRUD operations are absent:

| Missing Category | Count | Examples |
|-----------------|------:|---------|
| Entities | 8 | `GET /entities`, `POST /entities/{id}/execute` |
| Integrations | 9 | `GET /integrations`, `DELETE /integrations/{id}` |
| Profiles | 11 | `GET /profiles`, `PUT /profiles/active` |
| Auth/Keys | 8 | `POST /auth/api_keys`, `DELETE /auth/api_keys/{id}` |
| WiFi | 8 | `GET /wifi/status` |
| Docks | 6 | `GET /docks`, `GET /docks/{id}` |
| Config | 14 | Button, display, haptic, localization, network, power, sound |
| System (remaining) | 2 | `POST /system/reboot`, `POST /system/power_off` |

### Core WS Gaps (13 of ~70+ messages)

Auth + system control done. Missing:
- All entity messages (10)
- Profile messages (9)
- Integration messages (9)
- WiFi messages (8)
- Dock messages (9)
- All 16 event types (`entity_change`, `activity_group_change`, `wifi_change`, etc.)

### Dock WS Gaps

Envelope and auth complete. Missing the dock's primary purpose:
- `ir_send` -- Send IR command (most important dock operation)
- `get_status` -- Dock status (charging, battery, connection)
- `ir_learn_start` / `ir_learn_stop` / `ir_learn_result` -- IR learning
- `get_config` -- Dock configuration
- `status_update` event

### Integration WS Gaps

Envelope and auth complete. Missing all 6 **required** request types:
- `get_driver_version`, `get_device_state`, `get_available_entities`
- `subscribe_events`, `get_entity_states`, `entity_command`

Plus required events (`entity_change`, `device_state`) and setup flow (`setup_driver`, `set_driver_user_data`, `driver_setup_change`).

### Recommended Priority

1. **Entity operations** in REST + WS (highest practical value)
2. **Integration driver required messages** (needed to write drivers)
3. **Dock IR operations** (dock's primary purpose)
4. **System/admin endpoints** (full remote administration)

---

## 3. Ergonomics & Developer Experience

**Overall Grade: B+**

### Scorecard

| Category | Rating | Notes |
|----------|--------|-------|
| Idiomatic Rust & Typing | B+ | Strong enum usage; `SystemInfo` nullability loose; KnownMessage enums incomplete |
| Documentation & DX | B | Auth examples good; missing payload dispatch cookbook and cross-API guide |
| Performance | A- | `RawValue` consistent; `with_capacity` everywhere; minor clone overhead |
| Cross-API Consistency | B+ | Uniform naming; `kind` vs `type` difference documented but could be louder |
| Self-Documenting | B | Types readable; auth flow clear; error scenarios and workflows lack guidance |

### Top Issues

#### 3.1 Strict Nullability Gaps

`SystemInfo`, `CodeSetUploadResult`, and `ResourceItem` in `core_rest/types.rs` use `Option<T>` for fields the API always returns:

```rust
// Current -- forces unnecessary unwrap()
pub struct SystemInfo {
    pub model_name: Option<String>,  // always present
    pub model_number: Option<String>, // always present
    // ...
}

// Recommended
pub struct SystemInfo {
    pub model_name: String,
    pub model_number: String,
    pub serial_number: String,
    pub hw_revision: String,
}
```

Same issue with `CodeSetUploadResult` (all 3 counters always present) and `ResourceItem` (`type_name` and `id` always present).

#### 3.2 Sparse KnownMessage Enums

| Enum | Variants | Official Messages | Coverage |
|------|---------|-------------------|---------|
| `CoreWsKnownMessage` | 10 | ~150+ | ~7% |
| `DockWsKnownMessage` | 3 | ~10+ | ~30% |
| `IntegrationWsKnownMessage` | 3 | ~15+ | ~20% |

Most real-world message routing falls through to `Other(String)` with zero compile-time validation or IDE autocompletion.

#### 3.3 Undocumented `cat` Field

Event category is bare `Option<String>` with no indication of valid values. Should be either a typed enum with fallback:

```rust
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(untagged)]
pub enum CoreWsEventCategory {
    Known(CoreWsKnownCategory),
    Other(String),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum CoreWsKnownCategory {
    Entity, Integration, System, Activity,
    Profile, Remote, Dock, SoftwareUpdate,
}
```

Or at minimum a doc comment listing known values.

#### 3.4 Missing WS Usage Examples

No module-level code examples for:
- Message dispatch by `msg` field (the core WS usage pattern)
- Request envelope construction
- Dock auth handshake flow (connect -> auth_required -> auth -> authentication)

#### 3.5 Path Parameter Values Undocumented

`UploadResource` (`{resource_type}`), `GetResource` (`{resource_type}/{resource_id}`), and `InstallCustomComponent` (`{custom_component}`) don't document valid values in descriptions.

### Additional Findings

- **Timestamp format**: `ts` field documented as "Optional event timestamp" but format (ISO 8601?) not specified
- **Response codes**: `code` field documented as "Response status code" but vocabulary not specified (follows HTTP conventions but should state explicitly)
- **Dock event `msg_data` optionality**: `DockWsEventEnvelope::msg_data` is `Option<Box<RawValue>>` while Core/Integration use required `Box<RawValue>` -- verify against actual API
- **`IntegrationDriverInfo` missing `Hash`**: Correctly omitted (due to `BTreeMap`) but not documented why

---

## 4. Test Coverage

**Current: 12 tests -- lowest of any Schematic API module (median: ~35)**

### Test Inventory

All 12 tests are structural definition tests in `mod.rs` files. Zero tests in any `types.rs` file.

| Module | mod.rs Tests | types.rs Tests | Total |
|--------|:-----------:|:--------------:|:-----:|
| Core REST | 6 | 0 | 6 |
| Core WS | 2 | 0 | 2 |
| Dock WS | 2 | 0 | 2 |
| Integration WS | 2 | 0 | 2 |
| **Total** | **12** | **0** | **12** |

### Critical Gaps

1. **Zero serde tests for ~35 types/enums across 4 files** -- Every other Schematic API tests its types. Rename attributes (`#[serde(rename_all = "UPPERCASE")]`, `#[serde(rename = "type")]`, `#[serde(rename_all = "SCREAMING_SNAKE_CASE")]`) are completely untested.

2. **No `#[serde(untagged)]` tests** -- Three `MessageName` enums use `Known`/`Other` untagged deserialization. No tests verify known messages resolve to `Known(...)` rather than falling through to `Other(String)`.

3. **No `RawValue` envelope roundtrip tests** -- Nine struct types use `Box<RawValue>` for deferred payloads. No roundtrip tests.

4. **No `#[serde(rename = "type")]` field tests** -- Dock WS uses `type_name` in Rust but `type` on the wire. A rename attribute typo would silently break serialization.

### Comparison with Other Modules

| API Module | Total Tests |
|------------|:----------:|
| Bitbucket | 47 |
| Ollama | 46 |
| Samsung Smart TV | 45 |
| Eversolo | 44 |
| EMQX | 44 |
| ElevenLabs | 35 |
| Gitea | 35 |
| GitHub | 33 |
| GitLab | 33 |
| HuggingFace | 32 |
| Anthropic | 26 |
| LM Studio | 20 |
| **Unfolded Circle** | **12** |

### Recommended Test Additions

| Priority | Category | Tests to Add | Effort |
|----------|----------|:------------:|--------|
| 1 | Enum serialization (`rename_all`) | +15 | Small |
| 2 | Untagged enum behavior | +9 | Small |
| 3 | Envelope roundtrip with RawValue | +12 | Medium |
| 4 | Dock `serde(rename = "type")` field | +2 | Small |
| 5 | REST type roundtrips | +6 | Small |
| 6 | Definition-level improvements | +6 | Small |
| **Total** | | **+48** | ~2 hours |

### Priority 1 Example: Enum Serialization

```rust
// core_rest/types.rs
#[test]
fn integration_driver_type_serializes_uppercase() {
    assert_eq!(serde_json::to_string(&IntegrationDriverType::Local).unwrap(), "\"LOCAL\"");
    assert_eq!(serde_json::to_string(&IntegrationDriverType::Custom).unwrap(), "\"CUSTOM\"");
    assert_eq!(serde_json::to_string(&IntegrationDriverType::External).unwrap(), "\"EXTERNAL\"");
}

#[test]
fn driver_state_serializes_screaming_snake_case() {
    assert_eq!(serde_json::to_string(&DriverState::NotConfigured).unwrap(), "\"NOT_CONFIGURED\"");
    assert_eq!(serde_json::to_string(&DriverState::Active).unwrap(), "\"ACTIVE\"");
}
```

### Priority 2 Example: Untagged Enum Behavior

```rust
// core_ws/types.rs
#[test]
fn message_name_known_variant_deserializes() {
    let name: CoreWsMessageName = serde_json::from_str("\"auth\"").unwrap();
    assert_eq!(name, CoreWsMessageName::Known(CoreWsKnownMessage::Auth));
}

#[test]
fn message_name_unknown_falls_to_other() {
    let name: CoreWsMessageName = serde_json::from_str("\"get_entities\"").unwrap();
    assert_eq!(name, CoreWsMessageName::Other("get_entities".to_string()));
}
```

### Priority 4 Example: Dock `type` Field Rename

```rust
// dock_ws/types.rs
#[test]
fn dock_request_envelope_uses_type_field_name() {
    let json = r#"{"type":"req","id":1,"msg":"get_docks"}"#;
    let envelope: DockWsRequestEnvelope = serde_json::from_str(json).unwrap();
    assert_eq!(envelope.type_name, DockWsEnvelopeType::Req);

    let serialized = serde_json::to_string(&envelope).unwrap();
    assert!(serialized.contains("\"type\":"), "should serialize as 'type', not 'type_name'");
}
```

---

## Action Items Summary

### Must Do (High Impact)

| # | Action | Area | Effort |
|---|--------|------|--------|
| 1 | Add `parse_payload<T>()` helpers to all WS envelope types | Integration, Ergonomics | Small |
| 2 | Expand `IntegrationWsKnownMessage` to ~25 variants | Ergonomics, Completeness | Small |
| 3 | Add ~48 type-level serde tests | Test Coverage | Medium |
| 4 | Tighten `SystemInfo`, `CodeSetUploadResult`, `ResourceItem` nullability | Ergonomics | Small |

### Should Do (Medium Impact)

| # | Action | Area | Effort |
|---|--------|------|--------|
| 5 | Expand `CoreWsKnownMessage` and `DockWsKnownMessage` | Ergonomics | Small |
| 6 | Add typed event category enum (or doc comment listing values) | Ergonomics | Small |
| 7 | Add WS module-level dispatch and auth flow code examples | Documentation | Small |
| 8 | Document path parameter valid values in REST endpoint descriptions | Documentation | Small |
| 9 | Add entity CRUD REST endpoints (Phase 1 completeness) | Completeness | Medium |

### Nice to Have

| # | Action | Area | Effort |
|---|--------|------|--------|
| 10 | Add Integration WS required messages as typed schemas | Completeness | Medium |
| 11 | Add Dock IR operations (`ir_send`, `get_status`, IR learning) | Completeness | Medium |
| 12 | Add `build_event` helper for driver-as-server scenarios | Ergonomics | Small |
| 13 | Cross-API comparison docs (envelope differences, auth strategies) | Documentation | Small |
| 14 | Document `ts` timestamp format and `code` status vocabulary | Documentation | Small |
