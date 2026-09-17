---
description: |-
    A prompt for _routing_ fixes on various types of files:

    - `prompt` 
        - if passed a valid prompt file then it will be evaluated for any schema errors and fixed
        - if you want more wholistic advice on how to make a prompt better and not just strictly "make this prompt pass all schematic and structural issues"
$schema:
    prompt: file(match(**/*.md))
    issue: string
---
