//! Reference resolver for OpenAPI $ref resolution.
//!
//! This module provides the `RefResolver` struct for resolving `$ref` references
//! in OpenAPI documents to their target schemas, parameters, and responses.

use std::cell::RefCell;
use std::collections::HashSet;

use openapiv3::{OpenAPI, Parameter, ReferenceOr, Response, Schema};

use super::MAX_NESTING_DEPTH;
use super::diagnostics::OpenApiDiagnostic;
use crate::models::TypeRef;

/// Resolver for OpenAPI `$ref` references.
///
/// Provides methods to resolve schema, parameter, and response references
/// to their underlying definitions. Includes cycle detection to prevent
/// infinite loops in circular references.
///
/// ## Examples
///
/// ```text
/// use schematic_define::openapi::import::RefResolver;
///
/// let resolver = RefResolver::new(&doc);
///
/// // Resolve a schema reference
/// let schema = resolver.resolve_schema(&schema_ref)?;
/// ```
#[derive(Debug)]
pub struct RefResolver<'a> {
    /// The OpenAPI document being resolved.
    doc: &'a OpenAPI,
    /// Tracks visited references for cycle detection (per resolution chain).
    visited: RefCell<HashSet<String>>,
    /// Current nesting depth for depth limiting.
    depth: RefCell<usize>,
}

impl<'a> RefResolver<'a> {
    /// Creates a new resolver for the given OpenAPI document.
    ///
    /// ## Examples
    ///
    /// ```text
    /// use schematic_define::openapi::import::RefResolver;
    ///
    /// let resolver = RefResolver::new(&openapi_doc);
    /// ```
    pub fn new(doc: &'a OpenAPI) -> Self {
        Self {
            doc,
            visited: RefCell::new(HashSet::new()),
            depth: RefCell::new(0),
        }
    }

