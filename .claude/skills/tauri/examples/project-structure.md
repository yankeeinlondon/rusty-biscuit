# Recommended Project Structure

## Full Structure

```
my-tauri-app/
├── src-tauri/                    # Rust backend
│   ├── src/
│   │   ├── main.rs               # Entry point
│   │   ├── lib.rs                # App builder
│   │   ├── commands/             # Tauri commands
│   │   │   ├── mod.rs
│   │   │   ├── file.rs
│   │   │   └── settings.rs
│   │   ├── services/             # Business logic
│   │   │   ├── mod.rs
│   │   │   └── storage.rs
│   │   ├── models/               # Data structures
│   │   │   └── mod.rs
│   │   ├── state/                # App state
│   │   │   └── mod.rs
│   │   ├── plugins/              # Custom plugins
│   │   │   └── mod.rs
│   │   └── error.rs              # Error types
│   ├── capabilities/             # Permissions
│   │   └── default.json
│   ├── icons/                    # App icons
│   ├── Cargo.toml
│   └── tauri.conf.json           # Tauri config
│
├── src/                          # React frontend
│   ├── main.tsx                  # Entry point
│   ├── App.tsx                   # Root component
│   ├── router/                   # Routing
│   │   └── index.tsx
│   ├── views/                    # Pages (View)
│   │   ├── Home/
│   │   │   ├── index.tsx
│   │   │   └── HomeView.tsx
│   │   └── Settings/
│   │       └── index.tsx
│   ├── viewmodels/               # Logic hooks (ViewModel)
│   │   ├── useHomeViewModel.ts
│   │   └── useSettingsViewModel.ts
│   ├── models/                   # Types (Model)
│   │   └── index.ts
│   ├── stores/                   # Zustand stores
│   │   └── useAppStore.ts
│   ├── services/                 # Tauri bridge
│   │   └── tauriService.ts
│   ├── components/               # Reusable UI
│   │   ├── Button/
│   │   ├── Input/
│   │   └── Layout/
│   ├── hooks/                    # Custom hooks
│   │   └── useDebounce.ts
│   └── styles/
│       └── globals.css
│
├── public/                       # Static assets
├── index.html
├── package.json
├── vite.config.ts
└── tsconfig.json
```

## Key Principles

| Area | Principle |
|------|-----------|
| **Rust modules** | One file per feature domain |
| **Commands** | Thin layer, delegate to services |
| **Services** | Pure Rust, no Tauri dependencies |
| **Views** | Presentation only, no logic |
| **ViewModels** | All business logic as hooks |
| **Stores** | Global state with Zustand |
| **Services (TS)** | Single point for Tauri IPC |
