//! Assembly of the APIs that share one generated types module.
use schematic_define::{DefinitionContext, DefinitionError, RestApi};

/// Assemble the APIs of one module group from a single registration context.
///
/// The callback owns its API inventory; this seam maintains no parallel list.
/// Every returned API carries the finished context's complete components in
/// [`RestApi::schemas`], including types a later member registered after an
/// earlier member was built, and is re-validated against them.
///
/// ## Errors
///
/// Every distinct registration collision in the shared context, even when the
/// callback stopped on one; otherwise the callback's own error, or the first
/// reference that does not resolve in the finished components.
pub fn assemble_module_group(
    module_path: &str,
    define: impl FnOnce(&mut DefinitionContext) -> Result<Vec<RestApi>, DefinitionError>,
) -> Result<Vec<RestApi>, DefinitionError> {
    let mut context = DefinitionContext::types_in(module_path);
    let apis = define(&mut context);
    context.finish()?;
    let mut apis = apis?;
    let schemas = context.components()?;
    for api in &mut apis {
        api.schemas = schemas.clone();
        api.validate_references()?;
    }
    Ok(apis)
}

#[cfg(test)]
mod tests {
    use super::*;
    use super::super::registered_names;
    use schematic_define::{SuccessShape, Endpoint, TypeRef, definition_type, rust_path};
    use schemars::JsonSchema;
    use crate::huggingface::CommitInfo;
    mod ollama_fixture {
        use super::*;
        definition_type!(#[derive(JsonSchema)] #[allow(dead_code)] pub struct PullModelProgress { pub status: String });
        pub type PushModelProgress = PullModelProgress;
    }
    use ollama_fixture::PullModelProgress;

