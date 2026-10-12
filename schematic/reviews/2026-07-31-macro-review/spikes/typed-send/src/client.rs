//! The per-API client. `send` / `send_with` are the ONLY request methods and
//! contain no per-endpoint code: everything endpoint-specific comes from
//! `R: Endpoint` (types, codec, declared statuses) and the enum's
//! `into_parts`.

use std::sync::Arc;
use std::time::{Duration, Instant};

use crate::api::{DemoRequest, RequestBody, RequestParts};

/// What `send` needs from a generated request enum.
pub trait ErasedRequest {
    fn endpoint_id(&self) -> &'static str;
    fn into_parts(self) -> Result<RequestParts, String>;
}

impl ErasedRequest for DemoRequest {
    fn endpoint_id(&self) -> &'static str {
        DemoRequest::endpoint_id(self)
    }
    fn into_parts(self) -> Result<RequestParts, String> {
        DemoRequest::into_parts(self)
    }
}
use crate::runtime::{
    AnyResponseMutator, ApiError, AuthMethod, Decode, DecodeCtx, Endpoint, ErrorBody, ErrorKind,
    Outcome, RequestMeta, Response, ResponseContext, ResponseMeta, Secret, SupportsHooks,
    TransportError, TransportErrorKind, TypedMutator, VariantHooks, read_body, redact_headers,
    redact_query, REDACTED,
};

/// Where a resolved credential goes on the wire.
#[derive(Debug, Clone)]
pub enum AuthPlacement {
    Bearer,
    /// `ApiKeyParam` in the query string; the parameter is always redacted.
    QueryParam(&'static str),
}

/// Per-call options. `credential` overrides the client-level one (D11).
#[derive(Debug, Default, Clone)]
pub struct CallOptions {
    pub credential: Option<Secret>,
    /// Set by a pager; `None` for a single call.
    pub page: Option<u32>,
}

pub struct DemoClientBuilder {
    base_url: String,
    placement: AuthPlacement,
    client_credential: Option<Secret>,
    env_vars: Vec<String>,
    auth_required: bool,
    secret_query: Vec<&'static str>,
    secret_headers: Vec<&'static str>,
    request_id_header: Option<&'static str>,
    timeout: Duration,
    hooks: VariantHooks,
}

impl DemoClientBuilder {
    pub fn credential(mut self, token: Secret) -> Self {
        self.client_credential = Some(token);
        self
    }
    pub fn env_vars(mut self, vars: &[&str]) -> Self {
        self.env_vars = vars.iter().map(|s| (*s).to_string()).collect();
        self
    }
    pub fn auth_required(mut self, required: bool) -> Self {
        self.auth_required = required;
        self
    }
    pub fn placement(mut self, placement: AuthPlacement) -> Self {
        self.placement = placement;
        self
    }
    pub fn secret_query_param(mut self, name: &'static str) -> Self {
        self.secret_query.push(name);
        self
    }
    pub fn secret_header(mut self, name: &'static str) -> Self {
        self.secret_headers.push(name);
        self
    }
    pub fn request_id_header(mut self, name: &'static str) -> Self {
        self.request_id_header = Some(name);
        self
    }
    pub fn timeout(mut self, t: Duration) -> Self {
        self.timeout = t;
        self
    }
    pub fn pre_response_json(
        mut self,
        hook: impl Fn(&ResponseContext, serde_json::Value) -> Result<serde_json::Value, String>
        + Send
        + Sync
        + 'static,
    ) -> Self {
        self.hooks.pre_response_json = Some(Arc::new(hook));
        self
    }
    /// Same mechanism as today (endpoint-id keyed, `Any` downcast), but the
    /// closure's type comes from `R::Success`, and only JSON-decoded
    /// endpoints accept a mutator.
    pub fn mutate_response<R>(
        mut self,
        hook: impl Fn(&ResponseContext, &mut R::Success) -> Result<(), String> + Send + Sync + 'static,
    ) -> Self
    where
        R: Endpoint,
        R::Decode: SupportsHooks,
    {
        let m: Arc<dyn AnyResponseMutator> = Arc::new(TypedMutator::<R::Success, _>::new(hook));
        self.hooks.response_mutators.insert(R::ENDPOINT_ID, m);
        self
    }
    pub fn build(self) -> DemoClient {
        let http = reqwest::Client::builder()
            .redirect(reqwest::redirect::Policy::none())
            .timeout(self.timeout)
            .build()
            .expect("spike: client builds");
        DemoClient {
            http,
            base_url: self.base_url.trim_end_matches('/').to_string(),
            placement: self.placement,
            client_credential: self.client_credential,
            env_vars: self.env_vars,
            auth_required: self.auth_required,
            secret_query: self.secret_query,
            secret_headers: self.secret_headers,
            request_id_header: self.request_id_header,
            hooks: self.hooks,
        }
    }
}

pub struct DemoClient {
    http: reqwest::Client,
    base_url: String,
    placement: AuthPlacement,
    client_credential: Option<Secret>,
    env_vars: Vec<String>,
    auth_required: bool,
    secret_query: Vec<&'static str>,
    secret_headers: Vec<&'static str>,
    request_id_header: Option<&'static str>,
    hooks: VariantHooks,
}

impl DemoClient {
    pub fn builder(base_url: impl Into<String>) -> DemoClientBuilder {
        DemoClientBuilder {
            base_url: base_url.into(),
            placement: AuthPlacement::Bearer,
            client_credential: None,
            env_vars: vec![],
            auth_required: false,
            secret_query: vec![],
            secret_headers: vec![],
            request_id_header: None,
            timeout: Duration::from_secs(30),
            hooks: VariantHooks::default(),
        }
    }

