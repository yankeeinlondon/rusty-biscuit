//! The managed client, the typed send, and the raw send.
//!
//! A [`Client`] is built for one API marker `A` and sends only requests whose
//! [`EndpointSpec::Api`] is `A`. Generated code supplies the API's
//! [`ApiDefinition`]; callers adjust it through [`ClientBuilder`] but cannot
//! inject an HTTP client, so every send goes through the runtime's credential
//! resolution, origin policy, expiration, and redaction.
//!
//! ## Redirects
//!
//! The HTTP client never follows a redirect; every send ([`Client::send`] and
//! [`Client::send_raw`], through one shared execution core) follows them
//! itself, one hop at a time, so that the [`OriginPolicy`] is checked at every
//! hop. Each hop is rebuilt from the request as the application assembled it,
//! before any credential, and the resolved credential is attached only when
//! the policy allows it for that hop's target. Every hop shares the send's
//! single expiration.

use std::fmt;
use std::marker::PhantomData;
use std::sync::Arc;
use std::time::Duration;

use bytes::Bytes;
use http::header::{
    CONTENT_ENCODING, CONTENT_LANGUAGE, CONTENT_LENGTH, CONTENT_LOCATION, CONTENT_TYPE, COOKIE, HOST, LOCATION,
    TRANSFER_ENCODING, USER_AGENT,
};
use http::{HeaderMap, HeaderName, HeaderValue, Method, StatusCode};
use tokio::time::Instant;
use url::Url;

use crate::codec::{BodyReadError, Decode, DecodeFailureKind, LiveBody, StreamBounds};
use crate::credentials::{AuthConfig, Credential, CredentialPlacement, Resolved};
use crate::deadline::{Deadline, Expiration, Limit};
use crate::encode::{self, EncodedBody};
use crate::endpoint::{Api, EndpointSpec, ErrorBody, RequestParts};
use crate::hooks::{HookError, HookStage, Hooks, SupportsHooks};
use crate::error::{BuildError, InvalidArgument, TransportError, TransportErrorKind};
use crate::meta::{AuthMethod, RequestMeta, ResponseMeta};
use crate::options::{CallAuth, CallOptions};
use crate::origin::{Decision, OriginPolicy, OriginRules};
use crate::raw::{RawCompletion, RawResponse};
use crate::redact::{RedactedUrl, SecretNames};
use crate::response::{ApiError, ErrorKind, FailureCategory, Outcome, RedirectRefusal, Response};
use crate::transport::{self, ConnectionSettings, SharedTransport};

/// The per-request timeout used when the builder sets none.
pub const DEFAULT_TIMEOUT: Duration = Duration::from_secs(30);

/// The most redirects one send follows; the next redirect response is
/// refused with [`RedirectRefusal::HopLimit`].
pub const MAX_REDIRECTS: usize = 10;

/// What a generated API tells the runtime about itself.
///
/// Generated code builds one from the API definition; tests build one by
/// hand.
#[derive(Debug, Clone)]
pub struct ApiDefinition {
    base_url: String,
    auth: AuthConfig,
    secret_names: Vec<String>,
    request_id_header: Option<String>,
    cross_origin: OriginPolicy,
}

impl ApiDefinition {
    /// An API served at `base_url`, with no authentication.
    pub fn new(base_url: impl Into<String>) -> Self {
        Self {
            base_url: base_url.into(),
            auth: AuthConfig::none(),
            secret_names: Vec::new(),
            request_id_header: None,
            cross_origin: OriginPolicy::SameOrigin,
        }
    }

    /// Sets the API's cross-origin policy (default
    /// [`OriginPolicy::SameOrigin`]). Callers can only tighten it, through
    /// [`ClientBuilder::cross_origin`].
    #[must_use]
    pub fn cross_origin(mut self, policy: OriginPolicy) -> Self {
        self.cross_origin = policy;
        self
    }

    /// Sets how the API accepts credentials.
    #[must_use]
    pub fn auth(mut self, auth: AuthConfig) -> Self {
        self.auth = auth;
        self
    }

    /// Adds header and query names, beyond the built-in ones, whose values
    /// are never stored.
    #[must_use]
    pub fn secret_names<I, S>(mut self, names: I) -> Self
    where
        I: IntoIterator<Item = S>,
        S: Into<String>,
    {
        self.secret_names.extend(names.into_iter().map(Into::into));
        self
    }

    /// Names the response header that carries the provider's request id.
    #[must_use]
    pub fn request_id_header(mut self, name: impl Into<String>) -> Self {
        self.request_id_header = Some(name.into());
        self
    }
}

/// A client for the API marked by `A`.
///
/// Cloning is cheap and shares the connection pool and configuration.
pub struct Client<A: Api> {
    inner: Arc<Inner>,
    api: PhantomData<fn() -> A>,
}

impl<A: Api> Clone for Client<A> {
    fn clone(&self) -> Self {
        Self {
            inner: Arc::clone(&self.inner),
            api: PhantomData,
        }
    }
}

impl<A: Api> fmt::Debug for Client<A> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Client")
            .field("api", &A::NAME)
            .field("base_url", &self.inner.secrets.redact_url(&self.inner.base_url))
            .field("timeout", &self.inner.timeout)
            .finish_non_exhaustive()
    }
}

