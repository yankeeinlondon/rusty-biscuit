//! Building and decoding page requests.
//!
//! A page is sent as an ordinary typed send of [`PageRequest`], a
//! runtime-internal endpoint that keeps the listing endpoint's API, id,
//! method, and failure type, so credentials, the origin policy, redirects,
//! the deadline, and redaction apply exactly as to an unpaginated send, and
//! a refusal reports the listing endpoint.
//!
//! The page body is parsed once by [`PageDecoder`] into a JSON value, from
//! which the engine reads records and continuations and [`decode_items`]
//! decodes items with `serde_json::from_value`. The endpoint's own `Json`
//! codec already compiles that deserializer for its response type (its
//! global-hook branch decodes from a value), so a page adds no second
//! deserializer per item type.

use std::marker::PhantomData;
use std::ops::RangeInclusive;

use bytes::Bytes;
use http::{HeaderMap, Method};
use serde::de::DeserializeOwned;
use serde_json::{Map, Value};
use url::Url;

use super::engine::Plan;
use super::json::as_text;
use super::spec::{Carry, Input, Strategy};
use crate::codec::{Decode, DecodeFailure, DecodeFailureKind, LiveBody};
use crate::endpoint::{EndpointSpec, FormPart, RequestBody, RequestParts};
use crate::error::InvalidArgument;
use crate::hooks::HookStage;

/// One page request of the listing endpoint `R`.
pub(crate) struct PageRequest<R> {
    pub(crate) parts: Result<RequestParts, InvalidArgument>,
    pub(crate) endpoint: PhantomData<fn() -> R>,
}

impl<R: EndpointSpec> EndpointSpec for PageRequest<R> {
    type Api = R::Api;
    type Success = PageDocument;
    type Failure = R::Failure;
    type Decoder = PageDecoder;

    const ENDPOINT_ID: &'static str = R::ENDPOINT_ID;
    const METHOD: Method = R::METHOD;
    const DECLARED_ERROR_STATUSES: RangeInclusive<u16> = R::DECLARED_ERROR_STATUSES;

    fn into_parts(self) -> Result<RequestParts, InvalidArgument> {
        self.parts
    }
}

/// A 2xx page body that parsed as JSON, with the bytes and headers it came
/// with.
pub(crate) struct PageDocument {
    pub(crate) json: Value,
    /// The body as received, kept as evidence.
    pub(crate) bytes: Bytes,
    /// As received, not redacted: `Link` and continuation headers are read
    /// from them, never stored.
    pub(crate) headers: HeaderMap,
}

/// Reads a page body as JSON; a body that does not parse is
/// [`DecodeFailureKind::Malformed`] with its bytes.
///
/// The client's global JSON hook runs on a page as on an ordinary JSON
/// reply, because the definition's item and continuation paths describe the
/// body the hook produces. The endpoint's typed mutator does not: it is
/// typed by the endpoint's own response, which a page is not.
pub(crate) struct PageDecoder;

impl Decode for PageDecoder {
    type Output = PageDocument;

    async fn decode(body: LiveBody) -> Result<PageDocument, DecodeFailure> {
        let headers = body.headers().clone();
        let bytes = body.collect().await?;
        match serde_json::from_slice(&bytes) {
            Ok(json) => Ok(PageDocument { json, bytes, headers }),
            Err(_) => Err(DecodeFailure::new(DecodeFailureKind::Malformed, bytes)),
        }
    }

    async fn decode_hooked(body: LiveBody, hooks: HookStage<'_>) -> Result<PageDocument, DecodeFailure> {
        if !hooks.has_pre_response_json() {
            return Self::decode(body).await;
        }
        let mut document = Self::decode(body).await?;
        let json = std::mem::take(&mut document.json);
        document.json = hooks
            .pre_response_json(json)
            .map_err(|_| DecodeFailure::new(DecodeFailureKind::Hook, document.bytes.clone()))?;
        Ok(document)
    }
}

