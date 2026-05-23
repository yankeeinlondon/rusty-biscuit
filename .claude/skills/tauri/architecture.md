# Tauri 2.0 Skill — Proposed Architecture

This document summarizes the architecture the `tauri` skill proposes for building
professional cross-platform desktop apps, then drills into the Vue component
layer and how it communicates with the Rust backend.

## 1. High-Level Architecture

The skill prescribes a **two-process desktop application**:

| Process | Technology | Role |
|---------|-----------|------|
| **Backend (Core)** | Rust | Native OS access, business logic, state, security boundary |
| **Frontend (WebView)** | Vite + Vue + TypeScript | UI rendering, presentation, local UI state |

The two halves are separate runtimes. They never share memory; they communicate
only over Tauri's **IPC bridge**, which serializes all payloads as JSON. This is
the single most important architectural fact: every backend interaction is an
async, serialized, fallible boundary crossing.

```
┌──────────────────────────────────────────────────────────┐
│  WebView Process (frontend)                                │
│                                                            │
│   Vue Views → ViewModels → tauriService → invoke()         │
│        ▲                                       │           │
│        │ Pinia stores                          │           │
│        │                                       │           │
└────────┼───────────────────────────────────────┼───────────┘
         │  Tauri events (listen)                 │ IPC (JSON)
┌────────┼───────────────────────────────────────▼───────────┐
│  Core Process (backend)                                     │
│                                                             │
│   #[tauri::command] handlers → services → models            │
│                    ▲                                        │
│                    │  managed AppState (Mutex/RwLock)        │
│                    │  capability-based permission gate       │
└─────────────────────────────────────────────────────────────┘
```

### Backend layering (Rust, `src-tauri/src/`)

The skill organizes the Rust side into thin, single-responsibility modules:

- **`main.rs` / `lib.rs`** — entry point and the `tauri::Builder`. `lib.rs`
  registers managed state (`.manage(...)`), plugins (`.plugin(...)`), and the
  command handler list (`generate_handler![...]`).
- **`commands/`** — `#[tauri::command]` functions, grouped by feature domain
  (`file.rs`, `settings.rs`). These are a **thin IPC adapter layer** — they
  deserialize args, call services, and map errors.
- **`services/`** — pure business logic with *no Tauri dependency*, so it is
  unit-testable in isolation.
- **`models/`** — serializable data structures shared across the IPC boundary.
- **`state/`** — `AppState` held behind `Mutex` (or `tokio::RwLock` for
  async-heavy workloads), registered once via `.manage()`.
- **`error.rs`** — a single `AppError` enum, `Serialize`-able, with `From`
  conversions and a conversion into `tauri::ipc::InvokeError`.

### Security model

Tauri 2.0's **capability-based** model is central. Permissions are declared in
`src-tauri/capabilities/*.json` (e.g. `fs:allow-read` scoped to `$APPDATA/**`).
The frontend can only reach native APIs explicitly granted. The skill treats the
Rust side as the trust boundary: all frontend input is validated in Rust, paths
go through `tauri::path` APIs, and a strict CSP is set in `tauri.conf.json`.

### Frontend layering — MVVM (Vue, `src/`)

The frontend uses **Model-View-ViewModel**:

| Layer | Location | Responsibility |
|-------|----------|----------------|
| **Model** | `models/`, `stores/` | Types and global state (Pinia) |
| **View** | `views/`, `components/` | Pure presentation, no logic |
| **ViewModel** | `viewmodels/` | Business logic as composables |
| **Service** | `services/tauriService.ts` | The *only* place that calls `invoke()` |

Vue is itself an MVVM framework — a `<script setup>` instance is the ViewModel
for its template. The skill adds a dedicated `viewmodels/` layer of composables
so view logic stays testable and reusable independent of any single component.

Build tooling is Vite, with Vue Router for navigation and UnoCSS (an on-demand
atomic CSS engine) for styling — configured via the `unocss/vite` plugin, a
`uno.config.ts` design-token theme, and the `virtual:uno.css` import.

---

