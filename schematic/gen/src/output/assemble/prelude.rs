//! prelude.rs assembly for the generated schema crate.

use proc_macro2::TokenStream;
use quote::{format_ident, quote};
use schematic_define::RestApi;

use super::helpers::get_module_path;
use crate::output::ws_modules::ws_definition_modules;

/// Assembles the prelude.rs content for the schema crate.
///
/// ## Arguments
///
/// * `apis` - Slice of API definitions to include
///
/// ## Returns
///
/// A TokenStream containing the prelude.rs code.
pub fn assemble_prelude(apis: &[&RestApi]) -> TokenStream {
    assemble_prelude_with_options(apis, true)
}

/// Assembles prelude.rs with optional WebSocket definition helper exports.
pub fn assemble_prelude_with_options(apis: &[&RestApi], include_ws_helpers: bool) -> TokenStream {
    let api_reexports: Vec<_> = apis
        .iter()
        .map(|api| {
            let module_name = format_ident!("{}", get_module_path(api));
            let client_name = format_ident!("{}", api.name);
            let request_enum = format_ident!("{}Request", api.name);

            quote! {
                pub use crate::#module_name::{#client_name, #request_enum};
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
        .map(|api| format!("//! - [`{name}`] + [`{name}Request`]", name = api.name))
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
        //! This prelude exports the client structs, request enums, and shared error
        //! types. Response types are **not** re-exported here to avoid naming conflicts.
        //! Import them from specific API modules instead:
        //!
        //! ```text
        //! use schematic_schema::openai::Model;
        //! use schematic_schema::anthropic::CreateMessageResponse;
        //! ```
        //!
        //! ## Re-exports
        //!
        //! **Clients and request enums:**
        //!
        #client_list_tokens
        //!
        //! **WebSocket definition helpers:**
        //!
        #ws_helper_list_tokens
        //!
        //! **Shared types:**
        //!
        //! - [`SchematicError`] - Error type for all API operations
        //! - [`RequestParts`] - Low-level request decomposition tuple
        //!
        //! ## Examples
        //!
        //! ```text
        //! use schematic_schema::prelude::*;
        //!
        //! #[tokio::main]
        //! async fn main() -> Result<(), SchematicError> {
        //!     let client = OpenAI::new();
        //!
        //!     // Typed request with new()
        //!     let models = client.list_models().await?;
        //!
        //!     // Ergonomic From<&str> for single-param endpoints
        //!     use schematic_schema::openai::{RetrieveModelRequest, Model};
        //!     let model: Model = client
        //!         .request(RetrieveModelRequest::from("gpt-4"))
        //!         .await?;
        //!
        //!     Ok(())
        //! }
        //! ```

        // Shared types
        pub use crate::shared::{RequestParts, SchematicError};

        // API clients and request types
        #(#api_reexports)*

        // WebSocket definition helpers
        #(#ws_reexports)*
    }
}
