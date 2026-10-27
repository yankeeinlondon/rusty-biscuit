---
area: schematic
status: unscheduled
created: 2026-10-23
owner: Ken Snyder <ken@ken.net>
origin: review 2 of 2026-10-17-schematic-websocket
related:
    - 2026-10-17-schematic-websocket
packages:
    - schematic-definitions
---

# Schematic WebSocket Follow-ups

Items still open after the final review of `2026-10-17-schematic-websocket`.
That review found the feature production ready with no findings. It recorded
the one observation below, which is left for separately scheduled work. See
[review 2](../../../features/2026-10-17-schematic-websocket/review-2.md)
for the full context.

## R2-O1: Dock protocol skill contains an older wire format

**Defect.** The Unfolded Circle Dock API reference in the `unfolded-circle`
agent skill describes an older Dock wire format: messages carry `kind`
(`req`/`resp`), a `msg` name, and a nested `payload` object, and the
system-information command is `get_status`. The recorded UCD2 frames and the
repaired Dock definition use `type` (for example `type: dock`), a `command`
field with its parameters at the top level of the message, and
`get_sysinfo`. An agent that consults the skill while editing the Dock
definition can reintroduce the envelope shape that review 1 found decoding
recorded replies wrongly (R1-F2).

**Where.** `.claude/skills/unfolded-circle/dock-api.md` (1158 lines), starting
at the "Request Message Structure" section near line 186, with more
`"kind"` and `get_status` uses across the skill's files. The authoritative wire
format is `schematic/specs/unfolded-circle/UCD2-asyncapi.yaml`, and the
definition is `schematic/definitions/src/unfolded_circle/dock_ws/`.

**Why not fixed now.** It is older knowledge-base content, not code this
feature added or changed, and no runtime behavior depends on it. Correcting it
means rewriting a large reference document, and a review observation is not
repaired in the follow-up pass.

**What fixing it looks like.**

- Rewrite the message-structure, command, and event sections of `dock-api.md`
  against `UCD2-asyncapi.yaml`: `type` discriminator, top-level command
  parameters and event data, `get_sysinfo`, and the `auth_required` /
  `auth` / `authentication` handshake.
- Sweep the rest of the `unfolded-circle` skill (`SKILL.md`,
  `api-clients.md`, `api-model-rs.md`) for the same `kind` / `payload` /
  `get_status` vocabulary and correct it.
- Point the skill at the generated `unfolded_circle_dock_ws` client in
  `schematic-schema` as the typed way to speak the protocol.
