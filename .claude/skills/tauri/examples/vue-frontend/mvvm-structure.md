# MVVM Architecture for Vue Frontend

## Overview

MVVM (Model-View-ViewModel) separates concerns into three layers:

| Layer | Responsibility | Location |
|-------|----------------|----------|
| **Model** | Data, types, stores | `models/`, `stores/` |
| **View** | UI rendering (dumb) | `views/`, `components/` |
| **ViewModel** | Business logic | `viewmodels/` composables |

Vue is inherently MVVM — the reactive component instance created by
`<script setup>` is the ViewModel for its template. This skill keeps an explicit
`viewmodels/` layer so view logic can be unit tested and reused without mounting
a component.

## Data Flow

```
┌─────────────────────────────────────────────────┐
│                    View                         │
│  (.vue component - template + minimal script)   │
│                      │                          │
│              useXxxViewModel()                  │
│                      ▼                          │
├─────────────────────────────────────────────────┤
│                 ViewModel                       │
│  (Composable - business logic, reactive state)  │
│         │                    │                  │
│    useAppStore()      tauriService              │
│         ▼                    ▼                  │
├─────────────────────────────────────────────────┤
│                    Model                        │
│  (Pinia stores, types, Tauri bridge)            │
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

### 2. Store Layer (Pinia)

```typescript
// stores/useAppStore.ts
import { defineStore } from 'pinia';
import { ref } from 'vue';
import type { AppSettings } from '../models';

export const useAppStore = defineStore(
  'app',
  () => {
    const settings = ref<AppSettings>({
      theme: 'dark',
      language: 'en',
      autoSave: true,
    });

    function updateSettings(partial: Partial<AppSettings>) {
      settings.value = { ...settings.value, ...partial };
    }

    return { settings, updateSettings };
  },
  {
    persist: { key: 'app-settings' },
  },
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

### 4. ViewModel Layer (Composables)

```typescript
// viewmodels/useFileExplorerViewModel.ts
import { ref, toValue, watch, type MaybeRefOrGetter } from 'vue';
import { tauriService } from '../services/tauriService';
import type { FileInfo } from '../models';

export function useFileExplorerViewModel(initialDir: MaybeRefOrGetter<string>) {
  const files = ref<FileInfo[]>([]);
  const currentDir = ref(toValue(initialDir));
  const loading = ref(false);
  const error = ref<string | null>(null);

  async function loadFiles(dir: string) {
    loading.value = true;
    error.value = null;
    try {
      files.value = await tauriService.listFiles(dir);
      currentDir.value = dir;
    } catch (e) {
      error.value = e instanceof Error ? e.message : 'Failed to load files';
    } finally {
      loading.value = false;
    }
  }

  function navigateTo(dir: string) {
    loadFiles(dir);
  }

  function refresh() {
    loadFiles(currentDir.value);
  }

  // Initial load + react to a changing initialDir
  watch(() => toValue(initialDir), loadFiles, { immediate: true });

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

### 5. View Layer (Vue Components)

```vue
<!-- views/FileExplorer/FileExplorerView.vue -->
<script setup lang="ts">
import { useFileExplorerViewModel } from '../../viewmodels/useFileExplorerViewModel';
import FileList from '../../components/FileList.vue';
import LoadingSpinner from '../../components/LoadingSpinner.vue';
import ErrorMessage from '../../components/ErrorMessage.vue';
import type { FileInfo } from '../../models';

const props = defineProps<{ initialDir: string }>();

const { files, currentDir, loading, error, navigateTo, refresh } =
  useFileExplorerViewModel(() => props.initialDir);

function onFileClick(file: FileInfo) {
  if (file.isDir) navigateTo(file.path);
}
</script>

<template>
  <LoadingSpinner v-if="loading" />
  <ErrorMessage v-else-if="error" :message="error" @retry="refresh" />
  <div v-else class="p-4">
    <div class="flex items-center justify-between mb-4">
      <h1 class="text-xl font-semibold">{{ currentDir }}</h1>
      <button class="btn-secondary" @click="refresh">Refresh</button>
    </div>

    <FileList :files="files" @file-click="onFileClick" />
  </div>
</template>
```

## Benefits

| Benefit | Description |
|---------|-------------|
| **Testability** | ViewModels are plain composables — test without mounting |
| **Reusability** | Same ViewModel for different Views |
| **Separation** | Clear boundaries between layers |
| **Maintainability** | Changes in one layer don't affect others |
