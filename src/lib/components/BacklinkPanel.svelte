<script lang="ts">
  import type { BacklinkDto } from '$lib/types';
  import HighlightText from './HighlightText.svelte';

  interface Props {
    backlinks: BacklinkDto[];
    onSelect: (uid: string) => void;
    onClose: () => void;
  }

  let { backlinks, onSelect, onClose }: Props = $props();

  let panelRef = $state<HTMLDivElement | null>(null);
  let focusedIndex = $state(0);

  // Track keyboard navigation to prevent mouse interference
  let isKeyboardNavigating = false;
  let keyboardNavTimeout: ReturnType<typeof setTimeout> | null = null;

  // Focus panel on mount
  $effect(() => {
    if (panelRef) {
      panelRef.focus();
    }
  });

  // Reset focused index when backlinks change
  $effect(() => {
    if (backlinks.length > 0) {
      focusedIndex = 0;
    }
  });

  function handleKeydown(e: KeyboardEvent) {
    let newIndex = focusedIndex;

    switch (e.key) {
      case 'Escape':
        e.preventDefault();
        e.stopPropagation();
        onClose();
        return;
      case 'ArrowDown':
        e.preventDefault();
        if (backlinks.length > 0) {
          newIndex = focusedIndex < backlinks.length - 1 ? focusedIndex + 1 : focusedIndex;
        }
        break;
      case 'ArrowUp':
        e.preventDefault();
        if (backlinks.length > 0) {
          newIndex = focusedIndex > 0 ? focusedIndex - 1 : 0;
        }
        break;
      case 'Enter':
        e.preventDefault();
        if (backlinks.length > 0 && focusedIndex >= 0) {
          handleSelect(backlinks[focusedIndex].uid);
        }
        return;
      default:
        return;
    }

    if (newIndex !== focusedIndex) {
      isKeyboardNavigating = true;
      if (keyboardNavTimeout) clearTimeout(keyboardNavTimeout);
      keyboardNavTimeout = setTimeout(() => {
        isKeyboardNavigating = false;
      }, 150);
      focusedIndex = newIndex;

      // Scroll into view
      const listItems = panelRef?.querySelectorAll('.backlink-item');
      if (listItems && listItems[newIndex]) {
        listItems[newIndex].scrollIntoView({ block: 'nearest', behavior: 'instant' });
      }
    }
  }

  function handleMouseEnter(index: number) {
    if (isKeyboardNavigating) return;
    focusedIndex = index;
  }

  function handleSelect(uid: string) {
    onSelect(uid);
    onClose();
  }

  // Find matching text in context for highlighting
  function getContextMatches(context: string): { start: number; end: number }[] {
    // Find [[...]] pattern in context
    const match = context.match(/\[\[([^\]]+)\]\]/);
    if (match && match.index !== undefined) {
      return [{ start: match.index, end: match.index + match[0].length }];
    }
    return [];
  }
</script>

<!-- svelte-ignore a11y_no_noninteractive_tabindex -->
<div
  class="backlink-panel"
  bind:this={panelRef}
  tabindex="0"
  onkeydown={handleKeydown}
  role="dialog"
  aria-label="Backlinks"
