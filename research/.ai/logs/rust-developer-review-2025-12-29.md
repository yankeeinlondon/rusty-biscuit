# Rust Developer Review: Utility Functions Implementation Plan

**Date:** 2025-12-29
**Reviewer:** Rust Developer Sub-Agent
**Plan:** `/Volumes/coding/personal/dockhand/research/.ai/plans/2025-12-29.utility-functions-implementation.md`

## Skills Activated

- `rust-testing` - For unit, integration, property-based testing guidance
- `rust-logging` - For tracing/observability implementation patterns

---

## Overall Assessment

**APPROVE WITH CHANGES**

The plan is architecturally sound and demonstrates strong understanding of Rust patterns, error handling, and testing. However, several technical concerns need addressing before implementation begins, particularly around:

1. **API integration patterns** (rate limiting, authentication)
2. **Code generation safety** (syn usage vs regex)
3. **Type design** (generic Client types)
4. **Testing completeness** (missing integration test scenarios)

---

## Strengths

- **Excellent module organization** - Clean separation of concerns with `providers/`, `codegen/`, `model/` modules
- **Comprehensive error handling** - Good use of `thiserror` with contextual error types for each domain
- **Strong observability plan** - Proper use of `#[instrument]` and stderr logging for user-facing messages
- **TDD-friendly acceptance criteria** - Each phase has clear, testable deliverables
- **Parallelization awareness** - Correctly identifies independent phases (1-4) vs sequential (5)
- **Safety-first approach** - `inject_enum()` validates before and after modification with rollback strategy

---

## Concerns

### 1. Provider API Integration - Authentication & Rate Limiting (MAJOR)

**Issue:** The plan mentions exponential backoff for 429 responses but doesn't specify the implementation details. The rate limiting strategy is critical for production reliability.

**Location:** Phase 1, lines 305-309

**Current approach:**
```rust
// Rate Limiting:
- Implement exponential backoff for 429 responses
- Max 3 retries per provider
- If provider fails, continue with others (don't fail entire function)
```

**Concern:**
- No mention of initial retry delay or backoff multiplier
- No discussion of concurrent request limiting (multiple calls to `generate_provider_list()`)
- No strategy for persistent rate limit tracking across invocations
- Missing details on timeout values per request

**Suggested fix:**
```rust
// Add to Phase 1 technical details:

/// Rate limiting configuration
const INITIAL_RETRY_DELAY_MS: u64 = 1000;  // 1 second
const MAX_RETRY_DELAY_MS: u64 = 30000;     // 30 seconds
const BACKOFF_MULTIPLIER: f64 = 2.0;
const MAX_RETRIES: u32 = 3;
const REQUEST_TIMEOUT_SECS: u64 = 30;

/// Retry with exponential backoff
async fn fetch_with_retry<F, T>(
    provider_name: &str,
    fetch_fn: F,
) -> Result<T, ProviderError>
where
    F: Fn() -> BoxFuture<'static, Result<T, reqwest::Error>>,
{
    let mut delay = INITIAL_RETRY_DELAY_MS;

    for attempt in 0..=MAX_RETRIES {
        match fetch_fn().await {
            Ok(result) => return Ok(result),
            Err(e) if attempt < MAX_RETRIES => {
                if let Some(status) = e.status() {
                    if status == StatusCode::TOO_MANY_REQUESTS {
                        warn!("Rate limited by {}, retrying in {}ms", provider_name, delay);
                        tokio::time::sleep(Duration::from_millis(delay)).await;
                        delay = (delay as f64 * BACKOFF_MULTIPLIER).min(MAX_RETRY_DELAY_MS as f64) as u64;
                        continue;
                    }
                }
                return Err(ProviderError::HttpError(e));
            }
            Err(e) => return Err(ProviderError::HttpError(e)),
        }
    }

    Err(ProviderError::RateLimitExceeded {
        provider: provider_name.to_string()
    })
}
```

**Add to acceptance criteria:**
- [ ] Rate limiting retries with exponential backoff (1s, 2s, 4s delays)
- [ ] Request timeout set to 30 seconds per API call
- [ ] Tests verify retry behavior with mocked 429 responses

---

### 2. Code Generation Safety - syn vs Regex Trade-offs (CRITICAL)

**Issue:** The plan proposes regex-based enum removal for "MVP" but this introduces significant safety risks that contradict the safety-first approach.

**Location:** Phase 2, lines 428-438

