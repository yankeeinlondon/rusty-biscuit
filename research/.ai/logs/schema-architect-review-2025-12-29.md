# Schema Architect Review - Utility Functions Implementation Plan
**Date:** 2025-12-29
**Plan:** `/Volumes/coding/personal/dockhand/research/.ai/plans/2025-12-29.utility-functions-implementation.md`
**Reviewer:** Schema Architect Sub-Agent

---

## Overall Assessment

**Status:** Approve with Changes

The plan demonstrates strong data modeling foundations with well-designed types and good use of Rust's type system. However, there are several critical schema design issues that need addressing before implementation, particularly around:

1. **ResearchHealth** ownership and lifetime semantics
2. **ModelProvider** scalability concerns (50+ enum variants)
3. Type system opportunities for compile-time guarantees
4. Missing serde integration patterns

---

## Strengths

### 1. Strong Use of Discriminated Unions

The plan correctly uses Rust enums for state representation:

- **`ProviderListFormat`** - Clean two-variant enum for output formatting
- **`ResearchOutput`** - Already well-designed in existing codebase with `filename()` method and Display trait
- **`ModelKind`** - Good use of enum variants for categories (Fast/Normal/Smart) plus use-case variants (Summarize/Scrape/Consolidate)

### 2. Error Design Follows Best Practices

All proposed error types use `thiserror` with proper error contexts:

```rust
#[derive(Debug, Error)]
pub enum ProviderError {
    #[error("HTTP request failed: {0}")]
    HttpError(#[from] reqwest::Error),

    #[error("Rate limit exceeded for provider {provider}")]
    RateLimitExceeded { provider: String },
}
```

This aligns with Rust error handling conventions and provides good diagnostics.

### 3. Reuse of Existing Validation Infrastructure

The plan correctly identifies reuse opportunities:
- `parse_and_validate_frontmatter()` from `validation/frontmatter.rs`
- `ResearchOutput` enum from `list/types.rs`
- Pattern-matching existing validation logic from `lib.rs`

This demonstrates good schema evolution strategy - extending rather than replacing.

### 4. Type-Level Documentation

The plan includes comprehensive rustdoc comments explaining invariants and business rules, which is essential for schema understanding.

---

## Critical Concerns

### 🔴 Concern 1: ResearchHealth String Ownership

**Issue:** `ResearchHealth.research_type` uses owned `String` where it should use a reference or enum.

```rust
// CURRENT PLAN (Phase 4, line 705)
pub struct ResearchHealth {
    pub research_type: String,  // ❌ Owned string, no validation
    pub topic: String,
    // ...
}
```

**Problems:**
1. No compile-time validation that `research_type` is valid
2. Wastes memory with owned strings for fixed set of values
3. Allows invalid states: `research_type: "invalid_type"` compiles

**Solution:** Use an enum instead:

```rust
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ResearchType {
    Library,
    Tool,
    Software,
    Framework,
    // Extensible for future types
}

pub struct ResearchHealth {
    pub research_type: ResearchType,  // ✅ Type-safe
    pub topic: String,  // Keep as String (user-provided)
    pub ok: bool,
    pub missing_underlying: Vec<String>,
    pub missing_deliverables: Vec<ResearchOutput>,
    pub skill_structure_valid: bool,
}
```

**Benefits:**
- Invalid research types become compile errors
- Smaller memory footprint (enum vs String)
- Better serialization control (serde rename_all)
- Enables exhaustive pattern matching

**Impact:** Requires updating `research_health()` signature:
```rust
// Before
pub fn research_health(research_type: &str, topic: &str) -> Result<ResearchHealth>

// After
pub fn research_health(research_type: ResearchType, topic: &str) -> Result<ResearchHealth>
```

### 🔴 Concern 2: ModelProvider Scalability

**Issue:** Enum with 50+ variants will be hard to maintain and extend.

```rust
// PLAN (Phase 3, line 512)
pub enum ModelProvider {
    AnthropicClaudeOpus4_5,
    AnthropicClaudeSonnet4_5,
    AnthropicClaudeHaiku4_5,
    OpenaiGpt5_2,
    OpenaiO3,
    GeminiFlash3,
    // ... 50+ more variants
}
```

**Problems:**
1. Adding new models requires enum modification
2. Every match statement needs updating (exhaustiveness)
3. Cannot dynamically load models from API
4. Naming inconsistencies (underscores vs camelCase)

