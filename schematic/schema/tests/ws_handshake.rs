//! Handshake credentials, rejections, redirects, and redaction of generated
//! WebSocket clients, observed from a loopback mock server.

#![allow(clippy::result_large_err)]

mod common;
mod ws_mock;

use ws_mock::no_setup;

use std::time::{Duration, Instant};

use common::EnvGuard;
use schematic_runtime::{AuthMethod, Credential};
use schematic_schema::samsung_smart_tv_remote_ws::{
    RemoteControlConnectionParams, SamsungSmartTvRemoteWs,
};
use schematic_schema::unfolded_circle_core_ws::UnfoldedCircleCoreWs;
use schematic_schema::unfolded_circle_dock_ws::UnfoldedCircleDockWs;
use schematic_schema::ws_shared::{ReconnectPolicy, WsClientOptions, WsConnectionState, WsError};
use ws_mock::{Barrier, MockWsServer, RecordedRequest, Script, Step};

const SENTINEL: &str = "sentinel-7f3a";
/// Every Core resolver candidate, in resolver order.
const CORE_ENV: [&str; 2] = ["UCR_CORE_API_KEY", "UNFOLDED_CIRCLE_API_KEY"];
/// Generous against a loaded runner; every wait ends on an event well before it.
const WAIT: Duration = Duration::from_secs(3);

fn options() -> WsClientOptions {
    WsClientOptions::builder()
        .handshake_timeout(Duration::from_secs(2))
        .request_timeout(Duration::from_secs(2))
        .build()
}

fn reconnect() -> ReconnectPolicy {
    ReconnectPolicy {
        initial_backoff: Duration::from_millis(20),
        max_backoff: Duration::from_millis(40),
        multiplier: 1.0,
        jitter: 0.0,
        max_attempts: Some(5),
    }
}

/// Every value of header `name` (case-insensitive) the request carried.
fn header_values<'a>(request: &'a RecordedRequest, name: &str) -> Vec<&'a str> {
    let name = name.to_ascii_lowercase();
    request
        .headers
        .iter()
        .filter(|(key, _)| *key == name)
        .map(|(_, value)| value.as_str())
        .collect()
}

fn assert_no_sentinel(error: &WsError) {
    let display = error.to_string();
    let debug = format!("{error:?}");
    assert!(!display.contains(SENTINEL), "sentinel in Display: {display}");
    assert!(!debug.contains(SENTINEL), "sentinel in Debug: {debug}");
}

async fn wait_for_state<F: Fn() -> WsConnectionState>(state: F, expected: WsConnectionState) {
    let deadline = Instant::now() + WAIT;
    while state() != expected {
        assert!(Instant::now() < deadline, "state stayed {} instead of {expected}", state());
        tokio::time::sleep(Duration::from_millis(5)).await;
    }
}