**Current approach:**
```rust
// Alternative Simpler Approach (MVP):
fn remove_enum_definition_simple(content: &str, enum_name: &str) -> Result<String, CodegenError> {
    // Regex to match: pub enum EnumName { ... }
    let pattern = format!(r"(?s)pub\s+enum\s+{}\s*\{{[^}}]*\}}", regex::escape(enum_name));
    let re = Regex::new(&pattern)?;
    Ok(re.replace_all(content, "").to_string())
}
```

**Concern:**
- Regex pattern `[^}}]*` fails on nested enums or enums containing `}` in doc comments
- Cannot handle attributes like `#[derive(...)]` correctly
- Will break on complex enum patterns (generic bounds, where clauses)
- Contradicts the "safety-first" principle stated in Phase 2 description

**Example failures:**
```rust
/// Example with } in docstring: `match x { Some(v) => v, None => 0 }`
pub enum MyEnum { Variant }

#[derive(Debug)]
#[allow(dead_code)]
pub enum MyEnum<T: Debug> where T: Clone {
    Variant(T),
}
```

**Suggested fix:**

**Remove the regex approach entirely.** Use `syn` + `quote` for safe, correct AST manipulation:

```rust
use syn::{File, Item};
use quote::ToTokens;

fn remove_enum_definition(content: &str, enum_name: &str) -> Result<String, CodegenError> {
    if content.is_empty() {
        return Ok(String::new());
    }

    let mut ast: File = syn::parse_str(content)
        .map_err(|e| CodegenError::SyntaxError { message: e.to_string() })?;

    // Filter out the target enum
    ast.items.retain(|item| {
        !matches!(item, Item::Enum(e) if e.ident == enum_name)
    });

    // Reconstruct source from AST
    Ok(ast.into_token_stream().to_string())
}
```

**Add dependency:**
```toml
quote = "1.0"  # Required for ToTokens
```

**Update Phase 2 acceptance criteria:**
- [ ] Remove "Alternative Simpler Approach" regex implementation
- [ ] AST-based removal handles generic enums, attributes, and doc comments
- [ ] Property test: roundtrip parse → remove → parse preserves validity
- [ ] Tests include enums with `}` characters in doc comments

---

### 3. Model Selection - Generic Type Parameter Missing (MAJOR)

**Issue:** The plan shows `get_model()` returning `Client` but rig-core's `Client` type is generic over the agent type. The return type needs to be `Client<Agent>` or similar.

**Location:** Phase 3, lines 599-633

**Current signature:**
```rust
pub fn get_model(
    kind: ModelKind,
    desc: Option<&str>
) -> Result<Client, ModelError>
```

**Concern:**
- `rig_core::Client<T>` is generic - cannot return bare `Client`
- The plan's `try_build_client()` shows building agents but type is not reflected in signature
- Will not compile without specifying the generic parameter

**Suggested fix:**
```rust
use rig_core::{Client, Agent};

pub fn get_model(
    kind: ModelKind,
    desc: Option<&str>
) -> Result<Client<Agent>, ModelError> {
    // Implementation stays the same but returns Client<Agent>
}

// Alternative: Use type alias for clarity
pub type ModelClient = Client<Agent>;

pub fn get_model(
    kind: ModelKind,
    desc: Option<&str>
) -> Result<ModelClient, ModelError> {
    // ...
}
```

**Add to Phase 3 acceptance criteria:**
- [ ] Return type is `Client<Agent>` or type-aliased equivalent
- [ ] Compiles without type inference errors
- [ ] Tests construct clients and verify they implement expected traits

---

### 4. Testing Strategy - Missing Integration Test Scenarios (MAJOR)

**Issue:** The cross-cutting concerns mention integration tests but don't specify critical scenarios needed for production confidence.

**Location:** Lines 949-967

**Missing scenarios:**

**Phase 1 (`generate_provider_list`):**
- Real API call tests with actual credentials (opt-in with env var)
- Network failure handling (timeouts, connection refused)
- Malformed JSON response handling
- Empty model list responses
- Concurrent fetch behavior (are requests parallelized?)

**Phase 2 (`inject_enum`):**
- Inject into file with existing imports and other items
- Replace enum that's used elsewhere in the file (ensure no references break)
- Handle files with different line ending styles (CRLF vs LF)
- Permissions issues (read-only files)

