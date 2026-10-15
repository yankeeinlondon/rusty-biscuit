//! Shared module assembly (shared.rs).

use proc_macro2::TokenStream;
use quote::quote;

use crate::codegen::generate_request_helpers;

/// Assembles the shared module code (shared.rs).
///
/// It holds only what generated request types use to build their parts: the
/// cookie helpers and `FormFile`. Request execution, the response envelope,
/// errors, credentials, redirects, deadlines, redaction, and paging are
/// `schematic-runtime`'s.
pub fn assemble_shared_module() -> TokenStream {
    let request_helpers = generate_request_helpers();

    quote! {
        //! Request-side helpers shared by the generated API modules.
        //!
        //! Sending and response handling belong to [`schematic_runtime`].

        // `FormFile` is a field type on generated form-body structs, which derive
        // both traits, so it must derive them too.
        use serde::{Deserialize, Serialize};

        #request_helpers
    }
}
