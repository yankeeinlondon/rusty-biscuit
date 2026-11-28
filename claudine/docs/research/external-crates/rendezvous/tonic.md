---
prompt: |-
    The 'tonic' crate in Rust provides gRPC functionality which we will be using in the "rendezvous" daemon.
    
    Your task is to do a deep dive on the 'tonic' crate and answer the following questions through thorough research:
    
    - What is the functional footprint of the 'tonic' crate?
    - What features does tonic expose and what functionality do these features map to? When should you use each feature? When should you avoid?
    - What are the key URLs for this crate? Repo? Website? Docs?
    - What are 2-3 common use cases that this crate would be used for? For each use case, describe the use case and provide Rust code examples of how this use case might be implemented.
    - What do developers say about using this crate? What "gotchas" are there and how can they be worked around?
last_updated: 2026-05-24
---
## Functional Footprint

`tonic` is a native Rust implementation of gRPC over HTTP/2. It provides the full client and server machinery needed to define, generate, and run gRPC services with first-class `async/await` support. Its functional footprint covers:

- **Code generation**: `tonic-build` (via `prost`) generates typed Rust structs, client stubs, and server traits from `.proto` files at compile time.
- **Transport**: A batteries-included HTTP/2 client (`Channel`) and server (`Server`) built on `hyper`, `tokio`, and `tower`.
- **All four gRPC streaming patterns**: unary, server streaming, client streaming, and bidirectional streaming.
- **TLS**: Optional `rustls`-backed TLS for both client and server, with multiple crypto providers and root trust store options.
- **Compression**: Optional per-message compression (`gzip`, `deflate`, `zstd`).
- **Middleware/interceptors**: Request/response interception via Tower layers and gRPC-specific interceptors for metadata inspection, auth, logging, and cancellation.
- **Load balancing**: Client-side load balancing over multiple endpoints with dynamic endpoint updates.
- **Health checking**: Standard gRPC health check protocol via `tonic-health`.
- **Reflection**: gRPC server reflection for tools like `grpcurl` via `tonic-reflection`.
- **Metadata**: Full support for gRPC custom metadata (`MetadataMap`) and trailing headers.
- **Message limits**: Configurable max encoding/decoding message sizes and HTTP/2 frame sizes.
- **Graceful shutdown**: Server-side graceful shutdown with configurable connection age and keepalive settings.

---

## Feature Flags

| Feature               | What It Enables                                                                                                                            | When to Use                                                                                       | When to Avoid                                                                                            |
|-----------------------|--------------------------------------------------------------------------------------------------------------------------------------------|---------------------------------------------------------------------------------------------------|----------------------------------------------------------------------------------------------------------|
| `transport` (default) | Full client (`Channel`) and server (`Server`) implementation via `hyper` + `tokio` + `tower`. Enables `server` and `channel` sub-features. | Use this for almost all production applications.                                                  | Avoid only if you are bringing your own HTTP/2 transport and only need the generic gRPC codec machinery. |
| `server`              | Server portion of `transport`.                                                                                                             | Use when building only a gRPC server without client code (reduces compile time and dependencies). | Avoid if your crate also needs to act as a client.                                                       |
| `channel`             | Client portion of `transport`.                                                                                                             | Use when building only a gRPC client.                                                             | Avoid if your crate also needs to serve requests.                                                        |
| `router` (default)    | `axum`-based service router on the server.                                                                                                 | Use for composing multiple gRPC services or mixing gRPC with HTTP routes.                         | Avoid if you are using a custom router or a very minimal server build.                                   |
| `codegen` (default)   | Exports required for `tonic-build`/`tonic-prost-build` generated code.                                                                     | Always keep enabled when using generated stubs.                                                   | Avoid only if you hand-write all trait implementations (rare).                                           |
| `tls-ring`            | `rustls` TLS using the `ring` crypto provider.                                                                                             | Use for TLS when you prefer `ring` (widely supported, good performance).                          | Avoid if you need FIPS compliance or prefer `aws-lc-rs`.                                                 |
| `tls-aws-lc`          | `rustls` TLS using the `aws-lc-rs` crypto provider.                                                                                        | Use when you need AWS/libcrypto-based cryptography or FIPS-aligned tooling.                       | Avoid if `ring` is sufficient and you want slightly fewer transitive dependencies.                       |
| `tls-native-roots`    | Load system trust roots for `rustls` clients via `rustls-native-certs`.                                                                    | Use for client connections to public internet services using OS trust stores.                     | Avoid in minimal/containerized builds where you want a static root bundle.                               |
| `tls-webpki-roots`    | Bundle Mozilla's root CA store (`webpki-roots`).                                                                                           | Use for portable, self-contained clients (containers, static binaries).                           | Avoid if you must strictly use the host OS certificate store.                                            |
| `tls-connect-info`    | Extra `Connected` impls for TLS connectors.                                                                                                | Use when building a custom TLS connector without enabling other `tls-*` features.                 | Avoid if you already enable another `tls-*` feature (it is pulled in automatically).                     |
| `gzip`                | `flate2`-based gzip compression for requests, responses, and streams.                                                                      | Use when sending large payloads over slow or metered links.                                       | Avoid for CPU-bound low-latency services (compression adds overhead).                                    |
| `deflate`             | `flate2`-based deflate compression.                                                                                                        | Same as `gzip`, but sometimes better for text-heavy payloads.                                     | Same CPU overhead caveat as `gzip`.                                                                      |
| `zstd`                | `zstd` compression.                                                                                                                        | Use when you want better compression ratios than gzip with similar or better speed.               | Avoid if you cannot link the `zstd` C library or need minimal binary size.                               |

