//! Additive assembly seam for definitions and their shared module registry.
use schematic_define::{DefinitionContext, DefinitionError, RestApi};
use super::SchemaRegistry;

/// Definitions and registry produced from the same module-group context.
#[derive(Debug)]
pub struct ModuleGroup {
    /// APIs sharing the generated types module, in authored order.
    pub apis: Vec<RestApi>,
    /// Ordered nominal registrations, retaining nested schema extraction.
    pub registry: SchemaRegistry,
}

/// Assemble a module group once, propagating registration failures.
///
/// The callback owns its API inventory; this seam maintains no parallel list.
/// Existing literal-based lookups remain available until migration.
pub fn assemble_module_group(
    module_path: &str,
    define: impl FnOnce(&mut DefinitionContext) -> Result<Vec<RestApi>, DefinitionError>,
) -> Result<ModuleGroup, DefinitionError> {
    let mut context = DefinitionContext::types_in(module_path);
    let apis = define(&mut context);
    // Preserve all collected collisions even if the callback stops on one.
    context.finish()?;
    let apis = apis?;
    let registry = SchemaRegistry::from_context(&context)?;
    Ok(ModuleGroup { apis, registry })
}

#[cfg(test)]
mod tests {
    use super::*;
    use schematic_define::{Registration, TypeDesc, definition_type, rust_path};
    use schemars::JsonSchema;
    use crate::huggingface::CommitInfo;
    mod ollama_fixture {
        use super::*;
        definition_type!(#[derive(JsonSchema)] #[allow(dead_code)] pub struct PullModelProgress { pub status: String });
        pub type PushModelProgress = PullModelProgress;
    }
    use ollama_fixture::PullModelProgress;
    use crate::emqx::NodeInfo;

    // Test-only adaptations leave the shipped literal-based factories untouched.
    macro_rules! adapt {
        ($($t:ty),*) => { $(
            impl Registration for $t {
                fn register(c: &mut DefinitionContext) -> Result<TypeDesc, DefinitionError> {
                    c.register_as::<Self>(rust_path!($t))
                }
            }
        )* };
    }
    adapt!(CommitInfo, NodeInfo);