struct Inner {
    http: reqwest::Client,
    base_url: Url,
    timeout: Duration,
    default_headers: HeaderMap,
    auth: AuthConfig,
    credential: Option<Credential>,
    secrets: SecretNames,
    request_id_header: Option<String>,
    origins: OriginRules,
    hooks: Hooks,
}

/// Builds a [`Client`].
///
/// Options that configure connections (proxy, root certificates, invalid
/// certificate acceptance, connect timeout) apply to the client's own pool;
/// a client attached to a [`SharedTransport`] takes them from the transport
/// and refuses its own.
pub struct ClientBuilder<A: Api> {
    definition: ApiDefinition,
    timeout: Duration,
    connection: ConnectionSettings,
    transport: Option<SharedTransport>,
    user_agent: Option<String>,
    default_headers: Vec<(String, String)>,
    credential: Option<Credential>,
    env_order: Option<Vec<String>>,
    cross_origin: Option<OriginPolicy>,
    hooks: Hooks,
    api: PhantomData<fn() -> A>,
}

impl<A: Api> fmt::Debug for ClientBuilder<A> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("ClientBuilder")
            .field("api", &A::NAME)
            .field("timeout", &self.timeout)
            .field("credential", &self.credential)
            .finish_non_exhaustive()
    }
}

impl<A: Api> Client<A> {
    /// A builder seeded from `definition`.
    pub fn builder(definition: ApiDefinition) -> ClientBuilder<A> {
        ClientBuilder {
            definition,
            timeout: DEFAULT_TIMEOUT,
            connection: ConnectionSettings::default(),
            transport: None,
            user_agent: None,
            default_headers: Vec::new(),
            credential: None,
            env_order: None,
            cross_origin: None,
            hooks: Hooks::default(),
            api: PhantomData,
        }
    }
}

impl<A: Api> ClientBuilder<A> {
    /// Sends requests to `url` instead of the definition's base URL, for
    /// example a local mock.
    #[must_use]
    pub fn base_url(mut self, url: impl Into<String>) -> Self {
        self.definition.base_url = url.into();
        self
    }

    /// The per-request timeout (default 30 s). Under a shared
    /// [`Deadline`] a send is bounded by the earlier of the
    /// two.
    #[must_use]
    pub fn timeout(mut self, timeout: Duration) -> Self {
        self.timeout = timeout;
        self
    }

    /// Bounds how long establishing a connection may take.
    #[must_use]
    pub fn connect_timeout(mut self, timeout: Duration) -> Self {
        self.connection.connect_timeout = Some(timeout);
        self
    }

    /// Sends every request through the proxy at `url`.
    #[must_use]
    pub fn proxy(mut self, url: impl Into<String>) -> Self {
        self.connection.proxy = Some(url.into());
        self
    }

    /// Trusts `certificate` (PEM or DER) as a root, in addition to the
    /// platform's roots.
    #[must_use]
    pub fn add_root_certificate(mut self, certificate: impl Into<Vec<u8>>) -> Self {
        self.connection.roots.push(certificate.into());
        self
    }

    /// Accepts any server certificate, including self-signed and expired
    /// ones. Only for devices that cannot present a valid certificate.
    #[must_use]
    pub fn danger_accept_invalid_certificates(mut self) -> Self {
        self.connection.accept_invalid_certificates = true;
        self
    }

    /// Sends `value` as the `User-Agent` of every request.
    #[must_use]
    pub fn user_agent(mut self, value: impl Into<String>) -> Self {
        self.user_agent = Some(value.into());
        self
    }

    /// Sends a header with every request, unless the request sets the same
    /// header itself. A header that would carry a credential is refused when
    /// the client is built.
    #[must_use]
    pub fn default_header(mut self, name: impl Into<String>, value: impl Into<String>) -> Self {
        self.default_headers.push((name.into(), value.into()));
        self
    }

    /// Takes connections from `transport` instead of a private pool.
    #[must_use]
    pub fn transport(mut self, transport: &SharedTransport) -> Self {
        self.transport = Some(transport.clone());
        self
    }

    /// Sends `credential` when a call supplies none, before the environment
    /// fallback is read.
    #[must_use]
    pub fn credentials(mut self, credential: Credential) -> Self {
        self.credential = Some(credential);
        self
    }

    /// Reads the environment fallback's variables in this order instead of
    /// the definition's. For Basic authentication these are the password
    /// variables.
    #[must_use]
    pub fn env_order<I, S>(mut self, vars: I) -> Self
    where
        I: IntoIterator<Item = S>,
        S: Into<String>,
    {
        self.env_order = Some(vars.into_iter().map(Into::into).collect());
        self
    }

    /// Tightens the API's cross-origin policy for this client.
    ///
    /// [`OriginPolicy::Deny`] and [`OriginPolicy::SameOrigin`] are accepted,
    /// and so is an [`OriginPolicy::AllowOrigins`] list that is a subset of
    /// the API's own list. Over a `Deny` API only `Deny` is accepted. Any
    /// other choice fails [`build`](Self::build).
    #[must_use]
    pub fn cross_origin(mut self, policy: OriginPolicy) -> Self {
        self.cross_origin = Some(policy);
        self
    }

    /// Runs `hook` on every successful JSON body before it is decoded, so it
    /// can reshape the document to fit the endpoint's success type.
    ///
    /// It runs only for a 2xx reply to an endpoint whose codec supports
    /// hooks, and only once the body has parsed as JSON; registering again
    /// replaces it. See [`crate::hooks`] for the full order.
    #[must_use]
    pub fn pre_response_json<F>(mut self, hook: F) -> Self
    where
        F: Fn(&ResponseMeta, serde_json::Value) -> Result<serde_json::Value, HookError> + Send + Sync + 'static,
    {
        self.hooks.set_pre_response_json(hook);
        self
    }