    /// Resolves a schema reference to its underlying schema.
    ///
    /// ## Arguments
    ///
    /// * `ref_or` - A `ReferenceOr<Schema>` that may be a direct schema or a `$ref`
    ///
    /// ## Returns
    ///
    /// Returns `Ok(&Schema)` if resolution succeeds, or an error if:
    /// - The reference cannot be found
    /// - A circular reference is detected
    /// - Nesting depth exceeds the limit
    ///
    /// ## Examples
    ///
    /// ```text
    /// let schema_ref = ReferenceOr::Reference { reference: "#/components/schemas/Pet".to_string() };
    /// let schema = resolver.resolve_schema(&schema_ref)?;
    /// ```
    pub fn resolve_schema(
        &self,
        ref_or: &'a ReferenceOr<Schema>,
    ) -> Result<&'a Schema, ResolveError> {
        match ref_or {
            ReferenceOr::Item(schema) => Ok(schema),
            ReferenceOr::Reference { reference } => self.resolve_schema_ref(reference),
        }
    }

    /// Resolves a boxed schema reference to its underlying schema.
    ///
    /// Object properties and array items are spelled `ReferenceOr<Box<Schema>>`
    /// rather than `ReferenceOr<Schema>`; this is the same resolution with that
    /// shape unwrapped.
    pub fn resolve_boxed_schema(
        &self,
        ref_or: &'a ReferenceOr<Box<Schema>>,
    ) -> Result<&'a Schema, ResolveError> {
        match ref_or {
            ReferenceOr::Item(schema) => Ok(schema),
            ReferenceOr::Reference { reference } => self.resolve_schema_ref(reference),
        }
    }

    /// Resolves a schema reference by its reference string.
    fn resolve_schema_ref(&self, reference: &str) -> Result<&'a Schema, ResolveError> {
        // Check depth limit
        let current_depth = *self.depth.borrow();
        if current_depth >= MAX_NESTING_DEPTH {
            return Err(ResolveError::DepthExceeded {
                reference: reference.to_string(),
                max_depth: MAX_NESTING_DEPTH,
            });
        }

        // Check for cycles
        if self.visited.borrow().contains(reference) {
            return Err(ResolveError::Cycle {
                reference: reference.to_string(),
            });
        }

        // Parse the reference path
        let schema_name = Self::extract_schema_name(reference)?;

        // Look up in components
        let schema = self
            .doc
            .components
            .as_ref()
            .and_then(|c| c.schemas.get(schema_name))
            .ok_or_else(|| ResolveError::NotFound {
                reference: reference.to_string(),
                kind: "schema".to_string(),
            })?;

        // Mark as visited and increase depth
        self.visited.borrow_mut().insert(reference.to_string());
        *self.depth.borrow_mut() += 1;

        // Resolve the schema (it might itself be a reference)
        let result = self.resolve_schema(schema);

        // Clean up
        *self.depth.borrow_mut() -= 1;
        self.visited.borrow_mut().remove(reference);

        result
    }

    /// Resolves a parameter reference to its underlying parameter.
    ///
    /// ## Arguments
    ///
    /// * `ref_or` - A `ReferenceOr<Parameter>` that may be a direct parameter or a `$ref`
    ///
    /// ## Returns
    ///
    /// Returns `Ok(&Parameter)` if resolution succeeds, or an error if the
    /// reference cannot be found.
    pub fn resolve_parameter(
        &self,
        ref_or: &'a ReferenceOr<Parameter>,
    ) -> Result<&'a Parameter, ResolveError> {
        match ref_or {
            ReferenceOr::Item(param) => Ok(param),
            ReferenceOr::Reference { reference } => self.resolve_parameter_ref(reference),
        }
    }

    /// Resolves a parameter reference by its reference string.
    fn resolve_parameter_ref(&self, reference: &str) -> Result<&'a Parameter, ResolveError> {
        let param_name = Self::extract_component_name(reference, "parameters")?;

        self.doc
            .components
            .as_ref()
            .and_then(|c| c.parameters.get(param_name))
            .and_then(|ref_or| match ref_or {
                ReferenceOr::Item(param) => Some(param),
                ReferenceOr::Reference { .. } => None, // Nested refs not supported
            })
            .ok_or_else(|| ResolveError::NotFound {
                reference: reference.to_string(),
                kind: "parameter".to_string(),
            })
    }

    /// Resolves a response reference to its underlying response.
    ///
    /// ## Arguments
    ///
    /// * `ref_or` - A `ReferenceOr<Response>` that may be a direct response or a `$ref`
    ///
    /// ## Returns
    ///
    /// Returns `Ok(&Response)` if resolution succeeds, or an error if the
    /// reference cannot be found.
    pub fn resolve_response(
        &self,
        ref_or: &'a ReferenceOr<Response>,
    ) -> Result<&'a Response, ResolveError> {
        match ref_or {
            ReferenceOr::Item(response) => Ok(response),
            ReferenceOr::Reference { reference } => self.resolve_response_ref(reference),
        }
    }

    /// Resolves a response reference by its reference string.
    fn resolve_response_ref(&self, reference: &str) -> Result<&'a Response, ResolveError> {
        let response_name = Self::extract_component_name(reference, "responses")?;

        self.doc
            .components
            .as_ref()
            .and_then(|c| c.responses.get(response_name))
            .and_then(|ref_or| match ref_or {
                ReferenceOr::Item(response) => Some(response),
                ReferenceOr::Reference { .. } => None,
            })
            .ok_or_else(|| ResolveError::NotFound {
                reference: reference.to_string(),
                kind: "response".to_string(),
            })
    }

    /// Extracts the schema name from a reference like `#/components/schemas/Pet`.
    fn extract_schema_name(reference: &str) -> Result<&str, ResolveError> {
        Self::extract_component_name(reference, "schemas")
    }

    /// Extracts the component name from a reference.
    fn extract_component_name<'b>(
        reference: &'b str,
        component_type: &str,
    ) -> Result<&'b str, ResolveError> {
        let prefix = format!("#/components/{}/", component_type);
        if reference.starts_with(&prefix) {
            Ok(&reference[prefix.len()..])
        } else {
            Err(ResolveError::InvalidFormat {
                reference: reference.to_string(),
                expected: format!("#/components/{}/{{name}}", component_type),
            })
        }
    }

    /// Resolves a schema reference to a TypeRef, handling cycles gracefully.
    ///
    /// Unlike `resolve_schema`, this method returns `TypeRef::Unknown` for
    /// circular references instead of an error, with a diagnostic.
    pub fn resolve_to_type_ref(
        &self,
        ref_or: &ReferenceOr<Schema>,
        diagnostics: &mut Vec<OpenApiDiagnostic>,
    ) -> TypeRef {
        match ref_or {
            ReferenceOr::Item(_) => {
                // Direct schema - will be processed elsewhere
                TypeRef::Unknown
            }
            ReferenceOr::Reference { reference } => {
                // Check for cycle
                if self.visited.borrow().contains(reference) {
                    diagnostics.push(OpenApiDiagnostic::warn(
                        reference.clone(),
                        "Circular reference detected, using Unknown type".to_string(),
                    ));
                    return TypeRef::Unknown;
                }

                // Check depth
                if *self.depth.borrow() >= MAX_NESTING_DEPTH {
                    diagnostics.push(OpenApiDiagnostic::error(
                        reference.clone(),
                        format!("Nesting depth exceeds maximum {}", MAX_NESTING_DEPTH),
                    ));
                    return TypeRef::Unknown;
                }

                // Extract the type name from the reference
                match Self::extract_schema_name(reference) {
                    Ok(name) => TypeRef::Named(name.to_string()),
                    Err(_) => {
                        diagnostics.push(OpenApiDiagnostic::error(
                            reference.clone(),
                            "Invalid schema reference format".to_string(),
                        ));
                        TypeRef::Unknown
                    }
                }
            }
        }
    }

    /// Clears the visited set for a fresh resolution chain.
    pub fn reset(&self) {
        self.visited.borrow_mut().clear();
        *self.depth.borrow_mut() = 0;
    }
}

