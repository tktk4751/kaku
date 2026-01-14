<script lang="ts">
  import { onMount } from 'svelte';

  interface Props {
    title: string;
    message: string;
    confirmText?: string;
    cancelText?: string;
    danger?: boolean;
    onConfirm: () => void;
    onCancel: () => void;
  }

  let {
    title,
    message,
    confirmText = 'Confirm',
    cancelText = 'Cancel',
    danger = false,
    onConfirm,
    onCancel,
  }: Props = $props();

  let cancelButton: HTMLButtonElement;
  let confirmButton: HTMLButtonElement;

  onMount(() => {
    // Focus cancel button on mount for safety
    cancelButton?.focus();
  });

  function handleKeydown(event: KeyboardEvent) {
    if (event.key === 'Escape') {
      onCancel();
    } else if (event.key === 'Tab') {
      // Focus trap within dialog
      const focusableElements = [cancelButton, confirmButton];
      const firstElement = focusableElements[0];
      const lastElement = focusableElements[focusableElements.length - 1];

      if (event.shiftKey) {
        if (document.activeElement === firstElement) {
          event.preventDefault();
          lastElement?.focus();
        }
      } else {
        if (document.activeElement === lastElement) {
          event.preventDefault();
          firstElement?.focus();
        }
      }
    }
  }

  function handleBackdropClick(event: MouseEvent) {
    if (event.target === event.currentTarget) {
      onCancel();
    }
  }
</script>

<svelte:window onkeydown={handleKeydown} />

<!-- svelte-ignore a11y_click_events_have_key_events a11y_no_static_element_interactions -->
<div class="dialog-backdrop" onclick={handleBackdropClick}>
  <div
    class="dialog"
    role="alertdialog"
    aria-modal="true"
    aria-labelledby="dialog-title"
    aria-describedby="dialog-message"
  >
    <h2 id="dialog-title" class="dialog-title">{title}</h2>
    <p id="dialog-message" class="dialog-message">{message}</p>
    <div class="dialog-actions">
      <button
        bind:this={cancelButton}
        class="btn btn-secondary"
        onclick={onCancel}
      >
        {cancelText}
      </button>
      <button
        bind:this={confirmButton}
        class="btn"
        class:btn-danger={danger}
        class:btn-primary={!danger}
        onclick={onConfirm}
      >
        {confirmText}
      </button>
    </div>
  </div>
</div>

<style>
  .dialog-backdrop {
    position: fixed;
    inset: 0;
    display: flex;
    align-items: center;
    justify-content: center;
    background: rgba(0, 0, 0, 0.7);
    z-index: 300;
  }

  .dialog {
    width: 90%;
    max-width: 400px;
    padding: 24px;
    background: var(--bg-secondary);
    border-radius: 12px;
    border: 1px solid var(--border-color);
  }

  .dialog-title {
    font-size: 18px;
    font-weight: 600;
    color: var(--fg-primary);
    margin-bottom: 12px;
  }

  .dialog-message {
    font-size: 14px;
    color: var(--fg-secondary);
    line-height: 1.5;
    margin-bottom: 24px;
  }

  .dialog-actions {
    display: flex;
    justify-content: flex-end;
    gap: 12px;
  }

  .btn {
    padding: 10px 20px;
    border-radius: 6px;
    font-size: 14px;
    font-weight: 500;
    transition: all 0.15s;
    cursor: pointer;
  }

  .btn-secondary {
    background: var(--bg-tertiary);
    color: var(--fg-secondary);
  }

  .btn-secondary:hover {
    background: var(--bg-highlight);
    color: var(--fg-primary);
  }

  .btn-primary {
    background: var(--accent-blue);
    color: #fff;
  }

  .btn-primary:hover {
    opacity: 0.9;
  }

  .btn-danger {
    background: var(--accent-red, #f7768e);
    color: #fff;
  }

  .btn-danger:hover {
    opacity: 0.9;
  }
</style>
