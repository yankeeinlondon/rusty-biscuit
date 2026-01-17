# WebSocket Support for Schematic

## Problem Statement

The `schematic/define` package currently provides primitives for defining REST APIs with a request→response pattern. However, modern APIs increasingly use WebSocket connections for real-time bidirectional communication. The ElevenLabs Text-to-Speech streaming API exemplifies this pattern:

- **Connection Lifecycle**: Upgrade from HTTP to WebSocket, maintain connection, send multiple messages, close gracefully
- **Bidirectional Messages**: Client sends text chunks; server streams audio and alignment data
- **Message Sequencing**: Specific message order (initialize → stream text → close)
- **Connection Parameters**: Query params and headers at connection time
- **Streaming Responses**: Multiple server messages per client message

## Current Architecture Analysis

### REST API Primitives

The existing `schematic/define` package models synchronous request/response:

```rust
RestApi {
    endpoints: Vec<Endpoint>,
    // auth, headers, base_url...
}

Endpoint {
    method: RestMethod,  // GET, POST, etc.
    path: String,        // URL with params
    request: Option<Schema>,
    response: ApiResponse,  // Single response
}
```

**Key Characteristics**:
- One request produces one response
- HTTP methods (GET/POST/PUT/DELETE)
- Stateless by design
- Request/response schemas are sufficient

## Proposed Solution

### Option A: Parallel Type System (Recommended)

Create parallel WebSocket primitives alongside REST without modifying existing types. This maintains backward compatibility and clearly separates concerns.

#### New Core Types

```rust
// schematic/define/src/websocket.rs

/// WebSocket API definition
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WebSocketApi {
    /// API identifier
    pub name: String,
    pub description: String,
    /// Base WebSocket URL (wss://api.example.com)
    pub base_url: String,
    pub docs_url: Option<String>,
    /// Authentication strategy (reuse existing AuthStrategy)
    pub auth: AuthStrategy,
    pub env_auth: Vec<String>,
    pub env_username: Option<String>,
    /// Default headers for connection upgrade
    pub headers: Vec<(String, String)>,
    /// WebSocket endpoints
    pub endpoints: Vec<WebSocketEndpoint>,
}

/// A single WebSocket endpoint
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WebSocketEndpoint {
    /// Identifier (e.g., "StreamTextToSpeech")
    pub id: String,
    /// Path template (e.g., "/v1/text-to-speech/{voice_id}/stream")
    pub path: String,
    pub description: String,
    /// Query parameters for connection upgrade
    pub connection_params: Vec<ConnectionParam>,
    /// Connection lifecycle definition
    pub lifecycle: ConnectionLifecycle,
}

/// Query parameter for WebSocket connection
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ConnectionParam {
    pub name: String,
    pub param_type: ParamType,
    pub required: bool,
    pub default: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ParamType {
    String,
    Integer,
    Boolean,
    Float,
}

/// Defines the message flow for a WebSocket connection
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ConnectionLifecycle {
    /// Initial message schema (sent immediately after connection)
    pub initialize: Option<MessageSchema>,
    /// Streaming messages (can send multiple times)
    pub streaming_messages: Vec<MessageSchema>,
    /// Close message schema
    pub close: Option<MessageSchema>,
    /// Expected server message types
    pub server_messages: Vec<MessageSchema>,
}

/// Schema for a WebSocket message
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MessageSchema {
    /// Message type identifier
    pub id: String,
    pub description: String,
    /// Message direction
    pub direction: MessageDirection,
    /// Schema for message body
    pub schema: Schema,
    /// Whether this message can be sent/received multiple times
    pub repeatable: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MessageDirection {
    ClientToServer,
    ServerToClient,
    Bidirectional,
}
```

#### Example: ElevenLabs Streaming TTS

