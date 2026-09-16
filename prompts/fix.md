---
description: |-
    A prompt for _routing_ **fixes** on various types of files:

    - `prompt` 
        - if passed a valid prompt file then it will be evaluated for any schema errors and fixed
        - if you want more wholistic advice on how to make a prompt better and not just strictly "make this prompt pass all schematic and structural issues"
    - `issue`
        Pass in an issue description and it will take the following steps:

        1. Diagnose

            The `issue` text passed in is evaluated to determine if it detailed enough that the amount of work is known and a point of view can be established about how best to fix this (direct fix, write a spec, etc.).

            If more detail is needed then
        
        2. Take Action
        
$schema: 
    prompt: file(match(**/*.md)) -> caller can pass in a "prompt" to have an existing prompt file fixed
    issue: string -> description of the issue the caller wants diagnosed
    diagnosis: string -> a complete diagnosis of an issue, including a suggestion on how to fix the issue.
    suggested_action: enum(fix,spec)
    action: enum(fix,spec,auto,prefer-fix,prefer-spec,ask)
action: ask
# this is KNOWN to not be the correct syntax and a work in progress
# Note: the syntax is more aligned to a future syntax we may move to
initialize:
    - when: "issue"
      action:
      - info: "passed in _issue_ is being evaluated"
      # this is a new (not implemented) flow control directive
      # it's utility is to call another prompt to do some work
      # and then return to this prompt to finish.
      # 
      # -All Frontmatter mutations caused by the external prompt will
      # be brought into the `start` lifecycle event of this prompt
      - prep: "prompts/_fix/ensure-issue-is"
start:
    - when: "issue"
      action:
          - when: "diagnosis && suggested_action"
            action:
                - message: |-
                    We have a diagnosis for the issue passed in and the recommended fix approach is {{ diagnosis == 'fix' ? 'fix directly (no need to create a spec)' : 'create a _fix_ specification for this problem' }}:

                    _{{ trim(diagnosis) }}_
                - when: "diagnosis == 'ask'"
                  action:
                      - interview:
                          - title: "Take Action"
                            showcase: diagnosis
                            description: "Based on the diagnosis presented, what action would you like to take?"
                            choose_one: 
                                - choice: fix
                                - choice: spec
                                - choice: stop
                                  action:
                                      - stderr: |-

                                        Ok, no action taken.
                                      - stop
          - else:
              - error: |-
                    diagnosis
                        ? "the issue was sent for diagnosis but nothing came back!"
                        : "the issue was sent for diagnosis but no suggested action was returned"
            
---
