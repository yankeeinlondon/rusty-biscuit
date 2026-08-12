---
$schema:
    plan: file(required;eager;match(**/*plan*.md)) -> the plan which is being reviewed
    spec: file -> a spec file that the plan was based on
    design: file -> a design file that the plan was based on
    review: file -> a review that the plan was based on
description: >-
    This prompt will _review_ a plan before it's executed:

    - if there are any related docs such as a `spec`, `design`, or `review` document that the plan was based on then this review will try to identify those files and give the reviewing agent the context which these supplemental docs provide.
    - you may also explicitly pass in file references to these docs if the naming convension is non-standard

--
