---
sequence: 
    - macos
    - linux
    - windows
    - cross-os
    - summarize
agent: claude
model: |-
    {{
        seq.state == 'summarize'
            ? 'opus'
            : 'sonnet'
    }}
---
::block when="seq.state == 'macos'"


::end-block
::block when="seq.state == 'linux'"

::end-block
::block when="seq.state == 'windows'"

::end-block
::block when="seq.state == 'cross-os'"

::end-block
::block when="seq.state == 'summarize'"

::end-block