/// Errors that can occur during reference resolution.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ResolveError {
    /// Reference was not found in the document.
    NotFound {
        /// The reference that was not found.
        reference: String,
        /// The kind of reference (schema, parameter, response).
        kind: String,
    },
    /// Circular reference detected.
    Cycle {
        /// The reference where the cycle was detected.
        reference: String,
    },
    /// Nesting depth exceeded.
    DepthExceeded {
        /// The reference that exceeded depth.
        reference: String,
        /// Maximum allowed depth.
        max_depth: usize,
    },
    /// Invalid reference format.
    InvalidFormat {
        /// The malformed reference.
        reference: String,
        /// Expected format description.
        expected: String,
    },
}

impl std::fmt::Display for ResolveError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ResolveError::NotFound { reference, kind } => {
                write!(f, "{} not found: {}", kind, reference)
            }
            ResolveError::Cycle { reference } => {
                write!(f, "Circular reference detected: {}", reference)
            }
            ResolveError::DepthExceeded {
                reference,
                max_depth,
            } => {
                write!(f, "Nesting depth exceeded {} at: {}", max_depth, reference)
            }
            ResolveError::InvalidFormat {
                reference,
                expected,
            } => {
                write!(
                    f,
                    "Invalid reference format '{}', expected: {}",
                    reference, expected
                )
            }
        }
    }
}

impl std::error::Error for ResolveError {}

#[cfg(test)]
mod tests {
    use super::*;
    use openapiv3::{Components, ObjectType, SchemaData, SchemaKind, Type};

    // ========== Helper Functions ==========

    fn create_empty_doc() -> OpenAPI {
        OpenAPI {
            openapi: "3.0.0".to_string(),
            info: openapiv3::Info {
                title: "Test".to_string(),
                version: "1.0.0".to_string(),
                ..Default::default()
            },
            ..Default::default()
        }
    }