    /// Runs `hook` on the decoded success value of endpoint `R`, after the
    /// global JSON hook; registering again for `R` replaces it.
    ///
    /// `R` must be an endpoint of this client's API whose codec supports
    /// hooks; a text, binary, or empty endpoint does not compile.
    #[must_use]
    pub fn mutate_response<R, F>(mut self, hook: F) -> Self
    where
        R: EndpointSpec<Api = A>,
        R::Decoder: SupportsHooks,
        F: Fn(&ResponseMeta, &mut R::Success) -> Result<(), HookError> + Send + Sync + 'static,
    {
        self.hooks.set_mutator::<R, F>(hook);
        self
    }

    /// Builds the client.
    ///
    /// ## Errors
    ///
    /// Returns [`BuildError`] when the base URL is not a plain HTTP(S) URL,
    /// a default header is invalid or carries a credential, the client
    /// credential is of a kind the API does not accept, an allowed origin is
    /// not a bare HTTP(S) origin, the requested cross-origin policy loosens
    /// the API's, a connection option is set on a client attached to a shared
    /// transport, or the HTTP client cannot be initialized.
    pub fn build(self) -> Result<Client<A>, BuildError> {
        let base_url = parse_base_url(&self.definition.base_url)?;
        let origins = OriginRules::new(&base_url, &self.definition.cross_origin, self.cross_origin.as_ref())?;
        let mut auth = self.definition.auth;
        if let Some(vars) = self.env_order {
            auth.replace_env_order(vars);
        }
        if let Some(credential) = &self.credential
            && auth.placement_for(credential.kind()).is_none()
        {
            return Err(BuildError::IncompatibleCredential {
                supplied: credential.kind().name(),
            });
        }
        let secrets = SecretNames::builtin()
            .with_names(auth.placements().map(CredentialPlacement::secret_name))
            .with_names(&self.definition.secret_names);

        let mut default_headers = HeaderMap::new();
        for (name, value) in &self.default_headers {
            if secrets.is_secret_header(name) {
                return Err(BuildError::AuthBearingDefaultHeader { name: name.clone() });
            }
            let (name, value) = header_pair(name, value).ok_or_else(|| BuildError::InvalidHeader { name: name.clone() })?;
            default_headers.append(name, value);
        }
        if let Some(agent) = &self.user_agent {
            let value = HeaderValue::from_str(agent).map_err(|_| BuildError::InvalidHeader {
                name: USER_AGENT.to_string(),
            })?;
            default_headers.insert(USER_AGENT, value);
        }

        let http = match self.transport {
            Some(shared) => {
                if let Some(option) = self.connection.first_set_option() {
                    return Err(BuildError::TransportOptionConflict { option });
                }
                shared.http
            }
            None => self.connection.build()?,
        };

        Ok(Client {
            inner: Arc::new(Inner {
                http,
                base_url,
                timeout: self.timeout,
                default_headers,
                auth,
                credential: self.credential,
                secrets,
                request_id_header: self.definition.request_id_header,
                origins,
                hooks: self.hooks,
            }),
            api: PhantomData,
        })
    }
}

fn parse_base_url(text: &str) -> Result<Url, BuildError> {
    let url = Url::parse(text).map_err(|_| BuildError::InvalidBaseUrl {
        reason: "not an absolute URL",
    })?;
    if !matches!(url.scheme(), "http" | "https") {
        return Err(BuildError::InvalidBaseUrl {
            reason: "scheme is not http or https",
        });
    }
    if url.host_str().is_none_or(str::is_empty) {
        return Err(BuildError::InvalidBaseUrl { reason: "no host" });
    }
    if !url.username().is_empty() || url.password().is_some() {
        return Err(BuildError::InvalidBaseUrl {
            reason: "carries user information",
        });
    }
    Ok(url)
}

fn header_pair(name: &str, value: &str) -> Option<(HeaderName, HeaderValue)> {
    Some((HeaderName::from_bytes(name.as_bytes()).ok()?, HeaderValue::from_str(value).ok()?))
}

/// One hop's request as the application assembled it, before any credential.
#[derive(Clone)]
struct Assembled {
    method: Method,
    url: Url,
    headers: HeaderMap,
    body: Option<EncodedBody>,
}

/// How [`Client::execute`] ended once a response head arrived.
enum Exchange {
    /// A reply that is not a followable redirect, with its body unread.
    Final { meta: ResponseMeta, body: LiveBody },
    /// A redirect that stopped the send, with its body read.
    Stopped { meta: ResponseMeta, body: Bytes, stop: Stop },
}

/// Why a redirect stopped the send.
enum Stop {
    /// The redirect's own body was cut off.
    Interrupted,
    /// The redirect was not followed.
    Refused(RedirectRefusal),
    /// The next hop received no head.
    Failed { target: RedactedUrl, category: FailureCategory },
}