// R5: precedence per-call > client-level > first non-empty env variable.
#[tokio::test]
#[serial_test::serial]
async fn core_credential_precedence_per_call_then_client_then_env() {
    let env = EnvGuard::clear(&CORE_ENV);
    env.set("UCR_CORE_API_KEY", "sentinel-env-first");
    env.set("UNFOLDED_CIRCLE_API_KEY", "sentinel-env-second");
    let server = MockWsServer::start(vec![Script::accept()]).await;
    let with_client = UnfoldedCircleCoreWs::with_base_url(server.ws_origin())
        .credentials(Credential::api_key("sentinel-client"));
    let per_call = WsClientOptions::builder()
        .handshake_timeout(Duration::from_secs(2))
        .credential(Credential::api_key("sentinel-per-call"))
        .build();

    let ws = with_client.connect_core_ws_with_setup(per_call, no_setup).await.unwrap();
    assert_eq!(ws.auth_method(), &AuthMethod::PerCall);
    let _ = ws.close().await;

    let ws = with_client.connect_core_ws_with_setup(options(), no_setup).await.unwrap();
    assert_eq!(ws.auth_method(), &AuthMethod::ClientLevel);
    let _ = ws.close().await;

    let anonymous_client = UnfoldedCircleCoreWs::with_base_url(server.ws_origin());
    let ws = anonymous_client.connect_core_ws_with_setup(options(), no_setup).await.unwrap();
    assert_eq!(ws.auth_method(), &AuthMethod::EnvVar("UCR_CORE_API_KEY".to_string()));
    let _ = ws.close().await;

    env.set("UCR_CORE_API_KEY", "");
    let ws = anonymous_client.connect_core_ws_with_setup(options(), no_setup).await.unwrap();
    assert_eq!(
        ws.auth_method(),
        &AuthMethod::EnvVar("UNFOLDED_CIRCLE_API_KEY".to_string())
    );
    let _ = ws.close().await;

    server.wait_for_requests(4, WAIT).await;
    let requests = server.requests();
    let sent: Vec<Vec<&str>> = requests.iter().map(|r| header_values(r, "api-key")).collect();
    assert_eq!(
        sent,
        [
            vec!["sentinel-per-call"],
            vec!["sentinel-client"],
            vec!["sentinel-env-first"],
            vec!["sentinel-env-second"],
        ]
    );
    assert!(requests.iter().all(|r| r.query.is_none()));
}

// R6: no credential anywhere is an anonymous upgrade, not an error.
#[tokio::test]
#[serial_test::serial]
async fn core_without_any_credential_connects_anonymously() {
    let _env = EnvGuard::clear(&CORE_ENV);
    let server = MockWsServer::start(vec![Script::accept()]).await;
    let ws = UnfoldedCircleCoreWs::with_base_url(server.ws_origin())
        .connect_core_ws_with_setup(options(), no_setup)
        .await
        .unwrap();
    assert_eq!(ws.auth_method(), &AuthMethod::Anonymous);
    let _ = ws.close().await;

    let requests = server.requests();
    assert_eq!(requests.len(), 1);
    assert!(header_values(&requests[0], "api-key").is_empty());
    assert_eq!(requests[0].query, None);
}

// R7: a 401/403 on the first upgrade is returned, never retried with
// another credential or anonymously.
#[tokio::test]
#[serial_test::serial]
async fn initial_401_and_403_are_returned_without_a_second_attempt() {
    let env = EnvGuard::clear(&CORE_ENV);
    env.set("UCR_CORE_API_KEY", "sentinel-env");
    for status in [401u16, 403] {
        let server = MockWsServer::start(vec![Script::Refuse(status), Script::accept()]).await;
        let options = WsClientOptions::builder()
            .handshake_timeout(Duration::from_secs(2))
            .reconnect(Some(reconnect()))
            .build();
        let error = UnfoldedCircleCoreWs::with_base_url(server.ws_origin())
            .api_key("sentinel-client")
            .connect_core_ws_with_setup(options, no_setup)
            .await
            .err()
            .expect("the upgrade is refused");
        match &error {
            WsError::HandshakeRejected { status: got, .. } => assert_eq!(*got, status),
            other => panic!("expected HandshakeRejected({status}), got {other:?}"),
        }
        assert!(error.is_terminal_handshake_rejection());

        tokio::time::sleep(Duration::from_millis(150)).await;
        assert_eq!(server.requests().len(), 1, "HTTP {status} was retried");
        assert_eq!(server.connections(), 1);
        assert_eq!(
            header_values(&server.requests()[0], "api-key"),
            ["sentinel-client"]
        );
        server.shutdown().await;
    }
}

