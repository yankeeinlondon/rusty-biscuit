---
sequence: ../../terminal-apps.yaml
---

You are responsible for doing a deep dive into the {{state.intro}} terminal app.

Make sure you describe:

- the broad brand and character attributes that {{state.name}} make {{state.name}} stand out. How would someone describe the app in a sentence or two?
- describe the functionality supported in detail:
    - what OSC standards are supported?
        - create a table that shows the osc standard, short description, and a column indicating support level
    - what mode-based standards does the app support?
        - same table convention used here too
    - what about ECMA-48?
    - how well are unicode characters handles? multi-code characters?
- does the terminal support graphics rendering?
    - kitty graphics? iterm2 graphics? sixtel graphics?
- if {{state.name}} provides native multiplexing describe the feature set that it offers
    - is there programmatic access to the multiplexing or is it just accessed via configuration and key bindings
- what OS's the terminal operates under?
    - are there any notable variance in terms of functionality provided on per-OS level
- what color depths does {{state.name}} support?
- what ENV variables does {{state.name}} set and how can that metadata be leveraged?
    - how do the ENV variables set vary based on the underlying OS?
- Is {{state.name}} open source and if so what license is offered under?
- What are the key URLs for {{state.name}}?
    - documentation
    - repo
    - api (if one is provided)
- Does {{state.name}} have any native AI integrations and if so what are they?
    - can these AI integrations be accessed programmatically?
    - what is the URL for the AI features?
- What do developers say about the application? Are there any quirks or gotchas? How can these be worked around?
- When did {{state.name}} first get released to production?
- What version is latest version?
- What is the best URL to go to get the latest version?
- What package managers distribute this application?
- How frequently do new versions get released? How frequently to "major" versions get released?
- If an application running inside the terminal is interested in detecting/observing what terminal application is being run what are the ways {{state.name}} can be identified?

## Resources

If you need more information on any of the many terminal standards you'll find them under the biscuit-terminal/docs/research/terminal-standards:

### Modes Standards

::file-links biscuit-terminal/docs/research/terminal-standards/modes/*.md

### OSC Standards

::file-links biscuit-terminal/docs/research/terminal-standards/osc/*.md

## Prose Research Document

Write your research as prose to `biscuit-terminal/docs/research/terminals/{{state.file}}`. It should well formed CommonMark Markdown. If you want to create diagrams then use Mermaid code blocks.

## Metadata Capture

After completing the prose search document you will add the following Frontmatter properties on your research doc (`biscuit-terminal/docs/research/terminals/{{state.file}}`):

> **IMPORTANT:** never guess about the metadata values! If your initial research did not uncover some aspect of the metadata we need then take the time to research that detail.

- set `latest_version` to the latest version of the app
- updates (note these are both estimates):
    - set `update_freq` to the frequency that version updates are happening (e.g., 3mo, 5w, 1yr)
    - set `update_freq_major` to the frequency that major versions are happening (e.g., 3mo, 5w, 1yr)
- set the `os` frontmatter to the list of operating systems which {{state.name}} runs under
- osc standards:
    - set `osc-4` to boolean value; true indicates {{state.name}} supports the standard (both 4 and 104)
    - set `osc-5` to boolean value; true indicates {{state.name}} supports the standard (both 5 and 105)
    - set `osc-7` to boolean value; true indicates {{state.name}} supports the standard
    - set `osc-8` to boolean value; true indicates {{state.name}} supports the standard
    - set `osc-10` to boolean value; true indicates {{state.name}} supports the standard
    - set `osc-11` to boolean value; true indicates {{state.name}} supports the standard
    - set `osc-12` to boolean value; true indicates {{state.name}} supports the standard
    - set `osc-13` to boolean value; true indicates {{state.name}} supports the standard
    - set `osc-14` to boolean value; true indicates {{state.name}} supports the standard
    - set `osc-15` to boolean value; true indicates {{state.name}} supports the standard
    - set `osc-16` to boolean value; true indicates {{state.name}} supports the standard
    - set `osc-17` to boolean value; true indicates {{state.name}} supports the standard
    - set `osc-18` to boolean value; true indicates {{state.name}} supports the standard
    - set `osc-19` to boolean value; true indicates {{state.name}} supports the standard
    - set `osc-21` to boolean value; true indicates {{state.name}} supports the standard
    - set `osc-22` to boolean value; true indicates {{state.name}} supports the standard
    - set `osc-50` to boolean value; true indicates {{state.name}} supports the standard
    - set `osc-51` to boolean value; true indicates {{state.name}} supports the standard
    - set `osc-52` to boolean value; true indicates {{state.name}} supports the standard
    - set `osc-66` to boolean value; true indicates {{state.name}} supports the standard
    - set `osc-99` to boolean value; true indicates {{state.name}} supports the standard
    - set `osc-133` to boolean value; true indicates {{state.name}} supports the standard
    - set `osc-633` to boolean value; true indicates {{state.name}} supports the standard
    - set `osc-777` to boolean value; true indicates {{state.name}} supports the standard
    - set `osc-1337` to boolean value; true indicates {{state.name}} supports the standard
    - set `osc-3008` to boolean value; true indicates {{state.name}} supports the standard
    - set `osc-5113` to boolean value; true indicates {{state.name}} supports the standard
    - set `osc-5522` to boolean value; true indicates {{state.name}} supports the standard
