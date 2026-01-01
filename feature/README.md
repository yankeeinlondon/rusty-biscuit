# Feature Builder

This area of **Deckhand** provides a library and CLI for building AI features.

## Feature Structure

Let's start at "the end" first so we understand how a "feature" created with this CLI will be structured:

```txt
.
├── .ai
│   ├── features
│   │   ├── YYYY-MM-DD. {feature-name}.md
│   │   │   ├── feature.md
│   │   │   └── specs
│   │   │       └── feature.md
│   │   │       └── plan.md
│   │   │       └── phase1-implementation-log.md
│   │   │       └── phase2-implementation-log.md
│   │   │       └── phase3-implementation-log.md
│   │   └── README.md
│   ├── archived-features
```

## CLI Basics

### Create a new Feature

```sh
feature new "do something"
```

This command -- _using the `new` command_ -- will do the following:

1. Initial Prep (one time)

   - if your project doesn't already have a `.ai/features` folder that will be created and a `README.md` file will be created describing the intention of this folder. It will also create a `metadata.json` file with the following information:
       - whether this project is part of a monorepo or not
           - details on all _peers_ to this module if in module directory of a monorepo
           - details to all _children_ of this module/directory if there are modules defined below this directory.
       - The programming language used in this part of the repo
           - this can be varied if their are child modules but if not then it is expected to be only one
           - an attempt will be made to detect the programming language but if their is uncertainty then the user will be asked interactively.


   > **Note:** a _peer_ or _child_ monorepo will be described by:
   > 
   > - name
   > - programming language
   > - summary description
   > - filepath (relative to root of repo)

   - if your project doesn't already have a `.ai/archived-features` folder 

2. Feature Prep (per feature)

   - create a folder at `.ai/features/{YYYY}-{MM}-{DD}. {feature_name}.md`
       - Date Info
           - The date information is self explanatory but added so that your OS's sorting capabilities will have a reasonable "natural sort order" to it. Later on it will also provide contextual information that will help you remember 
       - Feature Name
           - The users feature name will be mildly treated:
               - all characters lowercase
               - leading and trailing whitespace removed
               - internal whitespace reduced to a single space and then made into a **kebab-case** variant (e.g., `my white pony` -> `my-white-pony`)
           - This is just to produce some uniformity and will not limit the AI features in any way.
   - create an empty folder called `specs`
       - This document is intended to be filled with as many supporting documents as you see fit.
       - Often this will be research or suggestions on packages you expect to use to achieve the feature's outcome
       - There is no required structure or naming convention to what you put in this folder but you will be expected to have the primary content be Markdown files (with `.md` file extension)
       - If those markdown files include image references then the images will be included when passed to the AI
   - create an empty file called `feature.md`
       - This is the "entry point" to your feature and is where you should describe your feature requirements.

### Plan a Feature

```sh
feature plan "${feature-name-subset}"
```

- You can pass any _subset_ of a feature's name to start the planning process
    - if your subset does not uniquely isolate a single feature then the available features will presented and you'll be interactively asked to choose from those options which have this subset as part of their name
        - Note: passing in **no** feature name information is also an option and you'll be presented with _all_ the features to choose from

#### How Planning Works

1. Feature Improvement

   ```mermaid
   ---
   title: FEATURE IMPROVEMENT
   displayMode: compact
   ---
   flowchart LR
       Start@{shape: subproc, label: "Start"}

       FlagSkip{skip}
       AutoAccept{auto accept}
       Improve[\Improve\]
       Ask{Ask}

       Start-->|plan| FlagSkip

       Save@{shape: notch-rect, label: "save"}

       
       FlagSkip -->|true| Ready
       FlagSkip -->|false| Improve

       Improve --> AutoAccept

       AutoAccept -->|true| Save
       AutoAccept -->|false| Ask;

       Ask-->|true| Save;
       Ask-->|false| Stop;

       Save --> Ready

       Ready@{ shape: stadium, label: "Ready" }
       Stop@{ shape: dbl-circ, label: "Stop" }
   ```


   - we will use an LLM to look at your existing feature prompt in `feature.md` along with all other "context files" (see list below) to make the prompt better:
        - the spec files you've provided
        - the `metadata.json` for this area of the repo
        - the `README.md` file for this area of the repo
        - the `README.md` files in any _children_ of this area
        - the dependencies found in either this area or in child areas
     > **Note:** if the `--skip-improvement` flag is included then this step will be skipped

  - once we have an "improved version" you'll be presented this improved version in the console and asked if you'd like to:

      1. Accept this improvement and continue
      2. Ignore this improvement and continue
      3. Edit this improved version and then continue
      4. Stop

    > **Note:** if the `--accept-improvement` was passed in OR the environment variable `ACCEPT_IMPROVEMENT` is set then the improved prompt will be saved back to the `feature.md` without any interactive involvement

2. Planning sent to Agent

    - a

3. Plan Validation

    - b