// R5/R7: a redial answered with 401, 403, or a redirect is pushed as an
// event and closes the client; the reconnect policy is not consulted again.
/// A redial whose upgrade gets `rejection` pushes it as an event and closes
/// the client: two upgrades in total, however many backoffs pass.
async fn assert_redial_rejection_is_terminal(rejection: Script, status: u16) {
    let _env = EnvGuard::clear(&CORE_ENV);
    let server = MockWsServer::start(vec![
        Script::Accept(vec![Step::Close]),
        rejection,
        Script::accept(),
    ])
    .await;
    let options = WsClientOptions::builder()
        .handshake_timeout(Duration::from_secs(2))
        .reconnect(Some(reconnect()))
        .build();
    let mut ws = UnfoldedCircleCoreWs::with_base_url(server.ws_origin())
        .api_key("sentinel-client")
        .connect_core_ws_with_setup(options, no_setup)
        .await
        .unwrap();

    let event = tokio::time::timeout(WAIT, ws.next_event())
        .await
        .expect("an event arrives")
        .expect("the event queue is open");
    match event {
        Err(WsError::HandshakeRejected { status: got, .. }) if got == status => {}
        other => panic!("expected HandshakeRejected({status}), got {other:?}"),
    }
    wait_for_state(|| ws.state(), WsConnectionState::Closed).await;

    // Several backoffs (20-40 ms each) later, still no further attempt.
    tokio::time::sleep(Duration::from_millis(250)).await;
    assert_eq!(server.requests().len(), 2);
    assert_eq!(server.connections(), 2);
    assert!(matches!(
        tokio::time::timeout(WAIT, ws.next_event()).await,
        Ok(None)
    ));
}

#[tokio::test]
#[serial_test::serial]
async fn redial_401_is_terminal() {
    assert_redial_rejection_is_terminal(Script::Refuse(401), 401).await;
}

#[tokio::test]
#[serial_test::serial]
async fn redial_403_is_terminal() {
    assert_redial_rejection_is_terminal(Script::Refuse(403), 403).await;
}

#[tokio::test]
#[serial_test::serial]
async fn redial_redirect_is_terminal() {
    let location = "ws://127.0.0.1:9/elsewhere".to_string();
    assert_redial_rejection_is_terminal(Script::Redirect { status: 307, location }, 307).await;
}

// R5: the credential is selected once per connect; a redial reuses it even
// after the environment changes.
#[tokio::test]
#[serial_test::serial]
async fn redial_reuses_the_credential_selected_at_connect() {
    let env = EnvGuard::clear(&CORE_ENV);
    env.set("UCR_CORE_API_KEY", "sentinel-v1");
    let gate = Barrier::new();
    let server = MockWsServer::start(vec![
        Script::Accept(vec![Step::Wait(gate.clone()), Step::Close]),
        Script::accept(),
    ])
    .await;
    let options = WsClientOptions::builder()
        .handshake_timeout(Duration::from_secs(2))
        .reconnect(Some(reconnect()))
        .build();
    let ws = UnfoldedCircleCoreWs::with_base_url(server.ws_origin())
        .connect_core_ws_with_setup(options, no_setup)
        .await
        .unwrap();
    assert_eq!(ws.auth_method(), &AuthMethod::EnvVar("UCR_CORE_API_KEY".to_string()));

    env.set("UCR_CORE_API_KEY", "sentinel-v2");
    gate.release();
    server.wait_for_requests(2, WAIT).await;

    let requests = server.requests();
    assert_eq!(header_values(&requests[0], "api-key"), ["sentinel-v1"]);
    assert_eq!(header_values(&requests[1], "api-key"), ["sentinel-v1"]);
    let _ = ws.close().await;
}

// R5: an API whose policy is `None` never sends the token its definition
// documents, even when that variable is set.
#[tokio::test]
#[serial_test::serial]
async fn dock_without_auth_policy_ignores_its_env_token() {
    let env = EnvGuard::clear(&["UCR_DOCK_TOKEN"]);
    env.set("UCR_DOCK_TOKEN", SENTINEL);
    let server = MockWsServer::start(vec![Script::accept()]).await;
    let ws = UnfoldedCircleDockWs::with_base_url(server.ws_origin())
        .connect_dock_root_with_setup(options(), no_setup)
        .await
        .unwrap();
    assert_eq!(ws.auth_method(), &AuthMethod::Anonymous);
    let _ = ws.close().await;

    let requests = server.requests();
    assert_eq!(requests.len(), 1);
    assert_eq!(requests[0].query, None);
    assert!(
        requests[0].headers.iter().all(|(_, value)| !value.contains(SENTINEL)),
        "the Dock token was sent: {:?}",
        requests[0].headers
    );
}

