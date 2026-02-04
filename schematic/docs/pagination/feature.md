# Pagination in Schematic

**IMPORTANT:** you must use the 'schematic' skill for this task!

Currently we do not support pagination as a first class citizen in the schematic packages(`schematic/define`, `schematic/gen`). During the implementation of this feature we will:

- provide a comprehensive solution for defining the pagination strategy which the API uses
    - Since many API's will use the same strategy across many endpoints of their API we will allow a strategy definition at the "API level"
    - When defining specific endpoint's we will allow for them to adopt the default strategy (possibly with refinement to promote better reuse of this default strategy) but we will also allow a bespoke pagination strategy at the individual (`Pagination::Default`, `Pagination::`)

- Focus on ergonomic and self-documenting solutions where ever possible
    - the ergonomics are important in both the API design (using `schematic/define` primitives) but even more critical when generating the resultant schema (using `schematic/gen`)
    - do NOT overlook refinements which need to be made in the `schematic/gen` module to ensure that the resulting API client benefits fully from your `schematic/define` primitives!

