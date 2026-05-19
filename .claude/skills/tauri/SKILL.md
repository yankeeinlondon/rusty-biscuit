---
name: tauri-v2
description: Guide for building professional desktop apps with Tauri 2.0, Rust backend, and MVVM React frontend. Use when creating cross-platform apps with Vite + React + Zustand + Tailwind CSS 4.
---

# Tauri 2.0 Desktop App Development

This skill provides guidance for building professional cross-platform desktop applications using Tauri 2.0 with a Rust backend and modern React frontend following MVVM architecture.

## When to Use This Skill

- Building cross-platform desktop apps (Windows, macOS, Linux)
- Migrating from Electron to Tauri for smaller bundle size
- Creating secure, performant native apps with web technologies
- Implementing complex state management between Rust and React

> [!CAUTION]
> **This skill is for Tauri 2.0 only.** Tauri 1.x uses different APIs and configuration.

## Prerequisites

- **Rust**: Install via [rustup](https://rustup.rs/)
- **Node.js**: 18+ LTS
- **Platform tools**:
  - macOS: Xcode Command Line Tools
  - Windows: Visual Studio Build Tools + WebView2
  - Linux: `webkit2gtk`, `libayatana-appindicator`

## Project Setup

### Quick Start

```bash
# Create new project with React + TypeScript
npm create tauri-app@latest my-app -- --template react-ts
cd my-app

# Install frontend dependencies
npm install zustand react-router-dom
npm install tailwindcss @tailwindcss/vite -D

# Run development
npm run tauri dev
```

### Tailwind CSS 4 Setup

```js
// vite.config.ts
import { defineConfig } from "vite";
import react from "@vitejs/plugin-react";
import tailwindcss from "@tailwindcss/vite";

export default defineConfig({
  plugins: [react(), tailwindcss()],
});
```

```css
/* src/styles/globals.css */
@import "tailwindcss";

@theme {
  --color-primary: oklch(0.6 0.2 250);
  --color-secondary: oklch(0.7 0.15 180);
  --font-sans: "Inter", system-ui, sans-serif;
}
```

---

## Rust Backend Architecture

### Module Organization

```
src-tauri/src/
├── main.rs              # Entry point (minimal)
├── lib.rs               # App builder, state/plugin registration
├── commands/            # Tauri commands by feature
│   ├── mod.rs
│   ├── file.rs
│   └── settings.rs
├── services/            # Business logic (pure Rust)
│   ├── mod.rs
│   └── storage.rs
├── models/              # Data structures
│   └── mod.rs
├── state/               # App state management
│   └── mod.rs
├── plugins/             # Custom Tauri plugins
│   └── mod.rs
└── error.rs             # Custom error types
```

### Command Patterns

```rust
// commands/file.rs
use tauri::State;
use crate::{state::AppState, error::AppError};

#[tauri::command]
pub async fn read_file(
    path: String,
    state: State<'_, AppState>,
) -> Result<String, String> {
    std::fs::read_to_string(&path)
        .map_err(|e| format!("Failed to read file: {}", e))
}

#[tauri::command]
pub async fn save_file(
    path: String,
    content: String,
) -> Result<(), String> {
    std::fs::write(&path, &content)
        .map_err(|e| format!("Failed to save file: {}", e))
}
```

### Error Handling

```rust
// error.rs
use serde::Serialize;

#[derive(Debug, Serialize)]
pub enum AppError {
    Io(String),
    Database(String),
    Validation(String),
    NotFound(String),
}

impl From<std::io::Error> for AppError {
    fn from(err: std::io::Error) -> Self {
        AppError::Io(err.to_string())
    }
}

// Convert to Tauri invoke error
impl From<AppError> for tauri::ipc::InvokeError {
    fn from(err: AppError) -> Self {
        tauri::ipc::InvokeError::from(serde_json::to_string(&err).unwrap())
    }
}
```

### State Management

```rust
// state/mod.rs
use std::sync::Mutex;
use serde::{Deserialize, Serialize};

#[derive(Default)]
pub struct AppState {
    pub settings: Mutex<AppSettings>,
    pub cache: Mutex<Vec<String>>,
}

#[derive(Default, Serialize, Deserialize, Clone)]
pub struct AppSettings {
    pub theme: String,
    pub language: String,
}

// lib.rs - Register state
pub fn run() {
    tauri::Builder::default()
        .manage(AppState::default())
        .invoke_handler(tauri::generate_handler![
            commands::file::read_file,
            commands::file::save_file,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
```

---

## Permissions & Security

### Capabilities (Tauri 2.0)

Tauri 2.0 uses a **capability-based security model**. Define permissions in `src-tauri/capabilities/`:

```json
// src-tauri/capabilities/default.json
{
  "$schema": "../gen/schemas/desktop-schema.json",
  "identifier": "default",
  "description": "Default capabilities for the app",
  "windows": ["main"],
  "permissions": [
    "core:default",
    "fs:default",
    "dialog:default",
    "shell:allow-open"
  ]
}
```

### Permission Scopes

```json
// Fine-grained file system access
{
  "permissions": [
    {
      "identifier": "fs:allow-read",
      "allow": [
        { "path": "$APPDATA/**" },
        { "path": "$DOCUMENT/**" }
      ]
    },
    {
      "identifier": "fs:allow-write",
      "allow": [
        { "path": "$APPDATA/**" }
      ]
    }
  ]
}
```

### Security Best Practices

| Practice | Implementation |
|----------|----------------|
| **Minimal permissions** | Only request what you need |
| **Input validation** | Validate all frontend data in Rust |
| **Path traversal prevention** | Use `tauri::path` APIs, not raw strings |
| **No `dangerousRemoteDomainIpcAccess`** | Avoid unless absolutely necessary |
| **CSP headers** | Configure in `tauri.conf.json` |

```json
// tauri.conf.json - Security settings
{
  "app": {
    "security": {
      "csp": "default-src 'self'; img-src 'self' data: https:; style-src 'self' 'unsafe-inline'"
    }
  }
}
```

---

## Plugins

### Official Plugins

Install via npm + Cargo:

```bash
# Dialog plugin
npm install @tauri-apps/plugin-dialog
cargo add tauri-plugin-dialog -F tauri-plugin-dialog/unstable

# File system plugin
npm install @tauri-apps/plugin-fs
cargo add tauri-plugin-fs

# Store plugin (persistent storage)
npm install @tauri-apps/plugin-store
cargo add tauri-plugin-store
```

### Register Plugins

```rust
// lib.rs
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_fs::init())
        .plugin(tauri_plugin_store::Builder::default().build())
        .invoke_handler(tauri::generate_handler![/* commands */])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
```

### Using Plugins in Frontend

```typescript
// Dialog
import { open, save } from '@tauri-apps/plugin-dialog';

const filePath = await open({
  multiple: false,
  filters: [{ name: 'Text', extensions: ['txt', 'md'] }],
});

// File system
import { readTextFile, writeTextFile } from '@tauri-apps/plugin-fs';

const content = await readTextFile(filePath);
await writeTextFile(filePath, newContent);

// Store (persistent key-value)
import { Store } from '@tauri-apps/plugin-store';

const store = await Store.load('settings.json');
await store.set('theme', 'dark');
const theme = await store.get<string>('theme');
```

### Custom Plugin

```rust
// plugins/mod.rs
use tauri::{
    plugin::{Builder, TauriPlugin},
    Runtime,
};

pub fn init<R: Runtime>() -> TauriPlugin<R> {
    Builder::new("my-plugin")
        .invoke_handler(tauri::generate_handler![plugin_command])
        .build()
}

#[tauri::command]
fn plugin_command() -> String {
    "Hello from plugin!".into()
}
```

---

## Build & Distribution

### Development

```bash
npm run tauri dev           # Hot-reload development
npm run tauri dev -- --release  # Test release build
```

### Production Build

```bash
npm run tauri build         # Build for current platform
```

### Build Configuration

```json
// tauri.conf.json
{
  "productName": "My App",
  "version": "1.0.0",
  "identifier": "com.mycompany.myapp",
  "build": {
    "beforeBuildCommand": "npm run build",
    "beforeDevCommand": "npm run dev",
    "devUrl": "http://localhost:5173",
    "frontendDist": "../dist"
  },
  "bundle": {
    "active": true,
    "icon": [
      "icons/32x32.png",
      "icons/128x128.png",
      "icons/icon.icns",
      "icons/icon.ico"
    ],
    "macOS": {
      "minimumSystemVersion": "10.13"
    },
    "windows": {
      "certificateThumbprint": null,
      "timestampUrl": ""
    }
  }
}
```

### Platform-Specific Builds

```bash
# Cross-compile (requires toolchain)
npm run tauri build -- --target x86_64-pc-windows-msvc
npm run tauri build -- --target aarch64-apple-darwin
npm run tauri build -- --target x86_64-unknown-linux-gnu
```

### Auto-Updater

```bash
npm install @tauri-apps/plugin-updater
cargo add tauri-plugin-updater
```

```rust
// lib.rs
.plugin(tauri_plugin_updater::Builder::default().build())
```

```json
// capabilities/default.json
{
  "permissions": ["updater:default"]
}
```

---

## Frontend Architecture (MVVM)

### Folder Structure

```
src/
├── main.tsx                 # Entry point
├── App.tsx                  # Router setup
├── router/                  # Route definitions
│   └── index.tsx
├── views/                   # View layer (pages)
│   ├── Home/
│   │   ├── index.tsx
│   │   └── HomeView.tsx
│   └── Settings/
│       └── index.tsx
├── viewmodels/              # ViewModel layer (hooks)
│   ├── useHomeViewModel.ts
│   └── useSettingsViewModel.ts
├── models/                  # Model layer (types)
│   └── index.ts
├── stores/                  # Zustand stores
│   └── useAppStore.ts
├── services/                # Tauri bridge
│   └── tauriService.ts
├── components/              # Reusable UI
│   └── Button/
├── hooks/                   # Custom hooks
└── styles/
    └── globals.css
```

### MVVM Pattern

| Layer | Responsibility | Example |
|-------|----------------|---------|
| **Model** | Data types, stores | `models/`, `stores/` |
| **View** | UI rendering (dumb) | `views/`, `components/` |
| **ViewModel** | Logic, state binding | `viewmodels/` hooks |

### Zustand Store

```typescript
// stores/useAppStore.ts
import { create } from 'zustand';
import { persist, createJSONStorage } from 'zustand/middleware';

interface AppState {
  theme: 'light' | 'dark';
  sidebarOpen: boolean;
  setTheme: (theme: 'light' | 'dark') => void;
  toggleSidebar: () => void;
}

export const useAppStore = create<AppState>()(
  persist(
    (set) => ({
      theme: 'dark',
      sidebarOpen: true,
      setTheme: (theme) => set({ theme }),
      toggleSidebar: () => set((s) => ({ sidebarOpen: !s.sidebarOpen })),
    }),
    {
      name: 'app-storage',
      storage: createJSONStorage(() => localStorage),
    }
  )
);
```

### Tauri Bridge Service

```typescript
// services/tauriService.ts
import { invoke } from '@tauri-apps/api/core';

export const tauriService = {
  async readFile(path: string): Promise<string> {
    return invoke<string>('read_file', { path });
  },

  async saveFile(path: string, content: string): Promise<void> {
    return invoke('save_file', { path, content });
  },

  async getSettings(): Promise<AppSettings> {
    return invoke<AppSettings>('get_settings');
  },
};
```

### ViewModel Hook

```typescript
// viewmodels/useHomeViewModel.ts
import { useState, useEffect, useCallback } from 'react';
import { tauriService } from '../services/tauriService';
import { useAppStore } from '../stores/useAppStore';

export function useHomeViewModel() {
  const [files, setFiles] = useState<string[]>([]);
  const [loading, setLoading] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const { theme } = useAppStore();

  const loadFiles = useCallback(async () => {
    setLoading(true);
    setError(null);
    try {
      const data = await tauriService.listFiles();
      setFiles(data);
    } catch (e) {
      setError(e instanceof Error ? e.message : 'Unknown error');
    } finally {
      setLoading(false);
    }
  }, []);

  useEffect(() => {
    loadFiles();
  }, [loadFiles]);

  return { files, loading, error, theme, loadFiles };
}
```

### View Component

```tsx
// views/Home/HomeView.tsx
import { useHomeViewModel } from '../../viewmodels/useHomeViewModel';

export function HomeView() {
  const { files, loading, error, loadFiles } = useHomeViewModel();

  if (loading) return <div className="animate-pulse">Loading...</div>;
  if (error) return <div className="text-error">{error}</div>;

  return (
    <div className="p-4">
      <h1 className="text-2xl font-bold text-primary">Files</h1>
      <ul className="mt-4 space-y-2">
        {files.map((file) => (
          <li key={file} className="p-2 bg-surface rounded">
            {file}
          </li>
        ))}
      </ul>
      <button onClick={loadFiles} className="mt-4 btn-primary">
        Refresh
      </button>
    </div>
  );
}
```

### React Router Setup

```tsx
// router/index.tsx
import { createBrowserRouter, RouterProvider } from 'react-router-dom';
import { lazy, Suspense } from 'react';

const Home = lazy(() => import('../views/Home'));
const Settings = lazy(() => import('../views/Settings'));

const router = createBrowserRouter([
  { path: '/', element: <Home /> },
  { path: '/settings', element: <Settings /> },
]);

export function AppRouter() {
  return (
    <Suspense fallback={<div>Loading...</div>}>
      <RouterProvider router={router} />
    </Suspense>
  );
}
```

---

## Decision Tree

```
What do you need?
├── Create new project
│   └── npm create tauri-app@latest -- --template react-ts
├── Add Rust command
│   └── Create in commands/, register in lib.rs
├── Add plugin
│   ├── Official → npm install + cargo add
│   └── Custom → Create in plugins/
├── Manage permissions
│   └── Edit capabilities/*.json
├── Manage frontend state
│   └── Use Zustand stores/
├── Call Rust from React
│   └── Use tauriService bridge
└── Build for production
    └── npm run tauri build
```

## Common Pitfalls

| Issue | Solution |
|-------|----------|
| Commands not found | Register in `generate_handler![]` |
| Permission denied | Add to capabilities/*.json |
| State not updating | Check Mutex lock is released |
| Build fails on CI | Install platform dependencies |
| Large bundle size | Enable `strip` and `lto` in Cargo.toml |

## Resources

### Examples

- [Project Structure](./examples/project-structure.md) - Recommended folder layout
- **Rust Backend:**
  - [Commands](./examples/rust-backend/commands.md) - Command patterns
  - [State](./examples/rust-backend/state.md) - State management
  - [Error Handling](./examples/rust-backend/error-handling.md) - Error types
- **React Frontend:**
  - [MVVM Structure](./examples/react-frontend/mvvm-structure.md) - Architecture guide
  - [Zustand Patterns](./examples/react-frontend/stores/zustand-patterns.md) - Store patterns
  - [Tauri Hooks](./examples/react-frontend/hooks/tauri-hooks.md) - Custom hooks

### Templates

- [tauri.conf.json](./resources/tauri-config-example.json) - Config template

### External Resources

- [Tauri 2.0 Docs](https://v2.tauri.app/)
- [Tauri Plugins](https://v2.tauri.app/plugin/)
