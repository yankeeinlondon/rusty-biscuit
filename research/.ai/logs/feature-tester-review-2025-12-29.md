# Feature Tester (Rust) Review - Utility Functions Implementation Plan

**Date:** 2025-12-29
**Reviewer:** Feature Tester (Rust) Sub-Agent
**Plan:** /Volumes/coding/personal/dockhand/research/.ai/plans/2025-12-29.utility-functions-implementation.md

---

## Overall Assessment

**APPROVE WITH CHANGES**

The plan demonstrates strong technical understanding and includes comprehensive testing considerations. However, there are critical gaps in the testing strategy that need to be addressed before implementation begins.

---

## Strengths

- **Well-structured phases:** Clear separation of concerns with proper dependency ordering
- **Comprehensive acceptance criteria:** Each phase includes specific, measurable test requirements
- **Property-based testing considered:** Plan mentions proptest for provider normalization and enum injection
- **Error handling first:** Each module defines error types using thiserror before implementation
- **Observability built-in:** Tracing instrumentation planned from the start
- **Test organization:** Co-located unit tests (`#[cfg(test)]`) and integration tests (`tests/`) properly separated
- **Realistic test counts:** 15+ for Phase 1, 12+ for Phase 2, 10+ for Phase 3, 8+ for Phase 4

---

## Concerns

### 1. HTTP Mocking Strategy Not Defined (Phase 1)

**Issue:** The plan mentions "Mock HTTP responses for provider API tests (use `wiremock` or similar)" but doesn't specify:
- Which crate to use (wiremock, mockito, httpmock)
- How to structure mocks for multiple providers
- Test data fixtures for API responses

**Suggested Fix:**
Add to Phase 1 deliverables:
```rust
// shared/tests/fixtures/provider_responses/openai.json
// shared/tests/fixtures/provider_responses/huggingface.json
```

Add to Phase 1 technical details:
```rust
#[cfg(test)]
mod tests {
    use wiremock::{MockServer, Mock, ResponseTemplate};
    use wiremock::matchers::{method, path};

    #[tokio::test]
    async fn fetch_openai_models_success() {
        let mock_server = MockServer::start().await;

        Mock::given(method("GET"))
            .and(path("/v1/models"))
            .respond_with(ResponseTemplate::new(200)
                .set_body_json(json!({
                    "data": [{"id": "gpt-5.2"}]
                })))
            .mount(&mock_server)
            .await;

        // Test against mock_server.uri()
    }
}
```

### 2. File Fixture Management Missing (Phase 2)

**Issue:** `inject_enum()` testing requires:
- Valid Rust files with various enum patterns
- Invalid syntax files for pre-check testing
- Files with multiple enums (test selective removal)
- Edge cases (empty files, no enum, malformed enum)

**Suggested Fix:**
Add to Phase 2 deliverables:
```
- Directory: shared/tests/fixtures/codegen/
  - valid_single_enum.rs
  - valid_multiple_enums.rs
  - invalid_syntax.rs
  - empty.rs
  - no_enum.rs
  - complex_enum_with_attributes.rs
```

### 3. rig-core Mocking Strategy Unclear (Phase 3)

**Issue:** The plan shows:
```rust
fn try_build_client(provider: ModelProvider) -> Result<Client, ModelError>
```

But doesn't explain:
- How to test fallback behavior without real API calls
- How to mock `rig_core::providers::*::Client::from_env()`
- How to simulate initialization failures

**Suggested Fix:**
Add trait abstraction for testability:
```rust
// shared/src/model/client_builder.rs
pub trait ClientBuilder {
    fn build(&self, provider_name: &str, model_id: &str) -> Result<Client, ModelError>;
}

// Production implementation
pub struct RigClientBuilder;

impl ClientBuilder for RigClientBuilder {
    fn build(&self, provider_name: &str, model_id: &str) -> Result<Client, ModelError> {
        // Existing rig-core logic
    }
}

// Test implementation
#[cfg(test)]
pub struct MockClientBuilder {
    pub fail_on: Vec<String>,
}

#[cfg(test)]
impl ClientBuilder for MockClientBuilder {
    fn build(&self, provider_name: &str, model_id: &str) -> Result<Client, ModelError> {
        if self.fail_on.contains(&provider_name.to_string()) {
            Err(ModelError::ClientInitFailed("mock failure".into()))
        } else {
            // Return mock client
        }
    }
}
```

