# Package Structure

## Layout

Most packages follow a **lib/cli split**:

```txt
📂 package-name/
├── README.md          # Package overview (functional goals, links to sub-modules)
├── justfile           # DevOps: build, lint, test, install
├── 📂 docs/           # Research, dependency docs, design notes
├── 📂 lib/
│   └── README.md      # Technical deep dive (architecture, crates, lessons learned)
└── 📂 cli/
    └── README.md      # CLI usage, flags, examples
```

Some packages have more sub-modules (e.g., `schematic` has `define/`, `definitions/`, `gen/`, `schema/`). The same README-per-sub-module pattern applies.

Single-crate packages (e.g., `so-you-say`, `model_id`) have one README that covers both functional and technical concerns.

## README Conventions

### Base README (package root)

- **What** the package does and **why** it exists - functional goals over technical details
- Links to each sub-module README with bullet points describing what each covers

### Sub-module READMEs

- Detailed functional goals
- Technical approach and architecture
- Key crates used and how they contribute to the solution
- **Lessons learned** section - important realizations discovered over time (create the heading even if empty initially)

## The `docs/` Folder

Found at the package or sub-package level:

- **`docs/dependencies.md`** - Dependencies with short descriptions (most common file)
- **Research documents** - Design explorations, crate evaluations, and planning artifacts; rich in detail but less relevant for day-to-day tasks. These should be incorporated into any skill being developed for the package.

## AI-Generated Documents

Some documents were produced by AI and include a `prompt` frontmatter property. Find them with:

```bash
sniff docs --has-prompt
```

## Source Code READMEs

A `README.md` may appear inside source subdirectories to provide context for a specific module or directory of code.

## Avoiding Drift

When modifying code, **update the relevant documentation in the same change**. Documentation drift - where docs describe something that no longer matches the implementation - erodes trust and causes mistakes. Treat docs as part of the deliverable, not an afterthought.

Key areas to check:

- README files when changing public APIs or behavior
- `docs/dependencies.md` when adding or removing crates
- Skill files (`.claude/skills/`) when changing patterns or architecture


