<script lang="ts">
  import { searchNotes } from '$lib/services/api';
  import type { SearchResultDto } from '$lib/types';
  import HighlightText from './HighlightText.svelte';

  interface Props {
    onSelect: (uid: string) => void;
    onClose: () => void;
  }

  let { onSelect, onClose }: Props = $props();

  let inputRef = $state<HTMLInputElement | null>(null);
  let resultsRef = $state<HTMLUListElement | null>(null);
  let query = $state('');
  let results = $state<SearchResultDto[]>([]);
  let isSearching = $state(false);
  let focusedIndex = $state(0);

  // Track if user is using keyboard navigation (to ignore mouse events during scroll)
  let isKeyboardNavigating = false;
  let keyboardNavTimeout: ReturnType<typeof setTimeout> | null = null;

  let debounceTimer: ReturnType<typeof setTimeout> | null = null;
  const DEBOUNCE_MS = 100;

  // Focus input on mount
  $effect(() => {
    if (inputRef) {
      setTimeout(() => inputRef?.focus(), 10);
    }
  });

  // Scroll focused item into view
  $effect(() => {
    if (resultsRef && results.length > 0 && focusedIndex >= 0) {
      const focusedItem = resultsRef.children[focusedIndex] as HTMLElement;
      if (focusedItem) {
        focusedItem.scrollIntoView({ block: 'nearest', behavior: 'instant' });
      }
    }
  });

  // Search when query changes
  $effect(() => {
    if (debounceTimer) {
      clearTimeout(debounceTimer);
    }

    if (!query.trim()) {
      results = [];
      focusedIndex = 0;
      return;
    }

    debounceTimer = setTimeout(async () => {
      isSearching = true;
      try {
        results = await searchNotes(query);
        focusedIndex = 0;
      } catch (e) {
        console.error('Search failed:', e);
        results = [];
      } finally {
        isSearching = false;
      }
    }, DEBOUNCE_MS);
  });

  function handleKeydown(e: KeyboardEvent) {
    let newIndex = focusedIndex;

    switch (e.key) {
      case 'Escape':
        e.preventDefault();
        onClose();
        return;
      case 'ArrowDown':
        e.preventDefault();
        if (results.length > 0) {
          newIndex = focusedIndex < results.length - 1 ? focusedIndex + 1 : focusedIndex;
        }
        break;
      case 'ArrowUp':
        e.preventDefault();
        if (results.length > 0) {
          newIndex = focusedIndex > 0 ? focusedIndex - 1 : 0;
        }
        break;
      case 'Enter':
        e.preventDefault();
        if (results.length > 0 && focusedIndex >= 0) {
          handleSelect(results[focusedIndex].uid);
        }
        return;
      default:
        return;
    }

    // Update focused index and mark as keyboard navigating
    if (newIndex !== focusedIndex) {
      isKeyboardNavigating = true;
      if (keyboardNavTimeout) clearTimeout(keyboardNavTimeout);
      keyboardNavTimeout = setTimeout(() => {
        isKeyboardNavigating = false;
      }, 150);
      focusedIndex = newIndex;
    }
  }

  function handleMouseEnter(index: number) {
    // Ignore mouse events during keyboard navigation (prevents scroll-back issue)
    if (isKeyboardNavigating) return;
    focusedIndex = index;
  }

  function handleSelect(uid: string) {
    onSelect(uid);
    onClose();
  }

  function handleBackdropClick(e: MouseEvent) {
    if (e.target === e.currentTarget) {
      onClose();
    }
  }

  function handleInput(e: Event) {
    const target = e.target as HTMLInputElement;
    query = target.value;
  }
</script>

<!-- svelte-ignore a11y_no_static_element_interactions -->
<!-- svelte-ignore a11y_click_events_have_key_events -->
<div class="palette-backdrop" onclick={handleBackdropClick}>
  <div class="palette">
    <div class="palette-input">
      <svg class="search-icon" width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
        <circle cx="11" cy="11" r="8"></circle>
        <line x1="21" y1="21" x2="16.65" y2="16.65"></line>
      </svg>
      <input
        bind:this={inputRef}
        type="text"
        value={query}
        oninput={handleInput}
        onkeydown={handleKeydown}
        placeholder="Search notes..."
        spellcheck="false"
      />
      <span class="searching-indicator">{isSearching ? '...' : ''}</span>
    </div>

    {#if query.trim()}
      <ul class="palette-results" role="listbox" bind:this={resultsRef}>
        {#each results as result, index (result.uid)}
          <li
            class="result-item"
            class:focused={focusedIndex === index}
            role="option"
            aria-selected={focusedIndex === index}
            onmouseenter={() => handleMouseEnter(index)}
          >
            <button onclick={() => handleSelect(result.uid)}>
              <span class="result-title">
                <HighlightText text={result.title || 'Untitled'} matches={result.title_matches} />
              </span>
              {#if result.content_preview}
                <span class="result-preview">
                  <HighlightText
                    text={result.content_preview.text}
                    matches={[{
                      start: result.content_preview.match_start,
                      end: result.content_preview.match_end
                    }]}
                  />
                </span>
              {/if}
            </button>
          </li>
        {:else}
          <li class="no-results">No results found</li>
        {/each}
      </ul>
    {/if}
  </div>
</div>

<style>
  .palette-backdrop {
    position: fixed;
    inset: 0;
    background: rgba(0, 0, 0, 0.5);
    display: flex;
    align-items: flex-start;
    justify-content: center;
    padding-top: 15vh;
    z-index: 1000;
  }

  .palette {
    width: 100%;
    max-width: 500px;
    background: var(--bg-secondary);
    border: 1px solid var(--border-color);
    border-radius: 10px;
    box-shadow: 0 8px 32px rgba(0, 0, 0, 0.3);
    overflow: hidden;
    /* Prevent position shift during content changes */
    transform: translateZ(0);
  }

  .palette-input {
    display: flex;
    align-items: center;
    gap: 10px;
    padding: 14px 16px;
    border-bottom: 1px solid var(--border-color);
  }

  .search-icon {
    flex-shrink: 0;
    color: var(--fg-muted);
  }

  .palette-input input {
    flex: 1;
    border: none;
    background: transparent;
    color: var(--fg-primary);
    font-size: 15px;
    outline: none;
  }

  .palette-input input::placeholder {
    color: var(--fg-muted);
  }

  .searching-indicator {
    color: var(--fg-muted);
    font-size: 14px;
    min-width: 20px;
  }

  .palette-results {
    max-height: 400px;
    min-height: 60px;
    overflow-y: scroll;
    padding: 8px;
    overscroll-behavior: contain;
    /* Prevent layout shift from scrollbar */
    scrollbar-gutter: stable;
  }

  .result-item {
    border-radius: 6px;
  }

  .result-item.focused {
    background: var(--bg-highlight);
  }

  .result-item button {
    display: flex;
    flex-direction: column;
    align-items: flex-start;
    width: 100%;
    padding: 10px 12px;
    text-align: left;
  }

  .result-item button:focus {
    outline: none;
  }

  .result-title {
    font-size: 14px;
    font-weight: 500;
    color: var(--fg-primary);
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
    width: 100%;
  }

  .result-preview {
    font-size: 12px;
    color: var(--fg-muted);
    margin-top: 4px;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
    width: 100%;
  }

  .no-results {
    padding: 24px;
    text-align: center;
    color: var(--fg-muted);
    font-size: 14px;
  }
</style>
