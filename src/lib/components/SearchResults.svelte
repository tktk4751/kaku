<script lang="ts">
  import type { SearchResultDto } from '$lib/types';
  import HighlightText from './HighlightText.svelte';

  interface Props {
    results: SearchResultDto[];
    onSelect: (uid: string) => void;
    focusedIndex?: number;
    onMouseEnter?: (index: number) => void;
  }

  let { results, onSelect, focusedIndex = -1, onMouseEnter }: Props = $props();

  function handleMouseEnter(index: number) {
    onMouseEnter?.(index);
  }
</script>

<ul class="search-results" role="listbox">
  {#each results as result, index (result.uid)}
    <!-- svelte-ignore a11y_no_static_element_interactions -->
    <li
      class="result-item"
      class:focused={focusedIndex === index}
      role="option"
      aria-selected={focusedIndex === index}
      onmouseenter={() => handleMouseEnter(index)}
    >
      <button onclick={() => onSelect(result.uid)}>
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

<style>
  .search-results {
    flex: 1;
    overflow-y: auto;
    padding: 4px 8px;
    overscroll-behavior: contain;
  }

  .result-item {
    border-radius: 6px;
    transition: background 0.15s;
  }

  .result-item.focused {
    background: var(--bg-highlight);
    outline: 2px solid var(--accent-blue);
    outline-offset: -2px;
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
    font-size: 13px;
    font-weight: 500;
    color: var(--fg-primary);
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
    width: 100%;
  }

  .result-preview {
    font-size: 11px;
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
    font-size: 13px;
  }
</style>
