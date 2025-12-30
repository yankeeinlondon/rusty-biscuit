# Rust Developer: Phase 3 - Artificial Analysis URL Generator

## Context

You are implementing the `artificial_analysis_url()` function in `/Volumes/coding/personal/dockhand/shared/src/providers/base.rs`.

## Required Skills

- rust
- rust-testing
- rust-logging
- tokio
- thiserror
- reqwest

## Task

Implement `artificial_analysis_url(model: &str) -> Result<Url, ProviderError>` using TDD (Test-Driven Development).

### Function Requirements

The function should:

1. **Strip provider prefix** if present (text before first `/`)
   - Example: `"openai/gpt-4o"` → `"gpt-4o"`

2. **Strip `-preview` suffix** if present
   - Example: `"claude-opus-4.5-20250929-preview"` → `"claude-opus-4.5-20250929"`

3. **Strip `:` and text after** if present
   - Example: `"gpt-4:turbo"` → `"gpt-4"`

4. **Construct URL**: `https://artificialanalysis.ai/models/{cleaned_model_name}`

5. **Return Result<Url, ProviderError>** (NOT panicking Url)
   - Return `Ok(url)` for valid URLs
   - Return `Err(ProviderError::InvalidUrl { model: String })` for invalid input

6. **Handle errors gracefully**:
   - Empty string input
   - Invalid URL characters
   - Never panic

### Error Type

You need to add a new variant to `ProviderError` enum in `/Volumes/coding/personal/dockhand/shared/src/providers/discovery.rs`:

```rust
#[error("Invalid model name for URL generation: {model}")]
InvalidUrl { model: String },
```

### Example Transformations

```rust
// Basic model name
"gpt-4o" → "https://artificialanalysis.ai/models/gpt-4o"

// Provider prefix
"openai/gpt-4o" → "https://artificialanalysis.ai/models/gpt-4o"

// Preview suffix
"claude-opus-4.5-20250929-preview" → "https://artificialanalysis.ai/models/claude-opus-4.5-20250929"

// Colon separator
"gpt-4:turbo" → "https://artificialanalysis.ai/models/gpt-4"

// All transformations combined
"openai/gpt-4-preview:turbo" → "https://artificialanalysis.ai/models/gpt-4"

// Empty string
"" → Err(ProviderError::InvalidUrl { model: "".to_string() })
```

## Implementation Steps (TDD)

### Step 1: Add InvalidUrl error variant

Edit `/Volumes/coding/personal/dockhand/shared/src/providers/discovery.rs` to add the new error variant to the `ProviderError` enum.

### Step 2: Write failing tests FIRST

Add tests in `#[cfg(test)] mod tests` block in `/Volumes/coding/personal/dockhand/shared/src/providers/base.rs`:

**Required Test Scenarios (minimum 8):**

1. Basic model name without transformations
2. Model with provider prefix (e.g., "openai/gpt-4o")
3. Model with `-preview` suffix
4. Model with `:` separator
5. Model with all transformations combined
6. Empty string input (should return Err)
7. Unicode characters in model name
8. Model with special URL characters

**Property-Based Tests (use proptest):**

1. Property: Never panics on arbitrary strings
2. Property: Idempotent transformations (applying twice = applying once)
3. Property: All successful URLs start with `https://artificialanalysis.ai/models/`

### Step 3: Implement the function

Replace the `todo!()` in the `artificial_analysis_url()` function with the actual implementation.

**Implementation hints:**

```rust
pub fn artificial_analysis_url(model: &str) -> Result<Url, ProviderError> {
    // Handle empty input
    if model.is_empty() {
        return Err(ProviderError::InvalidUrl {
            model: model.to_string()
        });
    }

    // Strip provider prefix (text before first '/')
    let model_name = model
        .split_once('/')
        .map(|(_, after)| after)
        .unwrap_or(model);

    // Strip -preview suffix
    let model_name = model_name
        .strip_suffix("-preview")
        .unwrap_or(model_name);

    // Strip : and text after
    let model_name = model_name
        .split_once(':')
        .map(|(before, _)| before)
        .unwrap_or(model_name);

    // Construct URL
    let url_str = format!("https://artificialanalysis.ai/models/{}", model_name);

    // Parse and return
    Url::parse(&url_str).map_err(|_| ProviderError::InvalidUrl {
        model: model.to_string()
    })
}
```

### Step 4: Run tests

```bash
cargo test --lib providers::base artificial_analysis_url
```

Ensure ALL tests pass.

### Step 5: Add property-based tests

Add proptest tests to verify:
- No panics on arbitrary strings
- Idempotent transformations
- Valid URL format

## Acceptance Criteria

- [ ] `ProviderError::InvalidUrl` variant added to discovery.rs
- [ ] `artificial_analysis_url()` implementation exists (no `todo!()`)
- [ ] Returns `Result<Url, ProviderError>` (not bare Url)
- [ ] Function strips provider prefix correctly
- [ ] Function strips `-preview` suffix correctly
- [ ] Function strips `:` and text after correctly
- [ ] Function returns valid Url in Ok variant
- [ ] Function returns Err for invalid input
- [ ] Unit tests cover all transformation cases (minimum 8 scenarios)
- [ ] Property-based tests verify no panics
- [ ] All tests pass: `cargo test --lib providers::base`

## Output Requirements

Return a JSON summary with:

```json
{
  "status": "COMPLETE | PARTIAL | BLOCKED",
  "files_modified": [
    {
      "path": "/absolute/path/to/file",
      "line_count": 123,
      "changes": "description of changes"
    }
  ],
  "tests": {
    "total": 11,
    "passed": 11,
    "failed": 0
  },
  "acceptance_criteria": {
    "error_variant_added": true,
    "implementation_complete": true,
    "returns_result": true,
    "strips_provider_prefix": true,
    "strips_preview_suffix": true,
    "strips_colon_separator": true,
    "returns_valid_url": true,
    "handles_errors": true,
    "unit_tests_complete": true,
    "property_tests_complete": true,
    "all_tests_pass": true
  },
  "issues": []
}
```

## Important Notes

- Use `use url::Url;` (already imported in base.rs)
- Use `use super::super::discovery::ProviderError;` or adjust import path as needed
- NO `unwrap()` or `expect()` in production code
- All tests should be in `#[cfg(test)] mod tests` block
- Property tests should use `proptest!` macro
- Write detailed test names that describe what they test

## Files to Edit

1. `/Volumes/coding/personal/dockhand/shared/src/providers/discovery.rs` - Add InvalidUrl variant
2. `/Volumes/coding/personal/dockhand/shared/src/providers/base.rs` - Implement function and tests

## Log File

Write your progress to: `/Volumes/coding/personal/dockhand/.ai/logs/phase3-rust-developer.log`

Include:
- Timestamp for each step
- Test results
- Any issues encountered
- Final summary

Begin implementation now.