---

## Key URLs

- **Repository**: https://github.com/hyperium/tonic
- **Docs.rs**: https://docs.rs/tonic
- **Website / Guide**: https://github.com/hyperium/tonic/tree/master/examples
- **Crate**: https://crates.io/crates/tonic
- **Chat / Community**: Tonic Discord (linked from the repo README)

---

## Common Use Cases

### 1. Microservice RPC with Unary Calls

The most common use case: services communicate over HTTP/2 with strongly typed, compiled protobuf contracts.

**`.proto`**

```protobuf
syntax = "proto3";

package orders;

service OrderService {
  rpc PlaceOrder (OrderRequest) returns (OrderResponse);
}

message OrderRequest {
  string item_id = 1;
  int32 quantity = 2;
}

message OrderResponse {
  string order_id = 1;
  bool success = 2;
}
```

**`build.rs`**

```rust
fn main() -> Result<(), Box<dyn std::error::Error>> {
    tonic_prost_build::compile_protos("proto/orders.proto", &["proto"])?;
    Ok(())
}
```

**Server**

```rust
use tonic::{transport::Server, Request, Response, Status};

pub mod orders {
    tonic::include_proto!("orders");
}

use orders::order_service_server::{OrderService, OrderServiceServer};
use orders::{OrderRequest, OrderResponse};

#[derive(Debug, Default)]
pub struct OrderServiceImpl;

#[tonic::async_trait]
impl OrderService for OrderServiceImpl {
    async fn place_order(
        &self,
        request: Request<OrderRequest>,
    ) -> Result<Response<OrderResponse>, Status> {
        let req = request.into_inner();
        println!("Placing order for {} x {}", req.item_id, req.quantity);

        let reply = OrderResponse {
            order_id: "ord-1234".into(),
            success: true,
        };
        Ok(Response::new(reply))
    }
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let addr = "[::1]:50051".parse()?;
    let svc = OrderServiceImpl::default();

    Server::builder()
        .add_service(OrderServiceServer::new(svc))
        .serve(addr)
        .await?;

    Ok(())
}
```

**Client**

```rust
pub mod orders {
    tonic::include_proto!("orders");
}

use orders::order_service_client::OrderServiceClient;
use orders::OrderRequest;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut client = OrderServiceClient::connect("http://[::1]:50051").await?;

    let request = tonic::Request::new(OrderRequest {
        item_id: "sku-42".into(),
        quantity: 3,
    });

    let response = client.place_order(request).await?;
    println!("RESPONSE={:?}", response.into_inner());

    Ok(())
}
```

---

### 2. Real-Time Bidirectional Streaming

Useful for chat, live data feeds, collaborative editing, or game servers where client and server send messages independently over a single long-lived connection.

**`.proto`**

```protobuf
syntax = "proto3";

package chat;

service ChatService {
  rpc Chat (stream ChatMessage) returns (stream ChatMessage);
}

message ChatMessage {
  string user = 1;
  string content = 2;
}
```

**Server**

```rust
use futures::StreamExt;
use std::pin::Pin;
use tokio::sync::broadcast;
use tokio_stream::wrappers::BroadcastStream;
use tonic::{Request, Response, Status, Streaming};

pub mod chat {
    tonic::include_proto!("chat");
}

use chat::chat_service_server::{ChatService, ChatServiceServer};
use chat::ChatMessage;

type ResponseStream = Pin<Box<dyn futures::Stream<Item = Result<ChatMessage, Status>> + Send>>;

pub struct ChatServer {
    tx: broadcast::Sender<ChatMessage>,
}

impl ChatServer {
    fn new() -> Self {
        let (tx, _rx) = broadcast::channel(100);
        ChatServer { tx }
    }
}

#[tonic::async_trait]
impl ChatService for ChatServer {
    type ChatStream = ResponseStream;

    async fn chat(
        &self,
        request: Request<Streaming<ChatMessage>>,
    ) -> Result<Response<Self::ChatStream>, Status> {
        let mut inbound = request.into_inner();
        let tx = self.tx.clone();
        let rx = self.tx.subscribe();

        tokio::spawn(async move {
            while let Some(Ok(msg)) = inbound.next().await {
                let _ = tx.send(msg);
            }
        });

        let outbound = BroadcastStream::new(rx).filter_map(|r| async move {
            match r {
                Ok(msg) => Some(Ok(msg)),
                Err(broadcast::error::RecvError::Lagged(_)) => None,
                Err(broadcast::error::RecvError::Closed) => None,
            }
        });

        Ok(Response::new(Box::pin(outbound)))
    }
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let addr = "[::1]:50051".parse()?;
    let server = ChatServer::new();

    tonic::transport::Server::builder()
        .add_service(ChatServiceServer::new(server))
        .serve(addr)
        .await?;

    Ok(())
}
```