## 2. Vue Components and How They Interact with Rust

This is the focus area: how a button click in a Vue component eventually runs
Rust code, and how the result flows back.

### 2.1 The four frontend roles

The skill deliberately keeps components "dumb" by splitting responsibilities:

1. **View components** (`views/`, `components/`) — `.vue` files that render a
   template from props and ViewModel output. They contain *no* `invoke()` calls
   and *no* async orchestration.
2. **ViewModel composables** (`viewmodels/useXxxViewModel.ts`) — own reactive
   state (`loading`, `error`, data as `ref`s), orchestrate calls, and expose a
   clean API to the View.
3. **Service** (`services/tauriService.ts`) — a flat object of typed wrapper
   functions; the single chokepoint for `invoke()`.
4. **Stores** (`stores/useAppStore.ts`) — Pinia stores for cross-view global
   state (theme, sidebar, recent files, open editor tabs).

### 2.2 The call path: View → ViewModel → Service → Rust

A request flows strictly downward, and results flow back up through reactivity:

```
HomeView.vue (@click on a button)
  └─> useHomeViewModel().loadFiles()        [ViewModel: sets loading, try/catch]
        └─> tauriService.listFiles(dir)     [Service: typed wrapper]
              └─> invoke<FileInfo[]>('list_files', { dir })   [Tauri IPC]
                    └─> #[tauri::command] list_files(dir)      [Rust handler]
                          └─> fs::read_dir(...) / service call [Rust logic]
```

**Service layer** — every command gets a typed wrapper so the rest of the
frontend never sees raw `invoke()` strings:

```typescript
// services/tauriService.ts
import { invoke } from '@tauri-apps/api/core';

export const tauriService = {
  listFiles: (dir: string) => invoke<FileInfo[]>('list_files', { dir }),
  readFile:  (path: string) => invoke<string>('read_file', { path }),
  saveFile:  (path: string, content: string) =>
    invoke<void>('save_file', { path, content }),
};
```

This isolates the IPC boundary: the command-name string and argument shape live
in exactly one place, and the generic type parameter (`invoke<FileInfo[]>`)
documents the expected backend response.

**ViewModel layer** — wraps the service call with UI lifecycle state. Because
every `invoke()` is async and can fail, the ViewModel owns the
`loading / error / data` triad as Vue `ref`s:

```typescript
// viewmodels/useFileExplorerViewModel.ts
import { ref, watch } from 'vue';
import { tauriService } from '../services/tauriService';
import type { FileInfo } from '../models';

export function useFileExplorerViewModel(initialDir: string) {
  const files = ref<FileInfo[]>([]);
  const loading = ref(false);
  const error = ref<string | null>(null);

  async function loadFiles(dir: string) {
    loading.value = true;
    error.value = null;
    try {
      files.value = await tauriService.listFiles(dir);   // ← crosses to Rust
    } catch (e) {
      error.value = e instanceof Error ? e.message : 'Failed to load files';
    } finally {
      loading.value = false;
    }
  }

  watch(() => initialDir, loadFiles, { immediate: true });
  return { files, loading, error, loadFiles };
}
```

**View layer** — a `.vue` component consumes only the ViewModel's return value
and renders. It branches on `loading` / `error` with `v-if` and is otherwise
pure:

```vue
<script setup lang="ts">
import { useFileExplorerViewModel } from '../../viewmodels/useFileExplorerViewModel';

const props = defineProps<{ initialDir: string }>();
const { files, loading, error, loadFiles } =
  useFileExplorerViewModel(props.initialDir);
</script>

<template>
  <LoadingSpinner v-if="loading" />
  <ErrorMessage v-else-if="error" :message="error" @retry="loadFiles" />
  <FileList v-else :files="files" />
</template>
```

### 2.3 Argument and result marshalling

Crossing the IPC boundary has concrete rules the components must respect:

- **Argument keys are camelCase on the JS side, snake_case in Rust.** Tauri
  auto-converts: `invoke('save_file', { newSettings })` maps to a Rust parameter
  `new_settings`. The service layer is where this mapping is kept consistent.
