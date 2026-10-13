//! Endpoint methods and their call types.
//!
//! Every REST endpoint gets an inherent method on its API struct, which
//! takes the endpoint's required values in `{Endpoint}Request::new` order and
//! returns a generated call type. The call type wraps the runtime's
//! `schematic_runtime::Call`, adds one setter per optional value, and
//! forwards the shared per-call methods, so nothing needs a trait import:
//!
//! ```text
//! /// A call to the `ListWidgets` endpoint, returned by [`Widgets::list_widgets`].
//! #[derive(Clone, Debug)]
//! #[must_use = "a call sends nothing until it is awaited"]
//! pub struct ListWidgetsCall<'a>(schematic_runtime::Call<'a, WidgetsApi, ListWidgetsRequest>);
//!
//! impl<'a> ListWidgetsCall<'a> {
//!     pub fn limit(self, value: i64) -> Self { ... }   // one per optional value
//!     pub fn anonymous(self) -> Self { ... }           // the shared per-call methods
//!     pub fn full(self) -> schematic_runtime::FullCall<'a, WidgetsApi, ListWidgetsRequest> { ... }
//!     ...
//! }
//!
//! impl<'a> core::future::IntoFuture for ListWidgetsCall<'a> { ... }
//!
//! impl Widgets {
//!     pub fn list_widgets(&self) -> ListWidgetsCall<'_> { ... }
//! }
//! ```
//!
//! Names come from [`names::resolve_names`]; the method docs from [`docs`].

mod docs;
pub mod names;
#[cfg(test)]
mod tests;

use proc_macro2::TokenStream;
use quote::{format_ident, quote};
use schematic_define::{Endpoint, RestApi};

use crate::codegen::api_struct::api_tag_name;
use crate::codegen::request_structs::request_body_type_name;
use crate::codegen::request_structs::shared::{
    param_field_name, query_param_type_to_rust_type, type_name_to_tokens,
};
use crate::parser::extract_path_params;

use docs::RequiredArg;
pub use names::{
    ApiMethodNames, EndpointNames, NameConflict, ParamLocation, Rename, RenameReason, RenameTarget,
    SetterName, call_type_name, endpoint_method_renames, resolve_names,
};

/// Generates every call type of `api` and one `impl` block that adds its
/// endpoint methods to the API struct.
///
/// Emitted beside the request types, so the call types land in the same
/// module (`requests.rs` for a split module, a child of the API struct's
/// module, which can still reach the struct's private client).
pub fn generate_endpoint_methods(api: &RestApi) -> TokenStream {
    let names = resolve_names(api);
    let client = format_ident!("{}", api.name);
    let tag = format_ident!("{}", api_tag_name(api));

    let mut call_types = TokenStream::new();
    let mut methods = TokenStream::new();
    for (endpoint, names) in api.endpoints.iter().zip(&names.endpoints) {
        let args = required_args(endpoint);
        call_types.extend(call_type(&api.name, &tag, names));
        methods.extend(endpoint_method(endpoint, names, &args));
    }

    quote! {
        #call_types

        impl #client {
            #methods
        }
    }
}

/// How a caller calls `endpoint` on a client bound to `client`, for docs:
/// `client.get_item("item_id")`, with each path parameter's name as its
/// value and `body` for a body.
pub(crate) fn example_invocation(api: &RestApi, endpoint: &Endpoint, client: &str) -> String {
    let names = resolve_names(api);
    let (_, names) = api
        .endpoints
        .iter()
        .zip(&names.endpoints)
        .find(|(candidate, _)| candidate.id == endpoint.id)
        .expect("the endpoint belongs to the API");
    let mut args: Vec<String> = extract_path_params(&endpoint.path)
        .into_iter()
        .map(|param| format!("\"{}\"", param_field_name(param)))
        .collect();
    if request_body_type_name(endpoint).is_some() {
        args.push("body".to_string());
    }
    format!("{client}.{}({})", names.method, args.join(", "))
}

/// The type a setter takes: the type of the request's `with_*` method.
pub(crate) fn setter_type(setter: &SetterName) -> TokenStream {
    query_param_type_to_rust_type(&setter.param.param_type)
}

