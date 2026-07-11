# Inline Sections

Today when someone runs `claudine inline-compose <file>` the file reference is only valid when the Frontmatter property 'prompt' has been defined and when that's the case the 'prompt' property is _composed_ and used as an Agentic prompt where the primary output of the Agentic process is meant to update the **body** of the same document referenced by `<file>`.

With this feature, we will add an alternative means of configuring and running the `inline-compose` operation:

- instead of defining the `prompt` Frontmatter property the `sections` frontmatter property must be set instead
- the `sections` Frontmatter property is not a single prompt but rather a series of prompts; an example might be:

    ```md
    ---
    sections:
        - "## Overview": "prompt string"
        - "## Financial Analysis": "prompt string"
        - "## Competitors": "prompt string"
    ---
    ```

    This example demonstrates the _shorthand_ form of the sections configuration:
        - each element in the array is a single key/value pair where 
        - the "key" is the section in the document where the content will be placed
        - the "value" is the prompt which will be used generate content for the given section

    Like the `prompt` triggered variant of an inline-compose, all prompts are _composed_ before being handed over to the Agent for processing.

    The long form variant of this style of inline-compose would look like:

    ```md
    ---
    sections:
        - section: "## Overview"
          prompt: "prompt string"
        - section: "## Financial Analysis"
          prompt: "prompt string"
        - section: "## Competitors"
          prompt: "prompt string"
    ---
    ```

    These two forms both behave identically and the long form -- at least currently -- doesn't expose any additional features which the short form doesn't have access to.

- Execution Semantics

    Since the sections property is always an array and array's have an explicit _order_ then if we run these prompts serially it would be possible for later sections to leverage the content generated in earlier sections.

    Serial execution is the default behavior for the _sections_ functionality and to enable it's full power we will need to add a way to have one section refer to an earlier section:

    ```yaml
    sections:
        - "## Overview": "provide a 2-3 paragraph summary of the company Nike"
        - "## Financial Analysis": "You are providing a financial analysis on the Nike company.\n\n**About Nike:**\n\n{{ section(this, '## Overview') }}"
    ```

    This example shows how we can feed the results of a section's resolved content into the next prompt.

    - this depends on a `section(file, section-name)` function to be added to Darkmatter's expression engine
    - furthermore, we need to introduce the `this` global variable as a context variable available in Darkmatter; `this` represents a file reference to the current document.
        - **Note:** the addition of `this` will be helpful in other places too and should be usable in any function which takes a file reference as a parameter
    - for `this` to work properly in this context we need to make sure that the `section()` function treats it's results as non idempotent (this is different from the default behavior which defaults to idempotent so that we get the benefit of all duplicate queries pointing to the same result)

- Concurrency

    While the ability to chain one section's results into the prompt of following sections is a nice benefit, sometimes the benefit of concurrent execution is more valuable or there is simply no need to pass information forward. In these situations Claudine will provide the option of setting the `mode` Frontmatter property to "parallel" which indicates that all section prompts should be run in parallel.

    A document configured for "section" based inline-composition is either "serial" or "parallel"; no hybrid configurations are allowed.
