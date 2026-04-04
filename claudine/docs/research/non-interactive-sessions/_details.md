---
sequence: "@claudine/providers"
file: "./{{state.file}}"
---

# Non-Interactive Sessions with Agents in Claudine

## Context

When {{state.desc}} is run as a **non-interactive** session, it can be instructed to output it's response as _structured data_. This structured data is much more valuable to **Claudine** than just text as we not only get the {{state.desc}}'s response but also lots of useful metadata which we can either respond to or report back to the caller in a well formatted way to help them understand progress and what is happening.


## Research

1. **SCHEMA**
      - Search the site ({{state.site}}) for a schema definition for this structured data
          - Documentation is great but it's not going to be as useful to us as a formal specification (JSON Schema, Open API, Typescript Type, Rust Struct/Enum, etc.)
          - if there is no mention of a schema on the official site then do searches more broadly to see if there is a well respected project that has potentially "formalized" this for us.
              - Beyond just a broad based search be sure to check projects like Vercel's AI SDK, LangChain's Typescript codebase, etc.
      - In the research document you're creating you will put what you find about formal or informal schema's into the `## Schema` section of the document
          - Be sure to communicate not only the schema but also what the source was, any useful URLs relating to this, and the schema definition language used
      - There might be some cases where a lot of "examples" are given in documentation but **not** a formal schema; 
          - This is NOT ideal so always exhaust all attempts to find a formal syntax for this format
          - But after not finding anything you should report that no formal specification appears to exist
              - List all the places where you looked for a specification
2. **DOCUMENTATION**
      - Find the URL/URLs for the official documentation for this structured data
      - Find a few popular blog posts are articles which describe how this data is structured or how it might be leveraged
      - Capture all you find regarding documentation into the `## Documentation` section of the document ("{{file}}")
3. **GOTCHAS**
      - Look across social media, articles, and other locations where you're like to hear developer conversations about their experiences with this structured data
      - 

## Your Task

You will create/update the Markdown file "{{file}}" with the **research** described above:

- The body of the document will be well formed, idiomatic Markdown prose content
    - if you wish to provide data visualizations then you should try to use `mermaid` diagrams
- In addition you will add the following structured data to the Frontmatter of the "{{file}}" document.

Can output a stream of structured JSON/JSONL data instead of just text when we run in a non-interactive mode. 




JSONL output when the `--output-format stream-json` flag is included. In non-interactive sessions which claudine wraps this is much more valuable than just text as it provides metadata we wouldn't get otherwise.
