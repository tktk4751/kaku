<script lang="ts">
  import { tick, onDestroy } from 'svelte';
  import { getAllTags, updateNoteTags, getNoteTags } from '$lib/services/api';
  import { handleError, handleSuccess } from '$lib/utils/errorHandler';
  import TagChip from './TagChip.svelte';

  interface Props {
    open: boolean;
    noteUid: string;
    onClose: () => void;
    onSave?: () => void;
  }

  let {
    open = $bindable(),
    noteUid,
    onClose,
    onSave,
  }: Props = $props();

  let inputElement = $state<HTMLInputElement | null>(null);
  let inputValue = $state('');
  let currentTags = $state<string[]>([]);
  let allTags = $state<string[]>([]);
  let suggestions = $state<string[]>([]);
  let focusedIndex = $state(-1);
  let isSaving = $state(false);

  // Load tags when dialog opens
  $effect(() => {
    if (open && noteUid) {
      loadTags();
    }
  });

  async function loadTags() {
    try {
      const [noteTagsResult, allTagsResult] = await Promise.all([
        getNoteTags(noteUid),
        getAllTags(),
      ]);
      currentTags = noteTagsResult.frontmatter_tags;
      allTags = allTagsResult;
    } catch (e) {
      handleError(e, 'Failed to load tags');
    }

    await tick();
    inputElement?.focus();
  }

  function handleClose() {
    inputValue = '';
    suggestions = [];
    focusedIndex = -1;
    open = false;
    onClose();
  }

  function handleOverlayClick(event: MouseEvent) {
    if (event.target === event.currentTarget) {
      handleClose();
    }
  }

  function updateSuggestions() {
    if (!inputValue.trim()) {
      suggestions = [];
      focusedIndex = -1;
      return;
    }

    const query = inputValue.toLowerCase();
    suggestions = allTags
      .filter(tag =>
        tag.toLowerCase().includes(query) &&
        !currentTags.includes(tag)
      )
      .slice(0, 5);
    focusedIndex = suggestions.length > 0 ? 0 : -1;
  }

  function addTag(tag: string) {
    const normalizedTag = tag.trim().toLowerCase();
    if (normalizedTag && !currentTags.includes(normalizedTag)) {
      currentTags = [...currentTags, normalizedTag];
    }
    inputValue = '';
    suggestions = [];
    focusedIndex = -1;
    inputElement?.focus();
  }

  function removeTag(tag: string) {
    currentTags = currentTags.filter(t => t !== tag);
  }

  function handleInput() {
    updateSuggestions();
  }

  function handleKeydown(e: KeyboardEvent) {
    switch (e.key) {
      case 'Enter':
        e.preventDefault();
        if (focusedIndex >= 0 && suggestions[focusedIndex]) {
          addTag(suggestions[focusedIndex]);
        } else if (inputValue.trim()) {
          addTag(inputValue);
        }
        break;

      case 'Backspace':
        if (!inputValue && currentTags.length > 0) {
          removeTag(currentTags[currentTags.length - 1]);
        }
        break;

      case 'ArrowDown':
        e.preventDefault();
        if (suggestions.length > 0) {
          focusedIndex = Math.min(focusedIndex + 1, suggestions.length - 1);
        }
        break;

      case 'ArrowUp':
        e.preventDefault();
        if (suggestions.length > 0) {
          focusedIndex = Math.max(focusedIndex - 1, 0);
        }
        break;

      case 'Escape':
        if (suggestions.length > 0) {
          e.stopPropagation();
          suggestions = [];
          focusedIndex = -1;
        } else {
          handleClose();
        }
        break;
    }
  }

  async function handleSave() {
    isSaving = true;
    try {
      await updateNoteTags(noteUid, currentTags);
      handleSuccess('Tags updated');
      onSave?.();
      handleClose();
    } catch (e) {
      handleError(e, 'Failed to save tags');
    } finally {
      isSaving = false;
    }
  }
</script>

