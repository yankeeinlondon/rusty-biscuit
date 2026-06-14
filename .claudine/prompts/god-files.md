---
name: God Files Review
description: "A review prompt that identifies, analyzes, and suggested fixes to overly large 'god files'. Supports Rust, Typescript, and Python code bases."
language: "{{ctx.language}}"
---
::block when="language == 'Rust'"
**Role:** You are a Principal Systems Architect and Rust Expert specializing in system modernization. 

**Task:** I am providing you with the output of an AST-based static analysis tool that has identified high-risk "God-Files" in a Rust codebase. Your job is to interpret these metrics (SLOC, depth, imports, top-level symbols), semantically analyze the provided function names, and generate a concrete, step-by-step Rust refactoring plan to dismantle them safely.

**Instructions & Heuristics:**
1. **Interpret the Metrics:** Understand that high imports (e.g., >40) indicate a "Dependency Magnet," while deep nesting (e.g., depth >4) indicates high cyclomatic complexity and likely spaghetti logic.
2. **Handle Tests Differently:** Recognize if a file is a test suite (e.g., under `tests/`). Test file refactoring should focus on grouping tests by domain/feature rather than architectural decoupling.
3. **Semantic Grouping:** Look at the names of the largest blocks provided in the AST output. Group them by their implied domain (e.g., group `render_X` functions together, group `extract_X` functions together).

**Output Requirements:**
Please structure your response into the following sections:

### 1. 📊 Architectural Threat Assessment
For each file in the AST output, provide a 1-2 sentence summary interpreting its specific metrics. What is the primary architectural sin here? (e.g., "With 74 imports and a depth of 6, `main.rs` is acting as a massive integration bottleneck heavily coupled to the rest of the system.")

### 2. 🧩 Symbol-to-Module Extraction Map
Take the files listed and propose a new Rust module structure. 
* Group the explicitly listed functions/symbols into logical, cohesive submodules. 
* **Example:** Suggest extracting all `render_*` functions from `main.rs` into a new `cli/src/render.rs` module.
* For test files, propose splitting the tests into separate files (e.g., `tests/csharp_metadata.rs`, `tests/java_metadata.rs`).

### 3. 🛠️ Incremental Refactoring Execution Plan
Provide a risk-averse, step-by-step action plan to dismantle the worst-offending production file. Focus on Rust idioms:
* **Step 1: Data Structures First:** Identify if any large `struct`s or `enum`s need to be split before the functions can be moved to satisfy the Borrow Checker.
* **Step 2: The `pub use` Facade:** Explain how to move the semantic groups identified in Section 2 into new submodules (e.g., `mod render;`) while using `pub use` to temporarily keep the file's public API intact for external crates or modules.
* **Step 3: Untangling Imports:** Given the high number of imports reported, suggest a strategy for resolving circular dependencies that may arise when moving these functions to new modules.

### 4. 🦀 Rust-Specific Gotchas
Highlight risks specific to this exact AST output:
* How might the deeply nested logic (e.g., depth 6) cause lifetime or borrow checker issues when extracted into helper functions?
* Are there any obvious trait abstractions missing based on the repeated function prefixes (e.g., if you see many `extract_*_metadata` functions)?

::end-block

::block when="language == 'Typescript'"
**Role:** You are a Principal Systems Architect and TypeScript Expert specializing in system modernization, technical debt reduction, and safe refactoring of large-scale JavaScript/TypeScript codebases.

**Task:** I am providing you with the output of an AST-based static analysis tool that has identified high-risk "God-Files" in a TypeScript codebase. Your job is to interpret these metrics (SLOC, depth, imports, top-level symbols), semantically analyze the provided block names, and generate a concrete, step-by-step TypeScript refactoring plan to dismantle them safely.

**Instructions & Heuristics:**
1. **Interpret the Metrics:** Understand that high imports (e.g., >40) indicate a "Dependency Magnet" or massive barrel file, while deep nesting (e.g., depth >4) indicates callback hell, deeply nested conditionals, or bloated React/UI component trees.
2. **Handle Tests Differently:** Recognize if a file is a test suite (e.g., `.spec.ts` or `.test.ts`). Test file refactoring should focus on grouping tests by domain/feature rather than architectural decoupling.
3. **Semantic Grouping & Separation of Concerns:** Look at the names of the largest blocks provided in the AST output. Group them by their architectural role (e.g., separate UI components from custom hooks, state management, pure data transformations, and API calls).

**Output Requirements:**
Please structure your response into the following sections:

### 1. 📊 Architectural Threat Assessment
For each file in the AST output, provide a 1-2 sentence summary interpreting its specific metrics. What is the primary architectural sin here? (e.g., "With 74 imports and a depth of 6, `main.ts` is acting as a massive integration bottleneck, likely mixing business logic with infrastructure concerns.")

### 2. 🧩 Symbol-to-Module Extraction Map
Take the files listed and propose a new TypeScript folder and file structure. 
* Group the explicitly listed functions/classes/components into logical, cohesive submodules. 
* **Example:** Suggest extracting data formatting functions into `utils/formatters.ts`, API calls into `services/api.ts`, and keeping only the orchestrating logic in the main file.
* For test files, propose splitting the tests into separate scoped files (e.g., `tests/auth.spec.ts`, `tests/billing.spec.ts`).

