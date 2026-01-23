// Debounce hook for delayed value updates
import { onDestroy } from 'svelte';

export function useDebounce<T>(getValue: () => T, delay: number = 100) {
  let debouncedValue = $state<T>(getValue());
  let timeout: ReturnType<typeof setTimeout> | null = null;

  $effect(() => {
    const value = getValue();
    if (timeout) clearTimeout(timeout);
    timeout = setTimeout(() => {
      debouncedValue = value;
    }, delay);
  });

  // MEMORY LEAK FIX
  onDestroy(() => {
    if (timeout) clearTimeout(timeout);
  });

  return {
    get value() { return debouncedValue; },
  };
}
