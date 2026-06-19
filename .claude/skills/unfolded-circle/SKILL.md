---
name: unfolded-circle-remote
description: Deep knowledge base for the Unfolded Circle Remote's TCP/IP integrations — the Core API and the Dock API. Use when developing or integrating with an Unfolded Circle Remote, implementing the Core or Dock API, or troubleshooting its integration protocol.
---
# Unfolded Circle Remote Two/3

One of the most powerful universal remotes on the market in 2026 is the Unfolded Circle Remote 3 (the "Two" was the prior version). While this remote can use infrared or bluetooth to communicate, it's biggest distinguishing feature is an emphasis on TCP/IP control. To this end, the team at Unfolded Circle have come up with an "Integration" standard which developers can write to.

## API's Provided

### Core APIs

The **Core** API's allow for interaction with the Unfolded Circle Remote's services. It is **not** to extend it's capabilities but to be able to remotely interact with the service.

For architecture overview, functionality, code examples (in Rust), and key gotcha's and how to work around them read:

- [Core API Deep Dive](./core-api.md) 

If all you want are the API documents:

- Official docs
    - UCR REST Core-API ( [API Docs](https://unfoldedcircle.github.io/core-api/rest/), [YAML definition](https://github.com/unfoldedcircle/core-api/tree/main/core-api/rest) )
    - UCR WS Core-API ( [API Docs](https://unfoldedcircle.github.io/core-api/ws/), [YAML definition](https://github.com/unfoldedcircle/core-api/tree/main/core-api/websocket) )

> **Note:** The websocket API provides all the utility/functionality of the REST API but adds event subscriptions with asynchronous notifications.

### Dock APIs

The **Dock** API's allow you to directly communicate with the charging dock(s), which also serve as IR blasters:

- [Dock API Deep Dive](./dock-api.md)

### Integration APIs

The Unfolded Circle WebSocket Integration-API allows writing device integration drivers for the Unfolded Circle Remotes. These drivers can be external hosted or loaded directly on the remote. In both cases they must support serving up an Websocket interface so that the remote can connect to it.

- For details on the Integration API, including Rust code examples, architecture overview and comparison and more, read: [Deep Dive on Integration Drivers](./integrations.md).
- If you just want the API specification -- which is defined with AsyncAPI:
    - Integration AsyncAPI ( [API Docs](https://unfoldedcircle.github.io/core-api/integration/), [YAML definition](https://unfoldedcircle.github.io/core-api/integration/) )
- [AsyncAPI Studio](https://studio.asyncapi.com/) is an online tool to help you create integrations for the Unfolded Circle remote.

## Integration Libraries

Integration libraries for Python, NodeJS, and Rust are made available:

- [Python API Wrapper for UC Integration API](https://github.com/unfoldedcircle/integration-python-library)
- [NodeJS API Wrapper for UC Integration API](https://github.com/unfoldedcircle/integration-node-library)
- [`api-model-rs` crate for Rust](./api-model-rs.md)

## API Clients in Schematic

Because we plan on working with the Unfolded Circle ecosystem a lot we have defined strongly typed API clients in Schematic.

- Read [API Clients in Schematic](./api-clients.md) for details on all of the resources in the **rusty-biscuit** monorepo's **Schematic** packages can help when working with any of the Unfolded Circle APIs.