---

### 3. gRPC Client with TLS and Interceptors

Calling external gRPC APIs (e.g., cloud provider APIs, vector databases like Qdrant, or LND) with authentication metadata and TLS.

```rust
use tonic::{transport::Channel, Request, Status, service::interceptor::Interceptor};

pub mod api {
    tonic::include_proto!("api");
}

use api::my_service_client::MyServiceClient;

#[derive(Clone)]
struct AuthInterceptor {
    token: String,
}

impl Interceptor for AuthInterceptor {
    fn call(&mut self, mut request: Request<()>) -> Result<Request<()>, Status> {
        request.metadata_mut().insert(
            "authorization",
            format!("Bearer {}", self.token).parse().unwrap(),
        );
        Ok(request)
    }
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let channel = Channel::from_static("https://api.example.com:443")
        .tls_config(tonic::transport::ClientTlsConfig::new())?
        .connect()
        .await?;

    let interceptor = AuthInterceptor {
        token: "secret-token".into(),
    };

    let client = MyServiceClient::with_interceptor(channel, interceptor);

    let request = Request::new(api::QueryRequest { id: 42 });
    let response = client.query(request).await?;
    println!("{:?}", response.into_inner());

    Ok(())
}
```

---

## Developer Feedback and Gotchas

### 1. The `#[tonic::async_trait]` Macro Is Required

Rust does not yet support `async fn` in traits by default. Tonic generates traits with async methods, so you must use `#[tonic::async_trait]` on your service implementations. This can add a small amount of opaque magic and extra boxing overhead.

**Workaround**: Write your impls with the macro as shown in all examples; the ecosystem treats this as idiomatic for now.

### 2. Streaming Return Types Are Verbose

Server and bidirectional streaming methods require an associated type (e.g., `type ListFeaturesStream = ...`). The exact type must implement `Stream<Item = Result<T, Status>> + Send + 'static`, which often leads to verbose `Pin<Box<dyn Stream<...>>>` type aliases.

**Workaround**: Use a type alias at the module level, or use `tokio_stream::wrappers::ReceiverStream` if you are sending data from a spawned task via an `mpsc` channel.

### 3. Default 4 MB Message Size Limit

Tonic imposes a default `max_decoding_message_size` of 4 MB. If you send large protobuf messages, you will hit an `OutOfRange` error. In some older versions, this limit was inconsistently applied to streaming responses.

**Workaround**: Explicitly raise the limit on both client and server:

```rust
Server::builder()
    .max_decoding_message_size(64 * 1024 * 1024)
    .add_service(MyServiceServer::new(svc))
    ...
```

And on the client:

```rust
let client = MyServiceClient::new(channel)
    .max_decoding_message_size(64 * 1024 * 1024);
```

### 4. `protoc` Is Required at Build Time

`tonic-build` (via `prost-build`) needs the Protocol Buffers compiler (`protoc`) installed on the build machine. This is a common CI gotcha.

**Workaround**: Install `protoc` in your CI pipeline (e.g., `sudo apt-get install -y protobuf-compiler` on Ubuntu). Alternatively, use `protoc-bin-vendored` or `protobuf-src` crates to bundle a `protoc` binary with your build.

### 5. Interceptors Are Request-Only; Use Tower for Full Middleware

Tonic's `Interceptor` trait only allows inspecting or cancelling the *request*. If you need to log responses, transform errors, or retry, an interceptor is insufficient.

**Workaround**: Use `tower` layers (e.g., `tower_http::trace::TraceLayer`) or the `tonic-middleware` ecosystem. Tonic's server and client are Tower services, so standard Tower middleware composes natively.

### 6. Client Streaming Drop Behavior

When using `ReceiverStream` (or similar) for client streaming, dropping the channel too early can cause the final messages to be lost before the gRPC client finishes sending them.

**Workaround**: Ensure the sender side of the stream lives long enough, or add a small `sleep`/`yield` before dropping the scope if you observe missing trailing messages.

### 7. TLS Feature Selection Can Be Confusing

There are multiple TLS features (`tls-ring`, `tls-aws-lc`, `tls-native-roots`, `tls-webpki-roots`). Enabling the wrong combination can lead to linker errors or missing root certificates.

**Workaround**: For typical internet-facing clients, use:

```toml
tonic = { version = "0.14", features = ["tls-ring", "tls-webpki-roots"] }
```

For servers with mTLS, read the `tonic` docs carefully and test certificate chains in a staging environment.
<choice>STOP</choice>
