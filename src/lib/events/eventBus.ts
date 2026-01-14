type EventCallback<T = unknown> = (data: T) => void;

interface Subscription {
  id: number;
  callback: EventCallback;
}

class EventBus {
  private handlers: Map<string, Subscription[]> = new Map();
  private nextId = 1;

  /**
   * Subscribe to an event
   */
  on<T>(event: string, callback: EventCallback<T>): () => void {
    const id = this.nextId++;
    const subscription: Subscription = { id, callback: callback as EventCallback };

    if (!this.handlers.has(event)) {
      this.handlers.set(event, []);
    }
    this.handlers.get(event)!.push(subscription);

    // Return unsubscribe function
    return () => this.off(event, id);
  }

  /**
   * Subscribe once
   */
  once<T>(event: string, callback: EventCallback<T>): () => void {
    const unsubscribe = this.on<T>(event, (data) => {
      unsubscribe();
      callback(data);
    });
    return unsubscribe;
  }

  /**
   * Unsubscribe
   */
  private off(event: string, id: number): void {
    const handlers = this.handlers.get(event);
    if (handlers) {
      const index = handlers.findIndex(h => h.id === id);
      if (index !== -1) {
        handlers.splice(index, 1);
      }
    }
  }

  /**
   * Emit an event
   */
  emit<T>(event: string, data?: T): void {
    const handlers = this.handlers.get(event);
    if (handlers) {
      handlers.forEach(h => h.callback(data));
    }

    // Also emit to wildcard handlers
    const wildcardHandlers = this.handlers.get('*');
    if (wildcardHandlers) {
      wildcardHandlers.forEach(h => h.callback({ event, data }));
    }
  }

  /**
   * Clear all subscriptions
   */
  clear(): void {
    this.handlers.clear();
  }
}

// Singleton instance
export const eventBus = new EventBus();

// Type-safe event names
export const Events = {
  // Note events
  NOTE_CREATED: 'note:created',
  NOTE_UPDATED: 'note:updated',
  NOTE_DELETED: 'note:deleted',
  NOTE_LOADED: 'note:loaded',
  NOTE_CONTENT_CHANGED: 'note:content_changed',

  // Save events
  SAVE_REQUESTED: 'save:requested',
  SAVE_STARTED: 'save:started',
  SAVE_COMPLETED: 'save:completed',
  SAVE_FAILED: 'save:failed',

  // UI events
  SIDEBAR_TOGGLE: 'ui:sidebar_toggle',
  SETTINGS_OPEN: 'ui:settings_open',
  SETTINGS_CLOSE: 'ui:settings_close',
  THEME_CHANGED: 'ui:theme_changed',

  // Window events
  WINDOW_SHOWN: 'window:shown',
  WINDOW_HIDDEN: 'window:hidden',
  WINDOW_FOCUS: 'window:focus',

  // Editor events
  EDITOR_FOCUS: 'editor:focus',
  EDITOR_BLUR: 'editor:blur',
} as const;

export type EventName = typeof Events[keyof typeof Events];
