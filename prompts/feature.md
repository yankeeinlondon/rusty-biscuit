---
$schema:
    - feat: file(required)
      fqn: file
    - feature: file(required)
      fqn: file
agent: pi
date-prefix: "{{ ctx.today + '-' }}"
feat: "{{ feat ? feat : feature }}"
candidates: |-
    {{ 
        map(
            (feat || feature) 
                ? find_files('features/**/*' + basename(feat || feature) + '*/spec.md') 
                : [] 
        , i -> relative(i))
    }}

initialize:
    - when: "!ctx.repo"
      then:
        - error: "This prompt is intended to only be used inside of a repo!"
    - when: "!feat && !feature"
      error: "The feature name was not passed in; use `feat={name}` or `feature={name}`!"
    - set:
        feat: "{{ ensure_leading(kebab_case(basename(feat || feature)), date-prefix) }}"
        fqn: |-
            {{ 
                ensure_leading(ensure_leading(kebab_case(basename(feat || feature)), date-prefix), "/features/")
            }}
    - when: "!feat"
      then:
          - error: "The feature name was not passed in; use `feat={name}` or `feature={name}`"
    - when: "file_exists(fqn)"
      then: 
        - info: "the <i>feature</i> {{link(fqn, parent_dir(fqn))}} exists."
        - edit: "{{ fqn }}"
      else:
        - when: "length(candidates) > 0"
          then:
            - info: "We didn't find a feature specification in <b>{{feat}}</b> but there are similarly named features:\n\n    {{ as_unordered_list(candidates) }}"
          else:
            - warn: "The <b>{{link(fqn, feat)}}</b> feature does not exist and we didn't find any other features that seemed to match the <dim>**/*</dim>{{without_date(feat)}}<dim>*</dim> file pattern. We will assume the goal is to create this spec now."
            - edit: "{{ ctx.repo_root + '/' + ensure_trailing(fqn, '/spec.md') }}"
        - skip
blocked: 
    - message: "- failed to create/edit **{{feat || feature }}**"
---
This prompt uses the lifecycle hooks to do it's work
