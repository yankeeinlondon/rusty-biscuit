---
sequence: ../../terminal-apps.yaml
---

You are responsible for doing a deep dive into the {{state.intro}} terminal app.

Make sure you describe:

- the broad brand and character attributes that {{state.name}} make {{state.name}} stand out. How would someone describe the app in a sentence or two?
- describe the functionality supported in detail:
    - what OSC standards are supported?
    - what mode-based standards does the app support?
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

- set `latest_version` to the latest version of the app
- set `update_freq` to
