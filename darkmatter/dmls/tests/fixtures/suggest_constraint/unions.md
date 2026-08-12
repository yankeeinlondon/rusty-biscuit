---
$schema:
  - choice:
      - boolean
      - string(suggest(first, second))
    root: string(suggest(arm-one))
  - root: string(suggest(arm-two))
choice: se
root: ar
---

# Union selection
