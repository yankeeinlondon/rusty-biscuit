# Claudine

![Claudine|30](../assets/claudine-512.png)

> Claude Code's ex-girlfriend who knows Claude's inner secrets but is now dating other Agents

## Feature Flow

Run `just flow` from the `claudine` package area to drive a feature directory from `spec.md` through clarify, design, planning, implementation, commit, review, and review-repair loops.

Use `just flow <filter>` to target a specific active feature or fix directory. The flow stores agent choices, `flow_iteration`, `flow_stage`, and the current plan `start_phase` mirror in `spec.md` frontmatter so reruns can skip completed artifacts and resume from the next incomplete stage.
