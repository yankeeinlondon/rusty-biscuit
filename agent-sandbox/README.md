# Agent Sandbox

## CLI

We provide a CLI binary called `sandbox` which helps you boot up and interact with sandbox containers.

## Containers

We define cross-OS container environments for development with CLI agents. Containers come in four tiers — a **full** dev container with everything, and three language-focused variants that trade breadth for smaller image size and faster startup.

### Container Tiers

| Feature                        | Full       | Rust       | TypeScript | Python     |
| ------------------------------ | ---------- | ---------- | ---------- | ---------- |
| **Rusty-biscuit repo**         | checkout   | checkout   | checkout   | checkout   |
| **Rusty-biscuit binaries**     | all        | all        | all        | all        |
| **CLI agents**                 | all        | all        | all        | all        |
| **Rust** (rustc, cargo)        | yes        | yes        | -          | -          |
| **TypeScript** (node, bun, pnpm) | yes      | yes (_for scripting_)  | yes        | -          |
| **Python** (python, uv)       | yes        | -          | -          | yes        |
| **Go**                         | yes        | -          | -          | -          |
| **Java** (JDK)                 | yes        | -          | -          | -          |
| **Swift**                      | yes        | -          | -          | -          |
| **dotnet SDK**                 | yes        | -          | -          | -          |
| **Compiler toolchains** (clang, zig, g++) | all | all   | -          | -          |
| **System libraries**           | all        | native/FFI | minimal    | scientific |
| **Graphics/GUI libs**          | all        | all        | -          | -          |
| **Debugging tools** (gdb, lldb, valgrind) | all | all   | -          | -          |
| **Build tools** (make, cmake, ninja, etc.) | all | all  | minimal    | minimal    |

### Common to All Tiers

Every container — regardless of tier — includes:

