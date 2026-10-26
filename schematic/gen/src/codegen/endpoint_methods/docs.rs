//! Rustdoc for endpoint methods and call types.
//!
//! An endpoint method's hover text is all a caller should need: the
//! endpoint's description, its HTTP method and path, whether it is
//! deprecated or authenticates differently from its API, the required
//! arguments, and the optional setters on the call type it returns.

use proc_macro2::TokenStream;
use schematic_define::{Endpoint, RestApi};

use crate::codegen::request_structs::endpoint_docs::{DEPRECATED_LINE, auth_paragraph};

use super::names::{EndpointNames, SetterName};

/// A required argument of an endpoint method, in `{Endpoint}Request::new`
/// order.
pub(super) struct RequiredArg {
    /// The argument name.
    pub name: String,
    /// The argument type.
    pub ty: TokenStream,
    /// What the argument fills: a path parameter or the request body.
    pub role: String,
}

/// The doc lines of an endpoint method. Each line is the text of one `///`
/// line, starting with a space when it is not empty.
pub(super) fn endpoint_method_doc(
    api: &RestApi,
    endpoint: &Endpoint,
    names: &EndpointNames,
    args: &[RequiredArg],
) -> Vec<String> {
    let mut lines = Vec::new();
    let description = endpoint.description.trim();
    if !description.is_empty() {
        lines.extend(description.lines().map(|line| doc_line(line.trim_end())));
        lines.push(String::new());
    }
    lines.push(format!(" `{} {}`", endpoint.method, endpoint.path));
    lines.push(String::new());
    if endpoint.deprecated {
        lines.push(doc_line(DEPRECATED_LINE));
        lines.push(String::new());
    }
    if let Some(auth) = auth_paragraph(api, endpoint) {
        lines.push(doc_line(&auth));
        lines.push(String::new());
    }
    lines.push(format!(
        " Returns a [`{call}`]: chain optional values and per-call settings on it, \
         then `.await` it to send.",
        call = names.call_type
    ));

    if !args.is_empty() {
        lines.push(String::new());
        lines.push(" ## Arguments".to_string());
        lines.push(String::new());
        for arg in args {
            lines.push(format!(" - `{}: {}`: {}", arg.name, type_text(&arg.ty), arg.role));
        }
    }

    if !names.setters.is_empty() {
        lines.push(String::new());
        lines.push(" ## Optional values".to_string());
        lines.push(String::new());
        for setter in &names.setters {
            let deprecated = if setter.param.deprecated {
                format!(" {DEPRECATED_LINE}")
            } else {
                String::new()
            };
            lines.push(format!(
                " - [`{name}`]({call}::{name}): {summary}{deprecated}",
                name = setter.setter,
                call = names.call_type,
                summary = setter_summary(setter)
            ));
        }
    }
    lines
}

/// The doc lines of a call type.
pub(super) fn call_type_doc(api_name: &str, names: &EndpointNames, deprecated: bool) -> Vec<String> {
    let mut lines = vec![
        format!(
            " A call to the `{id}` endpoint, returned by [`{api}::{method}`].",
            id = names.endpoint_id,
            api = api_name,
            method = names.method
        ),
        String::new(),
        " Chain optional values and per-call settings, then `.await` it: the result is \
         the endpoint's data or a [`CallError`](schematic_runtime::CallError). [`full`](Self::full), chained after \
         the optional values, returns the whole response instead, and [`build`](Self::build) checks the \
         call without sending it."
            .to_string(),
    ];
    if deprecated {
        lines.push(String::new());
        lines.push(doc_line(DEPRECATED_LINE));
    }
    if names.pager.is_some() {
        lines.push(String::new());
        lines.push(
            " The list is paginated: [`pager`](Self::pager) returns a \
             [`Pager`](schematic_runtime::pager::Pager) over its pages."
                .to_string(),
        );
    }
    lines
}

/// The doc lines of a setter.
pub(super) fn setter_doc(setter: &SetterName) -> Vec<String> {
    let mut lines = vec![format!(" Sets {}", setter_summary(setter))];
    if setter.param.deprecated {
        lines.extend([String::new(), doc_line(DEPRECATED_LINE)]);
    }
    lines
}

/// What a setter sets, in one line: location, wire name, value type, and
/// the parameter's description.
fn setter_summary(setter: &SetterName) -> String {
    let ty = super::setter_type(setter);
    let mut summary = format!(
        "the `{}` {} parameter (`{}`).",
        setter.param.name,
        setter.location,
        type_text(&ty)
    );
    if let Some(description) = setter.param.description.as_deref() {
        let one_line = description.split_whitespace().collect::<Vec<_>>().join(" ");
        if !one_line.is_empty() {
            summary.push(' ');
            summary.push_str(&one_line);
        }
    }
    summary
}

fn doc_line(text: &str) -> String {
    if text.is_empty() {
        String::new()
    } else {
        format!(" {text}")
    }
}

/// `tokens` as a type a reader writes, without the spaces token printing
/// puts around `<`, `>`, and `::`.
fn type_text(tokens: &TokenStream) -> String {
    tokens
        .to_string()
        .replace(" :: ", "::")
        .replace(":: ", "::")
        .replace(" < ", "<")
        .replace("< ", "<")
        .replace(" <", "<")
        .replace(" >", ">")
        .replace(" ,", ",")
}