**Phase 3 (`get_model`):**
- Missing API keys for all providers in stack
- API key present but invalid (auth failure)
- Network unavailable scenario
- Multiple concurrent `get_model()` calls

**Phase 4 (`research_health`):**
- Symlinked directories (common in research workflow)
- Partially complete topics (some prompts but no outputs)
- Corrupted SKILL.md (invalid YAML frontmatter)

**Suggested addition to plan:**

Add new section after line 967:

```markdown
### Integration Test Scenarios

**Phase 1 - Provider Discovery:**
```rust
// tests/provider_integration.rs

#[tokio::test]
#[ignore]  // Requires API keys
async fn test_real_openai_fetch() {
    dotenvy::dotenv().ok();
    let result = fetch_openai_models().await;
    assert!(result.is_ok());
    assert!(result.unwrap().len() > 10);
}

#[tokio::test]
async fn test_network_timeout() {
    // Use wiremock to simulate slow server
    let mock_server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path("/v1/models"))
        .respond_with(ResponseTemplate::new(200).set_delay(Duration::from_secs(60)))
        .mount(&mock_server)
        .await;

    let result = fetch_with_retry("openai", || fetch_fn(&mock_server.uri())).await;
    assert!(matches!(result, Err(ProviderError::HttpError(_))));
}
```

**Phase 2 - Code Generation:**
```rust
// tests/codegen_integration.rs

#[test]
fn test_inject_preserves_other_items() {
    let original = r#"
        use std::fmt;

        pub struct MyStruct;

        pub enum OldEnum {
            Variant,
        }

        impl MyStruct {
            pub fn new() -> Self { Self }
        }
    "#;

    let new_enum = r#"pub enum OldEnum {
        NewVariant,
    }"#;

    let result = inject_enum("OldEnum", new_enum, original).unwrap();

    // Verify struct and impl still present
    assert!(result.contains("pub struct MyStruct"));
    assert!(result.contains("impl MyStruct"));
    assert!(result.contains("NewVariant"));
    assert!(!result.contains("Variant,"));  // Old variant removed
}
```

**Phase 3 - Model Selection:**
```rust
// tests/model_selection_integration.rs

#[test]
fn test_missing_all_api_keys() {
    // Temporarily clear API key env vars
    std::env::remove_var("ANTHROPIC_API_KEY");
    std::env::remove_var("OPENAI_API_KEY");
    std::env::remove_var("GOOGLE_API_KEY");

    let result = get_model(ModelKind::Fast, Some("test"));
    assert!(matches!(result, Err(ModelError::NoValidModel)));
}
```

**Phase 4 - Research Health:**
```rust
// tests/research_health_integration.rs

#[test]
fn test_health_check_symlinked_topic() {
    let temp = tempdir().unwrap();
    let real_path = temp.path().join("real_topic");
    let link_path = temp.path().join("linked_topic");

    create_dir_all(&real_path).unwrap();
    std::os::unix::fs::symlink(&real_path, &link_path).unwrap();

    // Create prompts in real location
    write_all_prompts(&real_path);

    // Health check via symlink should work
    let health = research_health("library", link_path.to_str().unwrap()).unwrap();
    assert!(health.ok);
}
```
```

**Add to each phase's acceptance criteria:**
- [ ] Integration tests cover network failure scenarios
- [ ] Integration tests cover edge cases (empty responses, timeouts, etc.)
- [ ] Tests use fixtures for realistic data patterns

---

### 5. Observability - Missing Span Context (MINOR)

**Issue:** The `#[instrument]` usage is good but missing important context fields that would help debugging in production.

**Location:** Lines 1008-1033

**Current approach:**
```rust
#[instrument(skip(format))]
pub async fn generate_provider_list(format: Option<ProviderListFormat>) -> Result<String>
```

**Concern:**
- Skips `format` but doesn't include it in fields (loses information)
- No request ID or correlation ID for distributed tracing
- No timing information for performance debugging

**Suggested improvement:**
```rust
#[instrument(
    skip(format),
    fields(
        format = ?format.as_ref().map(|f| format!("{:?}", f)),
        provider_count = tracing::field::Empty,  // Fill later
        total_models = tracing::field::Empty
    )
)]
pub async fn generate_provider_list(
    format: Option<ProviderListFormat>
) -> Result<String, ProviderError> {
    let start = std::time::Instant::now();
    info!("Fetching provider list");

    let entries = fetch_all_providers().await?;

    tracing::Span::current()
        .record("provider_count", entries.len())
        .record("total_models", entries.iter().map(|e| e.models.len()).sum::<usize>());

    info!(elapsed_ms = start.elapsed().as_millis(), "Fetch complete");

    // ... format conversion
}
```

