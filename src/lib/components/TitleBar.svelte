<script lang="ts">
  import { invoke } from '@tauri-apps/api/core';

  interface Props {
    onClose?: () => Promise<void>;
    onMenuClick?: () => void;
  }

  let { onClose, onMenuClick }: Props = $props();

  async function handleClose(event: MouseEvent) {
    event.stopPropagation();

    // Call the onClose callback if provided (for saving before close)
    if (onClose) {
      await onClose();
    }

    // Hide the window (not quit)
    await invoke('hide_window');
  }

  async function startDrag(event: MouseEvent) {
    // Only start drag if clicking on the title bar itself, not on buttons
    const target = event.target as HTMLElement;
    if (target.closest('button')) return;
    if (event.button !== 0) return;

    const { getCurrentWindow } = await import('@tauri-apps/api/window');
    const appWindow = getCurrentWindow();
    await appWindow.startDragging();
  }
</script>

<!-- svelte-ignore a11y_no_static_element_interactions -->
<header class="title-bar" onmousedown={startDrag}>
  <div class="title-left">
    <button
      class="control-btn menu"
      onclick={(e) => { e.stopPropagation(); onMenuClick?.(); }}
      onmousedown={(e) => e.stopPropagation()}
      title="Menu (Ctrl+M)"
      aria-label="Open menu"
    >
      <svg width="16" height="16" viewBox="0 0 24 24" fill="currentColor">
        <path d="M3 18h18v-2H3v2zm0-5h18v-2H3v2zm0-7v2h18V6H3z"/>
      </svg>
    </button>
  </div>
  <div class="window-controls">
    <button
      class="control-btn close"
      onclick={(e) => handleClose(e)}
      onmousedown={(e) => e.stopPropagation()}
      title="Hide"
      aria-label="Hide window"
    >
      <svg width="12" height="12" viewBox="0 0 12 12">
        <path d="M3 3L9 9M9 3L3 9" stroke="currentColor" stroke-width="1.2" fill="none"/>
      </svg>
    </button>
  </div>
</header>

<style>
  .title-bar {
    display: flex;
    flex-direction: row;
    align-items: center;
    justify-content: space-between;
    width: 100%;
    height: 32px;
    min-height: 32px;
    flex-shrink: 0;
    padding: 0 8px 0 12px;
    background: var(--bg-secondary);
    border-bottom: 1px solid var(--border-color);
    user-select: none;
    -webkit-user-select: none;
    cursor: default;
    box-sizing: border-box;
  }

  .title-left {
    display: flex;
    align-items: center;
  }

  .control-btn.menu:hover {
    background: var(--bg-highlight);
    color: var(--fg-primary);
  }

  .window-controls {
    display: flex;
    gap: 4px;
  }

  .control-btn {
    display: flex;
    align-items: center;
    justify-content: center;
    width: 28px;
    height: 28px;
    border-radius: 6px;
    color: var(--fg-muted);
    transition: all 0.15s;
  }

  .control-btn.close:hover {
    background: var(--accent-red);
    color: #fff;
  }
</style>
