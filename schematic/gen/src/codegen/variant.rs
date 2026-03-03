//! Variant API types for response hooks and customization.
//!
//! This module generates types used by the `variant()` builder pattern,
//! including `ResponseContext` for hook callbacks and the type-erased
//! response mutation infrastructure.

use proc_macro2::TokenStream;
use quote::quote;

/// Generates the ResponseContext struct for hook callbacks.
///
/// ResponseContext carries metadata about the HTTP response to hook functions,
/// allowing them to make decisions based on endpoint, status, and headers.
///
/// ## Fields
///
/// - `endpoint_id`: Static identifier for the endpoint (e.g., "ListModels")
/// - `method`: HTTP method used (e.g., "GET", "POST")
/// - `path`: URL path with substituted parameters
/// - `url`: Full request URL
/// - `status`: HTTP status code (200, 404, etc.)
/// - `headers`: Response headers as (name, value) pairs
///
/// ## Why Vec<(String, String)> for headers?
///
/// We use `Vec<(String, String)>` instead of `reqwest::header::HeaderMap` to:
/// - Avoid coupling hook signatures to reqwest internals
/// - Allow hooks to be tested without HTTP dependencies
/// - Enable serialization/debugging of response context
pub fn generate_response_context() -> TokenStream {
    quote! {
        /// Context passed to response hooks containing request/response metadata.
        ///
        /// This struct provides hooks with information about the HTTP exchange,
        /// enabling context-aware response processing.
        ///
        /// ## Examples
        ///
        /// ```text
        /// let variant = api.variant()
        ///     .mutate_response::<ListModelsRequest>(|ctx, response| {
        ///         if ctx.status == 200 {
        ///             // Process successful response
        ///         }
        ///         Ok(())
        ///     })
        ///     .build();
        /// ```
        #[derive(Debug, Clone)]
        pub struct ResponseContext {
            /// The endpoint identifier (e.g., "ListModels", "CreateCompletion").
            pub endpoint_id: &'static str,

            /// The HTTP method used (e.g., "GET", "POST").
            pub method: &'static str,

            /// The URL path with parameters substituted.
            pub path: String,

            /// The full request URL.
            pub url: String,

            /// The HTTP status code from the response.
            pub status: u16,

            /// Response headers as (name, value) pairs.
            ///
            /// Using Vec instead of HeaderMap for decoupling from reqwest.
            pub headers: Vec<(String, String)>,
        }

        impl ResponseContext {
            /// Creates a new ResponseContext from response details.
            ///
            /// This is typically called internally by generated client code
            /// after receiving an HTTP response.
            #[must_use]
            pub fn new(
                endpoint_id: &'static str,
                method: &'static str,
                path: String,
                url: String,
                status: u16,
                headers: Vec<(String, String)>,
            ) -> Self {
                Self {
                    endpoint_id,
                    method,
                    path,
                    url,
                    status,
                    headers,
                }
            }

            /// Gets a header value by name (case-insensitive).
            ///
            /// Returns the first matching header value, or None if not found.
            #[must_use]
            pub fn header(&self, name: &str) -> Option<&str> {
                self.headers
                    .iter()
                    .find(|(n, _)| n.eq_ignore_ascii_case(name))
                    .map(|(_, v)| v.as_str())
            }
        }
    }
}

/// Generates the EndpointSpec trait for type-safe hook registration.
///
/// Each generated request struct implements this trait, providing
/// compile-time association between request types and their response types.
pub fn generate_endpoint_spec_trait() -> TokenStream {
    quote! {
        /// Associates a request type with its response type and endpoint identifier.
        ///
        /// This trait is implemented by generated request structs to enable
        /// type-safe response hook registration via the variant builder.
        ///
        /// ## Examples
        ///
        /// ```text
        /// // Generated implementation for ListModelsRequest
        /// impl EndpointSpec for ListModelsRequest {
        ///     type Response = ListModelsResponse;
        ///     const ENDPOINT_ID: &'static str = "ListModels";
        /// }
        ///
        /// // Allows type-safe hook registration
        /// variant.mutate_response::<ListModelsRequest>(|ctx, response| {
        ///     // `response` is &mut ListModelsResponse
        ///     Ok(())
        /// })
        /// ```
        pub trait EndpointSpec {
            /// The response type for this endpoint.
            type Response;

            /// Static identifier for this endpoint.
            const ENDPOINT_ID: &'static str;
        }
    }
}

