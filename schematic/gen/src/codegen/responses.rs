//! The success and failure halves of an endpoint's
//! `schematic_runtime::EndpointSpec` impl (R2, R20; Defaults D-15, D-23).
//!
//! Each half is a pure function of the definition returning the associated
//! types and the items they name (success and failure enums, stream end
//! markers), which the caller emits next to the request struct. Selection
//! and decoding live in the runtime (Default D-22): generated code carries
//! only route tables, end markers, enums, and their dispatch. Behavior for
//! users is described in `schematic/docs/responses-and-errors.md`.

use proc_macro2::{Ident, Literal, TokenStream};
use quote::{format_ident, quote};
use schematic_define::{
    ErrorTable, JsonTag, ResponseShape, RestApi, StatusMatch, StreamEnd, StreamFraming, SuccessShape,
};
use serde_json::Value;

use crate::codegen::request_structs::shared::{json_type_name, type_name_to_tokens};

/// The success half of an `EndpointSpec` impl.
pub(crate) struct SuccessCode {
    /// The `type Success` type.
    pub(crate) success: TokenStream,
    /// The `type Decoder` codec.
    pub(crate) decoder: TokenStream,
    /// The success enum and stream end markers the two types name; empty for
    /// a single shape that needs neither.
    pub(crate) items: TokenStream,
}

