# Custom React Hooks for Tauri

Custom React hooks for Tauri apps.

**Path:** `src/hooks/`

## useTauriCommand - Generic Command Hook

```typescript
import { invoke } from '@tauri-apps/api/core';
import { useCallback, useState } from 'react';

interface UseTauriCommandOptions<T> {
  onSuccess?: (data: T) => void;
  onError?: (error: Error) => void;
}

export function useTauriCommand<T, A extends Record<string, unknown>>(
  command: string,
  options?: UseTauriCommandOptions<T>
) {
  const [data, setData] = useState<T | null>(null);
  const [loading, setLoading] = useState(false);
  const [error, setError] = useState<Error | null>(null);

  const execute = useCallback(async (args?: A) => {
    setLoading(true);
    setError(null);

    try {
      const result = await invoke<T>(command, args);
      setData(result);
      options?.onSuccess?.(result);
      return result;
    } catch (e) {
      const err = e instanceof Error ? e : new Error(String(e));
      setError(err);
      options?.onError?.(err);
      throw err;
    } finally {
      setLoading(false);
    }
  }, [command, options]);

  const reset = useCallback(() => {
    setData(null);
    setError(null);
    setLoading(false);
  }, []);

  return { data, loading, error, execute, reset };
}

// Usage:
// const { data, loading, execute } = useTauriCommand<string, { path: string }>('read_file');
// await execute({ path: '/some/file.txt' });
```

## useDebounce - Debounced Value

```typescript
import { useEffect, useState } from 'react';

export function useDebounce<T>(value: T, delay: number): T {
  const [debouncedValue, setDebouncedValue] = useState(value);

  useEffect(() => {
    const timer = setTimeout(() => {
      setDebouncedValue(value);
    }, delay);

    return () => {
      clearTimeout(timer);
    };
  }, [value, delay]);

  return debouncedValue;
}

// Usage:
// const searchTerm = useDebounce(inputValue, 300);
```

## useLocalStorage - Persisted State

```typescript
import { useCallback, useState } from 'react';

export function useLocalStorage<T>(
  key: string,
  initialValue: T
): [T, (value: T | ((prev: T) => T)) => void] {
  const [storedValue, setStoredValue] = useState<T>(() => {
    try {
      const item = localStorage.getItem(key);
      return item ? JSON.parse(item) : initialValue;
    } catch {
      return initialValue;
    }
  });

  const setValue = useCallback((value: T | ((prev: T) => T)) => {
    setStoredValue(prev => {
      const newValue = value instanceof Function ? value(prev) : value;
      localStorage.setItem(key, JSON.stringify(newValue));
      return newValue;
    });
  }, [key]);

  return [storedValue, setValue];
}
```

## useKeyboardShortcut - Keyboard Shortcuts

```typescript
import { useEffect } from 'react';

type KeyModifiers = {
  ctrl?: boolean;
  alt?: boolean;
  shift?: boolean;
  meta?: boolean;
};

export function useKeyboardShortcut(
  key: string,
  callback: () => void,
  modifiers: KeyModifiers = {}
) {
  useEffect(() => {
    const handler = (event: KeyboardEvent) => {
      const matchesModifiers =
        (modifiers.ctrl === undefined || event.ctrlKey === modifiers.ctrl) &&
        (modifiers.alt === undefined || event.altKey === modifiers.alt) &&
        (modifiers.shift === undefined || event.shiftKey === modifiers.shift) &&
        (modifiers.meta === undefined || event.metaKey === modifiers.meta);

      if (event.key.toLowerCase() === key.toLowerCase() && matchesModifiers) {
        event.preventDefault();
        callback();
      }
    };

    window.addEventListener('keydown', handler);
    return () => window.removeEventListener('keydown', handler);
  }, [key, callback, modifiers]);
}

// Usage:
// useKeyboardShortcut('s', saveFile, { ctrl: true });
// useKeyboardShortcut('Escape', closeModal);
```

## useWindowSize - Window Dimensions

```typescript
import { useEffect, useState } from 'react';

interface WindowSize {
  width: number;
  height: number;
}

export function useWindowSize(): WindowSize {
  const [size, setSize] = useState<WindowSize>({
    width: window.innerWidth,
    height: window.innerHeight,
  });

  useEffect(() => {
    const handleResize = () => {
      setSize({
        width: window.innerWidth,
        height: window.innerHeight,
      });
    };

    window.addEventListener('resize', handleResize);
    return () => window.removeEventListener('resize', handleResize);
  }, []);

  return size;
}
```

## useTauriEvent - Listen to Tauri Events

```typescript
import { listen, type UnlistenFn } from '@tauri-apps/api/event';
import { useEffect } from 'react';

export function useTauriEvent<T>(
  event: string,
  callback: (payload: T) => void
) {
  useEffect(() => {
    let unlisten: UnlistenFn | undefined;

    listen<T>(event, (e) => {
      callback(e.payload);
    }).then((fn) => {
      unlisten = fn;
    });

    return () => {
      unlisten?.();
    };
  }, [event, callback]);
}

// Usage:
// useTauriEvent('file-changed', (path: string) => {
//   console.log('File changed:', path);
//   reloadFile(path);
// });
```
