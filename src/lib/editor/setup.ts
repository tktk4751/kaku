import { EditorState, type Extension } from '@codemirror/state';
import { EditorView, lineNumbers, highlightActiveLine, drawSelection, keymap, Decoration, type DecorationSet } from '@codemirror/view';
import { syntaxHighlighting, defaultHighlightStyle, bracketMatching } from '@codemirror/language';
import { StateField, StateEffect } from '@codemirror/state';
import { tokyoNightTheme } from './themes/tokyoNight';
import { livePreviewPlugin } from './extensions/livePreview';
import { getKeymapExtensions, getMarkdownExtension } from './extensions/keymaps';

import type { ThemeName } from '$lib/types';

// Custom search highlight effect
const setSearchMatches = StateEffect.define<{ ranges: { from: number; to: number }[] }>();
const clearSearchMatches = StateEffect.define();

// Search match decoration
const searchMatchMark = Decoration.mark({ class: 'cm-searchMatch' });
const searchMatchSelectedMark = Decoration.mark({ class: 'cm-searchMatch-selected' });

// State field for search highlights
const searchHighlightField = StateField.define<DecorationSet>({
  create() {
    return Decoration.none;
  },
  update(decorations, tr) {
    for (const e of tr.effects) {
      if (e.is(setSearchMatches)) {
        const marks = e.value.ranges.map((r, i) =>
          (i === 0 ? searchMatchSelectedMark : searchMatchMark).range(r.from, r.to)
        );
        return Decoration.set(marks, true);
      }
      if (e.is(clearSearchMatches)) {
        return Decoration.none;
      }
    }
    return decorations.map(tr.changes);
  },
  provide: (f) => EditorView.decorations.from(f),
});

export interface EditorConfig {
  parent: HTMLElement;
  doc?: string;
  onChange?: (content: string) => void;
  theme?: ThemeName;
  fontSize?: number;
  lineHeight?: number;
  showLineNumbers?: boolean;
}

export function createEditor(config: EditorConfig): EditorView {
  const { parent, doc = '', onChange, theme = 'tokyo-night', fontSize = 14, lineHeight = 1.6, showLineNumbers = true } = config;

  const updateListener = EditorView.updateListener.of((update) => {
    if (update.docChanged && onChange) {
      onChange(update.state.doc.toString());
    }
  });

  const extensions: Extension[] = [
    // Basic editor features
    ...(showLineNumbers ? [lineNumbers()] : []),
    highlightActiveLine(),
    drawSelection(),
    bracketMatching(),

    // Syntax highlighting
    syntaxHighlighting(defaultHighlightStyle, { fallback: true }),

    // Markdown support
    getMarkdownExtension(),

    // Keymaps (includes history, default keys, markdown shortcuts)
    ...getKeymapExtensions(),

    // Custom search highlight field
    searchHighlightField,

    // Live preview (hides markdown syntax on non-cursor lines)
    livePreviewPlugin(),

    // Theme (uses CSS variables - works with all color themes)
    tokyoNightTheme,

    // Font settings
    EditorView.theme({
      '&': {
        fontSize: `${fontSize}px`,
      },
      '.cm-content': {
        lineHeight: String(lineHeight),
      },
      '.cm-line': {
        lineHeight: String(lineHeight),
      },
    }),

    // Change listener
    updateListener,

    // Editor behavior
    EditorView.lineWrapping,
    EditorState.allowMultipleSelections.of(true),
    EditorView.editable.of(true),
  ];

  const state = EditorState.create({
    doc,
    extensions,
  });

  const view = new EditorView({
    state,
    parent,
  });

  return view;
}

// Helper to update editor content without losing cursor position
export function setEditorContent(view: EditorView, content: string): void {
  const currentContent = view.state.doc.toString();
  if (currentContent !== content) {
    view.dispatch({
      changes: { from: 0, to: currentContent.length, insert: content },
    });
  }
}

// Helper to focus editor
export function focusEditor(view: EditorView): void {
  view.focus();
}

// Custom search interface
export interface SearchMatch {
  from: number;
  to: number;
}

export interface SearchResult {
  matches: SearchMatch[];
  currentIndex: number;
}

// Find all matches in document
export function findMatches(
  view: EditorView,
  query: string,
  caseSensitive: boolean = false
): SearchMatch[] {
  if (!query) return [];

  const doc = view.state.doc.toString();
  const searchStr = caseSensitive ? query : query.toLowerCase();
  const searchDoc = caseSensitive ? doc : doc.toLowerCase();
  const matches: SearchMatch[] = [];

  let pos = 0;
  while (pos < searchDoc.length) {
    const index = searchDoc.indexOf(searchStr, pos);
    if (index === -1) break;
    matches.push({ from: index, to: index + query.length });
    pos = index + 1;
  }

  return matches;
}

// Highlight search matches
export function highlightMatches(view: EditorView, matches: SearchMatch[], currentIndex: number = 0): void {
  if (matches.length === 0) {
    view.dispatch({ effects: clearSearchMatches.of(null) });
    return;
  }

  // Reorder so current match is first (for selected styling)
  const reordered = [
    matches[currentIndex],
    ...matches.slice(0, currentIndex),
    ...matches.slice(currentIndex + 1),
  ];

  view.dispatch({
    effects: setSearchMatches.of({ ranges: reordered }),
  });
}

// Clear search highlights
export function clearHighlights(view: EditorView): void {
  view.dispatch({ effects: clearSearchMatches.of(null) });
}

// Scroll to match
export function scrollToMatch(view: EditorView, match: SearchMatch): void {
  view.dispatch({
    selection: { anchor: match.from, head: match.to },
    scrollIntoView: true,
  });
}

// Replace text at position
export function replaceAt(view: EditorView, from: number, to: number, replacement: string): void {
  view.dispatch({
    changes: { from, to, insert: replacement },
  });
}

// Replace all matches
export function replaceAll(view: EditorView, matches: SearchMatch[], replacement: string): number {
  if (matches.length === 0) return 0;

  // Sort matches in reverse order to maintain positions
  const sorted = [...matches].sort((a, b) => b.from - a.from);

  const changes = sorted.map((m) => ({ from: m.from, to: m.to, insert: replacement }));
  view.dispatch({ changes });

  return matches.length;
}