// R7: a redirect is reported with its redacted Location and never followed.
#[tokio::test]
#[serial_test::serial]
async fn redirect_is_not_followed_and_its_location_is_redacted() {
    let _env = EnvGuard::clear(&CORE_ENV);
    let target = MockWsServer::start(vec![Script::accept()]).await;
    let server = MockWsServer::start(vec![Script::Redirect {
        status: 302,
        location: format!("{}/?token={SENTINEL}", target.ws_origin()),
    }])
    .await;

    let error = UnfoldedCircleCoreWs::with_base_url(server.ws_origin())
        .connect_core_ws_with_setup(options(), no_setup)
        .await
        .err()
        .expect("the redirect is refused");
    assert!(
        matches!(error, WsError::HandshakeRejected { status: 302, .. }),
        "{error:?}"
    );
    assert!(error.is_terminal_handshake_rejection());
    let location = error.location().expect("the Location is kept");
    assert_eq!(location, format!("{}/?token=[redacted]", target.ws_origin()));
    let display = error.to_string();
    assert!(display.contains("302"), "{display}");
    assert!(display.contains(location), "{display}");
    assert_no_sentinel(&error);

    tokio::time::sleep(Duration::from_millis(100)).await;
    assert_eq!(server.requests().len(), 1);
    assert_eq!(target.requests().len(), 0);
    assert_eq!(target.connections(), 0);
}

// R7: secret response headers of a refusal keep their names, never values.
#[tokio::test]
#[serial_test::serial]
async fn refusal_headers_redact_set_cookie_and_secret_values() {
    let _env = EnvGuard::clear(&CORE_ENV);
    let server = MockWsServer::start(vec![Script::RefuseWith {
        status: 401,
        headers: vec![
            ("Set-Cookie".to_string(), format!("session={SENTINEL}")),
            ("X-Auth-Token".to_string(), SENTINEL.to_string()),
            ("API-KEY".to_string(), SENTINEL.to_string()),
            ("X-Reason".to_string(), "expired".to_string()),
        ],
    }])
    .await;

    let error = UnfoldedCircleCoreWs::with_base_url(server.ws_origin())
        .connect_core_ws_with_setup(options(), no_setup)
        .await
        .err()
        .expect("the upgrade is refused");
    let WsError::HandshakeRejected { status, headers } = &error else {
        panic!("expected HandshakeRejected, got {error:?}");
    };
    assert_eq!(*status, 401);
    let value_of = |name: &str| {
        headers
            .iter()
            .find(|(key, _)| key == name)
            .map(|(_, value)| value.as_str())
    };
    assert_eq!(value_of("set-cookie"), Some("[redacted]"));
    assert_eq!(value_of("x-auth-token"), Some("[redacted]"));
    assert_eq!(value_of("api-key"), Some("[redacted]"));
    assert_eq!(value_of("x-reason"), Some("expired"));
    assert_eq!(error.location(), None);
    assert_no_sentinel(&error);
}

fn samsung_params() -> RemoteControlConnectionParams {
    RemoteControlConnectionParams {
        name: "dGVzdA==".to_string(),
        token: Some(SENTINEL.to_string()),
    }
}

