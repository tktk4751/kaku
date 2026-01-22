// Keyboard shortcut parsing and matching utilities

export interface ParsedShortcut {
  ctrl: boolean;
  shift: boolean;
  alt: boolean;
  meta: boolean;
  key: string;
}

/**
 * Parse a shortcut string like "Ctrl+Shift+N" into its components
 */
export function parseShortcut(shortcut: string): ParsedShortcut {
  const parts = shortcut.toLowerCase().split('+');
  const key = parts[parts.length - 1];

  return {
    ctrl: parts.includes('ctrl'),
    shift: parts.includes('shift'),
    alt: parts.includes('alt'),
    meta: parts.includes('meta') || parts.includes('cmd'),
    key: key === 'space' ? ' ' : key,
  };
}

/**
 * Check if a keyboard event matches a shortcut string
 */
export function matchShortcut(event: KeyboardEvent, shortcut: string): boolean {
  const parsed = parseShortcut(shortcut);
  const eventKey = event.key.toLowerCase();

  return (
    event.ctrlKey === parsed.ctrl &&
    event.shiftKey === parsed.shift &&
    event.altKey === parsed.alt &&
    event.metaKey === parsed.meta &&
    eventKey === parsed.key
  );
}

/**
 * Format a ParsedShortcut back to a string for display
 */
export function formatShortcut(shortcut: ParsedShortcut): string {
  const parts: string[] = [];

  if (shortcut.ctrl) parts.push('Ctrl');
  if (shortcut.shift) parts.push('Shift');
  if (shortcut.alt) parts.push('Alt');
  if (shortcut.meta) parts.push('Cmd');

  const keyDisplay = shortcut.key === ' ' ? 'Space' : shortcut.key.toUpperCase();
  parts.push(keyDisplay);

  return parts.join('+');
}

/**
 * Create a shortcut string from a keyboard event (for recording shortcuts)
 */
export function shortcutFromEvent(event: KeyboardEvent): string | null {
  // Ignore modifier-only keypresses
  if (['Control', 'Shift', 'Alt', 'Meta'].includes(event.key)) {
    return null;
  }

  const parts: string[] = [];

  if (event.ctrlKey) parts.push('Ctrl');
  if (event.shiftKey) parts.push('Shift');
  if (event.altKey) parts.push('Alt');
  if (event.metaKey) parts.push('Cmd');

  // Require at least one modifier
  if (parts.length === 0) {
    return null;
  }

  const key = event.key === ' ' ? 'Space' : event.key.toUpperCase();
  parts.push(key);

  return parts.join('+');
}
