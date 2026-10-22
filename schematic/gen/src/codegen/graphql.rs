//! Request types for an API's GraphQL operations (R35, R36).
//!
//! Each operation becomes a request struct holding its typed variables and
//! implementing `schematic_runtime::EndpointSpec` with the `GraphQl<D>`
//! codec, so it goes through the same generic send, credentials,
//! cross-origin rule, deadline, and envelope as a REST endpoint. The runtime
//! sends it to the GraphQL address (`EndpointSpec::ADDRESS`), which the API
//! definition derives from the base URL by the definition's address rules
//! unless the caller overrides it. The document is emitted verbatim and never
//! parsed. See `schematic/docs/graphql.md`.

use proc_macro2::TokenStream;
use quote::{format_ident, quote};
use schematic_define::{AddressRule, GraphQlOperation, GraphQlVariables, OperationKind, RestApi};

use crate::codegen::request_structs::shared::{json_type_name, type_name_to_tokens};

/// The request struct and `EndpointSpec` impl of `operation`, named
/// `{id}{suffix}` like an endpoint's request struct.
///
/// Typed variables are the struct's one field, taken by `new`; an operation
/// without variables is a unit-like struct whose body omits `variables`
/// (Default D-27). `operationName` is sent only when the definition sets one
/// (Default D-21). The failure type is the runtime's composite of GraphQL
/// errors and the operation's effective HTTP error table (R37).
pub(crate) fn generate_operation_request(api: &RestApi, operation: &GraphQlOperation, suffix: &str, api_marker: &str) -> TokenStream {
    let struct_name = format_ident!("{}{}", operation.id, suffix);
    let marker = format_ident!("{}", api_marker);
    let id = &operation.id;
    let document = &operation.document;
    let data = type_name_to_tokens(&json_type_name(api, &operation.data));
    let operation_name = match &operation.operation_name {
        Some(name) => quote! { Some(#name) },
        None => quote! { None },
    };
    let failure = crate::codegen::responses::failure_code(api, id, &api.effective_operation_errors(operation), api_marker);
    let http_failure = &failure.failure;
    let errors_const = &failure.errors;
    let failure_items = &failure.items;

    let kind = match operation.kind {
        OperationKind::Query => "query",
        OperationKind::Mutation => "mutation",
        _ => "operation",
    };
    let summary = format!(" GraphQL {kind} `{id}`: {}", operation.description);
    let send_doc = " Send it with the client's `send`; it goes to the API's GraphQL address.";

    let (fields, constructor, variables) = match &operation.variables {
        GraphQlVariables::Typed(reference) => {
            let ty = type_name_to_tokens(&json_type_name(api, reference));
            (
                quote! { { #[doc = " The operation's variables, sent as `variables`."] pub variables: #ty, } },
                quote! {
                    #[doc = " Creates the request with its variables."]
                    pub fn new(variables: #ty) -> Self { Self { variables } }
                },
                quote! { Some(&self.variables) },
            )
        }
        _ => (
            quote! { ; },
            quote! {
                #[doc = " Creates the request; the operation takes no variables."]
                pub fn new() -> Self { Self }
            },
            quote! { None::<&()> },
        ),
    };
    let derives = match &operation.variables {
        GraphQlVariables::Typed(_) => quote! { #[derive(Debug, Clone)] },
        _ => quote! { #[derive(Debug, Clone, Copy, Default)] },
    };

    quote! {
        #[doc = #summary]
        #[doc = ""]
        #[doc = #send_doc]
        #derives
        pub struct #struct_name #fields

        impl #struct_name {
            #[doc = " The GraphQL document, sent exactly as written."]
            pub const DOCUMENT: &'static str = #document;
            #[doc = " The `operationName` sent with the document, if any."]
            pub const OPERATION_NAME: Option<&'static str> = #operation_name;

            #constructor
        }

        #failure_items

        impl schematic_runtime::EndpointSpec for #struct_name {
            type Api = #marker;
            type Success = #data;
            type Failure = schematic_runtime::GraphQlOperationError<#data, #http_failure>;
            type Decoder = schematic_runtime::GraphQl<#data>;

            const ENDPOINT_ID: &'static str = #id;
            const METHOD: schematic_runtime::Method = schematic_runtime::Method::POST;
            const ADDRESS: schematic_runtime::Address = schematic_runtime::Address::GraphQl;
            #errors_const

            fn into_parts(self) -> Result<schematic_runtime::RequestParts, schematic_runtime::InvalidArgument> {
                let body = schematic_runtime::graphql::request_body(Self::DOCUMENT, #variables, Self::OPERATION_NAME)?;
                Ok(schematic_runtime::RequestParts {
                    method: <Self as schematic_runtime::EndpointSpec>::METHOD,
                    path: String::new(),
                    query: Vec::new(),
                    headers: vec![("Accept".to_string(), "application/json".to_string())],
                    body,
                })
            }
        }
    }
}

/// The `.graphql(..)` call of the API definition: the address rules in
/// order, or nothing when the API has no GraphQL surface.
pub(crate) fn address_rules(api: &RestApi) -> Option<TokenStream> {
    let surface = api.graphql.as_ref()?;
    let rules = surface.address.iter().map(|rule| match rule {
        AddressRule::ReplaceSuffix { suffix, with } => {
            quote! { schematic_runtime::graphql::AddressRule::ReplaceSuffix { suffix: #suffix, with: #with } }
        }
        AddressRule::Append(path) => quote! { schematic_runtime::graphql::AddressRule::Append(#path) },
        // `AddressRule` is `#[non_exhaustive]`; a rule added later fails the
        // generated crate's build rather than deriving another address.
        other => {
            let message = format!("schematic-gen cannot emit the GraphQL address rule {other:?}");
            quote! { compile_error!(#message) }
        }
    });
    Some(quote! { .graphql(&[#(#rules),*]) })
}

#[cfg(test)]
mod tests {
    use schematic_define::{AuthStrategy, ErrorEntry, GraphQlSurface, ResponseShape, SchemaComponent, StatusMatch, TypeRef};
    use serde_json::json;

    use super::*;
    use crate::test_utils::{make_api, validate_generated_code};

    fn squash(code: &str) -> String {
        let flat: String = code.split_whitespace().collect();
        flat.replace(",)", ")").replace(",]", "]")
    }

    fn operation(id: &str, variables: Option<&str>, edit: impl FnOnce(&mut GraphQlOperation)) -> GraphQlOperation {
        let mut ctx = schematic_define::DefinitionContext::types_in("::defs");
        let mut operation = GraphQlOperation::query(id, "query Q($n: Int) { item(n: $n) { id } }")
            .describes("Read one item").no_variables().data::<serde_json::Value>(&mut ctx).build().unwrap();
        operation.data = TypeRef::Named("ItemData".into());
        if let Some(name) = variables { operation.variables = GraphQlVariables::Typed(TypeRef::Named(name.into())); }
        edit(&mut operation);
        operation
    }

    fn api_with(operations: Vec<GraphQlOperation>) -> RestApi {
        let mut api = make_api("Hub", "https://ghe.example/api/v3", AuthStrategy::None, vec![]);
        api.schemas = ["ItemData", "ItemVars", "Problem"].iter().map(|name| SchemaComponent {
            name: (*name).to_string(),
            rust_path: Some(format!("::defs::{name}")),
            schema: json!({"type": "object"}),
        }).collect();
        let mut surface = GraphQlSurface::new(AddressRule::github());
        surface.operations = operations;
        api.graphql = Some(surface);
        api
    }

    fn generated(api: &RestApi, index: usize) -> String {
        let operation = &api.graphql.as_ref().unwrap().operations[index];
        let tokens = generate_operation_request(api, operation, "Request", "HubApi");
        validate_generated_code(&tokens).expect("generated code parses");
        squash(&tokens.to_string())
    }

    fn assert_has(code: &str, expected: &str) {
        assert!(code.contains(&squash(expected)), "missing `{expected}` in:\n{code}");
    }

    #[test]
    fn typed_variables_are_the_request_field_and_reach_the_body_with_the_operation_name() {
        let api = api_with(vec![operation("ReadItem", Some("ItemVars"), |op| op.operation_name = Some("Q".into()))]);
        let code = generated(&api, 0);
        assert_has(&code, "pub struct ReadItemRequest { #[doc = \" The operation's variables, sent as `variables`.\"] pub variables: ::defs::ItemVars, }");
        assert_has(&code, "pub fn new(variables: ::defs::ItemVars) -> Self { Self { variables } }");
        assert_has(&code, "pub const DOCUMENT: &'static str = \"query Q($n: Int) { item(n: $n) { id } }\";");
        assert_has(&code, "pub const OPERATION_NAME: Option<&'static str> = Some(\"Q\");");
        assert_has(&code, "type Success = ::defs::ItemData;");
        assert_has(&code, "type Decoder = schematic_runtime::GraphQl<::defs::ItemData>;");
        assert_has(&code, "const METHOD: schematic_runtime::Method = schematic_runtime::Method::POST;");
        assert_has(&code, "const ADDRESS: schematic_runtime::Address = schematic_runtime::Address::GraphQl;");
        assert_has(&code, "schematic_runtime::graphql::request_body(Self::DOCUMENT, Some(&self.variables), Self::OPERATION_NAME)?");
        assert_has(&code, "path: String::new(),");
        assert_has(&code, "headers: vec![(\"Accept\".to_string(), \"application/json\".to_string())],");
        assert_has(&code, "const ENDPOINT_ID: &'static str = \"ReadItem\";");
    }

    #[test]
    fn an_operation_without_variables_omits_them_and_its_name_by_default() {
        let api = api_with(vec![operation("Viewer", None, |_| {})]);
        let code = generated(&api, 0);
        assert_has(&code, "#[derive(Debug, Clone, Copy, Default)] pub struct ViewerRequest;");
        assert_has(&code, "pub fn new() -> Self { Self }");
        assert_has(&code, "request_body(Self::DOCUMENT, None::<&()>, Self::OPERATION_NAME)");
        assert_has(&code, "pub const OPERATION_NAME: Option<&'static str> = None;");
    }

    #[test]
    fn the_failure_composes_graphql_errors_with_the_effective_http_table() {
        let mut api = api_with(vec![
            operation("Inherits", None, |_| {}),
            operation("Declares", None, |op| op.errors = vec![ErrorEntry::new(StatusMatch::exact(403), ResponseShape::Json(TypeRef::Named("Problem".into())))]),
            operation("OptsOut", None, |op| op.no_declared_errors = true),
        ]);
        let inherits = generated(&api, 0);
        assert_has(&inherits, "type Failure = schematic_runtime::GraphQlOperationError<::defs::ItemData, schematic_runtime::NoDeclaredError>;");
        assert!(!inherits.contains("constERRORS"), "an empty table keeps the runtime default: {inherits}");

        let declares = generated(&api, 1);
        assert_has(&declares, "type Failure = schematic_runtime::GraphQlOperationError<::defs::ItemData, ::defs::Problem>;");
        assert_has(&declares, "schematic_runtime::ErrorRoute::new(schematic_runtime::StatusMatch::Exact(403), 0)");

        api.errors = vec![ErrorEntry::new(StatusMatch::class(5), ResponseShape::Text)];
        assert_has(&generated(&api, 0), "GraphQlOperationError<::defs::ItemData, String>");
        let opted_out = generated(&api, 2);
        assert_has(&opted_out, "GraphQlOperationError<::defs::ItemData, schematic_runtime::NoDeclaredError>");
        assert!(!opted_out.contains("constERRORS"), "{opted_out}");
    }

    #[test]
    fn address_rules_join_the_api_definition_in_order_only_for_a_graphql_surface() {
        let api = api_with(vec![]);
        let rules = squash(&address_rules(&api).unwrap().to_string());
        assert_eq!(rules, squash(&quote! {
            .graphql(&[
                schematic_runtime::graphql::AddressRule::ReplaceSuffix { suffix: "/api/v3", with: "/api/graphql" },
                schematic_runtime::graphql::AddressRule::Append("/graphql")
            ])
        }.to_string()));
        let plain = make_api("Plain", "https://example.test", AuthStrategy::None, vec![]);
        assert!(address_rules(&plain).is_none());

        let definition = squash(&crate::codegen::generate_api_struct(&api).to_string());
        assert!(definition.contains(&rules), "the API definition carries the rules: {definition}");
        let plain_definition = squash(&crate::codegen::generate_api_struct(&plain).to_string());
        assert!(!plain_definition.contains(".graphql("), "{plain_definition}");
    }
}
