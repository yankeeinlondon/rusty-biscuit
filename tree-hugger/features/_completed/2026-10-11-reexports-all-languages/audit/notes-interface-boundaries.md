# Audit notes: interface-boundary languages

Scope: Java, PHP, Bash, Zsh, C, C++, C#. Phase 1 of the
`2026-10-11-reexports-all-languages` spec. This file is a working audit
record, not a decision. Recommendations are for the author.

## Method and evidence conventions

- Probe: disposable `th-probe` built on the repo's pinned grammars (java
  0.23.5, php 0.24.2 `LANGUAGE_PHP`, bash 0.25.1, zsh 0.56.0, c 0.24.2, cpp
  0.23.4, c-sharp 0.23.5; tree-sitter 0.26.9). The stock probe prints only
  named nodes. Anonymous tokens that carry meaning (`static`, `global`,
  `namespace`, `export`, `function`/`const` in PHP, `open`) were read with a
  copy of the probe that also prints anonymous tokens (`/tmp/th-probe-anon`,
  `ANON=1`). "Anon token" below means: visible only that way.
- Samples: `audit/samples/<lang>/`. `pos-` is a positive construct, `neg-` a
  nearby negative, `amb-` a dynamic or ambiguous variant. Raw probe output is
  not stored; re-run `th-probe <lang> <file>`.
- "Parses clean" = `has_error=false` and no `MISSING` for the whole sample.
  Where it is false the offending construct is named.
- Semantics sources were fetched on 2026-10-11. Anything without a fetched
  source says **source not verified**.
- Classification vocabulary (spec): *dedicated syntax*, *convention*, *no
  applicable construct*. Grammar gaps are recorded separately and are never
  evidence that a language lacks the concept.
- Spec definition used: a name or module obtained elsewhere is exposed
  through this module's interface. Spec out-of-scope list also bars inferring
  interfaces "solely from ... include statements".
- Pinned-grammar currency (crates.io `max_version`, 2026-10-11): java 0.23.5,
  c 0.24.2, c-sharp 0.23.5, bash 0.25.1, cpp 0.23.4 are the newest published.
  php has 0.25.1 and zsh has 0.63.4 published. No upgrade was tried (out of
  scope); newer versions are not evidence either way.

---

## Java

Dialect: Java SE 21. Source: JLS SE 21 chapter 7,
<https://docs.oracle.com/javase/specs/jls/se21/html/jls-7.html>.

### Constructs

| Construct | Sample | Nodes / fields (parses clean: yes unless noted) | Semantics | Class. |
|---|---|---|---|---|
| Single-type import `import java.util.List;` | `java/pos-import.java` | `import_declaration > scoped_identifier(scope:, name:)` | JLS 7.5: names are visible "only within the compilation unit that actually contains the import" | Outside definition |
| On-demand import `import java.util.*;` | same | `import_declaration > scoped_identifier, asterisk` (named node) | same | Outside |
| Static import `import static java.lang.Math.max;` and `.*` | same | identical shape to ordinary import; the only difference is anon token `static` | JLS 7.5.3/7.5.4, same file-local scope | Outside |
| `requires transitive M` | `java/pos-module-info.java` | `module_declaration(name:) > module_body > requires_module_directive(modifiers: requires_modifier "transitive", module:)` | JLS 7.7.1: any module that requires this one gets an implicit dependence on M ("implied readability") | Dubious, see below |
| `requires static transitive`, plain `requires` | same | modifiers repeat as separate `modifiers:` fields (`static`, then `transitive`) | 7.7.1 | Plain `requires` outside |
| `exports p;` / `exports p to m1, m2;` | same | `exports_module_directive(package:, modules:*)` | 7.7.2: exposes this module's own package | Outside (local export) |
| `opens p [to m]` | same | `opens_module_directive(package:, modules:*)` | 7.7.2: run-time/reflective access, not compile-time | Outside |
| `uses` / `provides ... with` | same | `uses_module_directive(type:)`, `provides_module_directive(provided:, <unfielded scoped_identifier>)` | service wiring | Outside |
| `open module` | `java/neg-open-module.java` | `module_declaration`; `open` is an anon token | 7.7: open module grants run-time access to all packages | Outside |
| Fully qualified use, nested type extends imported type | `java/neg-no-import.java`, `java/neg-field-alias.java` | no import nodes | no name is exposed by using a type | Negative control |

Note `module-info.java` parses to `program > module_declaration`, with no
`class_declaration`, so it is a different top-level shape from ordinary files.

### Semantic argument

- Ordinary and static imports are never forwarded. JLS 7.5 says the scope
  "specifically does not include other compilation units in the same
  package". There is no `export ... from` equivalent, and no alias form.
  Verdict: **no applicable construct** at the source-file level.
- `requires transitive` is the only forwarding-like construct. JLS 7.7.1
  includes a pointed note: package `p` in `m.D` "is not 're-exported' by
  `m.C` and `m.B`; rather, `m.A` is made to read `m.D` directly". The
  specification itself declines the word.

**Argument for counting it.** It is dedicated syntax with a one-statement,
fully static meaning: module `X` exposes module `M` through its interface.
It matches the spec's own wording "a name or module obtained elsewhere" and
the spec's "namespace/module forwarding" form. It parses cleanly with named
fields. It is the nearest thing Java has.

