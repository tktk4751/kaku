<script lang="ts">
  import { homeStore } from '$lib/stores/home.svelte';
  import NoteCard from './NoteCard.svelte';
  import NotePreviewModal from './NotePreviewModal.svelte';
  import TagChip from './TagChip.svelte';
  import type { GallerySortOrder } from '$lib/types';

  interface Props {
    onNavigateToNote: (uid: string) => void;
  }

  let { onNavigateToNote }: Props = $props();

  let searchQuery = $state('');
  let showPreview = $state(false);
  let previewNoteData = $state<typeof homeStore.items[0] | null>(null);

  function handleCardClick(note: typeof homeStore.items[0]) {
    previewNoteData = note;
    showPreview = true;
  }

  function handleCardDblClick(uid: string) {
    homeStore.hide();
    onNavigateToNote(uid);
  }

  function handleEditFromPreview(uid: string) {
    showPreview = false;
    previewNoteData = null;
    homeStore.hide();
    onNavigateToNote(uid);
  }

  function handleTagClick(tag: string) {
    showPreview = false;
    previewNoteData = null;
    homeStore.setTagFilter(tag);
  }

  function closePreview() {
    showPreview = false;
    previewNoteData = null;
  }

  function handleSortChange(value: GallerySortOrder) {
    homeStore.setSortOrder(value);
  }

  function handleKeydown(event: KeyboardEvent) {
    if (event.key === 'Escape') {
      if (showPreview) {
        closePreview();
      } else {
        homeStore.hide();
      }
    }
  }

  // Track visibility for body overflow
  $effect(() => {
    const visible = homeStore.isVisible;
    if (visible) {
      document.body.style.overflow = 'hidden';
    } else {
      document.body.style.overflow = '';
      // Reset preview when home view is hidden
      if (showPreview) {
        showPreview = false;
        previewNoteData = null;
      }
    }

    return () => {
      document.body.style.overflow = '';
    };
  });

  // Filter items based on search query (cached until dependencies change)
  let filteredItems = $derived.by(() => {
    const items = homeStore.items;
    const query = searchQuery.trim().toLowerCase();
    if (!query) return items;
    return items.filter(item =>
      item.title.toLowerCase().includes(query) ||
      item.preview.toLowerCase().includes(query) ||
      item.tags.some(tag => tag.toLowerCase().includes(query))
    );
  });

</script>

<svelte:window onkeydown={handleKeydown} />

