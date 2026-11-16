---
kind: canonical-documentation

---
# Claudine Configuration Files

There are two Claudine configuration files:

1. **User Scoped:** `~/.claudine/config.yaml`
2. **Repo Scoped:** `{repo-root}/.claudine/config.yaml`

When using Claudine _outside_ of a git repo the only configuration that is used is the _user_ scoped configuration file. However, when inside a repo, the Claudine configuration is a merge of both files where the repo's configuration always overrides the user's configuration on a per-property basis.

The elements of the configuration are strongly typed using [Simplified Schema](simplified-schema/README.md) grammar and are defined with a [schema trigger](simplified-schema/docs/topics/schema-trigger.md)