    fn create_doc_with_schema(name: &str, schema: Schema) -> OpenAPI {
        let mut doc = create_empty_doc();
        let mut components = Components::default();
        components
            .schemas
            .insert(name.to_string(), ReferenceOr::Item(schema));
        doc.components = Some(components);
        doc
    }

    fn create_simple_schema() -> Schema {
        Schema {
            schema_data: SchemaData::default(),
            schema_kind: SchemaKind::Type(Type::Object(ObjectType::default())),
        }
    }

    // ========== RefResolver Tests ==========

    #[test]
    fn new_creates_resolver() {
        let doc = create_empty_doc();
        let resolver = RefResolver::new(&doc);

        // Should be able to use the resolver
        assert!(resolver.visited.borrow().is_empty());
    }

    #[test]
    fn resolve_schema_item_returns_schema() {
        let doc = create_empty_doc();
        let resolver = RefResolver::new(&doc);

        let schema = create_simple_schema();
        let ref_or = ReferenceOr::Item(schema.clone());

        let result = resolver.resolve_schema(&ref_or).unwrap();
        assert_eq!(result.schema_kind, schema.schema_kind);
    }

    #[test]
    fn resolve_schema_ref_finds_schema() {
        let doc = create_doc_with_schema("Pet", create_simple_schema());
        let resolver = RefResolver::new(&doc);

        let ref_or = ReferenceOr::Reference {
            reference: "#/components/schemas/Pet".to_string(),
        };

        let result = resolver.resolve_schema(&ref_or);
        assert!(result.is_ok());
    }

    #[test]
    fn resolve_schema_ref_not_found() {
        let doc = create_empty_doc();
        let resolver = RefResolver::new(&doc);

        let ref_or = ReferenceOr::Reference {
            reference: "#/components/schemas/Missing".to_string(),
        };

        let result = resolver.resolve_schema(&ref_or);
        assert!(matches!(result, Err(ResolveError::NotFound { .. })));
    }

    #[test]
    fn resolve_schema_invalid_format() {
        let doc = create_empty_doc();
        let resolver = RefResolver::new(&doc);

        let ref_or = ReferenceOr::Reference {
            reference: "invalid/reference".to_string(),
        };

        let result = resolver.resolve_schema(&ref_or);
        assert!(matches!(result, Err(ResolveError::InvalidFormat { .. })));
    }

    #[test]
    fn resolve_detects_cycle() {
        // Create a schema that references itself
        let mut doc = create_empty_doc();
        let mut components = Components::default();

        // SelfRef -> $ref: #/components/schemas/SelfRef
        components.schemas.insert(
            "SelfRef".to_string(),
            ReferenceOr::Reference {
                reference: "#/components/schemas/SelfRef".to_string(),
            },
        );
        doc.components = Some(components);

        let resolver = RefResolver::new(&doc);

        let ref_or = ReferenceOr::Reference {
            reference: "#/components/schemas/SelfRef".to_string(),
        };

        let result = resolver.resolve_schema(&ref_or);
        assert!(matches!(result, Err(ResolveError::Cycle { .. })));
    }

    #[test]
    fn resolve_detects_indirect_cycle() {
        // A -> B -> A
        let mut doc = create_empty_doc();
        let mut components = Components::default();

        components.schemas.insert(
            "A".to_string(),
            ReferenceOr::Reference {
                reference: "#/components/schemas/B".to_string(),
            },
        );
        components.schemas.insert(
            "B".to_string(),
            ReferenceOr::Reference {
                reference: "#/components/schemas/A".to_string(),
            },
        );
        doc.components = Some(components);

        let resolver = RefResolver::new(&doc);

        let ref_or = ReferenceOr::Reference {
            reference: "#/components/schemas/A".to_string(),
        };

        let result = resolver.resolve_schema(&ref_or);
        assert!(matches!(result, Err(ResolveError::Cycle { .. })));
    }

