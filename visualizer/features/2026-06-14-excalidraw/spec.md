---
status: ready for planning and implementation
reviewed: true
---

# Visualizer - Excalidraw Core Specification

> **Reviewed status:** Ready for planning and implementation. Created and
> reviewed 2026-06-14.
>
> This document captures the architecture decisions for a **standalone desktop
> application** ("Visualizer") that embeds Excalidraw as a core feature.
> It is *not* related to the `excalidraw-zed-extension` project. Where this spec
> references that extension, it is only to contrast architectures and reuse
> hard-won integration knowledge.

---

## 1. Purpose & Scope

Visualizer is a cross-platform desktop application built with **Tauri 2** that
provides Excalidraw diagramming as a major part of a larger feature set. Unlike
the Zed extension (a live-preview tool spawned per-file), Visualizer is a
full-fledged editing application with its own UI shell, file management, and
additional product features layered around the Excalidraw canvas.

This spec intentionally defines the Excalidraw-centered application foundation
only. Later product features must build on the contracts below instead of
changing the editor, storage, or IPC model opportunistically.

### In scope

- Desktop shell architecture (Tauri 2)
- Frontend framework decision and rationale
- Excalidraw integration strategy
- Mapping of backend responsibilities to Tauri primitives
- Local document model, save semantics, and external-change handling
- Security and Tauri permission boundaries for local files and assets
- Packaging constraints that affect architecture

### Out of scope

- The specific "larger feature set" beyond diagramming (TBD)
- Collaboration / multiplayer
- Mobile targets
- Detailed visual design and interaction polish beyond the structural shell
- Cloud sync, accounts, and remote asset storage
- Plugin scripting or arbitrary user-provided code

---

## 2. Key Decisions

| # | Decision | Rationale |
|---|----------|-----------|
| D1 | **Desktop shell: Tauri 2** | Same `wry`/`tao` WebView foundation already proven in the Zed extension, plus batteries-included bundling, signing/notarization, auto-update, and a permissions model. |
| D2 | **Frontend framework: React** | Excalidraw's editor *is* a React class component (`App.tsx`); React is a mandatory peer dependency. Staying in React keeps the path to **upgrading Excalidraw versions frictionless** and gives native access to its composition slots (`<MainMenu>`, `<Sidebar>`, `<Footer>`, `<WelcomeScreen>`). A Vue shell + React bridge was considered and rejected — see §8. |
| D3 | **Excalidraw via `@excalidraw/excalidraw`** | Use the published package directly, never fork or re-implement the editor. Pin the exact package version in `package.json`; as of this review the current stable package is `0.18.1`, so planning should start there unless a newer stable release is deliberately evaluated. The `.excalidraw` JSON format remains the persisted document contract. |
| D4 | **Backend transport: Tauri IPC, not HTTP** | Replace the extension's `axum` HTTP server + SSE with Tauri commands (`invoke`) and events (`emit`/`listen`). No localhost server. |
| D5 | **Rust owns filesystem authority** | The React app may hold editor state, but opening, saving, watching, exporting to disk, and resolving file URLs are Rust-side responsibilities. This keeps path validation, permissions, and atomic writes out of the WebView. |
| D6 | **Document-first shell: one app instance, multiple documents** | Use `tauri-plugin-single-instance` to route second launches into the existing app. The initial product should support multiple documents in one process through tabs or windows backed by a shared Rust document registry; the exact tab/window UX can be planned separately. |
| D7 | **No broad filesystem plugin permissions in the MVP** | Prefer narrow custom commands over granting the frontend wide `fs:*` capabilities. Use the dialog plugin for user-mediated open/save paths and only expose file URLs via `convertFileSrc()` for assets tied to open documents. |
| D8 | **Atomic local saves with conflict detection** | Writes must go through a temporary file + atomic rename strategy and carry a last-seen file fingerprint. If the file changed externally since it was read, the app must stop and present a conflict state instead of overwriting silently. |
| D9 | **Local settings and Excalidraw library are app data, not sidecar files** | Persist app preferences and the user's Excalidraw library under the platform app-data directory, using Tauri path APIs. Project-scoped libraries are deferred until the broader workspace model is defined. |
| D10 | **Repo package naming must follow product names** | The existing placeholder Rust package name `src` is not acceptable for implementation. The Tauri Rust crate/package should be renamed to `visualizer`; any frontend package should use the same product name or a scoped monorepo name. |

