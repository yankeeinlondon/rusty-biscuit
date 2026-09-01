# Home Assistant REST API

Base: `$HA_BASE_URL/api/`. All requests need `Authorization: Bearer $HA_TOKEN`. Bodies are JSON.

## Core Endpoints

| Method | Path | Notes |
|--------|------|-------|
| GET | `/api/` | Liveness check; returns `{"message": "API running."}` |
| GET | `/api/config` | Instance config: version, location, unit system, time zone, components |
| GET | `/api/components` | Loaded component names |
| GET | `/api/events` | Available event types |
| POST | `/api/events/<event_type>` | Fire an event |
| GET | `/api/services` | All services and their schemas |
| POST | `/api/services/<domain>/<service>` | Call a service; body e.g. `{"entity_id": "light.kitchen"}`. Returns changed states |
| GET | `/api/states` | All current states (attributes included) |
| GET | `/api/states/<entity_id>` | One state; 404 if no current state |
| POST | `/api/states/<entity_id>` | Set/update a state object directly (does **not** actuate devices) |
| DELETE | `/api/states/<entity_id>` | Remove a state object |
| GET | `/api/history/period/<ISO timestamp>` | State history; query params: `filter_entity_id`, `end_time`, `minimal_response`, `significant_changes_only`. Heavy on large DBs — always filter |
| GET | `/api/logbook/<ISO timestamp>` | Logbook entries; params: `entity`, `end_time` |
| GET | `/api/error_log` | Raw text of the current error log — first stop for integration failures |
| POST | `/api/template` | Render Jinja: body `{"template": "{{ states('sensor.x') }}"}` → plain text |
| POST | `/api/config/core/check_config` | Validate configuration.yaml and related files; returns `{"result": "valid"|"invalid", "errors": ...}` |
| GET | `/api/calendars`, `/api/calendars/<entity_id>` | Calendar events (params: `start`, `end`) |
| GET | `/api/camera_proxy/<entity_id>` | Camera snapshot/stream |
| POST | `/api/intent/handle` | Process an intent |

## Config-Editing Endpoints (config component)

Undocumented on the main REST page but stable — this is what the HA UI's editors use. Keyed by the item's YAML `id`, **not** its entity_id.

| Method | Path | Effect |
|--------|------|--------|
| GET | `/api/config/automation/config/{id}` | Read one automation's config |
| POST | `/api/config/automation/config/{id}` | Save (update **or create**) an automation |
| DELETE | `/api/config/automation/config/{id}` | Delete it; also removes the entity registry entry |
| same | `/api/config/script/config/{id}` | Scripts |
| same | `/api/config/scene/config/{id}` | Scenes |

Behavior notes:

- POST requires the `config` component (loaded by `default_config`). The instance writes to its config storage and reloads the domain; invalid configs are rejected by a validator before writing.
- Deleting is permanent — no undo, no recycle bin. Back up first.
- These endpoints manage file-backed (`automations.yaml`, `scripts.yaml`, `scenes.yaml`) items, which is where UI-created automations live.

## Error Semantics

| Status | Meaning |
|--------|---------|
| 400 | Bad request (e.g. invalid JSON body) |
| 401 | Bad/missing token |
| 404 | Unknown endpoint or entity_id with no current state |
| 405 | Method not allowed |
| 500 | Handler exception — check `GET /api/error_log` |

## Quirks

- `POST /api/services/...` returns a list of states that changed **during the call**; a service that changes nothing returns `[]` — not an error.
- History without `filter_entity_id` on a busy instance can return megabytes; always scope it.
- `/api/error_log` is plain text of the whole log file, not structured records; grep it client-side.
- Service names use the integration's domain (`homeassistant.turn_on`, `automation.reload`, `homeassistant.reload_all`). Reload services exist per-domain: `automation.reload`, `script.reload`, `scene.reload`, `homeassistant.reload_core_config`.