>
  <div class="backlink-header">
    <span class="header-title">
      <svg class="icon" width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
        <path d="M10 13a5 5 0 0 0 7.54.54l3-3a5 5 0 0 0-7.07-7.07l-1.72 1.71"></path>
        <path d="M14 11a5 5 0 0 0-7.54-.54l-3 3a5 5 0 0 0 7.07 7.07l1.71-1.71"></path>
      </svg>
      Backlinks
      <span class="count">({backlinks.length})</span>
    </span>
    <button class="close-btn" onclick={onClose} title="Close (Esc)">
      <svg width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
        <line x1="18" y1="6" x2="6" y2="18"></line>
        <line x1="6" y1="6" x2="18" y2="18"></line>
      </svg>
    </button>
  </div>

  <div class="backlink-content">
    {#if backlinks.length === 0}
      <div class="no-backlinks">
        <svg class="empty-icon" width="32" height="32" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.5">
          <circle cx="12" cy="12" r="10"></circle>
          <line x1="12" y1="8" x2="12" y2="12"></line>
          <line x1="12" y1="16" x2="12.01" y2="16"></line>
        </svg>
        <p>No notes link to this note</p>
      </div>
    {:else}
      <ul class="backlink-list" role="listbox">
        {#each backlinks as link, index (link.uid)}
          <li
            class="backlink-item"
            class:focused={focusedIndex === index}
            role="option"
            aria-selected={focusedIndex === index}
            onmouseenter={() => handleMouseEnter(index)}
          >
            <button onclick={() => handleSelect(link.uid)}>
              <span class="link-title">{link.title || 'Untitled'}</span>
              <span class="link-context">
                <HighlightText text={link.context} matches={getContextMatches(link.context)} />
              </span>
            </button>
          </li>
        {/each}
      </ul>
    {/if}
  </div>

  <div class="backlink-footer">
    <span class="hint">↑↓ Navigate • Enter Select • Esc Close</span>
  </div>
</div>

<style>
  .backlink-panel {
    position: absolute;
    top: 50px;
    right: 20px;
    width: 340px;
    max-height: 450px;
    background: var(--bg-secondary);
    border: 1px solid var(--border-color);
    border-radius: 10px;
    box-shadow: 0 8px 32px rgba(0, 0, 0, 0.3);
    z-index: 100;
    display: flex;
    flex-direction: column;
    overflow: hidden;
    outline: none;
  }

  .backlink-header {
    display: flex;
    align-items: center;
    justify-content: space-between;
    padding: 12px 14px;
    border-bottom: 1px solid var(--border-color);
    background: var(--bg-tertiary);
  }

  .header-title {
    display: flex;
    align-items: center;
    gap: 8px;
    font-size: 13px;
    font-weight: 600;
    color: var(--fg-primary);
  }

  .icon {
    color: var(--accent-cyan);
  }

  .count {
    color: var(--fg-muted);
    font-weight: normal;
  }

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

  .close-btn:hover {
    background: var(--bg-highlight);
    color: var(--fg-primary);
  }

  .backlink-content {
    flex: 1;
    overflow-y: auto;
    overscroll-behavior: contain;
  }

  .no-backlinks {
    display: flex;
    flex-direction: column;
    align-items: center;
    justify-content: center;
    padding: 40px 20px;
    color: var(--fg-muted);
    text-align: center;
  }

  .empty-icon {
    margin-bottom: 12px;
    opacity: 0.5;
  }

  .no-backlinks p {
    font-size: 13px;
  }

  .backlink-list {
    padding: 8px;
  }

  .backlink-item {
    border-radius: 6px;
    margin-bottom: 4px;
  }

  .backlink-item:last-child {
    margin-bottom: 0;
  }

  .backlink-item.focused {
    background: var(--bg-highlight);
  }

  .backlink-item button {
    display: flex;
    flex-direction: column;
    align-items: flex-start;
    width: 100%;
    padding: 10px 12px;
    text-align: left;
    background: none;
    border: none;
    cursor: pointer;
  }

  .backlink-item button:focus {
    outline: none;
  }

  .link-title {
    font-size: 14px;
    font-weight: 500;
    color: var(--fg-primary);
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
    width: 100%;
  }

  .link-context {
    font-size: 12px;
    color: var(--fg-muted);
    margin-top: 4px;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
    width: 100%;
  }

  .link-context :global(.highlight) {
    color: var(--accent-cyan);
    background: color-mix(in srgb, var(--accent-cyan) 20%, transparent);
    padding: 0 2px;
    border-radius: 2px;
  }

  .backlink-footer {
    padding: 8px 14px;
    border-top: 1px solid var(--border-color);
    background: var(--bg-tertiary);
  }

  .hint {
    font-size: 11px;
    color: var(--fg-muted);
  }
</style>
