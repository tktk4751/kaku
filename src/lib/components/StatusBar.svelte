<script lang="ts">
  interface Props {
    onMenuClick: () => void;
  }

  let { onMenuClick }: Props = $props();
</script>

<footer class="status-bar">
  <button
    class="menu-btn"
    onclick={onMenuClick}
    title="Menu (Ctrl+M)"
    aria-label="Open menu"
    aria-keyshortcuts="Control+m"
  >
    <svg width="16" height="16" viewBox="0 0 24 24" fill="currentColor" aria-hidden="true">
      <path d="M3 18h18v-2H3v2zm0-5h18v-2H3v2zm0-7v2h18V6H3z"/>
    </svg>
  </button>
</footer>

<style>
  .status-bar {
    display: flex;
    align-items: center;
    height: 28px;
    min-height: 28px;
    flex-shrink: 0;
    padding: 0 12px;
    background: var(--bg-secondary);
    border-top: 1px solid var(--border-color);
  }

  .menu-btn {
    display: flex;
    align-items: center;
    justify-content: center;
    width: 24px;
    height: 24px;
    border-radius: 4px;
    color: var(--fg-muted);
    transition: all 0.15s;
  }

  .menu-btn:hover {
    background: var(--bg-highlight);
    color: var(--fg-primary);
  }
</style>
