//! Collection-level variables, and what one collection build collects.

use std::collections::BTreeSet;

use schematic_define::RestApi;

use crate::diagnostics::{Feature, Output, Severity, ValidationDiagnostic};
use crate::export::auth::ExportAuth;
use crate::export::examples::ExampleError;

use super::auth::auth_variables;
use super::types::PostmanVariable;

/// Appends `additions` to `target`, dropping any whose `key` is already
/// declared. Preserves order: existing entries stay first, new entries
/// follow in argument order.
pub(crate) fn merge_variables(target: &mut Vec<PostmanVariable>, additions: Vec<PostmanVariable>) {
    for var in additions {
        if !target.iter().any(|existing| existing.key == var.key) {
            target.push(var);
        }
    }
}

/// The collection variable holding a `location` parameter's value:
/// `path_owner`, `query_per_page`, `header_X-Request-Id`, `cookie_session`.
///
/// The location prefix keeps parameter variables apart from the base-URL and
/// credential variables, so a parameter named `username` never shares the
/// Basic auth's `username`. Characters outside `[A-Za-z0-9_-]` become `_`.
pub(crate) fn parameter_variable(location: &str, name: &str) -> String {
    let name: String = name
        .chars()
        .map(|c| if c.is_ascii_alphanumeric() || c == '_' || c == '-' { c } else { '_' })
        .collect();
    format!("{location}_{name}")
}

/// `{{variable}}`.
pub(crate) fn reference(variable: &str) -> String {
    format!("{{{{{variable}}}}}")
}

/// What building one collection collects besides its items: the parameter
/// variables its requests reference, which variables hold credentials or
/// secret parameters, and the examples it could not build.
#[derive(Default)]
pub(crate) struct Collected {
    /// Credential variables of every auth the requests use, in first-use
    /// order, each valued empty.
    pub credentials: Vec<PostmanVariable>,
    /// Parameter variables in first-reference order, each valued empty.
    pub parameters: Vec<PostmanVariable>,
    /// Variables an environment marks `secret`.
    pub secrets: BTreeSet<String>,
    /// Warnings for examples left out and GraphQL operations not exported.
    pub diagnostics: Vec<ValidationDiagnostic>,
}

impl Collected {
    /// Declares the credential variables `auth` references.
    pub fn auth(&mut self, auth: &ExportAuth) {
        let variables = auth_variables(auth);
        self.secrets.extend(variables.iter().map(|variable| variable.key.clone()));
        merge_variables(&mut self.credentials, variables);
    }

    /// Declares the variable of a `location` parameter named `name` and
    /// returns its key.
    pub fn parameter(&mut self, location: &str, name: &str, secret: bool) -> String {
        let key = parameter_variable(location, name);
        merge_variables(
            &mut self.parameters,
            vec![PostmanVariable {
                key: key.clone(),
                value: Some(String::new()),
                description: Some(format!("Value of the `{name}` {location} parameter")),
            }],
        );
        if secret {
            self.secrets.insert(key.clone());
        }
        key
    }

    /// Records that `role` of `endpoint` has no example: the request or
    /// response carries no body for it.
    pub fn example_failure(&mut self, api: &RestApi, endpoint: &str, role: &str, error: &ExampleError) {
        self.diagnostics.push(ValidationDiagnostic::new(
            Severity::Warning,
            &api.name,
            Some(endpoint),
            Feature::ExampleValue,
            Output::Postman,
            format!("no valid example for the {role}, so it is exported without a body: {error}"),
        ));
    }
}
