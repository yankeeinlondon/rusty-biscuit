# Custom Composables for Tauri

Reusable Vue composables for Tauri apps.

**Path:** `src/composables/`

## useTauriCommand - Generic Command Composable

```typescript
import { invoke } from '@tauri-apps/api/core';
import { ref, shallowRef } from 'vue';

interface UseTauriCommandOptions<T> {
  onSuccess?: (data: T) => void;
  onError?: (error: Error) => void;
}

export function useTauriCommand<T, A extends Record<string, unknown>>(
  command: string,
  options?: UseTauriCommandOptions<T>,
) {
  const data = shallowRef<T | null>(null);
  const loading = ref(false);
  const error = shallowRef<Error | null>(null);

  async function execute(args?: A) {
    loading.value = true;
    error.value = null;

    try {
      const result = await invoke<T>(command, args);
      data.value = result;
      options?.onSuccess?.(result);
      return result;
    } catch (e) {
      const err = e instanceof Error ? e : new Error(String(e));
      error.value = err;
      options?.onError?.(err);
      throw err;
    } finally {
      loading.value = false;
    }
  }

  function reset() {
    data.value = null;
    error.value = null;
    loading.value = false;
  }

  return { data, loading, error, execute, reset };
}

// Usage:
// const { data, loading, execute } = useTauriCommand<string, { path: string }>('read_file');
// await execute({ path: '/some/file.txt' });
```

## useDebounce - Debounced Value

```typescript
import { ref, watch, type Ref } from 'vue';

export function useDebounce<T>(source: Ref<T>, delay: number): Ref<T> {
  const debounced = ref(source.value) as Ref<T>;
  let timer: ReturnType<typeof setTimeout>;

  watch(source, (value) => {
    clearTimeout(timer);
    timer = setTimeout(() => {
      debounced.value = value;
    }, delay);
  });

  return debounced;
}

// Usage:
// const searchTerm = useDebounce(inputValue, 300);
```

## useLocalStorage - Persisted State

```typescript
import { ref, watch, type Ref } from 'vue';

export function useLocalStorage<T>(key: string, initialValue: T): Ref<T> {
  const stored = localStorage.getItem(key);
  const state = ref<T>(
    stored ? (JSON.parse(stored) as T) : initialValue,
  ) as Ref<T>;

  watch(
    state,
    (value) => {
      localStorage.setItem(key, JSON.stringify(value));
    },
    { deep: true },
  );

  return state;
}
```

## useKeyboardShortcut - Keyboard Shortcuts

```typescript
import { onMounted, onUnmounted } from 'vue';

interface KeyModifiers {
  ctrl?: boolean;
  alt?: boolean;
  shift?: boolean;
  meta?: boolean;
}

export function useKeyboardShortcut(
  key: string,
  callback: () => void,
  modifiers: KeyModifiers = {},
) {
  function handler(event: KeyboardEvent) {
    const matchesModifiers =
      (modifiers.ctrl === undefined || event.ctrlKey === modifiers.ctrl) &&
      (modifiers.alt === undefined || event.altKey === modifiers.alt) &&
      (modifiers.shift === undefined || event.shiftKey === modifiers.shift) &&
      (modifiers.meta === undefined || event.metaKey === modifiers.meta);

    if (event.key.toLowerCase() === key.toLowerCase() && matchesModifiers) {
      event.preventDefault();
      callback();
    }
  }

  onMounted(() => window.addEventListener('keydown', handler));
  onUnmounted(() => window.removeEventListener('keydown', handler));
}

// Usage (inside <script setup>):
// useKeyboardShortcut('s', saveFile, { ctrl: true });
// useKeyboardShortcut('Escape', closeModal);
```

## useWindowSize - Window Dimensions

```typescript
import { onMounted, onUnmounted, ref } from 'vue';

export function useWindowSize() {
  const width = ref(window.innerWidth);
  const height = ref(window.innerHeight);

  function handleResize() {
    width.value = window.innerWidth;
    height.value = window.innerHeight;
  }

  onMounted(() => window.addEventListener('resize', handleResize));
  onUnmounted(() => window.removeEventListener('resize', handleResize));

  return { width, height };
}
```

## useTauriEvent - Listen to Tauri Events

```typescript
import { listen, type UnlistenFn } from '@tauri-apps/api/event';
import { onMounted, onUnmounted } from 'vue';

export function useTauriEvent<T>(
  event: string,
  callback: (payload: T) => void,
) {
  let unlisten: UnlistenFn | undefined;

  onMounted(async () => {
    unlisten = await listen<T>(event, (e) => callback(e.payload));
  });

  onUnmounted(() => unlisten?.());
}

// Usage (inside <script setup>):
// useTauriEvent('file-changed', (path: string) => {
//   console.log('File changed:', path);
//   reloadFile(path);
// });
```
