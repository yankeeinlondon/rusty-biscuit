---
$schema:
    spec: file(required)
description: Reviews a draft specification and provides feedback to the author on how the spec could be improved

basename: "$(basename '{{ spec }}')"
dir: "$(dirname '{{ spec }}')"

start:
    message: "👀 reviewing the specification file: {{spec}}"
success:
    say: "The review of the draft specification file has completed"
    message: "✅ review of the draft specification '{{spec}}' has completed"
---
You are expected to review a draft specification document located at {{spec}}.

Provide constructive feedback on how this spec file could be improved:

- what feels like a gap in the scope of this specification
    - suggest ways that this might be addressed
    - you don't need to provide the full design but just describe the design ideas and let the original author fill this in
    - if you think there are a few alternates solutions that might fill this gap, list all of them and describe each option at a high level
- what mistakes the spec is making relative to other contracts or standards that are already established in the repo
    - make sure to distinguish "intended" changes to standards versus accidental
- suggest better wording if you think ideas are expressed unclearly


Write your review to "{{dir}}/review-{{basename}}".