    /// The one typed send. `R` decides `T`, `E`, and the codec.
    pub async fn send<R>(
        &self,
        request: R,
    ) -> Result<Response<R::Success, R::Failure>, TransportError>
    where
        R: Endpoint + Into<DemoRequest>,
    {
        self.send_with(request, CallOptions::default()).await
    }

    pub async fn send_with<R>(
        &self,
        request: R,
        options: CallOptions,
    ) -> Result<Response<R::Success, R::Failure>, TransportError>
    where
        R: Endpoint + Into<DemoRequest>,
    {
        self.send_via::<DemoRequest, R>(request, options).await
    }

    /// The shared body. In generated code this would live in the shared
    /// runtime, with each API's `send` a one-line wrapper naming its enum.
    pub async fn send_via<Q, R>(
        &self,
        request: R,
        options: CallOptions,
    ) -> Result<Response<R::Success, R::Failure>, TransportError>
    where
        Q: ErasedRequest,
        R: Endpoint + Into<Q>,
    {
        let started = Instant::now();
        let erased: Q = request.into();
        debug_assert_eq!(erased.endpoint_id(), R::ENDPOINT_ID);
        let parts = erased.into_parts();

        // D11: per-call, client-level, env (first non-empty), anonymous. Once.
        let (auth, token) = self.resolve_credential(&options);

        let (method, path, mut query, body, endpoint_headers) = match parts {
            Ok(p) => (p.method, p.path, p.query, p.body, p.headers),
            Err(e) => {
                let meta = self.request_meta(R::ENDPOINT_ID, "?", "", &[], &[], &auth, options.page);
                return Err(transport(meta, TransportErrorKind::Build(e), started));
            }
        };

        let mut headers: Vec<(String, String)> =
            vec![("user-agent".into(), "typed-send-spike".into())];
        headers.extend(endpoint_headers);
        let mut secret_query = self.secret_query.clone();
        if let Some(token) = &token {
            match self.placement {
                AuthPlacement::Bearer => {
                    headers.push(("authorization".into(), format!("Bearer {}", token.expose())));
                }
                AuthPlacement::QueryParam(name) => {
                    query.push((name.to_string(), token.expose().to_string()));
                }
            }
        }
        if let AuthPlacement::QueryParam(name) = self.placement {
            secret_query.push(name);
        }
        if matches!(body, RequestBody::Json(_)) {
            headers.push(("content-type".into(), "application/json".into()));
        }

        let red_query = redact_query(&query, &secret_query);
        let red_headers = redact_headers(
            headers.iter().map(|(k, v)| (k.as_str(), v.as_str())),
            &self.secret_headers,
        );
        let meta = RequestMeta {
            endpoint_id: R::ENDPOINT_ID,
            method,
            url: build_url(&self.base_url, &path, &red_query, &secret_query),
            headers: red_headers,
            query: red_query,
            auth: auth.clone(),
            page: options.page,
        };

        if token.is_none() && self.auth_required {
            let kind = TransportErrorKind::AuthenticationRequired {
                env_vars: self.env_vars.clone(),
            };
            return Err(transport(meta, kind, started));
        }

        let real_url = build_url(&self.base_url, &path, &query, &[]);
        let method_value = reqwest::Method::from_bytes(method.as_bytes())
            .map_err(|e| transport(meta.clone(), TransportErrorKind::Build(e.to_string()), started))?;
        let mut builder = self.http.request(method_value, &real_url);
        for (k, v) in &headers {
            builder = builder.header(k.as_str(), v.as_str());
        }
        if let RequestBody::Json(bytes) = body {
            builder = builder.body(bytes);
        }
        // `real_url` and `headers` (which hold the token) die here.
        drop(headers);

        let response = match builder.send().await {
            Ok(r) => r,
            Err(e) => return Err(transport(meta, classify(&e), started)),
        };

        let status = response.status().as_u16();
        let response_headers = redact_headers(
            response
                .headers()
                .iter()
                .map(|(k, v)| (k.as_str(), v.to_str().unwrap_or("<non-utf8>"))),
            &self.secret_headers,
        );
        if (300..400).contains(&status) {
            let location = response_headers
                .iter()
                .find(|(k, _)| k == "location")
                .map(|(_, v)| v.clone());
            return Err(transport(
                meta,
                TransportErrorKind::RedirectDenied { status, location },
                started,
            ));
        }
        let content_type = response_headers
            .iter()
            .find(|(k, _)| k == "content-type")
            .map(|(_, v)| v.clone());
        let request_id = self.request_id_header.and_then(|name| {
            response_headers
                .iter()
                .find(|(k, _)| k.eq_ignore_ascii_case(name))
                .map(|(_, v)| v.clone())
        });
        let ctx = ResponseContext {
            endpoint_id: R::ENDPOINT_ID,
            method,
            url: meta.url.clone(),
            status,
            headers: response_headers.clone(),
        };

        let outcome = if (200..300).contains(&status) {
            let dctx = DecodeCtx {
                response: &ctx,
                content_type,
                hooks: &self.hooks,
            };
            match <R::Decode as Decode<R::Failure>>::decode(response, dctx).await {
                Ok(value) => Outcome::Success(value),
                Err(error) => Outcome::Failure(error),
            }
        } else {
            Outcome::Failure(decode_error_response::<R>(status, response).await)
        };

        Ok(Response {
            meta: ResponseMeta {
                request: meta,
                status,
                headers: response_headers,
                // Buffered codecs: full body read. Streams: headers only.
                elapsed: started.elapsed(),
                request_id,
            },
            outcome,
        })
    }