**Argument against.** (1) It forwards a *dependency edge*, not a name:
no symbol of `M` is exposed, only readability, and the consumer resolves
`M`'s exports itself. (2) Its subject is a module, not a file or symbol, and
it exists in exactly one special file. (3) The feature is "symbol-oriented"
(the spec's Java row asks whether dependency exposure belongs here). (4) The
unused-import diagnostic has nothing to exempt: no import statement is
involved. (5) It would be the only construct whose expected result has no
exported name and no scope other than the module.

**Recommendation to the author.** Report Java as *no applicable construct*
for symbols. If the author wants module-level coverage, treat
`requires transitive` as a separate "module dependency exposure" evidence
kind (form `module`, source = module name, no alias) rather than as a
member of the symbol re-export list, and do not use it for lint exemptions.
This is a design choice, not a grammar issue.

### Expected result if `requires transitive` is admitted

| Field | Value for `requires transitive java.sql;` |
|---|---|
| exported name | none (module form) |
| original / member path | none |
| source module path | `java.sql` |
| enclosing scope | `module com.example.app` |
| alias | none |
| form | namespace/module (not wildcard, not named) |
| qualifiers | `static` for `requires static transitive`; qualified-ness absent |
| conditional | never |
| coverage limit | none beyond "module-info.java only"; module name may be arbitrary dotted path as written |

### Grammar gaps

None found. All module directive forms parse clean. `static`/`open` are anon
tokens, so a naive named-node query cannot distinguish `import static` from
`import`; the extractor must read the anon child.

---

## PHP

Dialect: PHP 8.x manual (the manual pages fetched do not pin a minor
version). Sources:
<https://www.php.net/manual/en/language.namespaces.importing.php>,
<https://www.php.net/manual/en/function.class-alias.php>,
<https://www.php.net/manual/en/function.require-once.php>.

### Constructs

| Construct | Sample | Nodes / fields | Semantics | Class. |
|---|---|---|---|---|
| `use Foo\Bar;`, `use Foo\Bar as Baz;` | `php/pos-use.php` | `namespace_use_declaration > namespace_use_clause > qualified_name(prefix: namespace_name, name)`, optional `alias: name` | Compile-time, per file; "included files will NOT inherit the parent file's importing rules" | Outside |
| Group use `use Foo\{A, B as C, D\E};` | same | `namespace_use_declaration(namespace_name, body: namespace_use_group > namespace_use_clause*)`; clauses hold bare `name` | same | Outside |
| `use function` / `use const` | same | anon-token field `type: function` / `type: const` on the clause (or on the declaration); invisible in the stock probe | same | Outside |
| Mixed group `use Foo\Sub\{function g, const H, I}` | same | `type:` field on each clause | same | Outside |
| Leading-backslash `use \Foo\Root;` | same | prefix anon `\` | same | Outside |
| `include`, `include_once`, `require`, `require_once` | `php/pos-include.php` | `expression_statement > include_expression / include_once_expression / require_expression / require_once_expression`; operand is `string`, `binary_expression` (`__DIR__ . '/d.php'`), or `parenthesized_expression` | Executes the file at run time in the includer's scope; `_once` skips repeats | Dubious, see below |
| `class_alias('Foo\Bar', 'Baz')` | `php/pos-class-alias.php` | `function_call_expression(function: name "class_alias", arguments > argument*)`; arguments are `string > string_content`, or `class_constant_access_expression` for `\Foo\Bar::class`, and a third `boolean` | Run-time global alias of "exactly the same" class; introduced 5.3.0; internal classes from 8.3.0; alias stored lowercase | Dubious, see below |
| Trait `use T;`, closure `use ($x)` | `php/neg-no-use.php` | `use_declaration` (in class body), `anonymous_function_use_clause` | not namespace imports | Negative control (distinct node types) |
| Declaring own class, `new \Foo\Bar()`, `Bar::class` | same | no import nodes | no name exposed | Negative |

All PHP samples parse clean.

### Semantic argument

- `use` is a compile-time name-resolution aid: "Importing is performed at
  compile-time, and so does not affect dynamic class, function or constant
  names", the `use` keyword is limited to the outermost scope, and the rules
  are "per file". Nothing a `use` creates is visible to another file.
  Verdict: **no applicable construct** (local naming only). This is the
  separation the spec asks for between local naming and outward alias.
- PHP has no module interface: every top-level class/function/const a file
  declares is globally reachable (by qualified name) once the file runs.
  Therefore "exposed through the module's interface" collapses to "defined
  at top level", which the spec already treats as a local export.

**`include`/`require` (dubious).**
For: a barrel file `require_once 'a.php'; require_once 'b.php';` makes the
definitions of `a.php` and `b.php` available to anyone who loads the barrel,
and the operand is often a static string. Against: it executes arbitrary code
(spec: "Do not ... execute source code"), the includer's scope receives the
variables too, the names forwarded are not enumerable from this file, the path
is frequently dynamic (`$path`, `__DIR__ . ...`), the construct may be
conditional, and the spec bars inference from include statements. Even the
included file's *return value* (`return require 'x.php';`, the common
config-array idiom) is a value, not a name.
Recommendation: **not a re-export**; classify as *no applicable construct*
and report the sourcing as an import-like load if an import record is ever
needed. If the author later wants it, the only honest model is a
whole-module/wildcard entry marked "executes file; static path only; limit:
dynamic".

**`class_alias` (dubious).**
For: it is the PHP idiom for backwards-compatible renames
(`class_alias('New\Name', 'Old\Name')` in a shim file). The alias is the same
class, globally resolvable, and both arguments are usually string literals,
so exported name, original name, and (via namespace) source are static.
Against: it is a runtime function call, not syntax; it only takes effect if
the file executes and the call is reached (can sit in `if`/function); the
second argument can be a variable or concatenation; it creates a global name,
not an entry in a *this file's* interface; the first argument might be
produced by `::class` of an imported alias, requiring `use` resolution; and
the third argument (`autoload`) changes failure behavior.
Recommendation: **out of the default contract** (no applicable construct,
documented as an unsupported dynamic pattern). If the author selects spec
option 2 broadly, admit it only for the literal/`::class` two-argument form
as *static convention* with a mandatory limit "runtime alias; takes effect
only if executed".

### Expected result if `class_alias` is admitted (literal form)

| Field | Value for `class_alias('Foo\Bar', 'Baz');` at file top level |
|---|---|
| exported name | `Baz` (global; case-folded to lowercase at run time per manual) |
| original / member path | `Bar` |
| source module path | `Foo` (namespace as written; no file path) |
| enclosing scope | file; a call inside a function or `if` is conditional |
| alias | the exported name itself |
| form | named |
| conditional | true if not a direct top-level statement |
| coverage limit | non-literal argument -> limited ("dynamic alias"); `::class` of a `use`-aliased name -> needs same-file `use` binding |

### Grammar gaps

None found for the audited constructs. Caveat: `use function` / `use const`
kind lives on an anon-token `type:` field, so a query that matches only named
children will treat all three kinds identically. php 0.25.1 exists on
crates.io; not tried.

---

## Bash

Grammar: tree-sitter-bash 0.25.1 (Bash 5.x manual; manual page does not
state a version). Sources:
<https://www.gnu.org/software/bash/manual/html_node/Bourne-Shell-Builtins.html>,
<https://www.gnu.org/software/bash/manual/html_node/Shell-Functions.html>.

### Constructs

| Construct | Sample | Nodes / fields | Semantics | Class. |
|---|---|---|---|---|
| `source file` / `. file` | `bash/pos-source.sh` | plain `command(name: command_name > word "source" or ".", argument: word / string / simple_expansion / process_substitution)` | Reads and executes commands "in the current shell context"; searches `PATH`; equivalent spelling | Dubious, see below |
| conditional forms `[ -f x ] && source x`, `if ...; then . x; fi` | same | `list`, `if_statement` wrapping the same `command` | same, runs only if branch taken | Dubious + conditional |
| Dynamic sourcing: `source $f`, `source <(cmd)`, loop `. "$f"`, `builtin source x`, `eval "source x"` | `bash/amb-dynamic-source.sh` | `simple_expansion`, `process_substitution`, `for_statement > do_group`, `builtin` as command name with `word "source"` argument, `eval` string | path unknowable | Unsupported dynamic |
| Function definition `foo() {}`, `function bar {}` | `bash/pos-export.sh` | `function_definition(name: word, body: compound_statement)` | defines a global function when executed | Local definition |
| `export VAR=1`, `export PATH` | same | `declaration_command` with anon `export`, then `variable_assignment(name:, value:)` or `variable_name` | marks variables for the environment of subsequently executed commands | Outside (environment) |
| `export -f foo`, `declare -fx bar` | same | `declaration_command`, flag as bare `word "-f"`/`"-fx"`, target `variable_name` (wrongly named: it is a function name) | `-f`: names refer to functions; manual: "Functions may be exported so that child shell processes automatically have them defined" | Outside (environment) |
| `declare -x`, `readonly`, `alias` | same | `declaration_command` (anon `declare`/`readonly`); `alias` is an ordinary `command` | not module-level | Outside |
| `echo source`, `x=source`, `cat/sh/bash ./lib.sh` | `bash/neg-plain.sh` | `command` with other names, `variable_assignment` | executing a script in a child shell does not load definitions | Negative controls |

All Bash samples parse clean.

### Semantic argument

Bash has no module system. Three distinct things must not be confused:

1. **Loading definitions** (`source`): the sourced file's functions and
   variables land in the current shell. This is an import with a side
   effect, not a forward.
2. **Environment propagation** (`export`, `export -f`): data/functions
   handed to child processes. The name stays defined in this shell regardless
   of `export`; `export` adds a *second audience* (children). Exported
   functions are not "obtained elsewhere": `export -f foo` exports a local
   function, and `export VAR` exports a local variable. Verdict: outside the
   definition.
3. **Module re-export**: no syntax. A "barrel" library script that sources
   others is the only analogue.

**Sourcing as a barrel (dubious).**
For: `lib/all.sh` containing only `source ./a.sh; source ./b.sh` does expose
a/b's functions to anything that sources `all.sh`, which is exactly how
shell libraries compose. The path is a literal in the common case.
Against: the sourced names are not visible from this file (wildcard with no
enumeration), execution has side effects, `source` is a builtin looked up
at run time (can be shadowed by a function or alias, can be passed `$f`),
the file may be conditional, and arguments may follow (positional
parameters). The spec bars inference from include statements, and there is no
signal distinguishing a barrel from a script that sources a helper for its
own use.
Recommendation: classify as *no applicable construct* for re-export; count
`source` as an unresolved import only. If the author wants barrels, model as
a wildcard/whole-module entry, evidence "static convention", limit
"dynamic or conditional path". Do not exempt anything from lints on this
basis.

### Expected result if sourcing is admitted

| Field | Value for `source ./lib.sh` at top level |
|---|---|
| exported name | none (whole-module / wildcard-like) |
| source module path | `./lib.sh` (verbatim; not resolved) |
| enclosing scope | file; function body or `if` -> conditional |
| alias | none |
| form | namespace/module |
| conditional | true under `list` / `if_statement` / function |
| coverage limit | non-literal argument, process substitution, loop, `builtin`/`eval` wrapper -> limited (dynamic) |

### Grammar gaps

None for audited constructs. Quirk: in `export -f foo` and `declare -fx bar`
the function name is a `variable_name` node, so the grammar cannot tell a
function export from a variable export without reading the flag word.

---

## Zsh

Grammar: tree-sitter-zsh 0.56.0 (separate grammar; probed independently).
Source: <https://zsh.sourceforge.io/Doc/Release/Shell-Builtin-Commands.html>
and <https://zsh.sourceforge.io/Doc/Release/Functions.html> (manual pages do
not state a zsh version). Part of the builtins page past character 100000
(including `zmodload`) was not read: **source not verified** for `zmodload`.

### Constructs

| Construct | Sample | Nodes / fields | Semantics | Class. |
|---|---|---|---|---|
| `source file`, `. file` | `zsh/pos-source.zsh` | same `command` shape as Bash, but string interpolation is `variable_ref > simple_variable_name` (Bash: `simple_expansion > variable_name`) | `.`: reads commands "in the current shell environment", searches `$path`; `source`: "same as `.`, except that the current directory is always searched first" | Dubious (as Bash) |
| dynamic sourcing | `zsh/amb-dynamic-source.zsh` | `variable_ref`, `process_substitution`, `for_statement` with `glob_pattern` values, `builtin`/`eval` | unknowable path | Unsupported dynamic |
| `autoload -Uz fn ...`, `autoload -U a b` | `zsh/pos-autoload.zsh` | ordinary `command(name: command_name > word "autoload", argument: word...)`; flags and names are indistinguishable `word`s | Marks `fn` as an undefined function whose body is searched in `fpath` on first call; `-U` suppresses alias expansion, `-z`/`-k` select style, `-w` takes compiled files, `+X` loads without executing | Not a forward, see below |
| `autoload -w`, `+X`, `zmodload` | `zsh/pos-autoload-variants.zsh` | `command` | `-w`: names are compiled files | as above |
| `fpath=(~/.zfunc $fpath)` | `zsh/pos-autoload.zsh` | `variable_assignment(name: variable_name, value: array)` | extends the autoload search path | Outside |
| function definition `foo() {}`, `function bar {}` | same | `function_definition(name: word, body: compound_statement)`; anon `function` token for second form | local definition | Local |
| `export VAR=1`, `typeset -x Z=3`, `typeset -fx foo` | same | `declaration_command` with anon `export`/`typeset`, **flags as `argument: word` fields** (Bash has them as unfielded children) and `variable_assignment` with an explicit `=` token | `export`: "marked for automatic export to the environment of subsequently executed commands", equivalent to `typeset -gx`; `-f` makes names refer to functions | Outside (environment) |
| `export -f foo` | same | parses clean as `declaration_command(export, argument: -f, argument: foo)` | The fetched zsh pages do not document function export; **source not verified** that `export -f` is meaningless in zsh. Parsing it clean is not evidence it works | Unclear, parse gap-free |
| `functions -c old new` | `zsh/pos-autoload-variants.zsh` | `command` | copies a function; not studied further | Outside |
| plain echo/assign/`zsh ./x` | `zsh/neg-plain.zsh` | `command` / `variable_assignment` | child process; no load | Negative controls |

All Zsh samples parse clean.

### Semantic argument

- `source` and `export`: same analysis as Bash, with the zsh-specific facts
  above (current-directory-first for `source`; `export` = `typeset -gx`).
  The Zsh Functions page says functions "execute in the same process as the
  caller", and says nothing about exporting functions to children.
- **`autoload` (dubious, zsh-specific).** For: it *names* a function
  that will be provided from elsewhere (an `fpath` file), making the name
  callable from this shell, and it is dedicated syntax. A completion
  or plugin file commonly consists of `autoload -Uz fn1 fn2`. Against: it is
  a lazy-load declaration of a dependency, equivalent to a forward declaration
  in C, not an exposure through *this file's* interface: the source file
  is not named (resolution is by `fpath`, a runtime variable), and the name
  becomes defined only in the current shell. The names are statically
  listed, which makes it superficially attractive, and flags (`-U`, `-z`,
  `+X`, `-X`) are unfielded `word`s.
  Recommendation: **not a re-export**. Record as unresolved import-like
  evidence if ever needed. Source of truth for the author: autoload provides
  a name without saying where it comes from.

Verdict for Zsh: *no applicable construct*, same as Bash, with `autoload` as
the principal near-miss.

### Where Bash and Zsh differ for the extractor

| Aspect | Bash 0.25.1 | Zsh 0.56.0 |
|---|---|---|
| variable reference | `simple_expansion > variable_name` | `variable_ref > simple_variable_name` |
| declaration flags/targets | unfielded `word` / `variable_name` children | `argument:` fields |
| keyword token | anon `export`/`declare`/`readonly` | anon `export`/`typeset` |
| `for ... in` values | `word` | `glob_pattern` for `*.sh` |
| `=` in assignments | not a child | explicit `=` token |

Separate query sets are required; none of the Bash node names can be reused
for variable-reference handling.

### Grammar gaps

None for audited constructs. No fixture proves absence of function export in
zsh (manual silent).

---

## C

Grammar: tree-sitter-c 0.24.2. Standard: C17 and C23 (C23 for
`__has_include`). Source: <https://en.cppreference.com/w/c/preprocessor/include>
(cites ISO 9899:2018 6.10.2 and ISO 9899:2024 6.10.1/6.10.2). No C
standard text was read directly; cppreference is a secondary source.

### Constructs

| Construct | Sample | Nodes / fields | Semantics | Class. |
|---|---|---|---|---|
| `#include <stdio.h>` | `c/pos-include.h` | `preproc_include(path: system_lib_string)` | textual replacement of the directive "by the entire contents" of the file | Dubious, see below |
| `#include "other.h"` | same | `path: string_literal > string_content` | same | Dubious |
| `#include MACRO_HDR` | same | `path: identifier` | macro-expanded include, target unknowable without preprocessing | Unsupported dynamic |
| include guard `#ifndef X / #define X / ... / #endif` | same | the whole file body is nested under `preproc_ifdef(name:)` (covers `#ifndef` and `#ifdef`; distinguished only by anon token) with `preproc_def(name: identifier)` | prevents repeated inclusion | Wrapper only; extractor must descend |
| conditional includes `#ifdef/#else/#endif`, `#if defined(X) && ...` | `c/pos-conditional.h` | `preproc_ifdef(... alternative: preproc_else)`, `preproc_if(condition: binary_expression with preproc_defined)` | includes happen under build config | Conditional |
| `#if __has_include(<x.h>)` | same | **ERROR**: `identifier "__has_include"` then `ERROR "("`; the following `#endif` degrades to `preproc_call` | C23 feature | **Grammar gap** (damage extends to the `#endif`) |
| `#define PUBLIC_FN other_fn` (object-like alias macro) | `c/pos-define-alias.h` | `preproc_def(name: identifier, value: preproc_arg "other_fn")` | text substitution; value is an opaque string | Convention? see below |
| function-like macro `#define CALL(x) other_call(x)` | same | `preproc_function_def(parameters: preproc_params, value: preproc_arg)` | text substitution | Outside |
| `typedef other_t my_t;`, `typedef struct other_s my_s;` | same | `type_definition(type: type_identifier / struct_specifier(name:), declarator: type_identifier)` | defines a new name for an existing type | Convention? see below |
| `extern int shared;`, `extern int f(void);` | same | `declaration > storage_class_specifier "extern"`, `declarator: identifier / function_declarator` | declares an entity with external linkage defined elsewhere | Outside |
| `extern "C" int x;` | same | **parses clean** as `linkage_specification(value: string_literal, body:)` | this is C++, yet the C grammar accepts it | Surprise (see below) |
| ordinary `.c` file with include and local definitions | `c/neg-decl.c` | `preproc_include`, `function_definition`, `declaration` | nothing exposed | Negative control |

All samples parse clean except `c/pos-conditional.h` (the `__has_include` line).

### Semantic argument

C has no module, export list, or visibility keyword at file level, other
than `static` (internal linkage). The only "interface" is a header file, and
that is a filename convention.

- **`#include` (dubious).** For: a header that includes another makes every
  declaration of the latter part of what includers see, and umbrella headers
  (`#include "a.h"` / `"b.h"`) are the C analogue of an ES barrel file.
  Against: inclusion is *textual substitution*; the including file's interface
  is whatever the preprocessor produces; nothing marks an include as
  intentional exposure rather than a private dependency (a header may include
  `<stddef.h>` just to use `size_t`); the spec bars concluding anything from
  include statements or filenames; the target is not enumerable; guards and
  conditionals make the included set configuration-dependent.
  Recommendation: **not a re-export** (*no applicable construct*); report an
  include as an import and leave public-header inference out of scope, as
  the spec states.
- **`#define` alias macro (dubious).** For: `#define PUBLIC_FN other_fn` is
  an explicit rename visible to includers. Against: macros have no scope,
  the value is an opaque `preproc_arg` string (could be anything,
  including multiple tokens or a literal), and evaluating them is a
  preprocessor task the spec excludes. Recommendation: **not a re-export**;
  at most, report with limit "preprocessor alias, unevaluated".
- **`typedef other_t my_t;` (dubious).** For: closest to Go's `type X = pkg.X`
  candidate; it is declaration-level and syntactically a renaming of a
  named type. Against: in C there is no export notion, so it cannot be
  said that `my_t` is exposed rather than merely declared; the RHS type may
  not come from an include at all (the audit cannot tell without
  preprocessing); a typedef is not a forward of a *name* (it declares a new
  name). Recommendation: **not a re-export**. If the author wants alias
  declarations everywhere, the form "named alias" with source unknown is
  the most that can be said.
- **`extern`**: a declaration of linkage, not an exposure of a module's
  contents; the defining module is not named.

### Expected result if aliases were admitted

| Field | `typedef other_t my_t;` | `#define PUBLIC_FN other_fn` |
|---|---|---|
| exported name | `my_t` | `PUBLIC_FN` |
| original name | `other_t` | `other_fn` (text) |
| source module path | unknown | unknown |
| enclosing scope | file (inside `preproc_ifdef` if guarded -> conditional flag) | same |
| alias | `my_t` | `PUBLIC_FN` |
| form | named | named |
| conditional | true under `preproc_if*` | true under `preproc_if*` |
| coverage limit | always "source unknown without preprocessing" | always "preprocessor alias, unevaluated" |

### Grammar gaps and surprises

1. `__has_include` (C23) is a parse error and the error breaks conditional
   balance (`#endif` becomes `preproc_call`), so everything after it may
   belong to the wrong branch. Treat as: damaged forwarding syntax must not
   yield a complete answer (spec acceptance).
2. The C grammar accepts `extern "C"` (C++ only). A fixture parsing clean
   proves nothing about C.
3. `#ifndef` and `#ifdef` share `preproc_ifdef`; direction is an anon token.
4. Macro-built include paths (`#include MACRO_HDR`) are `identifier`, with no
   way to know the target.

---

## C++

Grammar: tree-sitter-cpp 0.23.4 (newest on crates.io). Standards: C++11
(inline namespaces, alias declaration), C++17 (multi-declarator using),
C++20 (modules, `using enum`). Sources:
<https://en.cppreference.com/w/cpp/language/using_declaration>,
<https://en.cppreference.com/w/cpp/language/namespace>,
<https://en.cppreference.com/w/cpp/language/modules>,
<https://en.cppreference.com/w/cpp/preprocessor/include>,
<https://eel.is/c++draft/module.interface> and
<https://eel.is/c++draft/basic.lookup.qual> (working-draft mirror, commit
`c7015b4` / `c7015b48`, edition not stated). cppreference is secondary; the
draft pages are primary but of unknown edition.

