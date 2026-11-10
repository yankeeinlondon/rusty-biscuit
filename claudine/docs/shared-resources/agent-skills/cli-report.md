```sh
# reports all available skills grouped by scope (user/repo)
claudine skills
# reports skills whose names contain the string passed in
claudine skills <filter-string>
```

The amount of reporting detail you will be returned depends on how many skills there are in scope:

- if there are a lot of skills then you'll get the skill names grouped by their scope
- if there are more than one but a relatively small amount then you'll get the skill's name along with it's description
- if there is only one then you'll get all details reported including a graph of all the files included in the skill and how many (approximate) tokens each file contains.

There are times, however, where you want information on a single skill and in those cases you can run:

```sh
claudine skills <name>!
```

A trailing `!` requires an exact match on `name`, so the full report for that one skill is shown. There are no `list` or `find` subcommands: any word after `claudine skills` is a filter, and `-name` or `!name` excludes matching skills. When nothing matches, Claudine prints `No skills matching: …`.

## Exceptions

In addition to the skills reporting, any invalid skills or skills which are not fully synchronized will be reported.

> **Note:** only problems involving the displayed skill will be shown; so if there were an invalid skill called "foo" but you ran `claudine skills bar` then you would not see the invalid skill being warned about because that skill is _not in scope_

## CLI Switches

- `--fix` (alias of `--apply`) attempts to fix exceptions before reporting; it fixes every skill, since a filter narrows only the report
