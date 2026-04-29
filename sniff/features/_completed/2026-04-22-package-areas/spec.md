The sniff CLI already provides a `sniff repo packages` command which lists all of the packages in the repo. In this feature we'll add a command `sniff repo package-areas` which will provide exactly the same CLI switches as `sniff repo packages`. This includes:

--debug -- Emit raw developer tracing to stderr; repeat for higher verbosity
--json -- Output as JSON instead of text (with subcommand) or force JSON (no subcommand)
--list -- Render as a raw list (one name per line, no bullet)
--md -- Render as a Markdown unordered list (one `- name` per line)
--package-area -- Restrict output to packages in the specified package area
--perf -- Include structured performance timings and counters in the output
--plain -- Strip terminal escape codes from text output
--verbose -- Increase output verbosity (styled user output only; never raw tracing)

When the `-v`/`--verbose` flag is used then the package areas will be listed with the root directory for the package area in parenthesis: `{package-area} (<dim><i>{dir}</i></dim>)`.