    fn resolve_credential(&self, options: &CallOptions) -> (AuthMethod, Option<Secret>) {
        if let Some(t) = &options.credential {
            return (AuthMethod::PerCall, Some(t.clone()));
        }
        if let Some(t) = &self.client_credential {
            return (AuthMethod::ClientLevel, Some(t.clone()));
        }
        for var in &self.env_vars {
            if let Ok(v) = std::env::var(var)
                && !v.is_empty()
            {
                return (AuthMethod::EnvVar(var.clone()), Some(Secret::new(v)));
            }
        }
        (AuthMethod::Anonymous, None)
    }

    #[allow(clippy::too_many_arguments)]
    fn request_meta(
        &self,
        endpoint_id: &'static str,
        method: &'static str,
        path: &str,
        query: &[(String, String)],
        headers: &[(String, String)],
        auth: &AuthMethod,
        page: Option<u32>,
    ) -> RequestMeta {
        RequestMeta {
            endpoint_id,
            method,
            url: build_url(&self.base_url, path, query, &self.secret_query),
            headers: headers.to_vec(),
            query: query.to_vec(),
            auth: auth.clone(),
            page,
        }
    }
}

async fn decode_error_response<R: Endpoint>(
    status: u16,
    response: reqwest::Response,
) -> ApiError<R::Failure> {
    let body = match read_body::<R::Failure>(response).await {
        Ok(b) => b,
        Err(e) => return e,
    };
    let declared = <R::Failure as ErrorBody>::DECLARED
        && R::DECLARED_ERROR_STATUSES
            .iter()
            .any(|(lo, hi)| (*lo..=*hi).contains(&status));
    let kind = if !declared {
        ErrorKind::Undeclared
    } else {
        match <R::Failure as ErrorBody>::decode(&body) {
            Ok(e) => ErrorKind::Declared(e),
            Err(d) => ErrorKind::MalformedError(d),
        }
    };
    ApiError { kind, body }
}

fn transport(request: RequestMeta, kind: TransportErrorKind, started: Instant) -> TransportError {
    TransportError {
        request,
        kind,
        elapsed: started.elapsed(),
    }
}

/// Never stores `reqwest::Error` itself: its `Display` includes the full
/// URL, which may carry a query-string credential.
fn classify(e: &reqwest::Error) -> TransportErrorKind {
    let mut message = e.without_url_ref_string();
    let mut source = std::error::Error::source(e);
    while let Some(s) = source {
        message.push_str(": ");
        message.push_str(&s.to_string());
        source = s.source();
    }
    if e.is_timeout() {
        TransportErrorKind::Timeout
    } else if e.is_connect() {
        TransportErrorKind::Connect(message)
    } else {
        TransportErrorKind::Other(message)
    }
}

trait WithoutUrlString {
    fn without_url_ref_string(&self) -> String;
}

impl WithoutUrlString for reqwest::Error {
    fn without_url_ref_string(&self) -> String {
        // `without_url` consumes; `Display` without the URL is the kind text.
        let full = self.to_string();
        match self.url() {
            Some(url) => full.replace(&format!(" ({url})"), "").replace(url.as_str(), REDACTED),
            None => full,
        }
    }
}

fn build_url(base: &str, path: &str, query: &[(String, String)], literal_values: &[&str]) -> String {
    let mut url = format!("{base}{path}");
    for (i, (k, v)) in query.iter().enumerate() {
        url.push(if i == 0 { '?' } else { '&' });
        url.push_str(&percent_encode(k));
        url.push('=');
        // A redacted value is shown literally as `[redacted]`.
        if v == REDACTED && literal_values.iter().any(|s| s.eq_ignore_ascii_case(k)) {
            url.push_str(REDACTED);
        } else {
            url.push_str(&percent_encode(v));
        }
    }
    url
}

pub(crate) fn percent_encode(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    for b in s.bytes() {
        match b {
            b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'_' | b'.' | b'~' => out.push(b as char),
            _ => out.push_str(&format!("%{b:02X}")),
        }
    }
    out
}
