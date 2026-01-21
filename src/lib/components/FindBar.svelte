<script lang="ts">
  import type { EditorView } from '@codemirror/view';
  import {
    setSearchAndGoToFirst,
    clearSearch,
    getSearchState,
    goToNextMatch,
    goToPrevMatch,
    replaceCurrent,
    replaceAllMatches,
    type SearchState,
  } from '$lib/editor/setup';

  interface Props {
    editorView: EditorView | null;
    onClose: () => void;
  }

  let { editorView, onClose }: Props = $props();

  let findInput = $state<HTMLInputElement | null>(null);
  let replaceInput = $state<HTMLInputElement | null>(null);

  let query = $state('');
  let replacement = $state('');
  let caseSensitive = $state(false);
  let showReplace = $state(false);
  let searchState = $state<SearchState>({ matchCount: 0, currentMatch: 0 });

  // Debounce timer for search
  let searchTimer: ReturnType<typeof setTimeout> | null = null;
  const DEBOUNCE_MS = 50;

  // Focus find input on mount
  $effect(() => {
    if (findInput) {
      setTimeout(() => findInput?.focus(), 10);
    }
  });

  // Update search when query or case sensitivity changes (debounced)
  $effect(() => {
    if (searchTimer) {
      clearTimeout(searchTimer);
    }

    if (editorView && query) {
      searchTimer = setTimeout(() => {
        // Set search and automatically go to first match in document order
        setSearchAndGoToFirst(editorView!, query, caseSensitive);
        // Small delay to let CodeMirror update, then get state
        requestAnimationFrame(() => {
          if (editorView) {
            searchState = getSearchState(editorView);
          }
        });
      }, DEBOUNCE_MS);
    } else if (editorView) {
      clearSearch(editorView);
      searchState = { matchCount: 0, currentMatch: 0 };
    }
  });

  function handleFindKeydown(e: KeyboardEvent) {
    if (e.key === 'Escape') {
      handleClose();
    } else if (e.key === 'Enter') {
      e.preventDefault();
      if (e.shiftKey) {
        goToPrev();
      } else {
        goToNext();
      }
    }
  }

  function handleReplaceKeydown(e: KeyboardEvent) {
    if (e.key === 'Escape') {
      handleClose();
    } else if (e.key === 'Enter') {
      e.preventDefault();
      handleReplace();
    }
  }

  function goToNext() {
    if (!editorView || searchState.matchCount === 0) return;
    goToNextMatch(editorView);
    searchState = getSearchState(editorView);
  }

  function goToPrev() {
    if (!editorView || searchState.matchCount === 0) return;
    goToPrevMatch(editorView);
    searchState = getSearchState(editorView);
  }

  function handleReplace() {
    if (!editorView || searchState.matchCount === 0) return;
    replaceCurrent(editorView, replacement);
    // Update search state after replace
    requestAnimationFrame(() => {
      if (editorView) {
        searchState = getSearchState(editorView);
      }
    });
  }

  function handleReplaceAll() {
    if (!editorView || searchState.matchCount === 0) return;
    replaceAllMatches(editorView, replacement);
    // Update search state after replace all
    requestAnimationFrame(() => {
      if (editorView) {
        searchState = getSearchState(editorView);
      }
    });
  }

  function handleClose() {
    if (editorView) {
      clearSearch(editorView);
      editorView.focus();
    }
    query = '';
    replacement = '';
    searchState = { matchCount: 0, currentMatch: 0 };
    onClose();
  }

  function toggleReplace() {
    showReplace = !showReplace;
    if (showReplace) {
      setTimeout(() => replaceInput?.focus(), 10);
    }
  }
</script>