/// Generates the AnyResponseMutator trait and TypedMutator for type erasure.
///
/// The response hook system needs to store hooks for different response types
/// in a single HashMap. This requires type erasure at storage time, with
/// type recovery at invocation time.
///
/// ## Type Safety
///
/// - Registration is type-checked at compile time via `EndpointSpec`
/// - Storage uses type-erased `Arc<dyn AnyResponseMutator>`
/// - Invocation downcasts to the correct type (guaranteed to succeed)
pub fn generate_response_mutator_types() -> TokenStream {
    quote! {
        /// Type-erased trait for response mutation hooks.
        ///
        /// This trait allows storing hooks for different response types in
        /// a single collection. The actual type is preserved through the
        /// `TypedMutator` wrapper and recovered via `Any` downcast.
        ///
        /// ## Safety
        ///
        /// The downcast in `mutate` is guaranteed to succeed because:
        /// 1. Registration via `mutate_response::<R>()` ensures type consistency
        /// 2. Lookup is keyed by endpoint_id which maps 1:1 to response type
        /// 3. Type mismatch would require incorrect ENDPOINT_ID (impossible with codegen)
        pub trait AnyResponseMutator: Send + Sync {
            /// Applies the mutation to the response object.
            ///
            /// The `response` parameter is guaranteed to be the correct type
            /// for this mutator. Returns error if mutation fails.
            fn mutate(
                &self,
                ctx: &ResponseContext,
                response: &mut dyn std::any::Any,
            ) -> Result<(), SchematicError>;
        }

        /// Concrete wrapper for type-erased response mutators.
        ///
        /// This struct wraps a typed closure and implements `AnyResponseMutator`
        /// by downcasting the `dyn Any` to the expected type.
        pub struct TypedMutator<T, F>
        where
            T: Send + Sync + 'static,
            F: Fn(&ResponseContext, &mut T) -> Result<(), SchematicError> + Send + Sync + 'static,
        {
            hook: F,
            _marker: std::marker::PhantomData<T>,
        }

        impl<T, F> TypedMutator<T, F>
        where
            T: Send + Sync + 'static,
            F: Fn(&ResponseContext, &mut T) -> Result<(), SchematicError> + Send + Sync + 'static,
        {
            /// Creates a new TypedMutator wrapping the given hook.
            #[must_use]
            pub fn new(hook: F) -> Self {
                Self {
                    hook,
                    _marker: std::marker::PhantomData,
                }
            }
        }

        impl<T, F> AnyResponseMutator for TypedMutator<T, F>
        where
            T: Send + Sync + 'static,
            F: Fn(&ResponseContext, &mut T) -> Result<(), SchematicError> + Send + Sync + 'static,
        {
            fn mutate(
                &self,
                ctx: &ResponseContext,
                response: &mut dyn std::any::Any,
            ) -> Result<(), SchematicError> {
                let typed = response
                    .downcast_mut::<T>()
                    .ok_or_else(|| SchematicError::InternalError(
                        format!(
                            "Type mismatch in response mutator for endpoint '{}'",
                            ctx.endpoint_id
                        )
                    ))?;
                (self.hook)(ctx, typed)
            }
        }
    }
}

/// Generates the pre-response JSON hook type alias.
///
/// Pre-response hooks run after HTTP response but before deserialization,
/// allowing JSON structure transformation to fit existing response types.
pub fn generate_pre_response_hook_type() -> TokenStream {
    quote! {
        /// Hook for transforming JSON before deserialization.
        ///
        /// This hook receives the raw JSON response and can reshape it
        /// to match the expected response type structure.
        ///
        /// ## Examples
        ///
        /// ```text
        /// // Unwrap nested data envelope
        /// let variant = api.variant()
        ///     .pre_response_json(|ctx, json| {
        ///         Ok(json.get("data").cloned().unwrap_or(json))
        ///     })
        ///     .build();
        /// ```
        pub type PreResponseJsonHook = dyn Fn(&ResponseContext, serde_json::Value)
            -> Result<serde_json::Value, SchematicError>
            + Send
            + Sync;
    }
}

/// Generates the VariantHooks struct for storing configured hooks.
///
/// This struct is stored in variant API instances and contains
/// all registered hooks for response processing.
pub fn generate_variant_hooks() -> TokenStream {
    quote! {
        /// Container for all variant-specific hooks and configuration.
        ///
        /// This struct is stored in API client instances created via
        /// the `variant()` builder and holds response hooks.
        #[derive(Default)]
        pub struct VariantHooks {
            /// Optional hook for JSON transformation before deserialization.
            pub pre_response_json: Option<std::sync::Arc<PreResponseJsonHook>>,

            /// Per-endpoint response mutators, keyed by endpoint_id.
            pub response_mutators: std::collections::HashMap<
                &'static str,
                std::sync::Arc<dyn AnyResponseMutator>,
            >,
        }

        impl std::fmt::Debug for VariantHooks {
            fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                f.debug_struct("VariantHooks")
                    .field("pre_response_json", &self.pre_response_json.is_some())
                    .field("response_mutators_count", &self.response_mutators.len())
                    .finish()
            }
        }

        impl Clone for VariantHooks {
            fn clone(&self) -> Self {
                Self {
                    pre_response_json: self.pre_response_json.clone(),
                    response_mutators: self.response_mutators.clone(),
                }
            }
        }
    }
}

