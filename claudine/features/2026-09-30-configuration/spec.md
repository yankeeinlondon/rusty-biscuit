# Claudine Configuration

Claudine provides configuration files in both the user's home directory (`~/.claudine/config.json`) as well as in a repo (`{repo-root}/.claudine/config.json`) but currently that configuration is constrained by a schema and also has become very stale as well as being poorly organized. In addition the configuration TUI that Claudine provides is ineffective at addressing the actual configuration that Claudine has grown to support.

## Current Config

The current configuration today is:

::file ./inventory.md

> **Note:** this configuration uses [`SimplifiedSchema`](@darkamtter/docs/topics/schema/index.md) to describe the inventory
