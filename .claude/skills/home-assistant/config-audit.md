# Home Assistant Configuration Audit & Remediation

Playbooks for assessing and improving a live instance. All steps are **read-only** unless marked `[WRITE]`.

Prerequisites: `HA_BASE_URL` and `HA_TOKEN` in the environment (see SKILL.md). Use the WS client snippet from websocket-api.md for the WS calls.

## 1. Health Snapshot (start here)

| Check | Call |
|-------|------|
| Instance up, version, components | `GET /api/config` |
| Integration health panels | WS `system_health/info` |
| Active repair issues | WS `repairs/list_issues` |
| Config entries in error | WS `config_entries/get` → filter `state != "loaded"` |
| Recent errors | `GET /api/error_log` (tail it client-side) |

`repairs/list_issues` is the instance's own curated problem list — surface every item to the user with severity and `is_fixable`.

## 2. Entity Hygiene

Fetch both `get_states` and `config/entity_registry/list`, then join on `entity_id`:

| Finding | Detection | Typical fix `[WRITE]` |
|---------|-----------|------------------------|
| **Ghost entities** | Registry entry, no current state, and `disabled_by == null` | Check `entity/source` / owning integration; fix integration or `config/entity_registry/remove` if permanently gone |
| **Unavailable entities** | State == `unavailable` or `unknown` for extended time (cross-check `/api/history/period/`) | Fix device/integration; delete only if decommissioned |
| **No area assigned** | `area_id == null` on entity (and its device, via `config/device_registry/list`) | `config/entity_registry/update` or `config/device_registry/update` with `area_id` |
| **Disabled clutter** | `disabled_by == "user"` entries | Review with user; remove or re-enable |
| **Ugly auto names** | `name == null` and `original_name` looks like a serial/MAC, or entity_id matches `_2$`, `_switch$`, hashed suffixes | `config/entity_registry/update` with `name` / `new_entity_id` |
| **Duplicate concepts** | Same device exposing overlapping entities (e.g. `sensor.x_power` from two integrations) | Disable the redundant one via `disabled_by: "user"` |

Before renaming or removing any entity, run `search/related` `{item_type: "entity", item_id}` — automations, scripts, dashboards, and groups that reference it come back in one call.

## 3. Automation & Script Audit

1. `get_states` → all `automation.*` entities; note `state: off` (disabled) and `attributes.current > 0` (currently running).
2. For each, WS `automation/config` `{entity_id}` → full config.
3. Extract every referenced `entity_id` / `device_id` / `area_id` from triggers, conditions, and actions (recursively — actions can contain `choose`, `repeat`, `if/then` branches).
4. Flag configs referencing ids not present in states + registries → **broken references**.
5. WS `trace/list` `{domain: "automation", item_id}` → flag runs with `state: "error"` and automations that have never triggered in the retention window.
6. `test_condition` can evaluate a suspect condition against live state before changing it.

Fixes `[WRITE]`: save corrected config via `POST /api/config/automation/config/{id}`. The save endpoint validates before writing, but still run `POST /api/config/core/check_config` afterward and confirm the automation's state is `on`.

## 4. Helpers & Dead Weight

- WS `{domain}/list` for each helper domain; cross-reference each helper with `search/related` — helpers referenced by nothing and unchanged for months (check `/api/history/period/`) are cleanup candidates.
- `config_entries/get` → entries with `source: "ignore"` or disabled by user; confirm before removal.

## 5. Organization Pass

- `config/area_registry/list` → empty areas, duplicate-ish names ("Living Room" vs "Living room").
- `config/label_registry/list` → unused or inconsistent labels.
- Devices with `name_by_user == null` and cryptic manufacturer names → rename candidates `[WRITE]` via `config/device_registry/update`.
- Floors/areas missing hierarchy: floors with no areas, areas on no floor.

## 6. Remediation Order `[WRITE]`

1. **Back up**: WS `backup/generate` (or verify a recent backup via `backup/info`).
2. Fix integration-level problems first (failed config entries, offline devices) — they resolve many ghost/unavailable entities for free.
3. Structural changes (areas, floors, labels) before entity renames — rename references break silently in YAML configs that the API doesn't manage.
4. Renames last, one batch at a time, re-running `search/related` checks after each batch.
5. After any YAML-touching change: `POST /api/config/core/check_config`, then targeted reload (`automation.reload`, `script.reload`) — never `homeassistant.restart` unless required.
6. Re-run the audit section that motivated the change to confirm the finding is gone.

## Reporting

Present findings grouped by severity:

- **Broken**: repairs, failed config entries, automations referencing missing entities
- **Degraded**: unavailable/ghost entities, erroring traces
- **Untidy**: unassigned areas, naming inconsistencies, unused helpers

Get explicit confirmation per category before any `[WRITE]`, and keep a written record of every mutation made (entity, old value, new value) so the user can review or roll back.
