# Symbol Metadata Model for Static Analysis

What you want is effectively a **loss-minimizing symbol model**.

There are really **three distinct layers** of metadata you usually want to capture:

1. **Identity metadata**  
   What symbol is this?

2. **Structural and type metadata**  
   What is its shape and contract?

3. **Source, documentation, and semantic metadata**  
   How was it declared, documented, constrained, and intended to be used?

If your goal is to describe a symbol so completely that another tool could:

- render good docs,
- diff APIs,
- do semantic search,
- build dependency graphs,
- reconstruct type signatures,
- reason about compatibility,

then you want a schema that is broader than just `name + type`.

---

## 1. Common metadata every symbol should probably have

This is the base layer I would put on **all** symbols, regardless of whether they are a function, class, method, property, enum, interface, type alias, variable, module, or parameter.

### Identity

- `id`  
  A stable internal identifier for your analyzer.

- `name`  
  The local or simple name.

- `qualified_name`  
  A fully qualified symbol name.

  Examples:
    - `foo`
    - `pkg::module::foo`
    - `MyNamespace.MyClass.method`

- `display_name`  
  The preferred human-readable display name.

- `symbol_kind`  
  Examples:
    - function
    - class
    - method
    - property
    - enum
    - interface
    - type-alias
    - variable
    - module
    - namespace
    - parameter
    - field

- `language`  
  Examples:
    - Rust
    - TypeScript
    - Python
    - Go

### Source location

- `file_path`
- `module_path`
- `span`
    - start byte
    - end byte
- `start_line`
- `start_column`
- `end_line`
- `end_column`
- `body_span`
    - especially useful for functions and classes
- `declaration_span`
    - just the declaration/header
- `doc_span`
    - where associated docs/comments came from

### Visibility, linkage, and reachability

- `visibility`
    - public
    - private
    - protected
    - internal
    - crate
    - package
    - module-private

- `is_exported`
- `is_reexported`
- `is_default_export`
- `is_imported`
- `is_declaration_only`
- `is_ambient`
- `is_generated`
- `is_synthetic`
- `is_external`
    - declared outside the project
- `is_deprecated`
- `is_experimental`
- `stability`
    - stable
    - unstable
    - internal
    - beta

### Raw source capture

This is important if you want **full fidelity**.

- `source_text`  
  Exact text of the declaration.

- `signature_text`  
  Normalized header/signature only.

- `body_text`  
  Optional; often omitted for scale.

- `normalized_signature`  
  Canonicalized form useful for comparison.

- `attributes_raw`
    - raw annotations
    - attributes
    - decorators
    - modifiers

- `comments_raw`  
  Exact attached comment blocks.

### Documentation

Do not collapse docs into one string only. Keep structure.

- `doc_summary`
- `doc_description`
- `doc_tags`
    - `@param`
    - `@returns`
    - `@throws`
    - `@deprecated`
    - `@example`
    - `@see`
    - `@default`
    - `@remarks`

- `doc_examples`
- `doc_warnings`
- `doc_notes`
- `doc_authors`
- `doc_since`
- `doc_deprecated_message`

### Relationships

- `parent_symbol`
- `container_symbol`
- `members`
- `overrides`
- `implements`
- `extends`
- `references`
- `dependencies`
- `used_by`
- `related_symbols`

### Type abstraction

Every symbol should expose type information in multiple forms if possible:

- `type_repr`
    - human-readable type
- `type_canonical`
    - normalized or canonical type expression
- `type_ast`
    - structured representation
- `type_symbols`
    - referenced type symbols
- `generic_parameters`
- `constraints`
- `default_type_arguments`

---

## 2. A useful mental model: split symbol capture into five views

For each symbol, capture these five perspectives.

### A. Declaration view

What syntax declared it?

- modifiers
- keywords
- attributes
- decorators
- async, const, static, etc.
- type parameters
- parameter list
- return type
- where clause / bounds / constraints

### B. Type-contract view

What contract does it expose?

- callable
- constructible
- iterable
- fields/properties
- mutability
- lifetime and ownership semantics
- nullability
- error/throw behavior
- effect-system metadata if the language has it

### C. Documentation view

What did the author say?

- summary
- detailed text
- parameter docs
- examples
- deprecation notes
- remarks
- warnings

### D. Semantic view

What does analysis infer?

- exported
- reachable
- pure / impure
- async
- deterministic
- throws
- returns-never
- mutates-self
- mutates-args
- reads-global-state
- allocates
- recursion
- side effects

### E. Source-fidelity view

Can you reconstruct the declaration?

- exact text
- comment attachment
- whitespace-insensitive normalized text
- original token spans

This separation helps a lot because many analyzers initially mix these together and the schema becomes hard to extend.

---

## 3. Metadata typically needed for functions

Functions are the most metadata-dense symbols.

### Core function metadata