**Solution:** Use a structured type with validation:

```rust
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct ModelProvider {
    provider: String,  // "anthropic", "openai", "gemini"
    model: String,     // "claude-sonnet-4-5-20250929", "gpt-5.2"
}

impl ModelProvider {
    /// Construct with validation
    pub fn new(provider: impl Into<String>, model: impl Into<String>) -> Self {
        Self {
            provider: provider.into().to_lowercase(),
            model: model.into(),
        }
    }

    /// Convenience constructors for common models
    pub fn anthropic_sonnet_4_5() -> Self {
        Self::new("anthropic", "claude-sonnet-4-5-20250929")
    }

    pub fn openai_gpt_5_2() -> Self {
        Self::new("openai", "gpt-5.2")
    }

    pub fn to_rig_identifier(&self) -> (&str, &str) {
        (&self.provider, &self.model)
    }
}

// For predefined, validated models use a registry pattern
pub mod models {
    use super::ModelProvider;

    pub const ANTHROPIC_SONNET_4_5: ModelProvider =
        ModelProvider { provider: "anthropic", model: "claude-sonnet-4-5-20250929" };
    pub const OPENAI_GPT_5_2: ModelProvider =
        ModelProvider { provider: "openai", model: "gpt-5.2" };
    // ...
}
```

**Alternative:** Keep enum for type safety but auto-generate from provider list:

```rust
// Generate this via build.rs using generate_provider_list()
#[derive(Debug, Clone, Copy)]
pub enum KnownModel {
    #[cfg(feature = "anthropic")]
    AnthropicClaudeSonnet4_5,

    #[cfg(feature = "openai")]
    OpenaiGpt5_2,
}

impl KnownModel {
    pub fn to_provider(&self) -> ModelProvider {
        match self {
            Self::AnthropicClaudeSonnet4_5 =>
                ModelProvider::new("anthropic", "claude-sonnet-4-5-20250929"),
            // ...
        }
    }
}
```

**Recommendation:** Use struct-based approach for MVP, add const constructors for common models.

### 🟡 Concern 3: Missing Serde Derives on Core Types

**Issue:** Plan doesn't specify serde integration for key types that will need serialization.

Types needing `Serialize/Deserialize`:
- **`ResearchHealth`** - Will be serialized for CLI JSON output
- **`ModelProvider`** (if struct) - For caching/logging
- **`ModelStack`** - For configuration files
- **`LlmEntry`** - Already has derives (✅)

**Solution:** Add explicit serde derives to plan:

```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResearchHealth {
    #[serde(rename = "type")]
    pub research_type: ResearchType,
    pub topic: String,
    pub ok: bool,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub missing_underlying: Vec<String>,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub missing_deliverables: Vec<ResearchOutput>,
    pub skill_structure_valid: bool,
}
```

Note: Use `skip_serializing_if` for empty collections to reduce JSON noise.

### 🟡 Concern 4: ModelKind::TryExplicit Type Complexity

**Issue:** Recursive enum with boxing may have lifetime complications.

```rust
// PLAN (Phase 3, line 552)
pub enum ModelKind {
    // ...
    TryExplicit(ModelProvider, Box<ModelKind>),
}
```

**Problems:**
1. Boxing adds heap allocation overhead
2. Recursive enums complicate pattern matching
3. Type signature is opaque: what does `TryExplicit(model, Fast)` mean?

**Solution:** Make semantics explicit with a struct:

```rust
pub enum ModelKind {
    Fast,
    Normal,
    Smart,
    Summarize,
    Scrape,
    Consolidate,
    /// Try explicit model, fall back to category on failure
    TryExplicitWithFallback {
        preferred: ModelProvider,
        fallback: Box<ModelKind>,
    },
}

// Usage becomes clearer:
ModelKind::TryExplicitWithFallback {
    preferred: ModelProvider::anthropic_opus_4_5(),
    fallback: Box::new(ModelKind::Smart),
}
```

**Alternative:** Flatten the hierarchy:

```rust
pub struct ModelSelection {
    pub kind: ModelKind,
    pub explicit_override: Option<ModelProvider>,
}

impl ModelSelection {
    pub fn fast() -> Self {
        Self { kind: ModelKind::Fast, explicit_override: None }
    }

    pub fn try_explicit(model: ModelProvider, fallback: ModelKind) -> Self {
        Self { kind: fallback, explicit_override: Some(model) }
    }
}
```

