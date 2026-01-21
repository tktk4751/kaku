<script lang="ts">
  import type { MatchRange } from '$lib/types';

  interface Props {
    text: string;
    matches: MatchRange[];
  }

  let { text, matches }: Props = $props();

  // マッチ位置でテキストを分割（文字単位）
  function getSegments(text: string, matches: MatchRange[]): Array<{ text: string; highlight: boolean }> {
    if (!matches.length) {
      return [{ text, highlight: false }];
    }

    const chars = [...text];
    const segments: Array<{ text: string; highlight: boolean }> = [];
    let lastEnd = 0;

    // ソートして重複を除去
    const sortedMatches = [...matches].sort((a, b) => a.start - b.start);

    for (const match of sortedMatches) {
      const start = Math.min(match.start, chars.length);
      const end = Math.min(match.end, chars.length);

      if (start > lastEnd) {
        segments.push({
          text: chars.slice(lastEnd, start).join(''),
          highlight: false,
        });
      }

      if (end > start) {
        segments.push({
          text: chars.slice(start, end).join(''),
          highlight: true,
        });
      }

      lastEnd = Math.max(lastEnd, end);
    }

    if (lastEnd < chars.length) {
      segments.push({
        text: chars.slice(lastEnd).join(''),
        highlight: false,
      });
    }

    return segments;
  }

  const segments = $derived(getSegments(text, matches));
</script>

<span class="highlight-text">
  {#each segments as segment, i (i)}
    {#if segment.highlight}
      <mark class="match">{segment.text}</mark>
    {:else}
      {segment.text}
    {/if}
  {/each}
</span>

<style>
  .highlight-text {
    display: inline;
  }

  .match {
    background: var(--accent-yellow-dim, rgba(224, 175, 104, 0.3));
    color: var(--fg-primary);
    border-radius: 2px;
    padding: 0 1px;
  }
</style>
