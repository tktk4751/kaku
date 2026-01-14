<script lang="ts">
  import Toast from './Toast.svelte';

  interface ToastItem {
    id: number;
    message: string;
    type: 'error' | 'success' | 'info';
  }

  let toasts = $state<ToastItem[]>([]);
  let nextId = 0;

  export function showToast(message: string, type: 'error' | 'success' | 'info' = 'info') {
    const id = nextId++;
    toasts = [...toasts, { id, message, type }];
  }

  export function showError(message: string) {
    showToast(message, 'error');
  }

  export function showSuccess(message: string) {
    showToast(message, 'success');
  }

  function removeToast(id: number) {
    toasts = toasts.filter(t => t.id !== id);
  }
</script>

<div class="toast-container">
  {#each toasts as toast (toast.id)}
    <Toast
      message={toast.message}
      type={toast.type}
      onClose={() => removeToast(toast.id)}
    />
  {/each}
</div>

<style>
  .toast-container {
    position: fixed;
    top: 16px;
    right: 16px;
    display: flex;
    flex-direction: column;
    gap: 8px;
    z-index: 400;
    pointer-events: none;
  }

  .toast-container :global(.toast) {
    pointer-events: auto;
  }
</style>