- A recent checkout of the `rusty-biscuit` monorepo
- All `rusty-biscuit` CLI binaries (see below)
- All CLI agents (Claude Code, OpenCode, Codex CLI, Gemini CLI)
- Modern `git` with host configuration mapped in
- [`just`](https://just.systems/) task runner with provided recipes
- Core utilities: `curl`, `wget`, `tar`, `gzip`, `xz`, `unzip`, `7zip`, `jq`
- `openssl-dev`, `zlib`, `libcurl`, `sqlite`

### Rusty-Biscuit Binaries

All tiers include the following pre-built binaries:

| Binary       | Description                                                     |
| ------------ | --------------------------------------------------------------- |
| `bt`         | `biscuit-terminal` — terminal rendering                         |
| `bf`         | `biscuit-file` — file format conversion                         |
| `bh`         | `biscuit-hash` — cryptographic and non-cryptographic hashing    |
| `sniff`      | Host discovery: hardware, software, services, repo detection    |
| `so-you-say` | TTS functionality leveraging host system capabilities           |
| `playa`      | Audio playback and sound effects from the effect library        |
| `hug`        | Cross-language static analysis tools                            |
| `unchained`  | Configurable AI harness for custom agentic processing           |
| `claudine`   | Cross-agent orchestration across popular agentic frameworks     |

### OS Support

- **Linux:** Debian, Alpine
- **Windows**

### Full Dev Container

The full container includes every language, toolchain, and library. Use this when you need maximum flexibility or are working across multiple languages.

#### Languages

- Rust (`rustc` + `cargo`)
- JavaScript / TypeScript (`node`, `bun`, `pnpm`)
- Python (`python`, `uv`, `python-setuptools`)
- Go
- Java (JDK + `javac`)
- Swift
- dotnet SDK
- Perl, Ruby, Bash

#### Build Tools

- `make`, `cmake`, `gcc`, `ninja`, `meson`, `pkg-config`, `scons`, `bazel`, `buck2`, `pants`, `xmake`, `premake`, `qmake`
- Linux: `autoconf`, `automake`, `libtool`
- Windows: `msbuild`, `devenv`

#### Compiler Toolchains

- `clang`/`llvm`, `g++`, `clang++`, `zig`

#### Linkers

- binutils, lld, gold, mold
- llvm-ar, llvm-nm, llvm-strip
- patchelf, install_name_tool, otool, dumpbin

#### Debugging and Profiling

- `gdb`, `lldb`, `valgrind`
- `strace` / `ltrace`, `perf`, `dtrace` (macOS)
- `clang-tidy`, `clang-format`, `include-what-you-use`
- `bear`, `compiledb`

#### Documentation Generators

- `doxygen`, `sphinx`, `mdbook`, `groff`, `asciidoc`, `pandoc`

#### System Libraries

**Core:**
zlib, bzip2, xz/liblzma, zstd, libarchive, libcurl, libxml2, expat, sqlite, ncurses, readline, libffi, pcre2, gettext, iconv, uuid, libuv, openssl-dev

**Graphics/GUI:**
SDL2, Qt, GTK, cairo, freetype, fontconfig, harfbuzz, libpng, jpeg, tiff, webp

**Networking:**
libsodium, protobuf, zeromq, dbus, boost

**Parsers and Code Generators:**
bison, flex, re2c, swig, protoc, flatbuffers, capnproto, nasm/yasm, m4

#### OS-Specific Libraries

- **Linux:** glibc-devel, linux headers, libstdc++-dev, musl-dev
- **macOS:** Xcode command line tools, macOS SDK
- **Windows:** Windows SDK, MSVC runtime/dev components, NASM/YASM

### Rust Container

A focused environment for Rust development. Includes the full native toolchain and C/FFI libraries commonly needed for Rust crates with native dependencies.

**Languages:** Rust (`rustc` + `cargo`)

**Scripting:** `node`, `bun`, `pnpm`, `bash`

**Build tools:** `make`, `cmake`, `gcc`, `ninja`, `pkg-config`; Linux: `autoconf`, `automake`, `libtool`

**Compiler toolchains:** `clang`/`llvm`, `g++`, `zig`

**Linkers:** binutils, lld, mold, llvm-ar, llvm-nm, llvm-strip, patchelf

**Debugging:** `gdb`, `lldb`, `valgrind`, `strace`/`ltrace`, `perf`

**System libraries:** zlib, bzip2, xz/liblzma, zstd, libarchive, libcurl, libxml2, expat, sqlite, ncurses, readline, libffi, pcre2, openssl-dev, libuv, gettext, iconv, uuid

**Graphics/GUI:** SDL2, Qt, GTK, cairo, freetype, fontconfig, harfbuzz, libpng, jpeg, tiff, webp

**Networking:** libsodium, protobuf, zeromq, dbus

**Documentation:** `mdbook`

### TypeScript Container

A focused environment for JavaScript and TypeScript development.

**Languages:** Node.js, Bun, pnpm

**Build tools:** `make`, `gcc` (for native addons), `python` (node-gyp dependency)

**System libraries:** zlib, openssl-dev, libcurl, sqlite, libuv

**Documentation:** None beyond built-in tooling

### Python Container

A focused environment for Python development with scientific and data-oriented libraries available.

**Languages:** Python (`python`, `uv`, `python-setuptools`)

**Build tools:** `make`, `cmake`, `gcc`, `pkg-config` (for building C extensions)

**Compiler toolchains:** `clang`/`llvm`, `g++`

**System libraries:** zlib, bzip2, xz/liblzma, zstd, libcurl, libxml2, expat, sqlite, ncurses, readline, libffi, openssl-dev, libarchive

**Documentation:** `sphinx`

## CLI Agents

Each container has the following CLI agents pre-installed (but **not** pre-authenticated):

- Claude Code
- OpenCode
- Codex CLI
- Gemini CLI

The following environment variables are mapped from host into the container:

| Variable              | Service        |
| --------------------- | -------------- |
| `ANTHROPIC_API_KEY`   | Anthropic      |
| `BRAVE_API_KEY`       | Brave Search   |
| `DEEPSEEK_API_KEY`    | DeepSeek       |
| `GEMINI_API_KEY`      | Google Gemini  |
| `GROQ_API_KEY`        | Groq           |
| `OPENAI_API_KEY`      | OpenAI         |
| `OPENCODE_API_KEY`    | OpenCode       |
| `OPEN_ROUTER_API_KEY` | OpenRouter     |
| `MOONSHOT_API_KEY`    | Moonshot       |
| `X_AI_API_KEY`        | xAI            |
| `MISTRAL_API_KEY`     | Mistral        |
| `ZAI_API_KEY`         | ZAI            |
| `ZENMUX_API_KEY`      | Zenmux         |

## Git Support

- Every container includes a modern version of `git`
- Host git configuration (user name, email, signing preferences) is mapped into the container

## DevOps Toolbox

- All containers include [`just`](https://just.systems/)
- A set of devops scripts are provided as `just` recipes in the container's `justfile`
- Recipes cover common workflows: building, testing, linting, formatting, and deployment

## Docker

Docker is the primary (and currently only) container runtime.

### Dockerfiles

- We provide Dockerfiles for each OS and tier combination (e.g., `debian-full`, `debian-rust`, `alpine-typescript`)
- Use these as a base to build custom variants
- Pre-built images are published to Docker Hub

### Permissions and Users

Most Docker containers default to the `root` user, but this is problematic for CLI agents — agentic software becomes overly cautious about permissions when running as root. Since the container itself provides the isolation we need, we want the agent to operate without unnecessary permission friction.

All containers provide a user account called `agent` which:

- Has elevated permissions (member of the `sudo` group where applicable)
- Is the default user for all container operations
- Owns the workspace and tool directories

### File Mounts

These containers expose the following mount points:

| Mount Point   | Maps To (inside container)              | Purpose                              |
| ------------- | --------------------------------------- | ------------------------------------ |
| `skills`      | `~agent/.claude/skills/`                | Claude Code skills                   |
| `agents`      | `~agent/.claude/agents/`                | Claude Code sub-agent definitions    |
| `commands`    | `~agent/.claude/commands/`              | Claude Code custom slash commands    |
| `workspace`   | `~agent/workspace/`                     | Working directory for project files  |
| `cache`       | `~agent/.cache/`                        | Shared cache (cargo, pip, node, etc) |

> **Note:** The `unchained` and `claudine` binaries both leverage the skills, agents, and commands mapped into the container, regardless of which CLI agent initiated the session.
