---
description: Extracts lots of useful metadata in a Rust workspace
depends_on:
    binaries:
        - jq
        - cargo-outdated
---
# Dependency Upgrade Planner

You are acting as a senior technical analyst for the Rusty Biscuit monorepo. Your task is to create a phased upgrade plan for this Rust monorepo.

## Metadata

We have compiled a large set of useful metadata for your analysis.

### Cargo Metadata

All of the cargo metadata has been saved to disk: [metadata](target/dependency-report/metadata.json).
::shell cargo metadata --format-version 1 --all-features --locked > "{{ctx.repo_root}}/target/dependency-report/metadata.json"


### Cargo Tree Duplicates

::shell cargo tree --workspace --duplicates > "{{ctx.repo_root}}/target/dependency-report/cargo-tree-duplicates.txt" 2>&1 && echo "The results of the `cargo tree --workspace --duplicates` can be found at: [tree-duplicates](target/dependency-report/cargo-tree-duplicates.txt)" || echo "**Important:** running `cargo tree --workspace --duplicates` currently fails; you can see the error output at [tree-duplicates](target/dependency-report/cargo-tree-duplicates.txt)."

### Cargo Tree Features

::shell cargo tree --workspace -e features > "{{ctx.repo_root}}/target/dependency-report/cargo-tree-features.txt" 2>&1 && echo "The results of `cargo tree --workspace --e features` can be found at: [tree-features](target/dependency-report/cargo-tree-features.txt)"|| echo "**Important:** running `cargo tree --workspace --duplicates` currently fails; you can see the error output at [tree-features](target/dependency-report/cargo-tree-features.txt)."


### Outdated Report

::shell cargo-outdated outdated && echo "" || echo "**Important:** `cargo outdated --workspace --format json` currently fails due to a resolver conflict. Treat this as the first upgrade blocker, not as missing data. The error can be found at [outdated-errors](target/depencency-report/)"

### Workspace Dependencies

#### Direct Dependencies

::shell-block 
jq '[ .workspace_members[] as $member_id
    | .packages[] | select(.id == $member_id) as $pkg
    | $pkg.dependencies[]
    | {
        dependency_name: .name,
        package_name: $pkg.name,
        manifest_path: $pkg.manifest_path,
        req: .req,
        kind: (.kind // "normal"),
        optional: .optional,
        uses_default_features: .uses_default_features,
        features: .features,
        target: .target,
        rename: .rename,
        source: .source,
        registry: .registry,
        path: .path
      }
  ]
' "{{ctx.repo_root}}/target/dependency-report/metadata.json" \
  > "{{ctx.repo_root}}/target/dependency-report/workspace-direct-dependencies.json" \
  && echo "The direct dependencies this repo has have been cataloged in [direct dependencies](target/dependency-report/workspace-direct-dependencies.json)"
::end-block

#### Transient Dependencies

