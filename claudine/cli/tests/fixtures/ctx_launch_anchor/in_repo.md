---
title: inverse ctx launch anchor baseline
frontmatter_area: "{{ ctx.area }}"
frontmatter_repo: "{{ ctx.repo_root }}"
success:
  warn: "baseline.lifecycle.area=[{{ ctx.area }}] baseline.lifecycle.repo=[{{ ctx.repo_root }}]"
  stack:
    - when: "!ctx.area && !ctx.repo_root"
      action: {warn: "baseline.when.launch-context=true area=[{{ ctx.area }}] baseline.when.repo=[{{ ctx.repo_root }}]"}
---
baseline.body.area=[{{ ctx.area }}] baseline.body.repo=[{{ ctx.repo_root }}]
baseline.frontmatter.area=[{{ frontmatter_area }}] baseline.frontmatter.repo=[{{ frontmatter_repo }}]
