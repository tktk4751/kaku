<script lang="ts" generics="T">
  /**
   * VirtualList - High-performance virtual scrolling component
   *
   * Renders only visible items plus a small overscan buffer for smooth scrolling.
   * Designed to handle 100k+ items without performance degradation.
   *
   * @example
   * <VirtualList
   *   items={allNotes}
   *   itemHeight={60}
   *   let:item
   *   let:index
   * >
   *   <NoteItem {item} {index} />
   * </VirtualList>
   */

  import { onMount } from 'svelte';
  import type { Snippet } from 'svelte';

  interface Props {
    /** Array of all items */
    items: T[];
    /** Height of each item in pixels */
    itemHeight: number;
    /** Number of items to render above/below visible area */
    overscan?: number;
    /** CSS class for the container */
    class?: string;
    /** Content snippet that receives item and index */
    children: Snippet<[T, number]>;
  }

  let {
    items,
    itemHeight,
    overscan = 5,
    class: className = '',
    children
  }: Props = $props();

  let containerRef: HTMLDivElement | undefined = $state();
  let scrollTop = $state(0);
  let containerHeight = $state(400);

  // Derived values
  const totalHeight = $derived(items.length * itemHeight);
  const visibleCount = $derived(Math.ceil(containerHeight / itemHeight));
  const startIndex = $derived(Math.max(0, Math.floor(scrollTop / itemHeight) - overscan));
  const endIndex = $derived(Math.min(items.length, startIndex + visibleCount + overscan * 2));
  const offsetY = $derived(startIndex * itemHeight);
  const visibleItems = $derived(items.slice(startIndex, endIndex));

  function handleScroll(e: Event) {
    const target = e.target as HTMLDivElement;
    scrollTop = target.scrollTop;
  }

  onMount(() => {
    if (!containerRef) return;

    // Initial measurement
    containerHeight = containerRef.clientHeight;

    // Observe size changes
    const observer = new ResizeObserver((entries) => {
      const entry = entries[0];
      if (entry) {
        containerHeight = entry.contentRect.height;
      }
    });

    observer.observe(containerRef);

    return () => observer.disconnect();
  });

  /**
   * Scroll to a specific index
   */
  export function scrollToIndex(index: number, behavior: ScrollBehavior = 'instant') {
    if (!containerRef) return;
    const targetTop = index * itemHeight;
    containerRef.scrollTo({ top: targetTop, behavior });
  }

  /**
   * Ensure an index is visible (scroll if needed)
   */
  export function ensureVisible(index: number) {
    if (!containerRef) return;

    const itemTop = index * itemHeight;
    const itemBottom = itemTop + itemHeight;
    const viewTop = scrollTop;
    const viewBottom = scrollTop + containerHeight;

    if (itemTop < viewTop) {
      containerRef.scrollTo({ top: itemTop, behavior: 'instant' });
    } else if (itemBottom > viewBottom) {
      containerRef.scrollTo({ top: itemBottom - containerHeight, behavior: 'instant' });
    }
  }
</script>

<div
  bind:this={containerRef}
  class="virtual-list-container {className}"
  onscroll={handleScroll}
  role="listbox"
  tabindex="0"
>
  <div class="virtual-list-spacer" style="height: {totalHeight}px;">
    <div class="virtual-list-items" style="transform: translateY({offsetY}px);">
      {#each visibleItems as item, i (startIndex + i)}
        {@render children(item, startIndex + i)}
      {/each}
    </div>
  </div>
</div>

<style>
  .virtual-list-container {
    overflow-y: auto;
    position: relative;
    /* Prevent scroll chaining */
    overscroll-behavior: contain;
  }

  .virtual-list-spacer {
    position: relative;
  }

  .virtual-list-items {
    position: absolute;
    top: 0;
    left: 0;
    right: 0;
  }
</style>