    definition_type!(#[derive(JsonSchema)] #[allow(dead_code)] struct Nested { value: String });
    definition_type!(#[derive(JsonSchema)] #[allow(dead_code)] struct Explicit { nested: Nested });
    definition_type!(#[derive(JsonSchema)] #[schemars(rename = "Explicit")] #[allow(dead_code)] struct Clash { other: bool });
    definition_type!(#[derive(JsonSchema)] #[allow(dead_code)] struct Late { flag: bool });

    fn fixture_api<T: schematic_define::Registration>(ctx: &mut DefinitionContext, name: &str) -> Result<RestApi, DefinitionError> {
        let endpoint = Endpoint::get("Get", "/item").describes("Get").returns_json::<T>(ctx).build()?;
        RestApi::builder(ctx, name, "https://example.test").describes(name).no_auth().endpoints([endpoint]).build()
    }

    fn component<'a>(components: &'a [schematic_define::SchemaComponent], name: &str) -> &'a schematic_define::SchemaComponent {
        components.iter().find(|c| c.name == name).unwrap_or_else(|| panic!("missing component {name}"))
    }

    #[test]
    fn aliases_preserve_shipped_huggingface_and_ollama_type_identity() {
        let mut hf = DefinitionContext::types_in("::schematic_definitions::huggingface");
        let commit = hf.register_as::<CommitInfo>(rust_path!(crate::huggingface::Commit).schema_name("Commit")).unwrap();
        assert_eq!(hf.register::<Vec<CommitInfo>>().unwrap().type_ref(), &TypeRef::Array(Box::new(TypeRef::Named("Commit".into()))));
        assert!(commit.rust_path().ends_with("::huggingface::Commit"));
        assert_eq!(hf.register::<CommitInfo>().unwrap(), commit);
        let components = hf.components().unwrap();
        assert_eq!(components.iter().map(|c| c.name.as_str()).collect::<Vec<_>>(), ["Commit"]);
        assert!(component(&components, "Commit").rust_path.as_deref().is_some_and(|p| p.ends_with("::huggingface::Commit")));

        let mut ollama = DefinitionContext::types_in("::schematic_definitions::ollama");
        let progress = ollama.register_as::<PullModelProgress>(rust_path!(ollama_fixture::PushModelProgress)).unwrap();
        assert!(progress.rust_path().ends_with("::PushModelProgress"));
        assert_eq!(ollama.register::<PullModelProgress>().unwrap(), progress);
        assert_eq!(ollama.registrations().len(), 1);
        let components = ollama.components().unwrap();
        assert_eq!(component(&components, "PullModelProgress").rust_path, Some(progress.rust_path().to_owned()));
    }

    #[test]
    fn members_built_before_a_later_registration_carry_the_full_components() {
        let apis = assemble_module_group("::fixture", |ctx| {
            let first = fixture_api::<Nested>(ctx, "First")?;
            // Built before `Late` exists, so its own build saw a partial context.
            assert!(first.component("Late").is_none());
            let second = fixture_api::<Late>(ctx, "Second")?;
            Ok(vec![first, second])
        }).unwrap();
        assert_eq!(apis.iter().map(|a| a.name.as_str()).collect::<Vec<_>>(), ["First", "Second"]);
        assert_eq!(registered_names(&apis[0]), ["Nested", "Late"]);
        assert_eq!(apis[0].schemas, apis[1].schemas);
        for api in &apis { api.validate_references().unwrap(); }
    }

    #[test]
    fn a_dangling_reference_fails_group_assembly() {
        let error = assemble_module_group("::fixture", |ctx| {
            let mut api = fixture_api::<Nested>(ctx, "Fixture")?;
            api.endpoints[0].response = vec![SuccessShape::json(TypeRef::Array(Box::new(TypeRef::Named("Missing".into()))))];
            Ok(vec![api])
        }).unwrap_err();
        assert_eq!(error, DefinitionError::UnresolvedReference { api: "Fixture".into(), endpoint: "Get".into(), name: "Missing".into() });
    }

    #[test]
    fn components_preserve_order_nested_extraction_and_unused_types() {
        let apis = assemble_module_group("::fixture", |ctx| {
            ctx.register::<Explicit>()?;
            ctx.register::<Nested>()?;
            ctx.register::<Explicit>()?;
            Ok(vec![fixture_api::<Late>(ctx, "Fixture")?])
        }).unwrap();
        assert_eq!(registered_names(&apis[0]), ["Explicit", "Nested", "Late"]);
        let mut context = DefinitionContext::types_in("::fixture");
        context.register::<Explicit>().unwrap();
        let nested = context.components().unwrap();
        assert_eq!(nested.iter().map(|c| (c.name.as_str(), c.rust_path.is_some())).collect::<Vec<_>>(), [("Explicit", true), ("Nested", false)]);
    }

    mod left_status {
        use super::*;
        definition_type!(#[derive(JsonSchema)] #[allow(dead_code)] pub struct Status { value: String });
        definition_type!(#[derive(JsonSchema)] #[allow(dead_code)] pub struct Left { status: Status });
    }
    mod right_status {
        use super::*;
        definition_type!(#[derive(JsonSchema)] #[allow(dead_code)] pub struct Status { value: bool });
        definition_type!(#[derive(JsonSchema)] #[allow(dead_code)] pub struct Right { status: Status });
    }

    #[test]
    fn a_nested_component_conflict_registered_after_a_member_build_fails_assembly() {
        let error = assemble_module_group("::fixture", |ctx| {
            let first = fixture_api::<left_status::Left>(ctx, "First")?;
            ctx.register::<right_status::Right>()?;
            Ok(vec![first])
        }).unwrap_err();
        let DefinitionError::Collisions(errors) = &error else { panic!("expected collisions, got {error:?}") };
        assert!(matches!(errors.as_slice(), [DefinitionError::ComponentConflict { schema_name, first, second }]
            if schema_name == "Status" && first.ends_with("left_status::Left") && second.ends_with("right_status::Right")), "{errors:?}");
    }

    #[test]
    fn isolated_groups_can_reuse_names_but_shared_collisions_fail_assembly() {
        for clash in [false, true] {
            let apis = assemble_module_group("::fixture", |ctx| {
                if clash { Ok(vec![fixture_api::<Clash>(ctx, "Fixture")?]) } else { Ok(vec![fixture_api::<Explicit>(ctx, "Fixture")?]) }
            }).unwrap();
            assert_eq!(registered_names(&apis[0])[0], "Explicit");
        }
        let error = assemble_module_group("::fixture", |ctx| {
            ctx.register::<Explicit>()?;
            ctx.register::<Clash>()?;
            Ok(vec![])
        }).unwrap_err();
        assert!(matches!(error, DefinitionError::Collisions(errors) if errors.len() == 1));
        // The callback swallowing the collision does not hide it.
        let error = assemble_module_group("::fixture", |ctx| {
            ctx.register::<Explicit>()?;
            let _ = ctx.register::<Clash>();
            Ok(vec![])
        }).unwrap_err();
        assert!(matches!(error, DefinitionError::Collisions(errors) if errors.len() == 1));
        assert!(assemble_module_group("::fixture", |_| Err(DefinitionError::NotLeaf)).is_err());
    }
}

#[cfg(test)]
mod generic_paths {
    use super::*;

    #[test]
    fn bitbucket_nominal_wrapper_retains_its_concrete_public_type_path() {
        let mut context = DefinitionContext::types_in("::schematic_definitions::bitbucket");
        let descriptor = context.register::<crate::bitbucket::PaginatedResponse<crate::bitbucket::Repository>>().unwrap();
        let path: String = descriptor.rust_path().split_whitespace().collect();
        assert!(path.contains("bitbucket::PaginatedResponse<"), "{path}");
        assert!(path.contains("bitbucket::Repository>"), "{path}");
        assert_eq!(descriptor.type_ref(), &schematic_define::TypeRef::Named("PaginatedResponse<Repository>".into()));
        let components = context.components().unwrap();
        let component = components.iter().find(|c| c.name == "PaginatedResponse<Repository>").expect("wrapper component");
        assert_eq!(component.rust_path.as_deref(), Some(descriptor.rust_path()));
    }
}
