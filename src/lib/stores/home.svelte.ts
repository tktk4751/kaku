// Home gallery store
import { listNotesGallery } from '$lib/services/api';
import type { NoteGalleryItemDto, GallerySortOrder } from '$lib/types';

// Initial page size - optimized for scroll performance
const INITIAL_PAGE_SIZE = 24;
// Items to add when loading more
const LOAD_MORE_SIZE = 24;

function createHomeStore() {
  let isVisible = $state(false);
  let allItems = $state<NoteGalleryItemDto[]>([]); // All loaded items
  let displayLimit = $state(INITIAL_PAGE_SIZE); // How many items to show
  let isLoading = $state(false);
  let error = $state<string | null>(null);
  let sortOrder = $state<GallerySortOrder>('updated_at');
  let tagFilter = $state<string | null>(null);
  let previewUid = $state<string | null>(null);
  let loadTimeMs = $state<number | null>(null);

  // Displayed items (limited for performance)
  let items = $derived(allItems.slice(0, displayLimit));
  // Whether there are more items to load
  let hasMore = $derived(displayLimit < allItems.length);
  // Total count of all items
  let totalCount = $derived(allItems.length);

  async function load() {
    isLoading = true;
    error = null;
    const startTime = performance.now();
    try {
      allItems = await listNotesGallery(sortOrder, tagFilter ?? undefined);
      displayLimit = INITIAL_PAGE_SIZE; // Reset display limit on new load
    } catch (e) {
      error = e instanceof Error ? e.message : 'Failed to load notes';
    } finally {
      isLoading = false;
      loadTimeMs = Math.round(performance.now() - startTime);
    }
  }

  function loadMore() {
    if (hasMore) {
      displayLimit = Math.min(displayLimit + LOAD_MORE_SIZE, allItems.length);
    }
  }

  function show() {
    isVisible = true;
    load();
  }

  function hide() {
    isVisible = false;
    previewUid = null;
  }

  function toggle() {
    if (isVisible) {
      hide();
    } else {
      show();
    }
  }

  function setSortOrder(order: GallerySortOrder) {
    if (sortOrder !== order) {
      sortOrder = order;
      load();
    }
  }

  function setTagFilter(tag: string | null) {
    if (tagFilter !== tag) {
      tagFilter = tag;
      load();
    }
  }

  function clearTagFilter() {
    setTagFilter(null);
  }

  function openPreview(uid: string) {
    previewUid = uid;
  }

  function closePreview() {
    previewUid = null;
  }

  // Cached all unique tags (only recalculates when items change)
  let allTags = $derived.by(() => {
    const tagSet = new Set<string>();
    for (const item of items) {
      for (const tag of item.tags) {
        tagSet.add(tag);
      }
    }
    return Array.from(tagSet).sort();
  });

  return {
    get isVisible() { return isVisible; },
    get items() { return items; },
    get isLoading() { return isLoading; },
    get error() { return error; },
    get sortOrder() { return sortOrder; },
    get tagFilter() { return tagFilter; },
    get previewUid() { return previewUid; },
    get allTags() { return allTags; },
    get hasMore() { return hasMore; },
    get totalCount() { return totalCount; },
    get loadTimeMs() { return loadTimeMs; },
    load,
    loadMore,
    show,
    hide,
    toggle,
    setSortOrder,
    setTagFilter,
    clearTagFilter,
    openPreview,
    closePreview,
  };
}

export const homeStore = createHomeStore();