    definition_type!(#[derive(JsonSchema)] #[allow(dead_code)] struct Nested { value: String });
    definition_type!(#[derive(JsonSchema)] #[allow(dead_code)] struct Explicit { nested: Nested });
    definition_type!(#[derive(JsonSchema)] #[schemars(rename = "Explicit")] #[allow(dead_code)] struct Clash { other: bool });

    #[test]
    fn aliases_preserve_shipped_huggingface_and_ollama_type_identity() {
        let mut hf = DefinitionContext::types_in("::schematic_definitions::huggingface");
        let commit = hf.register_as::<CommitInfo>(rust_path!(crate::huggingface::Commit).schema_name("Commit")).unwrap();
        assert_eq!(hf.register::<Vec<CommitInfo>>().unwrap().wire_schema(), serde_json::json!({"type":"array","items":{"$ref":"#/components/schemas/Commit"}}));
        assert!(commit.rust_path().ends_with("::huggingface::Commit"));
        assert_eq!(hf.register::<CommitInfo>().unwrap(), commit);
        let registry = SchemaRegistry::from_context(&hf).unwrap();
        let expected = SchemaRegistry::new().register::<CommitInfo>("Commit");
        assert_eq!(registry.get("Commit"), expected.get("Commit"));
        assert_eq!(registry.names().collect::<Vec<_>>(), ["Commit"]);

        let mut ollama = DefinitionContext::types_in("::schematic_definitions::ollama");
        let progress = ollama.register_as::<PullModelProgress>(rust_path!(ollama_fixture::PushModelProgress)).unwrap();
        assert!(progress.rust_path().ends_with("::PushModelProgress"));
        assert_eq!(ollama.register::<PullModelProgress>().unwrap(), progress);
        assert_eq!(ollama.registrations().len(), 1);
        assert_eq!(SchemaRegistry::from_context(&ollama).unwrap().get("PullModelProgress"), SchemaRegistry::new().register::<PullModelProgress>("PullModelProgress").get("PullModelProgress"));
    }

    #[test]
    fn shared_ollama_and_emqx_assemblies_reuse_one_ordered_context() {
        let ollama = assemble_module_group("::schematic_definitions::ollama", |ctx| {
            let first = ctx.register::<PullModelProgress>()?;
            let second = ctx.register::<Option<Vec<PullModelProgress>>>()?;
            ctx.validate_reference("OllamaNative", "Pull", &first)?;
            ctx.validate_reference("OllamaOpenAI", "Progress", &second)?;
            Ok(vec![crate::ollama::define_ollama_native_api(), crate::ollama::define_ollama_openai_api()])
        }).unwrap();
        assert_eq!(ollama.apis.iter().map(|a| a.name.as_str()).collect::<Vec<_>>(), ["OllamaNative", "OllamaOpenAI"]);
        assert_eq!(ollama.registry.names().collect::<Vec<_>>(), ["PullModelProgress"]);
        let emqx = assemble_module_group("::schematic_definitions::emqx", |ctx| {
            ctx.register::<NodeInfo>()?;
            ctx.register::<Vec<NodeInfo>>()?;
            ctx.register::<NodeInfo>()?;
            Ok(vec![crate::emqx::define_emqx_basic_api(), crate::emqx::define_emqx_bearer_api()])
        }).unwrap();
        assert_eq!(emqx.apis.len(), 2);
        assert_eq!(emqx.registry.names().collect::<Vec<_>>(), ["NodeInfo"]);
    }

    #[test]
    fn adapter_preserves_order_nested_extraction_and_unused_types() {
        let group = assemble_module_group("::fixture", |ctx| {
            ctx.register::<Explicit>()?;
            ctx.register::<Nested>()?;
            ctx.register::<Explicit>()?;
            Ok(vec![])
        }).unwrap();
        assert!(group.apis.is_empty());
        assert_eq!(group.registry.names().collect::<Vec<_>>(), ["Explicit", "Nested"]);
        let expected = SchemaRegistry::new().register::<Explicit>("Explicit").register::<Nested>("Nested");
        assert_eq!(group.registry.to_openapi_schemas(), expected.to_openapi_schemas());
        assert_eq!(group.registry.to_openapi_schemas().keys().map(String::as_str).collect::<Vec<_>>(), ["Explicit", "Nested"]);
        let mut context = DefinitionContext::types_in("::fixture");
        context.register::<Explicit>().unwrap();
        let nested = SchemaRegistry::from_context(&context).unwrap().to_openapi_schemas();
        assert!(nested.contains_key("Nested"));
    }

    #[test]
    fn isolated_groups_can_reuse_names_but_shared_collisions_fail_assembly() {
        for clash in [false, true] {
            let group = assemble_module_group("::fixture", |ctx| {
                if clash { ctx.register::<Clash>()?; } else { ctx.register::<Explicit>()?; }
                Ok(vec![])
            }).unwrap();
            assert_eq!(group.registry.names().collect::<Vec<_>>(), ["Explicit"]);
        }
        let error = assemble_module_group("::fixture", |ctx| {
            ctx.register::<Explicit>()?;
            ctx.register::<Clash>()?;
            Ok(vec![])
        }).unwrap_err();
        assert!(matches!(error, DefinitionError::Collisions(errors) if errors.len() == 1));
        let mut invalid = DefinitionContext::types_in("::fixture");
        invalid.register::<Explicit>().unwrap();
        assert!(invalid.register::<Clash>().is_err());
        assert!(SchemaRegistry::from_context(&invalid).is_err());
        assert!(assemble_module_group("::fixture", |_| Err(DefinitionError::NotLeaf)).is_err());
    }
}
