---
failure:
    - when: "err.category == 'cap'"
      then:
          warn: "usage cap hit"
          proxy: "@prompts/feature-codex.md"
          no_error: true
      else:
          - warn: "{{ err.msg }}"
---
# bundle in then
