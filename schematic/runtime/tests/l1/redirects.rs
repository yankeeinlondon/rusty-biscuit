//! Cross-origin policy and redirect following over loopback mocks: the
//! redirect matrix and credential forwarding, link checks and absolute
//! request URLs, redirect mechanics, failed hops, the shared expiration, the
//! tighten-only override, and that no constructor lets the HTTP library
//! follow a redirect (R19, R21–R26).
//!
//! Origins A, B, and C are wiremock servers on loopback that differ only in
//! port, which makes them distinct origins.

use std::time::Duration;

use bytes::Bytes;
use http::Method;
use schematic_runtime::{
    ApiDefinition, AuthConfig, AuthMethod, BuildError, CallOptions, Client, Credential, CredentialPlacement, Deadline, Empty,
    ErrorKind, FailureCategory, Json, MAX_REDIRECTS, NoDeclaredError, OriginPolicy, RedirectRefusal, RequestBody,
    RequestParts, Response, SharedTransport, TransportErrorKind,
};
use tokio::net::TcpListener;
use wiremock::matchers::{path, path_regex, query_param};
use wiremock::{Mock, MockServer, Request, Respond, ResponseTemplate};

use crate::support::{Call, Item, SENTINEL, TestApi, builder, closed_port_url, listener, read_head, write_all};

/// The credential value every client sends; it must reach only the hops the
/// policy allows.
const SECRET: &str = "cred-4b1d9e";

/// Where the client places its credential.
#[derive(Debug, Clone, Copy)]
enum Form {
    Header,
    Cookie,
    Query,
}

const FORMS: [Form; 3] = [Form::Header, Form::Cookie, Form::Query];

impl Form {
    fn placement(self) -> CredentialPlacement {
        match self {
            Self::Header => CredentialPlacement::Bearer { header: None },
            Self::Cookie => CredentialPlacement::ApiKeyCookie { name: "sid".into() },
            Self::Query => CredentialPlacement::ApiKeyQuery { name: "api_key".into() },
        }
    }

    fn credential(self) -> Credential {
        match self {
            Self::Header => Credential::access_token(SECRET),
            Self::Cookie | Self::Query => Credential::api_key(SECRET),
        }
    }
}