/// One request of a session step (open, renew, or close) of the listing
/// endpoint `R`, sent with `R`'s API, id, credentials, and origin policy.
pub(crate) struct StepRequest<R> {
    pub(crate) parts: Result<RequestParts, InvalidArgument>,
    pub(crate) endpoint: PhantomData<fn() -> R>,
}

impl<R: EndpointSpec> EndpointSpec for StepRequest<R> {
    type Api = R::Api;
    type Success = StepDocument;
    type Failure = R::Failure;
    type Decoder = StepDecoder;

    const ENDPOINT_ID: &'static str = R::ENDPOINT_ID;
    const METHOD: Method = R::METHOD;
    const DECLARED_ERROR_STATUSES: RangeInclusive<u16> = R::DECLARED_ERROR_STATUSES;

    fn into_parts(self) -> Result<RequestParts, InvalidArgument> {
        self.parts
    }
}

/// A 2xx session-step reply: its JSON body, or `Null` when the body is
/// empty or not JSON (a close often answers with no body), and its headers.
pub(crate) struct StepDocument {
    pub(crate) json: Value,
    /// As received, not redacted: an open step's handle may be a header.
    pub(crate) headers: HeaderMap,
}

/// Reads a session-step reply; it never fails on the body's shape, because
/// only an open step reads anything from it, and a missing handle is that
/// step's own error.
pub(crate) struct StepDecoder;

impl Decode for StepDecoder {
    type Output = StepDocument;

    async fn decode(body: LiveBody) -> Result<StepDocument, DecodeFailure> {
        let headers = body.headers().clone();
        let bytes = body.collect().await?;
        let json = serde_json::from_slice(&bytes).unwrap_or(Value::Null);
        Ok(StepDocument { json, headers })
    }
}

/// Decodes a page's projected items, all or none.
pub(crate) fn decode_items<T: DeserializeOwned>(items: Vec<Value>) -> Option<Vec<T>> {
    items.into_iter().map(|item| serde_json::from_value(item).ok()).collect()
}

/// The parts of a planned page request.
///
/// The first request is the endpoint's own parts with the managed inputs
/// written over them. A follow-up keeps the ordinary inputs the carry rule
/// keeps; a followed link replaces the method, path, and query with the
/// link's (GET unless the follow-up names a method), drops the body, and
/// sets only carried query values on it. Managed inputs always win over ordinary ones.
///
/// ## Errors
///
/// Returns [`InvalidArgument`] for a refused plan, a managed value that
/// cannot be written where it goes, a follow-up path the endpoint's path
/// cannot fill, or a body the carry rule cannot select fields of; the reason
/// names the input, never its value.
pub(crate) fn build_parts(
    base: &RequestParts,
    plan: &Plan,
    strategy: &Strategy,
    path_template: &str,
) -> Result<RequestParts, InvalidArgument> {
    if let Some(problem) = &plan.problem {
        return Err(InvalidArgument::new(problem.clone()));
    }
    let carry = strategy.effective_carry();
    let follow_up = strategy.follow_up.as_ref().filter(|_| plan.follow_up);
    let mut parts = base.clone();
    if let Some(link) = &plan.link {
        let mut url = link.clone();
        let carried: Vec<(String, String)> = base
            .query
            .iter()
            .filter(|(name, _)| carry.keeps(name))
            .cloned()
            .collect();
        set_url_query(&mut url, &carried);
        parts.method = follow_up.and_then(|follow_up| follow_up.method.clone()).unwrap_or(Method::GET);
        parts.path = url.to_string();
        parts.query = Vec::new();
        parts.body = RequestBody::Empty;
    } else if plan.follow_up {
        if let Some(follow_up) = follow_up {
            if let Some(method) = &follow_up.method {
                parts.method = method.clone();
            }
            if let Some(template) = follow_up.path {
                parts.path = fill_path(template, path_template, &base.path)
                    .ok_or_else(|| InvalidArgument::new("the follow-up path cannot be filled from the endpoint path"))?;
            }
        }
        parts.query.retain(|(name, _)| carry.keeps(name));
        parts.body = carried_body(&base.body, carry)?;
    }
    for (input, value) in &plan.inputs {
        write_input(&mut parts, *input, value)?;
    }
    Ok(parts)
}

