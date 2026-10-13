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
            let marker = format_ident!("{}", api.name);
            let client = format_ident!("{}Client", api.name);

            quote! {
                pub use crate::#module_name::{#marker, #client};
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
        .map(|api| format!("//! - [`{name}`] + [`{name}Client`]", name = api.name))
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
        //! This prelude exports each API's marker type and client alias, and
        //! the runtime types every send uses. Request and response types are
        //! **not** re-exported here to avoid naming conflicts. Import them
        //! from specific API modules instead:
        //!
        //! ```text
        //! use schematic_schema::openai::{ListModelsRequest, ListModelsResponse};
        //! ```
        //!
        //! ## Re-exports
        //!
        //! **APIs and their clients:**
        //!
        #client_list_tokens
        //!
        //! **WebSocket definition helpers:**
        //!
        #ws_helper_list_tokens
        //!
        //! **Runtime types:** [`CallOptions`], [`Credential`], [`Deadline`],
        //! [`OriginPolicy`], [`Response`], [`RawResponse`], and
        //! [`TransportError`] from [`schematic_runtime`].
        //!
        //! ## Examples
        //!
        //! ```text
        //! use schematic_schema::prelude::*;
        //! use schematic_schema::openai::RetrieveModelRequest;
        //!
        //! let client = OpenAI::new()?;
        //! let response = client
        //!     .send(RetrieveModelRequest::from("gpt-4"), &CallOptions::new())
        //!     .await?;
        //! if let Some(model) = response.data() {
        //!     println!("{}", model.id);
        //! }
        //! ```

        pub use schematic_runtime::{
            CallOptions, Credential, Deadline, OriginPolicy, RawResponse, Response, TransportError,
        };

        // APIs and their clients
        #(#api_reexports)*

        // WebSocket definition helpers
        #(#ws_reexports)*
    }
}