Update acceptance criteria to test with mock builder.

### 4. research_health Filesystem Fixture Strategy (Phase 4)

**Issue:** Testing `research_health()` requires directory structures:
- Healthy topic (all files present)
- Missing Phase 1 prompts (various combinations)
- Missing Phase 2 outputs (skill, deep_dive, brief)
- Invalid SKILL.md frontmatter

**Suggested Fix:**
Add to Phase 4 technical details:
```rust
#[cfg(test)]
mod tests {
    use tempfile::TempDir;

    fn create_healthy_topic(base: &Path) -> PathBuf {
        let topic_path = base.join("library/pulldown-cmark");
        fs::create_dir_all(&topic_path).unwrap();
        fs::write(topic_path.join("overview.md"), "# Overview").unwrap();
        fs::write(topic_path.join("use_cases.md"), "# Use Cases").unwrap();
        // ... all standard prompts
        fs::create_dir_all(topic_path.join("skill")).unwrap();
        fs::write(topic_path.join("skill/SKILL.md"),
            "---\nname: test\ndescription: test\n---\n# Test").unwrap();
        fs::write(topic_path.join("deep_dive.md"), "# Deep Dive").unwrap();
        fs::write(topic_path.join("brief.md"), "# Brief").unwrap();
        topic_path
    }

    #[test]
    fn healthy_topic_returns_ok() {
        let temp = TempDir::new().unwrap();
        let topic_path = create_healthy_topic(temp.path());

        let health = research_health("library", "pulldown-cmark").unwrap();
        assert!(health.ok);
        assert!(health.missing_underlying.is_empty());
        assert!(health.missing_deliverables.is_empty());
        assert!(health.skill_structure_valid);
    }
}
```

Add to Phase 4 dependencies: `tempfile = "3"` in `[dev-dependencies]`

### 5. Missing Integration Test for Phase 1 + Phase 2 Combination

**Issue:** The plan shows Phase 1 (generate_provider_list) and Phase 2 (inject_enum) work together:
```
generate_provider_list(RustEnum) → inject_enum(name, output, file)
```

But there's no integration test verifying this end-to-end workflow.

**Suggested Fix:**
Add to Cross-Cutting Concerns testing strategy:
```rust
// shared/tests/provider_codegen_integration.rs
#[tokio::test]
async fn generate_and_inject_provider_enum() {
    // 1. Generate enum from provider list
    let enum_str = generate_provider_list(Some(ProviderListFormat::RustEnum))
        .await
        .unwrap();

    // 2. Inject into temporary file
    let temp_file = NamedTempFile::new().unwrap();
    inject_enum("ModelProvider", &enum_str, temp_file.path().to_str().unwrap())
        .await
        .unwrap();

    // 3. Verify file is valid Rust
    let content = fs::read_to_string(temp_file.path()).unwrap();
    syn::parse_file(&content).expect("Generated file must be valid Rust");

    // 4. Verify enum is present
    assert!(content.contains("pub enum ModelProvider"));
}
```

---

## Suggested Changes

### 1. Add HTTP Mocking Dependency (Phase 1)

**Change to shared/Cargo.toml:**
```toml
[dev-dependencies]
wiremock = "0.6"
```

**Change to Phase 1 acceptance criteria:**
- [ ] `wiremock` used for all provider API tests (no real network calls)
- [ ] Fixture files exist in `shared/tests/fixtures/provider_responses/`
- [ ] Tests cover 429 rate limit responses with retry verification
- [ ] Tests cover authentication failures (401/403)

### 2. Add File Fixtures (Phase 2)

**Change to Phase 2 deliverables:**
```
- Directory: shared/tests/fixtures/codegen/
  - valid_single_enum.rs (1 enum)
  - valid_multiple_enums.rs (3 enums, test selective removal)
  - invalid_syntax.rs (syntax error)
  - empty.rs (empty file)
  - no_enum.rs (valid Rust, no enum)
  - complex_enum.rs (enum with derives, attributes, doc comments)
```

**Change to Phase 2 acceptance criteria:**
- [ ] Tests use fixture files from `shared/tests/fixtures/codegen/`
- [ ] Test preserves other enums when replacing one in multi-enum file
- [ ] Test preserves enum attributes and doc comments