impl<A: Api> Client<A> {
    /// Sends `request` and decodes its reply with the endpoint's codec,
    /// following redirects the cross-origin policy allows.
    ///
    /// A 2xx reply to an endpoint whose codec supports hooks runs the
    /// client's response hooks (see [`crate::hooks`]).
    ///
    /// ## Returns
    ///
    /// `Ok` with an envelope whenever a response head arrived, whatever its
    /// status and whether its body could be read or decoded. After a redirect
    /// the envelope describes the last response received: the final reply, a
    /// redirect that was not followed ([`ErrorKind::RedirectRefused`]), or the
    /// last redirect when the next hop received no head
    /// ([`ErrorKind::RedirectFailed`]).
    ///
    /// ## Errors
    ///
    /// Returns [`TransportError`] only when no response head arrived: an
    /// argument, URL, or cross-origin target was refused before sending, the
    /// shared deadline had already expired, or the connection, TLS
    /// handshake, or wait for the first head failed.
    ///
    /// ## Notes
    ///
    /// The returned future is one heap pointer: the request's state machine
    /// is boxed, so joining many sends in one future needs no large stack.
    pub fn send<R>(
        &self,
        request: R,
        options: &CallOptions,
    ) -> impl Future<Output = Result<Response<R::Success, R::Failure>, TransportError>>
    where
        R: EndpointSpec<Api = A>,
    {
        Box::pin(self.send_unboxed(request, options))
    }

    /// The body of [`send`](Self::send), before boxing.
    async fn send_unboxed<R>(&self, request: R, options: &CallOptions) -> Result<Response<R::Success, R::Failure>, TransportError>
    where
        R: EndpointSpec<Api = A>,
    {
        let (meta, outcome) = match self.execute(request, options).await? {
            Exchange::Final { meta, body } => {
                let outcome = classify_reply::<R>(body, self.inner.hooks.stage::<R>(&meta)).await;
                (meta, outcome)
            }
            Exchange::Stopped { meta, body, stop } => {
                let kind = match stop {
                    Stop::Interrupted => ErrorKind::BodyInterrupted,
                    Stop::Refused(reason) => ErrorKind::RedirectRefused { reason },
                    Stop::Failed { target, category } => ErrorKind::RedirectFailed { target, category },
                };
                (meta, Outcome::Failure(ApiError { kind, body }))
            }
        };
        Ok(completed(options.deadline.as_ref(), meta, outcome))
    }

    /// Sends `request` and returns its reply's metadata and body bytes
    /// undecoded, through the same credentials, cross-origin policy,
    /// redirects, expiration, and redaction as [`send`](Self::send).
    ///
    /// No status is a failure here and no response hook runs.
    ///
    /// ## Returns
    ///
    /// `Ok` whenever a response head arrived. [`RawResponse::completion`]
    /// says whether the body is complete, was interrupted, or belongs to a
    /// redirect that was refused or whose next hop failed.
    ///
    /// ## Errors
    ///
    /// Returns [`TransportError`] in exactly the cases [`send`](Self::send)
    /// does: when no response head arrived.
    ///
    /// ## Notes
    ///
    /// The returned future is boxed, as [`send`](Self::send)'s is.
    pub fn send_raw<R>(&self, request: R, options: &CallOptions) -> impl Future<Output = Result<RawResponse, TransportError>>
    where
        R: EndpointSpec<Api = A>,
    {
        Box::pin(self.send_raw_unboxed(request, options))
    }

    /// The body of [`send_raw`](Self::send_raw), before boxing.
    async fn send_raw_unboxed<R>(&self, request: R, options: &CallOptions) -> Result<RawResponse, TransportError>
    where
        R: EndpointSpec<Api = A>,
    {
        let (meta, body, completion) = match self.execute(request, options).await? {
            Exchange::Final { meta, body } => match body.collect().await {
                Ok(bytes) => (meta, bytes, RawCompletion::Complete),
                Err(failure) => (meta, failure.retained, RawCompletion::BodyInterrupted),
            },
            Exchange::Stopped { meta, body, stop } => {
                let completion = match stop {
                    Stop::Interrupted => RawCompletion::BodyInterrupted,
                    Stop::Refused(reason) => RawCompletion::RedirectRefused { reason },
                    Stop::Failed { target, category } => RawCompletion::RedirectFailed { target, category },
                };
                (meta, body, completion)
            }
        };
        if let Some(deadline) = &options.deadline {
            deadline.record_completed();
        }
        Ok(RawResponse { meta, body, completion })
    }