{#if homeStore.isVisible}
  <div class="home-overlay">
    <div class="home-container">
      <header class="home-header">
        <div class="header-left">
          {#if homeStore.tagFilter}
            <div class="active-filter">
              <span class="filter-label">Filtered by:</span>
              <TagChip
                tag={homeStore.tagFilter}
                removable
                onRemove={() => homeStore.clearTagFilter()}
              />
            </div>
          {/if}
        </div>

        <!-- Search bar centered -->
        <div class="header-center">
          <div class="search-bar">
            <svg class="search-icon" width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
              <circle cx="11" cy="11" r="8"></circle>
              <line x1="21" y1="21" x2="16.65" y2="16.65"></line>
            </svg>
            <input
              type="text"
              class="search-input"
              placeholder="Search notes..."
              bind:value={searchQuery}
            />
            {#if searchQuery}
              <button class="clear-btn" onclick={() => searchQuery = ''} aria-label="Clear search">
                <svg width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
                  <line x1="18" y1="6" x2="6" y2="18"></line>
                  <line x1="6" y1="6" x2="18" y2="18"></line>
                </svg>
              </button>
            {/if}
          </div>
        </div>

        <div class="header-right">
          <!-- Sort toggle -->
          <div class="sort-toggle" role="group" aria-label="Sort order">
            <button
              class="sort-btn"
              class:active={homeStore.sortOrder === 'updated_at'}
              onclick={() => handleSortChange('updated_at')}
              title="Sort by updated"
              aria-pressed={homeStore.sortOrder === 'updated_at'}
            >
              <svg width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
                <circle cx="12" cy="12" r="10"></circle>
                <polyline points="12 6 12 12 16 14"></polyline>
              </svg>
            </button>
            <button
              class="sort-btn"
              class:active={homeStore.sortOrder === 'created_at'}
              onclick={() => handleSortChange('created_at')}
              title="Sort by created"
              aria-pressed={homeStore.sortOrder === 'created_at'}
            >
              <svg width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
                <rect x="3" y="4" width="18" height="18" rx="2" ry="2"></rect>
                <line x1="16" y1="2" x2="16" y2="6"></line>
                <line x1="8" y1="2" x2="8" y2="6"></line>
                <line x1="3" y1="10" x2="21" y2="10"></line>
              </svg>
            </button>
          </div>

          <button
            class="close-btn"
            onclick={() => homeStore.hide()}
            aria-label="Close home"
          >
            <svg width="24" height="24" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
              <line x1="18" y1="6" x2="6" y2="18"></line>
              <line x1="6" y1="6" x2="18" y2="18"></line>
            </svg>
          </button>
        </div>
      </header>

      {#if homeStore.allTags.length > 0}
        <div class="tag-bar">
          {#each homeStore.allTags as tag}
            <TagChip
              {tag}
              active={homeStore.tagFilter === tag}
              onclick={() => {
                if (homeStore.tagFilter === tag) {
                  homeStore.clearTagFilter();
                } else {
                  homeStore.setTagFilter(tag);
                }
              }}
            />
          {/each}
        </div>
      {/if}

      <main class="home-content">
        {#if homeStore.isLoading}
          <div class="loading">
            <div class="loading-spinner"></div>
            <span>Loading notes...</span>
          </div>
        {:else if homeStore.error}
          <div class="error">
            <p>{homeStore.error}</p>
            <button class="btn btn-secondary" onclick={() => homeStore.load()}>Retry</button>
          </div>
        {:else if filteredItems.length === 0}
          <div class="empty">
            {#if searchQuery}
              <p>No notes match "{searchQuery}"</p>
              <button class="btn btn-secondary" onclick={() => searchQuery = ''}>
                Clear search
              </button>
            {:else if homeStore.tagFilter}
              <p>No notes with tag "{homeStore.tagFilter}"</p>
              <button class="btn btn-secondary" onclick={() => homeStore.clearTagFilter()}>
                Clear filter
              </button>
            {:else}
              <p>No notes yet</p>
              <p class="empty-hint">Press Escape to go back and create your first note</p>
            {/if}
          </div>
        {:else}
          <div class="masonry-grid">
            {#each filteredItems as note (note.uid)}
              <NoteCard
                {note}
                onclick={() => handleCardClick(note)}
                ondblclick={() => handleCardDblClick(note.uid)}
                onTagClick={handleTagClick}
              />
            {/each}
          </div>

          <!-- Load more area -->
          <div class="load-more-area">
            {#if homeStore.hasMore}
              <button class="btn btn-secondary load-more-btn" onclick={() => homeStore.loadMore()}>
                Load more ({homeStore.totalCount - filteredItems.length} remaining)
              </button>
            {:else if homeStore.totalCount > 0}
              <span class="load-status">
                {homeStore.totalCount} notes
                {#if homeStore.loadTimeMs !== null}
                  <span class="load-time">({homeStore.loadTimeMs}ms)</span>
                {/if}
              </span>
            {/if}
          </div>
        {/if}
      </main>
    </div>
  </div>

  {#if showPreview && previewNoteData}
    {@const noteForPreview = previewNoteData}
    <NotePreviewModal
      open={showPreview}
      note={noteForPreview}
      onClose={closePreview}
      onEdit={() => handleEditFromPreview(noteForPreview.uid)}
      onTagClick={handleTagClick}
    />
  {/if}
{/if}

<style>
  .home-overlay {
    position: fixed;
    inset: 0;
    background: var(--bg-primary);
    z-index: 100;
    overflow: hidden;
  }

  .home-container {
    height: 100%;
    display: flex;
    flex-direction: column;
    max-width: 1400px;
    margin: 0 auto;
  }

  .home-header {
    display: flex;
    align-items: center;
    justify-content: space-between;
    padding: var(--size-4, 16px) var(--size-6, 24px);
    border-bottom: 1px solid var(--border-color);
    background: var(--bg-secondary);
    flex-shrink: 0;
    gap: var(--size-4, 16px);
  }

  .header-left {
    display: flex;
    align-items: center;
    gap: var(--size-4, 16px);
    flex-shrink: 0;
  }

  .active-filter {
    display: flex;
    align-items: center;
    gap: var(--size-2, 8px);
  }

  .filter-label {
    font-size: var(--font-size-sm, 13px);
    color: var(--fg-muted);
  }

  .header-center {
    flex: 1;
    max-width: 400px;
  }

  .search-bar {
    display: flex;
    align-items: center;
    gap: var(--size-2, 8px);
    padding: var(--size-2, 8px) var(--size-3, 12px);
    background: var(--bg-primary);
    border: 1px solid var(--border-color);
    border-radius: var(--radius-2, 8px);
    transition: border-color var(--duration-fast, 150ms) var(--ease-2, ease);
  }

  .search-bar:focus-within {
    border-color: var(--accent-blue);
  }

  .search-icon {
    color: var(--fg-muted);
    flex-shrink: 0;
  }

  .search-input {
    flex: 1;
    border: none;
    background: transparent;
    color: var(--fg-primary);
    font-size: var(--font-size-sm, 14px);
    outline: none;
    min-width: 0;
  }

  .search-input::placeholder {
    color: var(--fg-muted);
  }

  .clear-btn {
    display: flex;
    align-items: center;
    justify-content: center;
    padding: var(--size-1, 4px);
    color: var(--fg-muted);
    border-radius: var(--radius-1, 4px);
    transition: all var(--duration-fast, 150ms) var(--ease-2, ease);
  }

  .clear-btn:hover {
    background: var(--bg-highlight);
    color: var(--fg-primary);
  }

  .header-right {
    display: flex;
    align-items: center;
    gap: var(--size-3, 12px);
    flex-shrink: 0;
  }

  .sort-toggle {
    display: flex;
    background: var(--bg-tertiary);
    border: 1px solid var(--border-color);
    border-radius: var(--radius-2, 8px);
    overflow: hidden;
  }

  .sort-btn {
    display: flex;
    align-items: center;
    justify-content: center;
    padding: var(--size-2, 8px);
    color: var(--fg-muted);
    background: transparent;
    transition: all var(--duration-fast, 150ms) var(--ease-2, ease);
  }

  .sort-btn:hover {
    color: var(--fg-primary);
    background: var(--bg-highlight);
  }

  .sort-btn.active {
    color: var(--accent-blue);
    background: var(--bg-highlight);
  }

  .close-btn {
    display: flex;
    align-items: center;
    justify-content: center;
    padding: var(--size-2, 8px);
    color: var(--fg-muted);
    border-radius: var(--radius-2, 8px);
    transition: all var(--duration-fast, 150ms) var(--ease-2, ease);
  }

  .close-btn:hover {
    background: var(--bg-highlight);
    color: var(--fg-primary);
  }

  .tag-bar {
    display: flex;
    flex-wrap: wrap;
    gap: var(--size-2, 8px);
    padding: var(--size-3, 12px) var(--size-6, 24px);
    background: var(--bg-secondary);
    border-bottom: 1px solid var(--border-color);
    flex-shrink: 0;
  }

  .home-content {
    flex: 1;
    overflow-y: auto;
    padding: var(--size-6, 24px);
    /* GPU acceleration for smooth scrolling */
    transform: translateZ(0);
    -webkit-overflow-scrolling: touch;
  }

  .masonry-grid {
    column-count: 1;
    column-gap: var(--size-4, 16px);
  }

  @media (min-width: 500px) {
    .masonry-grid {
      column-count: 2;
    }
  }

  @media (min-width: 768px) {
    .masonry-grid {
      column-count: 3;
    }
  }

  @media (min-width: 1024px) {
    .masonry-grid {
      column-count: 4;
    }
  }

  @media (min-width: 1280px) {
    .masonry-grid {
      column-count: 5;
    }
  }

  .loading {
    display: flex;
    flex-direction: column;
    align-items: center;
    justify-content: center;
    gap: var(--size-4, 16px);
    padding: 60px;
    color: var(--fg-muted);
  }

  .loading-spinner {
    width: 32px;
    height: 32px;
    border: 3px solid var(--border-color);
    border-top-color: var(--accent-blue);
    border-radius: 50%;
    animation: spin 1s linear infinite;
  }

  @keyframes spin {
    to { transform: rotate(360deg); }
  }

  .error {
    display: flex;
    flex-direction: column;
    align-items: center;
    gap: var(--size-4, 16px);
    padding: 60px;
    color: var(--accent-red);
    text-align: center;
  }

  .empty {
    display: flex;
    flex-direction: column;
    align-items: center;
    gap: var(--size-3, 12px);
    padding: 60px;
    color: var(--fg-muted);
    text-align: center;
  }

  .empty-hint {
    font-size: var(--font-size-sm, 13px);
    opacity: 0.7;
  }

  .btn {
    display: inline-flex;
    align-items: center;
    justify-content: center;
    padding: var(--size-2, 8px) var(--size-4, 16px);
    font-size: var(--font-size-sm, 14px);
    font-weight: var(--font-weight-medium, 500);
    border-radius: var(--radius-2, 8px);
    transition: all var(--duration-fast, 150ms) var(--ease-2, ease);
    cursor: pointer;
  }

  .btn-secondary {
    background: var(--bg-tertiary);
    color: var(--fg-primary);
    border: 1px solid var(--border-color);
  }

  .btn-secondary:hover {
    background: var(--bg-highlight);
  }

  .load-more-area {
    display: flex;
    justify-content: center;
    align-items: center;
    padding: var(--size-6, 24px);
    min-height: 60px;
  }

  .load-more-btn {
    min-width: 200px;
  }

  .load-status {
    font-size: var(--font-size-sm, 13px);
    color: var(--fg-muted);
  }

  .load-time {
    opacity: 0.6;
  }
</style>
