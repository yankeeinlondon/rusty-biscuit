---
prompt: |-
    Your task is to research the "Unifi Web App/Website" which Unifi provides to support all of it's product lines.

    - this application is responsible for tying together all of Unifi's services together into a single pane of glass; this is not easy feat so start by discussing about how the various service offerings are expressed to the user from a logical site map perspective, a UI standpoint, etc.
        - be sure to mention any parts of the application where the individual service offerings are shown together as part of a single page (like "updates", etc.)
    - talk about the functional coverage that the web application provides versus the individual mobile applications that focus on a _per service offering_ scope
    - talk about how the version of the web application relates to a user's decisions
    - this application is a work in progress and therefore the UI and functionality being provided is always being updated and improved upon. Provide a clear timeline of major versions with associated dates and for each version discuss what the major changes were for each service offering.
    - be sure to explicitly state the latest version of the web application software (this can and probably should be done by service offering area)
    - discuss how the **Site Manager** functionality fits into the functional offering

    - identify 3-4 common use cases for each service offering and give examples of how the web application is able to address these use cases.
    - identify 1-4 common use cases which SPAN service offerings and give examples of how the web application is able to address these use cases.

    > **Note: **
    >
    > You can leverage the research already done in:
    > 
    > - [Network App](./network.md)
    > - [Protect App](./protect.md)
    > - [Access App](./access.md)
    > - [Connect App](./connect.md)
    > - [Talk App](./talk.md)
    > 
    > but -- and this is **IMPORTANT** -- do not use this research as a substitute for your
    > own research.

    Finally after having completed the prose for this research document, you will need to add
    the following metadata to this document:

    - `researched_by` as "{{ctx.agent}}/{{ctx.model}}"
    - `last_updated` as "{{ctx.today}}"
    - `latest_app_version` as the lastest app version by service area (e.g., a dictionary where the keys are the service offerings: "network", "protect", "access", "talk", "connect") and the values are the semantic version number
---
