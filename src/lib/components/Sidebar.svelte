<script lang="ts">
  import { untrack } from 'svelte';
  import { noteStore } from '$lib/stores/note.svelte';
  import { searchStore } from '$lib/stores/search.svelte';
  import ConfirmDialog from './ConfirmDialog.svelte';
  import SearchInput from './SearchInput.svelte';
  import SearchResults from './SearchResults.svelte';

  interface Props {
    isOpen: boolean;
    onClose: () => void;
    onNoteSelect: (uid: string) => void;
    onNewNote: () => void;
    onOpenSettings: () => void;
    onDeleteNote: (uid: string) => void;
  }

  let { isOpen, onClose, onNoteSelect, onNewNote, onOpenSettings, onDeleteNote }: Props = $props();

  let deleteConfirmUid = $state<string | null>(null);
  let deleteConfirmTitle = $state('');
  let focusedIndex = $state(-1);
  let searchFocusedIndex = $state(-1);
  let noteListElement: HTMLUListElement;
  let searchResultsElement: HTMLUListElement;

  // Use plain variable to track previous state (not reactive)
  let prevIsOpen = false;

  // Track if user is using keyboard navigation (to ignore mouse events during scroll)
  let isKeyboardNavigating = false;
  let keyboardNavTimeout: ReturnType<typeof setTimeout> | null = null;

  // Reset focused index only when sidebar opens
  $effect(() => {
    const currentIsOpen = isOpen;

    if (currentIsOpen && !prevIsOpen) {
      // Sidebar just opened - initialize focusedIndex
      untrack(() => {
        const currentIndex = noteStore.noteList.findIndex(n => n.uid === noteStore.currentNote?.uid);
        focusedIndex = currentIndex >= 0 ? currentIndex : 0;
      });
      // Focus the list element for keyboard navigation
      setTimeout(() => {
        noteListElement?.focus();
        scrollToIndex(focusedIndex);
      }, 50);
    } else if (!currentIsOpen && prevIsOpen) {
      // Sidebar just closed
      focusedIndex = -1;
      searchFocusedIndex = -1;
      // Clear search when closing sidebar
      searchStore.clear();
    }

    prevIsOpen = currentIsOpen;
  });

  // Handle search result selection
  function handleSearchSelect(uid: string) {
    searchStore.clear();
    onNoteSelect(uid);
  }

  // Handle search result mouse hover
  function handleSearchMouseEnter(index: number) {
    if (isKeyboardNavigating) return;
    searchFocusedIndex = index;
  }

  // Keyboard navigation for search results
  function handleSearchKeydown(event: KeyboardEvent) {
    if (!searchStore.isActive || searchStore.results.length === 0) return;

    const count = searchStore.results.length;
    let newIndex = searchFocusedIndex;

    switch (event.key) {
      case 'ArrowDown':
        event.preventDefault();
        newIndex = searchFocusedIndex < count - 1 ? searchFocusedIndex + 1 : searchFocusedIndex;
        break;
      case 'ArrowUp':
        event.preventDefault();
        newIndex = searchFocusedIndex > 0 ? searchFocusedIndex - 1 : 0;
        break;
      case 'Enter':
        event.preventDefault();
        if (searchFocusedIndex >= 0 && searchFocusedIndex < count) {
          handleSearchSelect(searchStore.results[searchFocusedIndex].uid);
        }
        return;
      case 'Escape':
        event.preventDefault();
        searchStore.clear();
        return;
    }

    if (newIndex !== searchFocusedIndex) {
      isKeyboardNavigating = true;
      if (keyboardNavTimeout) clearTimeout(keyboardNavTimeout);
      keyboardNavTimeout = setTimeout(() => {
        isKeyboardNavigating = false;
      }, 150);
      searchFocusedIndex = newIndex;
    }
  }

  function handleListKeydown(event: KeyboardEvent) {
    const noteCount = noteStore.noteList.length;
    if (noteCount === 0) return;

    let newIndex = focusedIndex;

    switch (event.key) {
      case 'ArrowDown':
        event.preventDefault();
        if (focusedIndex < noteCount - 1) {
          newIndex = focusedIndex + 1;
        }
        break;
      case 'ArrowUp':
        event.preventDefault();
        if (focusedIndex > 0) {
          newIndex = focusedIndex - 1;
        }
        break;
      case 'Enter':
        event.preventDefault();
        if (focusedIndex >= 0 && focusedIndex < noteCount) {
          onNoteSelect(noteStore.noteList[focusedIndex].uid);
        }
        return;
    }

    if (newIndex !== focusedIndex) {
      // Mark as keyboard navigating to ignore mouse events during scroll
      isKeyboardNavigating = true;
      if (keyboardNavTimeout) clearTimeout(keyboardNavTimeout);
      keyboardNavTimeout = setTimeout(() => {
        isKeyboardNavigating = false;
      }, 150);

      focusedIndex = newIndex;
      requestAnimationFrame(() => scrollToIndex(newIndex));
    }
  }

  function handleMouseEnter(index: number) {
    // Ignore mouse events during keyboard navigation (prevents scroll-back issue)
    if (isKeyboardNavigating) return;
    focusedIndex = index;
  }

  function scrollToIndex(index: number) {
    if (!noteListElement || index < 0) return;
    const items = noteListElement.querySelectorAll('.note-item-wrapper');
    const targetElement = items[index];
    if (targetElement) {
      targetElement.scrollIntoView({ block: 'nearest', behavior: 'instant' });
    }
  }

  function formatDate(dateStr: string): string {
    const date = new Date(dateStr);
    const now = new Date();
    const diff = now.getTime() - date.getTime();
    const days = Math.floor(diff / (1000 * 60 * 60 * 24));

    if (days === 0) {
      return date.toLocaleTimeString('ja-JP', { hour: '2-digit', minute: '2-digit' });
    } else if (days === 1) {
      return 'Yesterday';
    } else if (days < 7) {
      return `${days} days ago`;
    } else {
      return date.toLocaleDateString('ja-JP', { month: 'short', day: 'numeric' });
    }
  }

  function handleBackdropClick() {
    onClose();
  }

  function handleKeydown(event: KeyboardEvent, uid: string) {
    if (event.key === 'Enter' || event.key === ' ') {
      event.preventDefault();
      onNoteSelect(uid);
    }
  }

  function handleNoteClick(index: number, uid: string) {
    focusedIndex = index;
    onNoteSelect(uid);
  }

  function handleDeleteClick(event: MouseEvent, uid: string, title: string) {
    event.stopPropagation();
    deleteConfirmUid = uid;
    deleteConfirmTitle = title || 'Untitled';
  }

  function confirmDelete() {
    if (deleteConfirmUid) {
      onDeleteNote(deleteConfirmUid);
    }
    deleteConfirmUid = null;
  }

  function cancelDelete() {
    deleteConfirmUid = null;
  }
