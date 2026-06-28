## An Example of Poor Error Messages

Today we have errors like this:

```sh
 MarkdownError: transform failed
┃
┃ frontmatter key 'iteration': Interpolation evaluation failed for 'frontmatter(spec, 'review_iterations') ? frontmatter(spec,
┃ 'review_iterations') || 1 : 1': frontmatter() invalid file path: "features/2026-06-21-opencode-log-fix/spec.md"
┃
┃ Review the transform pipeline inputs and any configured rules.
```

This error basically comes down to a file reference NOT being valid but it's so dense in it's explanation that a user has to really focus on what's going on to understand what's wrong.

What we need to do in these situations is:

- report the missing file as the primary error condition and then explain the underlying details after that:

```sh
 MarkdownError: invalid file path
┃
┃ The invalid file reference <orange>features/2026-06-21-opencode-log-fix/spec.md</a></orange> was assigned
┃ to the <inverse>iteration</inverse> Frontmatter property when using the
┃ <blue><a href>{prompt}</a></blue> prompt file.
┃
┃ ```yaml
┃ $schema: 
┃     spec: "features/2026-06-21-opencode-log-fix/spec.md"
┃     iteration: "frontmatter(spec, 'review_iterations') ? frontmatter(spec,
┃ 'review_iterations') || 1 : 1': frontmatter()"
┃ ```
┃
┃ Did you mean?
┃ 
┃ - `{suggestion-1}`
┃ - `{suggestion-2}`
```

Let's review some of the fundamental differences which this new error provides:

1. Identifies the REAL problem (e.g., an invalid file reference) and makes that the FOCUS of the error message
2. Shows the variables in the underlying schema definition which are relevant
    - rather than show no YAML -- as we did in the original error -- or show ALL the YAML -- as we do in many other cases -- we instead recognize the variables which are involved in the error and focus on those variables!
    - this shows the variables which are relevant not just a big dump of information
    - Note: we not only show the _lines_ which are relevant but we show the parent `$schema` line too so that user can see the "shape/structure" of that the problematic 
3. The prompt file is not just "mentioned" but a OSC8 hyperlink is provided so that the caller can easily get back to prompt file to understand the file and/or make changes to it
4. A missing file is almost always a typo on the callers part and we should help them identify the file "they meant" where ever possible. To do this we will need some string subset and similarity semantics to bring up a small list of suggestions that feel like the most likely intended files

## Context

There is no point in trying to solve this _specific_ problem! We must identify the pattern which problem represents as well as look for similar patterns which are also providing dense, hard to understand error messages. Once we're able to see the patterns of bad reporting we can start to apply strategic solutions.

Having clear error messages is absolutely essential for not only Claudine but also Darkmatter. These two libraries and CLI's provide a powerful toolset and new users WILL make mistakes so it's super important that we help callers to quickly and painlessly **understand** the mistake they've made so it can be fixed without the user having to scour through documentation or blindly trying different options until something works.

The importance of this task means that finding the "right solution" over the "expediant solution" is an absolute requirement. The scope of this improvement must address these issues in Darkmatter and in Claudine.

> Note: remember we do not have an installed user base for Claudine and Darkmatter yet so we have the freedom to make breaking changes where necessary to achieve our goals. That doesn't mean we should strive for doing things in a breaking manner but if doing so provides notable benefits then this solution should be considered.

## Task
