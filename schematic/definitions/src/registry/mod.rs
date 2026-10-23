//! Module-group assembly for the shipped REST definitions.
//!
//! The APIs that share a generated types module are assembled from one
//! [`DefinitionContext`](schematic_define::DefinitionContext), and each of
//! them carries that context's complete schema components in
//! [`RestApi::schemas`](schematic_define::RestApi::schemas).
//!
//! ```
//! use schematic_definitions::registry::assemble_module_group;
//! use schematic_definitions::openai::{ListModelsResponse, Model};
//! use schematic_define::{Endpoint, RestApi};
//!
//! let apis = assemble_module_group("::schematic_definitions::openai", |ctx| {
//!     ctx.register::<Model>()?;
//!     let list = Endpoint::get("ListModels", "/models").describes("List models")
//!         .returns_json::<ListModelsResponse>(ctx).build()?;
//!     let api = RestApi::builder(ctx, "Models", "https://api.example.com")
//!         .describes("Models").no_auth().endpoints([list]).build()?;
//!     Ok(vec![api])
//! })?;
//! assert_eq!(apis[0].schemas.iter().map(|c| c.name.as_str()).collect::<Vec<_>>(),
//!     ["Model", "ListModelsResponse"]);
//! # Ok::<(), schematic_define::DefinitionError>(())
//! ```

mod typed;
pub use typed::assemble_module_group;
mod inventory;

pub use inventory::*;

/// Names of the registered (Rust-typed) components of `api`, in order.
#[cfg(test)]
pub(crate) fn registered_names(api: &schematic_define::RestApi) -> Vec<&str> {
    api.schemas.iter().filter(|c| c.rust_path.is_some()).map(|c| c.name.as_str()).collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::openai::Model;
    use schematic_define::DefinitionContext;

    #[test]
    fn component_schema_includes_description_from_doc_comments() {
        let mut context = DefinitionContext::types_in("::schematic_definitions::openai");
        context.register::<Model>().unwrap();
        let components = context.components().unwrap();
        let model = components.iter().find(|c| c.name == "Model").expect("Model component");
        let description = model.schema["description"].as_str();
        assert!(description.is_some_and(|d| d.contains("OpenAI model")), "got {description:?}");
    }

    #[test]
    fn registry_key_for_unknown_falls_back_to_lowercase() {
        assert_eq!(registry_key_for("MysteryApi"), "mysteryapi");
    }
}