- **Everything is JSON-serialized.** Rust `struct`s and `enum`s exposed across
  the boundary must derive `Serialize`/`Deserialize`; TypeScript `interface`s in
  `models/` must mirror them. There is no shared type generation in this skill —
  the two type definitions are maintained in parallel by convention.
- **All calls are async.** `invoke()` returns a `Promise`, even for trivial
  backend work, because it is a cross-process round trip.

### 2.4 Error handling across the boundary

The Rust backend returns `Result<T, AppError>`. `AppError` is a `Serialize` enum
tagged for JSON (`#[serde(tag = "type", content = "message")]`) and converts
into `tauri::ipc::InvokeError`. On the frontend, a rejected `invoke()` promise
carries that serialized error:

```typescript
try {
  await tauriService.readFile(path);
} catch (error) {
  const parsed = JSON.parse(error as string);   // { type, message }
  // surface parsed.type / parsed.message to the user
}
```

In practice the ViewModel catches the rejection and stores a `string` message in
an `error` ref; the View renders it via an `<ErrorMessage>` component. Components
never see raw `invoke()` rejections.

### 2.5 Reusable composables for the IPC boundary

The skill ships generic composables (`composables/`) so ViewModels don't
re-implement the loading/error pattern:

- **`useTauriCommand<T, A>(command, options)`** — generic command runner
  returning `{ data, loading, error, execute, reset }` (all reactive) with
  `onSuccess` / `onError` callbacks. A ViewModel can build on this instead of
  hand-rolling `ref` triads.
- **`useTauriEvent<T>(event, callback)`** — subscribes to **backend-pushed
  events** via `listen()` in `onMounted` and auto-unsubscribes in `onUnmounted`.
  This is the *reverse* direction: Rust emits an event (e.g. `file-changed`) and
  the component reacts without polling.

So component↔Rust interaction is **bidirectional**:

- **Pull / request-response:** component → `invoke()` → command → result.
- **Push / event:** Rust `app.emit(...)` → `useTauriEvent` → reactive update.

### 2.6 Where Pinia fits

`invoke()` results are *transient* request data and live in ViewModel `ref`s.
**Pinia stores hold cross-cutting UI state** — theme, sidebar, recent files,
open editor tabs — and persist it to `localStorage` via the
`pinia-plugin-persistedstate` plugin (the `persist` store option).

The skill also shows an **async store** pattern where the store itself calls
`tauriService` (e.g. `useSettingsStore.fetchSettings()` / `updateSettings()`),
including an optimistic update with rollback on a failed backend call. This is
the exception to "only the service calls invoke" — a store may consume the
service directly when the state it manages is genuinely global rather than
view-scoped.

One Vue-specific rule applies when components read store state: **destructuring
state or getters off a store loses reactivity** — wrap them in `storeToRefs()`.
Actions are plain functions and are safe to destructure directly.

### 2.7 Component interaction summary

| Concern | Owner | Notes |
|---------|-------|-------|
| Rendering | View component (`.vue`) | No `invoke`, no async |
| Loading/error state | ViewModel composable | `try/catch/finally` around service |
| IPC call site | `tauriService` | Only place `invoke()` string appears |
| Cross-process call | Tauri IPC | Async, JSON-serialized, fallible |
| Backend execution | `#[tauri::command]` | Thin adapter → `services/` |
| Backend → frontend push | `useTauriEvent` + `emit` | Reverse-direction updates |
| Global UI state | Pinia store | Persisted; may call service for async state |
| Backend errors | `AppError` → JSON → ViewModel `error` ref | Parsed `{ type, message }` |

### Key takeaway

The architecture's core discipline: **the Vue component tree never touches Rust
directly.** Every interaction funnels through `tauriService.ts`, is wrapped in a
ViewModel composable that manages async lifecycle, and is rendered by a pure
`.vue` View. This keeps the IPC boundary — the one place where serialization,
async failure, and naming conversions all converge — explicit, typed, and
testable in a single layer.
