import { EditorState, type Extension } from '@codemirror/state';
import { EditorView, lineNumbers, highlightActiveLine, drawSelection } from '@codemirror/view';
import { syntaxHighlighting, defaultHighlightStyle, bracketMatching } from '@codemirror/language';
import { tokyoNightTheme } from './themes/tokyoNight';
import { livePreviewPlugin } from './extensions/livePreview';
import { getKeymapExtensions, getMarkdownExtension } from './extensions/keymaps';

import type { ThemeName } from '$lib/types';

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