```rust
use schematic_define::{
    WebSocketApi, WebSocketEndpoint, ConnectionLifecycle,
    MessageSchema, MessageDirection, ConnectionParam, ParamType,
    AuthStrategy, Schema,
};

let elevenlabs = WebSocketApi {
    name: "ElevenLabs".to_string(),
    description: "ElevenLabs Text-to-Speech API".to_string(),
    base_url: "wss://api.elevenlabs.io".to_string(),
    docs_url: Some("https://elevenlabs.io/docs/api-reference".to_string()),
    auth: AuthStrategy::ApiKey {
        header: "xi-api-key".to_string(),
    },
    env_auth: vec!["ELEVENLABS_API_KEY".to_string()],
    env_username: None,
    headers: vec![],
    endpoints: vec![
        WebSocketEndpoint {
            id: "StreamTextToSpeech".to_string(),
            path: "/v1/text-to-speech/{voice_id}/stream-input".to_string(),
            description: "Stream text input for real-time TTS".to_string(),
            connection_params: vec![
                ConnectionParam {
                    name: "model_id".to_string(),
                    param_type: ParamType::String,
                    required: false,
                    default: None,
                },
                ConnectionParam {
                    name: "output_format".to_string(),
                    param_type: ParamType::String,
                    required: false,
                    default: Some("mp3_44100_128".to_string()),
                },
                ConnectionParam {
                    name: "enable_ssml_parsing".to_string(),
                    param_type: ParamType::Boolean,
                    required: false,
                    default: Some("false".to_string()),
                },
            ],
            lifecycle: ConnectionLifecycle {
                initialize: Some(MessageSchema {
                    id: "InitializeConnection".to_string(),
                    description: "Initialize connection with settings".to_string(),
                    direction: MessageDirection::ClientToServer,
                    schema: Schema::new("InitializeConnectionMessage"),
                    repeatable: false,
                }),
                streaming_messages: vec![
                    MessageSchema {
                        id: "SendText".to_string(),
                        description: "Send text chunk for synthesis".to_string(),
                        direction: MessageDirection::ClientToServer,
                        schema: Schema::new("SendTextMessage"),
                        repeatable: true,
                    },
                ],
                close: Some(MessageSchema {
                    id: "CloseConnection".to_string(),
                    description: "Close the WebSocket connection".to_string(),
                    direction: MessageDirection::ClientToServer,
                    schema: Schema::new("CloseConnectionMessage"),
                    repeatable: false,
                }),
                server_messages: vec![
                    MessageSchema {
                        id: "AudioOutput".to_string(),
                        description: "Audio chunk with alignment data".to_string(),
                        direction: MessageDirection::ServerToClient,
                        schema: Schema::new("AudioOutputMessage"),
                        repeatable: true,
                    },
                    MessageSchema {
                        id: "FinalOutput".to_string(),
                        description: "Final message indicating completion".to_string(),
                        direction: MessageDirection::ServerToClient,
                        schema: Schema::new("FinalOutputMessage"),
                        repeatable: false,
                    },
                ],
            },
        },
    ],
};
```

### Option B: Unified Endpoint Type

Extend the existing `Endpoint` type to support both REST and WebSocket. This approach unifies the API surface but increases complexity.

```rust
// schematic/define/src/types.rs (modified)

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum EndpointType {
    Rest(RestEndpointConfig),
    WebSocket(WebSocketEndpointConfig),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RestEndpointConfig {
    pub method: RestMethod,
    pub request: Option<Schema>,
    pub response: ApiResponse,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WebSocketEndpointConfig {
    pub connection_params: Vec<ConnectionParam>,
    pub lifecycle: ConnectionLifecycle,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Endpoint {
    pub id: String,
    pub path: String,
    pub description: String,
    pub headers: Vec<(String, String)>,
    pub endpoint_type: EndpointType,  // NEW
}
```

**Pros**:
- Single API type can contain both REST and WebSocket endpoints
- Code generation can handle mixed APIs uniformly

**Cons**:
- Breaking change to existing `Endpoint` type
- Increases cognitive load (developers must handle both types)
- REST-specific fields become optional/nested

### Option C: Trait-Based Abstraction

Define a trait for "network endpoints" that both REST and WebSocket implement.

```rust
pub trait NetworkEndpoint {
    fn id(&self) -> &str;
    fn path(&self) -> &str;
    fn description(&self) -> &str;
    fn protocol(&self) -> Protocol;
}

pub enum Protocol {
    Http,
    WebSocket,
}

impl NetworkEndpoint for Endpoint { /* REST */ }
impl NetworkEndpoint for WebSocketEndpoint { /* WS */ }
```

**Pros**:
- Polymorphism for generic tooling
- Maximum flexibility

**Cons**:
- Over-engineering for current needs
- Requires trait objects or generics in API definitions
- Harder to serialize/deserialize

## Recommendation

**Use Option A: Parallel Type System**

### Rationale

1. **Backward Compatibility**: No changes to existing REST API definitions
2. **Clear Separation**: REST and WebSocket have fundamentally different semantics
3. **Type Safety**: Compiler enforces correct usage patterns
4. **Discoverability**: Explicit types make the API surface clear
5. **Code Generation**: Generator can have separate logic for REST vs WebSocket

### Implementation Plan

#### Phase 1: Core Types (schematic/define)

1. Add `websocket.rs` module with new types
2. Export from `lib.rs` and `prelude.rs`
3. Add documentation and examples
4. Write unit tests

