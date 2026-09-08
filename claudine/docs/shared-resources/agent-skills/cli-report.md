```sh
# reports all available skills grouped by scope (user/repo)
claudine skills list
# same as `claudine skills list`
claudine skills
# reports skills which match the string passed in
claudine skills <filter-string>
```

The amount of reporting detail you will be returned depends on how many skills there are in scope:

- if there are a lot of skills then you'll get the skill names grouped by their scope
- if there are more than one but a relatively small amount then you'll get the skill's name along with it's description
- if there is only one then you'll get all details reported including a graph of all the files included in the skill and how many (approximate) tokens each file contains.

There are times, however, where you want information on a single skill and in those cases you can run:

```sh
claudine skills find <name>
```

It will look for an explicit match on `name` and if found will provide a full report on that skill. If it doesn't find an exact match it will return an error but include any similar skill names as a part of the "did you mean" section of the error.

## Exceptions

In addition to the skills reporting, any invalid skills or skills which are not fully synchronized will be reported.

> **Note:** only problems involving the displayed skill will be shown; so if there were an invalid skill called "foo" but you ran `claudine skills bar` then you would not see the invalid skill being warned about because that skill is _not in scope_

## CLI Switches

- `--fix` on both the list and find operations will attempt to fix any exceptions which have been listed