### Namespace-scope constructs (pre-modules)

| Construct | Sample | Nodes / fields | Semantics | Class. |
|---|---|---|---|---|
| `using ns::x;` in a namespace | `cpp/pos-using-decl.hpp` | `using_declaration > qualified_identifier(scope: namespace_identifier, name: identifier)` | "introduce a member of another namespace into the current namespace"; later declarations of the same name in `ns` are not seen | **Dedicated syntax** |
| `using ::global_fn;` | same | `qualified_identifier` with no `scope`, only `::` token | same, from global namespace | Dedicated syntax |
| `using ns::detail::deep;` | same | `name:` nests a second `qualified_identifier` (path is right-nested) | same | Dedicated syntax |
| `using ns::a, ns::b;` (C++17 multi-declarator) | same | **ERROR** inside the first `qualified_identifier`; second path survives | equivalent to a sequence of using-declarations | **Grammar gap** |
| `using enum ns::Color;` (C++20) | same | `using_declaration` with anon `enum` token, `qualified_identifier` | introduces all enumerators | Dedicated syntax; parses clean |
| `using typename B::U;` | same (class template), `cpp/pos-class-using.hpp` | **ERROR**: `scope: namespace_identifier "typename"`, `ERROR(identifier B)`, `:: U` | resolves dependent name | **Grammar gap** |
| `using namespace ns;` inside `namespace outer` | `cpp/pos-using-directive.hpp` | **same node type** `using_declaration` as above, but with anon `namespace` child and a bare `identifier "ns"` or `qualified_identifier` | "does not add any names to the declarative region"; for qualified lookup `outer::x` falls back to nominated namespaces only "if nothing is found" in `outer` (draft [namespace.qual]); transitive for unqualified lookup | **Dedicated syntax**, wildcard form (see below) |
| `using namespace std;` at file scope | same | `using_declaration` at top level | leaks into every includer of the header | Dedicated syntax, global scope, wildcard |
| `namespace alias = ns::long_name;` | same | `namespace_alias_definition(name:, nested_namespace_specifier)` | synonym for a namespace, not a new namespace | Convention (renamed namespace) |
| `inline namespace v1 { }` | same | `namespace_definition` with anon `inline` | members treated as members of the enclosing namespace | Outside (local) |
| `using T = ns::T;` | `cpp/pos-alias.hpp` | `alias_declaration(name: type_identifier, type: type_descriptor > qualified_identifier)` | new name for a type; templated alias under `template_declaration` | Convention (alias) |
| `typedef ns::T T2;`, `typedef ns::T *Tp;` | same | `type_definition(type: qualified_identifier, declarator: type_identifier / pointer_declarator)` | same | Convention (alias) |
| `#include <vector>`, `#include "other.hpp"`, `#pragma once` | `cpp/pos-include.hpp` | `preproc_include(path:)`, `preproc_call` | textual substitution (C++ cppreference page identical in wording to C) | Dubious, as C |
| `extern "C" { #include "c_api.h" }` | `cpp/pos-extern-c-block.hpp` | `linkage_specification(value:, body: declaration_list > preproc_include)` | linkage wrapper around an include | Dubious, as include |
| Plain namespace + `n::x` use | `cpp/neg-plain.cpp` | no `using_declaration` | no exposure | Negative control |
| `using ns::detail::{a, b};` (not C++) | `cpp/neg-using-grouped-invalid.hpp` | ERROR; this form is not valid C++ | C++ has no grouped using | Negative control (invalid) |