<div class="find-bar">
  <div class="find-row">
    <!-- Toggle replace button -->
    <button
      class="toggle-btn"
      class:active={showReplace}
      onclick={toggleReplace}
      title={showReplace ? 'Hide replace' : 'Show replace'}
      aria-label={showReplace ? 'Hide replace' : 'Show replace'}
    >
      <svg width="12" height="12" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
        <polyline points={showReplace ? "18 15 12 9 6 15" : "6 9 12 15 18 9"}></polyline>
      </svg>
    </button>

    <!-- Find input -->
    <div class="input-wrapper">
      <input
        bind:this={findInput}
        bind:value={query}
        type="text"
        placeholder="Find"
        spellcheck="false"
        onkeydown={handleFindKeydown}
      />
    </div>

    <!-- Match count -->
    <span class="match-count">
      {#if query && searchState.matchCount > 0}
        {searchState.currentMatch}/{searchState.matchCount}
      {:else if query}
        No results
      {/if}
    </span>

    <!-- Navigation -->
    <button class="nav-btn" onclick={goToPrev} disabled={searchState.matchCount === 0} title="Previous (Shift+Enter)">
      <svg width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
        <polyline points="18 15 12 9 6 15"></polyline>
      </svg>
    </button>
    <button class="nav-btn" onclick={goToNext} disabled={searchState.matchCount === 0} title="Next (Enter)">
      <svg width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
        <polyline points="6 9 12 15 18 9"></polyline>
      </svg>
    </button>

    <!-- Case sensitive toggle -->
    <button
      class="option-btn"
      class:active={caseSensitive}
      onclick={() => caseSensitive = !caseSensitive}
      title="Match case"
    >
      Aa
    </button>

    <!-- Spacer to push close button to far right -->
    <div class="flex-spacer"></div>

    <!-- Close button -->
    <button class="close-btn" onclick={handleClose} title="Close (Esc)">
      <svg width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
        <line x1="18" y1="6" x2="6" y2="18"></line>
        <line x1="6" y1="6" x2="18" y2="18"></line>
      </svg>
    </button>
  </div>

  <!-- Replace row (hidden by default) -->
  {#if showReplace}
    <div class="replace-row">
      <div class="spacer"></div>
      <div class="input-wrapper">
        <input
          bind:this={replaceInput}
          bind:value={replacement}
          type="text"
          placeholder="Replace"
          spellcheck="false"
          onkeydown={handleReplaceKeydown}
        />
      </div>
      <button class="action-btn" onclick={handleReplace} disabled={searchState.matchCount === 0} title="Replace">
        Replace
      </button>
      <button class="action-btn" onclick={handleReplaceAll} disabled={searchState.matchCount === 0} title="Replace all">
        All
      </button>
    </div>
  {/if}
</div>

<style>
  .find-bar {
    display: flex;
    flex-direction: column;
    gap: 6px;
    padding: 8px 12px;
    background: var(--bg-secondary);
    border-bottom: 1px solid var(--border-color);
  }

  .find-row,
  .replace-row {
    display: flex;
    align-items: center;
    gap: 6px;
  }

  .spacer {
    width: 24px;
  }

  .flex-spacer {
    flex: 1;
  }

  .input-wrapper {
    flex: 1;
    max-width: 240px;
  }

  .input-wrapper input {
    width: 100%;
    padding: 5px 10px;
    background: var(--bg-primary);
    border: 1px solid var(--border-color);
    border-radius: 4px;
    color: var(--fg-primary);
    font-size: 13px;
    outline: none;
    transition: border-color 0.15s;
  }

  .input-wrapper input:focus {
    border-color: var(--accent-blue);
  }

  .input-wrapper input::placeholder {
    color: var(--fg-muted);
  }

  .match-count {
    min-width: 70px;
    font-size: 12px;
    color: var(--fg-muted);
    text-align: center;
  }

  .toggle-btn,
  .nav-btn,
  .option-btn,
  .close-btn {
    display: flex;
    align-items: center;
    justify-content: center;
    width: 24px;
    height: 24px;
    padding: 0;
    background: transparent;
    border: none;
    border-radius: 4px;
    color: var(--fg-muted);
    cursor: pointer;
    transition: all 0.15s;
  }

  .toggle-btn:hover,
  .nav-btn:hover:not(:disabled),
  .option-btn:hover,
  .close-btn:hover {
    background: var(--bg-highlight);
    color: var(--fg-primary);
  }

  .nav-btn:disabled,
  .action-btn:disabled {
    opacity: 0.4;
    cursor: not-allowed;
  }

  .option-btn {
    font-size: 11px;
    font-weight: 600;
    width: auto;
    padding: 0 6px;
  }

  .option-btn.active,
  .toggle-btn.active {
    background: var(--accent-blue);
    color: var(--bg-primary);
  }

  .action-btn {
    padding: 4px 10px;
    background: var(--bg-highlight);
    border: 1px solid var(--border-color);
    border-radius: 4px;
    color: var(--fg-secondary);
    font-size: 12px;
    cursor: pointer;
    transition: all 0.15s;
  }

  .action-btn:hover:not(:disabled) {
    background: var(--bg-primary);
    color: var(--fg-primary);
  }
</style>
