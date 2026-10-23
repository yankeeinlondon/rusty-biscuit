//! The `schematic_runtime::EndpointSpec` implementation of a request type.

use proc_macro2::TokenStream;
use quote::{format_ident, quote};
use schematic_define::{Endpoint, RestApi, RestMethod};

use crate::codegen::responses::{failure_code, success_code};

/// Generates the `EndpointSpec` impl that ties `struct_name` to its API,
/// its success type and codec, its failure type and error table, and its
/// endpoint id, with `into_parts_body` as the body of `into_parts`, preceded
/// by the success enum, stream end markers, and failure enum those types
/// name (see [`crate::codegen::responses`]).
///
/// An endpoint without declared errors keeps `Failure = NoDeclaredError`
/// and no `ERRORS`, so every non-2xx reply is reported as undeclared. The
/// `ErrorBody` impl of a registered JSON failure type is emitted once per
/// API by module assembly, not here.
pub(super) fn generate_endpoint_spec_impl(
    api: &RestApi,
    struct_name: &proc_macro2::Ident,
    api_marker: &str,
    endpoint: &Endpoint,
    into_parts_body: TokenStream,
) -> TokenStream {
    let success = success_code(api, &endpoint.id, endpoint.success_shapes());
    let failure = failure_code(api, &endpoint.id, &api.effective_errors(endpoint), api_marker);
    let api_marker = format_ident!("{}", api_marker);
    let endpoint_id = &endpoint.id;
    let method = method_tokens(&endpoint.method);
    let (success_items, success_type, decoder) = (&success.items, &success.success, &success.decoder);
    let (failure_items, failure_type, errors) = (&failure.items, &failure.failure, &failure.errors);

    quote! {
        #success_items

        #failure_items

        impl schematic_runtime::EndpointSpec for #struct_name {
            type Api = #api_marker;
            type Success = #success_type;
            type Failure = #failure_type;
            type Decoder = #decoder;

            const ENDPOINT_ID: &'static str = #endpoint_id;
            const METHOD: schematic_runtime::Method = #method;

            #errors

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
