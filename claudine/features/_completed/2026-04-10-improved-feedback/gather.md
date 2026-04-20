---
sequence: "@claudine/docs/providers.yaml"
---
# Improved Feedback

When we run non-interactive prompts we rely heavily on getting prompt and informative information fed back to us to understand what progress we're making, what activities are taking place, and whether we might be "hung" or not. All of the Agents supported by Claudine support some form of streaming JSON format which we receive while providing updates to the user, however, there is often information we "have" which we're not currently sharing. This feature is about iterating over each provider and comparing what is "offered" and what we're reporting on. 

## {{state.desc}}

You job is to focus on {{state.name}}:

- start by reading the research on {{state.name}}'s non-interactive sessions at claudine/docs/research/non-interactive-sessions/{{state.file}}
- then familiarize yourself with the current implementation of {{state.name}}'s non-interactive sessions implementation in Claudine
- Now your task is to append to the document claudine/features/2026-04-10-improved-feedback/spec.md 
    - you will be responsible for adding a H2 section entitled `## {{state.name}} Suggestions`
    - add a H3 header underneath your top level H2 section called `### Additional Reporting Opportunities`
        - add a list of opportunities we have with {{state.name}} to report more information to the user
        - for each item, be sure to report on:
            - WHAT information we're adding to the report on STDOUT and STDERR
            - How we should present this to the user and how we can make it as nice looking as possible
            - Whether you feel this information should be sent to STDOUT or STDERR
            - Give a few examples of this kind of data and how that raw data would be converted into the information sent to the user
            - Add a subsection `#### Future Enhancements`
                - What data is _missing_ that ideally would be present to give a fuller picture of this information?
                - Is this missing data available through hook events? Through logging? 
    - add a H3 header called `### Current Problems`
        - list any current problems in the way we're parsing or presenting data for {{state.name}}
    - add an H3 section called `### Other Improvements`
        - add any opportunities you have noticed for better type safety
        - add any opportunities you have noticed for a more ergonomic programmatic experience
        - add any opportunities you have noticed for a clearer or more ergonomic UX for the user of Claudine
        - any areas where test coverage appears to be low and how you suggest we address this
- Remember that you are APPENDING to the `claudine/features/2026-04-10-improved-feedback/spec.md` document, not creating it
    - Be sure to preserve the content that was there before you and just append your content to the end of the current document.

