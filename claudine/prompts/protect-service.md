# Claudine Protect Service

The Claudine library and CLI will provide a **Protect** service which will work across all supported Agentic CLI's to provide protections from accidental security disclosures or the improper updates or removal files.

The **Protect** service will attempt to provide as much protection as possible without getting in the way of a "trust empowered" modern and efficient Agentic engagement. Attempts will to provide the same or similar levels of protection regardless of the platform used but each platform provides a distinct

## Existing Research

We have gathered a lot of research on each of the Agentic CLI's which we support. These research documents exist in the @claudine/docs/protect directory. Each file covers a different Agentic CLI provider.

> **Note:** for context-window efficient use of these document's frontmatter you can refer to the Typescript type in @claudine/docs/protect/schema.ts for a complete overview of the frontmatter each of these documents contains.

## Design Principles

- the **Protect** service will be defined as a `struct` in a new module in the library called "services" which will act the central actor in both defining and enforcing the policies configured
- the user and repo configurations will have a `protect` property which will be the entry point to the configuration of this service.
- **Protect** will operate in two modes:
    - YOLO mode
        - when a CLI is operating in YOLO mode we will be far more limited in our actions
        - the hooks/event model of a given provider
    - Normal mode

> **NOTE:** In phases _after_ the first implementation of **Protect** we will be adding the ability for Claudine to "wrap" the execution of each of these Agentic CLI's. By doing this we can do things like:
> - sanitizing the ENV variables which are provided to the Agent
> - provide some contextual ENV variables to help decisioning
> - We may be able to sniff out events better and do more in the event model
>
> This future feature is brought up just for future context. It does not have to be designed for initially but hopefully knowing about it may make the design we come up more able to adjust when we add this.


## Task

Your task is to synthesize the existing research into a comprehensive design solution for the **Protect** service. To do this you will act as an orchestrator and follow these steps:

> **Note:** before starting the orchestration make sure there is an empty file at @claudine/docs/protect-service.md with just two sections: `## Provider Details` and `## Protect Service` for our design

1. You will iterate over each Markdown document in @claudine/docs/protect and have each document do the following

    - Review the existing entries in the `## Provider Details` and then add in aspects about the specific Agent that must be considered in the overall design into a H3 heading section under `## Provider Details`. This section should also include a hyperlink reference back to the design document for that provider.
    - Review the existing global design for the Protect service in the `## Protect Service` section and then iterate on that design. Each iteration should look for:
        - ways to make the design cleaner and more ergonomic
        - ensure that the specific requirements that the

    Each document will be run in serial, and on each iteration the design document will:

    - add a new H3 level heading under the `## Provider Details`section
    - the global design found in the `## Protect Design` section will get another iteration on it from a fresh set of eyes and one with the benefit of slightly more provider details to inform this design
    - the sub-agent's MAIN responsibility is to update the design document at @claudine/docs/protect-service.md but it should also provide a summarized update to the orchestrator so this can be shared with the user as a form of update
