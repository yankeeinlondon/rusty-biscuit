we have added some important capabilities to Darkmatter's SimplifiedSchema grammar; this was primarily brought in via the following features:

- suggest-constraint (still finishing off the review cycle but core implementation complete)
- single-sourcing-schema
- schema-improvement
- schema-coercion
- compose-schemas
- schemas

In addition we have started to build out Darkmatter's LSP (DMLS) with the features:

- modal-and-autocomplete
- dmls

What I want to now do is two major things:

1. Ensure Solid Documentation

   - we formalize how documentation should be structured throughout this monorepo
   - we will build an advanced Claudine sequence called "prompts/document.md" which will walk through the process of documenting a completed feature/fix specification
   - we will use the changes above to exercise our sequence and iterate on the prompt until we believe it is "fit for purpose"

2. Brainstorm Improvements to Schemas and LSP

    - whereas the documentation section is strategic and will be used going forward for all features/fixes, this section is just allowing us to benefit from the updated documentation on Darkmatter's schema and LSP functionality 
    - with our understanding of the "current state" provided by the recently updated documentation we will then  brainstorm interactively on where we are and what should be added/updated to reach a mature version 1 solution for both areas of functionality
    - this work will result in us creating a new draft specification in @darkmatter/features/2026-07-10-v1-schemas-and-lsp/spec.md

### Task

Both pieces of work outlined above, however, should start out as a specification file: @darkmatter/features/2026-07-10-formalizing-documentation/spec.md ; your primary deliverable is:

1. the draft specification for formalizing documentation
2. a plan -- "plan.md" in the same directory -- which will intelligently bring an agent through 

## Solid Documentation

We use feature (and fix) specifications to create change. Feature specs is where you can expect to find new features as well as major changes to existing features. These documents are a snapshot in time that describe the changes we expect and they are highly valuable in understanding the history of change as well as providing a "current truth" when taking in aggregate. However, evaluating all changes over time is expensive and so we need to start to get in the habit of formalizing the documentation process that hangs off the back of the implementation of a feature or fix specification.

Documentation being created/updated will consist of the following elements:

> **IMPORTANT:** the ideas formulated below are an initial point of view; you are free to propose alternatives where you think that is appropriate as well expand on the ideas here ... this document is primarily meant to provide tiny bit of context and starting structure but mainly just to get the creative juices flowing during the brainstorming process to follow.

- **Feature** documents 
    - every major feature should have a "blessed" document that lives under the "docs" directory of the "package 
      area" where the feature exists
    - note: there may be a few packages in a package area that have their own "docs" directory and in those cases
      this is the most natural root for their feature documents; we _tend_ to use the docs directory off the 
      package area simply because a lot of package areas consist of just a CLI and library and the features 
      supported tend to be highly overlapped between these two packages
    - for any specification file there will be 
        - a 0:M relationship to existing feature documents where the spec is updating functionality which already existed
        - a 0:M relationship to new features documents where the given specification has introduced a new feature
    - A "feature document" needs to be able to both details as well as summary elements.
       - it must cover WHAT is the functionality but also WHY we have this functionality
       - it should provide at least a few examples of how to use this functionality
    - the audience for this kind of documentation is:
        - an Agent (for context on the current functionality of a feature)
        - a human reader (to gain familiarity on a topic or to lookup the "expected" functionality)
        - 
            but also present to a human reader who isn't steeped in symbol names and/or jargon. To ease the burden of trying to keep this balance we recommend that features consider representation not as a single Markdown document but as a "folder" of documents:
       - imagine a feature "dmls-autocomplete"
       - we could fully represent it as `dmls-autocomplete.md` and there is no problem with that if we feel it only really needs a single document. Particularly when a feature first lands, it may be that a single file is the right representation. 
       - if we instead would create a `dmls-autocomplete/index.md` file as the _entry point_ for this feature and then we can add other files that dig into specific areas
           - always make sure the file names you choose have a clear semantic relationship to the content in the file
       - a feature document should always include `features` and `fixes` frontmatter properties:
          - `features` is intended to host a list of the features which went into defining the feature
          - `fixes` is intended to host a list of fixes which went into refining the feature

          > Note: this monorepo names features and fixes with a directory name of `{YYYY}-{MM}-{DD}-{name}` which hosts all files related to the feature/fix. We will move this directory based on it's status (`_completed`, `_unscheduled`) and so the entries in the `features` and `fixes` directories should always just refer to the directory name of the feature as this will always be unique for a package area.

       - a feature document should always include:
           - `area` frontmatter which describe the package area this feature/fix was in
           - `packages` a list of packages which are directly related to providing this functionality; related functionality is NOT criteria for inclusion in this list. In most cases the list will be a single package but there will definitely be some which are defined across packages.
           - `symbols` a list of important symbols used in providing this feature. Doesn't need to enumerate all symbols but the key struct's and enum's that help to provide a feature
           - `kind` frontmatter set to "feature"

        **Note:** a feature that covers both a library feature and a CLI endpoint which exposes this feature should typically be an indication that the feature should be represented as a directory of files (where at least one file is focused on the CLI subcommand). Any files in that directory with a strong focus on the CLI should add a `cli` item to the `tags` frontmatter property (a list of tag strings) 


