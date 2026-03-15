### Update Readme Files

All package areas should have a `README.md` at their base as well as a `README.md` at the base of each of the packages in this package area.

- you can run `sniff docs --readme "$(sniff package-area)"` to get a list of all the README.md files in the package area
- you will act as an orchestrator
- iterate over each README.md and:
    - spawn a subagent to review the README doc and update or add to it so that the changes relating to the recently added "{{feature}}" feature are included and that everything in the README is accurate for the source code that the given readme is responsible for.
- once all README.md's have been updated you are done
