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
//! The call type of a paginated endpoint also forwards to its
//! `schematic_runtime::pager::Pager`: `pager()`, one method per pager control
//! the strategy authors (`page`, `offset`, `after`, `before`, `page_size`,
//! `keep_alive`), and, for a strategy without a session, `pages()`,
//! `fetch_page()`, `collect_all()`, and `partitioned()` when it authors
//! partitions. A session strategy forwards `with_session()` and
//! `close_grace()` instead.
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
use crate::codegen::pagination::{KnobForward, PagerForwards};
use crate::codegen::request_structs::path_params::typed_path_fields;
use crate::codegen::request_structs::{query_example_arg, request_body_type_name};
use crate::codegen::request_structs::shared::{
    extract_query_params, param_field_name, type_name_to_tokens,
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
        let args = required_args(api, endpoint, &names.request_type);
        call_types.extend(call_type(&api.name, &tag, names, endpoint.deprecated));
        methods.extend(endpoint_method(api, endpoint, names, &args));
    }

    quote! {
        #call_types

        impl #client {
            #methods
        }
    }
}

/// How a caller calls `endpoint` on a client bound to `client`, for docs:
/// `client.get_item("item_id")`, with each untyped path parameter's name as
/// its value, a plausible value of each typed path and required query
/// parameter, and `body` for a body.
pub(crate) fn example_invocation(api: &RestApi, endpoint: &Endpoint, client: &str) -> String {
    let names = resolve_names(api);
    let (_, names) = api
        .endpoints
        .iter()
        .zip(&names.endpoints)
        .find(|(candidate, _)| candidate.id == endpoint.id)
        .expect("the endpoint belongs to the API");
    let mut args: Vec<String> = match typed_path_fields(endpoint, &names.request_type) {
        Some(fields) => fields.iter().map(|field| field.example_arg()).collect(),
        None => extract_path_params(&endpoint.path)
            .into_iter()
            .map(|param| format!("\"{}\"", param_field_name(param)))
            .collect(),
    };
    // Adopted pagination parameters are never arguments.
    let endpoint = crate::codegen::pagination::pager_view(api, endpoint);
    let endpoint = endpoint.as_ref();
    args.extend(
        extract_query_params(&endpoint.params, &names.request_type)
            .iter()
            .filter(|param| param.required)
            .map(|param| query_example_arg(param).0),
    );
    if request_body_type_name(api, endpoint).is_some() {
        args.push("body".to_string());
    }
    format!("{client}.{}({})", names.method, args.join(", "))
}

/// The type a setter takes: the type of the request's `with_*` method.
pub(crate) fn setter_type(setter: &SetterName) -> TokenStream {
    setter.param.rust_type()
}

/// The required arguments of `endpoint`, whose request type is
/// `request_type`, in `{Endpoint}Request::new` order: each path parameter
/// (typed ones in first-occurrence template order), then each required
/// query parameter in declaration order, then the body.
fn required_args(api: &RestApi, endpoint: &Endpoint, request_type: &str) -> Vec<RequiredArg> {
    let mut args: Vec<RequiredArg> = match typed_path_fields(endpoint, request_type) {
        Some(fields) => fields
            .iter()
            .map(|field| RequiredArg {
                name: field.field.clone(),
                ty: field.arg_type(),
                role: format!("the `{{{}}}` path parameter", field.wire),
            })
            .collect(),
        None => extract_path_params(&endpoint.path)
            .into_iter()
            .map(|param| RequiredArg {
                name: param_field_name(param),
                ty: quote! { impl Into<String> },
                role: format!("the `{{{param}}}` path parameter"),
            })
            .collect(),
    };
    args.extend(
        extract_query_params(&endpoint.params, request_type)
            .into_iter()
            .filter(|param| param.required)
            .map(|param| RequiredArg {
                name: param.field_name(),
                ty: param.arg_type(),
                role: format!("the required `{}` query parameter", param.name),
            }),
    );
    if let Some(body) = request_body_type_name(api, endpoint) {
        args.push(RequiredArg {
            name: "body".to_string(),
            ty: type_name_to_tokens(&body),
            role: "the request body".to_string(),
        });
    }
    args
}

fn call_type(
    api_name: &str,
    tag: &proc_macro2::Ident,
    names: &EndpointNames,
    deprecated: bool,
) -> TokenStream {
    let call = format_ident!("{}", names.call_type);
    let request = format_ident!("{}", names.request_type);
    let doc = docs::call_type_doc(api_name, names, deprecated);
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

    let pager = names
        .pager
        .as_ref()
        .map(|pager| pager_forwards(tag, &request, pager));

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
            /// Returns `InvalidArgument` when the send would refuse the call's
            /// arguments: the request cannot be encoded, a header is invalid,
            /// or the credential cannot be placed.
            pub fn build(self) -> Result<Self, schematic_runtime::InvalidArgument> {
                self.0.build().map(Self)
            }

            #pager
        }

        impl<'a> ::core::future::IntoFuture for #call<'a> {
            type Output = <#runtime_call as ::core::future::IntoFuture>::Output;
            type IntoFuture = schematic_runtime::CallFuture<'a, Self::Output>;

            // Without `#[inline]` this concrete impl makes the generated crate
            // compile the runtime's send state machine for every endpoint;
            // inlined, only the endpoints a dependent awaits are compiled, in
            // that dependent, as with the generic `send`.
            #[inline]
            fn into_future(self) -> Self::IntoFuture {
                ::core::future::IntoFuture::into_future(self.0)
            }
        }
    }
}