// R7: a secret query value never reaches a dial error, whatever the failure.
#[tokio::test]
async fn query_secret_is_absent_from_every_dial_error() {
    // (a) connection refused: a port that was just free.
    let closed_port = {
        let listener = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
        listener.local_addr().unwrap().port()
    };
    // Windows retries the SYN for about 2 s before reporting a refused
    // loopback connect, so the attempt needs a budget well past that.
    let refused = SamsungSmartTvRemoteWs::with_base_url(format!("ws://127.0.0.1:{closed_port}"))
        .connect_remote_control(
            samsung_params(),
            WsClientOptions::builder()
                .handshake_timeout(Duration::from_secs(10))
                .build(),
        )
        .await
        .err()
        .expect("nothing listens on the port");
    assert!(matches!(refused, WsError::Handshake(_)), "{refused:?}");
    assert_no_sentinel(&refused);

    // (b) the server never answers the upgrade.
    let stall = MockWsServer::start(vec![Script::Stall]).await;
    let started = Instant::now();
    let timed_out = SamsungSmartTvRemoteWs::with_base_url(stall.ws_origin())
        .connect_remote_control(
            samsung_params(),
            WsClientOptions::builder()
                .handshake_timeout(Duration::from_millis(200))
                .build(),
        )
        .await
        .err()
        .expect("the upgrade stalls");
    assert!(matches!(timed_out, WsError::HandshakeTimeout(_)), "{timed_out:?}");
    assert!(started.elapsed() < Duration::from_secs(2));
    assert_no_sentinel(&timed_out);

    // (c) a redirect that echoes the token back.
    let redirect = MockWsServer::start(vec![Script::Redirect {
        status: 302,
        location: format!("/elsewhere?token={SENTINEL}"),
    }])
    .await;
    let redirected = SamsungSmartTvRemoteWs::with_base_url(redirect.ws_origin())
        .connect_remote_control(samsung_params(), options())
        .await
        .err()
        .expect("the redirect is refused");
    assert!(
        matches!(redirected, WsError::HandshakeRejected { status: 302, .. }),
        "{redirected:?}"
    );
    assert_eq!(redirected.location(), Some("/elsewhere?token=[redacted]"));
    assert_no_sentinel(&redirected);
    // The secret did go on the wire, so the redaction above is meaningful.
    assert!(redirect.requests()[0].target().contains(SENTINEL));

    let with_credential = WsClientOptions::builder()
        .credential(Credential::api_key(SENTINEL))
        .build();
    let debug = format!("{with_credential:?}");
    assert!(!debug.contains(SENTINEL), "{debug}");
}

// R5/R7: headers the upgrade manages or that carry the credential, and a
// credential kind the API does not accept, fail before any request.
#[tokio::test]
#[serial_test::serial]
async fn invalid_headers_and_credential_kinds_fail_before_dialing() {
    let _env = EnvGuard::clear(&CORE_ENV);
    let server = MockWsServer::start(vec![Script::accept()]).await;
    let client = || UnfoldedCircleCoreWs::with_base_url(server.ws_origin());

    for name in ["Sec-WebSocket-Key", "Host", "Upgrade", "API-KEY", "api-key"] {
        let error = client()
            .header(name, "x")
            .connect_core_ws_with_setup(options(), no_setup)
            .await
            .err()
            .unwrap_or_else(|| panic!("header {name} was accepted"));
        match &error {
            WsError::InvalidHeader { name: got, .. } => assert_eq!(got, name),
            other => panic!("expected InvalidHeader for {name}, got {other:?}"),
        }
    }

    let basic = WsClientOptions::builder()
        .credential(Credential::basic("user", SENTINEL))
        .build();
    let error = client().connect_core_ws_with_setup(basic, no_setup).await.err().expect("Basic is refused");
    assert!(matches!(error, WsError::InvalidCredential(_)), "{error:?}");
    assert_no_sentinel(&error);

    let error = client()
        .credentials(Credential::basic("user", SENTINEL))
        .connect_core_ws_with_setup(options(), no_setup)
        .await
        .err()
        .expect("a client-level Basic is refused");
    assert!(matches!(error, WsError::InvalidCredential(_)), "{error:?}");

    tokio::time::sleep(Duration::from_millis(50)).await;
    assert_eq!(server.requests().len(), 0);
    assert_eq!(server.connections(), 0);
}
