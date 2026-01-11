# Language Grammars

## Supported Grammars

### Via `syntect`

- Systems programming: `C`,`C++`,`Rust`,Go`,`Haskell`, and `Erlang`
- Web: `HTML`, `CSS`, `Javascript`, `PHP`, `ASP`, `Ruby on Rails`
- Scripting: `Python`, `Ruby`, `Perl`, `Bash`, `Lua`, and `TCL`
- Data/Configuration: `XML`, `JSON`, `YAML`, `Markdown`, and `Makefile`
- Java Ecosystem: `Java`, `Scala`, `Groovy`
- Apple Ecosystem: `Objective-C`, `Swift`
- Windows Ecosystem: `C#`, `Batch(DOS)`

### Via `two-face`

`two-face` -- which takes it's grammars from the [`bat`](https://github.com/sharkdp/bat) project -- provides a **super set** of grammars by taking in all the syntect grammars, in some cases providing new features to them and then adding new grammars on top.

1. **Modern Frontend**

   While syntect handles basic JS/CSS, two-face covers the modern ecosystem:

   - Frameworks: `JSX`, `TSX (React)`, `Vue Component`, `Svelte`, `Angular Template`.
   - Stylesheets: `SCSS`, `Sass`, `Less`, `Stylus`, `PostCSS`.
   - Languages: `CoffeeScript`, `Elm`, `PureScript`, `Dart`.

2. **Infrastructure & DevOps**

   This is where syntect is most lacking and two-face shines:

   - Infrastructure as Code: `Terraform (HCL)`, `CloudFormation`.
   - Containerization: `Dockerfile`, `Docker Compose` (via YAML enhancements).
   - Config Files: `Dotenv` (.env), `INI`, `Crontab`, `SshConfig`, `Systemd unit files`.
   - Web Servers: `Nginx`, `Apache Conf` (.htaccess).
   - Logging: `Syslog`, `Log file` (generic colorizers).

3. Systems & Niche Languages
   - Modern Systems: `Zig`, `Nim`, `Crystal`, `LLVM IR`.
   - Functional: `Clojure`, `F#`, `OCaml`, `Elixir`.
   - Niche/Scientific: `Julia`, `MATLAB`, `R`, `LaTeX`, `Protocol Buffers` (Protobuf), `GraphQL`.

4. **Shell & Scripting**

    `syntect` has **Bash**, but `two-face` adds:

    - Modern Shells: `Fish`, `Zsh`.
    - Windows: `PowerShell`, `Regedit` (.reg).
    - Automation: `Ansible` (extended YAML), `Jenkinsfile` (Groovy variants).

5. **Documentation and Data**

    - Data Formats: `TOML` (essential for Rust), `CSV`, `TSV`.
    - Docs: `Asciidoc`, `reStructuredText` (RST), `Org-mode`.

To know what grammars a particular binary supports you can run the following:

```rust
let ps = two_face::syntax::extra_newlines(); 

for syntax in ps.syntaxes() {
    println!("Name: {} | Extensions: {:?}", syntax.name, syntax.file_extensions);
}
```

### Matching Language Shortcuts

`syntect` provides some important matching utilities we will need to use when trying to match the language provided in a document's fenced code blocks:

```rust
use syntect::parsing::SyntaxSet;

fn main() {
    let ps = two_face::syntax::extra_newlines();

    // Map "ts" -> TypeScript
    let ts_syntax = ps.find_syntax_by_token("ts"); 
    
    // Map "rust" -> Rust
    let rs_syntax = ps.find_syntax_by_token("rust");

    if let Some(s) = ts_syntax {
        println!("Token 'ts' mapped to: {}", s.name); // Prints "TypeScript"
    }
}
```