::shell-block
jq '
  [
    .workspace_members[] as $member_id
    | .packages[] | select(.id == $member_id) as $pkg
    | $pkg.dependencies[]
    | {
        dependency_name: .name,
        declaration: {
          package_name: $pkg.name,
          manifest_path: $pkg.manifest_path,
          req: .req,
          kind: (.kind // "normal"),
          optional: .optional,
          uses_default_features: .uses_default_features,
          features: .features,
          target: .target,
          rename: .rename,
          source: .source,
          registry: .registry,
          path: .path
        }
      }
  ]
  | group_by(.dependency_name)
  | map({
      dependency_name: .[0].dependency_name,
      declarations: [.[] | .declaration],
      declared_by_count: length,
      requirement_set: [.[] | .declaration.req] | unique,
      kinds: [.[] | .declaration.kind] | unique,
      optional_values: [.[] | .declaration.optional] | unique,
      targets: [.[] | .declaration.target] | unique
    })
  | sort_by(.dependency_name)
' "{{ctx.repo_root}}/target/dependency-report/metadata.json" \
  > target/dependency-report/upgrades/workspace-dependency-requirements-by-crate.json \
  && echo "The transient dependencies this repo has, have been cataloged in [direct dependencies](target/dependency-report/workspace-direct-dependencies.json)"
::end-block

### Resolved External Package Metadata

With this data, we're answering the question "what external package are actually resolved, and do they have risky traits?"

::shell-block
jq '
  [
    .packages[]
    | select(.source != null)
    | {
        id,
        name,
        version,
        source,
        license,
        license_file,
        description,
        repository,
        homepage,
        documentation,
        rust_version,
        links,
        has_build_script: any(.targets[]?; .kind | index("custom-build")),
        is_proc_macro: any(.targets[]?; .kind | index("proc-macro")),
        target_kinds: ([.targets[]?.kind[]?] | unique),
        features
      }
  ]
' "{{ctx.repo_root}}/target/dependency-report/metadata.json" \
  > "{{ctx.repo_root}}/target/dependency-report/resolved-external-package-metadata.json" \
  && echo "The data can be found at: [Resolved External Package Metadata](target/dependency-report/resolved-external-package-metadata.json)"
::end-block

### Resolved Duplicate Version Groups

With this data, we're answering the question "Which external crates are resolved at multiple versions?"

::shell-block
jq '
  [
    .packages[]
    | select(.source != null)
    | {name, version, id, source}
  ]
  | group_by(.name)
  | map(select(length > 1))
  | map({
      name: .[0].name,
      versions: [.[].version] | unique,
      version_count: ([.[].version] | unique | length),
      packages: .
    })
  | sort_by(.name)
' "{{ctx.repo_root}}/target/dependency-report/metadata.json" \
  > "{{ctx.repo_root}}/target/dependency-report/upgrades/resolved-duplicate-version-groups.json" \
  && echo "The data can be found at: [Resolved Duplicate Version Groups](target/dependency-report/resolved-duplicate-version-groups.json)"
::end-block

### Workspace Controlled Duplicates

With this data, we're answer the question "Which duplicated crates are directly controlled by workspace manifests?"

::shell-block
jq -n \
  --slurpfile reqs target/dependency-report/upgrades/workspace-dependency-requirements-by-crate.json \
  --slurpfile dupes target/dependency-report/upgrades/resolved-duplicate-version-groups.json '
  ($reqs[0]) as $reqs
  | ($dupes[0]) as $dupes
  | [
      $dupes[]
      | . as $dupe
      | ($reqs[] | select(.dependency_name == $dupe.name)) as $req
      | {
          name: $dupe.name,
          resolved_versions: $dupe.versions,
          version_count: $dupe.version_count,
          direct_workspace_declarations: $req.declarations,
          requirement_set: $req.requirement_set,
          kinds: $req.kinds,
          targets: $req.targets
        }
    ]
  | sort_by(.name)
' > "{{ctx.repo_root}}/target/dependency-report/upgrades/workspace-controlled-duplicates.json" \
&& echo "The data can be found at: [Workspace Controlled Duplicates](target/dependency-report/workspace-controlled-duplicates.json)"
::end-block

### Upgrade Risk Surface

This report should help to distinguish "easy bump" from "isolate carefully."

::shell-block
jq '
  [
    .[]
    | . + {
        upgrade_risk_flags: (
          []
          + (if .is_proc_macro then ["proc-macro"] else [] end)
          + (if .has_build_script then ["build-script"] else [] end)
          + (if .links != null then ["native-links"] else [] end)
          + (if (.source // "" | startswith("git+")) then ["git-source"] else [] end)
          + (if (.name | test("openssl|ssl|tls|rustls|ring|aws-lc|crypto|argon|bcrypt|sha|hmac|aes|x509|webpki"; "i")) then ["crypto-or-tls"] else [] end)
          + (if (.name | test("tokio|hyper|reqwest|axum|tower|tungstenite|websocket|h2|oauth|tonic"; "i")) then ["networking"] else [] end)
          + (if (.name | test("sqlx|rusqlite|sqlite|postgres|mysql|diesel|sea-orm"; "i")) then ["database"] else [] end)
          + (if (.name | test("pdf|lopdf|image|png|jpeg|jpg|gif|webp|tiff|svg|resvg|usvg|xml|html|scraper|nom|pest|regex"; "i")) then ["parser-or-media"] else [] end)
          + (if (.name | test("sys$|bindgen|cc|cmake|libgit2|libssh2|sqlite|openssl-sys|coreaudio|onig|pcre2"; "i")) then ["native-build-surface"] else [] end)
          + (if (.name | test("clap|ratatui|crossterm|inquire|tui|console|indicatif"; "i")) then ["cli-or-tui"] else [] end)
        )
      }
  ]
  | map(select(.upgrade_risk_flags | length > 0))
  | sort_by(.name)
' "{{ctx.repo_root}}/target/dependency-report/upgrades/resolved-external-package-metadata.json" \
  > "{{ctx.repo_root}}/target/dependency-report/upgrades/upgrade-risk-surfaces.json" \
  && echo "The report can be found at: ${}"
::end-block

### Upgrade Family View

::shell-block
jq -n \
  --slurpfile packages target/dependency-report/upgrades/resolved-external-package-metadata.json \
  --slurpfile controlled target/dependency-report/upgrades/workspace-controlled-duplicates.json '
  def names_matching($re):
    [
      $packages[0][]
      | select(.name | test($re; "i"))
      | {
          name,
          version,
          source,
          has_build_script,
          is_proc_macro,
          links
        }
    ];

  def controlled_matching($re):
    [
      $controlled[0][]
      | select(.name | test($re; "i"))
    ];

  [
    {
      family: "cli-tui",
      rationale: "Terminal UI, prompts, console rendering, and CLI argument parsing often need coordinated upgrades.",
      packages: names_matching("clap|ratatui|crossterm|inquire|tui|console|indicatif|unicode-width|unicode-truncate|compact_str|strum"),
      workspace_controlled_duplicates: controlled_matching("clap|ratatui|crossterm|inquire|tui|console|indicatif|unicode-width|unicode-truncate|compact_str|strum")
    },
    {
      family: "http-tls-networking",
      rationale: "HTTP, TLS, websocket, and async networking upgrades can affect runtime behavior and feature selection.",
      packages: names_matching("reqwest|hyper|rustls|native-tls|openssl|tower|axum|tungstenite|tokio-tungstenite|h2|oauth|webpki"),
      workspace_controlled_duplicates: controlled_matching("reqwest|hyper|rustls|native-tls|openssl|tower|axum|tungstenite|tokio-tungstenite|h2|oauth|webpki")
    },
    {
      family: "pdf-document-parsing",
      rationale: "PDF and document parsing crates process complex input and should be upgraded or feature-gated carefully.",
      packages: names_matching("pdf|lopdf|pdf-extract"),
      workspace_controlled_duplicates: controlled_matching("pdf|lopdf|pdf-extract")
    },
    {
      family: "svg-image-rendering",
      rationale: "SVG and image rendering crates tend to move together and affect binary size, parsing surface, and rendering behavior.",
      packages: names_matching("resvg|usvg|image|png|jpeg|jpg|gif|webp|tiff|zune|kurbo|roxmltree|svgtypes|tiny-skia"),
      workspace_controlled_duplicates: controlled_matching("resvg|usvg|image|png|jpeg|jpg|gif|webp|tiff|zune|kurbo|roxmltree|svgtypes|tiny-skia")
    },
    {
      family: "git-native-build",
      rationale: "Git/native dependencies involve build scripts, system libraries, OpenSSL/libgit2/libssh2, and platform-specific failures.",
      packages: names_matching("git2|libgit2|libssh2|openssl-sys|libz-sys"),
      workspace_controlled_duplicates: controlled_matching("git2|libgit2|libssh2|openssl-sys|libz-sys")
    },
    {
      family: "audio-platform",
      rationale: "Audio and platform bindings often include native APIs and target-specific behavior.",
      packages: names_matching("cpal|rodio|coreaudio|symphonia|bindgen"),
      workspace_controlled_duplicates: controlled_matching("cpal|rodio|coreaudio|symphonia|bindgen")
    },
    {
      family: "serialization-config",
      rationale: "serde/json/yaml/toml/config crates are broad foundations; version skew may indicate deprecated YAML or parser stacks.",
      packages: names_matching("serde|serde_json|serde_yaml|serde_yaml_ng|toml|toml_edit|winnow|json|yaml"),
      workspace_controlled_duplicates: controlled_matching("serde|serde_json|serde_yaml|serde_yaml_ng|toml|toml_edit|winnow|json|yaml")
    },
    {
      family: "random-crypto-foundation",
      rationale: "rand/getrandom and crypto foundations often reveal old transitive anchors.",
      packages: names_matching("rand|getrandom|ring|argon|sha|hmac|aes|crypto"),
      workspace_controlled_duplicates: controlled_matching("rand|getrandom|ring|argon|sha|hmac|aes|crypto")
    }
  ]
  | map(. + {
      package_count: (.packages | length),
      controlled_duplicate_count: (.workspace_controlled_duplicates | length)
    })
' > "{{ctx.repo_root}}/target/dependency-report/upgrade-families.json" \
&& echo "The family view data be found at: [Upgrade Family View](target/dependency-report/upgrade-families.json)"
::end-block


### Feature Usage

This extracts feature configuration from direct workspace declarations, avoiding the need to scan every manifest.

::shell-block
jq '
  [
    .[]
    | select(
        (.features | length > 0)
        or (.uses_default_features == false)
        or (.optional == true)
        or (.target != null)
      )
    | {
        dependency_name,
        package_name,
        manifest_path,
        req,
        kind,
        optional,
        uses_default_features,
        features,
        target
      }
  ]
  | sort_by(.dependency_name, .package_name)
' "{{ctx.repo_root}}/target/dependency-report/workspace-direct-dependencies.json" \
  > "{{ctx.repo_root}}/target/dependency-report/workspace-dependency-feature-usage.json" \
  && echo "Feature usage data can be found at: [Feature Usage Data](target/dependency-report/workspace-dependency-feature-usage.json)"
::end-block

## Task

- identify dependency families that should be upgraded together
- identify direct workspace dependency requirements that anchor older versions
- identify transitive-only duplicates that should be left alone for now
- propose an upgrade order
- distinguish patch/minor/major/consolidate/feature-gate/replace/leave-alone
- include validation commands after each phase

Prioritize:

1. blockers that prevent tooling from running
2. direct workspace-controlled duplicate families
3. low-risk leaf upgrades
4. domain-specific stacks such as TUI, SVG/rendering, PDF, HTTP/TLS, Git/native, audio/platform
5. old transitive anchors that may require replacement or feature-gating

**IMPORTANT:** DO NOT recommend upgrading everything blindly.

## Closure

To complete this task you must create a high confidence multi-phase plan to upgrade the repo's dependencies.

- the plan should be saved to "reviews/{{ctx.today}}-dependency-upgrade/plan.md" as a idiomatic Markdown document
- once saved you will add and the following Frontmatter properties to "reviews/{{ctx.today}}-dependency-upgrade/plan.md":
    - `phases` - the last phase number in the plan
    - `start_phase` - typically 1 (but sometimes 0) representing the index of the first phase of work
