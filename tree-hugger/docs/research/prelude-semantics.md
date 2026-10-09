# Prelude‑like Patterns Across Languages: Identifying Important Symbols via Static Analysis, Language Features, and Build Tools

A **prelude** in Rust (e.g., the [standard prelude](https://doc.rust-lang.org/std/prelude)) re‑exports a curated set of frequently used symbols so users can write concise code without explicit imports.  Other languages rarely use the term *prelude*, but most have either built‑in identifiers, global scopes or conventions that achieve similar ends.  This report examines **Go, PHP, Python, C, C++, Lua, JavaScript and TypeScript**, summarising how their features can expose important symbols automatically and proposing heuristics for identifying a prelude‑like set via static analysis, feature detection and awareness of build tools.

## Go

### Built‑in functions, types and constants

Go’s specification defines a universe block of **predeclared identifiers** that are always in scope.  The official `builtin` package exists only to document these items; it states that it *“documents the built‑in functions, types and variables”* and clarifies that these identifiers aren’t actually in a package but are available everywhere [oai_citation:1‡pkg.go.dev](https://pkg.go.dev/builtin#:~:text=Overview%20%C2%B6).  The index lists functions (`append`, `cap`, `len`, `make`, etc.), types (`any`, `bool`, `int`, `string`, etc.), constants (`true`, `false`, `iota`), and variables (`nil`) [oai_citation:2‡pkg.go.dev](https://pkg.go.dev/builtin#:~:text=,type%20uintptr).  The language specification re‑states the list of **predeclared types**, **constants**, **zero values** and **functions** [oai_citation:3‡go.dev](https://go.dev/ref/spec#:~:text=The%20following%20identifiers%20are%20implicitly,Go%201.21).

### Export rules

Go treats identifiers beginning with an uppercase letter as exported when declared at package level [oai_citation:4‡go.dev](https://go.dev/ref/spec#:~:text=Exported%20identifiers%C2%B6).  Packages cannot implicitly re‑export names, so there is no direct prelude mechanism.  However, documentation and naming conventions make it clear which identifiers form the public API, and code analysis tools (e.g., `go/doc`) can use the uppercase rule to discover exported symbols.

### Identifying a prelude‑like set

1. **Static analysis**: parse Go source code to extract all predeclared identifiers from the specification and treat them as the built‑in prelude.  Then scan imported packages for exported names (uppercase) used without qualification.
2. **Build tool awareness**: the `go mod` build system does not provide an explicit prelude but relies on packages.  When building a project, look at packages imported in `import` statements to identify dependencies whose exported identifiers may be used without qualification inside the package (`.`, alias imports).  Tools like `golang.org/x/tools/go/packages` can assist.
3. **Feature detection**: detect usages of blank imports (`_`) or dot imports (using `.`), which bring all exported names of a package into the current scope.  Names imported in this way effectively act like a prelude and should be highlighted.

## PHP

### Global functions, constants and classes

PHP uses namespaces to organize code, but **built‑in functions and constants** still reside in the global namespace.  When code in a namespace calls an unqualified function name, PHP first searches the current namespace and, if not found, **falls back to the global function or constant** [oai_citation:5‡php.net](https://www.php.net/manual/en/language.namespaces.fallback.php#:~:text=Inside%20a%20namespace%2C%20when%20PHP,fully%20qualified%20Name%20as%20in).  Because built‑in functions and many classes predating PHP 5.3 lack namespaces, they are always available from any namespace [oai_citation:6‡blog.eduonix.com](https://blog.eduonix.com/2014/12/global-namespace-and-fallback-rules-in-php/#:~:text=,etc).  To access a global name explicitly, one can prefix it with a backslash (`\\`) [oai_citation:7‡php.net](https://www.php.net/manual/en/language.namespaces.global.php#:~:text=Without%20any%20namespace%20definition%2C%20all,the%20context%20of%20the%20namespace).

PHP also defines **superglobals** such as `$GLOBALS`, `$_SERVER`, `$_GET`, `$_POST`, and `$_SESSION`, which the manual says are *“always available in all scopes of a script”* [oai_citation:8‡php.net](https://www.php.net/manual/en/language.variables.superglobals.php#:~:text=Superglobals).  These variables operate like a built‑in prelude for stateful data.

### Autoload and Composer

The [Composer](https://getcomposer.org/) package manager can autoload files and classes.  The `autoload.files` section of `composer.json` specifies files that are included automatically in every request; this mechanism can be used to load functions that act like a prelude for a library [oai_citation:9‡getcomposer.org](https://getcomposer.org/doc/04-schema.md#:~:text=).  The PSR‑4 autoload configuration similarly maps namespaces to directories but does not automatically expose functions [oai_citation:10‡getcomposer.org](https://getcomposer.org/doc/04-schema.md#:~:text=Example%3A).

### Identifying a prelude‑like set

1. **Static analysis**: treat all built‑in functions (listed in the PHP manual) and superglobals as part of the prelude.  Recognize fallback resolution rules: when in a namespace, unqualified function calls should be analysed to see if they refer to built‑ins [oai_citation:11‡php.net](https://www.php.net/manual/en/language.namespaces.fallback.php#:~:text=Inside%20a%20namespace%2C%20when%20PHP,fully%20qualified%20Name%20as%20in).  Code analysis could flag these as default imports.
2. **Feature detection**: inspect `composer.json` for `autoload.files` to identify functions that are always included.  Tools like `composer dump-autoload` can reveal the autoloaded files.  Also check for `use function` declarations that import functions into local scope.
3. **Build tool awareness**: frameworks may define `index.php` or `bootstrap.php` files that preload helper functions.  Recognize such bootstrapping code to assemble a prelude.

## Python

### Built‑in functions and types

Python’s interpreter has a collection of **built‑in functions and types** (e.g., `len()`, `print()`, `range`, `list`, `dict`) that are *“always available”* [oai_citation:12‡docs.python.org](https://docs.python.org/3/library/functions.html#:~:text=Built).  These are accessible through the implicit global `builtins` module; the documentation notes that the `builtins` module provides direct access to all built‑in identifiers [oai_citation:13‡docs.python.org](https://docs.python.org/3/library/builtins.html#:~:text=%60builtins%60%20%E2%80%94%20Built).  When a module defines its own function with the same name as a built‑in, one can reference the original using `builtins.name` [oai_citation:14‡docs.python.org](https://docs.python.org/3/library/builtins.html#:~:text=%60builtins%60%20%E2%80%94%20Built).

### Controlling the public API with `__all__`

Python modules may define a list named `__all__` that contains strings naming the objects that should be imported when clients use `from module import *`.  Real Python emphasises using `__all__` to explicitly specify the public interface, prevent accidental exposure of internal names, and improve readability; it recommends keeping `__all__` focused on the public interface and updating it regularly [oai_citation:15‡realpython.com](https://realpython.com/python-all-attribute/#:~:text=to%20explicitly%20specify%20the%20public,with%20unnecessary%20or%20conflicting%20names).  Packages may include an `__init__.py` file that re‑exports selected submodules or symbols, acting as an explicit prelude for the package.

### Identifying a prelude‑like set

1. **Static analysis**: consider all names defined in the built‑in namespace as part of the prelude.  Inspect modules and packages for `__all__` lists and treat those names as the intended public interface.  If `__all__` is absent, heuristically expose all names not starting with an underscore.
2. **Feature detection**: use the `ast` module to parse Python files and identify assignments to `__all__`.  Also examine package `__init__.py` files, which often import or re‑export symbols to create a prelude for the package.
3. **Build tool awareness**: Python packaging tools (e.g., Poetry, pip) do not define entry points for modules’ public API, but an `entry_points` section in `setup.py`/`pyproject.toml` can declare console scripts.  For prelude extraction, focus on module exports rather than entry points.

## C

### Precompiled headers and global includes

C lacks modules and has no built‑in import mechanism beyond the preprocessor.  Many projects create a **precompiled header (PCH)** file (e.g., `pch.h` or `stdafx.h`) that includes frequently used headers.  Microsoft’s documentation explains that Visual Studio adds a `pch.h` file when creating a new project; stable standard library headers like `<vector>` are included here, and the precompiled header is compiled only when it or its included files change [oai_citation:16‡learn.microsoft.com](https://learn.microsoft.com/en-us/cpp/build/creating-precompiled-header-files#:~:text=When%20you%20create%20a%20new,you%20only%20make%20changes%20in).  The PCH speeds up builds and effectively acts as a prelude by centralising commonly used includes.

### Identifying a prelude‑like set

1. **Static analysis**: search for precompiled header includes in build configuration (e.g., `/Yu` compiler option) and treat the headers listed there as part of the prelude.  Also scan common header files (e.g., `common.h`) that are included across many source files.
2. **Feature detection**: detect `#include` directives within a PCH file and gather the included standard library headers and project headers.  Recognise that macros or inline functions defined there become globally available to translation units that include the PCH.
3. **Build tool awareness**: examine build scripts (Makefiles, CMakeLists.txt) for `add_precompiled_header()` or `add_definitions(-std=c11)` directives.  Tools like CMake’s `target_precompile_headers` hint at a prelude.

## C++

### Modules and headers

C++20 introduces **modules**, which can replace header files for interface definitions.  A module consists of an interface unit and (optionally) implementation units; the module is compiled once into a binary representation.  Microsoft notes that modules solve problems with header files and reduce compile times [oai_citation:17‡learn.microsoft.com](https://learn.microsoft.com/en-us/cpp/cpp/modules-cpp#:~:text=C%2B%2B20%20introduces%20modules,that%20import%20them).  Only declarations marked with the `export` keyword become visible to code that imports the module; non‑exported names and macros remain hidden [oai_citation:18‡learn.microsoft.com](https://learn.microsoft.com/en-us/cpp/cpp/modules-cpp#:~:text=C%2B%2B20%20introduces%20modules,that%20import%20them).  An example shows a module `Maths` with `export module Maths;` and an exported function `add`, which becomes visible to importers [oai_citation:19‡learn.microsoft.com](https://learn.microsoft.com/en-us/cpp/cpp/modules-cpp#:~:text=The%20,Example).

### Precompiled headers

C++ projects often use precompiled headers in the same way as C to centralise includes.  Libraries may also provide *header‑only* prelude files (e.g., `<boost/config.hpp>`).

### Identifying a prelude‑like set

1. **Static analysis**: for projects using modules, parse module interface units to extract exported names; treat them as the public prelude.  For traditional header files, look for `#include` patterns in a common header or precompiled header.
2. **Feature detection**: inspect build configurations for module maps (e.g., `.gcm` or `.pcm` files) and `export` keywords in source code.  Recognise `inline` namespaces and `using` directives that bring names into scope.
3. **Build tool awareness**: check CMake’s `CXX_STANDARD` and `target_precompile_headers` directives.  Projects may use `add_library(my_lib INTERFACE)` with `target_include_directories` to provide header‑only libraries that act as preludes.

## Lua

### Global environment and standard libraries

Lua maintains a distinguished **global environment**.  The language reference explains that the global variable `_G` is initialised with the global environment; when Lua loads a chunk, the default value of its `_ENV` is this global environment, so *“free names in Lua code refer to entries in the global environment”*.  All standard libraries are loaded into the global environment [oai_citation:20‡lua.org](https://www.lua.org/manual/5.4/manual.html#:~:text=Any%20table%20used%20as%20the,is%20called%20an%20environment).  Thus, functions like `print`, `table.insert`, `math.sin`, etc., are automatically available without requiring `require` statements.

### Identifying a prelude‑like set

1. **Static analysis**: treat all keys of the global environment (`_G`) at start‑up as part of the prelude.  Tools can emulate the Lua interpreter to list standard libraries loaded by default.
2. **Feature detection**: detect assignments to `_G` or modifications of the global environment.  Modules may deliberately pollute `_G` (e.g., `class.lua`), which effectively extends the prelude.
3. **Build tool awareness**: some Lua ecosystems (e.g., LÖVE, Roblox) preload specific libraries.  Recognise environment variables like `package.preload` and `package.path` in build scripts.

## JavaScript (Node.js) & TypeScript

### Standard built‑ins and global objects

JavaScript defines a set of **standard built‑in objects**—`Infinity`, `NaN`, `undefined`, and functions like `eval`, `parseInt`, `isFinite`, etc.—which are in the global scope [oai_citation:21‡developer.mozilla.org](https://developer.mozilla.org/en-US/docs/Web/JavaScript/Reference/Global_Objects#:~:text=Standard%20built).  These built‑ins are implicitly available without import, forming a language‑level prelude.

### Modules, packages and entry points

In Node.js, each file is a module.  A package’s `package.json` can specify a `main` entry point; more modern packages use an `exports` field that defines which modules and subpaths are part of the package’s public API.  The Node.js documentation notes that `exports` allows multiple entry points and prevents access to any other files; it is recommended for new packages and overrides `main` when both are present [oai_citation:22‡nodejs.org](https://nodejs.org/api/packages.html#:~:text=Package%20entry%20points).  The `exports` field encapsulates subpaths, meaning consumers cannot import internal files unless explicitly listed [oai_citation:23‡nodejs.org](https://nodejs.org/api/packages.html#:~:text=Conditional%20exports%20%20can%20be,dual%20CommonJS%2FES%20module%20packages%20section).  This mechanism is akin to a prelude because it selectively exposes modules to users.

TypeScript, being a superset of JavaScript, adds *types* but shares the module system.  Projects often create an `index.ts` file that re‑exports classes, functions and types from submodules.  This **barrel pattern** simplifies imports: clients can import everything from the root rather than individual files.  Such an `index.ts` functions as an explicit prelude.

### Identifying a prelude‑like set

1. **Static analysis**: treat JavaScript’s built‑in global objects and functions as part of the prelude.  Analyse `package.json` to identify `main` and `exports` fields; treat exported modules and subpaths as the library’s prelude.  For TypeScript, inspect `index.ts` files or `src/index.ts` and gather re‑exported symbols.
2. **Feature detection**: parse ES modules (`export ... from`) to identify aggregated exports.  Detect wildcard exports (`export * from './utils.js'`), which enlarge the prelude.  Use TypeScript’s compiler API to examine `declare` statements and type exports.
3. **Build tool awareness**: bundlers (Webpack, Rollup) use an `entry` configuration; the entry file often acts as the prelude.  Check bundler config files (`webpack.config.js`) to find the entry point.  For libraries using `tsup` or `vite`, examine the build scripts or `exports` map.

## Cross‑language observations and heuristics

Although these languages differ in syntax and tooling, common patterns emerge:

| Language | Built‑in/Global Prelude | Conventions & Tools | How to Detect / Extract |
|---|---|---|---|
| **Go** | Predeclared types, constants and functions (`append`, `len`, etc.) [oai_citation:24‡pkg.go.dev](https://pkg.go.dev/builtin#:~:text=,type%20uintptr) | Uppercase names exported; dot imports bring all names into scope | Parse spec or `builtin` package to list predeclared identifiers; identify dot imports; use `go/doc` for exported names. |
| **PHP** | Global functions and constants fallback when namespaced [oai_citation:25‡php.net](https://www.php.net/manual/en/language.namespaces.fallback.php#:~:text=Inside%20a%20namespace%2C%20when%20PHP,fully%20qualified%20Name%20as%20in); superglobals always available [oai_citation:26‡php.net](https://www.php.net/manual/en/language.variables.superglobals.php#:~:text=Superglobals) | Composer `autoload.files`, bootstrap scripts | Inspect `composer.json` and autoloaded files; treat built‑ins and superglobals as prelude. |
| **Python** | Built‑in functions and types always available [oai_citation:27‡docs.python.org](https://docs.python.org/3/library/functions.html#:~:text=Built); `builtins` module gives access [oai_citation:28‡docs.python.org](https://docs.python.org/3/library/builtins.html#:~:text=%60builtins%60%20%E2%80%94%20Built) | `__all__` defines public API [oai_citation:29‡realpython.com](https://realpython.com/python-all-attribute/#:~:text=to%20explicitly%20specify%20the%20public,with%20unnecessary%20or%20conflicting%20names) | Extract built‑in names; parse `__all__` lists; treat names not starting with `_` as public if `__all__` absent. |
| **C** | Precompiled header (e.g., `pch.h`) centralises includes [oai_citation:30‡learn.microsoft.com](https://learn.microsoft.com/en-us/cpp/build/creating-precompiled-header-files#:~:text=When%20you%20create%20a%20new,you%20only%20make%20changes%20in) | Build scripts specify PCH and common headers | Analyse PCH contents; inspect build config for precompile directives. |
| **C++** | C++20 modules export selected names [oai_citation:31‡learn.microsoft.com](https://learn.microsoft.com/en-us/cpp/cpp/modules-cpp#:~:text=C%2B%2B20%20introduces%20modules,that%20import%20them) | `export` keyword marks visible API [oai_citation:32‡learn.microsoft.com](https://learn.microsoft.com/en-us/cpp/cpp/modules-cpp#:~:text=The%20,Example); PCH similar to C | Parse module interface units to collect exported declarations; check PCH. |
| **Lua** | Global environment `_G` contains standard libraries [oai_citation:33‡lua.org](https://www.lua.org/manual/5.4/manual.html#:~:text=Any%20table%20used%20as%20the,is%20called%20an%20environment) | Modules can modify `_G` | List keys in `_G`; detect modifications; treat standard libraries as prelude. |
| **JS/TS** | Standard built‑in objects/functions in global scope [oai_citation:34‡developer.mozilla.org](https://developer.mozilla.org/en-US/docs/Web/JavaScript/Reference/Global_Objects#:~:text=Standard%20built) | `package.json` `exports` defines entry points [oai_citation:35‡nodejs.org](https://nodejs.org/api/packages.html#:~:text=Package%20entry%20points); barrel `index.ts` pattern | Parse built‑ins; inspect `package.json` for `exports`/`main`; examine `index.ts` for re‑exports; check bundler entry. |

## Conclusion

While the term **prelude** is specific to Rust, many languages provide built‑in identifiers or tools that implicitly import commonly used symbols.  By combining **static analysis** (extracting built‑ins, exported identifiers and re‑exports), **feature detection** (spotting `__all__`, autoload configurations, module `exports` fields, PCH includes) and **build tool awareness** (examining `composer.json`, `package.json`, bundler configs, PCH options and C++ modules), one can assemble a prelude‑like set of symbols for a given project.  This set clarifies the user‑facing API, guides documentation and helps language models focus on the most relevant names when answering questions or generating code.