/// The pager methods of a paginated endpoint's call type. Each is a thin
/// `#[inline]` forward, so only the ones a dependent calls are compiled.
fn pager_forwards(
    tag: &proc_macro2::Ident,
    request: &proc_macro2::Ident,
    forwards: &PagerForwards,
) -> TokenStream {
    let pager = quote! { schematic_runtime::pager::Pager<'a, #tag, #request> };
    let knobs = forwards.knobs.iter().map(|knob| {
        let name = format_ident!("{}", knob.name());
        let doc = format!(" See [`Pager::{name}`](schematic_runtime::pager::Pager::{name}).");
        let (arg, ty) = match knob {
            KnobForward::Page => (quote! { number }, quote! { u64 }),
            KnobForward::Offset => (quote! { offset }, quote! { u64 }),
            KnobForward::After => (
                quote! { cursor },
                quote! { impl Into<<#request as schematic_runtime::pager::ResumeAfter>::Cursor> },
            ),
            KnobForward::Before => (
                quote! { cursor },
                quote! { impl Into<<#request as schematic_runtime::pager::ResumeBefore>::Cursor> },
            ),
            KnobForward::PageSize => (quote! { size }, quote! { u32 }),
            KnobForward::KeepAlive => (quote! { value }, quote! { impl Into<String> }),
        };
        quote! {
            #[doc = #doc]
            #[inline]
            pub fn #name(self, #arg: #ty) -> #pager {
                self.0.pager().#name(#arg)
            }
        }
    });
    let iteration = forwards.sessionless.then(|| {
        quote! {
            /// See [`Pager::pages`](schematic_runtime::pager::Pager::pages).
            #[inline]
            pub fn pages(self) -> schematic_runtime::pager::Pages<'a, #tag, #request> {
                self.0.pager().pages()
            }

            /// See [`Pager::fetch_page`](schematic_runtime::pager::Pager::fetch_page).
            #[inline]
            pub fn fetch_page(
                self,
            ) -> schematic_runtime::CallFuture<'a, schematic_runtime::pager::PageResult<#request>> {
                self.0.pager().fetch_page()
            }

            /// See [`Pager::collect_all`](schematic_runtime::pager::Pager::collect_all).
            #[inline]
            pub fn collect_all(
                self,
            ) -> schematic_runtime::CallFuture<'a, schematic_runtime::pager::CollectResult<#request>> {
                self.0.pager().collect_all()
            }
        }
    });
    let partitioned = forwards.partitioned.then(|| {
        quote! {
            /// See [`Pager::partitioned`](schematic_runtime::pager::Pager::partitioned).
            ///
            /// ## Errors
            ///
            /// Returns `InvalidArgument` when `count` is zero or above the
            /// definition's maximum.
            #[inline]
            pub fn partitioned(
                self,
                count: u32,
            ) -> Result<
                Vec<schematic_runtime::pager::Pages<'a, #tag, #request>>,
                schematic_runtime::InvalidArgument,
            > {
                self.0.pager().partitioned(count)
            }
        }
    });
    let session = (!forwards.sessionless).then(|| {
        quote! {
            /// See [`Pager::close_grace`](schematic_runtime::pager::Pager::close_grace).
            #[inline]
            pub fn close_grace(self, grace: ::core::time::Duration) -> #pager {
                self.0.pager().close_grace(grace)
            }

            /// See [`Pager::with_session`](schematic_runtime::pager::Pager::with_session).
            #[inline]
            pub fn with_session<F, O>(
                self,
                scope: F,
            ) -> schematic_runtime::CallFuture<'a, schematic_runtime::pager::SessionResult<O, #request>>
            where
                F: for<'s> FnOnce(
                        &'s mut schematic_runtime::pager::SessionScope<'a, #tag, #request>,
                    ) -> schematic_runtime::CallFuture<'s, O>
                    + Send
                    + 'a,
                O: Send + 'a,
            {
                self.0.pager().with_session(scope)
            }
        }
    });

    quote! {
        /// See [`Call::pager`](schematic_runtime::Call::pager).
        #[inline]
        pub fn pager(self) -> #pager {
            self.0.pager()
        }

        #(#knobs)*

        #iteration

        #partitioned

        #session
    }
}

fn endpoint_method(
    api: &RestApi,
    endpoint: &Endpoint,
    names: &EndpointNames,
    args: &[RequiredArg],
) -> TokenStream {
    let method = format_ident!("{}", names.method);
    let call = format_ident!("{}", names.call_type);
    let request = format_ident!("{}", names.request_type);
    let doc = docs::endpoint_method_doc(api, endpoint, names, args);
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