    #[test]
    fn reset_clears_visited() {
        let doc = create_doc_with_schema("Pet", create_simple_schema());
        let resolver = RefResolver::new(&doc);

        // Simulate having visited a ref
        resolver.visited.borrow_mut().insert("#/test".to_string());

        resolver.reset();

        assert!(resolver.visited.borrow().is_empty());
    }

    #[test]
    fn resolve_to_type_ref_returns_named_for_valid_ref() {
        let doc = create_doc_with_schema("Pet", create_simple_schema());
        let resolver = RefResolver::new(&doc);

        let ref_or = ReferenceOr::Reference {
            reference: "#/components/schemas/Pet".to_string(),
        };

        let mut diags = Vec::new();
        let result = resolver.resolve_to_type_ref(&ref_or, &mut diags);

        assert!(matches!(result, TypeRef::Named(ref name) if name == "Pet"));
        assert!(diags.is_empty());
    }

    #[test]
    fn resolve_to_type_ref_returns_unknown_for_cycle() {
        let mut doc = create_empty_doc();
        let mut components = Components::default();
        components.schemas.insert(
            "SelfRef".to_string(),
            ReferenceOr::Reference {
                reference: "#/components/schemas/SelfRef".to_string(),
            },
        );
        doc.components = Some(components);

        let resolver = RefResolver::new(&doc);

        // Pre-mark as visited to simulate cycle
        resolver
            .visited
            .borrow_mut()
            .insert("#/components/schemas/SelfRef".to_string());

        let ref_or = ReferenceOr::Reference {
            reference: "#/components/schemas/SelfRef".to_string(),
        };

        let mut diags = Vec::new();
        let result = resolver.resolve_to_type_ref(&ref_or, &mut diags);

        assert!(matches!(result, TypeRef::Unknown));
        assert_eq!(diags.len(), 1);
        assert!(diags[0].is_warn());
    }

    #[test]
    fn resolve_to_type_ref_returns_unknown_for_invalid_format() {
        let doc = create_empty_doc();
        let resolver = RefResolver::new(&doc);

        let ref_or = ReferenceOr::Reference {
            reference: "invalid".to_string(),
        };

        let mut diags = Vec::new();
        let result = resolver.resolve_to_type_ref(&ref_or, &mut diags);

        assert!(matches!(result, TypeRef::Unknown));
        assert_eq!(diags.len(), 1);
        assert!(diags[0].is_error());
    }

    // ========== ResolveError Tests ==========

    #[test]
    fn resolve_error_display_not_found() {
        let err = ResolveError::NotFound {
            reference: "#/components/schemas/Missing".to_string(),
            kind: "schema".to_string(),
        };
        let display = err.to_string();
        assert!(display.contains("not found"));
        assert!(display.contains("Missing"));
    }

    #[test]
    fn resolve_error_display_cycle() {
        let err = ResolveError::Cycle {
            reference: "#/components/schemas/A".to_string(),
        };
        let display = err.to_string();
        assert!(display.contains("Circular"));
    }

    #[test]
    fn resolve_error_display_depth() {
        let err = ResolveError::DepthExceeded {
            reference: "#/test".to_string(),
            max_depth: 50,
        };
        let display = err.to_string();
        assert!(display.contains("50"));
    }

    #[test]
    fn resolve_error_display_format() {
        let err = ResolveError::InvalidFormat {
            reference: "bad".to_string(),
            expected: "#/components/schemas/{name}".to_string(),
        };
        let display = err.to_string();
        assert!(display.contains("Invalid"));
    }

    #[test]
    fn resolve_error_is_error_trait() {
        let err = ResolveError::NotFound {
            reference: "test".to_string(),
            kind: "schema".to_string(),
        };
        // Verify it implements std::error::Error
        let _: &dyn std::error::Error = &err;
    }
}
