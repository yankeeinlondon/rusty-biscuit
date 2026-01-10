# XSD Validation Strategy for the `api` Package

**Date**: 2026-01-10
**Status**: Proposed
**Author**: Research via Claude Code

## Background

The original plan assumed `quick-xml` could validate XML against XSD schemas. This is incorrect: `quick-xml` is a parser/serializer only, with no XSD validation capabilities. This document evaluates alternatives for runtime XSD validation in Rust.

## Candidates Evaluated

### 1. xmlschema (v0.0.1)

**Repository**: [github.com/sebastienrousseau/xmlschema](https://github.com/sebastienrousseau/xmlschema)

| Aspect | Details |
|--------|---------|
| Runtime validation | Claimed, but unverified |
| Pure Rust | Yes |
| XSD support | XSD 1.0 and 1.1 (claimed) |
| Maintenance | Very early stage (v0.0.1, 5 commits, Feb 2023) |
| Documentation | Minimal, no working examples |

**Pros**:
- Pure Rust, no C dependencies
- Claims XSD 1.0/1.1 support
- MIT/Apache-2.0 dual license

**Cons**:
- Extremely immature (v0.0.1)
- No concrete usage examples in README
- No evidence of production use
- May be vaporware (stated goals without implementation)

**Verdict**: **Not recommended** - Too immature for production use.

---

### 2. xsd-parser (v1.4.0)

**Repository**: [github.com/Bergmann89/xsd-parser](https://github.com/Bergmann89/xsd-parser)
**Documentation**: [docs.rs/xsd-parser](https://docs.rs/xsd-parser)

| Aspect | Details |
|--------|---------|
| Runtime validation | No - code generation only |
| Pure Rust | Yes |
| XSD support | Parses XSD to generate Rust types |
| Maintenance | Active (260 commits, v1.4.0, Jan 2025) |
| Documentation | Good |

**Pros**:
- Actively maintained
- Well-architected staged pipeline
- Generates type-safe Rust structs from XSD
- Works with quick-xml and serde

**Cons**:
- Does NOT provide runtime XSD validation
- Compile-time code generation approach only
- "Schema-Based Validation" listed as planned, not implemented

**Verdict**: **Not suitable** - Solves a different problem (code generation, not runtime validation).

---

### 3. libxml (v0.3.8) - libxml2 bindings

**Repository**: [github.com/KWARC/rust-libxml](https://github.com/KWARC/rust-libxml)
**Documentation**: [docs.rs/libxml](https://docs.rs/libxml)

| Aspect | Details |
|--------|---------|
| Runtime validation | Yes, via libxml2's xmlschemas module |
| Pure Rust | No - FFI to C library |
| XSD support | XSD 1.0 (full), XSD 1.1 (partial via libxml2) |
| Maintenance | Active (Sept 2025, v0.3.8) |
| Documentation | Good, includes schema example |

**Pros**:
- Proven XSD validation via libxml2
- Battle-tested C library underneath
- Supports schema validation, XPath, DOM manipulation
- Good Rust wrapper with safety guarantees

**Cons**:
- Requires libxml2-dev and CLang for builds
- **Not thread-safe** (libxml2 limitation)
- Cross-platform build complexity (macOS needs Homebrew, Windows needs vcpkg)
- Adds C dependency to otherwise pure-Rust project

**Verdict**: **Viable but heavyweight** - Best validation quality, but significant build complexity.

---

### 4. raxb-validate (v0.5.1) - RAXB Framework

**Repository**: [github.com/hd-gmbh-dev/raxb](https://github.com/hd-gmbh-dev/raxb)
**Documentation**: [docs.rs/raxb-validate](https://docs.rs/raxb-validate)

| Aspect | Details |
|--------|---------|
| Runtime validation | Yes, via raxb-libxml2-sys |
| Pure Rust | No - builds libxml2 from source via CMake |
| XSD support | XSD 1.0 via libxml2 |
| Maintenance | Active (Oct 2025, v0.5.1) |
| Documentation | Sparse (3.57% documented) |

**Pros**:
- Builds libxml2 from source (more portable than system dependency)
- Part of larger XML binding framework
- Integrates with quick-xml for serialization

**Cons**:
- Still requires libxml2 (just built from source)
- Very sparse documentation
- Small community (5 GitHub stars)
- CMake build dependency

**Verdict**: **Viable alternative to libxml** - Similar tradeoffs with slightly better portability.

---

### 5. No Runtime Validation (Parse-only mode)

| Aspect | Details |
|--------|---------|
| Runtime validation | No |
| Pure Rust | Yes (quick-xml only) |
| Approach | Skip XSD validation, rely on serde deserialization |

**Pros**:
- No new dependencies
- Already implemented with quick-xml
- Fast and simple
- Works for most use cases where schema conformance is guaranteed

**Cons**:
- No formal schema validation
- Type mismatches caught at deserialization, not validation
- Less informative error messages

**Verdict**: **Recommended default** - Pragmatic choice given ecosystem limitations.

---

## Comparison Matrix

| Crate | Runtime Validation | Pure Rust | Maturity | Thread-Safe | Build Complexity |
|-------|:-----------------:|:---------:|:--------:|:-----------:|:----------------:|
| xmlschema | Unverified | Yes | Very Low | Unknown | Low |
| xsd-parser | No | Yes | High | N/A | Low |
| libxml | Yes | No | High | No | High |
| raxb-validate | Yes | No | Medium | No | Medium |
| Parse-only | No | Yes | N/A | Yes | None |

## Recommendation

### Primary Strategy: Feature-Gated Optional Validation

Implement a **tiered approach** with optional XSD validation behind a feature flag:

1. **Default mode**: Parse-only with quick-xml (no new dependencies)
2. **Optional `xsd-validation` feature**: Enable libxml-based validation when needed

This preserves the lightweight nature of the package while allowing users who need XSD validation to opt in.

### Recommended Crate: `libxml` (v0.3.8)

When XSD validation is required, `libxml` is the most mature option:
- Proven libxml2 validation underneath
- Active maintenance (Sept 2025)
- Good documentation with schema examples
- Used in production by other projects

The thread-safety limitation is acceptable because:
- Validation can be performed in a dedicated thread/task
- Async HTTP calls can serialize validation through a single executor
- Most API client use cases don't require parallel validation of the same schema

## API Wrapper Design

### XmlSchema Trait (existing)

The current trait design is already well-suited for optional validation:

```rust
pub trait XmlSchema: serde::de::DeserializeOwned + Send + Sync {
    fn xsd_schema() -> Option<std::borrow::Cow<'static, str>> {
        None
    }
}
```

### XmlFormat with Optional Validation

```rust
// In response/format.rs

impl<X: XmlSchema + DeserializeOwned + Send + Sync> ResponseFormat for XmlFormat<X> {
    type Output = X;

    async fn parse(body: bytes::Bytes) -> Result<Self::Output, ValidationError> {
        // Step 1: Parse XML with quick-xml (always happens)
        let parsed: X = quick_xml::de::from_reader(body.as_ref())
            .map_err(ValidationError::XmlParse)?;

        // Step 2: Validate against XSD if available and feature enabled
        #[cfg(feature = "xsd-validation")]
        if let Some(schema) = X::xsd_schema() {
            validate_xsd(&body, &schema)?;
        }

        Ok(parsed)
    }

    fn content_type() -> &'static str {
        "application/xml"
    }
}
```

### Validation Module (feature-gated)

```rust
// In response/xsd.rs (only compiled with xsd-validation feature)

#[cfg(feature = "xsd-validation")]
mod xsd {
    use libxml::parser::Parser;
    use libxml::schemas::{SchemaParserContext, SchemaValidationContext};
    use crate::error::ValidationError;

    /// Validate XML content against an XSD schema.
    pub fn validate_xsd(xml: &[u8], xsd: &str) -> Result<(), ValidationError> {
        // Parse the XSD schema
        let schema_parser = SchemaParserContext::from_buffer(xsd.as_bytes())
            .map_err(|e| ValidationError::XsdValidation(format!("Schema parse error: {e}")))?;

        let schema = schema_parser.parse_schema()
            .map_err(|e| ValidationError::XsdValidation(format!("Schema compile error: {e}")))?;

        // Parse the XML document
        let parser = Parser::default();
        let doc = parser.parse_string(std::str::from_utf8(xml)
            .map_err(|e| ValidationError::XsdValidation(format!("Invalid UTF-8: {e}")))?)
            .map_err(|e| ValidationError::XsdValidation(format!("XML parse error: {e}")))?;

        // Validate document against schema
        let mut validator = SchemaValidationContext::from_schema(&schema)
            .map_err(|e| ValidationError::XsdValidation(format!("Validator init error: {e}")))?;

        validator.validate_document(&doc)
            .map_err(|errors| {
                let messages: Vec<String> = errors.iter()
                    .map(|e| e.message.clone().unwrap_or_default())
                    .collect();
                ValidationError::XsdValidation(messages.join("; "))
            })
    }
}

#[cfg(feature = "xsd-validation")]
pub use xsd::validate_xsd;
```

### Cargo.toml Changes

```toml
[features]
default = []
xsd-validation = ["dep:libxml"]

[dependencies]
# ... existing deps ...
libxml = { version = "0.3.8", optional = true }
```

## Fallback Strategy

When XSD validation is unavailable (feature disabled or no schema provided):

1. **Parse-only mode**: Use quick-xml for XML deserialization
2. **Serde validation**: Catch type mismatches during deserialization
3. **Runtime checks**: Allow types to implement additional validation via custom methods

```rust
pub trait XmlSchema: serde::de::DeserializeOwned + Send + Sync {
    /// Returns the XSD schema for validation, if available.
    fn xsd_schema() -> Option<std::borrow::Cow<'static, str>> {
        None
    }

    /// Additional runtime validation after deserialization.
    /// Called regardless of XSD validation status.
    fn validate(&self) -> Result<(), ValidationError> {
        Ok(())
    }
}
```

## Build Requirements (with xsd-validation feature)

### Linux
```bash
apt install libxml2-dev libclang-dev pkg-config
```

### macOS
```bash
brew install libxml2
# May need: export PKG_CONFIG_PATH="/opt/homebrew/opt/libxml2/lib/pkgconfig"
```

### Windows
Requires Visual Studio Build Tools and vcpkg with libxml2 package.

## Future Considerations

1. **Pure Rust XSD validator**: Monitor the `xmlschema` crate for maturity improvements
2. **xsd-parser validation**: Watch for the planned "Schema-Based Validation" feature
3. **WebAssembly**: The libxml dependency prevents WASM compilation; consider alternatives if needed

## Decision

**Accept the tiered approach**:
- Default: Parse-only mode with quick-xml
- Optional: `xsd-validation` feature using libxml

This balances practicality (most users don't need XSD validation) with capability (power users can enable it when required).

## Sources

- [xmlschema crate](https://crates.io/crates/xmlschema)
- [xmlschema GitHub](https://github.com/sebastienrousseau/xmlschema)
- [xsd-parser docs](https://docs.rs/xsd-parser)
- [xsd-parser GitHub](https://github.com/Bergmann89/xsd-parser)
- [libxml docs](https://docs.rs/libxml)
- [rust-libxml GitHub](https://github.com/KWARC/rust-libxml)
- [raxb GitHub](https://github.com/hd-gmbh-dev/raxb)
- [raxb-validate docs](https://docs.rs/raxb-validate)
- [Rust forum: XSD validation discussion](https://users.rust-lang.org/t/xml-xsd-validation-and-xsl-transform/30393)
