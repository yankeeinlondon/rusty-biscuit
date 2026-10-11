---
failure:
    - say: { text: "the run failed", no_error: true }
    - retry:
        max_attempts: 3
        backoff: exponential
        with:
            reason: "{{ err.msg }}"
    - set_frontmatter: ["d.md", "status", "failed"]
    - append_line: { file: "log.txt", text: "failed", no_error: true }
    - file_exists: "d.md"
    - set: { phase: 2 }
---
# long forms
