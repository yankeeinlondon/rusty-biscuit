//! The `schematic_runtime::EndpointSpec` implementation of a request type.

use proc_macro2::TokenStream;
use quote::{format_ident, quote};
use schematic_define::{ApiResponse, Endpoint, RestMethod};

use crate::codegen::request_structs::shared::type_name_to_tokens;

/// Generates the `EndpointSpec` impl that ties `struct_name` to its API,
/// its success type and codec, and its endpoint id, with `into_parts_body`
/// as the body of `into_parts`.
///
/// No definition declares an error body, so `Failure` is always
/// `NoDeclaredError`: every non-2xx reply is reported as undeclared.
pub(super) fn generate_endpoint_spec_impl(
    struct_name: &proc_macro2::Ident,
    api_marker: &str,
    endpoint: &Endpoint,
    into_parts_body: TokenStream,
) -> TokenStream {
    let api_marker = format_ident!("{}", api_marker);
    let endpoint_id = &endpoint.id;
    let method = method_tokens(&endpoint.method);
    let (success, decoder) = match &endpoint.response {
        ApiResponse::Json(schema) => {
            let ty = type_name_to_tokens(&schema.type_name);
            (ty.clone(), quote! { schematic_runtime::Json<#ty> })
        }
        ApiResponse::Text => (quote! { String }, quote! { schematic_runtime::Text }),
        ApiResponse::Binary => (quote! { bytes::Bytes }, quote! { schematic_runtime::Binary }),
        // `Empty`, and any response kind added later, drains the body.
        _ => (quote! { () }, quote! { schematic_runtime::Empty }),
    };

    quote! {
        impl schematic_runtime::EndpointSpec for #struct_name {
            type Api = #api_marker;
            type Success = #success;
            type Failure = schematic_runtime::NoDeclaredError;
            type Decoder = #decoder;

            const ENDPOINT_ID: &'static str = #endpoint_id;
            const METHOD: schematic_runtime::Method = #method;

            fn into_parts(self) -> Result<schematic_runtime::RequestParts, schematic_runtime::InvalidArgument> {
                #into_parts_body
            }
        }
    }
}

pub(crate) fn method_tokens(method: &RestMethod) -> TokenStream {
    match method {
        RestMethod::Get => quote! { schematic_runtime::Method::GET },
        RestMethod::Post => quote! { schematic_runtime::Method::POST },
        RestMethod::Put => quote! { schematic_runtime::Method::PUT },
        RestMethod::Patch => quote! { schematic_runtime::Method::PATCH },
        RestMethod::Delete => quote! { schematic_runtime::Method::DELETE },
        RestMethod::Head => quote! { schematic_runtime::Method::HEAD },
        RestMethod::Options => quote! { schematic_runtime::Method::OPTIONS },
        // `RestMethod` is `#[non_exhaustive]`; a method added later fails the
        // generated crate's build here rather than being sent as another.
        other => {
            let message = format!("schematic-gen cannot emit the HTTP method {other}");
            quote! { compile_error!(#message) }
        }
    }
}
