<script lang="ts">
  interface Props {
    value: string;
    onInput: (value: string) => void;
    onClear: () => void;
    isSearching?: boolean;
  }

  let { value, onInput, onClear, isSearching = false }: Props = $props();
  let inputRef: HTMLInputElement;

  function handleInput(e: Event) {
    const target = e.target as HTMLInputElement;
    onInput(target.value);
  }

  function handleKeydown(e: KeyboardEvent) {
    if (e.key === 'Escape') {
      onClear();
      inputRef?.blur();
    }
  }

  export function focus() {
    inputRef?.focus();
  }
</script>

<div class="search-input">
  <svg class="search-icon" width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
    <circle cx="11" cy="11" r="8"></circle>
    <line x1="21" y1="21" x2="16.65" y2="16.65"></line>
  </svg>

  <input
    bind:this={inputRef}
    type="text"
    {value}
    placeholder="Search..."
    oninput={handleInput}
    onkeydown={handleKeydown}
    class:searching={isSearching}
  />

  {#if value}
    <button class="clear-btn" onclick={onClear} aria-label="Clear search">
      <svg width="12" height="12" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
        <line x1="18" y1="6" x2="6" y2="18"></line>
        <line x1="6" y1="6" x2="18" y2="18"></line>
      </svg>
    </button>
  {/if}
</div>

<style>
  .search-input {
    display: flex;
    align-items: center;
    gap: 8px;
    padding: 8px 12px;
    background: var(--bg-primary);
    border: 1px solid var(--border-color);
    border-radius: 6px;
    margin: 8px 12px;
  }

  .search-input:focus-within {
    border-color: var(--accent-blue);
  }

  .search-icon {
    flex-shrink: 0;
    color: var(--fg-muted);
  }

  input {
    flex: 1;
    border: none;
    background: transparent;
    color: var(--fg-primary);
    font-size: 13px;
    outline: none;
    min-width: 0;
  }

  input::placeholder {
    color: var(--fg-muted);
  }

  input.searching {
    opacity: 0.7;
  }

  .clear-btn {
    display: flex;
    align-items: center;
    justify-content: center;
    padding: 4px;
    color: var(--fg-muted);
    border-radius: 4px;
    transition: all 0.15s;
  }

  .clear-btn:hover {
    background: var(--bg-highlight);
    color: var(--fg-primary);
  }
</style>
