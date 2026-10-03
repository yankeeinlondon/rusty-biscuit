# Claudine Configuration

Claudine provides configuration files in both the user's home directory (`~/.claudine/config.json`) as well as in a repo (`{repo-root}/.claudine/config.json`) but currently that configuration is constrained by a schema and also has become very stale as well as being poorly organized. In addition the configuration TUI that Claudine provides is ineffective at addressing the actual configuration that Claudine has grown to support.

## Current Config

The current configuration today is:

::file ./inventory.md

> **Note:** this configuration uses [`SimplifiedSchema`](@darkamtter/docs/topics/schema/index.md) to describe the inventory

## Portable Environment Variables

`biscuit-file`'s `PortablePath` (see `2026-09-30-reusable-path`) writes a file
reference as `{{VAR}}/rest` only for environment variables declared
*portable*: likely present on other hosts and meaning the same thing on each.
It already honors the `PORTABLE_ENV_VARIABLES` environment variable; Claudine
should also let a user declare them in configuration and pass them through
`PortablePath::with_portable_env(names)`.

- a list of variable names (each matching `[A-Z0-9_]+`) in both
  `~/.claudine/config.json` and `{repo-root}/.claudine/config.json`
- **precedence: most specific wins** — repository configuration overrides
  user configuration; the `PORTABLE_ENV_VARIABLES` environment variable is
  always added by `PortablePath` itself
- the key name and placement follow whatever organization this spec settles on