/// Sets each `(name, value)` on `url`'s query, replacing values of the same
/// names already there and keeping everything else in order.
fn set_url_query(url: &mut Url, pairs: &[(String, String)]) {
    if pairs.is_empty() {
        return;
    }
    let kept: Vec<(String, String)> = url
        .query_pairs()
        .filter(|(name, _)| !pairs.iter().any(|(set, _)| set == name))
        .map(|(name, value)| (name.into_owned(), value.into_owned()))
        .collect();
    let mut query = url.query_pairs_mut();
    query.clear();
    for (name, value) in kept.iter().chain(pairs) {
        query.append_pair(name, value);
    }
}

/// The body a follow-up re-sends under `carry`.
///
/// `All` keeps the body and `Nothing` drops it. `Only` and `AllExcept` keep
/// the named top-level fields of a JSON object, the named pairs of a URL-encoded
/// body, or the named parts of a multipart body, in order, and drop a body
/// left with none. A body whose fields cannot be named (raw bytes, or JSON
/// that is not an object) is refused rather than re-sent whole or dropped.
fn carried_body(body: &RequestBody, carry: Carry) -> Result<RequestBody, InvalidArgument> {
    let refused = || InvalidArgument::new("the follow-up carry rule cannot select fields of this request body");
    let kept = match (body, carry) {
        (RequestBody::Empty, _) | (_, Carry::All) => body.clone(),
        (_, Carry::Nothing) => RequestBody::Empty,
        (RequestBody::Json(text), _) => match serde_json::from_str::<Value>(text) {
            Ok(Value::Object(fields)) => {
                let kept: Map<String, Value> = fields.into_iter().filter(|(name, _)| carry.keeps(name)).collect();
                if kept.is_empty() {
                    RequestBody::Empty
                } else {
                    RequestBody::Json(Value::Object(kept).to_string())
                }
            }
            _ => return Err(refused()),
        },
        (RequestBody::UrlEncoded(pairs), _) => {
            RequestBody::UrlEncoded(pairs.iter().filter(|(name, _)| carry.keeps(name)).cloned().collect())
        }
        (RequestBody::Multipart(parts), _) => {
            RequestBody::Multipart(parts.iter().filter(|part| carry.keeps(part_name(part))).cloned().collect())
        }
        (RequestBody::Raw { .. }, _) => return Err(refused()),
    };
    Ok(match kept {
        RequestBody::UrlEncoded(pairs) if pairs.is_empty() => RequestBody::Empty,
        RequestBody::Multipart(parts) if parts.is_empty() => RequestBody::Empty,
        kept => kept,
    })
}

fn part_name(part: &FormPart) -> &str {
    match part {
        FormPart::Text { name, .. } | FormPart::Json { name, .. } | FormPart::File { name, .. } => name,
    }
}

/// Writes a managed `value` at `input`, replacing what is there.
pub(crate) fn write_input(parts: &mut RequestParts, input: Input, value: &Value) -> Result<(), InvalidArgument> {
    match input {
        Input::Query(name) => {
            let text = as_text(value).ok_or_else(|| structured(name))?;
            if is_absolute(&parts.path) {
                // A followed link keeps its query in the path.
                let mut url = Url::parse(&parts.path).map_err(|_| InvalidArgument::new("the page link is not a valid URL"))?;
                set_url_query(&mut url, &[(name.to_owned(), text)]);
                parts.path = url.to_string();
            } else {
                parts.query.retain(|(existing, _)| existing != name);
                parts.query.push((name.to_owned(), text));
            }
        }
        Input::Header(name) => {
            let text = as_text(value).ok_or_else(|| structured(name))?;
            parts.headers.retain(|(existing, _)| !existing.eq_ignore_ascii_case(name));
            parts.headers.push((name.to_owned(), text));
        }
        Input::Body(path) => {
            let mut document = match &parts.body {
                RequestBody::Empty => Value::Object(Map::new()),
                RequestBody::Json(text) => serde_json::from_str(text)
                    .map_err(|_| InvalidArgument::new("the request body is not a JSON document"))?,
                _ => return Err(InvalidArgument::new("a pagination input targets a body that is not JSON")),
            };
            set_body_path(&mut document, path, value.clone())?;
            parts.body = RequestBody::Json(document.to_string());
        }
    }
    Ok(())
}

