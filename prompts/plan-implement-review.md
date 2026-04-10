---
sequence:
    - name: plan
      prompt: "plan.md" 
    - name: implement
      prompt: "implement.md"
    - name: git-implement
      shell: "git add {{env.current_package_area}}/. && just commit"
    - name: review
      prompt: "review.md"
    - name: implement-review
      prompt: implement-review.md
    - name: git-review
      shell: "git add {{env.current_package_area}}/. && just commit"
---
