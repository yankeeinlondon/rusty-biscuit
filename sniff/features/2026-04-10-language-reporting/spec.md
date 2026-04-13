The current `sniff language` CLI command outputs something like:

```txt
💻❯ sniff language
Primary language: Rust

 ┌──────────────────┬─────────────────────────────────┬────────┐
 │ Language         │                           Usage │ Signal │
 ├──────────────────┼─────────────────────────────────┼────────┤
 │ Rust             │ 95 direct, 0 framework (100.0%) │  95.00 │
 └──────────────────┴─────────────────────────────────┴────────┘

- analyzed 227 files, 95 contributed to language selection
```

This isn't terrible but it's ambiguous in ways it shouldn't be.

- in a monorepo what makes sense is reporting on the primary language for a given "package" in that monorepo
    - however, if we call this command in a particular "package area" then we should report on all of the packages in this package area 
    - similarly if we're at the root of a monorepo then we should show all packages individually 
- in a non-monorepo then our current reporting may be close to correct but we need to be more explicit about the scope


## Monorepos

### In Monorepo Root

When we're in the root folder (or at least not in a "package area") of the monorepo then:

```sh
In the <yellow>{repo}</yellow> monorepo has {#} packages defined:

- <b>{package}</b>

    ┌──────────────────┬─────────────────────────────────┬────────┐
    │ Language         │                           Usage │ Signal │
    ├──────────────────┼─────────────────────────────────┼────────┤
    │ Rust             │ 95 direct, 0 framework (100.0%) │  95.00 │
    └──────────────────┴─────────────────────────────────┴────────┘
- etc.
```

### In Package Area

When we're in a package area the report should look like:

```sh
In the <yellow>{repo}</yellow> monorepo's "{package_area}" package area we have {#} packages defined:

- <b>{package}</b>

    - <b>{{language}}</b> is the primary language in this repo

      ┌──────────────────┬─────────────────────────────────┬────────┐
      │ Language         │                           Usage │ Signal │
      ├──────────────────┼─────────────────────────────────┼────────┤
      │ Rust             │ 95 direct, 0 framework (100.0%) │  95.00 │
      └──────────────────┴─────────────────────────────────┴────────┘

- etc.
```

### In Package

When we're within the actual folder/directory of a specific package then:

```sh
In the <yellow>{repo}</yellow> monorepo's <b>{package}</b> package (<dim><i>1 of {#} packages</i></dim>):

- <b>{{language}}</b> is the primary language in this package

    ┌──────────────────┬─────────────────────────────────┬────────┐
    │ Language         │                           Usage │ Signal │
    ├──────────────────┼─────────────────────────────────┼────────┤
    │ Rust             │ 95 direct, 0 framework (100.0%) │  95.00 │
    └──────────────────┴─────────────────────────────────┴────────┘

```


## Non-monorepos

```sh
Evaluating the <yellow>{repo}</yellow> (<dim><i>a non-monorepo repo</i>/<dim>) programming languages:

- <b>{{language}}</b> is the primary language in this repo

    ┌──────────────────┬─────────────────────────────────┬────────┐
    │ Language         │                           Usage │ Signal │
    ├──────────────────┼─────────────────────────────────┼────────┤
    │ Rust             │ 95 direct, 0 framework (100.0%) │  95.00 │
    └──────────────────┴─────────────────────────────────┴────────┘
    
```
