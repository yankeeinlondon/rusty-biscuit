### Review

We have just implemented the "{{feature}}" feature and all tests are passing but your task is now to review the implementation and make sure that it has faithfully implemented everything described in:

- [The Functional Specification]({{base_dir}}/spec.md)
- [The Technical Design]({{base_dir}}/spec.md)

During your review you should:

- look for gaps between the intended functionality versus what was actually changed
- look for ways in which the code could be more idiomatic or ergonomic
- ensure that all functionality has strong test coverage
- ensure that if this package area has both a CLI and Library that these two package have clear separations of concerns:
    - the Library should own all business logic
    - the CLI should be focused on reporting to the terminal and leveraging the library for data and logic
- finally ensure that all public/exported symbols are adequately annotated with doc based comments

Save your recommendations to: "{{base_dir}}/review.md"

