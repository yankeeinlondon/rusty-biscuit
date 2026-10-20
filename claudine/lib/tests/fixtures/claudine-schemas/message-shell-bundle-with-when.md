---
success:
    - when: "ctx.branch == 'main'"
      message: "merged"
      shell: "git push"
---
# message-shell-bundle-with-when