> **Reader's note:** This review changes the draft from "architecture sketch" to
> "implementation contract" by deciding file authority, save behavior, and
> Tauri permission boundaries. Those decisions reduce future flexibility, but
> they prevent accidental broad filesystem access and data-loss-prone save paths.

### D2 rationale in depth — why React over Vue

Excalidraw has two layers with opposite React-coupling:

- **Rendering layer (~0% React):** imperative dual-canvas drawing via RoughJS +
  Canvas 2D, coordinated by a `Renderer` class. Framework-agnostic in practice.
- **Editor layer (~100% React):** `App.tsx` is a class-component kernel that owns
  `Scene`, `Store`, `History`, `ActionManager`, `Library`. All chrome (toolbars,
  panels, dialogs, command palette) is React; state flows via React state + a
  custom action bus + Jotai atoms. **Not extractable from React.**

A Vue shell is technically possible (mount the React editor as an island via
`createRoot`/`createElement`, bridge through the imperative `excalidrawAPI`), but:

- It ships **both** React+ReactDOM (~130 KB gz) and Vue runtimes.
- Excalidraw's own UI slots want **React children** — injecting custom UI *inside*
  its menu/sidebar from Vue means Vue-in-React-in-Vue nesting. Avoid.
- It adds a hand-maintained bridge that complicates every Excalidraw upgrade.

Since Excalidraw is a *core* feature (not a peripheral one), the editor and much
custom UI will live close to its API. React keeps that native and upgrades cheap.

---

## 3. Architecture Overview

```
┌─────────────────────────────────────────────────────────┐
│ Tauri 2 App (Visualizer)                                 │
│                                                          │
│  ┌────────────────────────┐   IPC    ┌────────────────┐  │
│  │ WebView (system)        │ commands │ Rust Core      │  │
│  │  - React + Vite         │◄────────►│  - file I/O    │  │
│  │  - @excalidraw/excalidraw│  events  │  - watcher     │  │
│  │  - app shell / chrome   │          │  - export      │  │
│  └────────────────────────┘          │  - window mgmt │  │
│                                       └────────────────┘  │
└─────────────────────────────────────────────────────────┘
```

WebView engine: WebKit (macOS/Linux), WebView2 (Windows) — same as `wry`.
Linux requires **webkit2gtk 4.1** (Tauri v2).

---

## 4. Backend (Rust) - Tauri command/event surface

Replaces the Zed extension's HTTP routes. Reviewed surface:

| Tauri primitive | Replaces (extension route) | Notes |
|---|---|---|
| `#[command] open_document(path)` | `GET /data` | Validates path, reads UTF-8 JSON, restores through Excalidraw on the frontend, records file fingerprint in Rust document registry. |
| `#[command] save_document(handle, scene, expected_fingerprint)` | `POST /data` | Validates payload shape, detects external modification, writes atomically, returns new fingerprint. |
| `#[command] save_document_as(handle, scene, path)` | `POST /data` + file picker | Uses dialog-selected path, writes atomically, updates handle. |
| `#[command] export_document(handle, opts)` | `POST /export` | Use `tauri-plugin-dialog` for save dialog when no output path is supplied. |
| `#[command] read_library()` / `write_library()` | `GET/POST /library` | App-data persistence, schema/version wrapper, atomic writes. |
| `#[command] asset_url(handle, asset_id)` | asset route | Returns a `convertFileSrc()`-compatible URL only for assets associated with an open document. |
| `emit("document:changed-on-disk", payload)` | `GET /events` (SSE) | Driven by `notify` watcher; frontend enters conflict/reload state. |
| `tauri-plugin-single-instance` | lock file + `/ping` + `/focus` | In-app window reuse; no per-file process model. |
| Window lifecycle / `app.exit()` | `GET /shutdown` | |

### Things that go away vs. the extension

