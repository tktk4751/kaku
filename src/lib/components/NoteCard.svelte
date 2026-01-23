<script lang="ts">
  import TagChip from './TagChip.svelte';
  import type { NoteGalleryItemDto } from '$lib/types';

  interface Props {
    note: NoteGalleryItemDto;
    onclick: () => void;
    ondblclick: () => void;
    onTagClick?: (tag: string) => void;
  }

  let { note, onclick, ondblclick, onTagClick }: Props = $props();

  function formatDate(dateStr: string): string {
    const date = new Date(dateStr.replace(' ', 'T') + 'Z');
    const now = new Date();
    const diff = now.getTime() - date.getTime();
    const days = Math.floor(diff / (1000 * 60 * 60 * 24));

    if (days === 0) {
      return date.toLocaleTimeString(undefined, { hour: '2-digit', minute: '2-digit' });
    } else if (days === 1) {
      return 'Yesterday';
    } else if (days < 7) {
      return `${days} days ago`;
    } else {
      return date.toLocaleDateString(undefined, { month: 'short', day: 'numeric' });
    }
  }

  function handleTagClick(tag: string) {
    onTagClick?.(tag);
  }
</script>

<!-- svelte-ignore a11y_no_noninteractive_element_to_interactive_role -->
<!-- svelte-ignore a11y_click_events_have_key_events -->
<article
  class="note-card"
  role="button"
  tabindex="0"
  onclick={onclick}
  ondblclick={ondblclick}
  onkeydown={(e) => e.key === 'Enter' && onclick()}
>
  <header class="card-header">
    <h3 class="card-title">{note.title || 'Untitled'}</h3>
  </header>

  {#if note.preview}
    <div class="card-content">
      <p class="card-preview">{note.preview}</p>
    </div>
  {/if}

  <footer class="card-footer">
    {#if note.tags.length > 0}
      <div class="card-tags">
        {#each note.tags.slice(0, 3) as tag}
          <TagChip {tag} onclick={() => handleTagClick(tag)} />
        {/each}
        {#if note.tags.length > 3}
          <span class="more-tags">+{note.tags.length - 3}</span>
        {/if}
      </div>
    {/if}
    <time class="card-date">{formatDate(note.updated_at)}</time>
  </footer>
</article>

<style>
  .note-card {
    display: flex;
    flex-direction: column;
    gap: var(--size-3, 12px);
    padding: var(--size-4, 16px);
    background: var(--bg-secondary);
    border: 1px solid var(--border-color);
    border-radius: var(--radius-3, 12px);
    cursor: pointer;
    break-inside: avoid;
    margin-bottom: var(--size-4, 16px);
    /* Static shadow - no transition for scroll performance */
    box-shadow: var(--shadow-2);
    /* GPU layer for smooth scrolling */
    transform: translateZ(0);
    will-change: transform;
  }

  .note-card:hover {
    border-color: var(--accent-blue);
    /* Only change border on hover, shadow stays static */
  }

  .note-card:active {
    opacity: 0.9;
  }

  .note-card:focus-visible {
    outline: 2px solid var(--accent-blue);
    outline-offset: 2px;
  }

  .card-header {
    padding: 0;
  }

  .card-title {
    font-size: var(--font-size-base, 16px);
    font-weight: var(--font-weight-semibold, 600);
    line-height: 1.4;
    color: var(--fg-primary);
    display: -webkit-box;
    -webkit-line-clamp: 2;
    -webkit-box-orient: vertical;
    overflow: hidden;
    margin: 0;
  }

  .card-content {
    padding: 0;
  }

  .card-preview {
    font-size: var(--font-size-sm, 14px);
    line-height: 1.6;
    color: var(--fg-secondary);
    display: -webkit-box;
    -webkit-line-clamp: 6;
    -webkit-box-orient: vertical;
    overflow: hidden;
    margin: 0;
  }

  .card-footer {
    display: flex;
    flex-wrap: wrap;
    align-items: center;
    gap: var(--size-2, 8px);
    padding: 0;
    margin-top: auto;
    padding-top: var(--size-2, 8px);
    border-top: 1px solid var(--border-color);
  }

  .card-tags {
    display: flex;
    flex-wrap: wrap;
    gap: 6px;
    flex: 1;
  }

  .more-tags {
    font-size: var(--font-size-xs, 12px);
    color: var(--fg-muted);
    padding: var(--size-1, 4px) var(--size-2, 8px);
    background: var(--bg-tertiary);
    border-radius: var(--radius-round, 9999px);
  }

  .card-date {
    font-size: var(--font-size-xs, 12px);
    color: var(--fg-muted);
    white-space: nowrap;
    display: flex;
    align-items: center;
    gap: var(--size-1, 4px);
  }

  .card-date::before {
    content: '';
    width: 4px;
    height: 4px;
    border-radius: 50%;
    background: var(--fg-muted);
    opacity: 0.5;
  }
</style>
