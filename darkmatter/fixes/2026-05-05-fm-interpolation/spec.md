The @darkmatter/example-docs/shell-expansion/test.md file has the following content:

```md
---
uname: "$(uname)"
pwd: "$(pwd)"
combined: "The current directory is {{pwd}} and the OS is {{uname}}"
---

# Shell Expansion

## Greet

::shell ls -la ../

## From the Heart

Ok not the _heart_, I meant the Frontmatter:

- OS: {{uname}}
- CWD: {{pwd}}

{{combined}}
```

When _composed_ with `md @darkmatter/example-docs/shell-expansion/test.md` we get good results until we get to the interpolation of the 'combined' frontmatter property which is: `The current directory is $(pwd) and the OS is $(uname)`.

This should NOT be the case!

- both the `uname` and `pwd` frontmatter are correctly assigned from the shell expansion
- but the sequencing seems to not allowing us to assign the real values of these properties via interpolation to `combined`
- a user would expect this to work (as would I)
