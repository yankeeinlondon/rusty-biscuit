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
mod lookup;

pub use lookup::*;

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
    fn registry_key_for_covers_every_shipped_api() {
        let table = [
            ("Anthropic", "anthropic"),
            ("ArtificialAnalysisData", "artificial-analysis-data"),
            ("ArtificialAnalysisCritPt", "artificial-analysis-critpt"),
            ("OpenAI", "openai"),
            ("ElevenLabs", "elevenlabs"),
            ("Gitea", "gitea"),
            ("GitHub", "github"),
            ("GitLab", "gitlab"),
            ("HuggingFaceHub", "huggingface"),
            ("LmStudio", "lmstudio"),
            ("OllamaNative", "ollama-native"),
            ("OllamaOpenAI", "ollama-openai"),
            ("EmqxBasic", "emqx-basic"),
            ("EmqxBearer", "emqx-bearer"),
            ("Eversolo", "eversolo"),
            ("SamsungSmartTv", "samsung-smart-tv"),
            ("UnfoldedCircleCoreRest", "unfolded-circle-core-rest"),
            ("Bitbucket", "bitbucket"),
        ];
        for (api_name, expected_key) in table {
            assert_eq!(registry_key_for(api_name), expected_key, "registry_key_for({api_name:?})");
        }
        let mut shipped: Vec<String> = crate::all_apis().into_iter().map(|api| api.name).collect();
        let mut listed: Vec<String> = table.iter().map(|(name, _)| (*name).to_owned()).collect();
        shipped.sort();
        listed.sort();
        assert_eq!(shipped, listed, "every shipped API needs an explicit key");
    }

    #[test]
    fn registry_key_for_unknown_falls_back_to_lowercase() {
        assert_eq!(registry_key_for("MysteryApi"), "mysteryapi");
    }
}
