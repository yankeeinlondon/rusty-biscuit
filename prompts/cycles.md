---
$schema:
    review: file
description: |-
    Performs a review on the current package area for cyclometric risk.
review: {{ ctx.area }}/reviews/{{ ctx.today }}-cyclometric-risk/review.md
start:
    message: "👀  performing a cyclometric risk analysis on the **{{ctx.area}}** package area"
success:
    message: "🔁  the cyclometric risk analysis on **{{ctx.area}}** completed successfully"
    info: "the cyclometric risk analysis on **{{ctx.area}}** completed successfully"
---
