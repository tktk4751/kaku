<script lang="ts">
  import { loadNote } from '$lib/services/api';
  import TagChip from './TagChip.svelte';
  import type { NoteGalleryItemDto } from '$lib/types';

  interface Props {
    open: boolean;
    note: NoteGalleryItemDto;
    onClose: () => void;
    onEdit: () => void;
    onTagClick?: (tag: string) => void;
  }

  let {
    open,
    note,
    onClose,
    onEdit,
    onTagClick
  }: Props = $props();

  let content = $state('');
  let isLoading = $state(true);

  $effect(() => {
    if (open && note) {
      loadNoteContent();
    }
  });

  async function loadNoteContent() {
    isLoading = true;
    try {
      const fullNote = await loadNote(note.uid);
      content = fullNote.content;
    } catch (e) {
      content = 'Failed to load note content';
    } finally {
      isLoading = false;
    }
  }

  function handleOverlayClick(event: MouseEvent) {
    if (event.target === event.currentTarget) {
      onClose();
    }
  }

  function handleKeydown(event: KeyboardEvent) {
    if (!open) return;
    if (event.key === 'Escape') {
      onClose();
    } else if (event.key === 'Enter' && (event.ctrlKey || event.metaKey)) {
      event.preventDefault();
      onEdit();
    }
  }

  function formatDate(dateStr: string): string {
    const date = new Date(dateStr.replace(' ', 'T') + 'Z');
    return date.toLocaleDateString(undefined, {
      year: 'numeric',
      month: 'short',
      day: 'numeric',
      hour: '2-digit',
      minute: '2-digit',
    });
  }

  function handleTagClick(tag: string) {
    onClose();
    onTagClick?.(tag);
  }
</script>

<svelte:window onkeydown={handleKeydown} />