This separates concerns: `kind` determines fallback stack, `explicit_override` tries first.

### 🟢 Concern 5: LlmEntry Field Validation

**Issue:** `LlmEntry` has no validation of provider/model strings.

```rust
// PLAN (Phase 1, line 216)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LlmEntry {
    pub provider: String,
    pub model: String,
}
```

**Enhancement:** Add validation and normalization methods:

```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LlmEntry {
    pub provider: String,
    pub model: String,
}

impl LlmEntry {
    /// Construct with normalization
    pub fn new(provider: impl Into<String>, model: impl Into<String>) -> Self {
        Self {
            provider: provider.into().to_lowercase().replace(" ", "_"),
            model: model.into(),
        }
    }

    /// Validate provider/model are non-empty
    pub fn validate(&self) -> Result<(), ValidationError> {
        if self.provider.is_empty() {
            return Err(ValidationError::EmptyProvider);
        }
        if self.model.is_empty() {
            return Err(ValidationError::EmptyModel);
        }
        Ok(())
    }

    /// Format as "provider/model" string
    pub fn as_identifier(&self) -> String {
        format!("{}/{}", self.provider, self.model)
    }
}
```

---

## Suggested Changes

### Change 1: Add ResearchType Enum (Phase 4)

**File:** `research/lib/src/validation/health.rs`

```rust
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ResearchType {
    Library,
    Tool,
    Software,
    Framework,
}

impl ResearchType {
    /// Parse from string (for CLI compatibility)
    pub fn from_str(s: &str) -> Result<Self, ValidationError> {
        match s.to_lowercase().as_str() {
            "library" => Ok(Self::Library),
            "tool" => Ok(Self::Tool),
            "software" => Ok(Self::Software),
            "framework" => Ok(Self::Framework),
            _ => Err(ValidationError::InvalidResearchType(s.to_string())),
        }
    }
}

// Update ResearchHealth to use enum
pub struct ResearchHealth {
    pub research_type: ResearchType,  // Changed from String
    // ... rest unchanged
}

// Update function signature
pub fn research_health(
    research_type: ResearchType,  // Changed from &str
    topic: &str
) -> Result<ResearchHealth, ValidationError>
```

**Update call sites in Phase 5:**
```rust
// Before
let health = research_health("library", &topic)?;

// After
let research_type = ResearchType::from_str("library")?;
let health = research_health(research_type, &topic)?;

// Or directly:
let health = research_health(ResearchType::Library, &topic)?;
```

### Change 2: Restructure ModelProvider (Phase 3)

**File:** `shared/src/model/types.rs`

Replace enum-based approach with struct + const pattern:

```rust
/// Identifies a specific LLM provider and model combination
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct ModelProvider {
    provider: String,
    model: String,
}

impl ModelProvider {
    pub const fn new_const(provider: &'static str, model: &'static str) -> Self {
        Self {
            provider: provider,
            model: model,
        }
    }

    pub fn to_rig_identifier(&self) -> (&str, &str) {
        (&self.provider, &self.model)
    }
}

// Common models as constants
pub mod models {
    use super::ModelProvider;

    pub const ANTHROPIC_SONNET_4_5: ModelProvider =
        ModelProvider::new_const("anthropic", "claude-sonnet-4-5-20250929");

    pub const OPENAI_GPT_5_2: ModelProvider =
        ModelProvider::new_const("openai", "gpt-5.2");

    // ... etc
}
```

### Change 3: Add Serde Derives to ResearchHealth

**File:** `research/lib/src/validation/health.rs`

```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResearchHealth {
    #[serde(rename = "type")]
    pub research_type: ResearchType,

    pub topic: String,
    pub ok: bool,

    #[serde(skip_serializing_if = "Vec::is_empty", default)]
    pub missing_underlying: Vec<String>,

    #[serde(skip_serializing_if = "Vec::is_empty", default)]
    pub missing_deliverables: Vec<ResearchOutput>,

    pub skill_structure_valid: bool,
}
```

### Change 4: Enhance LlmEntry with Validation

**File:** `shared/src/providers/types.rs`

