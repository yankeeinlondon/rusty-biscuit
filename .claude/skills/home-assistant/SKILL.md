---
name: home-assistant
description: Interact with a Home Assistant instance over its REST and WebSocket APIs. Use when auditing, querying, or fixing Home Assistant configuration (entities, devices, areas, automations, scripts, helpers, registries, repairs), calling services, or rendering Jinja templates against a live instance.
---

# Home Assistant API

Two complementary HTTP APIs on the same port (default `8123`):

- **REST** (`/api/...`) — states, service calls, history, logbook, template rendering, config validation, and UI-grade config editing for automations/scripts/scenes
- **WebSocket** (`/api/websocket`) — everything REST does, plus the full configuration surface: entity/device/area/floor/label registries, config entries, helpers, repairs, traces, dashboards, backups, reference search

> For configuration audits and remediation, the **WebSocket API does the heavy lifting**. REST alone cannot see registry data, automation/script definitions, or repairs.

## Connecting & Authentication

- Base URL: `http://<host>:8123` (e.g. `homeassistant.local`). Nabu Casa remote URLs work but are slower.
- Token: long-lived access token, created in the HA UI → user profile → **Security** → **Long-Lived Access Tokens**.
- Convention for ad-hoc sessions: `HA_BASE_URL` and `HA_TOKEN` environment variables. Never write tokens to files or command history visible in output.

```bash
# REST
curl -s -H "Authorization: Bearer $HA_TOKEN" "$HA_BASE_URL/api/"

# WebSocket handshake (all frames are JSON text):
#   server -> {"type": "auth_required", "ha_version": "..."}
#   client -> {"type": "auth", "access_token": "$HA_TOKEN"}
#   server -> {"type": "auth_ok", "ha_version": "..."}   (or auth_invalid)
```

## Choosing REST vs. WebSocket

| Task | API | Command / Endpoint |
|------|-----|--------------------|
| Read entity states | either | `GET /api/states` · WS `get_states` |
| Call a service | either | `POST /api/services/{domain}/{service}` · WS `call_service` |
| Render a Jinja template | REST | `POST /api/template` |
| Validate YAML config | REST | `POST /api/config/core/check_config` |
| Read/save/delete an automation, script, or scene config | REST (config component) + WS read | `GET/POST/DELETE /api/config/automation/config/{id}` · WS `automation/config` |
| Entity/device/area/floor/label/category registries | WS | `config/entity_registry/list`, ... |
| Helpers (input_boolean, input_number, timer, counter, schedule, tag, ...) | WS | `{domain}/list`, `{domain}/create`, `{domain}/update`, `{domain}/delete` |
| Repairs (issues) | WS | `repairs/list_issues`, `repairs/ignore_issue` |
| What references this entity/device/area? | WS | `search/related` |
| Automation/script traces | WS | `trace/list`, `trace/get` |
| Config entries (integrations) | WS | `config_entries/get`, `config_entries/disable` |
| Dashboards (Lovelace) | WS | `lovelace/config`, `lovelace/config/save`, `lovelace/dashboards/...` |
| Backups | WS | `backup/info`, `backup/generate` |
| History / logbook / error log | REST | `/api/history/period/<ts>`, `/api/logbook/<ts>`, `/api/error_log` |

## Safety Rules for Mutation

1. **Audit read-only first.** Report findings and get confirmation before any write.
2. **Back up before mutating** — WS `backup/generate` (or confirm a recent backup exists via `backup/info`).
3. **Validate after writing YAML-touching changes** — `POST /api/config/core/check_config`, then reload only the affected domain (`POST /api/services/automation/reload`, etc.) rather than restarting.
4. **`entity_id` ≠ `unique_id`.** Registry operations are keyed by registry entry ID or `entity_id`; the config-edit REST endpoints are keyed by the automation/script **config `id`** (from its YAML), not the entity_id.
5. **Prefer disabling over deleting** entities and config entries until the user confirms nothing references them (check with `search/related`).
6. **Deleting an automation/script config** (`DELETE /api/config/{domain}/config/{id}`) also removes its entity registry entry — irreversible without a backup.

## Key Quirks

- `GET /api/states` returns only **current states**. An entity can exist in the registry with no state (device offline, integration not loaded), and a state can exist with no registry entry (YAML-defined or REST-created entities). An audit must join both sources.
- `POST /api/states/<entity_id>` sets a state value directly — it does **not** actuate a device. Use service calls for that.
- Registry `list` results include `disabled_by` (non-null = disabled) and `hidden_by` — filter on these for hygiene audits.
- `config/entity_registry/update` can rename (`new_entity_id`), retarget (`area_id`, `labels`, `icon`, `name`), and disable (`disabled_by: "user"`).
- The `POST /api/config/automation/config/{id}` endpoint **creates** the automation when `id` is new — the same endpoint is used for create and update.

## Detailed References

- [REST API](rest-api.md) — endpoint reference and quirks
- [WebSocket API](websocket-api.md) — protocol, verified command tables, reusable client snippet
- [Configuration Audit Workflows](config-audit.md) — read-only audit recipes and safe remediation playbooks

## Official Documentation

- [REST API](https://developers.home-assistant.io/docs/api/rest/)
- [WebSocket API](https://developers.home-assistant.io/docs/api/websocket/) — covers the core protocol; most config commands are only documented by the [core source](https://github.com/home-assistant/core/tree/dev/homeassistant/components) (this skill's tables are verified against it)
