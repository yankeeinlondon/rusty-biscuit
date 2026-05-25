use super::diagnostics::OpenApiDiagnostic;
use super::mappings;
use crate::auth::AuthStrategy;

/// Maps security schemes from the OpenAPI document to an `AuthStrategy`.
///
/// Inspects `components.securitySchemes` and returns the first supported scheme.
pub(super) fn map_auth_strategy(
    doc: &openapiv3::OpenAPI,
    diagnostics: &mut Vec<OpenApiDiagnostic>,
) -> AuthStrategy {
    let components = match &doc.components {
        Some(c) => c,
        None => return AuthStrategy::None,
    };

    for (name, scheme_ref) in &components.security_schemes {
        let scheme = match scheme_ref {
            openapiv3::ReferenceOr::Item(s) => s,
            openapiv3::ReferenceOr::Reference { reference } => {
                diagnostics.push(OpenApiDiagnostic::warn(
                    format!("#/components/securitySchemes/{}", name),
                    format!("Security scheme reference '{}' not supported", reference),
                ));
                continue;
            }
        };

        let location = format!("#/components/securitySchemes/{}", name);
        match mappings::map_security_scheme(scheme, diagnostics, &location) {
            Ok(auth) => return auth,
            Err(msg) => {
                diagnostics.push(OpenApiDiagnostic::warn(location, msg));
            }
        }
    }

    AuthStrategy::None
}