**Add to Phase 1-4 acceptance criteria:**
- [ ] Spans include key metrics (counts, timing) as structured fields
- [ ] Error cases include error type in span before returning

---

### 6. Property-Based Testing - Missing Invariants (MINOR)

**Issue:** The plan mentions proptest for normalization but doesn't specify the invariants to test.

**Location:** Line 960

**Suggested invariants:**

```rust
// shared/tests/property_tests.rs

use proptest::prelude::*;

proptest! {
    // Phase 1: Provider normalization
    #[test]
    fn provider_name_normalization_is_idempotent(name in "[a-zA-Z0-9 -]{1,50}") {
        let once = normalize_provider_name(&name);
        let twice = normalize_provider_name(&once);
        prop_assert_eq!(once, twice);
    }

    #[test]
    fn enum_variant_never_has_spaces(provider in "[a-zA-Z]{1,20}", model in "[a-zA-Z0-9.-]{1,20}") {
        let variant = to_enum_variant(&provider, &model);
        prop_assert!(!variant.contains(' '));
        prop_assert!(variant.chars().all(|c| c.is_alphanumeric() || c == '_'));
    }

    // Phase 2: Enum injection preserves syntax
    #[test]
    fn inject_enum_preserves_valid_syntax(
        enum_name in "[A-Z][a-zA-Z0-9]{1,20}",
        variants in prop::collection::vec("[A-Z][a-zA-Z0-9]{1,15}", 1..10)
    ) {
        let enum_str = format!(
            "pub enum {} {{\n{}\n}}",
            enum_name,
            variants.join(",\n")
        );

        let original = "pub struct Test;";
        let result = inject_enum(&enum_name, &enum_str, original)?;

        // Should parse as valid Rust
        prop_assert!(syn::parse_file(&result).is_ok());
    }

    // Phase 4: Research health is consistent
    #[test]
    fn research_health_ok_implies_no_issues(topic_name in "[a-z-]{1,30}") {
        // Setup fixture with complete topic
        let fixture = create_complete_topic_fixture(&topic_name);

        let health = research_health("library", &topic_name)?;

        if health.ok {
            prop_assert!(health.missing_underlying.is_empty());
            prop_assert!(health.missing_deliverables.is_empty());
            prop_assert!(health.skill_structure_valid);
        }
    }
}
```

**Add to acceptance criteria:**
- [ ] Property tests verify normalization is idempotent
- [ ] Property tests verify enum variants are valid Rust identifiers
- [ ] Property tests verify syntax preservation across inject operations

---

## Suggested Changes

### Change 1: Add Concurrent Request Limiting to Phase 1

**Rationale:** Multiple `research` commands running concurrently could trigger rate limits. Add request coalescing.

**Location:** After line 287 (Phase 1 format conversion)

**Addition:**
```rust
// Add to shared/src/providers/cache.rs

use tokio::sync::Mutex;
use std::sync::Arc;

lazy_static! {
    static ref FETCH_LOCK: Arc<Mutex<()>> = Arc::new(Mutex::new(()));
}

pub async fn generate_provider_list(
    format: Option<ProviderListFormat>
) -> Result<String, ProviderError> {
    // Check cache first
    if let Some(cached) = read_cache(format)? {
        return Ok(cached);
    }

    // Acquire lock to prevent concurrent API calls
    let _guard = FETCH_LOCK.lock().await;

    // Double-check cache after acquiring lock
    if let Some(cached) = read_cache(format)? {
        return Ok(cached);
    }

    // Fetch and cache
    let entries = fetch_all_providers().await?;
    let result = format_entries(entries, format)?;
    write_cache(&result, format)?;

    Ok(result)
}
```

---

### Change 2: Add `quote` Dependency to Phase 2

