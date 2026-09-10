---
# establish the prompt's schema requirements
$schema:
    review: file(required;eager) -> the review who's findings will be clarified with the caller
    spec: |-
        file -> The _specification file_ that is acting as the baseline requirements for the review that was conducted. 

        **Note:** reviews are NOT required to be attached to a specification file to be considered valid for this prompt
    design: |-
        file -> The _technical design_ file that provides additional baseline requirements for the conducted review. 

        - when a technical design is provided, it is often provided alongside a _specification file_ that compliments the technical details
        with functional specs
        - technical design docs are **not** required or assumed to be present to work with this prompt
# attach complimentary documents where present
spec: |-
    {{ file_exists(parent_dir(review) + "/spec.md") ? parent_dir(review) + "/spec.md" : null  }}
design: |-
    {{ file_exists(parent_dir(review) + "/design.md") ? parent_dir(review) + "/spec.md" : null  }}
# document lifecycle
initialize:
    stack:
        - action:
              - stderr: |-
                    To clarify the _findings_ in a review this prompt needs a valid **review document** passed in. There is no way to _infer_ what 
                    document was intended without any hints provided.
              - stop
---
