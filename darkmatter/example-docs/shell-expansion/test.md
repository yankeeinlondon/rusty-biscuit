---
uname: "$(uname)"
pwd: "$(pwd)"
combined: "The current directory is {{pwd}} and the OS is {{uname}}"
---

# Shell Expansion

## Greet

::shell ls -la ../

## From the Heart

Ok not the _heart_, I meant the Frontmatter:

- OS: {{uname}}
- CWD: {{pwd}}

{{combined}}