### Class-member constructs

| Construct | Sample | Nodes / fields | Semantics | Class. |
|---|---|---|---|---|
| `using B::x;` in a class body | `cpp/pos-class-using.hpp` | `using_declaration` under `field_declaration_list`, `scope: namespace_identifier "B"` | "introduces a member of a base class into the derived class", can raise access; overloads all brought in | Outside: member exposure, not a module export |
| `using B::B;` (inheriting constructors) | same | `qualified_identifier(scope: B, name: B)`, no special node | inherits constructors | Outside |
| `using Base = B;`, `typedef int I;` inside class | same | `alias_declaration`, `type_definition` under `field_declaration_list` | member alias | Outside |
| `private:` + `using B::y;` | same | `access_specifier` precedes the declaration | access applies | Outside, visibility is in preceding sibling |

Class-scope `using` is indistinguishable from namespace-scope `using` by node
type; only the parent (`field_declaration_list` vs `declaration_list`) differs.

### C++20 module constructs (grammar gap)

| Construct | Sample | Observed parse (cpp 0.23.4) | Semantics | Class. |
|---|---|---|---|---|
| `export module m;` | `cpp/pos-export-import.cppm` | `declaration(type: type_identifier "export", declarator: identifier "module", ERROR(identifier m))` | primary module interface unit | Gap |
| `export import other;` | same | `declaration(type: "export", declarator: identifier "import", ERROR(...))` | "if module B export-imports A, then importing B will also make visible all exports from A" | **Dedicated syntax, gap** |
| `export import :part;` | same | same shape, `ERROR(identifier part)` | re-exposes an interface partition; only the primary interface unit may do this | **Dedicated syntax, gap** |
| `export import <vector>;` | same | `declaration(type: "export", declarator: template_function(name: "import", arguments: template_argument_list))` | re-exports a header unit | **Dedicated syntax, gap** |
| `import hidden;`, `import :priv;` | same | **parses clean**: `declaration(type: type_identifier "import", declarator: identifier "hidden")` and `labeled_statement(label: "import")` | non-exported import | Gap, **silent misparse** |
| `module m;` (implementation unit), `module;` | `cpp/pos-impl-unit.cpp`, `cpp/pos-module-decls.cppm` | **parses clean** as `declaration(type_identifier "module", ...)` / `expression_statement(identifier "module")` | module declarations | Gap, **silent misparse** |
| `module : private;` | same | `labeled_statement(label: statement_identifier "module")` | private module fragment | Gap, silent |
| `export { using other::f; int g(); }` | `cpp/pos-export-block.cppm` | `compound_literal_expression(type: "export", value: initializer_list ...)` with `MISSING "}"` and trailing `ERROR "}"` | export-declaration group; `using` inside is an exported using-declaration | Dedicated syntax, gap |
| `export using other::h;` | same | `declaration(type "export", ERROR(identifier using), declarator: qualified_identifier)` | draft: each entity named "shall either be a type alias or have been introduced with a name having external linkage" | Dedicated syntax, gap |
| `export namespace ns3 { using other::i; }` | same | `function_definition(type "export", declarator: identifier "namespace", ERROR(ns3), body: compound_statement > using_declaration)` | exported namespace with forwarding inside | Dedicated syntax, gap |
| `export int local_fn();`, `export class K {};`, `export template<...> ...` | same | misparsed with ERROR nodes | own exports, not forwarding | Local export, gap |

