# Schematic API Clients for Unfolded Circle

Because we want to be able to develop high quality Integrations as well as Core and Dock API based solutions for the [**Unfolded Circle**](https://unfoldedcircle.com) ecosystem, we have developed type strong API clients for all of the published API's from Unfolded Circle.

- the API definitions aer in `schematic/definitions`
- these definitions use the `schematic/define` primitives to define the REST and Websocket APIs
- we "publish" these definitions using the `schematic/gen` code generations library
- the published definitions are put in `schematic/schema` for callers to use

## Schematic API Clients

- Core API
    - [Core REST API in `unfolded_circle_core_rest`](@schematic/schema/src/unfolded_circle_core_rest.rs)
    - [Core Websocket API in `unfolded_circle_core_ws`](@schematic/schema/src/unfolded_circle_core_ws.rs)
- Dock API
    - [Websocket API for Dock in `unfolded_circle_dock_ws`](@schematic/schema/src/unfolded_circle_dock_ws.rs)
- Integration API
    - [Websocket API for Integrations in `unfolded_circle_dock_ws`](@schematic/schema/src/unfolded_circle_doc_ws.rs)

### Definitions

Each of the API client's above are defined in `schematic/definitions` in separate modules:

- [Core REST API](@schematic/definitions/src/unfolded_circle/core_rest/mod.rs)
- [Core Websocket API](@schematic/definitions/src/unfolded_circle/core_ws/mod.rs)
- [Dock Websocket API](@schematic/definitions/src/unfolded_circle/dock_ws/mod.rs)
- [Integration Websocket API](@schematic/definitions/src/unfolded_circle/integration_ws/mod.rs)

### OpenAPI Definitions

Each of the API clients produced in schematic also has a full [OpenAPI](https://swagger.io/specification/) specification.

### Postman Configuration

COMING SOON