- **The per-file multi-process model** — existed only because the Zed extension
  runs in a WASM sandbox and can't open windows/sockets. Visualizer is one app
  process with multiple windows/tabs; reuse is in-memory state.
- **`axum`, SSE plumbing, `rust-embed` asset routes, lock-file dance** — collapse
  into commands + events + Tauri's built-in asset bundling.

### Document registry and file identity

Rust must maintain an in-memory document registry keyed by an opaque handle, not
by raw path strings flowing through React state. Each entry records:

- canonical local path, if the document has one
- last persisted fingerprint, at minimum `(modified_time, length)` and preferably
  a fast content hash when metadata is ambiguous
- dirty state known by the backend after successful saves
- watcher subscription state
- asset roots allowed for `convertFileSrc()` URL generation

The frontend can display paths and handles, but every command that touches disk
must resolve through the registry and revalidate the path. This avoids stale
paths, path traversal mistakes, and accidental writes to files the user has not
opened or selected.

### Save and conflict semantics

- First save for an untitled document must go through `save_document_as`.
- Normal save sends the current Excalidraw scene plus the last fingerprint the
  frontend observed.
- If the on-disk fingerprint differs from `expected_fingerprint`, Rust returns a
  typed conflict error and does not write.
- The frontend must expose three choices for conflicts: reload from disk, save as
  a new file, or keep editing without overwriting.
- Successful save returns the new fingerprint and clears the conflict state.
- Watcher-originated events must not apply `updateScene` directly. They should
  mark the document as changed externally and let the user choose reload.

Use Excalidraw's `captureUpdate` controls for programmatic reloads so external
reloads do not pollute the undo stack.

### Large-payload caution

Tauri's event system evaluates JS under the hood and is poor for large payloads.
For big diagrams with embedded base64 images:
- Read files into the WebView via `convertFileSrc()` where possible.
- Use Tauri 2 **raw IPC requests / `Channel`** for large structured transfers.

---

## 5. Frontend (React + Vite)

- **Build:** Vite (`@vitejs/plugin-react`). ESM only (Excalidraw v0.18+ dropped UMD).
- **Excalidraw:** exact-pinned `@excalidraw/excalidraw`, with `react` and
  `react-dom` as dependencies. Start from `0.18.1` unless planning deliberately
  chooses a newer stable version.
- **Asset path:** set `window.EXCALIDRAW_ASSET_PATH` in a `<script>` **before** the
  module loads; ship fonts locally from
  `node_modules/@excalidraw/excalidraw/dist/prod/fonts/`. (Carried over verbatim
  from the extension — WebView ≠ Chrome, no CDN.)
- **No SSR** — Excalidraw is client-only.
- **Theme:** resolve `auto` via `matchMedia` before mount (WebKitGTK quirk).
- **Data integrity:** funnel all imported data through Excalidraw's `restore()`.
- **Custom UI:** built with React, using Excalidraw composition slots
  (`<MainMenu>`, `<Sidebar>`, `<Footer>`) for in-editor chrome and ordinary React
  components for the surrounding app shell.

### Carry-over gotchas from the extension worth re-using

- `EXCALIDRAW_ASSET_PATH` must precede module load.
- `loadFromBlob` throws on invalid data → wrap in try/catch.
- `updateScene` undo behavior → use `captureUpdate` (`NEVER` for remote/programmatic
  updates) to avoid polluting the undo stack.
- WebKitGTK theme/`matchMedia` resolution before render.

### Frontend state boundary

React state owns transient editor UI state. Rust owns persisted document identity.
The frontend model should therefore separate:

- `EditorScene`: current Excalidraw elements, app state, files, and library state
- `DocumentHandle`: opaque backend handle plus display path and fingerprint
- `DocumentStatus`: clean, dirty, saving, save failed, changed on disk, conflict
- `ShellState`: tabs/windows, active document, recent files, theme, sidebar state

Do not store raw absolute paths as the only document identity in Zustand or React
context. Raw paths are display data and command arguments for user-selected
operations only.

### Import, export, and validation

- All `.excalidraw` imports must run through Excalidraw's `restore()` before
  entering editor state.