    /// The execution core shared by every send: assembles `request`, checks
    /// its origin, resolves the credential once, and follows redirects under
    /// one expiration until a reply that ends the send.
    ///
    /// It does not count the send against its deadline; the caller does,
    /// once the outcome is classified.
    async fn execute<R>(&self, request: R, options: &CallOptions) -> Result<Exchange, TransportError>
    where
        R: EndpointSpec<Api = A>,
    {
        let start = Instant::now();
        let inner = &*self.inner;
        let pre_send = |kind: TransportErrorKind, meta: RequestMeta| TransportError::new(meta, kind, start.elapsed());

        let parts = request
            .into_parts()
            .map_err(|argument| pre_send(TransportErrorKind::InvalidArgument(argument), self.base_meta::<R>()))?;
        let method = parts.method.clone();
        let mut assembled = self
            .assemble(parts)
            .map_err(|kind| pre_send(kind, self.meta_for::<R>(&method, &inner.base_url, &HeaderMap::new(), AuthMethod::Anonymous)))?;

        let Decision::Follow { credentials } = inner.origins.link(&assembled.url) else {
            let meta = self.meta_for::<R>(&assembled.method, &assembled.url, &assembled.headers, AuthMethod::Anonymous);
            let target = inner.secrets.redact_url(&assembled.url);
            return Err(pre_send(TransportErrorKind::CrossOriginRejected { target }, meta));
        };

        let resolved = inner.auth.resolve(&options.auth, inner.credential.as_ref());
        let resolved = match resolved {
            Ok(resolved) => resolved,
            Err(argument) => {
                let meta = self.meta_for::<R>(&assembled.method, &assembled.url, &assembled.headers, AuthMethod::Anonymous);
                return Err(pre_send(TransportErrorKind::InvalidArgument(argument), meta));
            }
        };
        if matches!(options.auth, CallAuth::Anonymous) {
            strip_credentials(&inner.auth, &mut assembled);
        }
        let (sent, auth) = match self.authenticate(&assembled, credentials, &resolved, false) {
            Ok(sent) => sent,
            Err(argument) => {
                let meta = self.meta_for::<R>(&assembled.method, &assembled.url, &assembled.headers, resolved.auth.clone());
                return Err(pre_send(TransportErrorKind::InvalidArgument(argument), meta));
            }
        };

        let deadline = options.deadline.as_ref();
        if let Some(deadline) = deadline
            && deadline.is_expired()
        {
            let meta = self.meta_for::<R>(&sent.method, &sent.url, &sent.headers, auth);
            return Err(pre_send(deadline_exceeded(deadline), meta));
        }
        let expiration = Expiration::start(start, inner.timeout, deadline);
        let expiry_category = match expiration.limit {
            Limit::SharedDeadline => FailureCategory::DeadlineExceeded,
            Limit::RequestTimeout => FailureCategory::Timeout,
        };

        let mut current = assembled;
        let mut hop = (sent, auth);
        let mut followed = 0;
        // The last redirect received, with its body, once a hop has been
        // followed.
        let mut last_redirect: Option<(ResponseMeta, Bytes)> = None;
        loop {
            let (sent, auth) = hop;
            let meta = self.meta_for::<R>(&sent.method, &sent.url, &sent.headers, auth);
            let failed_hop = |category: FailureCategory, last_redirect: Option<(ResponseMeta, Bytes)>| match last_redirect {
                Some((mut redirect_meta, body)) => {
                    redirect_meta.elapsed = start.elapsed();
                    let target = inner.secrets.redact_url(&current.url);
                    Ok(Exchange::Stopped {
                        meta: redirect_meta,
                        body,
                        stop: Stop::Failed { target, category },
                    })
                }
                None => Err(pre_send(category_kind(category, deadline), meta.clone())),
            };
            // A hop never starts after the send's expiration.
            if last_redirect.is_some() && Instant::now() >= expiration.at {
                return failed_hop(expiry_category, last_redirect);
            }
            let reply = match tokio::time::timeout_at(expiration.at, self.dispatch(&sent)).await {
                Ok(Ok(reply)) => reply,
                Ok(Err(error)) => return failed_hop(transport::classify(&error), last_redirect),
                Err(_) => return failed_hop(expiry_category, last_redirect),
            };

            let status = reply.status();
            let headers = reply.headers().clone();
            let response_meta = ResponseMeta::capture(
                &inner.secrets,
                meta,
                status,
                &headers,
                inner.request_id_header.as_deref(),
                start.elapsed(),
            );
            let body = live_body(reply, &expiration);
            if !is_followable_redirect(status) {
                return Ok(Exchange::Final { meta: response_meta, body });
            }

            // A redirect's body is read before the next hop, both as evidence
            // and so that its connection can be reused.
            let body = match body.collect().await {
                Ok(bytes) => bytes,
                Err(failure) => {
                    return Ok(Exchange::Stopped {
                        meta: response_meta,
                        body: failure.retained,
                        stop: Stop::Interrupted,
                    });
                }
            };
            let (next, credentials) = match self.next_hop(&current, status, &headers, followed) {
                Ok(next) => next,
                Err(reason) => {
                    return Ok(Exchange::Stopped {
                        meta: response_meta,
                        body,
                        stop: Stop::Refused(reason),
                    });
                }
            };
            hop = match self.authenticate(&next, credentials, &resolved, true) {
                Ok(sent) => sent,
                Err(_) => {
                    let target = inner.secrets.redact_url(&next.url);
                    return Ok(Exchange::Stopped {
                        meta: response_meta,
                        body,
                        stop: Stop::Failed {
                            target,
                            category: FailureCategory::Other,
                        },
                    });
                }
            };
            current = next;
            followed += 1;
            last_redirect = Some((response_meta, body));
        }
    }

    /// Checks a URL the caller is about to request, such as a next-page
    /// link, against the cross-origin policy for the endpoint `R`, without any
    /// network work.
    ///
    /// A same-origin URL passes under every policy, including
    /// [`OriginPolicy::Deny`].
    ///
    /// ## Errors
    ///
    /// Returns [`TransportError`] of kind
    /// [`TransportErrorKind::CrossOriginRejected`] when the policy refuses
    /// `url`'s origin, or when `url` is not a plain HTTP(S) URL.
    pub fn check_link<R>(&self, url: &Url) -> Result<(), TransportError>
    where
        R: EndpointSpec<Api = A>,
    {
        match self.inner.origins.link(url) {
            Decision::Follow { .. } => Ok(()),
            Decision::Refuse => {
                let meta = self.meta_for::<R>(&R::METHOD, url, &HeaderMap::new(), AuthMethod::Anonymous);
                let target = self.inner.secrets.redact_url(url);
                Err(TransportError::new(meta, TransportErrorKind::CrossOriginRejected { target }, Duration::ZERO))
            }
        }
    }