- `name`
- `qualified_name`
- `kind`
    - function
    - method
    - constructor
    - getter
    - setter
    - operator overload
    - closure/lambda
    - callback signature

- `visibility`
- `is_exported`
- `is_async`
- `is_generator`
- `is_const`
- `is_static`
- `is_virtual`
- `is_override`
- `is_abstract`
- `is_unsafe`
- `is_extern`
- `abi`
- `calling_convention`

### Signature

- `parameters`
- `return_type`
- `throws`
- `yield_type`
- `receiver`
    - e.g. `self`, `&self`, `&mut self`, `this`
- `generic_parameters`
- `where_clause` / `constraints`
- `variadic`
- `arity`
- `overload_set`
- `default_parameter_values`

### Parameters

Each parameter should itself be modeled almost like a symbol.

For each parameter:

- `name`
- `position`
- `label`
    - important for languages with named args / external names
- `type`
- `type_text`
- `type_ast`
- `is_optional`
- `is_rest` / `is_variadic`
- `is_keyword_only`
- `is_positional_only`
- `has_default`
- `default_value`
- `mutability`
- `ownership_mode`
    - by value
    - ref
    - mut ref
    - borrowed
    - moved
    - inout

- `nullability`
- `doc`
- `attributes`
- `decorators`
- `constraints`

### Return metadata

- `return_type`
- `return_type_text`
- `returns_by_reference`
- `nullability`
- `error_type`
    - very useful in Rust-like ecosystems
- `doc_returns`
- `can_return_early`
- `never_returns`

### Semantic function metadata

This is often inferred, not declared:

- `is_pure`
- `has_side_effects`
- `reads_global_state`
- `writes_global_state`
- `may_throw`
- `may_panic`
- `allocates`
- `blocks`
- `is_recursive`
- `calls`
- `called_by`
- `captures`
    - closures and lambdas

### Documentation details

- function-level description
- parameter docs by parameter name
- return docs
- throws docs
- examples
- remarks
- deprecation notice

If you want to preserve comments attached to parameters, keep both:

- `doc_tags.param["name"]`
- `parameter.doc`

That prevents loss when doc tags and inline parameter comments disagree.

---

## 4. Metadata typically needed for classes, structs, interfaces, and traits

These symbols are more about **members + inheritance + contracts**.

### Core

- `name`
- `qualified_name`
- `kind`
    - class
    - struct
    - interface
    - trait
    - protocol
    - record

- `visibility`
- `is_exported`
- `is_abstract`
- `is_final`
- `is_sealed`
- `is_open`
- `is_partial`

### Type parameters

- generic parameters
- bounds / constraints
- defaults
- variance if the language exposes it

### Inheritance / conformance

- `extends`
- `implements`
- `mixins`
- `traits`
- `superclass`
- `interfaces`
- `protocols`
- `conforms_to`

### Members

- fields
- properties
- methods
- constructors
- static members
- nested types
- associated types
- constants

Each member should have:

- name
- kind
- visibility
- static/instance
- mutability
- type
- docs
- attributes
- source span

### Class-level semantics

- `has_custom_constructor`
- `has_default_constructor`
- `is_instantiable`
- `is_copyable`
- `is_sendable`
- `is_sync`
- `is_thread_safe`
- `is_pod`
- `is_value_type`
- `is_reference_type`

Some of these are language-specific, but the pattern is useful.

### Docs

- summary
- remarks
- examples
- type parameter docs
- invariant docs
- usage notes

---

## 5. Metadata for objects, object literals, records, modules, and namespaces

These are often overlooked.

### For object-like symbols

- `name`
- `qualified_name`
- `kind`
    - object
    - module object
    - namespace
    - singleton
    - literal object
    - record

- `properties`
- `methods`
- `index_signatures`
- `call_signatures`
- `construct_signatures`
- mapped/dynamic properties
- `readonly`
- `frozen`
- `mutable`

### Property metadata

Each property or field should include:

- `name`
- `qualified_name`
- `kind`
    - field
    - property
    - constant
    - associated constant

- `visibility`
- `is_static`
- `readonly`
- `mutable`
- `optional`
- `computed_name`
- `type`
- `initializer`
- `default_value`
- `doc`
- `decorators`
- `attributes`

### Indexing and dynamic access

For languages that support it:

- string index signature
- numeric index signature
- symbol index signature
- key type
- value type

---

## 6. Metadata for type aliases, enums, unions, intersections, and generics

If your goal is “all components of a symbol”, these matter a lot.

### Type alias

- `name`
- `aliased_type`
- `aliased_type_ast`
- `generic_parameters`
- `constraints`
- `is_recursive_type`
- docs

### Enum

- `name`
- `backing_type`
- `variants`

For each variant:

- name
- discriminant
- associated fields/payload
- docs
- attributes

### Union / intersection

