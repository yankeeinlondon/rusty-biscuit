---
failure:
    - when: "err.code == 'cap.rate_limit'"
      then:
          - retry: 3
      else:
          - when: "err.category == 'cap'"
            then:
                - when: "ctx.has_codex"
                  then:
                      - proxy: "@prompts/feature-codex.md"
                  else:
                      - warn: "no codex"
            else:
                - warn: "unrecoverable: {{ err.msg }}"
---
# depth 3
