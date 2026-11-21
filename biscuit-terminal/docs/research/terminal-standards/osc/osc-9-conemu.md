---
$schema:
    description: string(required)
    apps_supporting: string[](required)
    apps: "{ name: string(required), urls: url(required)[], support: enum(true,false,unknown; required) }[]"
apps: "{{ load_data('biscuit-terminal/docs/terminal-apps.yaml') }}"
prompt: |-
    ConEmu defines a family of `OSC 9 ; <n> ; ...` sub-commands (9;1 through 9;12, e.g., 9;9 to report the current working directory, which Windows Terminal also uses). The 9;4 progress sub-command is researched separately in `osc-9-4-protocol.md`, so cover every other sub-command here.

    ::block when="seq.state == 'research'"
    Your task is to research this informal standard and:

    - provide a detailed overview of the standard.
        - How it works.
        - Whether it can be tested for.
        - What does each sub-command do, and which of them have been adopted outside ConEmu?
        - What visual representation, if any, does each adopted sub-command provide? How does that vary by terminal app?
        - How do these sub-commands conflict with iTerm2's `OSC 9 ; <text>` notification, and how can a program avoid that?
        - Where and when it was introduced? How well it is supported? Is there continuing adoption?

    Once you've completed your research and have written the prose content to the body of this document, you will need to update the Frontmatter properties:

    - set `description` as a one to two sentence description of this standard
    ::end-block
    ::block when="seq.state == 'apps'"
    The biscuit-terminal package in this repo cares about the following terminal apps:

    ::map apps "i => t.template"
    - {{i.name}} ({{i.url}})
    ::end-map

    Your task is to act as an orchestrator and have a subagent per supported app started where the subagent's job is to do research and answer definitively:

    1. DOES the app support this functionality? Valid answers are `true`,`false`,`unknown`
    2. What URL(s) prove that this support exists
    3. What is the latest version of the app as of this research being done?

    Once all of the subagents have reported back to you on this you need to look at the existing prose content on this page to see if there is a section `## App Support`:

    - if this section EXISTS, then replace the content of this section (keeping the rest of the content untouched) with your findings
    - if this section DOES NOT EXIST, then add the section and your findings under the H2 heading (again making sure to keep the rest of the content untouched in other sections)

    
    
    - set `apps_supporting` as a list of those apps which are known to support ConEmu's OSC 9 sub-commands
    ::end-block
sequence: 
    - research
    - apps
---