```rust
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct LlmEntry {
    pub provider: String,
    pub model: String,
}

impl LlmEntry {
    pub fn new(provider: impl Into<String>, model: impl Into<String>) -> Self {
        Self {
            provider: provider.into().to_lowercase().replace(" ", "_"),
            model: model.into(),
        }
    }

    pub fn as_identifier(&self) -> String {
        format!("{}/{}", self.provider, self.model)
    }

    pub fn validate(&self) -> bool {
        !self.provider.is_empty() && !self.model.is_empty()
    }
}
```

### Change 5: Simplify ModelKind::TryExplicit

**File:** `shared/src/model/types.rs`

```rust
pub enum ModelKind {
    Fast,
    Normal,
    Smart,
    Summarize,
    Scrape,
    Consolidate,
}

pub struct ModelSelection {
    pub kind: ModelKind,
    pub explicit_first: Option<ModelProvider>,
}

impl ModelSelection {
    pub fn from_kind(kind: ModelKind) -> Self {
        Self { kind, explicit_first: None }
    }

    pub fn try_explicit(model: ModelProvider, fallback: ModelKind) -> Self {
        Self { kind: fallback, explicit_first: Some(model) }
    }
}

// Update get_model signature
pub fn get_model(
    selection: ModelSelection,
    desc: Option<&str>
) -> Result<Client, ModelError>
```

---

## Parallelization Notes

### Schema-Related Dependencies

**Phase 1-4 Parallel Execution Safe:**
- ✅ Phase 1 (Provider Discovery) - Independent schemas
- ✅ Phase 2 (Code Injection) - Independent schemas
- ✅ Phase 3 (Model Selection) - Independent schemas (but see Change 2)
- ✅ Phase 4 (Research Health) - Depends on existing `ResearchOutput` enum (already exists)

**No schema conflicts** between phases - can run in parallel.

**Phase 5 Integration:**
- 🟡 Requires Phase 4 completion
- Must handle `ResearchType` enum parsing from CLI strings
- Update all call sites to use `ResearchType::from_str()` or direct enum values

---

## Missing Considerations

### 1. Schema Versioning Strategy

**Missing:** How will these schemas evolve over time?

**Recommendations:**

**For ResearchHealth:**
```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResearchHealth {
    #[serde(default = "default_version")]
    pub version: u8,  // Schema version

    pub research_type: ResearchType,
    // ... rest of fields
}

fn default_version() -> u8 { 1 }
```

**For LlmEntry (provider list format):**
```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProviderListResponse {
    pub version: String,  // "1.0"
    pub entries: Vec<LlmEntry>,
    pub generated_at: Option<String>,  // ISO8601 timestamp
}
```

### 2. NewType Pattern Opportunities

**Missing:** Domain-specific type wrappers for validation.

**Examples:**

```rust
// For topic names (alphanumeric + hyphens only)
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TopicName(String);

impl TopicName {
    pub fn new(s: impl Into<String>) -> Result<Self, ValidationError> {
        let s = s.into();
        if s.is_empty() || !s.chars().all(|c| c.is_alphanumeric() || c == '-' || c == '_') {
            return Err(ValidationError::InvalidTopicName(s));
        }
        Ok(Self(s))
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

// Update ResearchHealth
pub struct ResearchHealth {
    pub research_type: ResearchType,
    pub topic: TopicName,  // Type-safe topic names
    // ...
}
```

### 3. Default Implementations

**Missing:** Sensible defaults for partial construction.

**Recommendations:**

```rust
impl Default for ResearchHealth {
    fn default() -> Self {
        Self {
            research_type: ResearchType::Library,
            topic: String::new(),
            ok: false,
            missing_underlying: Vec::new(),
            missing_deliverables: Vec::new(),
            skill_structure_valid: false,
        }
    }
}
```

### 4. Builder Pattern for ModelStack

**Missing:** Ergonomic construction for complex stacks.

```rust
pub struct ModelStackBuilder {
    models: Vec<ModelProvider>,
}

impl ModelStackBuilder {
    pub fn new() -> Self {
        Self { models: Vec::new() }
    }

    pub fn add(mut self, model: ModelProvider) -> Self {
        self.models.push(model);
        self
    }

    pub fn fast_defaults(mut self) -> Self {
        self.models.extend([
            models::ANTHROPIC_HAIKU_4_5,
            models::GEMINI_FLASH_3,
        ]);
        self
    }

    pub fn build(self) -> ModelStack {
        ModelStack(self.models)
    }
}

// Usage
let stack = ModelStackBuilder::new()
    .add(models::ANTHROPIC_OPUS_4_5)
    .fast_defaults()
    .build();
```

