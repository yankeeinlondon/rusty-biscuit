---
dir: claudine/review/2026-04-07-comprehensive

sequence:
    - claude
    - codex
    - gemini
    - glm
    - consolidate
    - plan
---

Use the 'claudine', 'rust', and 'cli' skills.

::block when="state != 'consolidate' && state != 'plan'"

- A review was done on the Claudine package area.
- The contents of the review can be found in the file {{dir}}/review-{{state}}.md
- Your job is to iterate over all of the suggestions in the review and:
    - validate the concerns existence
        - if not a valid issue for any reason explain in detail WHY it is not a concern
    - provide details on how to correct this issue
- When completed with all of the items brought up in the review
    - ensure the {{dir}}/review-{{state}}.md has been updated with your comments
    - provide a summary of your findings to the caller
::end-block

::block when="state == 'consolidate'"

The following documents represent independent reviews which were performed by separate developers:

- {{dir}}/review-claude.md
- {{dir}}/review-codex.md
- {{dir}}/review-gemini.md
- {{dir}}/review-glm.md

You task is to **consolidate** the recommendations across all of the reviews into a consolidated review document you will save to {{dir}}/consolidated-review.md

- when consolidating make sure to look for duplicated findings across the documents and deduplicate
- when deduplicating be extra careful not to loose any important context that was uncovered or discussed in either of the duplicative sections

::end-block

::block when="state == 'plan'"

::file prompts/plan.md 

::end-block
