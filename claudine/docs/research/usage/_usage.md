---
sequence: @docs/providers.yaml
doc: "@claudine/docs/research/usage/{{state.file}}.md"
---

Being able to understand how much "usage" a subscription plan in it for the short term window (typically 5 hours) or the longer term (typically the week) is **VERY** valuable information. 

Your task is to research how a program can get this usage data for the current user from {{state.desc}}. 

- you will save all your findings to {{doc}}
- the approaches which will be considered are:
    1. API call

        - research if there's an API endpoint which provides this information
        - can all users with a subscription call it? is it only enterprise customers?
        - add a `## API Call Opportunities` section to the document "{{doc}}" and add all information you were able to find out about API opportunities to get usage information for {{state.name}}.
        - if there is a clear way to do this via API set the Frontmatter property `api` on "{{doc}}" to `true`; otherwise set to `false`

    2. CLI switch
        - research if it's possible to get the CLI switches to give us this information
        - maybe if we ask for a JSON response we can get this information?
        - add a section to the "{{doc}}" called `## CLI Switch Opportunities` and report on everything you find out about CLI Switch opportunities to get usage data for the user's subscription with {{state.name}}
        - if there is a clear way to do this via CLI set the Frontmatter property `cli_switch` on "{{doc}}" to `true`; otherwise set to `false`
    3. PTY Scraping 
        - to enable this {{state.name}} would need to have a `/status`, `/usage` command available to interactive sessions
            - sadly these slash commands seem to never be available on non-interactive sessions
        - research using the Rust crate 'expectrl' to scrape the information we want
        - the goal is to produce a mini-design for how expectrl could be used to do a two pass scrape of the content:
            - first pass would be to look for the current reporting shape that the CLI provides
            - the second pass would only be used if the first pass failed (likely due to the reporting have changed) and it would take more of a fuzzy search approach to find the metrics on the page

    4. 

::block when="state.model_provider"

- because {{state.desc}} is developed by a model-provider then you likely have a `/status` or `/usage` slash command
    - this slash command tends to ONLY work in a interactive session
    - that often includes passing in the `/status` or `/usage` as the preliminary command`
    - this makes getting at the data much more complicated then if we can run this as non-interactive prompt

::end-block
::block when="!state.model_provider"

::end-block