None of the C++20 module files in `cpp/` parses without error except where a
misparse is silent. The grammar has no module nodes at all:
`src/grammar.json` for 0.23.4 contains no occurrence of `module`.

### Semantic argument

**Namespace-level `using`-declaration.** Names introduced into a
namespace are found by qualified lookup (`outer::x`), so this is exposure of
an entity obtained elsewhere through the namespace's interface: dedicated
syntax, named form. The unit of "module" is the namespace (the file is a
header); C++ namespaces are not tied to files, so enclosing scope must be the
namespace path, and the exposure applies only to files that include this
header. Limits: the originating namespace's file is not named
(module path = `ns` as written); a using-declaration of overloaded functions
brings in all visible overloads at that point; textual-header
conditionality (`#ifdef` around it) is common.

**Using-directive.** `using namespace ns;` inside `namespace outer` adds no
names to `outer`, but draft [namespace.qual] makes `outer::x` fall back to
`ns` when `outer` lacks `x`, and cppreference notes transitivity for
unqualified lookup. It therefore exposes `ns` in a *lookup-fallback* way,
without a stable name list. Recommend: admit as wildcard-namespace form with
mandatory limit "lookup-only; later extension of ns is visible; no names
introduced; shadowed by same-named members of outer". At file scope
(`using namespace std;`) it leaks into every includer; report it as global
scope. It must not be reported as named forwarding.

