---
prompt: |-
    Provide a full overview of the JSON-RPC protocol.

    - describe the general semantics and syntax that JSON-RPC uses
    - describe any/all major versions of the specification along with dates these versions became available
    - what sort of "types" are supported with JSON RPC?
    - what sort of schema validation is supported within the JSON RPC spec?
    - describe any common gotchas that developers describe hitting when using the JSON RPC protocol along with any solutions or workarounds that help in avoiding these gotchas.
    - provide a simple code example of using JSON RPC in:
        - Typescript
        - Python
        - Rust

    Frontmatter:
    - make sure to update `last_updated` property every time the `prompt` is executed (format: YYYY-MM-DD)
    - make sure to set a `latest_version` property which should be the LATEST version of the specification

    Your research content should be added to the body of this document along with ensuring that the Frontmatter properties above are updated while preserving all other markdown properties.

last_updated: 2026-02-21
latest_version: "2.0"
update_policy:
    - MajorVersion(latest_version)
    - Duration(1 year)
---

# JSON-RPC Protocol Overview

JSON-RPC is a stateless, lightweight remote procedure call (RPC) protocol that uses JSON ([RFC 4627](https://www.ietf.org/rfc/rfc4627.txt)) as its data format. It is transport-agnostic — it can operate over HTTP, WebSockets, TCP sockets, stdin/stdout pipes, or any other mechanism that can carry text or binary data.

The core idea is simple: a client sends a JSON object describing a method to call and its parameters, and the server responds with a JSON object containing the result or an error.

## General Semantics and Syntax

### Request Object

A JSON-RPC request is a JSON object with the following members:

| Field | Type | Required | Description |
|-------|------|----------|-------------|
| `jsonrpc` | `String` | Yes | Must be exactly `"2.0"` |
| `method` | `String` | Yes | Name of the method to invoke. Names beginning with `rpc.` are reserved for system extensions. |
| `params` | `Array` or `Object` | No | Method parameters. Omitted if the method takes no arguments. |
| `id` | `String`, `Number`, or `Null` | Yes* | Unique identifier for correlating request/response. *Omitting `id` makes it a **Notification**. |

Parameters can be passed in two ways:

- **By-position** — `params` is an `Array`; arguments are matched in the order the server expects them.
- **By-name** — `params` is an `Object`; keys must match the server's expected parameter names (case-sensitive).

```json
{
  "jsonrpc": "2.0",
  "method": "subtract",
  "params": { "minuend": 42, "subtrahend": 23 },
  "id": 1
}
```

### Response Object

| Field | Type | Required | Description |
|-------|------|----------|-------------|
| `jsonrpc` | `String` | Yes | Must be `"2.0"` |
| `result` | _any_ | Conditional | Present on success. Must not exist if `error` is present. |
| `error` | `Object` | Conditional | Present on failure. Must not exist if `result` is present. |
| `id` | `String`, `Number`, or `Null` | Yes | Must match the request's `id`. Set to `Null` when the request `id` could not be determined (e.g., parse error). |

Exactly one of `result` or `error` must be present — never both, never neither.

```json
{
  "jsonrpc": "2.0",
  "result": 19,
  "id": 1
}
```

### Notification

A Notification is a request without an `id` member. The server **must not** reply to notifications, and the client has no way to detect errors from them.

```json
{
  "jsonrpc": "2.0",
  "method": "log",
  "params": ["something happened"]
}
```

### Error Object

When a call fails, the response's `error` member is an object with:

| Field | Type | Required | Description |
|-------|------|----------|-------------|
| `code` | `Integer` | Yes | Numeric error code indicating the type of error |
| `message` | `String` | Yes | Short, human-readable description |
| `data` | _any_ | No | Additional structured or primitive data about the error |

### Predefined Error Codes

| Code | Name | Meaning |
|------|------|---------|
| `-32700` | Parse error | Invalid JSON was received by the server |
| `-32600` | Invalid Request | The JSON sent is not a valid Request object |
| `-32601` | Method not found | The method does not exist or is not available |
| `-32602` | Invalid params | Invalid method parameter(s) |
| `-32603` | Internal error | Internal JSON-RPC error |
| `-32000` to `-32099` | Server error | Reserved for implementation-defined server errors |

The range `-32768` to `-32000` is reserved by the specification. Error codes outside this range are available for application use.

### Batch Requests

Multiple requests can be sent as a JSON `Array`. The server responds with an `Array` of corresponding response objects. Responses for notifications are excluded. If the batch is empty, the server returns a single Invalid Request error. If every item in the batch is a notification, the server returns nothing at all.

```json
[
  {"jsonrpc": "2.0", "method": "sum", "params": [1, 2], "id": 1},
  {"jsonrpc": "2.0", "method": "notify", "params": ["hello"]},
  {"jsonrpc": "2.0", "method": "sum", "params": [3, 4], "id": 2}
]
```

Response (note: no response for the notification):

```json
[
  {"jsonrpc": "2.0", "result": 3, "id": 1},
  {"jsonrpc": "2.0", "result": 7, "id": 2}
]
```

## Specification Versions

### JSON-RPC 1.0 (2005)

The original specification, published in 2005. Key characteristics:

- Only **positional parameters** (as an `Array`)
- Notifications indicated by `id: null` (rather than omitting `id`)
- No formal error code system
- Response always includes both `result` and `error` fields (one set to `null`)
- Introduced a `__jsonclass__` class-hinting mechanism for extending beyond native JSON types
- Transport: TCP/IP socket streams and HTTP POST

### JSON-RPC 1.1 Working Draft (August 2006)

A working draft by Atif Aziz and Jan-Klaas Kollhof that was **never finalized**. Notable changes:

- Bound exclusively to HTTP (both POST and GET)
- Introduced **named parameters** alongside positional
- Added a `version` member to requests and responses
- Defined a `system.describe` introspection method for service discovery
- Formalized error codes and error object structure
- **Removed** notification support
- Never gained widespread adoption; superseded by 2.0

### JSON-RPC 2.0 (March 2010, last revised January 2013)

The current and widely adopted specification. Major improvements over 1.0:

- **Named parameters** — `params` can be an `Object` with key-value pairs
- **Mandatory `jsonrpc: "2.0"`** — makes version detection trivial
- **Notifications** — defined as requests without `id` (cleaner than 1.0's `id: null`)
- **Standardized error codes** — predefined codes with reserved ranges
- **Batch requests** — send multiple requests/notifications in a single `Array`
- **Mutual exclusion** of `result`/`error` — response contains one or the other, never both
- **Transport agnostic** — no longer tied to any specific transport protocol
- **Not backward-compatible** with 1.0, though the `jsonrpc` field makes detection straightforward

## Supported Types

JSON-RPC inherits JSON's type system exactly. The specification defines six types, split into two categories:

### Primitive Types

| Type | Description | Example |
|------|-------------|---------|
| `String` | Unicode text, double-quoted | `"hello"` |
| `Number` | Integer or floating-point (no distinction) | `42`, `3.14`, `-1e10` |
| `Boolean` | Logical true/false | `true`, `false` |
| `Null` | Absence of value | `null` |

### Structured Types

| Type | Description | Example |
|------|-------------|---------|
| `Object` | Unordered collection of key-value pairs | `{"key": "value"}` |
| `Array` | Ordered sequence of values | `[1, 2, 3]` |

JSON-RPC does **not** define any type system beyond JSON's native types. There is no concept of dates, binary data, integers vs. floats, or custom types within the protocol itself. The 1.0 spec's `__jsonclass__` hinting mechanism was dropped in 2.0.

## Schema Validation

The JSON-RPC specification itself does **not** include built-in schema validation. However, there are several approaches to adding validation:

### JSON Schema

[JSON Schema](https://json-schema.org/) can validate JSON-RPC message structure. Example schema for a request:

```json
{
  "$schema": "http://json-schema.org/draft-07/schema#",
  "type": "object",
  "required": ["jsonrpc", "method"],
  "properties": {
    "jsonrpc": { "type": "string", "enum": ["2.0"] },
    "method": { "type": "string" },
    "params": {
      "oneOf": [{ "type": "array" }, { "type": "object" }]
    },
    "id": {
      "oneOf": [
        { "type": "string" },
        { "type": "number" },
        { "type": "null" }
      ]
    }
  },
  "additionalProperties": false
}
```

### OpenRPC

[OpenRPC](https://open-rpc.org/) is an open standard that provides an interface description format for JSON-RPC 2.0 APIs — analogous to OpenAPI/Swagger for REST APIs. It enables:

- **Service discovery** — clients can query available methods, parameter schemas, and return types
- **Contract enforcement** — parameter and return value schemas are defined with JSON Schema
- **Code generation** — client/server stubs can be generated from OpenRPC documents
- **Validation tooling** — automated request/response validation against declared schemas

### Application-Level Validation

Most production JSON-RPC implementations handle validation at the application layer:

- Validate parameter types and shapes in method handlers
- Return `-32602` (Invalid params) errors for validation failures
- Use language-specific validation libraries (Zod, Pydantic, serde, etc.)

## Common Gotchas

### 1. Confusing Notifications with Requests

**Problem:** Omitting `id` from a request turns it into a notification. The server will not respond, and the client will hang waiting for a response that never comes.

**Solution:** Always include `id` when you expect a response. Use notifications only for fire-and-forget operations. Generate unique IDs (incrementing integers or UUIDs).

### 2. The `id` Field and `null`

**Problem:** In 2.0, `id: null` in a **response** signals that the request's ID could not be determined (parse error). In 1.0, `id: null` in a **request** meant it was a notification. Mixing conventions causes confusion.

**Solution:** In 2.0, omit `id` entirely for notifications. Never use `null` as a request ID. Use integers or strings for IDs.

### 3. Batch Response Ordering

**Problem:** The spec does not guarantee that batch responses are returned in the same order as the requests. Servers may process requests concurrently and return responses in any order.

**Solution:** Always correlate responses by matching `id` fields, not by array index. Parse each response independently.

### 4. No Built-in Authentication or Authorization

**Problem:** The protocol has no concept of auth — it's purely about method invocation.

**Solution:** Implement authentication at the transport layer (HTTP headers, TLS client certificates, WebSocket handshake). Use JWT, OAuth 2.0, or API keys in HTTP headers. Never embed credentials in JSON-RPC params.

### 5. HTTP Caching Does Not Work

**Problem:** All JSON-RPC requests over HTTP use POST with `application/json` content type. HTTP caches cannot distinguish between different method calls to the same endpoint.

**Solution:** Implement application-level caching. Consider using ETags or custom cache headers for read-only methods. For read-heavy workloads, evaluate whether REST or GraphQL might be more appropriate.

### 6. Error Data is Unstructured

**Problem:** The `data` field in error objects has no required schema. Every implementation invents its own format, making generic error handling across services difficult.

**Solution:** Define a consistent error data schema for your API and document it. Consider using OpenRPC to formalize error schemas. Include machine-readable error codes in `data` alongside human-readable messages.

### 7. No Streaming or Server-Push

**Problem:** JSON-RPC is request-response only. There's no built-in mechanism for server-initiated messages or streaming results.

**Solution:** Use WebSocket transport for bidirectional communication. Implement server-to-client notifications over WebSocket connections. For streaming, consider [JSON-RPC over Server-Sent Events](https://www.jsonrpc.org/) or a complementary protocol.

### 8. Number Precision

**Problem:** JSON numbers follow IEEE 754 double-precision floating point. Large integers (>2^53) lose precision, which is especially problematic in blockchain and financial applications.

**Solution:** Transmit large numbers as strings. Document which fields use string-encoded numbers. Parse with arbitrary-precision libraries on the receiving end.

## Code Examples

### TypeScript

Using the [`json-rpc-2.0`](https://www.npmjs.com/package/json-rpc-2.0) package:

```typescript
import { JSONRPCServer, JSONRPCClient } from "json-rpc-2.0";

// --- Server ---
const server = new JSONRPCServer();

server.addMethod("add", ({ a, b }: { a: number; b: number }) => a + b);

server.addMethod("greet", ({ name }: { name: string }) => `Hello, ${name}!`);

// Simulate receiving a request (in practice, wire this to HTTP/WS)
async function handleRequest(json: string): Promise<string | null> {
  const request = JSON.parse(json);
  const response = await server.receive(request);
  return response ? JSON.stringify(response) : null;
}

// --- Client ---
let nextId = 1;
const client = new JSONRPCClient(async (request) => {
  const responseText = await handleRequest(JSON.stringify(request));
  if (responseText) {
    client.receive(JSON.parse(responseText));
  }
}, () => nextId++);

// Usage
const sum = await client.request("add", { a: 3, b: 5 });
console.log("Sum:", sum); // Sum: 8

const greeting = await client.request("greet", { name: "World" });
console.log(greeting); // Hello, World!
```

### Python

Using the [`jsonrpcserver`](https://pypi.org/project/jsonrpcserver/) and [`jsonrpcclient`](https://pypi.org/project/jsonrpcclient/) packages:

```python
from jsonrpcserver import method, Result, Success, dispatch
from jsonrpcclient import request, parse, Ok
import json

# --- Server ---
@method
def add(a: int, b: int) -> Result:
    return Success(a + b)

@method
def greet(name: str) -> Result:
    return Success(f"Hello, {name}!")

# Simulate receiving a request
def handle_request(json_str: str) -> str:
    return dispatch(json_str)

# --- Client-side usage ---
# Build a request
req = json.dumps(request("add", params={"a": 3, "b": 5}))
print("Request:", req)
# Request: {"jsonrpc": "2.0", "method": "add", "params": {"a": 3, "b": 5}, "id": 1}

# Process it through the server
response_str = handle_request(req)
print("Response:", response_str)
# Response: {"jsonrpc": "2.0", "result": 8, "id": 1}

# Parse the response
response = parse(json.loads(response_str))
if isinstance(response, Ok):
    print("Result:", response.result)  # Result: 8
```

### Rust

Using the [`jsonrpsee`](https://crates.io/crates/jsonrpsee) crate (async, built on tokio):

```rust
use jsonrpsee::core::client::ClientT;
use jsonrpsee::http_client::HttpClientBuilder;
use jsonrpsee::rpc_params;
use jsonrpsee::server::{RpcModule, Server};

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    // --- Server ---
    let server = Server::builder()
        .build("127.0.0.1:9944")
        .await?;

    let mut module = RpcModule::new(());

    module.register_method("add", |params, _, _| {
        let (a, b): (i64, i64) = params.parse()?;
        Ok::<_, jsonrpsee::types::ErrorObjectOwned>(a + b)
    })?;

    module.register_method("greet", |params, _, _| {
        let (name,): (String,) = params.parse()?;
        Ok::<_, jsonrpsee::types::ErrorObjectOwned>(format!("Hello, {name}!"))
    })?;

    let addr = server.local_addr()?;
    let handle = server.start(module);

    // --- Client ---
    let url = format!("http://{addr}");
    let client = HttpClientBuilder::default().build(&url)?;

    let sum: i64 = client.request("add", rpc_params![3, 5]).await?;
    println!("Sum: {sum}"); // Sum: 8

    let greeting: String = client.request("greet", rpc_params!["World"]).await?;
    println!("{greeting}"); // Hello, World!

    handle.stop()?;
    Ok(())
}
```

## Key Protocols Built on JSON-RPC

Several major protocols use JSON-RPC 2.0 as their message framing layer:

- **[Model Context Protocol (MCP)](https://modelcontextprotocol.io/)** — Anthropic's protocol for connecting LLMs to external tools and data sources
- **[Agent Communication Protocol (ACP)](https://agentcommunicationprotocol.dev/)** — standardized agent-to-agent communication
- **[Language Server Protocol (LSP)](https://microsoft.github.io/language-server-protocol/)** — IDE/editor language intelligence (completions, diagnostics, go-to-definition)
- **[Debug Adapter Protocol (DAP)](https://microsoft.github.io/debug-adapter-protocol/)** — standardized debugger communication
- **[Ethereum JSON-RPC](https://ethereum.org/en/developers/docs/apis/json-rpc/)** — blockchain node interaction
- **[OpenAI API](https://platform.openai.com/docs/api-reference)** — real-time voice/chat over WebSocket uses JSON-RPC framing

## References

- [JSON-RPC 2.0 Specification](https://www.jsonrpc.org/specification)
- [JSON-RPC 1.0 Specification](https://www.jsonrpc.org/specification_v1)
- [OpenRPC Specification](https://spec.open-rpc.org/)
- [json-rpc-2.0 (npm)](https://www.npmjs.com/package/json-rpc-2.0)
- [jsonrpsee (crates.io)](https://crates.io/crates/jsonrpsee)
- [jsonrpcserver (PyPI)](https://pypi.org/project/jsonrpcserver/)
