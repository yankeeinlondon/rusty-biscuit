---
name: "default"
iteration: 0
example: ""
---

# Testing Setting Props (_{{example}}_)

We can set the initial Frontmatter state with either:

- `--set '{ key: "value" }'` (_.e.g, JSON5 syntax_)
- or with _named_ parameters being set like so: `key="value"`

## Results

The 'name', 'iteration', and 'example' Frontmatter properties are set to defaults, but
in this case we are {{example}} so now:

- name: {{name}}
- iteration: {{iteration}}
- example: {{example}}

---