- **Getting Started** documents
    - Every "feature" will also have a "getting-started" counterpart
    - this document should live under a `getting-started` directory of the docs folder used for the feature document
    - a "getting started" document is a short, easily digested document intended for human readers to read to get an overview of a feature
    - all getting started documents should link to the more detailed **Feature** document (at least one Markdown link if not several)
    - a good "getting started" document:
        - explains in clear language WHAT and WHY this feature has been included
        - provides 2-3 usage examples
        - provides a list of more advanced features also available to advanced users (probably all links to the Feature doc's section on this topic) ... this should act as "teaser" to the reader (aka, "oh that sounds cool" ... so they're encouraged to click through to the detail). Don't oversell anything but just quickly represent these advanced features in short and sweet manner that highlights their utility
    - a getting started document should always include a `feature` frontmatter property which points to the feature document
    - all getting started docs should include a `kind` frontmatter set to `getting-started`

- **Research** documents
   - Research documents are _composable_ documents which provide a prompt to do research as well as a "content policy" that describes how long they are valid for
       - NOTE: the `content_policy` property will host a policy ruleset for determining if a research document is stale but this is FUTURE functionality; will be introduced relatively soon
   - one-off research topics are almost always `inline-compose` documents, but we're starting to add examples of "fleet research" like we are doing in claudine in areas like @claudine/docs/research/skills where we define a `_fleet.md` document which defines a fleet of research by defining a sequence and iterating over an enumeration (providers, local runners, etc.)
   - all research documents should set the `kind` property to "research"
   - this includes the `_fleet.md` like files but also these `_fleet.md` files should make sure to set the documents they produce as `kind: research` too.

- **README.md** documents
   - we place a README.md at the root of the package area as well as any underlying package
   - these documents should provide a summary for their scope:
      - a README.md at the package root should:
          - describe the scope boundary which all packages in this package area strive to serve/address
          - list out the packages in the package area (with a one sentence description for each); each list item should be a Markdown link to the README for the given package
      - a README.md at the root of a package should:
          - describe the package's utility
          - give a simple example
          - provide a list of functionality provided or supported by this package:
             - each feature listed should link to the Feature document detailing this feature
          - provide a backlink to the package area's README

- **Area Skill** documents
   - this monorepo has a long tradition of building an agent skill for every package area (or `packag`e in a few cases); basically there should be largely a 1:1 relationship between `ctx.area` and Agent Skills. 
   - this relationship is health and we want to continue it in manner which:
      - minimizes drift
      - provides a large amount of context for _developing_ in that area of the monorepo
      - keeps token costs low (where we can do that without sacrificing)
      
      > Note: a good example of one of the ways we be token efficient when building out the agent skill is how in Claudine we _summarize_ our fleet research and then "publish" it to the skill using the 'publish-summary-research' just recipe

There may be other documents scattered around but these are the ones we will focus on for the build out of our `prompts/document.md` prompt.

Things to consider when building the prompt:

- will require that a spec be passed in `claudine sequence prompts/document.md spec={filepath}`
   - are there other required or optional parameters needed?
- try to leverage the lifecycle hooks where possible to ensure more deterministic outcomes
- this prompt is setup as sequence, I can imagine the "states" this will go through to look like:

   1. Gather
      - list out the different "features" that the just completed spec updated or created
      - scan existing feature docs to understand the current inventory of features
         - could/should this inventory be cached to a YAML file that provides a feature/description lookup service?
      - build a "updated" and "created" feature list
   2. Associate
      - ask the user to confirm the "updated" and "created" feature lists gathered in prior stage
      - allow user to accept all or "some" of the features
      - ask the user if there were "other features" that were missed in the Gather stage
        - if there were:
            - then ask them to choose the feature docs which were updated by this feature 
                - user is shown all feature docs minus those already associated in the choose_many TUI component
        - then ask if they'd like to name a new feature that hasn't been captured so far
            - if yes then take the "name" and "description" of the feature from the user
   3. Refine
      - if the user added their own feature then we'd need to kick off an interactive "clarify" session to ensure we're representing this feature correctly and adjust any assumed boundaries in other features defined before we added this feature
      - if the user didn't add any features but associated a feature that existed already we should ask a spec-writer to validate that the addition of this feature is clear and that how the new spec interacts with this feature is clear. If it's not then we'd need to move into an interactive session with the user to clarify this.
   4. Feature Updates
      - act as orchestrator and have a subagent focus on a single feature document's updates 
      - you will obviously only assign subagents to features created or updated by the spec
      - these feature docs can be updated in parallel
   5. Getting Started Updates
      - similar process to Feature docs but updating the getting started docs
   6. README.md Updates
      - update the README.md docs that need updating
   7. Agent Skill Updates
      - look for fleet style updates to publish
      - find discussion in the skill that intersects with the features which were changed/created

> **IMPORTANT:** during the first draft of spec assume that what Claudine's "sequence" functionality can be adjusted to include things that you would want/need to perform the task. We ARE planning on updating sequence soon and so we'll feed ideas that make sense into a spec to update sequences. 
> 
> Off the top of my head I would say that allowing sequences to have richer interactive prompting using with a TUI might be useful. Also we want to be able to create a "group" which can be looped over in way similar to how the "loop" lifecycle hook currently loops over a singular document. 

## Brainstorming Schema and LSP Features

This brainstorming session will be informed by the updated documents we will have produced for Darkmatter during the development of the formalized documentation process and the "document.md" prompt. We want to mature both the schema grammar (a little) and the LSP functionality of DMLS (a bit more). To kick off the brainstorming, propose a list of features for each functional area and describe each one in a sentence or two. Let the caller then choose which features to start on.
