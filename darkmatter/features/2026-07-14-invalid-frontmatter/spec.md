# Invalid Frontmatter

Darkmatter has a "clean" feature that helps to cleanup "semi-standard" Markdown to be more standards based. What I've noticed is that it doesn't currently validate the YAML frontmatter and there have been more than one situation where an Agent produced an invalid entry in the YAML which just passed through the clean check unchallenged.

The most common error -- by far -- is that a property is assigned what is intended to be a _string_ value but the string is NOT quoted because YAML allows non-quoted strings but when certain characters are present (I think the starting character is the main determinant) the string must be quoted to be considered valid (both single and double quote characters are fine so long as they are consistent).
