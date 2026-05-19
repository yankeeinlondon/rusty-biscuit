---
sequence: @docs/providers.yaml
doc: ""
---

Being able to understand how much "usage" a subscription plan in it for the short term window (typically 5 hours) or the longer term (typically the week) is **VERY** valuable information. 

Your task is to research how a program can get this usage data for the current user from {{state.desc}}. 

- you will save all your findings to {{doc}}

::block when="state.model_provider"

- because {{state.desc}} is developed by a model-provider then you likely have a `/status` or `/usage` slash command
    - this slash command tends to ONLY work in a interactive session
    - that often includes passing in the `/status` or `/usage` as the preliminary command`
    - this makes getting at the data much more complicated then if we can run this asa non-interactive prompt

::end-block
::block when="!state.model_provider"

::end-block
