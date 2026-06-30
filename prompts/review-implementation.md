---
$schema:
    review: file(required; match(**/*review*.md; eager))
    template: file(required; match(prompts/*.md,.claudine/prompts/*.md); eager)
description: Reviews a draft specification and provides feedback to the author on how the spec could be improved



start:
    message: "🥸 reviewing the implementation of the findings from the review `{{parent_dir(review)}}`"
success:
    say: "The review of the draft specification file has completed"
    message: "✅ review of the draft specification '{{spec}}' has completed"
---

## Context

The review findings found in '{{review}}' were based on the follow


The review {{review}} had a number of findings which have now been implemented. Your task is to validate that the implementation