</script>

{#if isOpen}
  <!-- svelte-ignore a11y_click_events_have_key_events a11y_no_static_element_interactions -->
  <div class="backdrop" onclick={handleBackdropClick}></div>
{/if}

<aside
  class="sidebar"
  class:open={isOpen}
  role="navigation"
  aria-label="Note list"
  aria-hidden={!isOpen}
>
  <header class="sidebar-header">
    <h2 id="sidebar-title">Notes</h2>
    <button class="icon-btn" onclick={onNewNote} title="New note (Ctrl+N)" aria-label="Create new note">
      <svg width="18" height="18" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
        <line x1="12" y1="5" x2="12" y2="19"></line>
        <line x1="5" y1="12" x2="19" y2="12"></line>
      </svg>
    </button>
  </header>

  <!-- Search Bar -->
  <!-- svelte-ignore a11y_no_static_element_interactions -->
  <div onkeydown={handleSearchKeydown}>
    <SearchInput
      value={searchStore.query}
      onInput={(v) => searchStore.setQuery(v)}
      onClear={() => searchStore.clear()}
      isSearching={searchStore.isSearching}
    />
  </div>

  <!-- Search Results or Note List -->
  {#if searchStore.isActive}
    <SearchResults
      results={searchStore.results}
      onSelect={handleSearchSelect}
      focusedIndex={searchFocusedIndex}
      onMouseEnter={handleSearchMouseEnter}
    />
  {:else}
    <ul
    class="note-list"
    role="listbox"
    aria-labelledby="sidebar-title"
    tabindex="0"
    bind:this={noteListElement}
    onkeydown={handleListKeydown}
  >
    {#each noteStore.noteList as item, index (item.uid)}
      <!-- svelte-ignore a11y_no_static_element_interactions -->
      <li
        class="note-item-wrapper"
        class:focused={focusedIndex === index}
        role="option"
        aria-selected={noteStore.currentNote?.uid === item.uid}
        onmouseenter={() => handleMouseEnter(index)}
      >
        <button
          class="note-item"
          onclick={() => handleNoteClick(index, item.uid)}
          onkeydown={(e) => handleKeydown(e, item.uid)}
          aria-label="{item.title || 'Untitled'}, updated {formatDate(item.updated_at)}"
        >
          <span class="note-title">{item.title || 'Untitled'}</span>
          <span class="note-date" aria-hidden="true">{formatDate(item.updated_at)}</span>
        </button>
        <button
          class="delete-btn"
          onclick={(e) => handleDeleteClick(e, item.uid, item.title)}
          title="Delete note"
          aria-label="Delete {item.title || 'Untitled'}"
        >
          <svg width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
            <polyline points="3 6 5 6 21 6"></polyline>
            <path d="M19 6v14a2 2 0 0 1-2 2H7a2 2 0 0 1-2-2V6m3 0V4a2 2 0 0 1 2-2h4a2 2 0 0 1 2 2v2"></path>
          </svg>
        </button>
      </li>
    {:else}
      <li class="empty-state" role="option" aria-selected="false" aria-disabled="true">No notes yet</li>
    {/each}
  </ul>
  {/if}

  <footer class="sidebar-footer">
    <button class="settings-btn" onclick={onOpenSettings} aria-label="Open settings">
      <svg width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
        <circle cx="12" cy="12" r="3"></circle>
        <path d="M19.4 15a1.65 1.65 0 0 0 .33 1.82l.06.06a2 2 0 0 1 0 2.83 2 2 0 0 1-2.83 0l-.06-.06a1.65 1.65 0 0 0-1.82-.33 1.65 1.65 0 0 0-1 1.51V21a2 2 0 0 1-2 2 2 2 0 0 1-2-2v-.09A1.65 1.65 0 0 0 9 19.4a1.65 1.65 0 0 0-1.82.33l-.06.06a2 2 0 0 1-2.83 0 2 2 0 0 1 0-2.83l.06-.06a1.65 1.65 0 0 0 .33-1.82 1.65 1.65 0 0 0-1.51-1H3a2 2 0 0 1-2-2 2 2 0 0 1 2-2h.09A1.65 1.65 0 0 0 4.6 9a1.65 1.65 0 0 0-.33-1.82l-.06-.06a2 2 0 0 1 0-2.83 2 2 0 0 1 2.83 0l.06.06a1.65 1.65 0 0 0 1.82.33H9a1.65 1.65 0 0 0 1-1.51V3a2 2 0 0 1 2-2 2 2 0 0 1 2 2v.09a1.65 1.65 0 0 0 1 1.51 1.65 1.65 0 0 0 1.82-.33l.06-.06a2 2 0 0 1 2.83 0 2 2 0 0 1 0 2.83l-.06.06a1.65 1.65 0 0 0-.33 1.82V9a1.65 1.65 0 0 0 1.51 1H21a2 2 0 0 1 2 2 2 2 0 0 1-2 2h-.09a1.65 1.65 0 0 0-1.51 1z"></path>
      </svg>
      <span>Settings</span>
    </button>
  </footer>
</aside>

{#if deleteConfirmUid}
  <ConfirmDialog
    title="Delete Note"
    message="Are you sure you want to delete '{deleteConfirmTitle}'? This action cannot be undone."
    confirmText="Delete"
    cancelText="Cancel"
    danger={true}
    onConfirm={confirmDelete}
    onCancel={cancelDelete}
  />
{/if}

<style>
  .backdrop {
    position: fixed;
    inset: 0;
    background: rgba(0, 0, 0, 0.5);
    z-index: 90;
  }

  .sidebar {
    position: fixed;
    top: 0;
    left: 0;
    bottom: 0;
    width: 280px;
    background: var(--bg-secondary);
    border-right: 1px solid var(--border-color);
    display: flex;
    flex-direction: column;
    transform: translateX(-100%);
    transition: transform 0.2s ease;
    z-index: 100;
  }

  .sidebar.open {
    transform: translateX(0);
  }

  .sidebar-header {
    display: flex;
    align-items: center;
    justify-content: space-between;
    padding: 16px;
    border-bottom: 1px solid var(--border-color);
  }

  .sidebar-header h2 {
    font-size: 14px;
    font-weight: 600;
    color: var(--fg-primary);
  }

  .icon-btn {
    display: flex;
    align-items: center;
    justify-content: center;
    width: 32px;
    height: 32px;
    border-radius: 6px;
    color: var(--fg-secondary);
    transition: all 0.15s;
  }

  .icon-btn:hover {
    background: var(--bg-highlight);
    color: var(--fg-primary);
  }

  .note-list {
    flex: 1;
    overflow-y: auto;
    padding: 8px;
    outline: none;
    /* Prevent scroll chaining to parent elements */
    overscroll-behavior: contain;
  }

  .note-item-wrapper {
    display: flex;
    align-items: stretch;
    border-radius: 6px;
    transition: background 0.15s;
    position: relative;
  }

  /* Show delete button on hover or focus */
  .note-item-wrapper:hover .delete-btn,
  .note-item-wrapper.focused .delete-btn {
    opacity: 1;
  }

  /* Selection/focus state - only one style for both hover and keyboard */
  .note-item-wrapper.focused {
    background: var(--bg-highlight);
    outline: 2px solid var(--accent-blue);
    outline-offset: -2px;
  }

  .note-item {
    flex: 1;
    display: flex;
    flex-direction: column;
    align-items: flex-start;
    padding: 12px;
    border-radius: 6px 0 0 6px;
    text-align: left;
    min-width: 0;
  }

  /* Disable default focus outline - we use wrapper's focused class instead */
  .note-item:focus,
  .note-item:focus-visible {
    outline: none;
  }

  .delete-btn {
    display: flex;
    align-items: center;
    justify-content: center;
    width: 36px;
    opacity: 0;
    color: var(--fg-muted);
    transition: all 0.15s;
    border-radius: 0 6px 6px 0;
  }

  /* Disable default focus outline on delete button */
  .delete-btn:focus,
  .delete-btn:focus-visible {
    outline: none;
  }

  .delete-btn:hover {
    color: var(--accent-red, #f7768e);
    background: rgba(247, 118, 142, 0.1);
  }

  .note-title {
    font-size: 13px;
    font-weight: 500;
    color: var(--fg-primary);
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
    width: 100%;
  }

  .note-date {
    font-size: 11px;
    color: var(--fg-muted);
    margin-top: 4px;
  }

  .empty-state {
    padding: 24px;
    text-align: center;
    color: var(--fg-muted);
    font-size: 13px;
  }

  .sidebar-footer {
    padding: 12px;
    border-top: 1px solid var(--border-color);
  }

  .settings-btn {
    display: flex;
    align-items: center;
    gap: 8px;
    width: 100%;
    padding: 10px 12px;
    border-radius: 6px;
    font-size: 13px;
    color: var(--fg-secondary);
    transition: all 0.15s;
  }

  .settings-btn:hover {
    background: var(--bg-highlight);
    color: var(--fg-primary);
  }
</style>
