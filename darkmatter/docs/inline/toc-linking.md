# Table of Contents Linking

Darkmatter's DSL provides a compact syntax for a page to link into the Table of Contents of another Markdown page.

The basic syntax is:

> `::toc-linking <filename> [params]`

This instruction will provide links to the H2-H6 headings in the referenced file.

## Parameters

### Link Levels

While sometimes you want to have the full range of H2 to H6 level headings, sometimes it's better just to hit H2 or H3 level settings. Whichever levels you want to provide links to though we have a switch for that:

- `level=h2` - link to H2 headings
- `level=h3` - link to H3 headings
- `level=h2,h3` - link to H2 and H3 headings
- etc.

> **Note:** you can combine levels by adding `level=h{#}` more than once but you can also just use a comma to separate heading levels and define all your headings in a single key/value.

### Cleanup

The cleanup operation allows us to remove unnecessary cruft in the headings text.

- `cleanup=true` - provide _all_ cleanup services (e.g., `cleanup=emoji,number,capitalize`)
- `cleanup=false` (**default**) - no cleanup services
- `cleanup=emoji_leader` - strips out all emoji at the start of the heading text
- `cleanup=emoji_trailing` - strips out all emoji at the end of the heading text
- `cleanup=emoji` - strips out all emoji, regardless of location
- `cleanup=number` - strips out a leading numeric index (e.g., `1. some text` -> `some text`)
- `cleanup=capitalize` - ensures that the leading character, if alphanumeric, is capitalized

It is very common for AI generated content to add unwanted emoji or numeric indexes as part of their headings. By turning on one or more cleaning services we can be assured that the link text in the base document is formatted in the most consistent and clean manner.

> **Note:** you can combine cleanup services by adding `cleanup={service}` more than once but you can also just use a comma to separate services in a single definition.

### Filtering

- `filter={glob}`
- `keep={glob}`

The `filter` parameter allows you to blacklist certain headings using a glob pattern. By contrast, the `keep` parameter -- when used -- immediately filters out ALL headings and allows the glob pattern to _whitelist_ the headings which you want to retain.

#### Notes

- Casing
    - by default casing does not matter for matching glob patterns
    - if you want to be EXPLICIT though you can be by adding a `^` character to the beginning of the glob pattern.
- Combining Filter and Keep
    - you _can_ combine the `filter` and `keep` parameters
    - if you do then the `keep` parameters will be applied first and then the `filter` glob patterns
- Multiple of the Same
    - unlike many of the other parameters in this operation we can **not** combine glob patterns by comma delimiting them
    - the only valid way to have multiple `filter` or `keep` glob patterns is to express multiple key=value pairs.
    - when you do have multiple globs for the same operation they will be logically OR'd together
- Filter before Clean
    - the `filter` and `keep` parameters are matching on the heading text of the reference file, not the "cleaned" version which may be used as the link text to this heading.

### Handling No Results

It is **not** an error when there are no headings in the target markdown file (or the `filter`/`keep` policy has led to no headings remaining). By default this will result in the operation directive being removed and no text added. However, you may use the `empty` parameter to specify some other replacement value.

- example: `::toc-linking ./something.md empty="no results"`

## Alternate File References

There may be some cases where you _usually_ have a file named **A** but if it's not there then **B** can serve as a backup for **A**.

- example: `::toc-linking "./something.md | ./something-else.md"`

This operation allows you to provide an alternative file reference with the `|` operator.

- you can use the `|` operator as many times as you need to
- you can even terminate the list with `| false` to indicate that it's possible that _none_ of the file references will resolve and if that happens it should not be treated as an error.


## Errors

There are several ways the `::toc-linking` operation can result in an error. All errors should use the `thiserror` crate and have well thought out messages which help the caller understand what has happened.

Here's a non-exhaustive list of the kinds of errors we could expect from this operation:

- Invalid Filename
    - if the file referenced doesn't exist in the filesystem
    - if invalid characters are used for the filename then this is another variant
- Invalid Cleanup Service
    - if the `service={service}` parameter is found with a service that is invalid then this is an error
    - similarly if a list of services is provided and any one of these services is invalid this is an error

## Examples

### Basics

```md
::toc-linking ./something.md filter=3.* level=h2
```

In this example we will link to all the H2 headings found in the Markdown document `./something.md`, _except_ a heading who starts with `3.`. Alternatively:

```md
::toc-linking ./something.md keep="3.*" keep=4.* keep="*important*"
```

Here we're using a whitelisting approach and allowing H2 headings which start with either `3.` or `4.` or include `important` anywhere within the heading text.

#### Notes

- Quoting Values
    - you may have noticed that for some parameters we explicitly quoted the value whereas with others we did not.
    - similar to how YAML configurations behave, we don't require quoting the value but in a few edge cases where you want to be explicit about whitespace


### Advanced Use Case

Let's come up with one more example where put all the parameters to work:

```md
::toc-linking "./foo.md | ./bar.md | ./baz.md | false" filter=3.* level=h2 empty="nothing found"
```

#### Notes

- we will source the links from the file `foo.md` if present, otherwise falling back to `bar.md`, and then `baz.md`
- even if all three files do not exist we will not produce an error, instead we will return the `empty` parameter's "nothing found"
- if we are able to source a file (foo/bar/baz) then we will look at the H2 headings and filter out any that start with `3.`

--- 

[< back to **Pipeline Documentation**](../darkmatter-compose-pipeline.md)
