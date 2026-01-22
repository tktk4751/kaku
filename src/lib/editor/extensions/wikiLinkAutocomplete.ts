import {
  autocompletion,
  type CompletionContext,
  type CompletionResult,
} from '@codemirror/autocomplete';
import type { NoteListItemDto } from '$lib/types';

// Store for note titles - will be updated externally
let noteTitles: NoteListItemDto[] = [];

// Update the note titles list (called from outside)
export function updateNoteTitles(notes: NoteListItemDto[]): void {
  noteTitles = notes;
}

// Wiki link completion source
function wikiLinkCompletions(context: CompletionContext): CompletionResult | null {
  // Match [[ followed by any characters (but not ])
  const before = context.matchBefore(/\[\[[^\]]*$/);
  if (!before) return null;

  // Extract the query after [[
  const query = before.text.slice(2).toLowerCase();
  const from = before.from;

  // Filter matching notes
  const matchingNotes = noteTitles
    .filter((note) => note.title.toLowerCase().includes(query))
    .slice(0, 10);

  // If no query and no notes, show hint
  if (matchingNotes.length === 0 && query.length === 0) {
    return {
      from: from + 2,
      options: [
        {
          label: 'Type to search notes...',
          apply: '',
          type: 'text',
        },
      ],
    };
  }

  // Return completion options
  return {
    from: from + 2, // Start after [[
    options: matchingNotes.map((note) => ({
      label: note.title || 'Untitled',
      detail: formatDate(note.updated_at),
      apply: `${note.title}]]`,
      type: 'text',
    })),
    validFor: /^[^\]]*$/,
  };
}

// Format date for display
function formatDate(dateStr: string): string {
  try {
    const date = new Date(dateStr);
    const now = new Date();
    const diff = now.getTime() - date.getTime();
    const days = Math.floor(diff / (1000 * 60 * 60 * 24));

    if (days === 0) return 'Today';
    if (days === 1) return 'Yesterday';
    if (days < 7) return `${days} days ago`;
    return date.toLocaleDateString();
  } catch {
    return '';
  }
}

// Create the autocomplete extension for wiki links
export function wikiLinkAutocomplete() {
  return autocompletion({
    override: [wikiLinkCompletions],
    activateOnTyping: true,
    maxRenderedOptions: 10,
    icons: false,
  });
}