- constituent types
- discriminants
- narrowing tags
- exhaustiveness markers

### Generic parameter metadata

Treat generic parameters as first-class:

- `name`
- `kind`
    - type param
    - const param
    - lifetime param

- `constraints`
- `default`
- `variance`
- `doc`
- `position`
- `is_required`

For Rust especially, lifetimes and const generics matter if you want full type fidelity.

---

## 7. Comments and docs: attach them as structured data, not just text

You explicitly mentioned **including comments**, and that is the right instinct.

There are several kinds of comments worth distinguishing.

### A. Leading doc comments attached to symbol

Examples:

- Rust `///`
- JS/TS `/** ... */`
- Python docstrings
- JavaDoc / KDoc / XML docs

Store:

- raw text
- parsed structured tags
- source span

### B. Leading non-doc comments attached to symbol

These often contain valuable contextual notes.

Store:

- raw text
- whether the analyzer considers them attached
- line distance from declaration

### C. Inline comments

Examples:

- parameter comments
- field comments
- trailing comments on enum variants

Store:

- raw text
- attachment target
- attachment mode
    - leading
    - trailing
    - inline
    - detached

### D. Detached block comments

Sometimes a comment block is separated by whitespace but still semantically applies.

You may want heuristics for:

- nearest symbol
- same indentation
- no intervening non-comment tokens

### E. Parsed doc model

For full symbol capture, I would parse comments into:

- `summary`
- `description`
- `params`
- `returns`
- `throws`
- `examples`
- `remarks`
- `deprecated`
- `see_also`
- `default`
- `since`
- `authors`
- `references`

---

## 8. The hard part: “full type” is not one thing

When you say “any symbol’s full type”, there are usually at least **four versions** of that.

### 1. Syntactic declared type

Exactly what the source says.

Example:

- `fn foo<T: Display>(x: Option<T>) -> Result<String, Error>`

### 2. Canonical normalized type

Same meaning, normalized for comparison.

Examples:

- whitespace normalized
- fully qualified type paths
- reordered constraints if language semantics allow

### 3. Resolved semantic type

Links each type name to the symbol it refers to.

Examples:

- `Option<T>` resolves `Option` to a stdlib symbol ID
- `Error` resolves to a crate-local type alias or trait object

### 4. Expanded type

Useful, but dangerous:

- type aliases expanded
- inferred defaults applied
- maybe trait bounds expanded transitively

For many tools you want all four.

---

## 9. A practical cross-language symbol schema

A clean conceptual schema:

```text
Symbol
├── identity
├── source
├── visibility
├── docs
├── attributes
├── type_info
├── relationships
├── semantics
├── raw
└── kind_specific
```

A more concrete sketch:

```text
Symbol {
  id
  name
  qualified_name
  display_name
  symbol_kind
  language

  source: {
    file_path
    module_path
    declaration_span
    body_span
    doc_span
    source_text
    signature_text
  }

  visibility: {
    visibility
    is_exported
    is_reexported
    is_default_export
    is_external
    is_deprecated
    is_experimental
  }

  docs: {
    raw
    summary
    description
    tags
    examples
    remarks
    deprecated_message
  }

  attributes: [
    { name, arguments, raw_text }
  ]

  modifiers: [
    async, static, const, abstract, unsafe, readonly, final, sealed, ...
  ]

  type_info: {
    declared_type_text
    canonical_type_text
    type_ast
    generic_parameters
    constraints
    referenced_type_symbols
  }

  relationships: {
    parent_symbol_id
    container_symbol_id
    member_symbol_ids
    extends
    implements
    overrides
    references
    referenced_by
  }

  semantics: {
    is_pure
    has_side_effects
    may_throw
    may_panic
    is_recursive
    mutates_self
    mutates_arguments
    reads_global_state
    writes_global_state
  }

  kind_specific: FunctionInfo | ClassInfo | PropertyInfo | EnumInfo | ...
}
```

That pattern scales well.

---

## 10. Function-specific schema I would strongly recommend

Since functions are central, here is the minimum serious function model.

```text
FunctionInfo {
  receiver
  parameters[]
  return_type
  error_type
  generic_parameters[]
  where_constraints[]
  overloads[]
  is_variadic
  is_async
  is_generator
  is_const
  is_unsafe
  abi
}
```

And each parameter:

```text
ParameterInfo {
  id
  name
  position
  label
  type_text
  type_ast
  default_value
  has_default
  is_optional
  is_variadic
  nullability
  mutability
  ownership_mode
  attributes[]
  docs {
    raw
    summary
    description
  }
}
```

---

## 11. What people usually forget

These are the fields analyzers often omit and later regret omitting.

### Documentation fidelity

- raw docs vs parsed docs
- parameter docs separate from parameter type
- examples
- deprecation message, not just deprecation flag

### Generic metadata

