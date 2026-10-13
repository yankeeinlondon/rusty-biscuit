//! The hand-written request runtime shared by every REST client that
//! `schematic-gen` generates into `schematic-schema`.
//!
//! Generated code stays thin: per endpoint it declares a request type and its
//! [`EndpointSpec`] implementation. This crate owns everything that executes a
//! request and describes its result:
//!
//! - the [`Response`] envelope, its [`Outcome`], and [`ApiError`] with its
//!   [`ErrorKind`];
//! - [`TransportError`], the result when no HTTP response head arrived;
//! - request and response metadata ([`RequestMeta`], [`ResponseMeta`]) and the
//!   redaction that keeps credentials out of it ([`redact`]);
//! - the [`Decode`] boundary through which codecs read a live response body,
//!   and the built-in codecs [`Json`], [`Text`], [`Binary`], and [`Empty`];
//! - the managed [`Client`], created infallibly from an [`ApiDefinition`]
//!   and configured by chained settings that are validated on its first send
//!   or by [`Client::build`], optionally sharing a [`SharedTransport`];
//! - the [`Call`] that [`Client::send`] returns, which sends when awaited and
//!   returns the data or a [`CallError`], and its [`FullCall`] form, which
//!   returns the whole [`Response`];
//! - [`api_client!`], which gives a generated API's client struct the
//!   client's settings and sends as inherent methods;
//! - programmatic [`Credential`]s and the API's [`AuthConfig`], per-call
//!   [`CallOptions`] for [`Client::send_raw`], and the shared [`Deadline`];
//! - the [`OriginPolicy`] that governs redirects and followed links;
//! - [`Client::send_raw`] and its [`RawResponse`], for a reply without
//!   decoding;
//! - the two response [`hooks`], gated by [`SupportsHooks`].
//!
//! ## The governing rule
//!
//! A send returns `Ok` (a [`Response`] from an awaited [`FullCall`], or a
//! [`RawResponse`] from [`Client::send_raw`]) if and only if an HTTP response head (status
//! line and headers) was received; any HTTP status, including an undecodable
//! or cut-off body, is reported inside the envelope with its raw bytes. A send
//! returns `Err(TransportError)` only when no head arrived. An awaited
//! [`Call`] folds the two failure cases into one [`CallError`].
//!
//! ## What is safe to log
//!
//! Stored metadata and every `Display` implementation in this crate are free
//! of credentials: secret header values, secret query values, and URL user
//! information show [`redact::REDACTED`]. Raw body bytes and decoded provider
//! values are kept unchanged as evidence, so the `Debug` output of an
//! [`ApiError`] can print a credential that a provider echoed in its error
//! body.
//!
//! The runtime does not depend on `schematic-define`; generated code
//! translates definition data into runtime configuration.

mod api_client;
pub mod call;
pub mod client;
pub mod codec;
pub mod credentials;
pub mod deadline;
mod encode;
pub mod endpoint;
pub mod error;
pub mod hooks;
pub mod meta;
pub mod options;
pub mod origin;
pub mod raw;
pub mod redact;
pub mod response;
pub mod transport;

pub use call::{Call, CallError, CallFuture, FullCall};
pub use client::{ApiDefinition, Client, DEFAULT_TIMEOUT, MAX_REDIRECTS};
pub use codec::{Binary, BodyReadError, Decode, DecodeFailure, DecodeFailureKind, Empty, Json, LiveBody, Text};
pub use credentials::{AuthConfig, Credential, CredentialKind, CredentialPlacement, EnvFallback};
pub use deadline::Deadline;
pub use endpoint::{Api, EndpointSpec, ErrorBody, FormPart, RequestBody, RequestParts};
pub use error::{BuildError, InvalidArgument, TransportError, TransportErrorKind};
pub use hooks::{HookError, HookStage, SupportsHooks};
pub use meta::{AuthMethod, RequestMeta, ResponseMeta};
pub use options::CallOptions;
pub use origin::OriginPolicy;
pub use raw::{RawCompletion, RawJsonError, RawResponse};
pub use redact::{RedactedUrl, SecretNames};
pub use transport::{SharedTransport, SharedTransportBuilder};

/// The HTTP method type of [`EndpointSpec::METHOD`] and [`RequestParts::method`].
pub use http::Method;
/// The URL type of [`Client::check_link`].
pub use url::Url;
/// The JSON value type of [`Client::pre_response_json`], named by
/// [`api_client!`] expansions.
#[doc(hidden)]
pub use serde_json;
pub use response::{
    ApiError, ErrorKind, FailureCategory, NoDeclaredError, Outcome, RedirectRefusal, Response,
};

/// Compatibility marker between this runtime and generated code.
///
/// Generated `lib.rs` asserts at compile time that this value equals the
/// value its generator was built against, and the generator's partial
/// generation refuses to keep modules generated against a different value.
/// Increment it with every change that generated code can observe.
pub const ABI: u32 = 3;
