//! Choosing one of several declared success shapes for a 2xx reply.
//!
//! A multi-shape endpoint's success type is a generated enum implementing
//! [`Alternatives`]: a `const` table of [`Route`]s, one per shape, and a
//! decode of the chosen shape. Its codec is [`Shapes`]. The choice mirrors
//! the definition's rules:
//!
//! 1. A 204 reply selects the empty shape; no other shape is chosen on 204,
//!    and the empty shape is chosen on no other status.
//! 2. The shapes declaring the reply's status are the candidates; when none
//!    declares it, the shapes that declare no status are.
//! 3. The candidates whose media type equals the reply's `Content-Type`
//!    (case and parameters ignored) match.
//! 4. Several matches are told apart by their [`JsonTag`], read from the
//!    body.
//! 5. With no match, the candidate marked [`fallback`](Route::fallback), if
//!    any, is chosen.
//!
//! A reply that this leaves without a shape, including one whose
//! `Content-Type` is missing, invalid, or given twice with different values,
//! is [`ErrorKind::UnexpectedContentType`](crate::ErrorKind::UnexpectedContentType)
//! with its status, headers, and raw body. A single-shape endpoint uses its
//! codec directly and never checks the content type.

use std::marker::PhantomData;

use bytes::Bytes;
use http::header::CONTENT_TYPE;
use http::{HeaderMap, StatusCode};
use serde_json::Value;

use crate::codec::{Decode, DecodeFailure, DecodeFailureKind, LiveBody};
use crate::hooks::{HookStage, SupportsHooks};
use crate::stream::Scalar;

/// The kind of one declared success shape.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub enum ShapeKind {
    /// A JSON value; the only kind a response hook runs on.
    Json,
    /// Text.
    Text,
    /// Bytes.
    Binary,
    /// No body; chosen by a 204 reply.
    Empty,
    /// An SSE, NDJSON, or byte stream.
    Stream,
}

/// The tag that separates JSON shapes sharing a status and media type.
#[derive(Debug, Clone, Copy, PartialEq)]
#[non_exhaustive]
pub enum JsonTag {
    /// The body's top-level field `name` equals `value`.
    Field {
        /// The top-level field name.
        name: &'static str,
        /// The value that selects this shape.
        value: Scalar,
    },
    /// The body is a JSON array.
    Array,
    /// The body is a JSON object.
    Object,
}

/// The selection facts of one shape, built with `const` methods.
#[derive(Debug, Clone, Copy, PartialEq)]
#[non_exhaustive]
pub struct Route {
    /// The shape's kind.
    pub kind: ShapeKind,
    /// The effective media type; `None` for the empty and fallback shapes.
    pub media: Option<&'static str>,
    /// The 2xx statuses the shape declares; empty when it declares none.
    pub statuses: &'static [u16],
    /// The JSON tag, for a JSON shape that shares its status and media type.
    pub tag: Option<JsonTag>,
    /// Whether the shape takes a reply whose media type no other shape
    /// matches.
    pub fallback: bool,
}

impl Route {
    /// A shape of `kind` with no media type, statuses, tag, or fallback.
    pub const fn new(kind: ShapeKind) -> Self {
        Self {
            kind,
            media: None,
            statuses: &[],
            tag: None,
            fallback: false,
        }
    }

    /// Sets the effective media type, without parameters.
    #[must_use]
    pub const fn media(mut self, media: &'static str) -> Self {
        self.media = Some(media);
        self
    }

    /// Sets the declared statuses.
    #[must_use]
    pub const fn statuses(mut self, statuses: &'static [u16]) -> Self {
        self.statuses = statuses;
        self
    }

    /// Sets the JSON tag.
    #[must_use]
    pub const fn tag(mut self, tag: JsonTag) -> Self {
        self.tag = Some(tag);
        self
    }

    /// Marks the fallback shape.
    #[must_use]
    pub const fn fallback(mut self) -> Self {
        self.fallback = true;
        self
    }
}

/// The success enum of a multi-shape endpoint.
pub trait Alternatives: Sized + Send + 'static {
    /// One route per shape, in variant order.
    const ROUTES: &'static [Route];

    /// Decodes `body` as the shape at `index` of [`ROUTES`](Self::ROUTES),
    /// with that shape's codec. The runtime passes only valid indices.
    fn decode_alternative(index: usize, body: LiveBody) -> impl Future<Output = Result<Self, DecodeFailure>> + Send;
}

/// Marks an [`Alternatives`] enum whose every shape is JSON, so a typed
/// response mutator may be registered for it; generated code implements it
/// only then.
pub trait JsonAlternatives: Alternatives {}

/// The codec of a multi-shape endpoint whose success enum is `A`.
///
/// The client-global JSON hook runs when the chosen shape is JSON. It
/// supports a typed mutator only when `A` is [`JsonAlternatives`].
pub struct Shapes<A>(PhantomData<fn() -> A>);

impl<A: JsonAlternatives> SupportsHooks for Shapes<A> {}

impl<A: Alternatives> Decode for Shapes<A> {
    type Output = A;

    async fn decode(body: LiveBody) -> Result<A, DecodeFailure> {
        decode_shapes(body, None).await
    }