### 5. Const Validation

**Missing:** Compile-time guarantees where possible.

**Example:** Validate standard prompt filenames are lowercase:

```rust
// In check_missing_prompts (Phase 4, line 782)
const STANDARD_PROMPTS: &[(&str, &str)] = &[
    ("Overview", "overview.md"),
    ("Similar Libraries", "similar_libraries.md"),
    // ...
];

// Add compile-time check (using const fn when stable)
const fn validate_filename(s: &str) -> &str {
    // Future: check s is lowercase, no spaces
    s
}
```

---

## Evolution & Versioning Considerations

### Backwards Compatibility

**ResearchHealth:**
- Adding new fields is safe (use `#[serde(default)]`)
- Removing fields is breaking
- **Recommendation:** Add `version` field from day 1

**ModelProvider:**
- Struct approach allows adding providers without code changes
- Const pattern allows deprecating old models without breaking builds
- **Recommendation:** Use feature flags for experimental providers

**LlmEntry:**
- Current design is stable
- Can add optional fields (`api_version`, `endpoint_url`) later
- **Recommendation:** Already good for evolution

### Migration Path

**From scattered validation to research_health():**

1. **Phase 4:** Implement `research_health()` in parallel with old functions
2. **Phase 5:** Migrate call sites, mark old functions `#[deprecated]`
3. **Future:** Remove deprecated functions in breaking release

**Deprecation strategy:**
```rust
#[deprecated(
    since = "0.2.0",
    note = "Use validation::health::research_health() instead"
)]
pub fn check_missing_standard_prompts(path: &Path) -> Vec<String> {
    // Forward to new implementation
    let health = research_health(ResearchType::Library, "temp").unwrap();
    health.missing_underlying
}
```

---

## Testing Implications

### Schema-Specific Tests Needed

**Phase 1 (LlmEntry):**
```rust
#[test]
fn test_llm_entry_normalization() {
    let entry = LlmEntry::new("OpenAI", "gpt-5.2");
    assert_eq!(entry.provider, "openai");
    assert_eq!(entry.as_identifier(), "openai/gpt-5.2");
}

#[test]
fn test_llm_entry_validation() {
    let valid = LlmEntry::new("anthropic", "claude-3");
    assert!(valid.validate());

    let invalid = LlmEntry { provider: "".into(), model: "test".into() };
    assert!(!invalid.validate());
}

#[test]
fn test_llm_entry_serde_roundtrip() {
    let entry = LlmEntry::new("anthropic", "claude-sonnet-4-5");
    let json = serde_json::to_string(&entry).unwrap();
    let parsed: LlmEntry = serde_json::from_str(&json).unwrap();
    assert_eq!(entry, parsed);
}
```

**Phase 4 (ResearchHealth):**
```rust
#[test]
fn test_research_type_parsing() {
    assert_eq!(ResearchType::from_str("library").unwrap(), ResearchType::Library);
    assert_eq!(ResearchType::from_str("TOOL").unwrap(), ResearchType::Tool);
    assert!(ResearchType::from_str("invalid").is_err());
}

#[test]
fn test_research_health_serialization() {
    let health = ResearchHealth {
        research_type: ResearchType::Library,
        topic: "test".into(),
        ok: false,
        missing_underlying: vec!["overview.md".into()],
        missing_deliverables: vec![ResearchOutput::Skill],
        skill_structure_valid: false,
    };

    let json = serde_json::to_string_pretty(&health).unwrap();
    let parsed: ResearchHealth = serde_json::from_str(&json).unwrap();

    assert_eq!(parsed.research_type, health.research_type);
    assert_eq!(parsed.missing_underlying, health.missing_underlying);
}

#[test]
fn test_research_health_empty_vecs_omitted() {
    let health = ResearchHealth {
        research_type: ResearchType::Library,
        topic: "test".into(),
        ok: true,
        missing_underlying: vec![],  // Should be omitted from JSON
        missing_deliverables: vec![],  // Should be omitted from JSON
        skill_structure_valid: true,
    };

    let json = serde_json::to_string(&health).unwrap();
    assert!(!json.contains("missing_underlying"));
    assert!(!json.contains("missing_deliverables"));
}
```