**Type aliases (`using T = ns::T`, `typedef`).** Not using-declarations; they
declare a new name for a type (templated alias for a template). Comparable to
Go `type X = pkg.X`. Recommendation: author decision. If aliases are in
scope, admit only when the RHS is a plain qualified type name (no pointer,
cv-qualifier, template arguments, or function types), as *convention*, form
named alias, with `qualified_identifier` as the source path. Otherwise the
RHS shape is a type expression, not a forward.

**Class-member `using Base::x;`** exposes a base-class member under the
derived class. Spec: member exposure through a containing type must be
distinguished from a file's module interface. Verdict: **outside the
definition** as a module export; if the author wants Scala-style "members
through object", this is the C++ analogue and would need a scope field
holding the class name and an explicit "member" form. Recommendation: out
of v1.

**`#include`.** Same argument as C. In C++20 the standard allows an
implementation to replace `#include` of an importable header with an
`import` (cppreference "Module import"), which makes include semantics
less predictable. Recommendation: no applicable construct (import only).

**C++20 `export import`.** Defined in cppreference ("importing B will also
make visible all exports from A") and by the draft: an export-declaration
may be `export` module-import-declaration, with "the declarative effects of
its module-import-declaration". This is the textbook re-export:
dedicated syntax, module/wildcard form (re-exports the whole export set of
the module, partition or header unit), not enumerable. `export using` is
dedicated syntax, named form. `export { ... }` and `export namespace` are
containers whose children must be examined. The draft's example marks
`export using namespace N;` as OK although no rule text was found for
using-directives in an export-declaration (the page says "no explicit
rule"): the author should treat it as **source not verified** and I would
report it as a wildcard form with limit.

### Expected results (once parsable)

| Source form | exported name | original / path | source module | scope | alias | form | conditional | limit |
|---|---|---|---|---|---|---|---|---|
| `using ns::x;` in `namespace outer` | `x` | `x` under `ns` | `ns` (namespace, not file) | `outer` | none | named | if under `#if*` | overloads not enumerated; textual header only |
| `using ns::a, ns::b;` | `a`, `b` | same | `ns` | enclosing namespace | none | named, two entries | as above | currently gap |
| `using namespace ns;` in `outer` | none | none | `ns` | `outer` | none | wildcard-namespace | as above | lookup-only fallback, no name list |
| `using namespace std;` at file scope | none | none | `std` | global | none | wildcard-namespace | as above | leaks to includers |
| `using T = ns::T;` | `T` | `T` under `ns` | `ns` | enclosing namespace | `T` | named alias (if admitted) | as above | RHS must be plain qualified name |
| `export import other;` | none | none | module `other` | module `m` | none | module (re-exports full set) | no | gap |
| `export import :part;` | none | none | partition `:part` of this module | `m` | none | module | no | gap; partition is internal source |
| `export import <vector>;` | none | none | header unit `vector` | `m` | none | module | no | gap |
| `export using other::h;` | `h` | `h` under `other` | `other` (namespace; module unknown) | `m` | none | named | no | gap; entity must be alias or externally linked |
| `export { using other::f; }` | `f` | as above | as above | `m` | none | named | no | gap; export block is container only |
| class `using B::x;` | `x` | `B::x` | `B` (class) | class `D` | none | member | no | outside definition if v1 stays file-level |

### Grammar gaps and surprises

1. **No C++20 module support in the pinned grammar** (and none in the latest
   crates.io release). Fix would be a grammar change; spec forbids a grammar
   upgrade unless the author adds it. This is a **blocker for the C++20
   module part** of the goal; record it for the author.
2. **Silent misparse**: `import x;`, `module m;`, `module;` all parse with no
   error. `has_error` therefore cannot be used as a signal that a module file
   was understood. A C++ extractor that answered "no re-exports" on a
   `.cppm` file would be wrong, with a clean-looking tree.
3. `using typename` and C++17 multi-declarator `using a, b;` are parse errors.
4. `using_declaration` is used both for declarations and directives;
   only the anon `namespace` token distinguishes them.

---

## C#

Dialects: C# 10 (global using, file-scoped namespaces), C# 12 (aliases for any
type). Sources:
<https://learn.microsoft.com/en-us/dotnet/csharp/language-reference/keywords/using-directive>
(page dated 2026-01-22),
<https://learn.microsoft.com/en-us/dotnet/api/system.runtime.compilerservices.typeforwardedtoattribute>,
<https://learn.microsoft.com/en-us/dotnet/csharp/language-reference/language-specification/namespaces>
(C# standard draft; the fetch of the file-scoped proposal page was redirected
here; the section on file-scoped namespaces was seen as a grammar entry only).
The version number of the file-scoped namespace feature (C# 10) and C# 10's
`global using` come from the using-directive page's cross-reference to the
.NET 6 SDK; **source not verified** for the exact language version of
file-scoped namespaces.

### Constructs

| Construct | Sample | Nodes / fields | Semantics | Class. |
|---|---|---|---|---|
| `using System;`, `using System.Collections.Generic;` | `cs/pos-using.cs` | `using_directive > identifier / qualified_name(qualifier:, name:)` | "The scope of a using directive without the global modifier is the file in which it appears" | Outside |
| `using Alias = N.T<int>;` | same | `using_directive(name: identifier, <qualified_name>)`, `generic_name > type_argument_list` | file-scoped alias; no open generics (`List<>` illegal) | Outside |
| `using static System.Math;` | same | anon `static` token | imports static members and nested types; scope = file | Outside |
| `global using System.Linq;` | same | anon `global` before `using`, nothing else differs | "has the same effect as adding the same using directive to every source file in your project" | Outside, wider than file |
| `global using static ...;`, `global using G = ...;` | same | `global`+`static` / `global`+`name:` | compilation-wide | Outside, wider than file |
| C# 12 aliases `using Point = (int X, int Y);`, `int?`, `int[]`, `using unsafe Ptr = int*;`, `global::System.Text` | `cs/pos-using-alias-any.cs`, `cs/pos-using.cs` | `tuple_type`, `nullable_type`, `array_type`, `pointer_type` with anon `unsafe`; `alias_qualified_name` | alias to formerly restricted types | Outside; parses clean |
| using inside a block-scoped namespace | `cs/pos-using-in-namespace.cs` | `namespace_declaration > declaration_list > using_directive` | same file scope | Outside |
| file-scoped namespace `namespace App.Lib;` | `cs/pos-file-scoped.cs`, `cs/pos-using-after-file-scoped.cs` | `file_scoped_namespace_declaration(name: qualified_name)`; siblings of it (usings, classes) are flat **children of `compilation_unit`**, not nested | declaration scope | Scope only |
| `[assembly: TypeForwardedTo(typeof(T))]` | `cs/pos-typeforward.cs` | `global_attribute > attribute(name:, attribute_argument_list > attribute_argument > typeof_expression(type:))`; target `assembly` is an anon token before `:` | "Use the attribute to move a type from one assembly to another without disrupting callers"; attribute usage: assembly only, `AllowMultiple=true` | Dubious, see below |
| `typeof(Generic<>)` | same | `generic_name > type_argument_list "<>"` | open generic | same |
| attribute with qualified name `System.Runtime.CompilerServices.TypeForwardedTo` | same | `name: qualified_name` | same | same |
| `[assembly: InternalsVisibleTo("Friend")]` | same | `global_attribute` with `string_literal` | friend assembly | Negative (not a forward) |
| `using (var f = ...) {}` and `using var t = ...;` | `cs/neg-using-stmt.cs`, `cs/neg-plain.cs` | `using_statement` / `local_declaration_statement`, not `using_directive` | IDisposable | Negative control |
| fully qualified use, `class D : System.Collections.Generic.List<int>` | `cs/neg-plain.cs` | no directives | no exposure | Negative |

All C# samples parse clean.

### Semantic argument

- `using`, `using static`, `using Alias =` are consumption-side. Each
  affects only the file that holds it ("The scope of a `using` directive
  without the `global` modifier is the file"). Nothing is visible to other
  files or other assemblies. Verdict: **no applicable construct** (not
  re-exports).
- `global using` widens the scope to "all files in the compilation". It still
  does not alter the assembly's public surface: other assemblies
  never see it. It is therefore a project-wide *import*, not a re-export.
  Nearest analogy: a prelude. Its effect on lints is relevant: a `global
  using` can never be "unused" in a single file, and a per-file
  `unused-import` check would flag it falsely. That is a diagnostics
  consideration, not a re-export, and I did not verify the repo's diagnostic
  handling for C#.
- Nothing in C# source exposes another type under a new name from an
  assembly's public API. Public members whose *signatures* mention foreign
  types expose a dependency but not a name.

**`TypeForwardedTo` (dubious).**
For: the only dedicated, explicit C# syntax by which an assembly states that
a type is reachable *through it* but defined elsewhere. The attribute is
checked by the compiler, has one fixed meaning, and the argument `typeof(T)`
is a type name, which gives the original name and namespace. Against:
(1) the forwarding assembly is not named in the file: the attribute is
assembly-level metadata applied during compilation, so "source module"
cannot be determined from the file; (2) the destination type's namespace may
be unqualified in source (`typeof(T)` with a `using` or `global using` in
another file), so the full name cannot be established from one file;
(3) the attribute is resolved by name only, and a file can define its own
`TypeForwardedTo`; (4) the meaning is binary compatibility for already
compiled callers, not API design; (5) the spec asks to flag constructs
"whose meaning depends on compilation or assembly metadata".
Recommendation: admit as *dedicated syntax* with a mandatory limit
("assembly forward; source assembly and full name unresolved from one file"),
or exclude the whole family as assembly-metadata. I lean to admitting it
with the limit, since it is the only C# construct that matches the
definition and it is explicit; but this is a call for the author about
whether assembly-level facts belong in a symbol tool. `InternalsVisibleTo`
must not be admitted.

### Expected result (if `TypeForwardedTo` is admitted)

| Field | `[assembly: TypeForwardedTo(typeof(Other.Lib.T))]` |
|---|---|
| exported name | `T` (name and namespace unchanged by forwarding) |
| original name / member path | `Other.Lib.T` as written |
| source module path | unknown (the assembly that now defines `T` is in metadata, not source) |
| enclosing scope | assembly (compilation unit level) |
| alias | none |
| form | named |
| conditional | true if wrapped in `#if` (not tested; **source not verified** for preprocessing nodes in this grammar) |
| coverage limit | `typeof(X)` with unqualified `X` -> limited (needs using resolution); generic arity `List<>` preserved; match by attribute name only (shadowable) |

### Grammar gaps

None found. Notes: anon tokens (`global`, `static`, `unsafe`, `assembly`)
carry the distinction between local, static, and global usings and between
assembly and ordinary attributes. In file-scoped namespace files the
`using_directive`s and types are *siblings* of
`file_scoped_namespace_declaration`; enclosing scope must be derived by
position, unlike the block-scoped form.

---

## Open questions / grammar gaps (consolidated)

Grammar gaps and fidelity problems:

1. **C++20 modules are unsupported by tree-sitter-cpp 0.23.4**, which is
   also the newest crates.io release. `export module`, `export import`
   (named, partition, header unit), `export { }`, `export using`, `export
   namespace` all produce ERROR/MISSING or wrong nodes. The author has to
   choose: accept as a documented blocker (spec: "record the blocker for the
   author"), add a grammar change to scope, or accept a text-pattern fallback
   (which would conflict with the spec's insistence on node evidence).
2. **C++ modules misparse silently**: `import x;`, `module m;`, `module;`
   parse with `has_error=false` as plain declarations. Parse cleanliness
   cannot gate a "computed and empty" answer for C++.
3. C++: `using typename X::Y;` and C++17 `using a, b;` are parse errors.
4. C: `__has_include` (C23) is a parse error that corrupts `#if/#endif`
   nesting for the rest of the file. The C grammar also accepts `extern "C"`.
5. PHP: newer grammar 0.25.1 and Zsh: newer 0.63.4 exist on crates.io; the
   effect on the audit was not tested (no upgrades attempted).
6. Anon tokens carry meaning in Java (`static`, `open`), PHP (`function`,
   `const` on `type:`), C++ (`namespace` distinguishing directive from
   declaration, `enum`, `inline`), and C# (`global`, `static`, `unsafe`). Any
   query that matches only named children loses these distinctions.
7. Bash and Zsh differ structurally (see table); they need separate queries.

Decisions for the author:

1. Does Java `requires transitive` belong in this symbol feature
   (recommendation: no; at most a separate module-dependency kind)?
2. Are PHP `class_alias`, shell `source`, and C/C++ `#include` counted at
   all (recommendation: no, as unsupported dynamic or out of scope per the
   include-statement exclusion)?
3. Do declaration-level aliases (C++ `using T = ns::T`, `typedef`, C
   `typedef`) count as forwarding? Defined together with the Go `type X = pkg.X`
   decision elsewhere in the audit.
4. C++ using-directives (`using namespace ns;` inside a namespace): admit as
   a wildcard-namespace form with a lookup-fallback limit?
5. C# `TypeForwardedTo`: admit with "source unresolved" limit, or exclude as
   assembly-metadata (lean: admit with limit)?
6. Class-member forwarding (C++ `using Base::x`): in scope only if the
   report gets a "member" form; otherwise out.
7. C# `global using` and per-file `unused-import` lint interaction: not
   verified against repo behavior.

Not verified / source gaps:

- Zsh `zmodload` semantics and whether `export -f` has any meaning in zsh.
- Whether `export using namespace N;` is permitted by the C++ standard
  (draft example says OK, no rule text found).
- Exact C# language version for file-scoped namespaces and exact C++ draft
  edition of the eel.is mirror.
- C standard text (only cppreference read).

---

## Per-language summary

| Language | Classification | Notes |
|---|---|---|
| Java | No applicable construct for symbols; `requires transitive` is dependency exposure, recommended as a separate kind or out | Grammar complete. Imports are file-local per JLS 7.5; JLS 7.7.1 itself says it is not a re-export. |
| PHP | No applicable construct (`use`, `include`); `class_alias` is a dubious runtime convention, recommend out | Grammar complete; `function`/`const` is an anon-token field. |
| Bash | No applicable construct; `source` barrel is dubious (wildcard-module at best), recommend out | Grammar complete. Distinguish loading, env propagation, re-export. |
| Zsh | No applicable construct; `source` as Bash; `autoload` is a lazy-load declaration, not a forward | Separate grammar; structural differences from Bash; `export -f` semantics unverified. |
| C | No applicable construct; `#include`, `#define` alias, `typedef` are not interface exposure | Gaps: `__has_include` (C23); grammar accepts `extern "C"`. |
| C++ | Dedicated syntax: namespace `using`-declaration (named), using-directive (wildcard, with limit), `export import`/`export using` (modules, but unparseable). Convention: type aliases. Outside: class-member `using`, `#include` | Blocker: no C++20 module support; silent misparse of `import`/`module`. |
| C# | No applicable construct for `using`/`global using`/aliases; `TypeForwardedTo` is a dubious dedicated attribute needing a limit | Grammar complete; file-scoped namespace is flat. |
