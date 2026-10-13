//! prelude.rs assembly for the generated schema crate.

use proc_macro2::TokenStream;
use quote::{format_ident, quote};
use schematic_define::RestApi;

use super::helpers::get_module_path;
use crate::output::ws_modules::ws_definition_modules;

/// Assembles the prelude.rs content for the schema crate.
pub fn assemble_prelude(apis: &[&RestApi]) -> TokenStream {
    assemble_prelude_with_options(apis, true)
}

/// Assembles prelude.rs with optional WebSocket definition helper exports.
pub fn assemble_prelude_with_options(apis: &[&RestApi], include_ws_helpers: bool) -> TokenStream {
    let api_reexports: Vec<_> = apis
        .iter()
        .map(|api| {
            let module_name = format_ident!("{}", get_module_path(api));
            let client = format_ident!("{}", api.name);

            quote! {
                pub use crate::#module_name::#client;
            }
        })
        .collect();
    let ws_reexports: Vec<_> = if include_ws_helpers {
        ws_definition_modules()
            .iter()
            .map(|ws| {
                let module_name = format_ident!("{}", ws.module);
                let helper_name = format_ident!("{}", ws.helper);
                quote! {
                    pub use crate::#module_name::define_api as #helper_name;
                }
            })
            .collect()
    } else {
        Vec::new()
    };

    let client_list: Vec<String> = apis
        .iter()
        .map(|api| format!("//! - [`{name}`]", name = api.name))
        .collect();
    let client_list_str = client_list.join("\n");
    let client_list_tokens: TokenStream = client_list_str.parse().unwrap_or_default();
    let ws_helper_list_tokens: TokenStream = if include_ws_helpers {
        ws_definition_modules()
            .iter()
            .map(|ws| format!("//! - [`{}`] ({})", ws.helper, ws.display_name))
            .collect::<Vec<_>>()
            .join("\n")
            .parse()
            .unwrap_or_default()
    } else {
        TokenStream::new()
    };

    quote! {
        //! Convenient re-exports for working with generated API clients.
        //!
        //! This prelude exports each API's client struct, and
        //! the runtime types every call uses. Request, call, and response
        //! types are **not** re-exported here to avoid naming conflicts. An
        //! endpoint method needs none of them imported; when one must be
        //! named, import it from its API module:
        //!
        //! ```text
        //! use schematic_schema::openai::{ListModelsCall, ListModelsRequest, ListModelsResponse};
        //! ```
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
        //!
        //! ## Examples
        //!
        //! ```text
        //! use schematic_schema::prelude::*;
        //!
        //! let client = OpenAI::new();
        //! let model = client.retrieve_model("gpt-4").await?;
        //! println!("{}", model.id);
        //! ```

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
