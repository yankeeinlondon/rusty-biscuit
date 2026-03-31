# Frontmatter and Recursion

During transclusion:

- the frontmatter of the parent document is passed to the child documents
- if the child document has the properties defined then
    - if the property is a scalar value or a list value then the child document's value is retained _over_ the parents
    - if the property is a dictionary value then we merge the object's keys, giving the child's values priority over the parent's in the case where the parent


## Exceptions

- `ctx` is a property which is provided to each document by Darkmatter; it is recommended that document authors **not** use this property themselves but it is supported. When a document _does_ have a `ctx` property then we will use the normal recursion process described above.