- Rust should still reject clearly invalid or oversized JSON before passing it to
  the WebView. The first implementation may use a conservative maximum file size
  and a shallow check for an object payload; deeper format normalization belongs
  to Excalidraw.
- Export is not the same as save. Save persists `.excalidraw` JSON; export writes
  rendered SVG/PNG or clipboard-oriented artifacts and does not change the
  document fingerprint unless explicitly specified by a future feature.
- Embedded image files must be scoped to the open document and exposed through
  Tauri asset URLs instead of broad filesystem reads from the frontend.

---

## 6. Security, Permissions, and Packaging

### Tauri capabilities

The default capability should start narrow:

- `core:default`
- `dialog:default`
- minimum event/window permissions needed for document events and focusing
- no broad `fs:default` unless a specific implementation task proves custom
  commands are insufficient
- no `shell:allow-open` unless external URL/file opening is implemented and
  allowlisted

The frontend should not be able to read or write arbitrary files through plugin
APIs. File access should be mediated by Rust commands that know the document
registry and by user-selected paths returned from dialogs.

### Content security policy

The CSP must allow local app assets, Excalidraw font assets, `data:` images for
embedded scene files, and Tauri-converted asset URLs. It must not allow remote
scripts. If remote images are ever supported, they must be a separate feature
with an explicit fetch/cache policy.

### Packaging

- Tauri 2 platform prerequisites must be documented before implementation:
  WebView2 on Windows, WebKitGTK 4.1 on Linux, and macOS signing/notarization
  requirements for distributed builds.
- Auto-update is not part of the MVP unless signing identities and release
  channels are known. Do not add updater permissions until that decision is made.
- Bundle Excalidraw fonts and static assets locally. No CDN dependency is allowed
  for core editor rendering.

---

## 7. Testing and Acceptance Criteria

Implementation is complete when these behaviors are covered:

- Opening a valid `.excalidraw` file restores into the editor and records a
  backend document handle.
- Invalid Excalidraw JSON reports a typed error without replacing the current
  scene.
- Saving writes atomically and updates the backend fingerprint.
- Saving after an external file change refuses to overwrite and enters a conflict
  state.
- Watcher events mark documents as externally changed without mutating the undo
  stack.
- `save as` moves an untitled or existing document to the selected path and
  updates the registry.
- Library read/write persists under app data and survives app restart.
- Excalidraw fonts load from the bundled asset path on macOS, Windows, and Linux.
- Large embedded-image scenes avoid event payload transfer for image bytes.
- The app starts as a single instance and routes second-launch file paths to the
  existing process.

Recommended test split:

- Rust unit tests for path validation, document registry behavior, atomic write
  conflicts, and typed error serialization.
- Frontend unit tests for document status transitions and Excalidraw wrapper
  behavior around `restore()` / `updateScene`.
- One Tauri integration or smoke test per platform tier for startup, bundled
  assets, and open/save command wiring.

---

## 8. Rejected / Deferred Alternatives

| Alternative | Verdict | Reason |
|---|---|---|
| Vue shell + React-bridged Excalidraw | **Rejected** | Ships two runtimes; can't use Excalidraw's React slots cleanly; complicates upgrades. |
| Re-implement renderer in Vue (use only the format) | **Rejected** | Massive effort; loses all upstream features (elbow arrows, command palette, Mermaid import, cropping, multiplayer undo). |
| Electron instead of Tauri | **Deferred/No** | Heavier (bundles Chromium); Tauri reuses the proven `wry`/`tao` stack and is lighter. |
| Node/Python **sidecar** for backend | **Rejected** | Backend logic is trivial and already idiomatic Rust; sidecar reintroduces the very HTTP-process complexity Tauri lets us drop. |
| Broad frontend filesystem plugin use | **Rejected for MVP** | Easier to prototype, but it bypasses the document registry and makes conflict-safe saves harder to enforce. |
| Storing Excalidraw library beside every document | **Deferred** | Could be useful for projects, but the workspace model is not defined yet; app-data storage is simpler and matches Excalidraw's global-library behavior. |

---

## 9. Open Questions

### 9.1 Broader feature set beyond diagramming

