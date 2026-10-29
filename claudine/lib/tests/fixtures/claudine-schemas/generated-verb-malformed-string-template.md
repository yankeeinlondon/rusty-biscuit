---
start:
    - ensure_file: "{{ 1 + }}"
    - trim: { text: 'hello {{ 1 + }}' }
    - ensure_file: [notes.txt, 'out/{{ 1 + }}.md']
---
# generated-verb-malformed-string-template
