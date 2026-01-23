// Shared keyboard/mouse list navigation hook
// Used by: CommandPalette, BacklinkPanel, TagEditDialog
import { onDestroy } from 'svelte';

export interface ListNavigationOptions {
  /** Initial focused index */
  initialIndex?: number;
  /** Loop navigation at boundaries */
  loop?: boolean;
  /** Timeout for keyboard nav flag (ms) */
  keyboardNavTimeout?: number;
}

export function useListNavigation<T>(
  getItems: () => T[],
  options: ListNavigationOptions = {}
) {
  const {
    initialIndex = 0,
    loop = false,
    keyboardNavTimeout = 150,
  } = options;

  let focusedIndex = $state(initialIndex);
  let isKeyboardNavigating = $state(false);
  let timeout: ReturnType<typeof setTimeout> | null = null;

  // MEMORY LEAK FIX: Cleanup on destroy
  onDestroy(() => {
    if (timeout) clearTimeout(timeout);
  });

  function setKeyboardNav() {
    isKeyboardNavigating = true;
    if (timeout) clearTimeout(timeout);
    timeout = setTimeout(() => {
      isKeyboardNavigating = false;
    }, keyboardNavTimeout);
  }

  function navigateUp() {
    const items = getItems();
    if (items.length === 0) return;
    setKeyboardNav();
    if (focusedIndex > 0) {
      focusedIndex--;
    } else if (loop) {
      focusedIndex = items.length - 1;
    }
  }

  function navigateDown() {
    const items = getItems();
    if (items.length === 0) return;
    setKeyboardNav();
    if (focusedIndex < items.length - 1) {
      focusedIndex++;
    } else if (loop) {
      focusedIndex = 0;
    }
  }

  function handleKeydown(
    e: KeyboardEvent,
    callbacks: {
      onSelect?: (item: T) => void;
      onEscape?: () => void;
    } = {}
  ) {
    const items = getItems();

    switch (e.key) {
      case 'ArrowDown':
        e.preventDefault();
        navigateDown();
        break;
      case 'ArrowUp':
        e.preventDefault();
        navigateUp();
        break;
      case 'Enter':
        e.preventDefault();
        if (items[focusedIndex] && callbacks.onSelect) {
          callbacks.onSelect(items[focusedIndex]);
        }
        break;
      case 'Escape':
        e.preventDefault();
        e.stopPropagation();
        callbacks.onEscape?.();
        break;
    }
  }

  function handleMouseEnter(index: number) {
    // Prevent mouse interference during keyboard navigation
    if (isKeyboardNavigating) return;
    focusedIndex = index;
  }

  function reset() {
    focusedIndex = initialIndex;
  }

  function scrollIntoView(containerRef: HTMLElement | null, itemSelector: string) {
    if (!containerRef) return;
    const items = containerRef.querySelectorAll(itemSelector);
    items[focusedIndex]?.scrollIntoView({ block: 'nearest', behavior: 'instant' });
  }

  return {
    get focusedIndex() { return focusedIndex; },
    set focusedIndex(v: number) { focusedIndex = v; },
    get isKeyboardNavigating() { return isKeyboardNavigating; },
    handleKeydown,
    handleMouseEnter,
    navigateUp,
    navigateDown,
    reset,
    scrollIntoView,
  };
}
