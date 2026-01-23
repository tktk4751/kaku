<script lang="ts">
  interface Props {
    tag: string;
    removable?: boolean;
    active?: boolean;
    onRemove?: () => void;
    onclick?: () => void;
  }

  let {
    tag,
    removable = false,
    active = false,
    onRemove,
    onclick,
  }: Props = $props();

  function handleRemove(event: MouseEvent) {
    event.stopPropagation();
    onRemove?.();
  }

  function handleClick(event: MouseEvent) {
    event.stopPropagation();
    onclick?.();
  }

  function handleKeydown(event: KeyboardEvent) {
    if (event.key === 'Enter' || event.key === ' ') {
      event.preventDefault();
      event.stopPropagation();
      onclick?.();
    }
  }
</script>

<span
  class="tag-chip"
  class:active
  class:clickable={!!onclick}
  onclick={onclick ? handleClick : undefined}
  onkeydown={onclick ? handleKeydown : undefined}
  role={onclick ? "button" : undefined}
  tabindex={onclick ? 0 : undefined}
>
  <svg class="tag-icon" width="12" height="12" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round">
    <path d="M20.59 13.41l-7.17 7.17a2 2 0 0 1-2.83 0L2 12V2h10l8.59 8.59a2 2 0 0 1 0 2.82z"></path>
    <line x1="7" y1="7" x2="7.01" y2="7"></line>
  </svg>
  <span class="tag-text">{tag}</span>
  {#if removable}
    <button
      type="button"
      class="remove-btn"
      onclick={handleRemove}
      aria-label="Remove tag {tag}"
    >
      <svg width="12" height="12" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round">
        <line x1="18" y1="6" x2="6" y2="18"></line>
        <line x1="6" y1="6" x2="18" y2="18"></line>
      </svg>
    </button>
  {/if}
</span>

<style>
  .tag-chip {
    display: inline-flex;
    align-items: center;
    gap: var(--size-1, 4px);
    padding: var(--size-1, 4px) var(--size-2, 8px);
    font-size: var(--font-size-xs, 12px);
    font-weight: var(--font-weight-medium, 500);
    line-height: 1;
    border-radius: var(--radius-round, 9999px);
    background: var(--bg-tertiary);
    color: var(--fg-secondary);
    max-width: 180px;
    /* Only transition background-color for better performance */
    transition: background-color 0.15s ease;
  }

  .tag-chip.active {
    background: var(--accent-blue);
    color: #ffffff;
  }

  .tag-chip.clickable {
    cursor: pointer;
  }

  .tag-chip.clickable:hover {
    background: var(--bg-highlight);
    color: var(--fg-primary);
  }

  .tag-chip.clickable.active:hover {
    background: var(--accent-blue);
    filter: brightness(1.1);
  }

  .tag-chip:focus-visible {
    outline: 2px solid var(--accent-blue);
    outline-offset: 2px;
  }

  .tag-icon {
    flex-shrink: 0;
    opacity: 0.7;
  }

  .tag-text {
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  .remove-btn {
    display: flex;
    align-items: center;
    justify-content: center;
    margin-left: var(--size-1, 4px);
    padding: 2px;
    border-radius: 50%;
    color: inherit;
    opacity: 0.7;
  }

  .remove-btn:hover {
    background: var(--accent-red);
    color: #ffffff;
    opacity: 1;
  }

  .remove-btn:focus-visible {
    outline: 2px solid var(--accent-blue);
    outline-offset: 1px;
  }
</style>
