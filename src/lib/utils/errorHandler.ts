// Error handling utility with toast integration
import { toast } from 'svelte-sonner';

export function handleError(error: unknown, context?: string) {
  const message = error instanceof Error
    ? error.message
    : typeof error === 'object' && error !== null && 'message' in error
      ? String((error as { message: unknown }).message)
      : 'An unexpected error occurred';

  console.error(`[${context ?? 'Error'}]`, error);
  toast.error(message, { description: context, duration: 5000 });
}

export function handleSuccess(message: string) {
  toast.success(message, { duration: 3000 });
}
