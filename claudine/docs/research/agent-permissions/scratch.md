  Option A — Project level (.claude/settings.local.json)

  Add the two entries that cover it:

  "WebFetch",
  "Bash(curl:*)"

⏺ Update(.claude/settings.local.json)
  ⎿  Added 2 lines
      27        "Bash(git log:*)",
      28        "Bash(rustc:*)",
      29        "WebSearch",
      30 +      "WebFetch",
      31 +      "Bash(curl:*)",
      32        "WebFetch(domain:docs.rs)",
      33        "Bash(claudine skills:*)",
      34        "Bash(/Volumes/coding/personal/rusty-biscuit/target/debug/claudine skills:*)",

⏺ Option B — Global (~/.claude/settings.json)

  To make WebFetch always allowed everywhere without having to add it per-project, add to ~/.claude/settings.json:

  "WebFetch",
  "Bash(curl:*)"

  Those two entries go in the permissions.allow array. I'll leave this to you since it's a global setting.

  ---
  Option C — Claudine config (orchestrator level)

  The permission_request binding in ~/.claudine/config.json currently only plays a sound. A call action can return a blocking
  decision. The simplest form using echo:

  "permission_request": {
    "enabled": true,
    "actions": [
      { "type": "log", "target": { "type": "file", "rotate_daily": true } },
      { "type": "sound_effect", "name": "doorbell-2", "volume": 1.0, "speed": 1.0 },
      { "type": "call", "command": "echo", "args": ["{\"decision\": \"allow\"}"] }
    ],
    "matcher": null
  }

  Important caveat: this auto-approves all permission requests, not just WebFetch, because the claudine matcher doesn't filter
  PermissionRequest by tool name. Looking at matcher.rs:26:

  // For all other events, matcher has no field to match against
  _ => true,

  PermissionRequest falls into the _ arm, so any matcher pattern is ignored and always returns true. This is a gap — the event
  carries a tool_name but the matcher doesn't use it for PermissionRequest. Fixing that is a separate issue.

  ---
  Option D — Subagent level (subagent frontmatter)

  For subagents specifically, the tools frontmatter field controls their allowed tools:

  ---
  name: web-research-agent
  description: Researches external URLs
  tools: WebFetch, Bash(curl:*), Read, Write
  ---

  This only covers that specific subagent. The orchestrating session (the --fp prompt) still needs permission from settings.
  Subagent frontmatter cannot grant permissions to the orchestrator.

  ---
  Summary

  ┌──────────────────┬─────────────────────────────────────────────────────────┬───────────────────────────────────────────────┐
  │      Level       │                           How                           │                     Scope                     │
  ├──────────────────┼─────────────────────────────────────────────────────────┼───────────────────────────────────────────────┤
  │ Global           │ Add "WebFetch", "Bash(curl:*)" to                       │ All projects, all sessions                    │
  │                  │ ~/.claude/settings.json permissions.allow               │                                               │
  ├──────────────────┼─────────────────────────────────────────────────────────┼───────────────────────────────────────────────┤
  │ Project          │ Already done — added to .claude/settings.local.json     │ This repo only                                │
  ├──────────────────┼─────────────────────────────────────────────────────────┼───────────────────────────────────────────────┤
  │ Claudine config  │ Add call action to permission_request binding           │ All sessions via claudine, but approves ALL   │
  │                  │                                                         │ tools indiscriminately                        │
  ├──────────────────┼─────────────────────────────────────────────────────────┼───────────────────────────────────────────────┤
  │ Subagent         │ tools: WebFetch, Bash(curl:*) in agent .md file         │ That subagent only, not the orchestrator      │
  │ frontmatter      │                                                         │                                               │
  └──────────────────┴─────────────────────────────────────────────────────────┴───────────────────────────────────────────────┘

  Recommended: The project-level edit (already applied) handles the immediate case. The global setting is the right long-term answer
   since web research is generally low-risk and you don't want to add it to every project.
