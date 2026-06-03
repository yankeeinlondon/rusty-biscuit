---
sequence: 
    - name: "start"
    - name: "api-detection"
    - name: "auth-scraping"
    - name: "change-detection"
    - name: "library-detection"
    - name: "page-categories"
    - name: "main-content"
    - name: "metadata"
    - name: "page-categories"
    - name: "variants"
    - name: "snapshots"
    - name: "tantivy"
    - name: "variants"
    - name: "closure"
prompt: |-
    The body of this Markdown document is meant to be a draft model specification for the following entities:

    1. WebPage - a Rust enumeration with both `Simple` and `Complete` variants which both describe all the metadata characteristics of a web page in varying levels of detail.
    2. WebSite - a more macro set of metadata that describes macro metadata about the site as well as reference pages which we have information about. For example, this struct should provide a place to store information about the website's ownership, provide glob patterns to identify different page types on the site, etc.
    3. Company - information about a company
    4. Person - information about a person
    5. Product - information about a product
    6. Place - information about a place/location
    
    ::block when="state.name = 'start'"
    ## Task

    No prior research has been done but the Markdown document has been seeded with the **structure** of the document we are trying to create. Your task is to do broad based research on each of the entities described above and come up with an initial set of Rust structs and enums that shape these entities.

    - each entity has a H2 heading designated for it
    - inside each H2 heading are two sections:
        - `### Schema`
        - `### Notes`
    - you are to keep the structure as is but fill in each H3 level section:
        - the schema section is primarily a Rust code block which offers structs and enums that help to define the section's entity
        - the notes section is for adding a unordered list of bullet points which provide context to the schema, elucidate rational for design decisions, bring up open questions which need more research, etc.

    Because this is the first (of several) passes, these sections are currently empty but you must add a first draft to each H2/H3 section.
    
    ::end-block
    
    ::block when="!contains('start','closure', state.name)"
    ## Task

    A draft of the various entities described above should already exist in this document. Your task is to review the document "./{{state.name}}.md" (which contains research information about a related topic to web page scraping and the Reaper project). Based on the knowledge in the research document, review the existing content in the body of this Markdown document and update the schema where you see fit as well as add additional context to the notes sections.

    Once you've reviewed and updated each section in the document, and SAVED your updates, the task is complete.
    ::end-block
    
    ::block when="state.name = 'closure'"
    We have gone through several iterations of defining schemas for the entities above. We have now reached the final step and your task is to read through each section:

    - look for errors, redundancies, or overlapping ideas and fix inline in the body of this document
    - when you have completed all sections of the document add a new H2 heading at the end: `## Next Steps for Closure`
        - in this section describe how complete you believe these different entities are
        - mention any other entities you feel are missing from the current design
            - missing for the **Reaper** project which is a comprehensive screen scraping project
        - describe any further steps you believe would be helpful in finalizing the schema's of the core Reaper entities.
    ::end-block
---
# Reaper Schemas

## WebPage

### Schema

```rust

```

### Notes


## WebSite
### Schema

```rust

```

### Notes


## Company
### Schema

```rust

```

### Notes


## Person
### Schema

```rust

```

### Notes


## Product
### Schema

```rust

```

### Notes


## Place

### Schema

```rust

```

### Notes
