---
# 2026-10-25-loops-and-flow-control: the review loop. `spec` and `agent`
# are supplied here because the example's own `$schema` requires them.
$schema:
    spec: file(required; eager; match(**/*spec*.md)) -> the specification under review
    agent: string(required) -> the agent for every step
    max: number -> review passes before exhaustion
spec: ./review-spec.md
agent: claude
max: 5
loop:
    while: "true"            # ends only through `break` or the cap
    max: "{{ max }}"
sequence:
    - name: implement
      prompt: "@prompts/_implement/implement-plan.md"
      when: "frontmatter(spec, 'implemented') != true"
      with: { spec: "{{ spec }}" }
    - name: review
      group:
          name: review
          tasks:
              - name: review
                prompt: "@prompts/_reviews/feature-review.md"
                with: { spec: "{{ spec }}" }
          start:
              - set:
                    review_before: "{{ frontmatter(spec, 'review_iterations') }}"
              - when: "!is_null(review_before) && (!is_integer(review_before) || review_before < 0)"
                then:
                    - error: "review_iterations must be a nonnegative integer"
              - set:
                    expected_iteration: "{{ (review_before || 0) + 1 }}"
              - set:
                    latest_review: "{{ dirname(absolute(spec)) + '/review-' + expected_iteration + '.md' }}"
              - when: "file_exists(latest_review)"
                then:
                    - error: "expected a new review, but {{ latest_review }} already exists"
          success:
              - when: "!is_integer(frontmatter(spec, 'review_iterations')) || frontmatter(spec, 'review_iterations') != expected_iteration"
                then:
                    - error: "review did not advance review_iterations by exactly one"
              - when: "!file_exists(latest_review)"
                then:
                    - error: "the review did not write {{ latest_review }}"
              - when: "as_json([frontmatter(latest_review, 'recurrence')]) != '[true]' && as_json([frontmatter(latest_review, 'recurrence')]) != '[false]'"
                then:
                    - error: "{{ latest_review }} has no boolean `recurrence`"
              - when: "as_json([frontmatter(latest_review, 'ready')]) != '[true]' && as_json([frontmatter(latest_review, 'ready')]) != '[false]'"
                then:
                    - error: "{{ latest_review }} has no boolean `ready`"
              - when: "frontmatter(latest_review, 'recurrence') == true"
                then:
                    - warn: "a finding class recurred; stopping for a human"
                    - break: { reason: "recurring finding class", code: 2 }
              - when: "frontmatter(latest_review, 'ready') == true"
                then:
                    - message: "review marked the work ready"
                    - break: "ready"
    - name: repair
      prompt: "@prompts/_implement/implement-suggestions.md"
      with: { spec: "{{ spec }}" }
    - name: stage
      shell: git add .
    - name: commit
      prompt: "@prompts/commit.md"
      with: { agent: "{{ agent }}", message: "repairs from {{ basename(latest_review) }}" }
---
Body.
