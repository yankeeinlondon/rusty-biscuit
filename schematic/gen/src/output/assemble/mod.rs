//! Assemble submodules for generated code.

mod api;
mod helpers;
mod lib_rs;
mod prelude;
mod shared;

pub use api::{
    SPLIT_THRESHOLD_LINES, SplitApiParts, assemble_api_code, assemble_api_module,
    assemble_api_module_with_options, assemble_combined_api_module, assemble_split_api_module,
    assemble_split_combined_api_module,
};
pub use helpers::{get_module_path, get_request_suffix};
pub use lib_rs::{assemble_lib_rs, assemble_lib_rs_with_options};
pub use prelude::{assemble_prelude, assemble_prelude_with_options};
pub use shared::assemble_shared_module;