### 3. Add ClientBuilder Abstraction (Phase 3)

**Change to Phase 3 deliverables:**
```
- File: shared/src/model/client_builder.rs (trait abstraction for testing)
```

**Change to Phase 3 technical details:**
Add `ClientBuilder` trait (see Concern #3 above).

**Change to Phase 3 acceptance criteria:**
- [ ] `ClientBuilder` trait exists with production and test implementations
- [ ] Tests use `MockClientBuilder` to simulate initialization failures
- [ ] Fallback tests verify all models in stack are attempted before NoValidModel error

### 4. Add tempfile Dependency (Phase 4)

**Change to research/lib/Cargo.toml:**
```toml
[dev-dependencies]
tempfile = "3"
```

**Change to Phase 4 acceptance criteria:**
- [ ] Tests use `tempfile::TempDir` for filesystem fixtures
- [ ] Helper functions create various topic states (healthy, missing_prompts, etc.)
- [ ] Tests clean up temporary directories after execution

### 5. Add End-to-End Integration Test

**Change to Cross-Cutting Concerns:**
Add new section:
```markdown
### End-to-End Integration Tests

**Phase 1 + Phase 2:**
- `shared/tests/provider_codegen_integration.rs` - Generate enum from APIs, inject into file, verify syntax

**Phase 3 + Phase 1:**
- `shared/tests/model_provider_integration.rs` - Verify ModelProvider enum matches generate_provider_list output

**Phase 4 + Phase 5:**
- `research/lib/tests/health_migration_integration.rs` - Verify research_health produces same results as old scattered checks
```

---

## Test Scenarios to Add

### Phase 1 (generate_provider_list)

**Missing scenarios:**
- [ ] Provider API returns malformed JSON (test error handling)
- [ ] Provider API times out (test timeout behavior)
- [ ] Multiple providers fail, but one succeeds (test partial success)
- [ ] Deduplication when multiple providers return same model
- [ ] Enum variant naming collision handling (e.g., "gpt-5.2" vs "gpt_5_2")

### Phase 2 (inject_enum)

**Missing scenarios:**
- [ ] Enum with generic parameters: `pub enum Foo<T> { ... }`
- [ ] Enum with visibility modifiers: `pub(crate) enum Foo { ... }`
- [ ] Enum with where clauses
- [ ] File with multiple modules containing enums (same name in different modules)
- [ ] Race condition simulation (file modified between read and write)

### Phase 3 (get_model)

**Missing scenarios:**
- [ ] TryExplicit fallback exhausts all models in fallback stack
- [ ] ModelKind::Scrape maps to correct stack (verify it's Fast)
- [ ] Stderr logging format verification (verify "- using the {model} from {provider} to {desc}")
- [ ] Concurrent get_model calls (thread safety)

### Phase 4 (research_health)

**Missing scenarios:**
- [ ] Topic path exists but is a file, not a directory
- [ ] Topic path is a symlink
- [ ] SKILL.md exists but is empty (frontmatter validation)
- [ ] SKILL.md frontmatter missing required fields
- [ ] Partially complete topic (some Phase 1 files, no Phase 2 files)
- [ ] All files present but SKILL.md frontmatter invalid (ok should be false)

### Phase 5 (migration)

**Missing scenarios:**
- [ ] Verify `research list` output unchanged after migration
- [ ] Verify `research link` rejects unhealthy topics
- [ ] Verify `research library` warning messages match old behavior
- [ ] Performance comparison (research_health vs scattered checks)

---

## Missing Considerations

### 1. Test Data Reproducibility

The plan doesn't address:
- How to ensure mock data stays synchronized with real API schemas
- How to update test fixtures when provider APIs change
- Version pinning for test data

**Recommendation:**
Add to Phase 1 deliverables:
```
- File: shared/tests/fixtures/README.md (documents fixture update process)
- Script: scripts/update_provider_fixtures.sh (fetches latest API responses)
```

### 2. Property-Based Test Coverage

The plan mentions proptest but doesn't define properties to test.

**Recommendation:**
Add to Phase 1 acceptance criteria:
```rust
// Property: Provider normalization is idempotent
proptest! {
    #[test]
    fn normalize_provider_idempotent(name in ".*") {
        let normalized = normalize_provider_name(&name);
        let double_normalized = normalize_provider_name(&normalized);
        assert_eq!(normalized, double_normalized);
    }
}
```

Add to Phase 2 acceptance criteria:
```rust
// Property: inject_enum preserves valid syntax
proptest! {
    #[test]
    fn inject_enum_preserves_syntax(enum_name in "[A-Z][a-zA-Z0-9]*") {
        let valid_rust = "fn main() {}";
        let enum_str = format!("pub enum {} {{ A, B }}", enum_name);
        let result = inject_enum(&enum_name, &enum_str, "temp.rs").await;
        assert!(result.is_ok());
        let content = fs::read_to_string("temp.rs").unwrap();
        assert!(syn::parse_file(&content).is_ok());
    }
}
```

### 3. Error Message Quality Testing

The plan defines error types but doesn't verify error messages are helpful.

**Recommendation:**
Add to all phases:
```rust
#[test]
fn error_messages_are_helpful() {
    let err = ProviderError::RateLimitExceeded {
        provider: "openai".into()
    };
    let msg = err.to_string();
    assert!(msg.contains("openai"));
    assert!(msg.contains("Rate limit"));
}
```

### 4. Logging Verification

Phase 3 requires stderr logging but doesn't specify how to test it.

**Recommendation:**
Add to Phase 3 technical details:
```rust
#[test]
fn get_model_logs_to_stderr() {
    use std::io::Write;
    use std::sync::{Arc, Mutex};

    // Capture stderr
    let captured = Arc::new(Mutex::new(Vec::new()));
    let captured_clone = captured.clone();

    // Mock stderr
    let mock_stderr = Box::new(captured_clone);

    get_model(ModelKind::Fast, Some("test task")).unwrap();

    let output = String::from_utf8(captured.lock().unwrap().clone()).unwrap();
    assert!(output.contains("using the"));
    assert!(output.contains("to test task"));
}
```

Or use `tracing-test` crate for structured log verification.

### 5. Blast Radius Validation Insufficient (Phase 5)

**Issue:** Phase 5 says "Blast Radius: `cargo test` (full test suite)" but doesn't specify:
- Pre-migration test baseline (number of passing tests)
- Post-migration test expectations (same count, no regressions)
- Manual testing checklist

**Recommendation:**
Add to Phase 5 acceptance criteria:
```markdown
**Pre-Migration Validation:**
- [ ] Run `cargo test --lib` and record test count: _____ (baseline)
- [ ] Run `cargo test --test '*'` and record test count: _____ (baseline)

**Post-Migration Validation:**
- [ ] All baseline tests still pass
- [ ] No new test failures introduced
- [ ] Manual smoke tests:
  - [ ] `research list` - displays issues correctly
  - [ ] `research link valid-topic` - succeeds
  - [ ] `research link invalid-topic` - rejects with clear error
  - [ ] `research library new-topic` - validates prompts
```

---

## Summary of Required Changes

### Critical (Must Address Before Implementation)

1. **Add HTTP mocking strategy** (Phase 1) - wiremock dependency and fixtures
2. **Add file fixtures** (Phase 2) - codegen test files
3. **Add ClientBuilder abstraction** (Phase 3) - for testable fallback behavior
4. **Add tempfile dependency** (Phase 4) - for filesystem test isolation
5. **Add pre/post migration test baseline** (Phase 5) - regression detection

### Recommended (Should Address Before Implementation)

6. **Add end-to-end integration tests** - verify phases work together
7. **Define property-based test properties** - explicit proptest strategies
8. **Add error message quality tests** - verify user-facing messages
9. **Add logging verification tests** - stderr capture in Phase 3

### Nice-to-Have (Can Address During Implementation)

10. **Add test data update scripts** - keep fixtures synchronized with APIs
11. **Add performance benchmarks** - verify research_health <100ms target
12. **Add concurrent testing** - thread safety verification

---

## Conclusion

The plan is solid overall with strong technical foundations. The main gaps are in test infrastructure (mocking, fixtures, abstractions) rather than test coverage. Addressing the critical changes above will ensure a smooth TDD workflow where tests can be written before implementation.

**Recommendation:** Approve with changes. Implement the 5 critical changes before beginning Phase 1 implementation.

---

**Review Completed:** 2025-12-29
**Next Action:** Update plan with critical changes, then begin Phase 1 implementation with TDD workflow
