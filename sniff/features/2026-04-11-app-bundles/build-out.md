---
dir: sniff/features/2026-app-bundles
log: {{dir}}/log.md
windows_plan: {{dir}}/windows-plan.md
windows_design: {{dir}}/windows-design.md
linux_plan: {{dir}}/linux-plan.md
linux_design: {{dir}}/linux-design.md

sequence:
    - implement_windows
    - commit
    - review_windows
    - suggestions_windows
    - commit
    - review_2_windows
    - suggestions_2_windows
    - implement_linux
    - review_linux
    - suggestions_linux
    - review_2_linux
    - suggestions_2_linux
---

::block when="state=='implement_windows'"

- implement the plan in {{windows_plan}}
- use the 'sniff' skill
  ::end-block

::block when="state=='review_windows'"
::file @prompts/review-feature.md dir="{{dir}}" design="window-design.md" iteration=1
::end-block

::block when="state=='suggestions_windows'"
::file @prompts/implement-feature-review-suggestions.md dir="{{dir}}" design="window-design.md" review="windows-review" iteration=1
::end-block

::block when="state=='review_2_windows'"
::file @prompts/review-feature.md dir="{{dir}}" design="window-design.md" iteration=2
::end-block

::block when="state=='suggestions_2_windows'"
::file @prompts/implement-feature-review-suggestions.md dir="{{dir}}" design="window-design.md" review="windows-review" iteration=2
::end-block

::block when="state=='implement_linux'"

- implement the plan in {{linux_plan}}
- use the 'sniff' skill
  ::end-block

::block when="state=='review_linux'"
::file @prompts/review-feature.md dir="{{dir}}" design="linux-design.md" iteration=1
::end-block

::block when="state=='suggestions_linux'"
::file @prompts/implement-feature-review-suggestions.md dir="{{dir}}" design="linux-design.md" review="linux-review" iteration=1
::end-block

::block when="state=='review_2_linux'"
::file @prompts/review-feature.md dir="{{dir}}" design="linux-design.md" iteration=2
::end-block

::block when="state=='suggestions_2_linux'"
::file @prompts/implement-feature-review-suggestions.md dir="{{dir}}" design="linux-design.md" review="linux-review" iteration=2
::end-block

::block when="state=='commit'"
::shell "just commit"
::end-block