- type parameters
- bounds
- defaults
- const generics
- lifetimes

### Exact source text

- declaration text
- signature text
- attached comments raw

### Symbol relationships

- parent/container
- overridden method
- implemented interface member
- referenced type symbols

### Modifiers and annotations

- decorators
- attributes
- macros involved
- derive/annotation metadata

### Semantic flags

- pure / impure
- throws / panics
- mutates self / mutates args
- async / blocking / allocating

### Multiple type views

- declared
- canonical
- resolved
- expanded

---

## 12. Tree-sitter-specific reality check

Since you are using tree-sitter: tree-sitter is excellent for **syntax**, but not by itself for full semantic typing.

Think in terms of the following layers.

### What tree-sitter can give you well

- declaration boundaries
- names
- modifiers
- parameter lists
- return type syntax
- generic syntax
- comments and doc blocks
- attributes, decorators, and macros
- containment hierarchy
- exact source spans

### What usually requires another layer

- resolved types
- symbol linking
- export analysis across files
- inheritance resolution
- type alias expansion
- overload unification
- semantic flags like purity or side effects

So your architecture probably wants:

1. **Parse layer** via tree-sitter  
   Raw symbol extraction.

2. **Binding and resolution layer**  
   Connect names to symbols.

3. **Semantic enrichment layer**  
   Infer export status, purity, mutability, dependencies, and similar properties.

4. **Documentation layer**  
   Parse JSDoc, Rustdoc, and similar documentation.

If you try to force all of this into one pass, it gets brittle.

---

## 13. A good minimum viable “complete symbol” set

If I were designing v1 of this system, I would capture at least the following.

### For all symbols

- name
- qualified name
- symbol kind
- file path
- declaration span
- visibility/export state
- raw declaration text
- raw attached docs/comments
- parsed docs
- modifiers/attributes
- declared type text
- canonical type text
- parent/container relationship

### For functions and methods

- parameters with:
    - name
    - type
    - optional/default/variadic status
    - docs

- return type
- generics
- constraints
- receiver
- async/static/const/unsafe/etc.
- throws/error metadata

### For classes, structs, interfaces, and traits

- generics
- extends/implements
- fields/properties
- methods
- constructors
- docs
- annotations/modifiers

### For fields and properties

- type
- mutability/readonly
- optional/default
- visibility
- static/instance
- docs

That already gets you very far.

---

## 14. Recommendation: model symbols as composable facets

Instead of one giant struct with 150 optional fields, use composable pieces.

For example:

- `IdentityFacet`
- `SourceFacet`
- `DocsFacet`
- `VisibilityFacet`
- `TypeFacet`
- `ModifierFacet`
- `RelationFacet`
- `SemanticFacet`
- `FunctionFacet`
- `ClassFacet`
- `PropertyFacet`
- `EnumFacet`

That makes Rust modeling much cleaner because:

- common fields are reusable,
- symbol-specific fields are explicit,
- serialization stays understandable,
- you avoid `Option<Option<Vec<_>>>` hell.

---

## 15. Concise completeness checklist

When asking “have I fully captured this symbol?”, use this checklist.

### Symbol completeness checklist

- **Who is it?**
    - name
    - qualified name
    - kind

- **Where is it?**
    - file and span

- **Can others access it?**
    - visibility/export status

- **What exactly was written?**
    - raw declaration and comments

- **What does it mean structurally?**
    - full type/signature

- **What are its parts?**
    - parameters
    - fields
    - methods
    - variants
    - generics

- **How is it constrained?**
    - bounds
    - where clauses
    - defaults

- **How is it documented?**
    - summary
    - detailed docs
    - parameter docs
    - examples

- **How does it relate to others?**
    - parent
    - members
    - inheritance
    - references

- **What semantic behavior matters?**
    - async
    - mutability
    - side effects
    - throws/panics
    - purity

---

## 16. Final recommendation

If you want a schema that can support documentation, static analysis, semantic search, and API diffing, then each symbol should generally preserve:

- **identity**
- **location**
- **visibility**
- **raw declaration**
- **comments/docs**
- **type contract**
- **modifiers/attributes**
- **relationships**
- **semantic inference**
- **kind-specific detail**

The most important design choice is to avoid treating “type” as a single string. In practice you usually want:

- declared type
- canonical type
- resolved type
- expanded type

And for comments/docs, keep both:

- **raw attached comment text**
- **parsed structured doc data**

That combination gives you a much better chance of representing the symbol with high fidelity.

If helpful, the next logical step is to define an actual Rust schema with:

- `Symbol`
- `SymbolKind`
- `SymbolIdentity`
- `SourceSpan`
- `DocComment`
- `TypeRef`
- `FunctionMetadata`
- `ClassMetadata`
- `PropertyMetadata`
- `EnumMetadata`

so you have a concrete, serializable model to implement against tree-sitter.
