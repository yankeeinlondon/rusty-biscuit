//! Fixtures shared by the managed-send suites: API tags, a configurable
//! endpoint, a declared error body, and raw-socket mock helpers.
//!
//! Raw-socket mocks cover what wiremock cannot: a cut-off body, a stalled
//! head, connection counting, and proxy routing.

use std::future::Future;
use std::marker::PhantomData;
use std::net::SocketAddr;

use http::Method;
use schematic_runtime::{
    Api, ApiDefinition, Client, Decode, EndpointSpec, ErrorBody, InvalidArgument, NoDeclaredError,
    RequestParts,
};
use serde::Deserialize;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::{TcpListener, TcpStream};
use tokio::task::JoinHandle;

/// A value that must never appear in stored metadata or `Display` text.
pub const SENTINEL: &str = "SENTINEL-7f3a9c";

/// The API every fixture client is built for.
pub struct TestApi;

impl Api for TestApi {
    const NAME: &'static str = "Test";
}

/// A JSON success body.
#[derive(Debug, Clone, PartialEq, Deserialize)]
pub struct Item {
    pub id: u64,
}

/// A declared JSON error body.
#[derive(Debug, Clone, PartialEq, Deserialize)]
pub struct ProviderError {
    pub message: String,
}

impl ErrorBody for ProviderError {
    fn decode_body(_: usize, body: &[u8]) -> Option<Self> {
        serde_json::from_slice(body).ok()
    }
}

/// An endpoint of [`TestApi`] decoded by `D`, failing with `E`, that sends
/// whatever parts it was built with.
pub struct Call<D, E = NoDeclaredError> {
    parts: Result<RequestParts, InvalidArgument>,
    codec: PhantomData<fn() -> (D, E)>,
}

impl<D, E> Call<D, E> {
    pub fn new(parts: RequestParts) -> Self {
        Self {
            parts: Ok(parts),
            codec: PhantomData,
        }
    }

    pub fn get(path: &str) -> Self {
        Self::new(RequestParts::new(Method::GET, path))
    }

    /// An endpoint whose parts cannot be assembled.
    pub fn invalid(reason: &'static str) -> Self {
        Self {
            parts: Err(InvalidArgument::new(reason)),
            codec: PhantomData,
        }
    }
}

// By hand: a derive would require the same of `D` and `E`.
impl<D, E> std::fmt::Debug for Call<D, E> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Call").field("parts", &self.parts).finish()
    }
}

impl<D, E> Clone for Call<D, E> {
    fn clone(&self) -> Self {
        Self {
            parts: self.parts.clone(),
            codec: PhantomData,
        }
    }
}

impl<D, E> EndpointSpec for Call<D, E>
where
    D: Decode,
    E: ErrorBody,
{
    type Api = TestApi;
    type Success = D::Output;
    type Failure = E;
    type Decoder = D;

    const ENDPOINT_ID: &'static str = "call";
    const METHOD: Method = Method::GET;

    fn into_parts(self) -> Result<RequestParts, InvalidArgument> {
        self.parts
    }
}

/// A builder for a [`TestApi`] client of `definition`, pointed at `base_url`.
pub fn builder(definition: ApiDefinition, base_url: &str) -> Client<TestApi> {
    Client::<TestApi>::new(definition).base_url(base_url)
}

/// A client with no authentication pointed at `base_url`.
pub fn plain_client(base_url: &str) -> Client<TestApi> {
    builder(ApiDefinition::new("https://api.example.invalid"), base_url)
        .build()
        .expect("client builds")
}

/// A listener on an ephemeral loopback port and its `http://` base URL.
pub async fn listener() -> (TcpListener, String) {
    let listener = TcpListener::bind("127.0.0.1:0").await.expect("bind loopback");
    let address: SocketAddr = listener.local_addr().expect("local address");
    (listener, format!("http://{address}"))
}

/// Reads one request head (through the blank line) from `stream`.
pub async fn read_head(stream: &mut TcpStream) -> String {
    let mut head = Vec::new();
    let mut byte = [0_u8; 1];
    while !head.ends_with(b"\r\n\r\n") {
        match stream.read(&mut byte).await {
            Ok(0) | Err(_) => break,
            Ok(_) => head.push(byte[0]),
        }
    }
    String::from_utf8_lossy(&head).into_owned()
}

/// Writes `bytes` and flushes, ignoring a peer that already left.
pub async fn write_all(stream: &mut TcpStream, bytes: &[u8]) {
    let _ = stream.write_all(bytes).await;
    let _ = stream.flush().await;
}

/// Serves each accepted connection with `handle`, one at a time, in accept
/// order.
pub async fn serve<F, Fut>(handle: F) -> (String, AbortOnDrop)
where
    F: Fn(TcpStream) -> Fut + Send + 'static,
    Fut: Future<Output = ()> + Send + 'static,
{
    let (listener, url) = listener().await;
    let task = tokio::spawn(async move {
        while let Ok((stream, _)) = listener.accept().await {
            handle(stream).await;
        }
    });
    (url, AbortOnDrop(task))
}

/// Aborts a mock server task when the test ends, however it ends.
pub struct AbortOnDrop(pub JoinHandle<()>);

impl Drop for AbortOnDrop {
    fn drop(&mut self) {
        self.0.abort();
    }
}

/// A port on loopback that nothing listens on.
pub async fn closed_port_url() -> String {
    let (listener, url) = listener().await;
    drop(listener);
    url
}
