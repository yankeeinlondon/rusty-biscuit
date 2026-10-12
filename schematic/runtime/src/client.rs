//! The managed client and the typed send.
//!
//! A [`Client`] is built for one API marker `A` and sends only requests whose
//! [`EndpointSpec::Api`] is `A`. Generated code supplies the API's
//! [`ApiDefinition`]; callers adjust it through [`ClientBuilder`] but cannot
//! inject an HTTP client, so every send goes through the runtime's credential
//! resolution, expiration, and redaction.

use std::fmt;
use std::marker::PhantomData;
use std::sync::Arc;
use std::time::Duration;

use bytes::Bytes;
use http::header::{CONTENT_TYPE, COOKIE, USER_AGENT};
use http::{HeaderMap, HeaderName, HeaderValue, Method};
use tokio::time::Instant;
use url::Url;

use crate::codec::{BodyReadError, Decode, DecodeFailureKind, LiveBody, StreamBounds};
use crate::credentials::{AuthConfig, Credential, CredentialPlacement};
use crate::deadline::{Deadline, Expiration, Limit};
use crate::encode::{self, EncodedBody};
use crate::endpoint::{Api, EndpointSpec, ErrorBody, RequestParts};
use crate::error::{BuildError, InvalidArgument, TransportError, TransportErrorKind};
use crate::meta::{AuthMethod, RequestMeta, ResponseMeta};
use crate::options::{CallAuth, CallOptions};
use crate::redact::SecretNames;
use crate::response::{ApiError, ErrorKind, FailureCategory, Outcome, Response};
use crate::transport::{self, ConnectionSettings, SharedTransport};

/// The per-request timeout used when the builder sets none.
pub const DEFAULT_TIMEOUT: Duration = Duration::from_secs(30);

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
}

impl ApiDefinition {
    /// An API served at `base_url`, with no authentication.
    pub fn new(base_url: impl Into<String>) -> Self {
        Self {
            base_url: base_url.into(),
            auth: AuthConfig::none(),
            secret_names: Vec::new(),
            request_id_header: None,
        }
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

    /// Builds the client.
    ///
    /// ## Errors
    ///
    /// Returns [`BuildError`] when the base URL is not a plain HTTP(S) URL,
    /// a default header is invalid or carries a credential, the client
    /// credential is of a kind the API does not accept, a connection option
    /// is set on a client attached to a shared transport, or the HTTP client
    /// cannot be initialized.
    pub fn build(self) -> Result<Client<A>, BuildError> {
        let base_url = parse_base_url(&self.definition.base_url)?;
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

/// A request ready for its first hop: the URL and headers as the application
/// assembled them, before any credential.
struct Assembled {
    method: Method,
    url: Url,
    headers: HeaderMap,
    body: Option<EncodedBody>,
}

impl<A: Api> Client<A> {
    /// Sends `request` and decodes its reply with the endpoint's codec.
    ///
    /// ## Returns
    ///
    /// `Ok` with an envelope whenever a response head arrived, whatever its
    /// status and whether its body could be read or decoded.
    ///
    /// ## Errors
    ///
    /// Returns [`TransportError`] only when no response head arrived: an
    /// argument or URL was refused before sending, the shared deadline had
    /// already expired, or the connection, TLS handshake, or wait for the
    /// head failed.
    pub async fn send<R>(&self, request: R, options: &CallOptions) -> Result<Response<R::Success, R::Failure>, TransportError>
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
        let auth = resolved.auth.clone();
        if let Some((placement, value)) = &resolved.sent
            && let Err(argument) = place_credential(&mut assembled, placement, value)
        {
            let meta = self.meta_for::<R>(&assembled.method, &assembled.url, &assembled.headers, auth);
            return Err(pre_send(TransportErrorKind::InvalidArgument(argument), meta));
        }

        let meta = self.meta_for::<R>(&assembled.method, &assembled.url, &assembled.headers, auth);
        let deadline = options.deadline.as_ref();
        if let Some(deadline) = deadline
            && deadline.is_expired()
        {
            return Err(pre_send(deadline_exceeded(deadline), meta));
        }
        let expiration = Expiration::start(start, inner.timeout, deadline);

        let mut http_request = inner.http.request(assembled.method.clone(), assembled.url.clone());
        http_request = http_request.headers(assembled.headers.clone());
        if let Some(body) = &assembled.body {
            http_request = http_request.body(body.bytes.clone());
        }
        let reply = match tokio::time::timeout_at(expiration.at, http_request.send()).await {
            Ok(Ok(reply)) => reply,
            Ok(Err(error)) => return Err(pre_send(category_kind(transport::classify(&error), deadline), meta)),
            Err(_) => {
                let kind = match (expiration.limit, deadline) {
                    (Limit::SharedDeadline, Some(deadline)) => deadline_exceeded(deadline),
                    _ => TransportErrorKind::Timeout,
                };
                return Err(pre_send(kind, meta));
            }
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
        let outcome = classify_reply::<R>(body).await;
        if let Some(deadline) = deadline {
            deadline.record_completed();
        }
        Ok(Response {
            meta: response_meta,
            outcome,
        })
    }

    /// Joins the endpoint's path and query onto the base URL and assembles
    /// its headers and body.
    fn assemble(&self, parts: RequestParts) -> Result<Assembled, TransportErrorKind> {
        let inner = &*self.inner;
        let RequestParts {
            method,
            path,
            query,
            headers: part_headers,
            body,
        } = parts;
        let separator = if path.starts_with('/') || path.is_empty() { "" } else { "/" };
        let joined = format!("{}{separator}{path}", inner.base_url.as_str().trim_end_matches('/'));
        let mut url = Url::parse(&joined).map_err(|_| TransportErrorKind::InvalidUrl)?;
        url.set_fragment(None);
        if !query.is_empty() {
            url.query_pairs_mut().extend_pairs(&query);
        }

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
        match placement {
            CredentialPlacement::ApiKeyQuery { name } => remove_query(&mut assembled.url, name),
            CredentialPlacement::ApiKeyCookie { name } => remove_cookie(&mut assembled.headers, name),
            other => {
                if let Some(header) = other.header_name() {
                    assembled.headers.remove(header);
                }
            }
        }
    }
}

fn remove_query(url: &mut Url, name: &str) {
    let kept: Vec<(String, String)> = url
        .query_pairs()
        .filter(|(pair_name, _)| pair_name != name)
        .map(|(pair_name, value)| (pair_name.into_owned(), value.into_owned()))
        .collect();
    if kept.len() == url.query_pairs().count() {
        return;
    }
    url.set_query(None);
    if !kept.is_empty() {
        url.query_pairs_mut().extend_pairs(&kept);
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
            assembled.url.query_pairs_mut().append_pair(name, value);
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

/// Classifies a received reply: a 2xx through the endpoint's codec, any other
/// status by its declared error body.
async fn classify_reply<R: EndpointSpec>(body: LiveBody) -> Outcome<R::Success, R::Failure> {
    let status = body.status();
    if status.is_success() {
        return match R::Decoder::decode(body).await {
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