    /// Starts one hop's request.
    async fn dispatch(&self, sent: &Assembled) -> reqwest::Result<reqwest::Response> {
        let mut request = self.inner.http.request(sent.method.clone(), sent.url.clone()).headers(sent.headers.clone());
        if let Some(body) = &sent.body {
            request = request.body(body.bytes.clone());
        }
        request.send().await
    }

    /// The request actually sent for `hop`, and the credential source it
    /// reports.
    ///
    /// With `credentials`, the resolved credential is attached; on a later
    /// hop (`replace`) a value the redirect target already carries in the
    /// credential's slot is replaced rather than refused. Without, every
    /// secret header and secret query value is removed.
    fn authenticate(
        &self,
        hop: &Assembled,
        credentials: bool,
        resolved: &Resolved,
        replace: bool,
    ) -> Result<(Assembled, AuthMethod), InvalidArgument> {
        let mut sent = hop.clone();
        if !credentials {
            strip_secrets(&self.inner.secrets, &mut sent);
            return Ok((sent, AuthMethod::Anonymous));
        }
        if let Some((placement, value)) = &resolved.sent {
            if replace {
                remove_placement(&mut sent, placement);
            }
            place_credential(&mut sent, placement, value)?;
        }
        Ok((sent, resolved.auth.clone()))
    }

    /// The next hop for a redirect received in reply to `current`, and
    /// whether the credential goes with it.
    fn next_hop(
        &self,
        current: &Assembled,
        status: StatusCode,
        headers: &HeaderMap,
        followed: usize,
    ) -> Result<(Assembled, bool), RedirectRefusal> {
        let location = headers.get(LOCATION).ok_or(RedirectRefusal::MissingLocation)?;
        let location = std::str::from_utf8(location.as_bytes()).map_err(|_| RedirectRefusal::InvalidLocation)?;
        // Resolved against the hop's URL before any credential, so a
        // relative `Location` never inherits a query-string credential.
        let mut target = current.url.join(location).map_err(|_| RedirectRefusal::InvalidLocation)?;
        if !matches!(target.scheme(), "http" | "https") || !target.username().is_empty() || target.password().is_some() {
            return Err(RedirectRefusal::UnsafeLocation);
        }
        if followed >= MAX_REDIRECTS {
            return Err(RedirectRefusal::HopLimit);
        }
        target.set_fragment(None);
        let Decision::Follow { credentials } = self.inner.origins.redirect(&target) else {
            return Err(RedirectRefusal::CrossOrigin);
        };

        let becomes_get = match status {
            StatusCode::MOVED_PERMANENTLY | StatusCode::FOUND => current.method == Method::POST,
            StatusCode::SEE_OTHER => current.method != Method::HEAD,
            _ => false,
        };
        let mut next = current.clone();
        next.url = target;
        next.headers.remove(HOST);
        if becomes_get {
            next.method = Method::GET;
            next.body = None;
            for name in [CONTENT_TYPE, CONTENT_LENGTH, CONTENT_ENCODING, CONTENT_LANGUAGE, CONTENT_LOCATION, TRANSFER_ENCODING] {
                next.headers.remove(name);
            }
        }
        Ok((next, credentials))
    }

    /// Joins the endpoint's path (or takes it whole when it is an absolute
    /// URL) and query onto the base URL, and assembles its headers and body.
    fn assemble(&self, parts: RequestParts) -> Result<Assembled, TransportErrorKind> {
        let inner = &*self.inner;
        let RequestParts {
            method,
            path,
            query,
            headers: part_headers,
            body,
        } = parts;
        let mut url = if is_absolute_url(&path) {
            // An absolute URL, such as a followed next-page link, is checked
            // against the cross-origin policy before it is sent.
            Url::parse(&path)
        } else {
            let separator = if path.starts_with('/') || path.is_empty() { "" } else { "/" };
            Url::parse(&format!("{}{separator}{path}", inner.base_url.as_str().trim_end_matches('/')))
        }
        .map_err(|_| TransportErrorKind::InvalidUrl)?;
        url.set_fragment(None);
        append_query(&mut url, query.iter().map(|(name, value)| (name.as_str(), value.as_str())));

        let mut headers = inner.default_headers.clone();
        let mut replaced = Vec::new();
        for (name, value) in &part_headers {
            let (name, value) = header_pair(name, value).ok_or_else(|| {
                TransportErrorKind::InvalidArgument(InvalidArgument::new(format!("header `{name}` is not valid")))
            })?;
            if !replaced.contains(&name) {
                headers.remove(&name);
                replaced.push(name.clone());
            }
            headers.append(name, value);
        }
        let body = encode::encode(&body);
        if let Some(body) = &body {
            let value = HeaderValue::from_str(&body.content_type).map_err(|_| {
                TransportErrorKind::InvalidArgument(InvalidArgument::new("the body content type is not a valid header"))
            })?;
            headers.insert(CONTENT_TYPE, value);
        }
        Ok(Assembled {
            method,
            url,
            headers,
            body,
        })
    }

    fn meta_for<R: EndpointSpec>(&self, method: &Method, url: &Url, headers: &HeaderMap, auth: AuthMethod) -> RequestMeta {
        RequestMeta::capture(&self.inner.secrets, R::ENDPOINT_ID, method.clone(), url, headers, auth)
    }

