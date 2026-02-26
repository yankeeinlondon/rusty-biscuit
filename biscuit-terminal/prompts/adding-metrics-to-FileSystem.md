# Adding Metrics to the `FileSystem` struct

The current `FileSystem` struct is a nice way to present the file structure to the terminal but currently there's no way to report any attributes or metrics about the various files. In this feature we are going to change that.

The metrics we want to be able to report on are:

- Size
    - `file_size`
    - `tokens`
- DateTime
    - `created`
    - `created_since`
    - `modified`
    - `modified_since`
- Permissions
    - `permissions`
    - `permissions_numeric`
    - `owner`
    - `group`

In the library we'll create a number of builder functions to indicate that we would like reporting on a given metric:

- `show_XXX()` - where XXX is the attribute above (e.g., `file_size`, `tokens`, `created`, etc.)
- `show_XXX_with_filename<T: Into<String>>(glob: Vec<T>)` - XXX is again the attribute name but in this case we will only show the specified attribute for filenames which match the glob patterns passed in (globs will allow for negation globs too like `!not-this*` )
- `show_XXX_highlight_greater_than(num: u32)`

## Render Format

The basic format which all metrics will fit into is:

- for a singular metric: `( <dim>{metric}:</dim> {value} )`
- for multiple metrics: `( <dim>{metric}:</dim> {value}, <dim>{metric2}:</dim> {value2} )`

### Variants

A few of the metrics types require a little extra instruction on how they're rendered:

- the `file_size` metric should be presented as `file size`
- the `created_since` and `modified_since` should just use metric name _without_ the `_since`
    - these two metrics will show the created date and modified date but instead of a date literal it will add a descriptive delta like `2 days ago`, `1 month ago`, `1 year ago`, etc.
- the `permissions_numeric` will present permissions as a number (e.g., 740, 600, etc.) and the metric should just be `perm`
- the `permissions` metric will show a string like `.rw-r--r--` to represent the permissions and use a metric name of just `perm`
    - the `r` permission should be green
    - the `w` permission should be red
    - the `x` permission should be orange

### Tokens

The `tokens` metric is a way to estimate the number of LLM _tokens_ this file would be. To calculate this we will calculate the token count based on content type:

1. Log files, JSON, YAML, TOML: 2.5 chars/token
2. Other text documents (incl. Markdown, etc.): 4 chars/token
3. non-text documents (no reporting)