/// The success half for the endpoint or operation `id` declaring `shapes`.
///
/// One shape is its plain type with its own codec; several are the
/// `{id}Response` enum (one variant per shape, in declaration order, named
/// by [`success_variant_names`]) decoded by `schematic_runtime::Shapes`. A
/// stream with a terminator gets an end marker named by [`end_marker_names`].
pub(crate) fn success_code(api: &RestApi, id: &str, shapes: &[SuccessShape]) -> SuccessCode {
    if let [shape] = shapes {
        let lowered = lower(api, &shape.shape, &format_ident!("{id}End"), id);
        return SuccessCode { success: lowered.payload, decoder: lowered.codec, items: lowered.end_marker };
    }
    let enum_name = format_ident!("{id}Response");
    let names = success_variant_names(shapes);
    let mut items = TokenStream::new();
    let (mut variants, mut routes, mut arms) = (Vec::new(), Vec::new(), Vec::new());
    for (index, (shape, name)) in shapes.iter().zip(&names).enumerate() {
        let variant = variant_ident(name, index);
        let lowered = lower(api, &shape.shape, &format_ident!("{id}{variant}End"), id);
        items.extend(lowered.end_marker);
        let docs = success_variant_docs(shape);
        let payload = &lowered.payload;
        let codec = &lowered.codec;
        let decoded = quote! { <#codec as schematic_runtime::Decode>::decode(body).await };
        let (declaration, arm) = if shape.shape == ResponseShape::Empty {
            (quote! { #(#[doc = #docs])* #variant }, quote! { #decoded.map(|()| Self::#variant) })
        } else {
            (quote! { #(#[doc = #docs])* #variant(#payload) }, quote! { #decoded.map(Self::#variant) })
        };
        variants.push(declaration);
        routes.push(route(shape));
        arms.push(match_arm(index, shapes.len(), arm));
    }
    let json_only = shapes.iter().all(|shape| matches!(shape.shape, ResponseShape::Json(_)));
    let json_marker = json_only.then(|| quote! { impl schematic_runtime::JsonAlternatives for #enum_name {} });
    let doc = format!(" The success reply of `{id}`: one variant per declared shape, chosen from the reply's status, media type, and JSON tag.");
    items.extend(quote! {
        #[doc = #doc]
        #[derive(Debug)]
        pub enum #enum_name {
            #(#variants,)*
        }

        impl schematic_runtime::Alternatives for #enum_name {
            const ROUTES: &'static [schematic_runtime::Route] = &[#(#routes),*];

            async fn decode_alternative(index: usize, body: schematic_runtime::LiveBody) -> Result<Self, schematic_runtime::DecodeFailure> {
                match index {
                    #(#arms)*
                }
            }
        }

        #json_marker
    });
    SuccessCode { success: quote! { #enum_name }, decoder: quote! { schematic_runtime::Shapes<#enum_name> }, items }
}

/// The variant names of a multi-shape success enum, in shape order (D-15):
/// the author's `variant`, else the kind (`Json`, `Text`, `Binary`, `Empty`,
/// `Stream`), suffixed by the media subtype when kinds repeat
/// (`application/problem+json` gives `JsonProblem`). Names may still repeat;
/// gen validation reports that.
pub(crate) fn success_variant_names(shapes: &[SuccessShape]) -> Vec<String> {
    let bases: Vec<(String, bool)> = shapes.iter().map(|shape| match &shape.variant {
        Some(name) => (name.clone(), true),
        None => (kind_name(&shape.shape).to_string(), false),
    }).collect();
    let media: Vec<Option<String>> = shapes.iter().map(SuccessShape::media_type).collect();
    let kinds: Vec<&str> = shapes.iter().map(|shape| kind_name(&shape.shape)).collect();
    disambiguate(&bases, &media, &kinds)
}

/// The stream end markers `shapes` generate for `id`: `{id}End` for a single
/// stream, `{id}{Variant}End` for a stream variant. A byte stream and a
/// consumed connection-close end need none.
pub(crate) fn end_marker_names(id: &str, shapes: &[SuccessShape]) -> Vec<String> {
    let needs_marker = |shape: &SuccessShape| matches!(&shape.shape,
        ResponseShape::Stream { framing: StreamFraming::Sse(_) | StreamFraming::Ndjson(_), end, keep }
            if *keep || *end != StreamEnd::ConnectionClose);
    if let [shape] = shapes {
        return needs_marker(shape).then(|| format!("{id}End")).into_iter().collect();
    }
    shapes.iter().zip(success_variant_names(shapes)).filter(|(shape, _)| needs_marker(shape))
        .map(|(_, variant)| format!("{id}{variant}End")).collect()
}

/// The failure half of an `EndpointSpec` impl.
pub(crate) struct FailureCode {
    /// The `type Failure` type.
    pub(crate) failure: TokenStream,
    /// The `const ERRORS` item, or nothing when no error is declared (the
    /// trait default is then never consulted).
    pub(crate) errors: TokenStream,
    /// The `{owner}Error` enum and its `ErrorBody` impl, when generated.
    pub(crate) items: TokenStream,
    /// A registered JSON type that is the failure type itself and so needs
    /// `impl ErrorBody<ApiMarker>`. Not emitted here: one API's endpoints
    /// may share the type, so module assembly emits one impl per distinct
    /// type through [`api_error_body_impls`].
    pub(crate) error_body_type: Option<String>,
}

/// The failure half for `owner_id` (an endpoint id, or a GraphQL
/// operation id for the operation's HTTP error table) whose effective error
/// table is `table`, in the API whose marker type is `api_marker` (R20,
/// D-23).
///
/// - no entries (none declared, or opted out): `NoDeclaredError`, no
///   `ERRORS`.
/// - one distinct shape (`(shape, content_type)`): that shape's type:
///   `String`, `bytes::Bytes`, `()`, `serde_json::Value`, or the registered
///   JSON type, whose `ErrorBody` impl module assembly emits.
/// - several distinct shapes: the `{owner_id}Error` enum, one variant per
///   distinct shape in first-entry order. A lone JSON shape whose Rust type
///   the runtime already decodes as something other than JSON (`String`,
///   `bytes::Bytes`, `()`) gets the enum too, since its type cannot carry a
///   second `ErrorBody` impl.
///
/// `ERRORS` lists every entry, each routed to its distinct shape's index.
pub(crate) fn failure_code(api: &RestApi, owner_id: &str, table: &ErrorTable, api_marker: &str) -> FailureCode {
    let distinct = distinct_error_shapes(table);
    if distinct.is_empty() {
        return FailureCode { failure: quote! { schematic_runtime::NoDeclaredError }, errors: TokenStream::new(), items: TokenStream::new(), error_body_type: None };
    }
    let routes = table.entries().iter().map(|entry| {
        let alternative = distinct.iter().position(|shape| *shape == (&entry.shape, entry.content_type.as_deref())).unwrap_or_default();
        let alternative = Literal::usize_unsuffixed(alternative);
        let status = status_match(&entry.status);
        quote! { schematic_runtime::ErrorRoute::new(#status, #alternative) }
    });
    let errors = quote! { const ERRORS: &'static [schematic_runtime::ErrorRoute] = &[#(#routes),*]; };
    let single = match distinct.as_slice() {
        [(shape, _)] => single_error_type(api, shape),
        _ => None,
    };
    if let Some((failure, error_body_type)) = single {
        return FailureCode { failure, errors, items: TokenStream::new(), error_body_type };
    }

    let enum_name = format_ident!("{owner_id}Error");
    let marker = format_ident!("{api_marker}");
    let names = error_variant_names(api, &distinct);
    let (mut variants, mut arms) = (Vec::new(), Vec::new());
    for (index, ((shape, _), name)) in distinct.iter().zip(&names).enumerate() {
        let variant = variant_ident(name, index);
        let statuses: Vec<String> = table.entries().iter()
            .filter(|entry| (&entry.shape, entry.content_type.as_deref()) == distinct[index])
            .map(|entry| entry.status.to_string()).collect();
        let doc = format!(" Declared for status {}.", statuses.join(", "));
        let (declaration, decoded) = match shape {
            ResponseShape::Json(reference) => {
                let ty = type_name_to_tokens(&json_type_name(api, reference));
                (quote! { #variant(#ty) }, quote! { schematic_runtime::serde_json::from_slice::<#ty>(body).ok().map(Self::#variant) })
            }
            ResponseShape::Text => (quote! { #variant(String) }, quote! { Some(Self::#variant(String::from_utf8_lossy(body).into_owned())) }),
            ResponseShape::Binary => (quote! { #variant(bytes::Bytes) }, quote! { Some(Self::#variant(bytes::Bytes::copy_from_slice(body))) }),
            ResponseShape::Empty => (quote! { #variant }, quote! { Some(Self::#variant) }),
            _ => (quote! { #variant }, unsupported("a stream error shape")),
        };
        variants.push(quote! { #[doc = #doc] #declaration });
        let index = Literal::usize_unsuffixed(index);
        arms.push(quote! { #index => #decoded, });
    }
    let doc = format!(" The declared error body of `{owner_id}`: one variant per distinct error shape.");
    let lone_note = (distinct.len() == 1).then(|| {
        let note = " The runtime decodes this JSON shape's Rust type as something other than JSON, so it is wrapped to be decoded as JSON.";
        quote! { #[doc = ""] #[doc = #note] }
    });
    let items = quote! {
        #[doc = #doc]
        #lone_note
        #[derive(Debug)]
        pub enum #enum_name {
            #(#variants,)*
        }

        impl schematic_runtime::ErrorBody<#marker> for #enum_name {
            fn decode_body(alternative: usize, body: &[u8]) -> Option<Self> {
                match alternative {
                    #(#arms)*
                    _ => None,
                }
            }
        }
    };
    FailureCode { failure: quote! { #enum_name }, errors, items, error_body_type: None }
}

/// The distinct shapes of `table`, in first-entry order. Two entries share a
/// shape when both their shape and declared content type are equal.
pub(crate) fn distinct_error_shapes(table: &ErrorTable) -> Vec<(&ResponseShape, Option<&str>)> {
    let mut shapes = Vec::new();
    for entry in table.entries() {
        let shape = (&entry.shape, entry.content_type.as_deref());
        if !shapes.contains(&shape) {
            shapes.push(shape);
        }
    }
    shapes
}

/// Whether [`failure_code`] generates the `{owner}Error` enum for `table`.
pub(crate) fn failure_enum_needed(api: &RestApi, table: &ErrorTable) -> bool {
    match distinct_error_shapes(table).as_slice() {
        [] => false,
        [(shape, _)] => single_error_type(api, shape).is_none(),
        _ => true,
    }
}

/// One `impl ErrorBody<api_marker>` decoding JSON per distinct type of
/// `types`, in first-seen order.
pub(crate) fn error_body_impls(api_marker: &str, types: &[String]) -> TokenStream {
    let marker = format_ident!("{api_marker}");
    let mut seen: Vec<String> = Vec::new();
    let mut impls = TokenStream::new();
    for type_name in types {
        let key: String = type_name.split_whitespace().collect();
        if seen.contains(&key) {
            continue;
        }
        seen.push(key);
        let ty = type_name_to_tokens(type_name);
        impls.extend(quote! {
            impl schematic_runtime::ErrorBody<#marker> for #ty {
                fn decode_body(_: usize, body: &[u8]) -> Option<Self> {
                    schematic_runtime::serde_json::from_slice(body).ok()
                }
            }
        });
    }
    impls
}

/// The `ErrorBody` impls of every registered JSON type that is the whole
/// failure type of one of `api`'s endpoints or GraphQL operations,
/// deduplicated: the API module emits these once.
pub(crate) fn api_error_body_impls(api: &RestApi, api_marker: &str) -> TokenStream {
    let endpoints = api.endpoints.iter().map(|endpoint| (endpoint.id.as_str(), api.effective_errors(endpoint)));
    let operations = api.graphql.iter().flat_map(|surface| &surface.operations)
        .map(|operation| (operation.id.as_str(), api.effective_operation_errors(operation)));
    let types: Vec<String> = endpoints.chain(operations)
        .filter_map(|(id, table)| failure_code(api, id, &table, api_marker).error_body_type)
        .collect();
    error_body_impls(api_marker, &types)
}

/// JSON spellings the runtime implements `ErrorBody` for, for every API,
/// by a decode other than JSON.
const RUNTIME_NON_JSON_BODIES: &[&str] = &["String", "std::string::String", "alloc::string::String", "bytes::Bytes", "()",
    "schematic_runtime::NoDeclaredError"];

/// JSON spellings the runtime decodes as JSON for every API.
const RUNTIME_JSON_BODIES: &[&str] = &["serde_json::Value", "schematic_runtime::serde_json::Value"];

/// The failure type of a table with the one distinct `shape`, and the JSON
/// type that needs a generated `ErrorBody` impl; `None` when the shape needs
/// the failure enum.
fn single_error_type(api: &RestApi, shape: &ResponseShape) -> Option<(TokenStream, Option<String>)> {
    match shape {
        ResponseShape::Text => Some((quote! { String }, None)),
        ResponseShape::Binary => Some((quote! { bytes::Bytes }, None)),
        ResponseShape::Empty => Some((quote! { () }, None)),
        ResponseShape::Json(reference) => {
            let spelling = json_type_name(api, reference);
            let key: String = spelling.split_whitespace().collect();
            let key = key.trim_start_matches("::");
            if RUNTIME_NON_JSON_BODIES.contains(&key) {
                return None;
            }
            let needed = (!RUNTIME_JSON_BODIES.contains(&key)).then(|| spelling.clone());
            Some((type_name_to_tokens(&spelling), needed))
        }
        _ => Some((unsupported("a stream error shape"), None)),
    }
}

/// Variant names of a failure enum: the JSON type's last path segment
/// (generics dropped), else the kind; repeats take the media subtype, then
/// a 1-based position, so a failure enum always generates.
fn error_variant_names(api: &RestApi, distinct: &[(&ResponseShape, Option<&str>)]) -> Vec<String> {
    let bases: Vec<(String, bool)> = distinct.iter().map(|(shape, _)| {
        let name = match shape {
            ResponseShape::Json(reference) => syn::parse_str::<syn::TypePath>(&json_type_name(api, reference)).ok()
                .and_then(|path| path.path.segments.last().map(|segment| pascal_case(&segment.ident.to_string())))
                .unwrap_or_else(|| "Json".to_string()),
            other => kind_name(other).to_string(),
        };
        (name, false)
    }).collect();
    let media: Vec<Option<String>> = distinct.iter().map(|(shape, content_type)| {
        let mut probe = SuccessShape::new((*shape).clone());
        probe.content_type = content_type.map(str::to_owned);
        probe.media_type()
    }).collect();
    let kinds: Vec<&str> = distinct.iter().map(|(shape, _)| kind_name(shape)).collect();
    let mut names = disambiguate(&bases, &media, &kinds);
    for index in 0..names.len() {
        if names[..index].contains(&names[index]) {
            names[index] = format!("{}{}", names[index], index + 1);
        }
    }
    names
}

/// Suffixes each derived (not authored) name that another shape shares with
/// its media subtype, unless the subtype only repeats the shape's `kinds`
/// name.
fn disambiguate(bases: &[(String, bool)], media: &[Option<String>], kinds: &[&str]) -> Vec<String> {
    bases.iter().zip(media).zip(kinds).map(|(((name, authored), media), kind)| {
        if *authored || bases.iter().filter(|(other, _)| other == name).count() < 2 {
            return name.clone();
        }
        format!("{name}{}", media_suffix(media.as_deref(), kind))
    }).collect()
}

/// The PascalCase media subtype without a structured suffix (`+json`) or an
/// `x-` prefix; empty when it only repeats `kind` (`application/json` for
/// `Json`) or there is no media type.
fn media_suffix(media: Option<&str>, kind: &str) -> String {
    let Some(media) = media else { return String::new() };
    let subtype = media.split_once('/').map_or(media, |(_, subtype)| subtype);
    let subtype = subtype.split('+').next().unwrap_or(subtype);
    let suffix = pascal_case(subtype.strip_prefix("x-").unwrap_or(subtype));
    if suffix.eq_ignore_ascii_case(kind) { String::new() } else { suffix }
}

fn pascal_case(text: &str) -> String {
    text.trim_start_matches("r#").split(|c: char| !c.is_ascii_alphanumeric()).filter(|part| !part.is_empty()).map(|part| {
        let mut chars = part.chars();
        chars.next().map(|first| first.to_ascii_uppercase().to_string() + chars.as_str()).unwrap_or_default()
    }).collect()
}

fn kind_name(shape: &ResponseShape) -> &'static str {
    match shape {
        ResponseShape::Json(_) => "Json",
        ResponseShape::Text => "Text",
        ResponseShape::Binary => "Binary",
        ResponseShape::Empty => "Empty",
        _ => "Stream",
    }
}

/// `name` as an identifier. Define validation admits only UpperCamelCase
/// names, but stored fields are public; an invalid one becomes `Shape{n}`
/// rather than aborting generation.
fn variant_ident(name: &str, index: usize) -> Ident {
    match syn::parse_str::<Ident>(name) {
        Ok(ident) => ident,
        Err(_) => format_ident!("Shape{}", index + 1),
    }
}

/// The last arm matches everything: the runtime passes only valid indices.
fn match_arm(index: usize, count: usize, body: TokenStream) -> TokenStream {
    if index + 1 == count {
        quote! { _ => #body, }
    } else {
        let index = Literal::usize_unsuffixed(index);
        quote! { #index => #body, }
    }
}

struct Lowered {
    payload: TokenStream,
    codec: TokenStream,
    end_marker: TokenStream,
}

fn lower(api: &RestApi, shape: &ResponseShape, end_name: &Ident, owner: &str) -> Lowered {
    let plain = |payload, codec| Lowered { payload, codec, end_marker: TokenStream::new() };
    match shape {
        ResponseShape::Json(reference) => {
            let ty = type_name_to_tokens(&json_type_name(api, reference));
            plain(ty.clone(), quote! { schematic_runtime::Json<#ty> })
        }
        ResponseShape::Text => plain(quote! { String }, quote! { schematic_runtime::Text }),
        ResponseShape::Binary => plain(quote! { bytes::Bytes }, quote! { schematic_runtime::Binary }),
        ResponseShape::Empty => plain(quote! { () }, quote! { schematic_runtime::Empty }),
        ResponseShape::Stream { framing, end, keep } => {
            let (item, codec) = match framing {
                StreamFraming::Sse(reference) => (reference, quote! { schematic_runtime::Sse }),
                StreamFraming::Ndjson(reference) => (reference, quote! { schematic_runtime::Ndjson }),
                StreamFraming::Bytes => return plain(quote! { schematic_runtime::ByteStream }, quote! { schematic_runtime::ByteStream }),
                _ => return plain(unsupported("this stream framing"), unsupported("this stream framing")),
            };
            let item = type_name_to_tokens(&json_type_name(api, item));
            let (marker, end_marker) = stream_end_marker(end_name, owner, end, *keep);
            Lowered { payload: quote! { schematic_runtime::ItemStream<#item> }, codec: quote! { #codec<#item, #marker> }, end_marker }
        }
        _ => plain(unsupported("this response shape"), unsupported("this response shape")),
    }
}

/// The end-marker type of a stream, and the marker item when one is
/// generated (`UntilClose` covers a consumed connection close).
fn stream_end_marker(name: &Ident, owner: &str, end: &StreamEnd, keep: bool) -> (TokenStream, TokenStream) {
    let condition = match end {
        StreamEnd::ConnectionClose if !keep => return (quote! { schematic_runtime::UntilClose }, TokenStream::new()),
        StreamEnd::ConnectionClose => quote! { schematic_runtime::EndCondition::ConnectionClose },
        StreamEnd::DataLineEquals(data) => quote! { schematic_runtime::EndCondition::DataLineEquals(#data) },
        StreamEnd::EventNamed(event) => quote! { schematic_runtime::EndCondition::EventNamed(#event) },
        StreamEnd::FieldEquals { field, value } => {
            let path = field.split('.');
            let value = scalar(value);
            quote! { schematic_runtime::EndCondition::FieldEquals { path: &[#(#path),*], value: #value } }
        }
        _ => unsupported("this stream end"),
    };
    let keep = keep.then(|| quote! { const KEEP: bool = true; });
    let doc = format!(" The end condition of the `{owner}` stream.");
    let item = quote! {
        #[doc = #doc]
        #[derive(Debug, Clone, Copy)]
        pub struct #name;

        impl schematic_runtime::StreamEnd for #name {
            const CONDITION: schematic_runtime::EndCondition = #condition;
            #keep
        }
    };
    (quote! { #name }, item)
}

fn scalar(value: &Value) -> TokenStream {
    match value {
        Value::String(text) => quote! { schematic_runtime::Scalar::String(#text) },
        Value::Bool(flag) => quote! { schematic_runtime::Scalar::Bool(#flag) },
        Value::Number(number) => match (number.as_i64(), number.as_u64()) {
            (Some(signed), _) => {
                let magnitude = Literal::u64_unsuffixed(signed.unsigned_abs());
                let sign = (signed < 0).then(|| quote! { - });
                quote! { schematic_runtime::Scalar::Integer(#sign #magnitude) }
            }
            (None, Some(unsigned)) => {
                let unsigned = Literal::u64_unsuffixed(unsigned);
                quote! { schematic_runtime::Scalar::Unsigned(#unsigned) }
            }
            _ => {
                let float = Literal::f64_unsuffixed(number.as_f64().unwrap_or_default());
                quote! { schematic_runtime::Scalar::Float(#float) }
            }
        },
        _ => unsupported("a non-scalar JSON value"),
    }
}

fn route(shape: &SuccessShape) -> TokenStream {
    let kind = match &shape.shape {
        ResponseShape::Json(_) => quote! { Json },
        ResponseShape::Text => quote! { Text },
        ResponseShape::Binary => quote! { Binary },
        ResponseShape::Empty => quote! { Empty },
        _ => quote! { Stream },
    };
    let mut route = quote! { schematic_runtime::Route::new(schematic_runtime::ShapeKind::#kind) };
    if let Some(media) = shape.media_type() {
        route.extend(quote! { .media(#media) });
    }
    if !shape.statuses.is_empty() {
        let statuses = shape.statuses.iter().map(|&status| Literal::u16_unsuffixed(status));
        route.extend(quote! { .statuses(&[#(#statuses),*]) });
    }
    if let Some(tag) = &shape.json_tag {
        let tag = match tag {
            JsonTag::Field { name, value } => {
                let value = scalar(value);
                quote! { schematic_runtime::JsonTag::Field { name: #name, value: #value } }
            }
            JsonTag::Array => quote! { schematic_runtime::JsonTag::Array },
            JsonTag::Object => quote! { schematic_runtime::JsonTag::Object },
            _ => unsupported("this JSON tag"),
        };
        route.extend(quote! { .tag(#tag) });
    }
    if shape.fallback {
        route.extend(quote! { .fallback() });
    }
    route
}

/// The shape's note (R13a), then what selects it.
fn success_variant_docs(shape: &SuccessShape) -> Vec<String> {
    let mut lines: Vec<String> = Vec::new();
    if let Some(note) = &shape.note {
        lines.extend(note.lines().map(|line| format!(" {line}")));
        lines.push(String::new());
    }
    if shape.shape == ResponseShape::Empty {
        lines.push(" Chosen by a 204 reply.".to_string());
        return lines;
    }
    let mut selection = match shape.media_type() {
        Some(media) => format!(" Chosen by media type `{media}`"),
        None => " Chosen when no other shape matches the reply's media type".to_string(),
    };
    if !shape.statuses.is_empty() {
        let statuses: Vec<String> = shape.statuses.iter().map(u16::to_string).collect();
        selection.push_str(&format!(" on status {}", statuses.join(", ")));
    }
    match &shape.json_tag {
        Some(JsonTag::Field { name, value }) => selection.push_str(&format!(" when the body's `{name}` field is `{value}`")),
        Some(JsonTag::Array) => selection.push_str(" when the body is a JSON array"),
        Some(JsonTag::Object) => selection.push_str(" when the body is a JSON object"),
        _ => {}
    }
    selection.push('.');
    lines.push(selection);
    lines
}

fn status_match(status: &StatusMatch) -> TokenStream {
    match *status {
        StatusMatch::Exact(code) => {
            let code = Literal::u16_unsuffixed(code);
            quote! { schematic_runtime::StatusMatch::Exact(#code) }
        }
        StatusMatch::Range { lo, hi } => {
            let (lo, hi) = (Literal::u16_unsuffixed(lo), Literal::u16_unsuffixed(hi));
            quote! { schematic_runtime::StatusMatch::Range { lo: #lo, hi: #hi } }
        }
        _ => unsupported("this error status match"),
    }
}

/// A compile error in the generated crate, at the item that needs a form
/// added to a `#[non_exhaustive]` definition enum after this generator.
fn unsupported(what: &str) -> TokenStream {
    let message = format!("schematic-gen cannot emit {what}");
    quote! { ::core::compile_error!(#message) }
}

#[cfg(test)]
mod tests {
    use schematic_define::{ApiResponse, AuthStrategy, Endpoint, ErrorEntry, PrimitiveType, RestMethod, SchemaComponent, TypeRef};
    use serde_json::json;

    use super::*;
    use crate::codegen::generate_request_struct_with_options;
    use crate::test_utils::{format_generated_code, make_api, make_endpoint, validate_generated_code};

    /// An API whose components `types` are spelled `::defs::{name}`.
    fn api_with(endpoints: Vec<Endpoint>, types: &[&str]) -> RestApi {
        let mut api = make_api("Fixture", "https://example.test", AuthStrategy::None, vec![]);
        api.endpoints = endpoints;
        api.schemas = types.iter().map(|name| SchemaComponent {
            name: (*name).to_string(),
            rust_path: Some(format!("::defs::{name}")),
            schema: json!({"type": "object"}),
        }).collect();
        api
    }

    fn endpoint(id: &str, shapes: Vec<SuccessShape>) -> Endpoint {
        let mut endpoint = make_endpoint(id, RestMethod::Post, "/items", None);
        endpoint.response = ApiResponse::json_type("Item");
        endpoint.shapes_preview = shapes;
        endpoint
    }

    fn json(name: &str) -> ResponseShape {
        ResponseShape::Json(TypeRef::Named(name.into()))
    }

    fn shape(kind: ResponseShape, edit: impl FnOnce(&mut SuccessShape)) -> SuccessShape {
        let mut shape = SuccessShape::new(kind);
        edit(&mut shape);
        shape
    }

    fn plain(kind: ResponseShape) -> SuccessShape {
        SuccessShape::new(kind)
    }

    fn stream(framing: StreamFraming, end: StreamEnd, keep: bool) -> ResponseShape {
        ResponseShape::Stream { framing, end, keep }
    }

    /// `code` without whitespace or a trailing comma before `)` or `]`.
    fn squash(code: &str) -> String {
        let flat: String = code.split_whitespace().collect();
        flat.replace(",)", ")").replace(",]", "]")
    }

    /// The request struct and impls of `id`, checked to parse, with all
    /// whitespace removed.
    fn generated(api: &RestApi, id: &str) -> String {
        let endpoint = api.endpoints.iter().find(|endpoint| endpoint.id == id).expect("fixture endpoint");
        let tokens = generate_request_struct_with_options(api, endpoint, "Request", None, "FixtureApi");
        validate_generated_code(&tokens).expect("generated code parses");
        squash(&tokens.to_string())
    }

    fn assert_has(code: &str, expected: &str) {
        assert!(code.contains(&squash(expected)), "missing `{expected}` in:\n{code}");
    }

    #[test]
    fn a_single_plain_shape_keeps_its_type_and_codec_and_declares_no_error() {
        let cases = [
            (ApiResponse::json_type("Item"), "::defs::Item", "schematic_runtime::Json<::defs::Item>"),
            (ApiResponse::Text, "String", "schematic_runtime::Text"),
            (ApiResponse::Binary, "bytes::Bytes", "schematic_runtime::Binary"),
            (ApiResponse::Empty, "()", "schematic_runtime::Empty"),
        ];
        for (response, success, decoder) in cases {
            let mut single = endpoint("Get", vec![]);
            single.response = response;
            let code = generated(&api_with(vec![single], &["Item"]), "Get");
            assert_has(&code, &format!("type Success = {success}; type Failure = schematic_runtime::NoDeclaredError; type Decoder = {decoder};"));
            assert!(!code.contains("ERRORS") && !code.contains("enum") && !code.contains("StreamEnd"), "{code}");
        }
    }

    #[test]
    fn a_single_sse_stream_consuming_its_data_line_terminator_names_a_generated_end_marker() {
        let shapes = vec![plain(stream(StreamFraming::Sse(TypeRef::Named("Chunk".into())), StreamEnd::DataLineEquals("[DONE]".into()), false))];
        let code = generated(&api_with(vec![endpoint("StreamChat", shapes)], &["Chunk"]), "StreamChat");

        assert_has(&code, "type Success = schematic_runtime::ItemStream<::defs::Chunk>;");
        assert_has(&code, "type Decoder = schematic_runtime::Sse<::defs::Chunk, StreamChatEnd>;");
        assert_has(&code, "pub struct StreamChatEnd;");
        assert_has(&code, r#"impl schematic_runtime::StreamEnd for StreamChatEnd {
            const CONDITION: schematic_runtime::EndCondition = schematic_runtime::EndCondition::DataLineEquals("[DONE]");
        }"#);
        assert!(!code.contains("KEEP"), "a consumed terminator keeps the trait default: {code}");
    }

    #[test]
    fn a_kept_ndjson_field_terminator_splits_its_dotted_path() {
        let end = StreamEnd::FieldEquals { field: "stats.done".into(), value: json!(true) };
        let shapes = vec![plain(stream(StreamFraming::Ndjson(TypeRef::Named("Chunk".into())), end, true))];
        let code = generated(&api_with(vec![endpoint("Generate", shapes)], &["Chunk"]), "Generate");

        assert_has(&code, "type Decoder = schematic_runtime::Ndjson<::defs::Chunk, GenerateEnd>;");
        assert_has(&code, r#"const CONDITION: schematic_runtime::EndCondition = schematic_runtime::EndCondition::FieldEquals {
            path: &["stats", "done"], value: schematic_runtime::Scalar::Bool(true) };
            const KEEP: bool = true;"#);
    }

    #[test]
    fn a_byte_stream_and_a_connection_close_stream_need_no_end_marker() {
        let bytes = vec![plain(stream(StreamFraming::Bytes, StreamEnd::ConnectionClose, false))];
        let closing = vec![plain(stream(StreamFraming::Sse(TypeRef::Named("Chunk".into())), StreamEnd::ConnectionClose, false))];
        let api = api_with(vec![endpoint("Speak", bytes), endpoint("Watch", closing)], &["Chunk"]);

        let code = generated(&api, "Speak");
        assert_has(&code, "type Success = schematic_runtime::ByteStream;");
        assert_has(&code, "type Decoder = schematic_runtime::ByteStream;");
        let code = generated(&api, "Watch");
        assert_has(&code, "type Decoder = schematic_runtime::Sse<::defs::Chunk, schematic_runtime::UntilClose>;");
        for id in ["Speak", "Watch"] {
            assert!(end_marker_names(id, &api.endpoints.iter().find(|e| e.id == id).unwrap().success_shapes()).is_empty());
            assert!(!generated(&api, id).contains("pub struct"), "{id}");
        }
    }

    #[test]
    fn scalars_keep_their_json_kind() {
        let cases = [
            (json!("done"), r#"schematic_runtime::Scalar::String("done")"#),
            (json!(false), "schematic_runtime::Scalar::Bool(false)"),
            (json!(7), "schematic_runtime::Scalar::Integer(7)"),
            (json!(-3), "schematic_runtime::Scalar::Integer(-3)"),
            (json!(u64::MAX), "schematic_runtime::Scalar::Unsigned(18446744073709551615)"),
            (json!(1.5), "schematic_runtime::Scalar::Float(1.5)"),
        ];
        for (value, expected) in cases {
            assert_eq!(squash(&scalar(&value).to_string()), squash(expected), "{value}");
        }
    }

    #[test]
    fn json_plus_sse_is_an_enum_whose_variant_docs_carry_the_notes() {
        let shapes = vec![
            shape(json("Message"), |shape| shape.note = Some("returned when `stream` is false or omitted".into())),
            shape(stream(StreamFraming::Sse(TypeRef::Named("Event".into())), StreamEnd::EventNamed("message_stop".into()), false),
                |shape| shape.note = Some("returned when `stream` is true".into())),
        ];
        let api = api_with(vec![endpoint("CreateMessage", shapes)], &["Message", "Event"]);
        let endpoint = &api.endpoints[0];
        let tokens = generate_request_struct_with_options(&api, endpoint, "Request", None, "FixtureApi");
        let pretty = format_generated_code(&tokens).expect("formats");
        let code = generated(&api, "CreateMessage");

        assert!(pretty.contains("/// returned when `stream` is false or omitted\n    ///\n    /// Chosen by media type `application/json`."), "{pretty}");
        assert!(pretty.contains("/// returned when `stream` is true\n"), "{pretty}");
        assert_has(&code, "# [derive (Debug)] pub enum CreateMessageResponse {");
        assert_has(&code, "Json(::defs::Message),");
        assert_has(&code, "Stream(schematic_runtime::ItemStream<::defs::Event>),");
        assert_has(&code, r#"const ROUTES: &'static [schematic_runtime::Route] = &[
            schematic_runtime::Route::new(schematic_runtime::ShapeKind::Json).media("application/json"),
            schematic_runtime::Route::new(schematic_runtime::ShapeKind::Stream).media("text/event-stream"),
        ];"#);
        assert_has(&code, "0 => <schematic_runtime::Json<::defs::Message> as schematic_runtime::Decode>::decode(body).await.map(Self::Json),");
        assert_has(&code, "_ => <schematic_runtime::Sse<::defs::Event, CreateMessageStreamEnd> as schematic_runtime::Decode>::decode(body).await.map(Self::Stream),");
        assert_has(&code, r#"EndCondition::EventNamed("message_stop")"#);
        assert_has(&code, "type Success = CreateMessageResponse;");
        assert_has(&code, "type Decoder = schematic_runtime::Shapes<CreateMessageResponse>;");
        assert!(!code.contains("JsonAlternatives"), "a stream alternative takes no typed mutator: {code}");
    }

    #[test]
    fn json_shapes_told_apart_by_status_are_all_json_alternatives() {
        let shapes = vec![
            shape(json("Item"), |shape| { shape.statuses = vec![200]; shape.variant = Some("Record".into()); }),
            shape(json("Job"), |shape| { shape.statuses = vec![202]; shape.variant = Some("Accepted".into()); }),
        ];
        let code = generated(&api_with(vec![endpoint("Create", shapes)], &["Item", "Job"]), "Create");

        assert_has(&code, "Record(::defs::Item), ");
        assert_has(&code, "Accepted(::defs::Job), ");
        assert_has(&code, r#"Route::new(schematic_runtime::ShapeKind::Json).media("application/json").statuses(&[200])"#);
        assert_has(&code, r#"Route::new(schematic_runtime::ShapeKind::Json).media("application/json").statuses(&[202])"#);
        assert_has(&code, "impl schematic_runtime::JsonAlternatives for CreateResponse {}");
    }

    #[test]
    fn tagged_json_groups_route_by_discriminator_or_json_kind() {
        let tagged = |value: &str, name: &str| shape(json(name), |shape| {
            shape.json_tag = Some(JsonTag::Field { name: "type".into(), value: json!(value) });
            shape.variant = Some(name.into());
        });
        let kind = |tag: JsonTag, name: &str| shape(json(name), |shape| { shape.json_tag = Some(tag); shape.variant = Some(name.into()); });
        let api = api_with(vec![
            endpoint("GetSource", vec![tagged("commit_file", "File"), tagged("commit_directory", "Directory")]),
            endpoint("GetContents", vec![kind(JsonTag::Array, "Listing"), kind(JsonTag::Object, "Entry")]),
        ], &["File", "Directory", "Listing", "Entry"]);

        let code = generated(&api, "GetSource");
        assert_has(&code, r#".media("application/json").tag(schematic_runtime::JsonTag::Field { name: "type", value: schematic_runtime::Scalar::String("commit_file") })"#);
        assert_has(&code, r#".tag(schematic_runtime::JsonTag::Field { name: "type", value: schematic_runtime::Scalar::String("commit_directory") })"#);
        assert_has(&code, "impl schematic_runtime::JsonAlternatives for GetSourceResponse {}");
        let code = generated(&api, "GetContents");
        assert_has(&code, ".tag(schematic_runtime::JsonTag::Array)");
        assert_has(&code, ".tag(schematic_runtime::JsonTag::Object)");
    }

    #[test]
    fn a_binary_fallback_has_no_media_type_and_disables_json_alternatives() {
        let shapes = vec![
            shape(json("Directory"), |shape| shape.variant = Some("Directory".into())),
            shape(ResponseShape::Binary, |shape| { shape.fallback = true; shape.variant = Some("File".into()); }),
        ];
        let code = generated(&api_with(vec![endpoint("GetSource", shapes)], &["Directory"]), "GetSource");

        assert_has(&code, "Directory(::defs::Directory), ");
        assert_has(&code, "File(bytes::Bytes), ");
        assert_has(&code, "schematic_runtime::Route::new(schematic_runtime::ShapeKind::Binary).fallback()");
        assert_has(&code, "_ => <schematic_runtime::Binary as schematic_runtime::Decode>::decode(body).await.map(Self::File),");
        assert!(!code.contains("JsonAlternatives"), "{code}");
    }

    #[test]
    fn json_plus_empty_has_a_unit_variant_chosen_by_204() {
        let shapes = vec![plain(json("Item")), plain(ResponseShape::Empty)];
        let api = api_with(vec![endpoint("Update", shapes)], &["Item"]);
        let pretty = format_generated_code(&generate_request_struct_with_options(&api, &api.endpoints[0], "Request", None, "FixtureApi")).unwrap();
        let code = generated(&api, "Update");

        assert_has(&code, "Json(::defs::Item), ");
        assert!(pretty.contains("/// Chosen by a 204 reply.\n    Empty,"), "{pretty}");
        assert_has(&code, "schematic_runtime::Route::new(schematic_runtime::ShapeKind::Empty)];");
        assert_has(&code, "_ => <schematic_runtime::Empty as schematic_runtime::Decode>::decode(body).await.map(|()| Self::Empty),");
        assert!(!code.contains("JsonAlternatives"), "{code}");
    }

    #[test]
    fn derived_variant_names_take_the_media_subtype_only_when_kinds_repeat() {
        let typed = |content_type: &str| shape(json("Item"), |shape| shape.content_type = Some(content_type.into()));
        let names = success_variant_names(&[plain(json("Item")), typed("application/problem+json"), plain(ResponseShape::Text)]);
        assert_eq!(names, ["Json", "JsonProblem", "Text"]);
        let names = success_variant_names(&[
            shape(ResponseShape::Text, |shape| shape.content_type = Some("text/html".into())),
            plain(ResponseShape::Text),
            plain(stream(StreamFraming::Ndjson(TypeRef::Named("Item".into())), StreamEnd::ConnectionClose, false)),
        ]);
        assert_eq!(names, ["TextHtml", "TextPlain", "Stream"]);
        let named = shape(json("Item"), |shape| { shape.statuses = vec![202]; shape.variant = Some("Json".into()); });
        assert_eq!(success_variant_names(&[shape(json("Item"), |shape| shape.statuses = vec![200]), named]), ["Json", "Json"],
            "an unresolved repeat is left for validation");
    }

    fn error(status: StatusMatch, kind: ResponseShape) -> ErrorEntry {
        ErrorEntry::new(status, kind)
    }

    #[test]
    fn inherited_api_errors_with_one_json_shape_fail_as_that_type() {
        let mut api = api_with(vec![endpoint("Get", vec![])], &["Item", "Problem"]);
        api.errors = vec![error(StatusMatch::Exact(404), json("Problem")), error(StatusMatch::Range { lo: 500, hi: 599 }, json("Problem"))];
        let code = generated(&api, "Get");

        assert_has(&code, "type Failure = ::defs::Problem;");
        assert_has(&code, "const ERRORS: &'static [schematic_runtime::ErrorRoute] = &[
            schematic_runtime::ErrorRoute::new(schematic_runtime::StatusMatch::Exact(404), 0),
            schematic_runtime::ErrorRoute::new(schematic_runtime::StatusMatch::Range { lo: 500, hi: 599 }, 0),
        ];");
        assert!(!code.contains("ErrorBody"), "the impl is emitted once per API by module assembly: {code}");
        let failure = failure_code(&api, "Get", &api.effective_errors(&api.endpoints[0]), "FixtureApi");
        assert_eq!(failure.error_body_type.as_deref(), Some("::defs::Problem"));
    }

    #[test]
    fn an_endpoint_overlay_with_a_second_shape_generates_the_failure_enum() {
        let mut overlaid = endpoint("Get", vec![]);
        overlaid.errors = vec![error(StatusMatch::Range { lo: 500, hi: 599 }, ResponseShape::Text), error(StatusMatch::Exact(429), ResponseShape::Empty)];
        let mut api = api_with(vec![overlaid], &["Item", "Problem"]);
        api.errors = vec![error(StatusMatch::Exact(404), json("Problem")), error(StatusMatch::Range { lo: 400, hi: 499 }, json("Problem"))];
        let code = generated(&api, "Get");

        assert_has(&code, "type Failure = GetError;");
        assert_has(&code, "pub enum GetError { #[doc = \" Declared for status 404, 400-499.\"] Problem(::defs::Problem),");
        assert_has(&code, "Text(String), ");
        assert_has(&code, "Empty, }");
        assert_has(&code, "&[schematic_runtime::ErrorRoute::new(schematic_runtime::StatusMatch::Exact(404), 0),
            schematic_runtime::ErrorRoute::new(schematic_runtime::StatusMatch::Range { lo: 400, hi: 499 }, 0),
            schematic_runtime::ErrorRoute::new(schematic_runtime::StatusMatch::Range { lo: 500, hi: 599 }, 1),
            schematic_runtime::ErrorRoute::new(schematic_runtime::StatusMatch::Exact(429), 2),]");
        assert_has(&code, "impl schematic_runtime::ErrorBody<FixtureApi> for GetError {
            fn decode_body(alternative: usize, body: &[u8]) -> Option<Self> {
                match alternative {
                    0 => schematic_runtime::serde_json::from_slice::<::defs::Problem>(body).ok().map(Self::Problem),
                    1 => Some(Self::Text(String::from_utf8_lossy(body).into_owned())),
                    2 => Some(Self::Empty),
                    _ => None,");
        assert_eq!(failure_code(&api, "Get", &api.effective_errors(&api.endpoints[0]), "FixtureApi").error_body_type, None);
    }

    #[test]
    fn an_opted_out_endpoint_declares_no_error_and_no_table() {
        let mut opted_out = endpoint("Get", vec![]);
        opted_out.no_declared_errors = true;
        let mut api = api_with(vec![opted_out], &["Item", "Problem"]);
        api.errors = vec![error(StatusMatch::Exact(404), json("Problem"))];
        let code = generated(&api, "Get");

        assert_has(&code, "type Failure = schematic_runtime::NoDeclaredError;");
        assert!(!code.contains("ERRORS"), "{code}");
    }

    #[test]
    fn a_lone_error_shape_uses_the_runtime_body_unless_the_runtime_decodes_its_type_otherwise() {
        let primitive = |kind| ResponseShape::Json(TypeRef::Primitive(kind));
        let cases = [
            (ResponseShape::Text, Some("String")),
            (ResponseShape::Binary, Some("bytes::Bytes")),
            (ResponseShape::Empty, Some("()")),
            (primitive(PrimitiveType::Json), Some("::serde_json::Value")),
            (primitive(PrimitiveType::String), None),
        ];
        for (kind, failure) in cases {
            let mut api = api_with(vec![endpoint("Get", vec![])], &["Item"]);
            api.errors = vec![error(StatusMatch::Exact(400), kind.clone())];
            let code = generated(&api, "Get");
            let generated = failure_code(&api, "Get", &api.effective_errors(&api.endpoints[0]), "FixtureApi");
            assert_eq!(generated.error_body_type, None, "{kind:?} needs no generated impl");
            assert_eq!(failure_enum_needed(&api, &api.effective_errors(&api.endpoints[0])), failure.is_none());
            match failure {
                Some(failure) => assert_has(&code, &format!("type Failure = {failure};")),
                None => {
                    assert_has(&code, "type Failure = GetError;");
                    assert_has(&code, "pub enum GetError {");
                    assert_has(&code, "String(::std::string::String), }");
                    assert!(code.contains(&squash("so it is wrapped to be decoded as JSON")), "{code}");
                    assert_has(&code, "0 => schematic_runtime::serde_json::from_slice::<::std::string::String>(body).ok().map(Self::String),");
                }
            }
        }
    }

    #[test]
    fn repeated_error_variant_names_take_the_media_subtype_then_a_position() {
        let mut api = api_with(vec![], &["Problem"]);
        api.schemas.push(SchemaComponent { name: "OtherProblem".into(), rust_path: Some("::other::Problem".into()), schema: json!({"type": "object"}) });
        let mut html = error(StatusMatch::Exact(502), ResponseShape::Text);
        html.content_type = Some("text/html".into());
        let table = ErrorTable::resolve(&[
            error(StatusMatch::Exact(400), json("Problem")),
            error(StatusMatch::Exact(404), json("OtherProblem")),
            error(StatusMatch::Exact(500), ResponseShape::Text),
            html,
        ], &[], false);
        assert_eq!(error_variant_names(&api, &distinct_error_shapes(&table)), ["Problem", "Problem2", "TextPlain", "TextHtml"]);
    }

    #[test]
    fn error_body_impls_are_one_per_distinct_type() {
        let impls = squash(&error_body_impls("FixtureApi", &["::defs::Problem".into(), ":: defs :: Problem".into(), "::defs::Other".into()]).to_string());
        assert_eq!(impls.matches("implschematic_runtime::ErrorBody<FixtureApi>for").count(), 2, "{impls}");
        assert!(impls.contains("for::defs::Problem{fndecode_body(_:usize,body:&[u8])->Option<Self>{schematic_runtime::serde_json::from_slice(body).ok()}}"), "{impls}");
    }
}
