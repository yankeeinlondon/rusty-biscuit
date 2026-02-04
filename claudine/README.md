# Claudine

> Claude Code's ex-girlfriend who knows Claude's inner secrets but is now dating other Agents

## Functional Goal

This library takes a JSON configuration which is Agent neutral and allows you to consistently respond to "hooks/events" across the major Agentic CLI's while at the same time ensuring that all providers who support "skills" have their repo-scoped skills linked.

## Usage

This application provides a CLI based interface for interaction and is built around a user's configuration which is stored in `~/.hooker` as JSON content.

See the [CLI](./docs/cli.md) document for more details on the commands provided.

## Key Dependencies

Uses the following libraries from this monorepo:

- `darkmatter` for rich markdown rendering and composition
- `biscuit-speaks` for TTS functionality
- `unchained-ai` to allows actions to interact with AI (models and agents)
