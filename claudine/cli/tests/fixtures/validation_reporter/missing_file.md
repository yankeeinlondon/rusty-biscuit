---
prompt: "noop"
pre_checks:
  - file_exists: "definitely-not-a-real-path-xyz.toml"
    message: "{{source_file}} requires definitely-not-a-real-path-xyz.toml"
---

body
