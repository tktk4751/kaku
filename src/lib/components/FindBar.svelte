<script lang="ts">
  import type { EditorView } from '@codemirror/view';
  import {
    findMatches,
    highlightMatches,
    clearHighlights,
    scrollToMatch,
    replaceAt,
    replaceAll,
    type SearchMatch,
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
  let matches = $state<SearchMatch[]>([]);
  let currentIndex = $state(0);

  // Derived display for match count
  const matchDisplay = $derived(
    matches.length > 0 ? `${currentIndex + 1}/${matches.length}` : 'No results'
  );

  // Focus find input on mount
  $effect(() => {
    if (findInput) {
      setTimeout(() => findInput?.focus(), 10);
    }
  });

  // Update search when query or case sensitivity changes
  $effect(() => {
    if (editorView && query) {
      matches = findMatches(editorView, query, caseSensitive);
      currentIndex = matches.length > 0 ? 0 : -1;
      highlightMatches(editorView, matches, Math.max(0, currentIndex));
      if (matches.length > 0) {
        scrollToMatch(editorView, matches[0]);
      }
    } else if (editorView) {
      matches = [];
      currentIndex = -1;
      clearHighlights(editorView);
    }
  });

  function handleFindKeydown(e: KeyboardEvent) {
    if (e.key === 'Escape') {
      handleClose();
    } else if (e.key === 'Enter') {
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
      handleReplace();
    }
  }

  function goToNext() {
    if (matches.length === 0 || !editorView) return;
    currentIndex = (currentIndex + 1) % matches.length;
    highlightMatches(editorView, matches, currentIndex);
    scrollToMatch(editorView, matches[currentIndex]);
  }

  function goToPrev() {
    if (matches.length === 0 || !editorView) return;
    currentIndex = (currentIndex - 1 + matches.length) % matches.length;
    highlightMatches(editorView, matches, currentIndex);
    scrollToMatch(editorView, matches[currentIndex]);
  }

  function handleReplace() {
    if (!editorView || matches.length === 0 || currentIndex < 0) return;

    const match = matches[currentIndex];
    replaceAt(editorView, match.from, match.to, replacement);

    // Re-search after replace
    matches = findMatches(editorView, query, caseSensitive);
    if (matches.length > 0) {
      currentIndex = Math.min(currentIndex, matches.length - 1);
      highlightMatches(editorView, matches, currentIndex);
      scrollToMatch(editorView, matches[currentIndex]);
    } else {
      currentIndex = -1;
      clearHighlights(editorView);
    }
  }

  function handleReplaceAll() {
    if (!editorView || matches.length === 0) return;

    replaceAll(editorView, matches, replacement);
    matches = [];
    currentIndex = -1;
    clearHighlights(editorView);
  }

  function handleClose() {
    if (editorView) {
      clearHighlights(editorView);
      editorView.focus();
    }
    query = '';
    replacement = '';
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
      {#if query}
        {matchDisplay}
      {/if}
    </span>

    <!-- Navigation -->
    <button class="nav-btn" onclick={goToPrev} disabled={matches.length === 0} title="Previous (Shift+Enter)">
      <svg width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
        <polyline points="18 15 12 9 6 15"></polyline>
      </svg>
    </button>
    <button class="nav-btn" onclick={goToNext} disabled={matches.length === 0} title="Next (Enter)">
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
      <button class="action-btn" onclick={handleReplace} disabled={matches.length === 0} title="Replace">
        Replace
      </button>
      <button class="action-btn" onclick={handleReplaceAll} disabled={matches.length === 0} title="Replace all">
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