Files to create:
- `schematic/define/src/websocket.rs`

Files to modify:
- `schematic/define/src/lib.rs` - add `pub mod websocket;`
- `schematic/define/src/prelude.rs` - export WebSocket types

#### Phase 2: Code Generation (schematic/gen)

Extend the generator to handle WebSocket APIs:

1. **Connection Management**:
   ```rust
   // Generated code structure
   pub struct ElevenLabsWs {
       connection: WebSocketStream,
       base_url: String,
   }

   impl ElevenLabsWs {
       pub async fn connect(voice_id: &str) -> Result<Self>;
       pub async fn send_initialize(&mut self, msg: InitializeConnectionMessage) -> Result<()>;
       pub async fn send_text(&mut self, msg: SendTextMessage) -> Result<()>;
       pub async fn close(&mut self, msg: CloseConnectionMessage) -> Result<()>;
       pub async fn receive(&mut self) -> Result<ServerMessage>;
   }

   pub enum ServerMessage {
       AudioOutput(AudioOutputMessage),
       FinalOutput(FinalOutputMessage),
   }
   ```

2. **Message Schemas**: Generate struct definitions from `MessageSchema`
3. **Authentication**: Inject auth headers/params during connection
4. **Error Handling**: WebSocket-specific errors (connection closed, invalid message)

Dependencies needed:
- `tokio-tungstenite` - WebSocket client
- `futures` - Async stream handling

#### Phase 3: Documentation

1. Update `schematic/define/README.md` with WebSocket examples
2. Create migration guide for adding WebSocket to existing APIs
3. Document code generation patterns

#### Phase 4: Example Definition

Create `schematic/definitions/src/elevenlabs.rs`:

```rust
use schematic_define::{WebSocketApi, /* ... */};

pub fn elevenlabs_api() -> WebSocketApi {
    // Full ElevenLabs definition
}
```

### Alternative WebSocket Client Patterns

The generator could support different usage patterns:

**Pattern 1: Explicit State Machine**
```rust
let mut conn = ElevenLabsWs::connect(voice_id).await?;
conn.send_initialize(init_msg).await?;
conn.send_text(text_msg).await?;
conn.close(close_msg).await?;
```

**Pattern 2: Builder with Callbacks**
```rust
ElevenLabsWs::builder()
    .voice_id(voice_id)
    .on_audio(|audio| { /* handle */ })
    .on_final(|final_msg| { /* handle */ })
    .connect()
    .await?
    .send_text_stream(text_stream)
    .await?;
```

**Pattern 3: Stream-Based**
```rust
let (mut sink, mut stream) = ElevenLabsWs::connect(voice_id).await?;
sink.send(ClientMessage::Initialize(init_msg)).await?;
while let Some(msg) = stream.next().await {
    match msg? {
        ServerMessage::AudioOutput(audio) => { /* ... */ },
        ServerMessage::FinalOutput(_) => break,
    }
}
```

**Recommendation**: Start with Pattern 1 (explicit state machine) for simplicity and type safety. Later add Pattern 3 for advanced users who want fine-grained control.

## Open Questions

1. **Reconnection Logic**: Should the generated code handle reconnects automatically?
2. **Backpressure**: How do we handle slow consumers when server sends data faster than client processes?
3. **Timeouts**: Connection-level vs message-level timeouts?
4. **Subprotocols**: Do we need to support WebSocket subprotocols?
5. **Binary vs Text**: How to handle binary WebSocket frames (ElevenLabs uses JSON)?
6. **Compression**: Support for `permessage-deflate` extension?

## Success Criteria

1. ✅ Zero breaking changes to existing REST API definitions
2. ✅ Type-safe WebSocket message handling
3. ✅ Clear separation between REST and WebSocket concepts
4. ✅ Generated code follows Rust async best practices
5. ✅ Comprehensive examples and documentation
6. ✅ Support for real-world APIs (ElevenLabs as reference)

## Future Extensions

1. **Server-Sent Events (SSE)**: Similar streaming pattern, simpler than WebSocket
2. **GraphQL Subscriptions**: WebSocket-based subscription protocol
3. **gRPC Streaming**: Bidirectional streaming over HTTP/2
4. **MQTT**: Pub/sub over WebSocket transport

## References

- ElevenLabs WebSocket API: https://elevenlabs.io/docs/api-reference/text-to-speech/v-1-text-to-speech-voice-id-stream-input
- WebSocket RFC 6455: https://tools.ietf.org/html/rfc6455
- `tokio-tungstenite`: https://docs.rs/tokio-tungstenite/
- Rust WebSocket ecosystem: https://lib.rs/search?q=websocket