{#if open}
  <!-- svelte-ignore a11y_click_events_have_key_events a11y_no_static_element_interactions -->
  <div class="modal-overlay" onclick={handleOverlayClick}>
    <div class="modal" role="dialog" aria-labelledby="preview-title" aria-modal="true">
      <header class="modal-header">
        <h2 id="preview-title" class="modal-title">{note.title || 'Untitled'}</h2>
        <div class="header-actions">
          <button class="edit-btn" onclick={onEdit} aria-label="Edit note">
            <svg width="18" height="18" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
              <path d="M12 20h9"></path>
              <path d="M16.5 3.5a2.1 2.1 0 0 1 3 3L7 19l-4 1 1-4 12.5-12.5z"></path>
            </svg>
            <span>Edit</span>
          </button>
          <button class="close-btn" onclick={onClose} aria-label="Close">
            <svg width="20" height="20" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
              <line x1="18" y1="6" x2="6" y2="18"></line>
              <line x1="6" y1="6" x2="18" y2="18"></line>
            </svg>
          </button>
        </div>
      </header>

      <div class="modal-meta">
        <time class="meta-date">Updated: {formatDate(note.updated_at)}</time>
        {#if note.tags.length > 0}
          <div class="meta-tags">
            {#each note.tags as tag}
              <TagChip {tag} onclick={() => handleTagClick(tag)} />
            {/each}
          </div>
        {/if}
      </div>

      <div class="modal-body">
        {#if isLoading}
          <div class="loading">Loading...</div>
        {:else}
          <pre class="content">{content}</pre>
        {/if}
      </div>

      <footer class="modal-footer">
        <span class="hint">Ctrl+Enter to edit</span>
        <div class="actions">
          <button class="btn btn-secondary" onclick={onClose}>Close</button>
        </div>
      </footer>
    </div>
  </div>
{/if}

<style>
  .modal-overlay {
    position: fixed;
    inset: 0;
    background: rgba(0, 0, 0, 0.5);
    display: flex;
    align-items: center;
    justify-content: center;
    z-index: var(--layer-4, 1000);
    animation: fade-in var(--duration-fast, 150ms) var(--ease-2, ease);
  }

  @keyframes fade-in {
    from { opacity: 0; }
    to { opacity: 1; }
  }

  @keyframes scale-up {
    from {
      opacity: 0;
      transform: scale(0.95);
    }
    to {
      opacity: 1;
      transform: scale(1);
    }
  }

  .modal {
    background: var(--bg-secondary);
    border: 1px solid var(--border-color);
    border-radius: var(--radius-3, 12px);
    box-shadow: var(--shadow-5, 0 25px 50px -12px rgba(0,0,0,0.25));
    max-width: 700px;
    width: 90vw;
    max-height: 85vh;
    display: flex;
    flex-direction: column;
    animation: scale-up var(--duration-fast, 150ms) var(--ease-2, ease);
  }

  .modal-header {
    display: flex;
    align-items: flex-start;
    justify-content: space-between;
    padding: var(--size-4, 16px);
    border-bottom: 1px solid var(--border-color);
  }

  .modal-title {
    font-size: var(--font-size-xl, 20px);
    font-weight: var(--font-weight-semibold, 600);
    color: var(--fg-primary);
    margin: 0;
    padding-right: var(--size-4, 16px);
    word-break: break-word;
  }

  .header-actions {
    display: flex;
    align-items: center;
    gap: var(--size-2, 8px);
    flex-shrink: 0;
  }

  .edit-btn {
    display: inline-flex;
    align-items: center;
    gap: var(--size-1, 4px);
    padding: var(--size-2, 8px) var(--size-3, 12px);
    font-size: var(--font-size-sm, 14px);
    font-weight: var(--font-weight-medium, 500);
    color: var(--fg-primary);
    background: var(--bg-tertiary);
    border: 1px solid var(--border-color);
    border-radius: var(--radius-2, 8px);
    transition: all var(--duration-fast, 150ms) var(--ease-2, ease);
  }

  .edit-btn:hover {
    background: var(--bg-highlight);
  }

  .close-btn {
    display: flex;
    align-items: center;
    justify-content: center;
    padding: var(--size-2, 8px);
    border-radius: var(--radius-2, 8px);
    color: var(--fg-muted);
    transition: all var(--duration-fast, 150ms) var(--ease-2, ease);
    flex-shrink: 0;
  }

  .close-btn:hover {
    background: var(--bg-highlight);
    color: var(--fg-primary);
  }

  .modal-meta {
    display: flex;
    flex-wrap: wrap;
    align-items: center;
    gap: var(--size-2, 8px);
    padding: var(--size-3, 12px) var(--size-4, 16px);
    border-bottom: 1px solid var(--border-color);
    background: var(--bg-tertiary);
  }

  .meta-date {
    font-size: var(--font-size-xs, 12px);
    color: var(--fg-muted);
  }

  .meta-tags {
    display: flex;
    flex-wrap: wrap;
    gap: var(--size-1, 4px);
  }

  .modal-body {
    flex: 1;
    overflow-y: auto;
    padding: var(--size-4, 16px);
  }

  .loading {
    display: flex;
    align-items: center;
    justify-content: center;
    padding: var(--size-8, 32px);
    color: var(--fg-muted);
  }

  .content {
    white-space: pre-wrap;
    word-break: break-word;
    font-size: var(--font-size-sm, 14px);
    line-height: 1.75;
    font-family: var(--font-family-mono, 'SF Mono', 'Fira Code', monospace);
    margin: 0;
    color: var(--fg-primary);
  }

  .modal-footer {
    display: flex;
    align-items: center;
    justify-content: space-between;
    padding: var(--size-4, 16px);
    border-top: 1px solid var(--border-color);
  }

  .hint {
    font-size: var(--font-size-xs, 12px);
    color: var(--fg-muted);
  }

  .actions {
    display: flex;
    gap: var(--size-2, 8px);
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

  .btn-primary {
    background: var(--accent-blue);
    color: #ffffff;
    border: none;
  }

  .btn-primary:hover {
    filter: brightness(1.1);
  }
</style>
