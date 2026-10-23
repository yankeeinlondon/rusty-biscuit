//! The mock WebSocket server's own tests: what it records, what each script
//! does to a real client, and that it shuts down deterministically.

// tungstenite's `Error` is large; every client call here returns it.
#![allow(clippy::result_large_err)]

mod ws_mock;

use std::time::Duration;

use futures_util::{SinkExt, StreamExt};
use tokio_tungstenite::tungstenite::client::IntoClientRequest;
use tokio_tungstenite::tungstenite::{Error, Message};
use ws_mock::{Barrier, FailSwitch, FailingStream, MockWsServer, Script, Step};

/// Generous against a loaded runner; every wait below ends on an event well
/// before it.
const WAIT: Duration = Duration::from_secs(5);

type Client = tokio_tungstenite::WebSocketStream<
    tokio_tungstenite::MaybeTlsStream<tokio::net::TcpStream>,
>;

async fn connect(url: &str) -> Result<Client, Error> {
    tokio_tungstenite::connect_async(url).await.map(|(ws, _)| ws)
}

fn status_of(result: Result<Client, Error>) -> u16 {
    match result {
        Err(Error::Http(response)) => response.status().as_u16(),
        Err(other) => panic!("expected an HTTP rejection, got {other}"),
        Ok(_) => panic!("expected an HTTP rejection, got an upgrade"),
    }
}

#[tokio::test]
async fn records_the_upgrade_path_query_and_headers() {
    let server = MockWsServer::start(vec![Script::accept()]).await;
    let mut request = format!("{}/v1/stream?b=2&a=1", server.ws_origin())
        .into_client_request()
        .unwrap();
    request.headers_mut().insert("X-Probe", "probe-value".parse().unwrap());
    let (_ws, _) = tokio_tungstenite::connect_async(request).await.unwrap();

    server.wait_for_requests(1, WAIT).await;
    let requests = server.requests();
    assert_eq!(requests.len(), 1);
    assert_eq!(requests[0].connection, 0);
    assert_eq!(requests[0].path, "/v1/stream");
    assert_eq!(requests[0].query.as_deref(), Some("b=2&a=1"));
    assert_eq!(requests[0].target(), "/v1/stream?b=2&a=1");
    assert_eq!(requests[0].header("x-probe"), Some("probe-value"));
    assert_eq!(requests[0].header("X-PROBE"), Some("probe-value"));
    assert_eq!(requests[0].header("authorization"), None);
    assert_eq!(server.connections(), 1);
}

#[tokio::test]
async fn sends_scripted_frames_in_order() {
    let server = MockWsServer::start(vec![Script::Accept(vec![
        Step::text("one"),
        Step::Send(Message::Binary(vec![2u8].into())),
        Step::Close,
    ])])
    .await;
    let mut ws = connect(&server.ws_origin()).await.unwrap();

    assert_eq!(ws.next().await.unwrap().unwrap(), Message::Text("one".into()));
    assert_eq!(ws.next().await.unwrap().unwrap(), Message::Binary(vec![2u8].into()));
    assert!(matches!(ws.next().await, Some(Ok(Message::Close(_))) | None));
}

#[tokio::test]
async fn drops_the_connection_after_n_received_frames() {
    let server = MockWsServer::start(vec![Script::Accept(vec![Step::Receive(2), Step::Drop])]).await;
    let mut ws = connect(&server.ws_origin()).await.unwrap();
    for text in ["a", "b", "c"] {
        // The third send may or may not fail locally; the drop is observed
        // on the read side.
        let _ = ws.send(Message::Text(text.into())).await;
    }

    let ended = tokio::time::timeout(WAIT, async {
        loop {
            match ws.next().await {
                None | Some(Err(_)) | Some(Ok(Message::Close(_))) => return,
                Some(Ok(_)) => {}
            }
        }
    })
    .await;
    assert!(ended.is_ok(), "the client must see the scripted drop");
    assert_eq!(
        server.frames(),
        [(0, Message::Text("a".into())), (0, Message::Text("b".into()))]
    );
}

#[tokio::test]
async fn refuses_and_redirects_and_counts_each_connection() {
    let server = MockWsServer::start(vec![
        Script::Refuse(403),
        Script::Redirect {
            status: 307,
            location: "ws://elsewhere.invalid/stream".to_string(),
        },
        Script::accept(),
    ])
    .await;

    assert_eq!(status_of(connect(&format!("{}/a", server.ws_origin())).await), 403);
    match connect(&format!("{}/b", server.ws_origin())).await {
        Err(Error::Http(response)) => {
            assert_eq!(response.status().as_u16(), 307);
            assert_eq!(
                response.headers().get("location").unwrap(),
                "ws://elsewhere.invalid/stream"
            );
        }
        other => panic!("expected a redirect, got {:?}", other.map(|_| ())),
    }
    let _ws = connect(&format!("{}/c", server.ws_origin())).await.unwrap();
    // Scripts run out: the last one repeats.
    let _again = connect(&format!("{}/d", server.ws_origin())).await.unwrap();

    server.wait_for_requests(4, WAIT).await;
    let paths: Vec<String> = server.requests().into_iter().map(|r| r.path).collect();
    assert_eq!(paths, ["/a", "/b", "/c", "/d"]);
    assert_eq!(server.connections(), 4);
}

