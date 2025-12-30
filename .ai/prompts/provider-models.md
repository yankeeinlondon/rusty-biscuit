# Task

This content is intended to be used as input to the planning process. The output should be a detailed multi-phased plan.

- this plan will effect multiple modules in this monorepo, the following sections will address each module separately

## Shared Library

Review the code-review comments in @.ai/code-reviews/20251230.provider-base-implementation.md as input to the planning process and then create a plan to refactor the "provider" and "provider/model" functionality exposed in the shared library. 

The goal is to provide a solution where:

### ProviderModel

- there is an enumeration `ProviderModel` which has every known model -- at a given point in time -- enumerated.
    - this enumeration will be used in code bases as a _typed_ reference to a known model
    - the `ProviderModel` _might_ look something like:

        ```rust
        enum ProviderModel {
            // example of a normal provider's model
            Anthropic__ClaudeOpus_4_5_20251101,
            Anthropic__ClaudeSonnet_4_5_20250929,
            // example of an "outlet" which allows users
            // to _try_ to use an undocumented model; this
            // allows bleeding edge models which have not 
            // yet be added statically to the library
            Anthropic(String),
            // example of an aggregator's model offering
            OpenRouter__Anthropic__Opus_4_5_20251101
            // example of an aggregator's "outlet"
            OpenRouter(String)
        }
        ```

    - the `ProviderModel` should implement the **TryFrom** trait so that 
        - `String` or `&str` like `anthropic/claude-opus-4-5-20251101` can be converted into the appropriate element in the enumeration.
        - if a string's name can map to a statically defined enum variant based on naming standard conversion then we use that to create a successful match
        - if there is no direct matching then we will _confirm_ that the string is a valid model by making an API call to the provider's `/model` endpoint on their OpenAI API
        - if we can get a confirmation that the model DOES exist then we return it as a `PROVIDER(String)` outlet variant.
        - if the API is inaccessible, doesn't return in a reasonably short timeout window, or does not provide a match to the value passed in then this should fail.

    - the `ProviderModel` should NOT be created manually but rather as an act of "meta programming". This will allow us to easily (likely on a build) update the known models into the enumeration.
        - you may want to use the `inject_enum()` function in the shared library to help achieve this but if there is a better way of doing this then just make sure the rationale for this other approach is documented
    - the `ProviderModel` should provide an implementation for `update()` which leverages the `get_all_providers_models()` (see below) to update the enumeration's definition of itself. Key requirements to the update process:
        - the update process may not have access to some providers due to missing API Keys in the environment so we should never remove provider models because we can't connect to the provider at that point.
        - if an aggregator or OpenRouter or ZenMux detects new models we can and should update the underlying provider's models too:
            - If in the future we find that OpenRouter has a model called `anthropic/claude-opus-4-6-20260301` and we do NOT currently have the API key for Anthropic directly, it is safe to add the `Anthropic__ClaudeOpus_4_6_20260301` variant as well as the `OpenRouter__Anthropic__ClaudeOpus_4_6_20260301` variant to the enumeration.
            - If we DO have direct access to the underlying provider's API then "hints" from aggregators like OpenRouter should not be applied directly to the underlying provider. In the example in the previous bullet point, we would add `OpenRouter__Anthropic__ClaudeOpus_4_6_20260301` when it's found in OpenRouter but we would NOT add `Anthropic__ClaudeOpus_4_6_20260301` _because_ OpenRouter hinted that it's available. Instead we would interrogate Anthropic directly to see what new models exist.

### API module

- I have created a directory for the `api` module and a README.md to very briefly describe this area.
- The main goal is to have a module which provides both:
    - lower level primitives and types for API's in general (e.g., `ApiAuth`, etc.)
    - as well as API specific utilities like `get_provider_models_from_api()`, etc. which can make API calls for you.
- As part of this planning process we must have a function like `get_provider_models_from_api()` which will get models from a particular provider.
- We will also need the following:
    - `get_all_provider_models()` which iterates over all _available_ providers (aka, those where an API Key is available) and get's all the models available across these providers
    

**NOTE:** we want to be sure that our implementation -- after executing this plan -- is completely DRY. No duplicative functions!

**NOTE:** at the completion of the implementation we need to make sure documentation is up-to-date and any new documents which are required are created and accurate. 


## Research Area

- Fix naming of custom prompts
    - I thought there had been some work on this and you may find some -- likely dead code -- intended to address this problem, but, the issue is:
        - when a user provides one or more "custom questions/prompts" with the `research library <pkg> [...prompt]` command it must determine a filename to use for storing the prompts response.
        - currently we use a generic naming convention of `question_1.md`, `question_2.md`, etc.
        - these names have no semantic meaning and we DO want them to have semantic meaning just like the core research documents we always create have meaningful names.
        - to achieve this we should do the following:
            1. create a utility function `extract_prompt_name(prompt)` which:
                  - looks at the front of the prompt for text which looks like:
                      - `{filename} -> ${prompt}`
                  - the idea is that if a user wants to name their prompt themselves they can do so by putting in the filename followed by the `->` marker. 
                  - the filename can not have spaces in it (all leading and trailing spaces will automatically be trimmed). And although the `.md` extension is not required it is generally a good idea.
                  - the filename (with the `.md` added if the user omitted this) is then established and it must be quickly validated:
                      - The filename can NOT be any of the core underlying research documents which are always created
                      - If it is a conflicting name will be ignored and we'll move onto the second approach of using an LLM to name the file. However, we will log to STDERR `- a custom prompt expressed an invalid filename (${name}) which conflicts with the core research docs being produced.`
                  - this function should return a tuple: `( prompt, name )`, where:
                      - `prompt` is now the prompt _without_ the naming prefix section (e.g., all content after the `->`)
                      - of course if there is no naming expression added to the prompt then the prompt remains unchanged
                      - if a filename was successfully extracted then this will hold the filename but if not then it is not defined (`Option<String>`)
            2. after calling the `extract_prompt_name(prompt)` function, if the filename was not able to be resolved then we will need to use an LLM to help us name it:
               - we should create a function called `async prompt_filename(prompt)` which will asynchronously send the prompt to an LLM to summarize into the prompt into a compact 3-5 word expression. 
               - when the LLM has provided the 3-5 word expression we will convert it to snake_case naming convention and add the `.md` file extension.
               - assuming that no other file exists with this name for the given research topic (validation should look at the `metadata.json` file) then this will be returned as our name.
               - if the name HAS been used before then we will just append `_1`, `_2`, etc. until we have a unique name.
