import { EditorState, type Extension } from '@codemirror/state';
import { EditorView, lineNumbers, drawSelection, keymap } from '@codemirror/view';
import { syntaxHighlighting, defaultHighlightStyle, bracketMatching } from '@codemirror/language';
import { search, SearchQuery, setSearchQuery, getSearchQuery, findNext, findPrevious, replaceNext, replaceAll as cmReplaceAll, searchKeymap, highlightSelectionMatches } from '@codemirror/search';
import { tokyoNightTheme } from './themes/tokyoNight';
import { livePreviewPlugin } from './extensions/livePreview';
import { wikiLinkPlugin, wikiLinkClickHandler } from './extensions/wikiLink';
import { wikiLinkAutocomplete, updateNoteTitles } from './extensions/wikiLinkAutocomplete';
import { getKeymapExtensions, getMarkdownExtension } from './extensions/keymaps';

// Re-export for external use
export { updateNoteTitles };

import type { ThemeName } from '$lib/types';

export interface EditorConfig {
  parent: HTMLElement;
  doc?: string;
  onChange?: (content: string) => void;
  onWikiLinkClick?: (title: string) => void;
  theme?: ThemeName;
  fontSize?: number;
  lineHeight?: number;
  showLineNumbers?: boolean;
}

export function createEditor(config: EditorConfig): EditorView {
  const { parent, doc = '', onChange, onWikiLinkClick, theme = 'tokyo-night', fontSize = 14, lineHeight = 1.6, showLineNumbers = true } = config;

  const updateListener = EditorView.updateListener.of((update) => {
    if (update.docChanged && onChange) {
      onChange(update.state.doc.toString());
    }
  });

  const extensions: Extension[] = [
    // Basic editor features
    ...(showLineNumbers ? [lineNumbers()] : []),
    // NOTE: highlightActiveLine() is intentionally NOT included
    // to keep the editor clean without current line highlighting
    drawSelection(),
    bracketMatching(),

    // Syntax highlighting
    syntaxHighlighting(defaultHighlightStyle, { fallback: true }),

    // Markdown support
    getMarkdownExtension(),

    // Keymaps (includes history, default keys, markdown shortcuts)
    ...getKeymapExtensions(),

    // CodeMirror search extension (optimized, handles highlighting automatically)
    // Panel is hidden via CSS, but highlighting still works
    search({
      literal: true,
    }),
    // Highlight selection matches
    highlightSelectionMatches(),

    // Live preview (hides markdown syntax on non-cursor lines)
    livePreviewPlugin(),

    // Wiki link support ([[title]] and [[title|display]])
    wikiLinkPlugin(),
    wikiLinkAutocomplete(),
    ...(onWikiLinkClick ? [wikiLinkClickHandler(onWikiLinkClick)] : []),

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

// Search interface using CodeMirror's optimized search
export interface SearchState {
  matchCount: number;
  currentMatch: number; // 1-indexed, 0 means no current match
}

// Set search query (uses CodeMirror's optimized search)
export function setSearch(view: EditorView, query: string, caseSensitive: boolean = false): void {
  const searchQuery = new SearchQuery({
    search: query,
    caseSensitive,
    literal: true,
  });
  view.dispatch({ effects: setSearchQuery.of(searchQuery) });
}

// Set search and go to first match in document order
export function setSearchAndGoToFirst(view: EditorView, query: string, caseSensitive: boolean = false): void {
  const searchQuery = new SearchQuery({
    search: query,
    caseSensitive,
    literal: true,
  });
  // Move cursor to start of document, then set search and find first match
  view.dispatch({
    effects: setSearchQuery.of(searchQuery),
    selection: { anchor: 0 },
  });
  // Find next will now find the first match in document order
  findNext(view);
}

// Clear search
export function clearSearch(view: EditorView): void {
  const searchQuery = new SearchQuery({ search: '' });
  view.dispatch({ effects: setSearchQuery.of(searchQuery) });
}

// Get current search state (match count and current position)
export function getSearchState(view: EditorView): SearchState {
  const query = getSearchQuery(view.state);
  if (!query.valid) {
    return { matchCount: 0, currentMatch: 0 };
  }

  // Count matches using cursor
  let matchCount = 0;
  let currentMatch = 0;
  const cursor = query.getCursor(view.state.doc);
  const selection = view.state.selection.main;

  let result = cursor.next();
  while (!result.done) {
    matchCount++;
    // Check if this match contains the selection
    if (result.value.from <= selection.from && result.value.to >= selection.to) {
      currentMatch = matchCount;
    }
    result = cursor.next();
  }

  return { matchCount, currentMatch };
}

// Go to next match
export function goToNextMatch(view: EditorView): boolean {
  return findNext(view);
}

// Go to previous match
export function goToPrevMatch(view: EditorView): boolean {
  return findPrevious(view);
}

// Replace current match
export function replaceCurrent(view: EditorView, replacement: string): boolean {
  const query = getSearchQuery(view.state);
  if (!query.valid) return false;

  // Set replacement in query
  const newQuery = new SearchQuery({
    search: query.search,
    caseSensitive: query.caseSensitive,
    literal: true,
    replace: replacement,
  });
  view.dispatch({ effects: setSearchQuery.of(newQuery) });

  return replaceNext(view);
}

// Replace all matches
export function replaceAllMatches(view: EditorView, replacement: string): boolean {
  const query = getSearchQuery(view.state);
  if (!query.valid) return false;

  // Set replacement in query
  const newQuery = new SearchQuery({
    search: query.search,
    caseSensitive: query.caseSensitive,
    literal: true,
    replace: replacement,
  });
  view.dispatch({ effects: setSearchQuery.of(newQuery) });

  return cmReplaceAll(view);
}
