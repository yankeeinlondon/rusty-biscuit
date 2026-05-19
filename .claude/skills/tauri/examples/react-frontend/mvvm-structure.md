# MVVM Architecture for React Frontend

## Overview

MVVM (Model-View-ViewModel) separates concerns into three layers:

| Layer | Responsibility | Location |
|-------|----------------|----------|
| **Model** | Data, types, stores | `models/`, `stores/` |
| **View** | UI rendering (dumb) | `views/`, `components/` |
| **ViewModel** | Business logic | `viewmodels/` hooks |

## Data Flow

```
┌─────────────────────────────────────────────────┐
│                    View                         │
│  (React component - presentation only)          │
│                      │                          │
│              useXxxViewModel()                  │
│                      ▼                          │
├─────────────────────────────────────────────────┤
│                 ViewModel                       │
│  (Custom hook - business logic, state binding)  │
│         │                    │                  │
│    useAppStore()      tauriService              │
│         ▼                    ▼                  │
├─────────────────────────────────────────────────┤
│                    Model                        │
│  (Zustand stores, types, Tauri bridge)          │
└─────────────────────────────────────────────────┘
```

## Implementation

### 1. Model Layer

```typescript
// models/index.ts
export interface User {
  id: string;
  name: string;
  email: string;
}

export interface FileInfo {
  path: string;
  name: string;
  size: number;
  isDir: boolean;
}

export interface AppSettings {
  theme: 'light' | 'dark';
  language: string;
  autoSave: boolean;
}
```

### 2. Store Layer (Zustand)

```typescript
// stores/useAppStore.ts
import { create } from 'zustand';
import { persist } from 'zustand/middleware';
import type { AppSettings } from '../models';

interface AppState {
  settings: AppSettings;
  updateSettings: (partial: Partial<AppSettings>) => void;
}

export const useAppStore = create<AppState>()(
  persist(
    (set) => ({
      settings: {
        theme: 'dark',
        language: 'en',
        autoSave: true,
      },
      updateSettings: (partial) =>
        set((state) => ({
          settings: { ...state.settings, ...partial },
        })),
    }),
    { name: 'app-settings' }
  )
);
```

### 3. Service Layer (Tauri Bridge)

```typescript
// services/tauriService.ts
import { invoke } from '@tauri-apps/api/core';
import type { FileInfo, AppSettings } from '../models';

export const tauriService = {
  // File operations
  async listFiles(dir: string): Promise<FileInfo[]> {
    return invoke<FileInfo[]>('list_files', { dir });
  },

  async readFile(path: string): Promise<string> {
    return invoke<string>('read_file', { path });
  },

  async saveFile(path: string, content: string): Promise<void> {
    return invoke('save_file', { path, content });
  },

  // Settings
  async getSettings(): Promise<AppSettings> {
    return invoke<AppSettings>('get_settings');
  },

  async updateSettings(settings: AppSettings): Promise<void> {
    return invoke('update_settings', { newSettings: settings });
  },
};
```

### 4. ViewModel Layer (Custom Hooks)

```typescript
// viewmodels/useFileExplorerViewModel.ts
import { useState, useCallback, useEffect } from 'react';
import { tauriService } from '../services/tauriService';
import type { FileInfo } from '../models';

export function useFileExplorerViewModel(initialDir: string) {
  const [files, setFiles] = useState<FileInfo[]>([]);
  const [currentDir, setCurrentDir] = useState(initialDir);
  const [loading, setLoading] = useState(false);
  const [error, setError] = useState<string | null>(null);

  const loadFiles = useCallback(async (dir: string) => {
    setLoading(true);
    setError(null);
    try {
      const data = await tauriService.listFiles(dir);
      setFiles(data);
      setCurrentDir(dir);
    } catch (e) {
      setError(e instanceof Error ? e.message : 'Failed to load files');
    } finally {
      setLoading(false);
    }
  }, []);

  const navigateTo = useCallback((dir: string) => {
    loadFiles(dir);
  }, [loadFiles]);

  const refresh = useCallback(() => {
    loadFiles(currentDir);
  }, [currentDir, loadFiles]);

  useEffect(() => {
    loadFiles(initialDir);
  }, [initialDir, loadFiles]);

  return {
    files,
    currentDir,
    loading,
    error,
    navigateTo,
    refresh,
  };
}
```

### 5. View Layer (React Components)

```tsx
// views/FileExplorer/FileExplorerView.tsx
import { useFileExplorerViewModel } from '../../viewmodels/useFileExplorerViewModel';
import { FileList } from '../../components/FileList';
import { LoadingSpinner } from '../../components/LoadingSpinner';
import { ErrorMessage } from '../../components/ErrorMessage';

interface Props {
  initialDir: string;
}

export function FileExplorerView({ initialDir }: Props) {
  const {
    files,
    currentDir,
    loading,
    error,
    navigateTo,
    refresh,
  } = useFileExplorerViewModel(initialDir);

  if (loading) return <LoadingSpinner />;
  if (error) return <ErrorMessage message={error} onRetry={refresh} />;

  return (
    <div className="p-4">
      <div className="flex items-center justify-between mb-4">
        <h1 className="text-xl font-semibold">{currentDir}</h1>
        <button onClick={refresh} className="btn-secondary">
          Refresh
        </button>
      </div>
      
      <FileList
        files={files}
        onFileClick={(file) => {
          if (file.isDir) {
            navigateTo(file.path);
          }
        }}
      />
    </div>
  );
}
```

## Benefits

| Benefit | Description |
|---------|-------------|
| **Testability** | ViewModels can be unit tested without UI |
| **Reusability** | Same ViewModel for different Views |
| **Separation** | Clear boundaries between layers |
| **Maintainability** | Changes in one layer don't affect others |
