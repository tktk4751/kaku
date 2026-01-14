<script lang="ts">
  import { onMount } from 'svelte';
  import type { Snippet } from 'svelte';

  interface Props {
    children: Snippet;
    fallback?: Snippet<[{ error: Error; reset: () => void }]>;
  }

  let { children, fallback }: Props = $props();

  let error = $state<Error | null>(null);

  function handleError(event: ErrorEvent) {
    error = event.error || new Error(event.message);
    event.preventDefault();
  }

  function handleUnhandledRejection(event: PromiseRejectionEvent) {
    error = event.reason instanceof Error ? event.reason : new Error(String(event.reason));
    event.preventDefault();
  }

  function reset() {
    error = null;
  }

  onMount(() => {
    window.addEventListener('error', handleError);
    window.addEventListener('unhandledrejection', handleUnhandledRejection);

    return () => {
      window.removeEventListener('error', handleError);
      window.removeEventListener('unhandledrejection', handleUnhandledRejection);
    };
  });
</script>

{#if error}
  {#if fallback}
    {@render fallback({ error, reset })}
  {:else}
    <div class="error-boundary">
      <div class="error-content">
        <h2>Something went wrong</h2>
        <p class="error-message">{error.message}</p>
        <button class="retry-btn" onclick={reset}>Try Again</button>
      </div>
    </div>
  {/if}
{:else}
  {@render children()}
{/if}

<style>
  .error-boundary {
    display: flex;
    align-items: center;
    justify-content: center;
    height: 100%;
    padding: 24px;
    background: var(--bg-primary);
  }

  .error-content {
    text-align: center;
    max-width: 400px;
  }

  h2 {
    font-size: 20px;
    font-weight: 600;
    color: var(--fg-primary);
    margin-bottom: 12px;
  }

  .error-message {
    font-size: 14px;
    color: var(--fg-secondary);
    margin-bottom: 24px;
    padding: 12px;
    background: var(--bg-secondary);
    border-radius: 8px;
    font-family: monospace;
    word-break: break-word;
  }

  .retry-btn {
    padding: 10px 24px;
    background: var(--accent-blue);
    color: #fff;
    border-radius: 6px;
    font-size: 14px;
    font-weight: 500;
    transition: opacity 0.15s;
  }

  .retry-btn:hover {
    opacity: 0.9;
  }
</style>