    /// Metadata for a request whose parts could not be assembled: the base
    /// URL and the endpoint's declared method.
    fn base_meta<R: EndpointSpec>(&self) -> RequestMeta {
        self.meta_for::<R>(&R::METHOD, &self.inner.base_url, &HeaderMap::new(), AuthMethod::Anonymous)
    }
}

fn deadline_exceeded(deadline: &Deadline) -> TransportErrorKind {
    TransportErrorKind::DeadlineExceeded {
        completed_requests: deadline.completed_requests(),
        deadline_elapsed: deadline.elapsed(),
    }
}

fn category_kind(category: FailureCategory, deadline: Option<&Deadline>) -> TransportErrorKind {
    match category {
        FailureCategory::Connect => TransportErrorKind::Connect,
        FailureCategory::Tls => TransportErrorKind::Tls,
        FailureCategory::Timeout => TransportErrorKind::Timeout,
        FailureCategory::DeadlineExceeded => match deadline {
            Some(deadline) => deadline_exceeded(deadline),
            None => TransportErrorKind::Timeout,
        },
        _ => TransportErrorKind::Other,
    }
}

/// Removes every value any of the API's placements could carry, for a call
/// that asked for no credential.
fn strip_credentials(auth: &AuthConfig, assembled: &mut Assembled) {
    for placement in auth.placements() {
        remove_placement(assembled, placement);
    }
}

/// Removes whatever `placement` would carry.
fn remove_placement(assembled: &mut Assembled, placement: &CredentialPlacement) {
    match placement {
        CredentialPlacement::ApiKeyQuery { name } => remove_query(&mut assembled.url, |pair_name| pair_name == name),
        CredentialPlacement::ApiKeyCookie { name } => remove_cookie(&mut assembled.headers, name),
        other => {
            if let Some(header) = other.header_name() {
                assembled.headers.remove(header);
            }
        }
    }
}

/// Removes every secret header, including `Cookie`, and every secret query
/// value, for a hop the credential must not reach.
fn strip_secrets(secrets: &SecretNames, assembled: &mut Assembled) {
    let names: Vec<HeaderName> = assembled.headers.keys().filter(|name| secrets.is_secret_header(name.as_str())).cloned().collect();
    for name in names {
        assembled.headers.remove(name);
    }
    remove_query(&mut assembled.url, |name| secrets.is_secret_query(name));
}

fn remove_query(url: &mut Url, remove: impl Fn(&str) -> bool) {
    let kept: Vec<(String, String)> = url
        .query_pairs()
        .filter(|(pair_name, _)| !remove(pair_name))
        .map(|(pair_name, value)| (pair_name.into_owned(), value.into_owned()))
        .collect();
    if kept.len() == url.query_pairs().count() {
        return;
    }
    url.set_query(None);
    append_query(url, kept.iter().map(|(name, value)| (name.as_str(), value.as_str())));
}

/// Appends `pairs` to `url`'s query. Names and values are percent-encoded
/// except for RFC 3986 unreserved characters, so a space is sent as `%20`,
/// never `+`; the generated clients have always put this form on the wire.
fn append_query<'a>(url: &mut Url, pairs: impl IntoIterator<Item = (&'a str, &'a str)>) {
    let mut query = url.query().unwrap_or_default().to_string();
    for (name, value) in pairs {
        if !query.is_empty() {
            query.push('&');
        }
        encode_query_component(name, &mut query);
        query.push('=');
        encode_query_component(value, &mut query);
    }
    if !query.is_empty() {
        url.set_query(Some(&query));
    }
}

fn encode_query_component(text: &str, out: &mut String) {
    for byte in text.bytes() {
        if byte.is_ascii_alphanumeric() || matches!(byte, b'-' | b'_' | b'.' | b'~') {
            out.push(char::from(byte));
        } else {
            out.push_str(&format!("%{byte:02X}"));
        }
    }
}

fn remove_cookie(headers: &mut HeaderMap, name: &str) {
    let kept: Vec<String> = cookie_pairs(headers).filter(|(cookie, _)| cookie != name).map(|(cookie, value)| format!("{cookie}={value}")).collect();
    headers.remove(COOKIE);
    if !kept.is_empty()
        && let Ok(value) = HeaderValue::from_str(&kept.join("; "))
    {
        headers.insert(COOKIE, value);
    }
}

fn cookie_pairs(headers: &HeaderMap) -> impl Iterator<Item = (String, String)> + '_ {
    headers
        .get_all(COOKIE)
        .iter()
        .filter_map(|value| value.to_str().ok())
        .flat_map(|value| value.split(';'))
        .map(str::trim)
        .filter(|piece| !piece.is_empty())
        .map(|piece| match piece.split_once('=') {
            Some((name, value)) => (name.to_string(), value.to_string()),
            None => (piece.to_string(), String::new()),
        })
}

