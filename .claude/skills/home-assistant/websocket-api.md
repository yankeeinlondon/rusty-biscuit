# Home Assistant WebSocket API

Endpoint: `ws://$HA_HOST:8123/api/websocket` (or `wss://`). The full configuration surface lives here.

Command tables below are verified against `home-assistant/core@dev` source (websocket registrations in each component).

## Protocol Lifecycle

```
server -> {"type": "auth_required", "ha_version": "..."}
client -> {"type": "auth", "access_token": "<token>"}
server -> {"type": "auth_ok", "ha_version": "..."}     # or auth_invalid -> connection closes
```

After auth, every client message carries a unique integer `id` (client-incremented):

```json
{"id": 1, "type": "get_states"}
```

Responses:

```json
{"id": 1, "type": "result", "success": true,  "result": ...}
{"id": 1, "type": "result", "success": false, "error": {"code": "...", "message": "..."}}
```

Subscriptions attach to the subscribing command's `id`; events arrive as `{"id": <sub_id>, "type": "event", "event": ...}` until `unsubscribe_events` (with `subscription: <sub_id>`) or disconnect.

## Core Commands

| Type | Purpose |
|------|---------|
| `ping` / `pong` | Keepalive |
| `get_states` | All current states |
| `get_config` | Instance config |
| `get_services` | Service registry |
| `call_service` | `{domain, service, service_data?, target?, return_response?}` |
| `subscribe_events` | `{event_type?}` — all events if omitted |
| `unsubscribe_events` | `{subscription: <id>}` |
| `subscribe_trigger` | Live-test a trigger platform config; fires events when it matches |
| `render_template` | Subscribe to a Jinja template's rendered value as it changes |
| `test_condition` | Evaluate a condition config against current state; returns `{result: bool}` |
| `entity/source` | Which integration/platform backs each entity |
| `fire_event` | Fire an event on the bus |

## Registries (config component)

All take `{"type": <cmd>, ...}`; list commands return arrays of entries.

| Registry | Commands |
|----------|----------|
| Entity | `config/entity_registry/list`, `get`, `get_entries`, `list_for_display` (enabled only, lightweight), `update`, `remove`, `settings/get`, `settings/update` |
| Device | `config/device_registry/list`, `update`, `remove`, `remove_config_entry` |
| Area | `config/area_registry/list`, `create`, `update`, `delete`, `reorder` |
| Floor | `config/floor_registry/list`, `create`, `update`, `delete`, `reorder` |
| Label | `config/label_registry/list`, `create`, `update`, `delete` |
| Category | `config/category_registry/list`, `create`, `update`, `delete` (scoped per registry type) |

Key `config/entity_registry/update` fields: `area_id`, `name`, `icon`, `labels` (array), `disabled_by` (`"user"` to disable, `null` to enable), `hidden_by`, `new_entity_id` (rename — states/history follow the new id).
Device registry entries carry `area_id`, `name_by_user`, `labels`, `disabled_by` — setting a device area does not cascade to already-assigned entity areas (only entities with no explicit area inherit the device area).

Registry entry shape (entity) — fields an audit cares about:

```json
{"id": "<registry uuid>", "entity_id": "light.kitchen", "platform": "hue",
 "device_id": "...", "area_id": "kitchen"|null, "labels": [],
 "disabled_by": null|"user"|"integration", "hidden_by": null,
 "original_name": "...", "unique_id": "...", "aliases": [], "categories": {}}
```

## Config Entries (integrations)

`config_entries/get` (all entries with `state`, `reason`, `source`, `disabled_by`), `config_entries/get_single`, `config_entries/update`, `config_entries/disable` (`{entry_id, disabled_by: "user"|null}`), `config_entries/subscribe` (change feed), `config_entries/flow/progress`, `config_entries/ignore_flow`.

An entry with `state: "failed_setup"` or `setup_error` plus a `reason` is a top audit finding — cross-reference `GET /api/error_log`.

## Automations, Scripts, Scenes

| Command | Use |
|---------|-----|
| `automation/config` `{entity_id}` | Read an automation's full config (triggers/conditions/actions) |
| `script/config` `{entity_id}` | Read a script's full config |
| `trace/list` `{domain, item_id?}` | Recent runs ("traces") with timestamps and results |
| `trace/get` `{domain, item_id, run_id}` | Full trace: which nodes executed, variables, errors |

