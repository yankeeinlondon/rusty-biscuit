//! prelude.rs assembly for the generated schema crate.

use proc_macro2::TokenStream;
use quote::{format_ident, quote};
use schematic_define::RestApi;

use schematic_definitions::naming::resolve_module_name;
use schematic_definitions::registry::ApiRegistration;

use super::lib_rs::{client_example, example_api, feature_doc};
use crate::output::ws_modules::{ws_helper_name, ws_registrations};

/// Assembles the prelude.rs content for the schema crate, exporting the
/// `define_api` helper of every WebSocket entry in `registrations` (none for
/// an empty slice).
///
/// Each re-export, and its line in the module docs, is behind the Cargo
/// feature of the module it comes from.
pub fn assemble_prelude(apis: &[&RestApi], registrations: &[ApiRegistration]) -> TokenStream {
    let api_reexports: Vec<_> = apis
        .iter()
        .map(|api| {
            let module = resolve_module_name(api);
            let module_name = format_ident!("{}", module);
            let client = format_ident!("{}", api.name);

            quote! {
                #[cfg(feature = #module)]
                pub use crate::#module_name::#client;
            }
        })
        .collect();
    let ws_reexports: Vec<_> = ws_registrations(registrations)
        .map(|(entry, _)| {
            let module = entry.module;
            let module_name = format_ident!("{}", module);
            let helper_name = format_ident!("{}", ws_helper_name(module));
            quote! {
                #[cfg(feature = #module)]
                pub use crate::#module_name::define_api as #helper_name;
            }
        })
        .collect();

    let client_list_tokens: TokenStream = apis
        .iter()
        .map(|api| feature_doc(&resolve_module_name(api), &format!(" - [`{}`]", api.name)))
        .collect();
    let ws_helper_list_tokens: TokenStream = ws_registrations(registrations)
        .map(|(entry, _)| {
            feature_doc(
                entry.module,
                &format!(" - [`{}`] ({})", ws_helper_name(entry.module), entry.name),
            )
        })
        .collect();

    // The example compiles only where its API's module does.
    let example_tokens: TokenStream = match example_api(apis) {
        Some(api) => {
            let module = resolve_module_name(api);
            let mut lines = vec![String::new(), " ## Examples".to_string(), String::new()];
            lines.extend(
                client_example(Some(api))
                    .into_iter()
                    .map(|line| if line.is_empty() { line } else { format!(" {line}") }),
            );
            lines.iter().map(|line| feature_doc(&module, line)).collect()
        }
        None => TokenStream::new(),
    };

    quote! {
        //! Convenient re-exports for working with generated API clients.
        //!
        //! This prelude exports the client struct of each API whose module
        //! feature is enabled, and the runtime types every call uses. Request, call, and response
        //! types are **not** re-exported here to avoid naming conflicts. An
        //! endpoint method needs none of them imported; when one must be
        //! named, import it from its API module.
        //!
        //! ## Re-exports
        //!
        //! **API clients:**
        //!
        #client_list_tokens
        //!
        //! **WebSocket definition helpers:**
        //!
        #ws_helper_list_tokens
        //!
        //! **Runtime types:** [`Call`], [`CallError`], [`CallOptions`],
        //! [`Credential`], [`Deadline`], [`FullCall`], [`OriginPolicy`],
        //! [`Response`], [`RawResponse`], and [`TransportError`] from
        //! [`schematic_runtime`].
        #example_tokens

        pub use schematic_runtime::{
            Call, CallError, CallOptions, Credential, Deadline, FullCall, OriginPolicy, RawResponse,
            Response, TransportError,
        };

        // API clients
        #(#api_reexports)*

        // WebSocket definition helpers
        #(#ws_reexports)*
    }
}
