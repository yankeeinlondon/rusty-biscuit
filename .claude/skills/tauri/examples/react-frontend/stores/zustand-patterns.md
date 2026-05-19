# Zustand Store Patterns

Zustand store patterns for Tauri apps.

**Path:** `src/stores/useAppStore.ts`

## Basic Store with Persist

```typescript
import { create } from 'zustand';
import { createJSONStorage, persist } from 'zustand/middleware';

interface AppState {
  theme: 'light' | 'dark' | 'system';
  sidebarOpen: boolean;
  recentFiles: string[];

  // Actions
  setTheme: (theme: 'light' | 'dark' | 'system') => void;
  toggleSidebar: () => void;
  addRecentFile: (path: string) => void;
  clearRecentFiles: () => void;
}

export const useAppStore = create<AppState>()(
  persist(
    (set, get) => ({
      // State
      theme: 'dark',
      sidebarOpen: true,
      recentFiles: [],

      // Actions
      setTheme: (theme) => set({ theme }),

      toggleSidebar: () => set((state) => ({
        sidebarOpen: !state.sidebarOpen
      })),

      addRecentFile: (path) => set((state) => {
        const files = [path, ...state.recentFiles.filter(f => f !== path)];
        return { recentFiles: files.slice(0, 10) }; // Keep last 10
      }),

      clearRecentFiles: () => set({ recentFiles: [] }),
    }),
    {
      name: 'app-storage',
      storage: createJSONStorage(() => localStorage),
      partialize: (state) => ({
        theme: state.theme,
        recentFiles: state.recentFiles,
      }),
    }
  )
);
```

## Store with Immer for Complex State

```typescript
import { create } from 'zustand';
import { immer } from 'zustand/middleware/immer';

interface Tab {
  id: string;
  path: string;
  name: string;
  content: string;
  isDirty: boolean;
}

interface EditorState {
  tabs: Tab[];
  activeTabId: string | null;

  openTab: (file: FileInfo) => void;
  closeTab: (id: string) => void;
  setActiveTab: (id: string) => void;
  updateTabContent: (id: string, content: string) => void;
}

export const useEditorStore = create<EditorState>()(
  immer((set) => ({
    tabs: [],
    activeTabId: null,

    openTab: (file) => set((state) => {
      const existing = state.tabs.find(t => t.path === file.path);
      if (existing) {
        state.activeTabId = existing.id;
        return;
      }

      const newTab: Tab = {
        id: crypto.randomUUID(),
        path: file.path,
        name: file.name,
        content: file.content,
        isDirty: false,
      };

      state.tabs.push(newTab);
      state.activeTabId = newTab.id;
    }),

    closeTab: (id) => set((state) => {
      const index = state.tabs.findIndex(t => t.id === id);
      if (index === -1) return;

      state.tabs.splice(index, 1);

      if (state.activeTabId === id) {
        state.activeTabId = state.tabs[Math.max(0, index - 1)]?.id ?? null;
      }
    }),

    setActiveTab: (id) => set((state) => {
      state.activeTabId = id;
    }),

    updateTabContent: (id, content) => set((state) => {
      const tab = state.tabs.find(t => t.id === id);
      if (tab) {
        tab.content = content;
        tab.isDirty = true;
      }
    }),
  }))
);
```

## Async Store with Tauri Integration

```typescript
import { create } from 'zustand';
import { tauriService } from '../services/tauriService';

interface Settings {
  theme: string;
  fontSize: number;
  autoSave: boolean;
}

interface SettingsState {
  settings: Settings | null;
  loading: boolean;
  error: string | null;

  fetchSettings: () => Promise<void>;
  updateSettings: (partial: Partial<Settings>) => Promise<void>;
}

export const useSettingsStore = create<SettingsState>()((set, get) => ({
  settings: null,
  loading: false,
  error: null,

  fetchSettings: async () => {
    set({ loading: true, error: null });
    try {
      const settings = await tauriService.getSettings();
      set({ settings, loading: false });
    } catch (e) {
      set({
        error: e instanceof Error ? e.message : 'Failed to fetch settings',
        loading: false
      });
    }
  },

  updateSettings: async (partial) => {
    const current = get().settings;
    if (!current) return;

    const newSettings = { ...current, ...partial };
    set({ settings: newSettings });

    try {
      await tauriService.updateSettings(newSettings);
    } catch (e) {
      // Rollback on error
      set({ settings: current });
      throw e;
    }
  },
}));
```

## Usage in Components

```typescript
import { useAppStore } from './stores/useAppStore';
import { useEditorStore } from './stores/useEditorStore';

function Component() {
  // Select specific state to avoid re-renders
  const theme = useAppStore((s) => s.theme);
  const setTheme = useAppStore((s) => s.setTheme);
  
  // Or destructure for multiple values
  const { tabs, activeTabId, openTab } = useEditorStore();
}
```
