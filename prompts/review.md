---
$schema:
    - spec: file(required;eager) -> the specification file that is the basis of the review
    - plan: "file(required;eager) -> the plan file who's implementation you want to review"
    - review: "file(required;eager) -> the review file who's findings have now been implemented"
description: |-
    This prompt can be use to perform a variety of review types. The specific type of
    review is determined by the parameters the user passes in and the frontmatter state
    of files in the filesystem.

    1. **Spec Review**

        - pass in a `spec` parameter to a valid specification file
        - the spec file will be evaluated and if the `review` Frontmatter property is _not_ **true** then we will run the spec review
        - a spec review will make _inline_ changes to the spec file (versus 
    
    2. **Review of the Implementation of a Spec**

        - pass in a `spec` parameter after implementing the spec to have the implementation reviewed relative to what the _spec_ specified
        - **Note:** the spec's `reviewed` or `implemented` property must be set to **true**
    
    3. ** Review of the Implementation of Review Findings**

        - pass in a `review` file reference to a review who's findings you have now implemented
        - the review will create a review _iteration_ with findings focused on what left to be done the original review finding and/or any additional new findings
initialize:
    stack:
        - when: "spec && ( frontmatter(spec, 'reviewed') == true || frontmatter(spec, 'implemented') == true"
          action:
              - proxy: "prompts/_reviews/review-feature.md"
        - when: "spec"
          action:
              - proxy: "prompts/_reviews/review-spec-inline.md"
        - when: "plan && file_exists(dirname(plan) + '/' + replace(basename(plan),'plan','spec'))"
          action:
              - proxy: "prompts/_reviews/review-feature.md"
        - when: "plan"
          action:
              - proxy: "prompts/_reviews/review-implementation.md"
        - when: "review"
          action:
              - proxy: "prompts/_reviews/review-suggestions-implementation.md"
---

You asked for a review but didn't pass in the right parameters to get proxied to
the right prompt.