fn structured(name: &str) -> InvalidArgument {
    InvalidArgument::new(format!("pagination input `{name}` cannot carry a structured value"))
}

fn set_body_path(document: &mut Value, path: &[&str], value: Value) -> Result<(), InvalidArgument> {
    let Some((last, parents)) = path.split_last() else {
        return Err(InvalidArgument::new("a pagination body input has no path"));
    };
    let mut node = document;
    for key in parents {
        let Value::Object(map) = node else {
            return Err(InvalidArgument::new("a pagination body input crosses a value that is not an object"));
        };
        node = map.entry((*key).to_owned()).or_insert_with(|| Value::Object(Map::new()));
    }
    let Value::Object(map) = node else {
        return Err(InvalidArgument::new("a pagination body input crosses a value that is not an object"));
    };
    map.insert((*last).to_owned(), value);
    Ok(())
}

fn is_absolute(path: &str) -> bool {
    let lower = path.get(..8).unwrap_or(path).to_ascii_lowercase();
    lower.starts_with("http://") || lower.starts_with("https://")
}

/// Fills `template`'s `{placeholders}` with the values the endpoint's
/// `endpoint_template` took in `path`, matched segment by segment; `None`
/// when `path` does not match or a placeholder is not the endpoint's.
pub(crate) fn fill_path(template: &str, endpoint_template: &str, path: &str) -> Option<String> {
    let names: Vec<&str> = endpoint_template.split('/').collect();
    let values: Vec<&str> = path.split('/').collect();
    if names.len() != values.len() {
        return None;
    }
    let mut filled = template.to_owned();
    for (name, value) in names.iter().zip(&values) {
        if let Some(param) = name.strip_prefix('{').and_then(|rest| rest.strip_suffix('}')) {
            filled = filled.replace(&format!("{{{param}}}"), value);
        } else if name != value {
            return None;
        }
    }
    (!filled.contains('{')).then_some(filled)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn follow_up_path_takes_the_endpoint_path_values() {
        assert_eq!(
            fill_path("/repos/{owner}/{repo}/next", "/repos/{owner}/{repo}/items", "/repos/a%20b/c/items").as_deref(),
            Some("/repos/a%20b/c/next")
        );
        assert_eq!(fill_path("/x/{other}", "/repos/{owner}", "/repos/a"), None);
        assert_eq!(fill_path("/x", "/repos/{owner}", "/other/a"), None);
    }

    #[test]
    fn link_query_values_are_replaced_and_others_kept_in_order() {
        let mut url = Url::parse("https://a.test/x?cursor=c1&per_page=5&q=a").expect("url");
        set_url_query(&mut url, &[("per_page".to_owned(), "50".to_owned())]);
        assert_eq!(url.as_str(), "https://a.test/x?cursor=c1&q=a&per_page=50");
    }

    #[test]
    fn body_inputs_create_nested_objects() {
        let mut document = Value::Object(Map::new());
        set_body_path(&mut document, &["pit", "id"], Value::from("h")).expect("set");
        assert_eq!(document, serde_json::json!({ "pit": { "id": "h" } }));
        let mut scalar = serde_json::json!({ "pit": 1 });
        assert!(set_body_path(&mut scalar, &["pit", "id"], Value::from("h")).is_err());
    }
}