**Save/delete are REST**, not WS: `POST|DELETE /api/config/{automation|script|scene}/config/{id}` (see rest-api.md).

`trace/list` results include per-run `state` (`stopped`, `error`, ...) — the fastest way to find automations that are erroring or never fire.

## Helpers (storage collections)

Each helper domain registers `{domain}/list`, `{domain}/create`, `{domain}/update`, `{domain}/delete`, `{domain}/subscribe`:

`input_boolean`, `input_number`, `input_text`, `input_select`, `input_datetime`, `input_button`, `counter`, `timer`, `schedule`, `tag`, `zone` (storage zones), `person`, `lovelace/dashboards`, `lovelace/resources`.

`update` takes the item's `id` plus the fields to change; `create`/`delete` take the item payload / `id`.

## Repairs (issue registry)

| Command | Use |
|---------|-----|
| `repairs/list_issues` | All active issues: `{domain, issue_id, severity, breaks_in_ha_version?, is_fixable, ...}` |
| `repairs/get_issue_data` `{domain, issue_id}` | Issue detail/data payload |
| `repairs/ignore_issue` `{domain, issue_id, ignore: bool}` | Mute/unmute an issue |

Fixable issues (`is_fixable: true`) are resolved through interactive repair flows (`repairs/fix_issue` + `repairs/fix_issue/flow` with a data-entry flow protocol) — usually better done in the UI unless automatable input is known.

## Search & Relationships

`search/related` `{item_type, item_id}` where `item_type` ∈ `automation`, `area`, `config_entry`, `device`, `entity`, `group`, `scene`, `script`, `helper`, `tag`, ... Returns maps of related ids per type.

**This is the dependency graph.** Before renaming/removing any entity, device, or area, run `search/related` on it to see which automations/scripts/dashboards/groups reference it.

## Lovelace (dashboards)

`lovelace/info`, `lovelace/config` `{url_path?}` (read a dashboard config), `lovelace/config/save` `{url_path?, config}` (write), `lovelace/config/delete`. Dashboard CRUD via the `lovelace/dashboards/*` collection commands. Default dashboard has no `url_path`.

## Backups

`backup/info` (backups + agent state), `backup/details` `{backup_id}`, `backup/generate` (full backup with optional name/password), `backup/generate_with_automatic_settings`, `backup/delete`, `backup/restore`, `backup/config/info`, `backup/config/update`, `backup/subscribe_events` (progress).

## System Health

`system_health/info` — per-integration health panels (recorder DB size, network reachability, version info). Good first call in an audit.

## Minimal Reusable Client (Python)

No dependency beyond `websockets` (`pip install websockets`); runs a batch of commands and returns results keyed by command:

```python
import asyncio, json, os, websockets

async def ha_ws(commands: list[dict]) -> dict[str, dict]:
    url = os.environ["HA_BASE_URL"].replace("http", "ws") + "/api/websocket"
    results, next_id, pending = {}, 1, {}
    async with websockets.connect(url) as ws:
        assert json.loads(await ws.recv())["type"] == "auth_required"
        await ws.send(json.dumps({"type": "auth", "access_token": os.environ["HA_TOKEN"]}))
        assert json.loads(await ws.recv())["type"] == "auth_ok"
        for cmd in commands:
            cmd = {"id": next_id, **cmd}
            pending[next_id] = cmd["type"]
            next_id += 1
            await ws.send(json.dumps(cmd))
        while pending:
            msg = json.loads(await ws.recv())
            if msg.get("type") == "result":
                results[pending.pop(msg["id"])] = msg
    return results

# results = asyncio.run(ha_ws([{"type": "get_states"},
#                              {"type": "config/entity_registry/list"},
#                              {"type": "repairs/list_issues"}]))
```

Notes:

- Duplicate `type` keys in `results` collide — use one call per type per batch, or key by id instead.
- For large payloads (`get_states` on a big instance is 1–5 MB), raise the frame limit: `websockets.connect(url, max_size=32 * 1024 * 1024)`.
- `websocat` works for one-offs: `wscat`-style interactive use is fine, but scripting JSON framing by hand is error-prone — prefer the snippet above.