**Phase 3 (ModelProvider struct):**
```rust
#[test]
fn test_model_provider_construction() {
    let provider = models::ANTHROPIC_SONNET_4_5;
    let (prov, model) = provider.to_rig_identifier();
    assert_eq!(prov, "anthropic");
    assert_eq!(model, "claude-sonnet-4-5-20250929");
}

#[test]
fn test_model_provider_equality() {
    let p1 = ModelProvider::new_const("anthropic", "claude-sonnet-4-5");
    let p2 = ModelProvider::new_const("anthropic", "claude-sonnet-4-5");
    assert_eq!(p1, p2);
}
```

### Property-Based Testing Opportunities

**Provider normalization:**
```rust
use proptest::prelude::*;

proptest! {
    #[test]
    fn prop_provider_normalization_lowercase(s in "[A-Za-z0-9 -]+") {
        let entry = LlmEntry::new(&s, "model");
        assert!(entry.provider.chars().all(|c| c.is_lowercase() || c == '_'));
    }

    #[test]
    fn prop_research_type_roundtrip(rt in prop_oneof![
        Just(ResearchType::Library),
        Just(ResearchType::Tool),
        Just(ResearchType::Software),
        Just(ResearchType::Framework),
    ]) {
        let json = serde_json::to_string(&rt).unwrap();
        let parsed: ResearchType = serde_json::from_str(&json).unwrap();
        assert_eq!(rt, parsed);
    }
}
```

---

## Final Recommendations

### Critical (Must Address Before Implementation)

1. **Change ResearchHealth.research_type to enum** (Concern 1)
2. **Reconsider ModelProvider as 50+ variant enum** (Concern 2) - use struct or const pattern
3. **Add serde derives to ResearchHealth** (Concern 3)

### High Priority (Should Address)

4. **Simplify ModelKind::TryExplicit** (Concern 4) - use struct with optional explicit field
5. **Add validation to LlmEntry** (Concern 5)
6. **Add schema versioning fields** (Missing Consideration 1)

### Medium Priority (Nice to Have)

7. **Consider NewType pattern for TopicName** (Missing Consideration 2)
8. **Add Default implementations** (Missing Consideration 3)
9. **Implement builder pattern for ModelStack** (Missing Consideration 4)

### Low Priority (Future Enhancements)

10. **Const validation for standard prompts** (Missing Consideration 5)
11. **Auto-generate ModelProvider enum via build.rs** (Alternative to Concern 2)

---

## Approval Conditions

This plan is **approved for implementation** pending the following changes:

### Required Before Starting Phase 4:
- [ ] Add `ResearchType` enum definition to Phase 4 deliverables
- [ ] Update `research_health()` signature to use `ResearchType` instead of `&str`
- [ ] Add `#[derive(Serialize, Deserialize)]` to `ResearchHealth` struct
- [ ] Add serde attributes (`skip_serializing_if`, `default`) to `ResearchHealth` fields

### Required Before Starting Phase 3:
- [ ] Decide on `ModelProvider` design: enum vs struct vs hybrid
- [ ] Document the chosen approach in the plan
- [ ] If using enum, add plan for auto-generation from `generate_provider_list()`
- [ ] If using struct, add const pattern for common models

### Optional (Recommended):
- [ ] Add schema version fields to `ResearchHealth` and `ProviderListResponse`
- [ ] Enhance `LlmEntry` with validation methods
- [ ] Simplify `ModelKind::TryExplicit` to use struct or flatten

---

## Summary for Orchestrator

**Schema Architect has reviewed the Utility Functions Implementation Plan.**

**Status:** ✅ Approved with Changes

**Key Issues:**
1. ResearchHealth uses String where enum is needed (type safety)
2. ModelProvider as 50+ variant enum won't scale (maintenance burden)
3. Missing serde integration for serialization
4. Some type signatures can be improved for compile-time guarantees

**Impact:** Changes required in Phase 3 (ModelProvider design) and Phase 4 (ResearchType enum) before implementation begins. Phase 1-2 can proceed as-is.

**Next Steps:**
1. Plan author addresses critical concerns (ResearchType enum, ModelProvider design)
2. Update plan with serde derives and validation methods
3. Proceed with parallel Phase 1-4 implementation
4. Schema Architect available for follow-up review if needed

---

**Review Complete**
**Reviewer:** Schema Architect Sub-Agent
**Date:** 2025-12-29