/// The required arguments of `endpoint`, in `{Endpoint}Request::new` order:
/// each path parameter, then the body.
fn required_args(endpoint: &Endpoint) -> Vec<RequiredArg> {
    let mut args: Vec<RequiredArg> = extract_path_params(&endpoint.path)
        .into_iter()
        .map(|param| RequiredArg {
            name: param_field_name(param),
            ty: quote! { impl Into<String> },
            role: format!("the `{{{param}}}` path parameter"),
        })
        .collect();
    if let Some(body) = request_body_type_name(endpoint) {
        args.push(RequiredArg {
            name: "body".to_string(),
            ty: type_name_to_tokens(&body),
            role: "the request body".to_string(),
        });
    }
    args
}

fn call_type(api_name: &str, tag: &proc_macro2::Ident, names: &EndpointNames) -> TokenStream {
    let call = format_ident!("{}", names.call_type);
    let request = format_ident!("{}", names.request_type);
    let doc = docs::call_type_doc(api_name, names);
    let runtime_call = quote! { schematic_runtime::Call<'a, #tag, #request> };

    let setters = names.setters.iter().map(|setter| {
        let name = format_ident!("{}", setter.setter);
        let with = format_ident!("with_{}", setter.field);
        let ty = setter_type(setter);
        let doc = docs::setter_doc(setter);
        quote! {
            #(#[doc = #doc])*
            pub fn #name(self, value: #ty) -> Self {
                Self(self.0.map_request(|request| request.#with(value)))
            }
        }
    });

    quote! {
        #(#[doc = #doc])*
        #[derive(Clone, Debug)]
        #[must_use = "a call sends nothing until it is awaited"]
        pub struct #call<'a>(#runtime_call);

        impl<'a> #call<'a> {
            #(#setters)*

            /// See [`Call::anonymous`](schematic_runtime::Call::anonymous).
            pub fn anonymous(self) -> Self {
                Self(self.0.anonymous())
            }

            /// See [`Call::credential`](schematic_runtime::Call::credential).
            pub fn credential(self, credential: schematic_runtime::Credential) -> Self {
                Self(self.0.credential(credential))
            }

            /// See [`Call::deadline`](schematic_runtime::Call::deadline).
            pub fn deadline(self, deadline: &schematic_runtime::Deadline) -> Self {
                Self(self.0.deadline(deadline))
            }

            /// See [`Call::full`](schematic_runtime::Call::full). Chain it
            /// last: this call's setters are not available on the result.
            pub fn full(self) -> schematic_runtime::FullCall<'a, #tag, #request> {
                self.0.full()
            }

            /// See [`Call::build`](schematic_runtime::Call::build).
            ///
            /// ## Errors
            ///
            /// Returns `InvalidArgument` when the request cannot be encoded.
            pub fn build(self) -> Result<Self, schematic_runtime::InvalidArgument> {
                self.0.build().map(Self)
            }
        }

        impl<'a> ::core::future::IntoFuture for #call<'a> {
            type Output = <#runtime_call as ::core::future::IntoFuture>::Output;
            type IntoFuture = schematic_runtime::CallFuture<'a, Self::Output>;

            fn into_future(self) -> Self::IntoFuture {
                ::core::future::IntoFuture::into_future(self.0)
            }
        }
    }
}

fn endpoint_method(endpoint: &Endpoint, names: &EndpointNames, args: &[RequiredArg]) -> TokenStream {
    let method = format_ident!("{}", names.method);
    let call = format_ident!("{}", names.call_type);
    let request = format_ident!("{}", names.request_type);
    let doc = docs::endpoint_method_doc(endpoint, names, args);
    let params = args.iter().map(|arg| {
        let name = format_ident!("{}", arg.name);
        let ty = &arg.ty;
        quote! { #name: #ty }
    });
    let construct = if args.is_empty() {
        quote! { #request::default() }
    } else {
        let names = args.iter().map(|arg| format_ident!("{}", arg.name));
        quote! { #request::new(#(#names),*) }
    };

    quote! {
        #(#[doc = #doc])*
        pub fn #method(&self, #(#params),*) -> #call<'_> {
            #call(self.0.send(#construct))
        }
    }
}