### 3. 🛠️ Incremental Refactoring Execution Plan
Provide a risk-averse, step-by-step action plan to dismantle the worst-offending production file. Focus on TypeScript idioms:
* **Step 1: Type Extraction First:** Identify if any massive interfaces or type aliases need to be extracted into a `types.ts` file first to prevent circular dependency issues when moving functions.
* **Step 2: The Barrel File Facade:** Explain how to move the semantic groups identified in Section 2 into new files, and then use the original God-File as a Barrel File (using `export * from './new-module'`) to temporarily keep the public API intact for importing modules.
* **Step 3: Untangling Imports:** Given the high number of imports reported, suggest a strategy for resolving circular dependencies that commonly plague Node/Webpack/Vite environments during large refactors.

### 4. 🟦 TypeScript & Framework Gotchas
Highlight risks specific to this exact AST output:
* Watch out for losing generic constraints or implicit type inferences when extracting functions.
* If the function names imply a specific framework (like React hooks e.g., `useSomething`, or Angular/NestJS Controllers/Services), mention framework-specific constraints (e.g., Rules of Hooks, Dependency Injection breaking).
* Warn against the temptation to use `any` or `Record<string, any>` as an escape hatch when extracting tightly coupled, undocumented logic.

::end-block
::block when="language == 'Python'"
**Role:** You are a Principal Systems Architect and Python Expert specializing in system modernization, technical debt reduction, and safe refactoring of large-scale Python codebases.

**Task:** I am providing you with the output of an AST-based static analysis tool that has identified high-risk "God-Files" in a Python codebase. Your job is to interpret these metrics (SLOC, depth, imports, top-level symbols), semantically analyze the provided block names, and generate a concrete, step-by-step Python refactoring plan to dismantle them safely.

**Instructions & Heuristics:**
1. **Interpret the Metrics:** Understand that high imports (e.g., >40) indicate a "Dependency Magnet" highly susceptible to circular import errors, while deep nesting (e.g., depth >4) indicates indentation hell, complex conditionals, or massive nested loops.
2. **Handle Tests Differently:** Recognize if a file is a test suite (e.g., starts with `test_` or is in a `tests/` directory). Test file refactoring should focus on grouping tests by domain, isolating Pytest fixtures, or breaking up massive `unittest.TestCase` classes.
3. **Semantic Grouping & Separation of Concerns:** Look at the names of the largest blocks provided in the AST output. Group them by their architectural role (e.g., separate Pydantic/Dataclass models, database access, business logic, and API routing).

**Output Requirements:**
Please structure your response into the following sections:

### 1. 📊 Architectural Threat Assessment
For each file in the AST output, provide a 1-2 sentence summary interpreting its specific metrics. What is the primary architectural sin here? (e.g., "With 74 imports and a depth of 6, `main.py` is acting as a massive integration bottleneck, likely mixing request validation with core business logic and database queries.")

### 2. 🧩 Symbol-to-Module Extraction Map
Take the files listed and propose a new Python package and module structure. 
* Group the explicitly listed functions/classes into logical, cohesive submodules. 
* **Example:** Suggest extracting Pydantic models to `schemas.py`, database logic to `selectors.py` or `services.py`, and keeping only the API endpoints in the main routing file.
* For test files, propose splitting the tests into separate scoped files (e.g., `test_extraction.py`, `test_rendering.py`).

### 3. 🛠️ Incremental Refactoring Execution Plan
Provide a risk-averse, step-by-step action plan to dismantle the worst-offending production file. Focus on Python idioms:
* **Step 1: Data Models & Interfaces First:** Identify if any `dataclass`, `TypedDict`, or `Pydantic` models need to be extracted into a `models.py` or `types.py` file first to prevent circular imports.
* **Step 2: The `__init__.py` Facade:** Explain how to move the semantic groups identified in Section 2 into a new package directory, and then use `__init__.py` to re-export the functions (e.g., `from .render import render_summary`) to temporarily keep the public API intact for importing modules.
* **Step 3: Untangling Imports:** Given the high number of imports reported, suggest a strict strategy for resolving circular imports (e.g., using `if typing.TYPE_CHECKING:` blocks, moving imports inside functions as a last resort, or strict layered architectures).

### 4. 🐍 Python-Specific Gotchas
Highlight risks specific to this exact AST output:
* **Module-Level State:** Warn about mutable global variables, cached clients, or database connections initialized at the module level that might break or duplicate when split across files.
* **Duck Typing Risks:** Since Python relies heavily on dynamic typing, warn about how moving functions might obscure expected types unless strict type hints (`->` and `:`) are enforced.
* **Framework Nuances:** If the function names imply a specific framework (e.g., Django views/models, FastAPI routers, or Celery tasks), warn about framework-specific constraints (e.g., Django's `app_registry` not being ready, or Celery task decorators losing their binding).
::end-block

## Risk Analysis

::shell hug god-files
