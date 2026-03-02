---
name: unfolded-circle-remote
description: Deep knowledge base for working with and developing the Unfolded Circle Remote's TCP/IP Integrations.
---
# Unfolded Circle Remote Two/3

One of the most powerful universal remotes on the market in 2026 is the Unfolded Circle Remote 3 (the "Two" was the prior version). While this remote can use infrared or bluetooth to communicate, it's biggest distinguishing feature is an emphasis on TCP/IP control. To this end, the team at Unfolded Circle have come up with an "Integration" standard which developers can write to.

## API's Provided

### Core APIs

The **Core** API's allow for interaction with the Unfolded Circle Remote's services. It is **not** to extend it's capabilities but to be able to remotely interact with the service.

You should read the [Core API Deep Dive](./core-api.md) for architecture overview, code examples (in Rust), and 

- UCR REST Core-API ( [API Docs](https://unfoldedcircle.github.io/core-api/rest/), [YAML definition](https://github.com/unfoldedcircle/core-api/tree/main/core-api/rest) )
- UCR WS Core-API ( [API Docs](https://unfoldedcircle.github.io/core-api/ws/), [YAML definition](https://github.com/unfoldedcircle/core-api/tree/main/core-api/websocket) )

The websocket API provides all the utility/functionality of the REST API but adds event subscriptions with asynchronous notifications.



### Dock APIs

The **Dock** API's allow you to directly communicate with the charging dock(s) and take full control of it's features.

- Dock AsyncAPI ( [API Docs](https://unfoldedcircle.github.io/core-api/dock/), [YAML definition](https://github.com/unfoldedcircle/core-api/tree/main/dock-api) )

### Integration APIs

The Unfolded Circle WebSocket Integration-API allows writing device integration drivers for the Unfolded Circle Remotes.

The API specification is defined with AsyncAPI in YAML format. The WebSocket communication is using text messages with JSON payload.

- Integration AsyncAPI ( [API Docs](https://unfoldedcircle.github.io/core-api/integration/), [YAML definition](https://unfoldedcircle.github.io/core-api/integration/) )
- [AsyncAPI Studio](https://studio.asyncapi.com/) is an online tool to help you create integrations for the Unfolded Circle remote.

## Integration Libraries

Integration libraries for Python and NodeJS are made available:

- [Python API Wrapper for UC Integration API](https://github.com/unfoldedcircle/integration-python-library)
- [NodeJS API Wrapper for UC Integration API](https://github.com/unfoldedcircle/integration-node-library)

