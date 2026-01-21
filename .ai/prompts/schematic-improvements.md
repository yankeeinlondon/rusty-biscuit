# Schematic Improvements

Future enhancements based on lessons learned from API implementations.

## Generator Improvements

### 1. Configurable Module Paths

**Problem**: The generator assumes `module_name == api_name.to_lowercase()`, but multiple APIs may share a definitions module.

**Current Workaround**: Manual fixups in generated files.

**Proposed Solution**: Add optional `module_path` field to `RestApi`:

```rust
RestApi {
    name: "OllamaNative".to_string(),
    module_path: Some("ollama".to_string()),  // NEW: explicit module path
    // ...
}
```

Generated code would use:

```rust
pub use schematic_definitions::ollama::*;  // Uses module_path instead of lowercased name
```

### 2. Naming Collision Detection

**Problem**: If body types share names with endpoint IDs, the generator creates recursive structs that don't compile.

**Current Workaround**: Convention to use `*Body` suffix for body types.

**Proposed Solution**: Add compile-time validation in generator:

```rust
// In generator, before generating code:
for endpoint in &api.endpoints {
    if let Some(ApiRequest::Json(schema)) = &endpoint.request {
        if schema.type_name == format!("{}Request", endpoint.id) {
            return Err(GeneratorError::NamingCollision {
                endpoint_id: endpoint.id.clone(),
                body_type: schema.type_name.clone(),
                suggestion: format!("{}Body", endpoint.id),
            });
        }
    }
}
```

### 3. Optional Default Derive

**Problem**: All body types must derive `Default`, even when semantically invalid (e.g., required fields become empty strings).

**Current Workaround**: Accept that defaults may be API-invalid but Rust-valid.

**Proposed Solutions**:

**Option A**: Make Default optional via `RestApi` configuration:

```rust
RestApi {
    generate_defaults: false,  // Don't generate Default for wrapper structs
    // ...
}
```

**Option B**: Generate builder pattern instead:

```rust
// Instead of Default:
impl GenerateRequest {
    pub fn new(body: GenerateBody) -> Self { Self { body } }
    pub fn builder() -> GenerateRequestBuilder { ... }
}
```

### 4. Wrapper Struct Naming Configuration

**Problem**: Wrapper structs always use `{EndpointId}Request` pattern, which may conflict with existing type names.

**Proposed Solution**: Add configurable suffix:

```rust
RestApi {
    request_suffix: "Req".to_string(),  // Default: "Request"
    // ...
}
// Generates: GenerateReq instead of GenerateRequest
```

## Definition Tooling Improvements

### 5. Schema Validation CLI

Add a validation subcommand to catch issues before generation:

```bash
schematic-gen validate --api ollama

# Output:
# ✓ No naming collisions detected
# ✓ All body types derive Default
# ⚠ Module path 'ollamanative' doesn't match definitions structure
#   Suggestion: Add module_path: "ollama" to API definition
```

### 6. Re-export Path Inference

Automatically detect the correct module path by scanning `schematic-definitions/src/`:

```rust
// Generator logic:
fn find_module_path(api_name: &str) -> Result<String, GeneratorError> {
    let candidates = [
        api_name.to_lowercase(),           // "ollamanative"
        api_name.to_lowercase().replace("_", ""),  // strip underscores
    ];

    for candidate in candidates {
        if definitions_module_exists(&candidate) {
            return Ok(candidate);
        }
    }

    // Fallback: search for module containing the API definition
    search_definitions_for_api(api_name)
}
```

## Documentation Improvements

### 7. Definition Examples in Generator Output

Include commented examples in generated code showing how to construct requests:

```rust
/// Request for Generate endpoint.
///
/// # Example
/// ```
/// let request = GenerateRequest {
///     body: GenerateBody {
///         model: "llama2".to_string(),
///         prompt: "Hello".to_string(),
///         ..Default::default()
///     },
/// };
/// ```
pub struct GenerateRequest { ... }
```

## Priority

| Improvement | Impact | Effort | Priority |
|-------------|--------|--------|----------|
| 1. Configurable Module Paths | High | Low | **P1** |
| 2. Naming Collision Detection | High | Low | **P1** |
| 3. Optional Default Derive | Medium | Medium | P2 |
| 5. Schema Validation CLI | Medium | Medium | P2 |
| 6. Re-export Path Inference | Medium | High | P3 |
| 4. Wrapper Struct Naming | Low | Low | P3 |
| 7. Definition Examples | Low | Low | P3 |