    async fn decode_hooked(body: LiveBody, hooks: HookStage<'_>) -> Result<A, DecodeFailure> {
        decode_shapes(body, Some(hooks)).await
    }
}

async fn decode_shapes<A: Alternatives>(body: LiveBody, hooks: Option<HookStage<'_>>) -> Result<A, DecodeFailure> {
    let status = body.status();
    let headers = body.headers().clone();
    let (index, body) = match select(A::ROUTES, status, &headers) {
        Choice::One(index) => (index, body),
        Choice::ByTag(matched) => {
            let bytes = body.collect().await?;
            match select_by_tag(A::ROUTES, &matched, &bytes) {
                Some(index) => (index, LiveBody::buffered(status, headers.clone(), bytes)),
                None => return Err(DecodeFailure::new(DecodeFailureKind::UnexpectedContentType, bytes)),
            }
        }
        Choice::Unexpected => {
            let bytes = body.collect().await?;
            return Err(DecodeFailure::new(DecodeFailureKind::UnexpectedContentType, bytes));
        }
    };
    let hooks = hooks.filter(|hooks| A::ROUTES[index].kind == ShapeKind::Json && !hooks.is_empty());
    let Some(hooks) = hooks else {
        return A::decode_alternative(index, body).await;
    };
    let original = body.collect().await?;
    let malformed = || DecodeFailure::new(DecodeFailureKind::Malformed, original.clone());
    let hook_failed = || DecodeFailure::new(DecodeFailureKind::Hook, original.clone());
    let decoded = if hooks.has_pre_response_json() {
        // The global hook sees only a body that parses as JSON.
        let json: Value = serde_json::from_slice(&original).map_err(|_| malformed())?;
        let json = hooks.pre_response_json(json).map_err(|_| hook_failed())?;
        Bytes::from(serde_json::to_vec(&json).map_err(|_| malformed())?)
    } else {
        original.clone()
    };
    let mut value = A::decode_alternative(index, LiveBody::buffered(status, headers, decoded))
        .await
        .map_err(|failure| DecodeFailure::new(failure.kind, original.clone()))?;
    hooks.mutate(&mut value).map_err(|_| hook_failed())?;
    Ok(value)
}

/// The outcome of selecting by status and media type.
enum Choice {
    One(usize),
    /// Several shapes match; their JSON tags decide.
    ByTag(Vec<usize>),
    Unexpected,
}

fn select(routes: &[Route], status: StatusCode, headers: &HeaderMap) -> Choice {
    if status == StatusCode::NO_CONTENT {
        return routes
            .iter()
            .position(|route| route.kind == ShapeKind::Empty)
            .map_or(Choice::Unexpected, Choice::One);
    }
    let status = status.as_u16();
    let bodied = || routes.iter().enumerate().filter(|(_, route)| route.kind != ShapeKind::Empty);
    let declaring: Vec<usize> = bodied()
        .filter(|(_, route)| route.statuses.contains(&status))
        .map(|(index, _)| index)
        .collect();
    let candidates = if declaring.is_empty() {
        bodied().filter(|(_, route)| route.statuses.is_empty()).map(|(index, _)| index).collect()
    } else {
        declaring
    };
    let Some(media) = content_type(headers) else {
        return Choice::Unexpected;
    };
    let matched: Vec<usize> = candidates
        .iter()
        .copied()
        .filter(|index| {
            let route = &routes[*index];
            !route.fallback && route.media.is_some_and(|declared| declared.eq_ignore_ascii_case(&media))
        })
        .collect();
    match matched.len() {
        0 => candidates
            .iter()
            .copied()
            .find(|index| routes[*index].fallback)
            .map_or(Choice::Unexpected, Choice::One),
        1 => Choice::One(matched[0]),
        _ => Choice::ByTag(matched),
    }
}

/// The reply's media type without parameters, lowercased; `None` when the
/// header is missing, invalid, or given more than once with different
/// media types.
fn content_type(headers: &HeaderMap) -> Option<String> {
    let mut found: Option<String> = None;
    for value in headers.get_all(CONTENT_TYPE) {
        let media = value.to_str().ok()?.split(';').next()?.trim().to_ascii_lowercase();
        let (kind, subtype) = media.split_once('/')?;
        let token = |part: &str| {
            !part.is_empty() && part.bytes().all(|byte| byte.is_ascii_graphic() && !b"*/()<>@,;:\\\"[]?={}".contains(&byte))
        };
        if !token(kind) || !token(subtype) {
            return None;
        }
        match &found {
            Some(earlier) if *earlier != media => return None,
            _ => found = Some(media),
        }
    }
    found
}

fn select_by_tag(routes: &[Route], matched: &[usize], body: &[u8]) -> Option<usize> {
    let first = body.iter().find(|byte| !byte.is_ascii_whitespace());
    let mut json: Option<Option<Value>> = None;
    matched.iter().copied().find(|index| match routes[*index].tag {
        Some(JsonTag::Array) => first == Some(&b'['),
        Some(JsonTag::Object) => first == Some(&b'{'),
        Some(JsonTag::Field { name, value }) => json
            .get_or_insert_with(|| serde_json::from_slice(body).ok())
            .as_ref()
            .and_then(|json| json.as_object()?.get(name))
            .is_some_and(|field| value.matches(field)),
        None => false,
    })
}