/// Adds the resolved credential, refusing a request that already carries a
/// value where the credential goes.
fn place_credential(assembled: &mut Assembled, placement: &CredentialPlacement, value: &str) -> Result<(), InvalidArgument> {
    match placement {
        CredentialPlacement::ApiKeyQuery { name } => {
            if assembled.url.query_pairs().any(|(pair_name, _)| pair_name == name.as_str()) {
                return Err(InvalidArgument::new(format!(
                    "the request already sets the query parameter `{name}` that carries the credential"
                )));
            }
            append_query(&mut assembled.url, [(name.as_str(), value)]);
        }
        CredentialPlacement::ApiKeyCookie { name } => {
            if cookie_pairs(&assembled.headers).any(|(cookie, _)| &cookie == name) {
                return Err(InvalidArgument::new(format!(
                    "the request already sets the cookie `{name}` that carries the credential"
                )));
            }
            let mut cookies: Vec<String> = cookie_pairs(&assembled.headers).map(|(cookie, value)| format!("{cookie}={value}")).collect();
            cookies.push(format!("{name}={}", encode_cookie_value(value)));
            let header = HeaderValue::from_str(&cookies.join("; ")).map_err(|_| InvalidArgument::new("the cookie header is not valid"))?;
            assembled.headers.insert(COOKIE, header);
        }
        other => {
            let name = other.header_name().unwrap_or("Authorization");
            let header_name = HeaderName::from_bytes(name.as_bytes())
                .map_err(|_| InvalidArgument::new(format!("the credential header `{name}` is not valid")))?;
            if assembled.headers.contains_key(&header_name) {
                return Err(InvalidArgument::new(format!(
                    "the request already sets the header `{name}` that carries the credential"
                )));
            }
            let mut header = HeaderValue::from_str(value)
                .map_err(|_| InvalidArgument::new(format!("the credential for `{name}` is not a valid header value")))?;
            header.set_sensitive(true);
            assembled.headers.insert(header_name, header);
        }
    }
    Ok(())
}

/// Percent-encodes every byte that is not a cookie-octet (RFC 6265).
fn encode_cookie_value(value: &str) -> String {
    let mut encoded = String::with_capacity(value.len());
    for byte in value.bytes() {
        match byte {
            0x21 | 0x23..=0x2B | 0x2D..=0x3A | 0x3C..=0x5B | 0x5D..=0x7E => encoded.push(char::from(byte)),
            _ => encoded.push_str(&format!("%{byte:02X}")),
        }
    }
    encoded
}

/// Whether `path` is an absolute HTTP(S) URL rather than a path.
fn is_absolute_url(path: &str) -> bool {
    let lower = path.get(..8).unwrap_or(path).to_ascii_lowercase();
    lower.starts_with("http://") || lower.starts_with("https://")
}

/// The statuses whose `Location` the runtime follows; any other 3xx is a
/// final reply.
fn is_followable_redirect(status: StatusCode) -> bool {
    matches!(
        status,
        StatusCode::MOVED_PERMANENTLY
            | StatusCode::FOUND
            | StatusCode::SEE_OTHER
            | StatusCode::TEMPORARY_REDIRECT
            | StatusCode::PERMANENT_REDIRECT
    )
}

/// Counts a classified send against its deadline and wraps its envelope.
fn completed<T, E>(deadline: Option<&Deadline>, meta: ResponseMeta, outcome: Outcome<T, E>) -> Response<T, E> {
    if let Some(deadline) = deadline {
        deadline.record_completed();
    }
    Response { meta, outcome }
}

/// Wraps a received reply's body for a codec, bounded by the send's
/// expiration.
fn live_body(reply: reqwest::Response, expiration: &Expiration) -> LiveBody {
    let status = reply.status();
    let headers = reply.headers().clone();
    let stream = futures_util::stream::unfold(Some(reply), |state| async move {
        let mut reply = state?;
        match reply.chunk().await {
            Ok(Some(piece)) => Some((Ok(piece), Some(reply))),
            Ok(None) => None,
            // reqwest reports a cut-off body as a decode error; any read error
            // after the head is an interruption.
            Err(_) => Some((Err(BodyReadError::Interrupted), None)),
        }
    });
    LiveBody::new(status, headers, stream, Some(expiration.at)).with_stream_bounds(StreamBounds {
        idle: Some(expiration.timeout),
        deadline: expiration.deadline_at,
    })
}

/// Classifies a received reply: a 2xx through the endpoint's codec and the
/// client's `hooks`, any other status by its declared error body.
async fn classify_reply<R: EndpointSpec>(body: LiveBody, hooks: HookStage<'_>) -> Outcome<R::Success, R::Failure> {
    let status = body.status();
    if status.is_success() {
        return match R::Decoder::decode_hooked(body, hooks).await {
            Ok(value) => Outcome::Success(value),
            Err(failure) => {
                let kind = match failure.kind {
                    DecodeFailureKind::Interrupted => ErrorKind::BodyInterrupted,
                    DecodeFailureKind::Hook => ErrorKind::Hook,
                    _ => ErrorKind::MalformedSuccess,
                };
                Outcome::Failure(ApiError {
                    kind,
                    body: failure.retained,
                })
            }
        };
    }
    let bytes: Bytes = match body.collect().await {
        Ok(bytes) => bytes,
        Err(failure) => {
            return Outcome::Failure(ApiError {
                kind: ErrorKind::BodyInterrupted,
                body: failure.retained,
            });
        }
    };
    let kind = if R::Failure::DECLARED && R::DECLARED_ERROR_STATUSES.contains(&status.as_u16()) {
        match R::Failure::decode_body(&bytes) {
            Some(declared) => ErrorKind::Declared(declared),
            None => ErrorKind::MalformedError,
        }
    } else {
        ErrorKind::Undeclared
    };
    Outcome::Failure(ApiError { kind, body: bytes })
}
