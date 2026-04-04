---
sequence: "@claudine/providers"
---

# Non-Interactive Sessions with Agents in Claudine

## Context

When {{state.desc}} is run as a **non-interactive** session, it can be instructed to output it's response as _structured data_. This structured data is much more valuable to **Claudine** than just text as we not only get the {{state.desc}}'s response but also lots of useful metadata which we can either respond to or report back to the caller in a well formatted way to help them understand progress and what is happening.

## Research

1. **SCHEMA** (`## Schema`)
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
1. **DOCUMENTATION** (`## Documentation`)
   - Find the URL/URLs for the official documentation for this structured data
   - Find a few popular blog posts are articles which describe how this data is structured or how it might be leveraged
   - Capture all you find regarding documentation into the `## Documentation` section of the document ("{{state.file}}")
1. **CLI** (`## CLI`)

    - most Agents will provide plain text (with Markdown formatting) as their default output for non-interactive sessions
    - this means that to SWITCH to a structured data output format you will need to use the Agent's CLI to specify this format.
    - In the `## CLI` section of the document you will:
        - list out the enumerated set of available output formats that {{state.name}} provides as options
            - indicating what each options translates into in terms of data/information
        - specify what the CLI syntax to used to specify an output format
        - list any known side effects in behavior which might exist when you change the output format

1. **GOTCHAS** (`## Gotchas`)
      - Look across social media, articles, and other locations where you're like to hear developer conversations about their experiences with this structured data

1. **TIMELINE** (`## Timeline`)

    - Document all the major timeline events which you can find for the {{state.name}} agent
    - The timeline should have it's primary focus be on the events over time which related to structured output specifically (e.g., when was it introduced? where more output formats changed later? did the schema become formalized at a later date? Has the schema been versioned since it was introduced?)
    - While the **main** focus needs to stay focused on things relating to structured output, other major timeline events can add context to the timeline and they should be added where they help a user to understand but keep these events to less than 5
    - Write this timeline info to the `## Timeline` section of "{{state.file}}"

1. **USE CASES** (`## Use Cases`)

    - in the `## Use Cases` section of the document we will dig into details around how certain use cases can be detected and worked with in {{state.name}}'s unstructured data
    - each use case will be it's own H3 Heading under the `## Use Cases`
    - the use cases are:
        - **Plan Cap Approaching** (_a message is received that indicates that the user's plan is approaching a CAP in their plan_)
            - What is the event type/types for this kind of event?
            - What are the ways to distinguish it from other events similar to it?
            - Is there a way to extract how much is left before the caller is capped? Is it a percentage? A number of tokens?
            - Is there any way to know what the timeframe is before the plan's cap window is reset?
        - **Plan Capped** (_a message/event is received that indicates that the user's plan has been capped!)
            - What is the event type/types for this kind of event?
            - What are the ways to distinguish it from other events similar to it?
            - Is there a way to extract how much is left before the caller is capped? Is it a percentage? A number of tokens?
            - Is there any way to know what the timeframe is before the plan's cap window is reset?
        - **No Funds** (_a message/event that indicates that the user doesn't have the funds in their account to continue_)
            - What is the event type/types for this kind of event?
            - What are the ways to distinguish it from other events similar to it?
        - **Auth**
            - Is there any way to detect the "kind" auth that the user used (API Key, subscription, other?)
        - **Permissions: Can't Read File**
            - Is there any message/event which indicates that an attempt to read a certain file was attempted but access was denied?
            - How can the file's full path be identified?
            - Is there any indication of a "reason" why access was blocked?
            - What are the ways to distinguish it from other events similar to it?
        - **Permissions: Can't Write File**
            - Is there any message/event which indicates that an attempt to write to certain file was attempted but access was denied?
            - How can the file's full path be identified?
            - Is there any indication of a "reason" why access was blocked?
            - What are the ways to distinguish it from other events similar to it?
        - **Tokens Consumed**
            - What event convey's how many tokens were consumed overall for the session?
            - Are there other events which can evaluate tokens at a more granular level (e.g., per turn, etc.)
            - Do these events also provide some "cost basis" information?
        - **Model Used**
            - Are there events which indicate which model the agent is using?
            - Do these events always get fired or only under certain circumstances?
            - What is the nomenclature that is used for these models? Model abbreviations? Full model names with release date? Is the underlying provider of the model also mentioned?

    - for every use case:
       - indicate whether the event that is being exposed is also exposed as a hook
           - if it is, is the information provided as part of the stream identical to that which is provided by the Hook's event? If not, what is different

1. **SUMMARY** (`## Summary`)

    - Summarize the state of {{state.name}}'s' structured outputs in non-interactive sessions 
    - Write this to the `## Summary` section and make sure the summary is at the top of the document

## Your Task

You will create/update the Markdown file "{{state.file}}" with the **research** described above:

- The body of the document will be well formed, idiomatic Markdown prose content
    - if you wish to provide data visualizations then you should feel free to use `mermaid` diagrams
- In addition you will add the following structured data to the Frontmatter of the "{{state.file}}" document:
    - `schema` - the URL to the best schema definition you were able to find (best is always the provider's if they actually provide one); if not formal schema found then leave blank
    - `schema_type` - if a formal schema definition language was used to define the specification then it should be added here (e.g., `json-schema`, `open-api`, `json-api`, `async-api`, `raml`, etc.)
    - `data_format` - the data format which is returned (JSON, JSONL, NDJSON, etc.)
    - `docs` - the primary URL for documentation on the topic of non-interactive sessions returning structured data
    - `last_updated` - todays date in YYYY-MM-DD (today is: "{{ctx.today}}")
    - if the document DID NOT exist before and you were forced to create it:
        - then add the `created` date and use the same YYYY-MM-DD format (today is: "{{ctx.today}}")
        - if you came into an existing document which you updated then leave the `created` property 'as is'

> **IMPORTANT:** keep the caller/user informed by providing updates to your progress whenever possible
