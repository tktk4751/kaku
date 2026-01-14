<script lang="ts">
  import { onMount } from 'svelte';

  interface Props {
    message: string;
    type?: 'error' | 'success' | 'info';
    duration?: number;
    onClose: () => void;
  }

  let { message, type = 'info', duration = 5000, onClose }: Props = $props();

  onMount(() => {
    if (duration > 0) {
      const timer = setTimeout(onClose, duration);
      return () => clearTimeout(timer);
    }
  });
</script>

<div class="toast" class:error={type === 'error'} class:success={type === 'success'} role="alert">
  <span class="toast-message">{message}</span>
  <button class="toast-close" onclick={onClose} aria-label="Close">
    <svg width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
      <line x1="18" y1="6" x2="6" y2="18"></line>
      <line x1="6" y1="6" x2="18" y2="18"></line>
    </svg>
  </button>
</div>

<style>
  .toast {
    display: flex;
    align-items: center;
    gap: 12px;
    padding: 12px 16px;
    background: var(--bg-secondary);
    border: 1px solid var(--border-color);
    border-radius: 8px;
    box-shadow: 0 4px 12px rgba(0, 0, 0, 0.3);
    animation: slideIn 0.2s ease;
  }

  .toast.error {
    border-color: var(--accent-red, #f7768e);
    background: rgba(247, 118, 142, 0.1);
  }

  .toast.success {
    border-color: var(--accent-green, #9ece6a);
    background: rgba(158, 206, 106, 0.1);
  }

  @keyframes slideIn {
    from {
      transform: translateY(-20px);
      opacity: 0;
    }
    to {
      transform: translateY(0);
      opacity: 1;
    }
  }

  .toast-message {
    flex: 1;
    font-size: 13px;
    color: var(--fg-primary);
  }

  .toast-close {
    display: flex;
    align-items: center;
    justify-content: center;
    width: 24px;
    height: 24px;
    border-radius: 4px;
    color: var(--fg-muted);
    transition: all 0.15s;
  }

  .toast-close:hover {
    background: var(--bg-highlight);
    color: var(--fg-primary);
  }
</style>
