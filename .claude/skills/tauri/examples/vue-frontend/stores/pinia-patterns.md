# Pinia Store Patterns

Pinia store patterns for Tauri apps. These examples use **setup stores**
(composition-style) — the most idiomatic style for modern Vue.

**Path:** `src/stores/useAppStore.ts`

> Register Pinia and the persistence plugin once in `main.ts`:
>
> ```typescript
> import { createPinia } from 'pinia';
> import piniaPluginPersistedstate from 'pinia-plugin-persistedstate';
>
> const pinia = createPinia();
> pinia.use(piniaPluginPersistedstate);
> createApp(App).use(pinia).mount('#app');
> ```

## Basic Store with Persist

```typescript
import { defineStore } from 'pinia';
import { ref } from 'vue';

export const useAppStore = defineStore(
  'app',
  () => {
    // State
    const theme = ref<'light' | 'dark' | 'system'>('dark');
    const sidebarOpen = ref(true);
    const recentFiles = ref<string[]>([]);

    // Actions
    function setTheme(value: 'light' | 'dark' | 'system') {
      theme.value = value;
    }

    function toggleSidebar() {
      sidebarOpen.value = !sidebarOpen.value;
    }

    function addRecentFile(path: string) {
      const files = [path, ...recentFiles.value.filter((f) => f !== path)];
      recentFiles.value = files.slice(0, 10); // Keep last 10
    }

    function clearRecentFiles() {
      recentFiles.value = [];
    }

    return {
      theme,
      sidebarOpen,
      recentFiles,
      setTheme,
      toggleSidebar,
      addRecentFile,
      clearRecentFiles,
    };
  },
  {
    persist: {
      key: 'app-storage',
      // Persist only a subset of state (the rest stays in-memory)
      pick: ['theme', 'recentFiles'],
    },
  },
);
```

## Store with Complex Nested State

Pinia mutates reactive state **directly** — Vue's proxy reactivity tracks nested
changes, so no Immer-style immutable-update helper is needed.

```typescript
import { defineStore } from 'pinia';
import { ref } from 'vue';
import type { FileInfo } from '../models';

interface Tab {
  id: string;
  path: string;
  name: string;
  content: string;
  isDirty: boolean;
}

export const useEditorStore = defineStore('editor', () => {
  const tabs = ref<Tab[]>([]);
  const activeTabId = ref<string | null>(null);

  function openTab(file: FileInfo & { content: string }) {
    const existing = tabs.value.find((t) => t.path === file.path);
    if (existing) {
      activeTabId.value = existing.id;
      return;
    }

    const newTab: Tab = {
      id: crypto.randomUUID(),
      path: file.path,
      name: file.name,
      content: file.content,
      isDirty: false,
    };

    tabs.value.push(newTab);
    activeTabId.value = newTab.id;
  }

  function closeTab(id: string) {
    const index = tabs.value.findIndex((t) => t.id === id);
    if (index === -1) return;

    tabs.value.splice(index, 1);

    if (activeTabId.value === id) {
      activeTabId.value = tabs.value[Math.max(0, index - 1)]?.id ?? null;
    }
  }

  function setActiveTab(id: string) {
    activeTabId.value = id;
  }

  function updateTabContent(id: string, content: string) {
    const tab = tabs.value.find((t) => t.id === id);
    if (tab) {
      // Direct mutation — Vue reactivity handles it
      tab.content = content;
      tab.isDirty = true;
    }
  }

  return {
    tabs,
    activeTabId,
    openTab,
    closeTab,
    setActiveTab,
    updateTabContent,
  };
});
```

## Async Store with Tauri Integration

```typescript
import { defineStore } from 'pinia';
import { ref } from 'vue';
import { tauriService } from '../services/tauriService';

interface Settings {
  theme: string;
  fontSize: number;
  autoSave: boolean;
}

export const useSettingsStore = defineStore('settings', () => {
  const settings = ref<Settings | null>(null);
  const loading = ref(false);
  const error = ref<string | null>(null);

  async function fetchSettings() {
    loading.value = true;
    error.value = null;
    try {
      settings.value = await tauriService.getSettings();
    } catch (e) {
      error.value = e instanceof Error ? e.message : 'Failed to fetch settings';
    } finally {
      loading.value = false;
    }
  }

  async function updateSettings(partial: Partial<Settings>) {
    if (!settings.value) return;

    const previous = { ...settings.value };
    settings.value = { ...settings.value, ...partial }; // optimistic

    try {
      await tauriService.updateSettings(settings.value);
    } catch (e) {
      settings.value = previous; // Rollback on error
      throw e;
    }
  }

  return { settings, loading, error, fetchSettings, updateSettings };
});
```

## Computed / Derived State (Getters)

In a setup store, a getter is just a `computed`:

```typescript
import { defineStore } from 'pinia';
import { computed, ref } from 'vue';

export const useEditorStore = defineStore('editor', () => {
  const tabs = ref<Tab[]>([]);
  const activeTabId = ref<string | null>(null);

  const activeTab = computed(() =>
    tabs.value.find((t) => t.id === activeTabId.value) ?? null,
  );
  const hasUnsaved = computed(() => tabs.value.some((t) => t.isDirty));

  return { tabs, activeTabId, activeTab, hasUnsaved };
});
```

## Usage in Components

```vue
<script setup lang="ts">
import { storeToRefs } from 'pinia';
import { useAppStore } from './stores/useAppStore';
import { useEditorStore } from './stores/useEditorStore';

const appStore = useAppStore();
// State + getters → use storeToRefs to keep reactivity when destructuring
const { theme, sidebarOpen } = storeToRefs(appStore);
// Actions → destructure directly (they are plain functions, not reactive)
const { setTheme, toggleSidebar } = appStore;

const editorStore = useEditorStore();
const { tabs, activeTab } = storeToRefs(editorStore);
const { openTab } = editorStore;
</script>
```

> **Key rule:** destructuring state/getters off a store loses reactivity. Wrap
> them in `storeToRefs()`. Actions are safe to destructure directly. (This is the
> Pinia counterpart of choosing a precise selector in other state libraries.)
