#[test]
fn current_rest_guidance_uses_supported_authoring_surface() {
    let documents = [
        ("define README", include_str!("../README.md")),
        ("area README", include_str!("../../README.md")),
        ("definitions README", include_str!("../../definitions/README.md")),
        ("generator README", include_str!("../../gen/README.md")),
        ("builders", include_str!("../../docs/definition-builders.md")),
        ("registration", include_str!("../../docs/typed-registration.md")),
        ("OpenAPI import", include_str!("../../docs/io/import-openapi.md")),
        ("OpenAPI export", include_str!("../../docs/io/export-openapi.md")),
        ("Postman export", include_str!("../../docs/io/export-postman.md")),
        ("compiling example", include_str!("../../docs/authoring-examples.md")),
        ("generated clients", include_str!("../../docs/generation/generated-clients.md")),
        ("define skill", include_str!("../../../.claude/skills/schematic-define/SKILL.md")),
        ("schematic skill", include_str!("../../../.claude/skills/schematic-define/schematic/SKILL.md")),
        ("create command", include_str!("../../../.claude/commands/create-schematic-api.md")),
    ];
    for (name, document) in documents {
        for obsolete in [
            "Endpoint {", "RestApi {", "ApiResponse::json_type(",
            "ApiRequest::json_type(", "Schema::with_path(",
            "PaginationStyle", "PaginationResponse", "pagination_exceptions", ".register::<Model>(\"Model\")",
        ] {
            assert!(!document.contains(obsolete), "{name} teaches obsolete REST authoring: {obsolete}");
        }
    }
}