The app shell can be planned now, but the larger product direction still affects
navigation, storage, and whether a workspace model is needed.

Suggested solutions:

- **Option A: Diagram-first editor with recent files and global library.**
  - Pros: smallest MVP, validates Tauri/Excalidraw integration quickly, avoids
    premature workspace abstractions.
  - Cons: later workspace features may require navigation changes.
- **Option B: Workspace-first app with project folders and project libraries.**
  - Pros: better if the real product is a multi-file visual knowledge tool.
  - Cons: forces storage, indexing, and UI decisions before core editor behavior
    is proven.
- **Option C: Canvas plus asset manager.**
  - Pros: useful for image-heavy diagrams and reusable components.
  - Cons: expands scope into asset ingestion, previews, and project structure.

**Recommendation:** Option A. It keeps the implementation focused on the
highest-risk integration points in this spec. Workspace/project concepts can be
added later because D5-D9 keep persisted identity in Rust instead of scattering
filesystem assumptions through the frontend.

### 9.2 Document presentation: tabs, windows, or both

The architecture should support multiple documents in one process, but the first
UX needs a concrete choice.

Suggested solutions:

- **Option A: Single window with tabs.**
  - Pros: simplest state model, predictable single-instance routing, easiest
    conflict UI.
  - Cons: less native for users who expect separate windows on large desktops.
- **Option B: Multi-window only.**
  - Pros: native desktop feel and better multi-monitor use.
  - Cons: more Tauri window lifecycle complexity and harder shared state.
- **Option C: Tabs first, detachable windows later.**
  - Pros: starts simple while preserving a path to power-user workflows.
  - Cons: later detaching requires careful state ownership.

**Recommendation:** Option C, implemented initially as Option A. The backend
document registry should be window-agnostic from day one, but UI planning should
ship tabs first.

### 9.3 Distribution and auto-update

Signing identities, release channels, and update infrastructure are not known.

Suggested solutions:

- **Option A: No updater in MVP.**
  - Pros: avoids premature permissions and release signing complexity.
  - Cons: manual installs until distribution is settled.
- **Option B: Add updater now with a placeholder channel.**
  - Pros: exercises release plumbing early.
  - Cons: requires signing/release decisions that are outside this spec.
- **Option C: Build local-only update abstraction with updater disabled.**
  - Pros: keeps call sites ready.
  - Cons: speculative code with no immediate behavior.

**Recommendation:** Option A. Add updater permissions only when release channels
and signing identities are known.

### 9.4 View-only or embedded rendering

The React-free Excalidraw rendering layer could support previews, thumbnails, or
exports without mounting the full editor, but this is not necessary for the core
editor MVP.

Suggested solutions:

- **Option A: Defer view-only rendering.**
  - Pros: keeps scope on editor correctness and save safety.
  - Cons: no thumbnail grid or quick preview in the first implementation.
- **Option B: Add a read-only Excalidraw component mode.**
  - Pros: uses official rendering paths and can power previews.
  - Cons: still React-bound and may not reduce runtime cost much.
- **Option C: Build a Rust/native renderer.**
  - Pros: could support non-WebView exports someday.
  - Cons: very high effort and duplicates upstream behavior.

**Recommendation:** Option A. Use Excalidraw's built-in export APIs for MVP
exports and revisit view-only rendering when a concrete feature needs it.

---

## 10. References

- Tauri 2 architecture & IPC: https://v2.tauri.app/concept/architecture/
- Tauri 2 IPC and raw requests: https://v2.tauri.app/develop/calling-rust/
- Excalidraw integration docs: https://docs.excalidraw.com/docs/@excalidraw/excalidraw/integration
- Excalidraw npm package: https://www.npmjs.com/package/@excalidraw/excalidraw
- Excalidraw "Rethinking the Component API": https://plus.excalidraw.com/blog/redesigning-editor-api
- Excalidraw as Design Editor Core (#6921): https://github.com/excalidraw/excalidraw/issues/6921
- Vue ↔ React bridge guide (for the record, not chosen): https://medium.com/@ylenius/how-to-use-react-components-in-a-vue-project-excalidraw-integration-guide-b94c7a22f7d4