{#if open}
  <!-- svelte-ignore a11y_click_events_have_key_events a11y_no_static_element_interactions -->
  <div class="modal-overlay" onclick={handleOverlayClick}>
    <div class="modal" role="dialog" aria-labelledby="tag-dialog-title" aria-modal="true">
      <header class="modal-header">
        <h2 id="tag-dialog-title" class="modal-title">Edit Tags</h2>
        <p class="modal-description">Add or remove tags for this note</p>
      </header>

      <div class="modal-body">
        <!-- Current tags -->
        <div class="tags-container">
          {#if currentTags.length === 0}
            <span class="no-tags">No tags</span>
          {:else}
            {#each currentTags as tag (tag)}
              <TagChip {tag} removable onRemove={() => removeTag(tag)} />
            {/each}
          {/if}
        </div>

        <!-- Input with autocomplete -->
        <div class="input-container">
          <input
            bind:this={inputElement}
            bind:value={inputValue}
            class="tag-input"
            placeholder="Type to add tag..."
            oninput={handleInput}
            onkeydown={handleKeydown}
          />

          {#if suggestions.length > 0}
            <div class="suggestions">
              {#each suggestions as suggestion, index (suggestion)}
                <button
                  type="button"
                  class="suggestion-item"
                  class:focused={focusedIndex === index}
                  onclick={() => addTag(suggestion)}
                  onmouseenter={() => focusedIndex = index}
                >
                  {suggestion}
                </button>
              {/each}
            </div>
          {/if}
        </div>

        <p class="hint">Press Enter to add, Backspace to remove last, Escape to close suggestions</p>
      </div>

      <footer class="modal-footer">
        <button class="btn btn-secondary" onclick={handleClose}>Cancel</button>
        <button class="btn btn-primary" onclick={handleSave} disabled={isSaving}>
          {isSaving ? 'Saving...' : 'Save'}
        </button>
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
    max-width: 400px;
    width: 90vw;
    animation: scale-up var(--duration-fast, 150ms) var(--ease-2, ease);
  }

  .modal-header {
    padding: var(--size-4, 16px);
    border-bottom: 1px solid var(--border-color);
  }

  .modal-title {
    font-size: var(--font-size-lg, 18px);
    font-weight: var(--font-weight-semibold, 600);
    color: var(--fg-primary);
    margin: 0;
  }

  .modal-description {
    font-size: var(--font-size-sm, 14px);
    color: var(--fg-muted);
    margin: var(--size-1, 4px) 0 0 0;
  }

  .modal-body {
    padding: var(--size-4, 16px);
    display: flex;
    flex-direction: column;
    gap: var(--size-4, 16px);
  }

  .tags-container {
    display: flex;
    flex-wrap: wrap;
    gap: var(--size-2, 8px);
    min-height: 44px;
    padding: var(--size-3, 12px);
    border: 1px solid var(--border-color);
    border-radius: var(--radius-2, 8px);
    background: var(--bg-tertiary);
  }

  .no-tags {
    font-size: var(--font-size-sm, 14px);
    color: var(--fg-muted);
  }

  .input-container {
    position: relative;
  }

  .tag-input {
    width: 100%;
    padding: var(--size-2, 8px) var(--size-3, 12px);
    font-family: inherit;
    font-size: var(--font-size-sm, 14px);
    background: var(--bg-primary);
    color: var(--fg-primary);
    border: 1px solid var(--border-color);
    border-radius: var(--radius-2, 8px);
    outline: none;
    transition: border-color var(--duration-fast, 150ms) var(--ease-2, ease);
  }

  .tag-input:focus {
    border-color: var(--accent-blue);
  }

  .tag-input::placeholder {
    color: var(--fg-muted);
  }

  .suggestions {
    position: absolute;
    z-index: 50;
    width: 100%;
    margin-top: var(--size-1, 4px);
    border: 1px solid var(--border-color);
    border-radius: var(--radius-2, 8px);
    background: var(--bg-secondary);
    box-shadow: var(--shadow-3, 0 4px 12px rgba(0,0,0,0.15));
    overflow: hidden;
    max-height: 200px;
    overflow-y: auto;
  }

  .suggestion-item {
    width: 100%;
    padding: var(--size-2, 8px) var(--size-3, 12px);
    text-align: left;
    font-size: var(--font-size-sm, 14px);
    color: var(--fg-primary);
    background: transparent;
    transition: background var(--duration-fast, 150ms) var(--ease-2, ease);
  }

  .suggestion-item:hover,
  .suggestion-item.focused {
    background: var(--bg-highlight);
  }

  .hint {
    font-size: var(--font-size-xs, 12px);
    color: var(--fg-muted);
    margin: 0;
  }

  .modal-footer {
    display: flex;
    justify-content: flex-end;
    gap: var(--size-2, 8px);
    padding: var(--size-4, 16px);
    border-top: 1px solid var(--border-color);
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

  .btn:disabled {
    opacity: 0.5;
    cursor: not-allowed;
  }

  .btn-secondary {
    background: var(--bg-tertiary);
    color: var(--fg-primary);
    border: 1px solid var(--border-color);
  }

  .btn-secondary:hover:not(:disabled) {
    background: var(--bg-highlight);
  }

  .btn-primary {
    background: var(--accent-blue);
    color: #ffffff;
    border: none;
  }

  .btn-primary:hover:not(:disabled) {
    filter: brightness(1.1);
  }
</style>