**Rationale:** Required for safe AST-to-source reconstruction (Concern #2)

**Location:** Line 344 (Phase 2 dependencies)

**Change:**
```diff
- Updated: `shared/Cargo.toml` - add `syn = { version = "2.0", features = ["full", "parsing"] }`
+ Updated: `shared/Cargo.toml` - add dependencies:
+   syn = { version = "2.0", features = ["full", "parsing"] }
+   quote = "1.0"
```

---

### Change 3: Specify Model Selection Return Type (Phase 3)

**Rationale:** Addresses Concern #3 - generic type parameter

**Location:** Line 599 (Phase 3 model selection signature)

**Change:**
```diff
- pub fn get_model(
-     kind: ModelKind,
-     desc: Option<&str>
- ) -> Result<Client, ModelError> {
+ use rig_core::{Client, Agent};
+
+ pub type ModelClient = Client<Agent>;
+
+ pub fn get_model(
+     kind: ModelKind,
+     desc: Option<&str>
+ ) -> Result<ModelClient, ModelError> {
```

---

### Change 4: Add Benchmark for `research_health` Performance

**Rationale:** NFR-6 specifies <100ms target but no verification planned

**Location:** After line 998 (Performance Considerations)

**Addition:**
```rust
// benches/research_health.rs

use criterion::{black_box, criterion_group, criterion_main, Criterion};
use research_lib::validation::research_health;

fn benchmark_health_check(c: &mut Criterion) {
    // Setup fixture with 10 topics, 6 prompts each, 3 outputs each
    let fixture = create_large_fixture(10);

    c.bench_function("research_health_typical", |b| {
        b.iter(|| {
            research_health(
                black_box("library"),
                black_box("pulldown-cmark")
            )
        })
    });

    c.bench_function("research_health_large_repo", |b| {
        b.iter(|| {
            // Simulate 100 topics
            for i in 0..100 {
                research_health(
                    black_box("library"),
                    black_box(&format!("topic-{}", i))
                ).ok();
            }
        })
    });
}

criterion_group! {
    name = benches;
    config = Criterion::default().significance_level(0.05);
    targets = benchmark_health_check
}
criterion_main!(benches);
```

**Add to Phase 4 acceptance criteria:**
- [ ] Benchmark confirms <100ms for typical topic (6 prompts + 3 outputs)
- [ ] Benchmark confirms <10s for 100 topics (average <100ms each)

---

## Parallelization Notes

**Phases 1-4 can run fully in parallel** with these caveats:

1. **Shared types coordination:** If `ModelProvider` enum (Phase 3) is intended to be auto-generated by Phase 1, there's a hidden dependency. **Clarify:** Should Phase 1 generate the enum that Phase 3 uses? If yes, Phase 3 depends on Phase 1+2.

2. **Test data coordination:** Integration tests for Phase 5 need fixtures from Phase 4. Can prepare fixtures in parallel but final integration must wait.

3. **Cargo.toml merge conflicts:** All phases modify `Cargo.toml`. Use feature branches and coordinate merges:
   - Phase 1: No new deps (existing reqwest/serde)
   - Phase 2: Add `syn`, `quote`
   - Phase 3: No new deps (existing rig-core)
   - Phase 4: No new deps

**Suggested workflow:**
```
Day 1-2:  Phase 1, 2, 3, 4 in parallel (4 developers)
Day 3:    Integration testing + Cargo.toml merge
Day 4:    Phase 5 implementation
Day 5:    Full integration test + manual testing
```

---

## Missing Considerations

### 1. Provider API Versioning

The plan doesn't address what happens when provider APIs change (e.g., OpenAI deprecates `/v1/models`).

**Suggestion:** Add API version tracking to cache:
```rust
#[derive(Serialize, Deserialize)]
struct CachedProviderList {
    version: String,  // e.g., "openai-v1"
    generated_at: DateTime<Utc>,
    models: Vec<LlmEntry>,
}
```

### 2. `inject_enum` Formatting Consistency

After injecting the enum, the file might have inconsistent formatting (e.g., 2-space vs 4-space indents).

**Suggestion:** Add optional `rustfmt` post-processing:
```rust
pub async fn inject_enum(
    name: &str,
    new_enum: &str,
    file_path: &str,
    format: bool,  // Default true
) -> Result<(), CodegenError> {
    // ... existing logic ...

    if format {
        // Run rustfmt on the file
        std::process::Command::new("rustfmt")
            .arg(file_path)
            .output()?;
    }

    Ok(())
}
```

### 3. Model Selection Telemetry

The plan logs to stderr but doesn't capture metrics for analysis (e.g., "how often do we fall back to secondary models?").

**Suggestion:** Add optional metrics collection:
```rust
#[cfg(feature = "metrics")]
use metrics::{counter, histogram};

pub fn get_model(kind: ModelKind, desc: Option<&str>) -> Result<ModelClient, ModelError> {
    let start = std::time::Instant::now();

    for (attempt, model_provider) in stack.0.iter().enumerate() {
        match try_build_client(*model_provider) {
            Ok(client) => {
                #[cfg(feature = "metrics")]
                {
                    counter!("model_selection.success", 1, "kind" => format!("{:?}", kind));
                    histogram!("model_selection.attempt", attempt as f64);
                    histogram!("model_selection.duration_ms", start.elapsed().as_millis() as f64);
                }
                return Ok(client);
            },
            Err(e) => {
                #[cfg(feature = "metrics")]
                counter!("model_selection.fallback", 1);
                continue;
            }
        }
    }

    Err(ModelError::NoValidModel)
}
```

### 4. Research Health - Incremental Validation

`research_health()` re-checks all files on every call. For `research list` with 100 topics, this is 600+ filesystem checks.

**Suggestion:** Add optional caching based on directory mtime:
```rust
use std::time::SystemTime;

#[derive(Clone)]
struct HealthCache {
    last_checked: SystemTime,
    health: ResearchHealth,
}

pub fn research_health_cached(
    research_type: &str,
    topic: &str,
    max_age: Duration,
) -> Result<ResearchHealth, ValidationError> {
    let topic_path = get_topic_path(research_type, topic)?;
    let mtime = fs::metadata(&topic_path)?.modified()?;

    // Check cache
    if let Some(cached) = HEALTH_CACHE.get(&topic_path) {
        if mtime <= cached.last_checked && cached.last_checked.elapsed()? < max_age {
            return Ok(cached.health.clone());
        }
    }

    // Cache miss or stale - re-check
    let health = research_health(research_type, topic)?;
    HEALTH_CACHE.insert(topic_path, HealthCache {
        last_checked: SystemTime::now(),
        health: health.clone(),
    });

    Ok(health)
}
```

### 5. Error Recovery Documentation

The plan has good error types but doesn't document the expected user experience when errors occur.

**Suggestion:** Add "Error Handling UX" section:

```markdown
## Error Handling User Experience

**Provider API Failures:**
- User sees: "⚠️  Failed to fetch models from OpenAI (rate limit). Continuing with cached data..."
- Action: Continue with other providers, use cached data if available
- Retry: Exponential backoff automatic

**Code Injection Failures:**
- User sees: "❌ Cannot inject enum: syntax error in target file at line 42"
- Action: Abort operation, preserve original file
- Retry: Fix syntax error, re-run

**Model Selection Failures:**
- User sees: "❌ No valid model available (ANTHROPIC_API_KEY not set)"
- Action: Abort operation with clear next steps
- Fix: Set API key in environment or `.env`

**Research Health Failures:**
- User sees: "⚠️  Topic 'pulldown-cmark' is missing: Overview, Use Cases"
- Action: Continue with warning, flag in `research list` output
- Fix: Run `research library pulldown-cmark` to regenerate
```

---

## Summary

The plan is **approved with changes** pending resolution of:

1. **CRITICAL:** Replace regex enum removal with AST-based approach (Concern #2)
2. **MAJOR:** Specify rate limiting implementation details (Concern #1)
3. **MAJOR:** Fix generic type parameter in `get_model()` return type (Concern #3)
4. **MAJOR:** Add comprehensive integration test scenarios (Concern #4)

**Estimated implementation time with changes:**
- Phase 1: 2 days (with rate limiting)
- Phase 2: 2 days (AST-based approach more complex than regex)
- Phase 3: 1.5 days (straightforward with type fix)
- Phase 4: 1 day (mostly refactoring)
- Phase 5: 1 day (integration + testing)
- **Total: 7.5 days** (individual contributor, with testing)

**Parallel execution (4 developers):**
- Days 1-2: Phases 1-4 in parallel
- Day 3: Integration + merge
- Day 4: Phase 5 + final testing
- **Total: 4 days**

---

## Next Steps

1. **Clarify open question Q1:** Approve `$HOME/.cache/dockhand/provider_list.json` with 24hr TTL
2. **Resolve Concern #2:** Commit to AST-based approach, drop regex fallback
3. **Add missing dependencies:** `quote = "1.0"` to Phase 2
4. **Expand integration tests:** Add scenarios from Concern #4
5. **Begin implementation** after plan update approved

