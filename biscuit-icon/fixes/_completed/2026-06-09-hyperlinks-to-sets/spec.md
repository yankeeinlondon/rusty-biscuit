We now have good reporting when we run `icon sets` but we need to add a quality of life improvement where set name is rendered as `<blue><a href={url}>{set-name}</a></blue>`. The Iconify site has a "route" for every icon name so linking them to the right URL should be easy. The basic pattern is: `https://icon-sets.iconify.design/{prefix}`

As an example:

- the Material Design Icons set has a prefix of "mdi"
- the URL for it's icons is https://icon-sets.iconify.design/mdi/
