# Block Columns

## Goal

There is no native implementation of columns in Markdown so in DarkMatter we want to add this with the `::columns` and `::break` directives:

- the number of columns must be expressed but the number can be responsive to the viewport and the [breakpoints](../reference/breakpoints.md) which have been set.
- by default setting will give all columns equal width but a user can override this and set column widths explicitly
- certain styling options can be assigned to all or some of the columns as well with the following switches:
    - `--gap <size>` - allows the default column gaps of `0.5rem` be overridden
    - `--bg <color>` - allow the background color to be set
    - `--text-align <alignment>` - change the alignment of text
    - `--text-color <color>` - change the text color
    - `--border <size>` - add a border around the column of a specified width
    - `--border-color <color>` - specifies the border color to use (only has effect when used in conjunction with `border`)
    - `--rounded <xs|sm|md|lg|xl|xxl>` - can set any visible border to a specified rounding amount (only has effect when used in conjunction with `border`)

## Syntax

The general syntax to start a columned layout is:

~~~md
::columns <quantity> [flags]
~~~

- The `quantity` (aka, the number of columns) MUST be specified but it can be done in one of two ways:
    - **Fixed:** you can assign the number of columns _regardless_ of the viewer's viewport size. This is done with something like `::columns 3`.
    - **Responsive:**
        - you can assign one or more [breakpoints](../reference/breakpoints.md) where you want to change the number of columns
        - The _starting_ or _default_ number of columns is **1** and the responsive settings move from smallest to largest.
        - if you define columns with `::columns md: 2, xl: 3` then this will configure the following responsive settings:
            - `{ xs: 1, sm: 1, md: 2, lg: 2, xl: 3, xxl: 3 }`

    > **Note:** the examples so far have used static numeric values but you _can_ use frontmatter interpolation as well but if you do you must use the `||` operator to set a default value in case the variable is not set when rendered
    >
    > example: `::columns {{columns||2}}`

- Following the `quantity` parameter you can include any number of switches from the list above to modify the appearance to your liking:
    - `::columns 2 --gap 1rem --border 2px --rounded xl` would:
        - override the column gaps to `1rem`
        - add a border of 2px around each column
- All of the configurable switches other than "--gap" may also be defined on a per-column basis:
    - `::columns 3 --border [0,2px]`
        - in this example we have expressed there should be:
            - three columns
            - the first column should have a border of `0` (aka, no border)
            - the second column should have a border of `2px`
            - the third column is _not_ explicitly defined but when the number of columns exceeds the definition then every column afterward will use the last setting. In this case that means that the third column will have a border of `2px`

### Using `::break` to mark column splits

To indicate that you are moving from one column to the next you use the `::break` directive.

It is considered best practice to have a blank line above and below this `::break` but the only strict requirement is that the `::break` starts at column 0.

Because we may have a variable number columns based on responsive layout we can opt instead for using _responsive_ break points:

- `::break md` -- will create a column break for medium viewport only
- `::break md lg` -- will create a column break for medium and large viewports
- `::break md+" -- will create a column break for medium _and larger_ viewports

> **Note:** responsive breaks are not really necessary when moving between 1 or 2 columns because if the viewport dictates only 1 column then all `::break` directives will be ignored.
>
> **Note:** the idea of "ignored breaks" applies more broadly ... under **any** situation where the column count has been reached all subsequent `::break` directives will be ignored.

### The `--balance` and `--height` switches

Up to now we've been reliant on being explicit about column breaks by strategically dropping `::break` directives where we want to the column breaks to occur. This is often exactly what we want to do. However, another common need is to have the columns breaks be "auto-magically" added for us. This is where the "balance" and "height" switches come into play.


#### Using the `--balance` switch

The `--balance` switch indicates that we want the CSS subsystem to _balance_ the height of the columns as much as possible.

> **Hint:** consider using `column-fill: balance` in CSS

Usage would look like:

-
