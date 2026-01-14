# Sniff Package

## Features

- We need to add some additional CLI switches to narrow down what we are reporting on. We already have "top level" switches like `--filesystem` and `--hardware` which help to isolate what we're reporting but we should add one more to the top level -- `--os` for operating system level info -- and several more which target a more detailed level:

    - `--git` isolates to reporting only on the filesystem/git info
    - `--monorepo` isolates to on the filesystem/monorepo info
    - `--language` isolates on the filesystem/language info
- FIX filesystem/monorepos and filesystem/dependencies
    - The filesystem/dependencies section is useless currently
