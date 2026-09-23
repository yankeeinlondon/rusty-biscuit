---
status: future
kind: canonical
type: compose
subtype: inline
---
# The `doc-list` Directive

The `doc-list` directive allows you add a real-time query to identify documents in your repo (or file tree if not in a repo).

## Syntax

> `::doc-list <filters> <organizational> <reporting-style>`

### Filters

You rarely want _all_ documents listed so typically you would start by narrowing down the documents you are looking for. The filter parameters are:

- `kind`, `type`, and `subtype` target the properties they're named after
    - if a kind filter were added of `kind=schema` then it would filter down to only documents who's `kind` Frontmatter property is `schema`
    - you can use prefix or postfix wildcards such as `type="project*"` which will match on documents who's `type` property starts with "project"
- `defines` let's you filter by a property or set of properties which are _defined_ in the Frontmatter
    - if you used `defines=blast-radius` then Darkmatter would filter down to only documents that define the `blast-radius` property
    - if you want to add more than one property you can use a CSV format:
        - `defines=blast-radius,symbols` will filter down to document that define `blast-radius` OR `symbols`
- `where` allows you to express your query in a JSON/JSON5 representation
    - if you wanted to list documents that define a `blast-radius` property as a string array but do NOT define a `title` property then you could:
        - `where="{ blast-radius: string[], title: null }"`
        - uses the key's to represent the properties you are interested in, and
        - the values are a type borrowed from [`SimplifiedSchema`](^darkmatter/docs/topics/schemas/index.md)

        > **Note:** if you wanted to filter on a properties existence but didn't care about the _type_ of the property you can do that with the `any`/`unknown` type

### Organizational


- **Sorting**
    - `sort={method},{asc,desc}`
        - size
        - last_updated
        - created
        - filename
        - title
    - `sort_by_prop={prop},{asc|desc}` let's you sort by a specific Frontmatter property

- **Grouping**
    - `group-by=`


- **Enumerating** a Property
    - when we decide to _enumerate_ a property we are deciding to focus more an a property tag of the document set than the documents themselves
    - `enumerate=kind` instructs this directive to produce a unique list of _values_ found in the `kind` documents in the repo (file tree if _not_ in repo).
    - You can also enumerate to two or three levels:
        - `enumerate=kind->type->subtype`
        - This will produce a hierarchical tree of results; the JSON schema for a node is:

            ```yaml
            property: string(required) -> the property being enumerated
            value: any -> an enumerated value found in the property
            children: self[]
            ```

            > schema defined using [SimplifiedSchema](^darkmatter/docs/topics/schemas/index.md)
    - sorting and grouping are not supported when using enumeration
        - if they are included alongside enumeration then we'll log a warning to STDERR
    - how this enumeration is _rendered_ (when not asking for JSON) will be covered in the next section


## Reporting

Like other CLI endpoints in **Sniff** we support the `--json` switch to have the output be in JSON format. When this is used, this command _only_ reports valid JSON; nothing more.




## Related Libraries

- most of this functionality is provided directly from the [`sniff`](@sniff/README.md) library in this monorepo
