# Preflight Improvements

When we run Claudine's "pre-flight" checks we need to make sure we're respecting the roles that Darkmatter and Claudine are meant to play:

1. Darkmatter is responsible for gathering all shell commands which might be run (not Claudine)
1. Darkmatter is checking not only Markdown's body for `::file` directives but that it's also checking for more recently added Frontmatter shell commands.
1. Claudine will gather the possible shell commands found in pre and post validation checks
1. Claudine will check get the list of shell commands which _might_ be executed during a `compose`, `inline-compose`,or `sequence` operation
    - this _should_ be consistent with the documentation found claudine/docs/topics/pre-flight-checks.md 

If you find any inconsistencies, please document in claudine/features/2026-04-11-preflight-improvements/inconsistencies.md 