/// Generates the InternalError variant addition to SchematicError.
///
/// This is needed for type mismatch errors in the response mutator system.
pub fn generate_internal_error_variant() -> TokenStream {
    quote! {
        /// Internal error in schematic runtime.
        ///
        /// This indicates a bug in the generated code or type system violation.
        /// Should never occur in normal operation.
        #[error("Internal error: {0}")]
        InternalError(String),
    }
}

/// Assembles all variant-related types into a single TokenStream.
///
/// This combines all variant types for inclusion in shared.rs.
pub fn generate_variant_types() -> TokenStream {
    let response_context = generate_response_context();
    let endpoint_spec = generate_endpoint_spec_trait();
    let mutator_types = generate_response_mutator_types();
    let pre_response_hook = generate_pre_response_hook_type();
    let variant_hooks = generate_variant_hooks();

    quote! {
        // ============================================================
        // Variant API Types
        // ============================================================

        #response_context

        #endpoint_spec

        #pre_response_hook

        #mutator_types

        #variant_hooks
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::codegen::request_structs::{format_generated_code, validate_generated_code};

    #[test]
    fn generate_response_context_produces_valid_syntax() {
        let tokens = generate_response_context();
        assert!(
            validate_generated_code(&tokens).is_ok(),
            "Generated ResponseContext should be syntactically valid"
        );
    }

    #[test]
    fn generate_response_context_has_required_fields() {
        let tokens = generate_response_context();
        let code = format_generated_code(&tokens).expect("Failed to format code");

        assert!(
            code.contains("endpoint_id: &'static str"),
            "Missing endpoint_id field"
        );
        assert!(
            code.contains("method: &'static str"),
            "Missing method field"
        );
        assert!(code.contains("path: String"), "Missing path field");
        assert!(code.contains("url: String"), "Missing url field");
        assert!(code.contains("status: u16"), "Missing status field");
        assert!(
            code.contains("headers: Vec<(String, String)>"),
            "Missing headers field"
        );
    }

    #[test]
    fn generate_endpoint_spec_produces_valid_syntax() {
        let tokens = generate_endpoint_spec_trait();
        assert!(
            validate_generated_code(&tokens).is_ok(),
            "Generated EndpointSpec should be syntactically valid"
        );
    }

    #[test]
    fn generate_endpoint_spec_has_associated_type_and_const() {
        let tokens = generate_endpoint_spec_trait();
        let code = format_generated_code(&tokens).expect("Failed to format code");

        assert!(
            code.contains("type Response"),
            "Missing Response associated type"
        );
        assert!(
            code.contains("const ENDPOINT_ID: &'static str"),
            "Missing ENDPOINT_ID const"
        );
    }

    #[test]
    fn generate_response_mutator_types_produces_valid_syntax() {
        let tokens = generate_response_mutator_types();
        assert!(
            validate_generated_code(&tokens).is_ok(),
            "Generated mutator types should be syntactically valid"
        );
    }

    #[test]
    fn generate_response_mutator_types_has_trait_and_struct() {
        let tokens = generate_response_mutator_types();
        let code = format_generated_code(&tokens).expect("Failed to format code");

        assert!(
            code.contains("pub trait AnyResponseMutator"),
            "Missing AnyResponseMutator trait"
        );
        assert!(
            code.contains("pub struct TypedMutator"),
            "Missing TypedMutator struct"
        );
    }

    #[test]
    fn generate_variant_hooks_produces_valid_syntax() {
        let tokens = generate_variant_hooks();
        assert!(
            validate_generated_code(&tokens).is_ok(),
            "Generated VariantHooks should be syntactically valid"
        );
    }

    #[test]
    fn generate_variant_hooks_has_required_fields() {
        let tokens = generate_variant_hooks();
        let code = format_generated_code(&tokens).expect("Failed to format code");

        assert!(
            code.contains("pre_response_json:"),
            "Missing pre_response_json field"
        );
        assert!(
            code.contains("response_mutators:"),
            "Missing response_mutators field"
        );
    }

    #[test]
    fn generate_variant_types_combines_all_types() {
        let tokens = generate_variant_types();
        let code = format_generated_code(&tokens).expect("Failed to format code");

        assert!(code.contains("ResponseContext"), "Missing ResponseContext");
        assert!(code.contains("EndpointSpec"), "Missing EndpointSpec");
        assert!(
            code.contains("AnyResponseMutator"),
            "Missing AnyResponseMutator"
        );
        assert!(code.contains("TypedMutator"), "Missing TypedMutator");
        assert!(code.contains("VariantHooks"), "Missing VariantHooks");
    }
}