fn json(id: u64) -> ResponseTemplate {
    ResponseTemplate::new(200).set_body_raw(format!(r#"{{"id":{id}}}"#), "application/json")
}

fn redirect(status: u16, location: impl AsRef<str>) -> ResponseTemplate {
    ResponseTemplate::new(status).insert_header("Location", location.as_ref())
}

async fn mount(server: &MockServer, at: &str, response: ResponseTemplate) {
    Mock::given(path(at)).respond_with(response).mount(server).await;
}

async fn requests(server: &MockServer) -> Vec<Request> {
    server.received_requests().await.expect("recording is on")
}

async fn requests_to(server: &MockServer, at: &str) -> Vec<Request> {
    requests(server).await.into_iter().filter(|request| request.url.path() == at).collect()
}

/// Whether `secret` appears anywhere in the request's URL or headers.
fn carries(request: &Request, secret: &str) -> bool {
    request.url.as_str().contains(secret)
        || request.headers.values().any(|value| String::from_utf8_lossy(value.as_bytes()).contains(secret))
}

/// A definition for API base A that accepts `form`'s placement.
fn definition(form: Form) -> ApiDefinition {
    ApiDefinition::new("https://unused.invalid").auth(AuthConfig::none().accept(form.placement()))
}

/// A client for base A with a client-level credential in `form`, under the
/// definition policy `policy` (`None`: the definition sets nothing).
fn client(a: &MockServer, form: Form, policy: Option<OriginPolicy>) -> Client<TestApi> {
    let mut definition = definition(form);
    if let Some(policy) = policy {
        definition = definition.cross_origin(policy);
    }
    builder(definition, &a.uri()).credentials(form.credential()).build().expect("client builds")
}

async fn get(client: &Client<TestApi>, at: &str) -> Response<Item, NoDeclaredError> {
    client.send(Call::<Json<Item>>::get(at), &CallOptions::new()).await.expect("a head arrived")
}

fn refusal<T, E>(response: &Response<T, E>) -> Option<RedirectRefusal> {
    match response.error().map(|error| &error.kind) {
        Some(ErrorKind::RedirectRefused { reason }) => Some(*reason),
        _ => None,
    }
}

/// Three origins: A redirects to itself, to B, and to C.
struct Origins {
    a: MockServer,
    b: MockServer,
    c: MockServer,
}

async fn origins() -> Origins {
    let (a, b, c) = (MockServer::start().await, MockServer::start().await, MockServer::start().await);
    mount(&a, "/same", redirect(302, "/final")).await;
    mount(&a, "/final", json(1)).await;
    mount(&a, "/cross", redirect(302, format!("{}/landing", b.uri()))).await;
    mount(&a, "/to-c", redirect(302, format!("{}/landing", c.uri()))).await;
    mount(&b, "/landing", json(2)).await;
    mount(&c, "/landing", json(3)).await;
    Origins { a, b, c }
}

/// What a policy does with one redirect.
#[derive(Debug, Clone, Copy, PartialEq)]
enum Expect {
    Refused,
    Followed { credentials: bool },
}

/// Sends `at` on A, whose redirect lands on `target`'s `landing` path with
/// the body `{"id": id}`, and checks the policy's expected handling.
async fn check(label: &str, client: &Client<TestApi>, origins: &Origins, at: &str, target: (&MockServer, &str, u64), expect: Expect) {
    let (server, landing, id) = target;
    let before = requests_to(server, landing).await.len();
    let response = get(client, at).await;
    let after = requests_to(server, landing).await;
    match expect {
        Expect::Refused => {
            assert_eq!(refusal(&response), Some(RedirectRefusal::CrossOrigin), "{label} {at}: {response}");
            assert_eq!(response.meta.status, 302, "{label} {at}");
            assert!(response.meta.headers.contains_key("location"), "{label} {at}: Location stays visible");
            assert_eq!(after.len(), before, "{label} {at}: the refused target receives zero requests");
        }
        Expect::Followed { credentials } => {
            assert_eq!(response.data(), Some(&Item { id }), "{label} {at}: {response}");
            assert_eq!(after.len(), before + 1, "{label} {at}: followed exactly once");
            let hop = after.last().expect("the target was reached");
            assert_eq!(carries(hop, SECRET), credentials, "{label} {at}: credential on the followed hop");
            let auth = if credentials { AuthMethod::ClientLevel } else { AuthMethod::Anonymous };
            assert_eq!(response.meta.request.auth, auth, "{label} {at}: the reported source matches what was sent");
        }
    }
    let first = requests_to(&origins.a, at).await;
    assert!(carries(first.last().expect("A was asked"), SECRET), "{label} {at}: the base origin always gets the credential");
}

/// One row of the redirect matrix: the definition policy and what happens to
/// a redirect to A itself, to B, and to C.
async fn matrix_row(label: &str, policy: impl Fn(&Origins) -> Option<OriginPolicy>, same: Expect, cross: Expect, to_c: Expect) {
    for form in FORMS {
        let origins = origins().await;
        let client = client(&origins.a, form, policy(&origins));
        let label = format!("{label} / {form:?}");
        check(&label, &client, &origins, "/same", (&origins.a, "/final", 1), same).await;
        check(&label, &client, &origins, "/cross", (&origins.b, "/landing", 2), cross).await;
        check(&label, &client, &origins, "/to-c", (&origins.c, "/landing", 3), to_c).await;
    }
}

const REFUSED: Expect = Expect::Refused;
const WITH: Expect = Expect::Followed { credentials: true };
const WITHOUT: Expect = Expect::Followed { credentials: false };

#[tokio::test]
async fn a_definition_that_sets_nothing_behaves_as_same_origin() {
    matrix_row("default", |_| None, WITH, REFUSED, REFUSED).await;
}

#[tokio::test]
async fn same_origin_follows_only_redirects_to_the_base_origin() {
    matrix_row("same_origin", |_| Some(OriginPolicy::same_origin()), WITH, REFUSED, REFUSED).await;
}

#[tokio::test]
async fn allow_origins_follows_listed_origins_with_the_credential_and_refuses_the_rest() {
    matrix_row("allow_origins", |o| Some(OriginPolicy::allow_origins([o.b.uri()])), WITH, WITH, REFUSED).await;
}

#[tokio::test]
async fn allow_without_credentials_follows_anywhere_and_sends_the_credential_only_to_the_base() {
    matrix_row("allow_without_credentials", |_| Some(OriginPolicy::allow_without_credentials()), WITH, WITHOUT, WITHOUT)
        .await;
}

#[tokio::test]
async fn danger_allow_any_follows_anywhere_with_the_credential() {
    matrix_row("danger_allow_any", |_| Some(OriginPolicy::danger_allow_any()), WITH, WITH, WITH).await;
}

#[tokio::test]
async fn deny_follows_no_redirect_at_all() {
    matrix_row("deny", |_| Some(OriginPolicy::deny()), REFUSED, REFUSED, REFUSED).await;
}

#[tokio::test]
async fn a_chain_is_checked_again_at_every_hop_and_reports_the_refused_hop() {
    let origins = origins().await;
    mount(&origins.a, "/chain", redirect(302, "/hop")).await;
    mount(&origins.a, "/hop", redirect(302, format!("{}/landing", origins.b.uri()))).await;
    let client = client(&origins.a, Form::Header, Some(OriginPolicy::same_origin()));

    let response = get(&client, "/chain").await;

    assert_eq!(refusal(&response), Some(RedirectRefusal::CrossOrigin));
    assert!(response.meta.request.url.as_str().ends_with("/hop"), "the refused 3xx's own request: {}", response.meta.request.url);
    assert_eq!(response.meta.headers["location"], format!("{}/landing", origins.b.uri()));
    assert!(requests(&origins.b).await.is_empty());
    for at in ["/chain", "/hop"] {
        assert!(carries(&requests_to(&origins.a, at).await[0], SECRET), "{at} is on the base origin");
    }
}

#[tokio::test]
async fn a_foreign_hop_stays_anonymous_until_the_chain_returns_to_the_base_origin() {
    for form in FORMS {
        let origins = origins().await;
        let (a, b) = (&origins.a, &origins.b);
        mount(a, "/out", redirect(302, format!("{}/b1", b.uri()))).await;
        mount(b, "/b1", redirect(302, "/b2")).await;
        mount(b, "/b2", redirect(307, format!("{}/back", a.uri()))).await;
        mount(a, "/back", json(9)).await;
        let client = client(a, form, Some(OriginPolicy::allow_without_credentials()));

        let response = get(&client, "/out").await;

        assert_eq!(response.data(), Some(&Item { id: 9 }), "{form:?}");
        let foreign = requests(b).await;
        assert_eq!(foreign.len(), 2, "{form:?}");
        assert!(foreign.iter().all(|request| !carries(request, SECRET)), "{form:?}: B -> B stays anonymous");
        assert!(carries(&requests_to(a, "/back").await[0], SECRET), "{form:?}: B -> A restores the credential");
        assert_eq!(response.meta.request.auth, AuthMethod::ClientLevel);
        assert_eq!(url::Url::parse(response.meta.request.url.as_str()).expect("url").path(), "/back", "{form:?}");
    }
}

#[tokio::test]
async fn a_foreign_hop_drops_every_secret_the_application_set_and_a_same_origin_hop_keeps_them() {
    let origins = origins().await;
    let (a, b) = (&origins.a, &origins.b);
    mount(a, "/out", redirect(302, format!("{}/landing?token={SENTINEL}&page=2", b.uri()))).await;
    let definition = definition(Form::Header).secret_names(["X-Session"]).cross_origin(OriginPolicy::allow_without_credentials());
    let client = builder(definition, &a.uri()).credentials(Form::Header.credential()).build().expect("client");
    let request = |at: &str| {
        let mut parts = RequestParts::new(Method::GET, at);
        parts.headers = vec![
            ("Cookie".into(), format!("pref=dark; session={SENTINEL}")),
            ("Proxy-Authorization".into(), format!("Basic {SENTINEL}")),
            ("X-Session".into(), SENTINEL.into()),
            ("X-Plain".into(), "kept".into()),
        ];
        Call::<Json<Item>>::new(parts)
    };

    let foreign = client.send(request("/out"), &CallOptions::new()).await.expect("head");
    let same = client.send(request("/same"), &CallOptions::new()).await.expect("head");

    assert_eq!(foreign.data(), Some(&Item { id: 2 }));
    let landed = &requests(b).await[0];
    for secret in [SECRET, SENTINEL] {
        assert!(!carries(landed, secret), "{secret} reached the foreign origin: {landed:?}");
    }
    for header in ["authorization", "cookie", "proxy-authorization", "x-session"] {
        assert!(!landed.headers.contains_key(header), "{header} reached the foreign origin");
    }
    assert_eq!(landed.headers["x-plain"], "kept");
    assert_eq!(landed.url.query(), Some("page=2"));

    assert_eq!(same.data(), Some(&Item { id: 1 }));
    let kept = &requests_to(a, "/final").await[0];
    for header in ["authorization", "cookie", "proxy-authorization", "x-session", "x-plain"] {
        assert!(kept.headers.contains_key(header), "{header} is kept on a same-origin hop");
    }
}

#[tokio::test]
async fn check_link_applies_the_policy_with_no_request() {
    let origins = origins().await;
    let (a, b, c) = (origins.a.uri(), origins.b.uri(), origins.c.uri());
    let link = |base: &str| url::Url::parse(&format!("{base}/items?token={SENTINEL}&page=2")).expect("link");
    // (policy, B allowed, C allowed)
    let rows = [
        ("same_origin", OriginPolicy::same_origin(), false, false),
        ("allow_origins", OriginPolicy::allow_origins([b.clone()]), true, false),
        ("allow_without_credentials", OriginPolicy::allow_without_credentials(), true, true),
        ("danger_allow_any", OriginPolicy::danger_allow_any(), true, true),
        ("deny", OriginPolicy::deny(), false, false),
    ];
    for (label, policy, b_allowed, c_allowed) in rows {
        let client = client(&origins.a, Form::Query, Some(policy));
        assert!(client.check_link::<Call<Json<Item>>>(&link(&a)).is_ok(), "{label}: a same-origin link passes");
        for (target, allowed) in [(&b, b_allowed), (&c, c_allowed)] {
            let result = client.check_link::<Call<Json<Item>>>(&link(target));
            assert_eq!(result.is_ok(), allowed, "{label}: {target}");
            if let Err(error) = result {
                let TransportErrorKind::CrossOriginRejected { target: refused } = &error.kind else {
                    panic!("{label}: {:?}", error.kind);
                };
                assert_eq!(refused.as_str(), format!("{target}/items?token=[redacted]&page=2"));
                assert_eq!(error.request.endpoint_id, "call");
                assert!(!error.to_string().contains(SENTINEL), "{label}: {error}");
            }
        }
        let unsafe_links = ["ftp://127.0.0.1/x".to_string(), a.replace("http://", "http://user:pw@")];
        for unsafe_link in unsafe_links {
            let unsafe_link = url::Url::parse(&unsafe_link).expect("url");
            assert!(client.check_link::<Call<Json<Item>>>(&unsafe_link).is_err(), "{label}: {unsafe_link}");
        }
    }
    for server in [&origins.a, &origins.b, &origins.c] {
        assert!(requests(server).await.is_empty(), "a link check sends nothing");
    }
}

#[tokio::test]
async fn an_absolute_request_url_is_checked_before_it_is_sent() {
    let origins = origins().await;
    let (a, b) = (&origins.a, &origins.b);
    let foreign = format!("{}/landing?token={SENTINEL}", b.uri());

    let strict = client(a, Form::Header, Some(OriginPolicy::same_origin()));
    let error = strict.send(Call::<Json<Item>>::get(&foreign), &CallOptions::new()).await.expect_err("refused before sending");
    assert!(matches!(error.kind, TransportErrorKind::CrossOriginRejected { .. }), "{:?}", error.kind);
    assert!(!error.to_string().contains(SENTINEL), "{error}");
    assert!(requests(b).await.is_empty());

    let loose = client(a, Form::Header, Some(OriginPolicy::allow_without_credentials()));
    let response = loose.send(Call::<Json<Item>>::get(&foreign), &CallOptions::new()).await.expect("sent");
    assert_eq!(response.data(), Some(&Item { id: 2 }));
    assert_eq!(response.meta.request.auth, AuthMethod::Anonymous);
    assert!(!carries(&requests(b).await[0], SECRET));

    let deny = client(a, Form::Header, Some(OriginPolicy::deny()));
    let own = format!("{}/final", a.uri());
    let response = deny.send(Call::<Json<Item>>::get(&own), &CallOptions::new()).await.expect("sent");
    assert_eq!(response.data(), Some(&Item { id: 1 }), "a same-origin absolute URL passes under deny");
    assert!(carries(&requests_to(a, "/final").await[0], SECRET));
}

/// One redirect status and request method, and what the next hop must
/// receive.
struct MechanicsCase {
    status: u16,
    method: Method,
    next_method: Method,
    body_kept: bool,
}

#[tokio::test]
async fn each_redirect_status_rewrites_or_replays_the_request() {
    let cases = [
        MechanicsCase { status: 301, method: Method::POST, next_method: Method::GET, body_kept: false },
        MechanicsCase { status: 302, method: Method::POST, next_method: Method::GET, body_kept: false },
        MechanicsCase { status: 301, method: Method::PUT, next_method: Method::PUT, body_kept: true },
        MechanicsCase { status: 303, method: Method::POST, next_method: Method::GET, body_kept: false },
        MechanicsCase { status: 303, method: Method::PUT, next_method: Method::GET, body_kept: false },
        MechanicsCase { status: 303, method: Method::HEAD, next_method: Method::HEAD, body_kept: false },
        MechanicsCase { status: 307, method: Method::POST, next_method: Method::POST, body_kept: true },
        MechanicsCase { status: 308, method: Method::POST, next_method: Method::POST, body_kept: true },
    ];
    for case in cases {
        let server = MockServer::start().await;
        mount(&server, "/from", redirect(case.status, "/to")).await;
        mount(&server, "/to", ResponseTemplate::new(204)).await;
        let client = builder(ApiDefinition::new("https://unused.invalid"), &server.uri()).build().expect("client");
        let mut parts = RequestParts::new(case.method.clone(), "/from");
        parts.headers = vec![("Content-Language".into(), "en".into())];
        if case.method != Method::HEAD {
            parts.body = RequestBody::Json(r#"{"a":1}"#.into());
        }

        let response = client.send(Call::<Empty>::new(parts), &CallOptions::new()).await.expect("head");

        let label = format!("{} {}", case.status, case.method);
        assert_eq!(response.meta.status, 204, "{label}");
        assert_eq!(response.meta.request.method, case.next_method, "{label}");
        let landed = &requests_to(&server, "/to").await[0];
        assert_eq!(landed.method, case.next_method, "{label}");
        let expected_body: &[u8] = if case.body_kept { br#"{"a":1}"# } else { b"" };
        assert_eq!(landed.body, expected_body, "{label}");
        assert_eq!(landed.headers.contains_key("content-type"), case.body_kept, "{label}");
        let rewritten = case.next_method != case.method;
        assert_eq!(landed.headers.contains_key("content-language"), !rewritten, "{label}: body headers go with the body");
    }
}

#[tokio::test]
async fn a_redirect_without_a_usable_location_is_refused_with_its_evidence() {
    let server = MockServer::start().await;
    let unsafe_user = server.uri().replace("http://", "http://user:pw@");
    let cases: [(&str, Option<&str>, RedirectRefusal); 4] = [
        ("/missing", None, RedirectRefusal::MissingLocation),
        ("/invalid", Some("http://[::1"), RedirectRefusal::InvalidLocation),
        ("/scheme", Some("ftp://127.0.0.1/x"), RedirectRefusal::UnsafeLocation),
        ("/user", Some(&unsafe_user), RedirectRefusal::UnsafeLocation),
    ];
    for (at, location, _) in &cases {
        let mut response = ResponseTemplate::new(302).set_body_raw("moved", "text/plain");
        if let Some(location) = location {
            response = response.insert_header("Location", *location);
        }
        mount(&server, at, response).await;
    }
    let client = builder(ApiDefinition::new("https://unused.invalid").cross_origin(OriginPolicy::danger_allow_any()), &server.uri())
        .build()
        .expect("client");

    for (at, location, reason) in cases {
        let response = get(&client, at).await;
        assert_eq!(refusal(&response), Some(reason), "{at}");
        assert_eq!(response.meta.status, 302, "{at}");
        assert_eq!(response.meta.headers.contains_key("location"), location.is_some(), "{at}");
        assert_eq!(response.error().expect("failure").body, Bytes::from_static(b"moved"), "{at}");
    }
    assert_eq!(requests(&server).await.len(), 4, "nothing beyond the four redirects was requested");
}

#[tokio::test]
async fn a_relative_location_resolves_against_the_hop_without_its_credential_or_fragment() {
    let server = MockServer::start().await;
    mount(&server, "/dir/start", redirect(302, "../next?x=1#frag")).await;
    Mock::given(path("/next")).and(query_param("page", "2")).respond_with(json(5)).with_priority(1).mount(&server).await;
    mount(&server, "/next", redirect(302, "?page=2")).await;
    let client = client(&server, Form::Query, None);

    let response = get(&client, "/dir/start").await;

    assert_eq!(response.data(), Some(&Item { id: 5 }), "{response}");
    let hops = requests(&server).await;
    let queries: Vec<_> = hops.iter().map(|request| (request.url.path().to_string(), request.url.query().map(str::to_string))).collect();
    assert_eq!(
        queries,
        [
            ("/dir/start".to_string(), Some(format!("api_key={SECRET}"))),
            ("/next".to_string(), Some(format!("x=1&api_key={SECRET}"))),
            ("/next".to_string(), Some(format!("page=2&api_key={SECRET}"))),
        ],
        "the credential is placed once per hop and never inherited from the previous URL"
    );
    assert!(!response.meta.request.url.as_str().contains('#'));
}

/// `/hop/{n}` redirects to `/hop/{n + 1}` until `last`, which answers 200.
struct Hops {
    last: usize,
}

impl Respond for Hops {
    fn respond(&self, request: &Request) -> ResponseTemplate {
        let n: usize = request.url.path().trim_start_matches("/hop/").parse().expect("hop number");
        if n == self.last { json(n as u64) } else { redirect(302, format!("/hop/{}", n + 1)) }
    }
}

#[tokio::test]
async fn ten_redirects_are_followed_and_the_eleventh_is_refused() {
    for (last, followed) in [(MAX_REDIRECTS, true), (MAX_REDIRECTS + 1, false)] {
        let server = MockServer::start().await;
        Mock::given(path_regex("^/hop/[0-9]+$")).respond_with(Hops { last }).mount(&server).await;
        let client = builder(ApiDefinition::new("https://unused.invalid"), &server.uri()).build().expect("client");

        let response = get(&client, "/hop/0").await;

        assert_eq!(requests(&server).await.len(), MAX_REDIRECTS + 1, "last = {last}");
        if followed {
            assert_eq!(response.data(), Some(&Item { id: MAX_REDIRECTS as u64 }));
        } else {
            assert_eq!(refusal(&response), Some(RedirectRefusal::HopLimit));
            assert_eq!(response.meta.status, 302);
            assert!(response.meta.request.url.as_str().ends_with(&format!("/hop/{MAX_REDIRECTS}")));
        }
    }
}

#[tokio::test]
async fn an_explicit_host_and_a_redirects_cookie_never_reach_the_next_hop() {
    let origins = origins().await;
    let (a, b) = (&origins.a, &origins.b);
    mount(a, "/set", redirect(302, "/final").insert_header("Set-Cookie", "session=abc")).await;
    let client = builder(ApiDefinition::new("https://unused.invalid").cross_origin(OriginPolicy::danger_allow_any()), &a.uri())
        .build()
        .expect("client");
    let with_host = |at: &str| {
        let mut parts = RequestParts::new(Method::GET, at);
        parts.headers = vec![("Host".into(), "a.example.test".into())];
        Call::<Json<Item>>::new(parts)
    };

    client.send(with_host("/cross"), &CallOptions::new()).await.expect("head");
    get(&client, "/set").await;

    assert_eq!(requests_to(a, "/cross").await[0].headers["host"], "a.example.test", "the first hop sends the caller's Host");
    let b_host = b.uri().trim_start_matches("http://").to_string();
    assert_eq!(requests(b).await[0].headers["host"], b_host.as_str(), "the next hop names its own host");
    assert!(!requests_to(a, "/final").await[0].headers.contains_key("cookie"), "no cookie jar");
}

#[tokio::test]
async fn a_followed_redirect_whose_target_refuses_the_connection_keeps_the_redirect() {
    let closed = closed_port_url().await;
    let a = MockServer::start().await;
    mount(&a, "/start", redirect(302, format!("{closed}/x?token={SENTINEL}")).set_body_raw("moving", "text/plain")).await;
    let follow = builder(ApiDefinition::new("https://unused.invalid").cross_origin(OriginPolicy::allow_origins([closed.clone()])), &a.uri())
        .build()
        .expect("client");
    let refuse = builder(ApiDefinition::new("https://unused.invalid"), &a.uri()).build().expect("client");
    let deadline = Deadline::after(Duration::from_secs(30));

    let failed = follow.send(Call::<Json<Item>>::get("/start"), &CallOptions::new().deadline(&deadline)).await.expect("a head arrived");
    let refused = refuse.send(Call::<Json<Item>>::get("/start"), &CallOptions::new()).await.expect("a head arrived");

    let error = failed.error().expect("failure");
    let ErrorKind::RedirectFailed { target, category } = &error.kind else {
        panic!("{:?}", error.kind);
    };
    assert_eq!(*category, FailureCategory::Connect);
    assert_eq!(target.as_str(), format!("{closed}/x?token=[redacted]"));
    assert_eq!(failed.meta.status, 302);
    assert_eq!(failed.meta.headers["location"], format!("{closed}/x?token=[redacted]"));
    assert!(failed.meta.request.url.as_str().ends_with("/start"), "the metadata describes the last received response");
    assert_eq!(error.body, Bytes::from_static(b"moving"));
    assert!(!failed.to_string().contains(SENTINEL), "{failed}");
    assert_eq!(deadline.completed_requests(), 1, "the logical send counts once");

    assert_eq!(refusal(&refused), Some(RedirectRefusal::CrossOrigin), "a policy refusal of the same 302 stays a refusal");
    assert_eq!(refused.meta.status, 302);
}

/// A, which answers its 302 after `a_delay`, and B, whose landing answers
/// after `b_delay`, with a client that follows A to B.
async fn delayed_pair(a_delay: Duration, b_delay: Duration) -> (MockServer, MockServer, Client<TestApi>, Duration) {
    let (a, b) = (MockServer::start().await, MockServer::start().await);
    mount(&a, "/start", redirect(302, format!("{}/landing", b.uri())).set_delay(a_delay)).await;
    mount(&b, "/landing", json(2).set_delay(b_delay)).await;
    let timeout = Duration::from_millis(700);
    let client = builder(ApiDefinition::new("https://unused.invalid").cross_origin(OriginPolicy::danger_allow_any()), &a.uri())
        .timeout(timeout)
        .build()
        .expect("client");
    (a, b, client, timeout)
}

#[tokio::test]
async fn redirect_hops_share_one_expiration_and_never_restart_it() {
    // Each hop alone fits inside the timeout; together they do not.
    let (_a, b, client, timeout) = delayed_pair(Duration::from_millis(200), Duration::from_millis(600)).await;

    let response = get(&client, "/start").await;

    let error = response.error().expect("failure");
    assert!(
        matches!(error.kind, ErrorKind::RedirectFailed { category: FailureCategory::Timeout, .. }),
        "{:?}",
        error.kind
    );
    assert_eq!(response.meta.status, 302);
    assert!(response.meta.elapsed >= timeout, "elapsed covers the whole send: {:?}", response.meta.elapsed);
    assert_eq!(requests(&b).await.len(), 1);
}

#[tokio::test]
async fn a_shared_deadline_expiring_between_hops_is_a_failed_redirect_counted_once() {
    let (_a, _b, client, _) = delayed_pair(Duration::from_millis(100), Duration::from_secs(5)).await;
    let deadline = Deadline::after(Duration::from_millis(400));

    let response = client.send(Call::<Json<Item>>::get("/start"), &CallOptions::new().deadline(&deadline)).await.expect("head");

    let error = response.error().expect("failure");
    assert!(
        matches!(error.kind, ErrorKind::RedirectFailed { category: FailureCategory::DeadlineExceeded, .. }),
        "{:?}",
        error.kind
    );
    assert!(response.meta.elapsed < Duration::from_secs(4), "the deadline, not B's delay, ended the send");
    assert_eq!(deadline.completed_requests(), 1);
}

#[tokio::test]
async fn a_followed_send_reports_the_final_hop_and_the_whole_elapsed_time() {
    let (a, b, client, _) = delayed_pair(Duration::from_millis(100), Duration::from_millis(100)).await;
    let deadline = Deadline::after(Duration::from_secs(30));

    let response = client.send(Call::<Json<Item>>::get("/start"), &CallOptions::new().deadline(&deadline)).await.expect("head");

    assert_eq!(response.data(), Some(&Item { id: 2 }));
    assert_eq!(response.meta.request.url.as_str(), format!("{}/landing", b.uri()));
    assert!(response.meta.elapsed >= Duration::from_millis(200), "{:?}", response.meta.elapsed);
    assert_eq!(deadline.completed_requests(), 1, "redirect hops are not counted");
    assert_eq!(requests(&a).await.len(), 1);
}

#[tokio::test]
async fn a_redirect_whose_body_is_cut_off_is_an_interrupted_body() {
    let (socket, url): (TcpListener, String) = listener().await;
    let server = tokio::spawn(async move {
        let (mut stream, _) = socket.accept().await.expect("accept");
        read_head(&mut stream).await;
        write_all(&mut stream, b"HTTP/1.1 302 Found\r\nLocation: /next\r\nContent-Length: 100\r\n\r\nshort").await;
    });
    let client = builder(ApiDefinition::new("https://unused.invalid"), &url).build().expect("client");

    let response = get(&client, "/start").await;
    server.await.expect("server task");

    assert_eq!(response.meta.status, 302);
    assert!(response.is_interrupted(), "{response}");
    assert_eq!(response.error().expect("failure").body, Bytes::from_static(b"short"));
}

/// Builds a client for a definition and base URL, given a plain and a
/// configured shared transport.
type Construct = fn(ApiDefinition, &str, &SharedTransport, &SharedTransport) -> Client<TestApi>;

#[tokio::test]
async fn no_constructor_lets_the_http_library_follow_a_redirect() {
    let origins = origins().await;
    let shared = SharedTransport::builder().build().expect("transport");
    let configured = SharedTransport::builder()
        .connect_timeout(Duration::from_secs(5))
        .danger_accept_invalid_certificates()
        .build()
        .expect("transport");
    let constructors: [(&str, Construct); 4] = [
        ("standalone", |d, url, _, _| builder(d, url).build().expect("client")),
        ("standalone with connection options", |d, url, _, _| {
            builder(d, url).connect_timeout(Duration::from_secs(5)).danger_accept_invalid_certificates().build().expect("client")
        }),
        ("shared transport", |d, url, shared, _| builder(d, url).transport(shared).build().expect("client")),
        ("configured shared transport", |d, url, _, configured| builder(d, url).transport(configured).build().expect("client")),
    ];

    for (label, construct) in constructors {
        let refusing = construct(ApiDefinition::new("https://unused.invalid"), &origins.a.uri(), &shared, &configured);
        let before = requests(&origins.b).await.len();
        let response = get(&refusing, "/cross").await;
        assert_eq!(refusal(&response), Some(RedirectRefusal::CrossOrigin), "{label}");
        assert_eq!(requests(&origins.b).await.len(), before, "{label}: the target was never contacted");

        let following = construct(
            ApiDefinition::new("https://unused.invalid").cross_origin(OriginPolicy::danger_allow_any()),
            &origins.a.uri(),
            &shared,
            &configured,
        );
        assert_eq!(get(&following, "/cross").await.data(), Some(&Item { id: 2 }), "{label}");
        assert_eq!(requests(&origins.b).await.len(), before + 1, "{label}: followed once, by the runtime");
    }
}

#[tokio::test]
async fn a_cross_origin_override_can_only_tighten_the_definition() {
    let origins = origins().await;
    let (b, c) = (origins.b.uri(), origins.c.uri());
    let d = "http://127.0.0.1:1";
    let build = |definition: OriginPolicy, requested: OriginPolicy| {
        builder(ApiDefinition::new("https://unused.invalid").cross_origin(definition), &origins.a.uri())
            .cross_origin(requested)
            .build()
    };
    let refused = |definition: &'static str, requested: &'static str| BuildError::OriginPolicyNotPermitted { definition, requested };
    let rejected = [
        (OriginPolicy::deny(), OriginPolicy::same_origin(), refused("deny", "same_origin")),
        (OriginPolicy::same_origin(), OriginPolicy::allow_origins([b.clone()]), refused("same_origin", "allow_origins")),
        (OriginPolicy::allow_origins([b.clone(), c.clone()]), OriginPolicy::allow_origins([b.clone(), d.into()]), refused("allow_origins", "allow_origins")),
        (OriginPolicy::allow_origins([b.clone(), c.clone()]), OriginPolicy::allow_origins([d]), refused("allow_origins", "allow_origins")),
        (OriginPolicy::allow_without_credentials(), OriginPolicy::allow_without_credentials(), refused("allow_without_credentials", "allow_without_credentials")),
        (OriginPolicy::danger_allow_any(), OriginPolicy::danger_allow_any(), refused("danger_allow_any", "danger_allow_any")),
        (OriginPolicy::same_origin(), OriginPolicy::allow_origins(["https://b.test/path"]), BuildError::InvalidAllowedOrigin { reason: "has a path" }),
    ];
    for (definition, requested, expected) in rejected {
        let label = format!("{} over {}", requested.name(), definition.name());
        assert_eq!(build(definition, requested).expect_err(&label), expected, "{label}");
    }
    let message = build(OriginPolicy::deny(), OriginPolicy::same_origin()).expect_err("refused").to_string();
    assert!(message.contains("same_origin") && message.contains("deny"), "{message}");
    let invalid = builder(ApiDefinition::new("https://unused.invalid").cross_origin(OriginPolicy::allow_origins(["b.test"])), &origins.a.uri()).build();
    assert!(matches!(invalid, Err(BuildError::InvalidAllowedOrigin { .. })), "an invalid definition entry fails the build");

    // Each accepted override takes effect.
    let deny = build(OriginPolicy::danger_allow_any(), OriginPolicy::deny()).expect("deny is always accepted");
    assert_eq!(refusal(&get(&deny, "/same").await), Some(RedirectRefusal::CrossOrigin));
    let subset = build(OriginPolicy::allow_origins([b.clone(), c.clone()]), OriginPolicy::allow_origins([b.clone()])).expect("subset");
    assert_eq!(get(&subset, "/cross").await.data(), Some(&Item { id: 2 }));
    assert_eq!(refusal(&get(&subset, "/to-c").await), Some(RedirectRefusal::CrossOrigin));
    let same = build(OriginPolicy::allow_without_credentials(), OriginPolicy::same_origin()).expect("same_origin");
    assert_eq!(refusal(&get(&same, "/cross").await), Some(RedirectRefusal::CrossOrigin));
    let same_over_same = build(OriginPolicy::same_origin(), OriginPolicy::same_origin()).expect("same_origin over itself");
    assert_eq!(get(&same_over_same, "/same").await.data(), Some(&Item { id: 1 }));

    assert_eq!(requests(&origins.c).await.len(), 0, "no rejected or refused form reached C");
    assert_eq!(requests(&origins.b).await.len(), 1, "only the subset client reached B");
}
