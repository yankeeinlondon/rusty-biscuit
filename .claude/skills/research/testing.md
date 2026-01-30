# Testing the Research Package

Testing patterns and practices for the research library and CLI.

## Running Tests

```bash
# All tests
cargo test -p research-lib
cargo test -p research-cli

# Using justfile
just -f research/justfile test

# Specific test
cargo test -p research-lib test_metadata_migration

# With output
cargo test -p research-lib -- --nocapture
```

## Test Dependencies

| Crate | Purpose |
|-------|---------|
| `wiremock` | HTTP mocking for provider APIs |
| `tempfile` | Temporary directories for output tests |
| `serial_test` | Test isolation for environment variables |
| `tracing-test` | Tracing assertions with `#[traced_test]` |

## Environment Variable Tests

Tests that manipulate environment variables must use `#[serial_test::serial]`:

```rust
use serial_test::serial;

#[test]
#[serial]
fn test_research_dir_env_var() {
    std::env::set_var("RESEARCH_DIR", "/tmp/test");
    // ... test logic
    std::env::remove_var("RESEARCH_DIR");
}
```

This prevents race conditions when multiple tests modify the same variables.

## Mocking HTTP Requests

Use `wiremock` for package manager API tests:

```rust
use wiremock::{Mock, MockServer, ResponseTemplate, matchers::*};

#[tokio::test]
async fn test_crates_io_detection() {
    let mock_server = MockServer::start().await;

    Mock::given(method("GET"))
        .and(path("/api/v1/crates/clap"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({
            "crate": {
                "name": "clap",
                "description": "Command Line Argument Parser"
            }
        })))
        .mount(&mock_server)
        .await;

    // Test against mock_server.uri()
}
```

## Temporary Directory Tests

Use `tempfile` for output directory tests:

```rust
use tempfile::TempDir;

#[test]
fn test_metadata_save() {
    let temp = TempDir::new().unwrap();
    let path = temp.path().join("metadata.json");

    let metadata = ResearchMetadata::new("test-topic");
    metadata.save_to(&path).unwrap();

    assert!(path.exists());
    // temp directory cleaned up automatically
}
```

## Tracing Tests

Use `tracing-test` for logging assertions:

```rust
use tracing_test::traced_test;

#[tokio::test]
#[traced_test]
async fn test_tool_invocation_logged() {
    // ... trigger tool call

    assert!(logs_contain("Invoking tool"));
    assert!(logs_contain("tool.name"));
}
```

## Test Fixtures

### Metadata Fixtures

```rust
fn create_test_metadata() -> ResearchMetadata {
    ResearchMetadata {
        schema_version: 1,
        kind: ResearchDetails::Library(LibraryDetails {
            package_manager: Some("crates.io".to_string()),
            language: Some("Rust".to_string()),
            url: None,
            repository: None,
        }),
        additional_files: HashMap::new(),
        created_at: Utc::now(),
        updated_at: Utc::now(),
        brief: None,
        summary: None,
        when_to_use: None,
    }
}
```

### Topic Fixtures

```rust
fn create_test_topic(name: &str) -> Topic {
    Topic::new(
        name.to_string(),
        KindCategory::Software(Software::new(name.to_string())),
    )
}
```

## Integration Test Patterns

### Full Pipeline Test

```rust
#[tokio::test]
#[ignore]  // Requires API keys
async fn test_full_research_pipeline() {
    let temp = TempDir::new().unwrap();
    let output_dir = temp.path().join("test-lib");

    let result = research(
        "test-lib",
        Some(output_dir.clone()),
        &[],
        false,
        false,
    ).await.unwrap();

    assert!(result.succeeded > 0);
    assert!(output_dir.join("metadata.json").exists());
    assert!(output_dir.join("overview.md").exists());
}
```

### Incremental Mode Test

```rust
#[tokio::test]
async fn test_incremental_adds_questions() {
    let temp = TempDir::new().unwrap();

    // First run
    // ... create initial metadata

    // Second run with questions
    let result = research(
        "test-lib",
        Some(temp.path().to_path_buf()),
        &["New question?".to_string()],
        false,
        false,
    ).await;

    // Verify question file created
    assert!(temp.path().join("question_1.md").exists());
}
```

## Common Test Issues

### API Key Requirements

Tests hitting real APIs need environment variables:
```bash
OPENAI_API_KEY=... GEMINI_API_KEY=... cargo test -p research-lib
```

Mark expensive tests with `#[ignore]`:
```rust
#[test]
#[ignore]
fn test_requires_api_keys() { }
```

Run ignored tests explicitly:
```bash
cargo test -p research-lib -- --ignored
```

### Async Test Timeouts

Set timeouts for async tests:
```rust
#[tokio::test(flavor = "multi_thread")]
async fn test_with_timeout() {
    tokio::time::timeout(
        Duration::from_secs(30),
        async_operation()
    ).await.unwrap();
}
```

### File System Race Conditions

Use unique temp directories per test:
```rust
#[test]
fn test_file_operations() {
    let temp = TempDir::with_prefix("research-test-").unwrap();
    // Unique prefix prevents collisions
}
```

## Test Coverage

Key areas to test:

1. **Metadata serialization**: v0/v1 roundtrip, migration
2. **Package manager detection**: All 6 managers
3. **Overlap detection**: New vs conflicting prompts
4. **Output structure**: All expected files created
5. **Error handling**: Missing files, invalid JSON, network errors
6. **Cancellation**: Partial completion, preserved results
