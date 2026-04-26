---
pkg: "{{ctx.current_package_area}}"
file: "{{pkg}}/reviews/{{ctx.today}}-dry-review/review.md"

success: "🍸 the DRY review `{{file}}` has completed"
failure: "❌ the DRY review for `{{file}}` failed to complete!"
---
Review the {{pkg}} package area and focus on opportunities to:

- Identify duplicated logic that should be extracted into a shared function, method, hook, trait, macro, or utility.
- Look for repeated conditionals or branching logic that could be centralized behind a clearer abstraction.
- Check whether similar data transformations are implemented in multiple places with small variations.
- Flag repeated constants, magic strings, enum-like literals, configuration values, or protocol names.
- Verify that validation rules are defined in one place rather than reimplemented across callers.
- Check whether error handling, logging, retry behavior, or fallback logic is duplicated.
- Look for copy/paste code where only names, paths, labels, or types differ.
- Identify repeated test setup code that could be moved into fixtures, builders, helpers, or factories.
- Check whether repeated CLI/API argument parsing patterns can be represented declaratively.
- Look for repeated serialization/deserialization logic that could use shared types or schemas.
- Verify that business rules are expressed once and reused rather than inferred independently in multiple modules.
- Check whether duplicated documentation comments or examples indicate duplicated concepts in the implementation.
- Prefer abstractions that reduce meaningful duplication, but avoid abstracting coincidental similarity too early.
- Ensure any proposed abstraction improves readability and locality rather than merely reducing line count.
- Watch for “almost duplicate” code paths that may diverge over time and create inconsistent behavior.
- Confirm that shared abstractions have clear names, narrow responsibilities, and do not introduce excessive coupling.
- Prefer parameterization, composition, or small helpers over large generic abstractions when the variation is simple.
- Verify that tests cover the shared abstraction once and cover important call-site-specific behavior separately.
- Check whether duplicated type definitions or interfaces should be consolidated or generated from a common source.
- Look for repeated state-machine transitions, lifecycle steps, or workflow stages that could be modeled explicitly.

To finalize the task:

- Finalize the review document: 
    - Make sure each of your review suggestions are categorized as one of the following:
        - CRITICAL
        - URGENT
        - IMPORTANT
        - NICE-TO-HAVE
    - For each suggestion give a High/Medium/Low level of effort required to implement this suggestion
    - Save your review recommendations to "{{file}}"
- set the `recommendations` frontmatter on "{{file}}" to the total number of suggestions you came up with
- set the `critical` frontmatter property on "{{file}}" to the total number or suggestions you feel are CRITICAL to get into the production code. Note: this kind of review doesn't typically surface too many CRITICAL suggestions because the nature of this review is more to "optimize" than "fix" but there it is possible that this review surfaced a logic problem, used poor type safety, 
