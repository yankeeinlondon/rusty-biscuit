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
├── src/                          # Vue frontend
│   ├── main.ts                   # Entry point
│   ├── App.vue                   # Root component
│   ├── router/                   # Routing
│   │   └── index.ts
│   ├── views/                    # Pages (View)
│   │   ├── Home/
│   │   │   └── HomeView.vue
│   │   └── Settings/
│   │       └── SettingsView.vue
│   ├── viewmodels/               # Logic composables (ViewModel)
│   │   ├── useHomeViewModel.ts
│   │   └── useSettingsViewModel.ts
│   ├── models/                   # Types (Model)
│   │   └── index.ts
│   ├── stores/                   # Pinia stores
│   │   └── useAppStore.ts
│   ├── services/                 # Tauri bridge
│   │   └── tauriService.ts
│   ├── components/               # Reusable UI (.vue)
│   │   ├── BaseButton.vue
│   │   ├── BaseInput.vue
│   │   └── AppLayout.vue
│   ├── composables/              # Generic composables
│   │   └── useDebounce.ts
│   └── styles/
│       └── globals.css
│
├── public/                       # Static assets
├── index.html
├── package.json
├── uno.config.ts                 # UnoCSS config
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
| **ViewModels** | All business logic as composables |
| **Stores** | Global state with Pinia |
| **Services (TS)** | Single point for Tauri IPC |