#[tokio::test]
async fn refuses_with_scripted_response_headers_in_order() {
    let server = MockWsServer::start(vec![Script::RefuseWith {
        status: 401,
        headers: vec![
            ("Set-Cookie".to_string(), "a=1".to_string()),
            ("Set-Cookie".to_string(), "b=2".to_string()),
            ("X-Reason".to_string(), "expired".to_string()),
        ],
    }])
    .await;

    match connect(&server.ws_origin()).await {
        Err(Error::Http(response)) => {
            assert_eq!(response.status().as_u16(), 401);
            let cookies: Vec<_> = response.headers().get_all("set-cookie").iter().collect();
            assert_eq!(cookies, ["a=1", "b=2"]);
            assert_eq!(response.headers().get("x-reason").unwrap(), "expired");
        }
        other => panic!("expected a 401 rejection, got {:?}", other.map(|_| ())),
    }
    server.wait_for_requests(1, WAIT).await;
    assert_eq!(server.connections(), 1);
}

#[tokio::test]
async fn a_dropped_tcp_connection_fails_the_handshake() {
    let server = MockWsServer::start(vec![Script::DropTcp]).await;
    let result = connect(&server.ws_origin()).await;
    assert!(result.is_err(), "a closed socket cannot upgrade");
    server.wait_for_connections(1, WAIT).await;
    assert!(server.requests().is_empty(), "the request was never read");
}

#[tokio::test]
async fn a_stalled_upgrade_never_answers() {
    let server = MockWsServer::start(vec![Script::Stall]).await;
    let attempt =
        tokio::time::timeout(Duration::from_millis(500), connect(&server.ws_origin())).await;
    assert!(attempt.is_err(), "the handshake must still be pending");
    assert_eq!(server.connections(), 1);
}

#[tokio::test]
async fn a_barrier_holds_a_script_until_released() {
    let barrier = Barrier::new();
    let server = MockWsServer::start(vec![Script::Accept(vec![
        Step::Wait(barrier.clone()),
        Step::text("after"),
    ])])
    .await;
    let mut ws = connect(&server.ws_origin()).await.unwrap();

    let early = tokio::time::timeout(Duration::from_millis(300), ws.next()).await;
    assert!(early.is_err(), "no frame may arrive before the barrier opens");
    barrier.release();
    let frame = tokio::time::timeout(WAIT, ws.next()).await.unwrap();
    assert_eq!(frame.unwrap().unwrap(), Message::Text("after".into()));
}

#[tokio::test]
async fn shutdown_ends_open_connections_and_closes_the_listener() {
    let server = MockWsServer::start(vec![Script::Accept(vec![Step::Stall])]).await;
    let addr = server.addr();
    let mut ws = connect(&server.ws_origin()).await.unwrap();

    server.shutdown().await;

    let ended = tokio::time::timeout(WAIT, async {
        loop {
            match ws.next().await {
                None | Some(Err(_)) | Some(Ok(Message::Close(_))) => return,
                Some(Ok(_)) => {}
            }
        }
    })
    .await;
    assert!(ended.is_ok(), "the open connection must end on shutdown");
    assert!(
        tokio::net::TcpStream::connect(addr).await.is_err(),
        "the listener must be closed once shutdown returns"
    );
}

#[tokio::test]
async fn concurrent_servers_bind_distinct_ephemeral_ports() {
    let servers =
        futures_util::future::join_all((0..8).map(|_| MockWsServer::start(vec![Script::accept()])))
            .await;
    let mut ports: Vec<u16> = servers.iter().map(|s| s.addr().port()).collect();
    assert!(servers.iter().all(|s| s.addr().ip().is_loopback()));
    ports.sort_unstable();
    ports.dedup();
    assert_eq!(ports.len(), servers.len(), "every server has its own port");

    let clients = futures_util::future::join_all(
        servers
            .iter()
            .enumerate()
            .map(|(index, server)| connect_owned(format!("{}/server-{index}", server.ws_origin()))),
    )
    .await;
    assert!(clients.iter().all(Result::is_ok));
    for (index, server) in servers.iter().enumerate() {
        server.wait_for_requests(1, WAIT).await;
        let paths: Vec<String> = server.requests().into_iter().map(|r| r.path).collect();
        assert_eq!(paths, [format!("/server-{index}")], "each server sees only its own client");
    }
}

async fn connect_owned(url: String) -> Result<Client, Error> {
    connect(&url).await
}

#[tokio::test]
async fn an_armed_failing_stream_fails_the_next_write() {
    let server = MockWsServer::start(vec![Script::Accept(vec![Step::Receive(1)])]).await;
    let tcp = tokio::net::TcpStream::connect(server.addr()).await.unwrap();
    let switch = FailSwitch::new();
    let (mut ws, _) = tokio_tungstenite::client_async(
        format!("{}/open", server.ws_origin()),
        FailingStream::new(tcp, switch.clone()),
    )
    .await
    .expect("the handshake writes before the switch is armed");

    ws.send(Message::Text("before".into())).await.unwrap();
    server.wait_for_frames(1, WAIT).await;
    switch.arm();
    match ws.send(Message::Text("after".into())).await {
        Err(Error::Io(err)) => assert_eq!(err.kind(), std::io::ErrorKind::BrokenPipe),
        other => panic!("expected the injected write failure, got {other:?}"),
    }
    assert_eq!(server.frames(), [(0, Message::Text("before".into()))]);
}
