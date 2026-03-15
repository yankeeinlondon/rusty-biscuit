### Implement Review Suggestions (`suggestions`)

Iterate over the review suggestions in the document "{{base_dir}}/review.md", and for each:

- create a subagent and pass the subagent the suggestion to implement and test
- Append to the log file:
    - the log file is located at: `{{base_dir}}/log.md`
    - start your log entry with the heading `## Review Suggestions Implemented`
    - then add a timestamp
    - then list out the files which were mutated during the review implementation
    - then summarize the changes made
    - use `md set "{{base_dir}}/log.md" last_updated "${YYYY}-${MM}-${DD}" --save`
