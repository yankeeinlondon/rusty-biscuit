---
$schema:
    - spec: file(required;eager;match(**/*spec*.md)) -> the specification file that is the basis of the review
    - plan: "file(required;eager;match(**/*plan*.md)) -> the plan file who's implementation you want to review"
    - review: "file(required;eager;match(**/*review*.md)) -> the review file who's findings have now been implemented"
description: |-
    This prompt can be used to perform a variety of review _types_. The specific type of
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
        - when: "spec && ( frontmatter(spec, 'reviewed') == true || frontmatter(spec, 'implemented') == true )"
          action:
              - proxy: "./_reviews/feature-review.md"
        - when: "spec"
          action:
              - proxy: "./_reviews/review-spec-inline.md"
        - when: "plan && file_exists(dirname(plan) + '/' + replace(basename(plan),'plan','spec'))"
          action:
              - proxy: "./_reviews/feature-review.md"
        - when: "plan"
          action:
              - proxy: "./_reviews/review-implementation.md"
        - when: "review"
          action:
              - proxy: "./_reviews/suggestion-review.md"
        - action:
              - warn: "using the parameters passed in, we found no match for a review type!"
              - stop
---

You asked for a review but didn't pass in the right parameters to get proxied to
the right prompt. 

- when the correct parameters are passed this prompt will _proxy_ to the appropriate review prompt.
