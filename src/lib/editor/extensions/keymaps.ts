import { keymap } from '@codemirror/view';
import {
  defaultKeymap,
  indentWithTab,
  history,
  historyKeymap,
} from '@codemirror/commands';
import { markdown } from '@codemirror/lang-markdown';
import { languages } from '@codemirror/language-data';

// Markdown-specific keybindings
const markdownKeymap = keymap.of([
  // Ctrl+B for bold
  {
    key: 'Ctrl-b',
    run: (view) => {
      const { from, to } = view.state.selection.main;
      const selectedText = view.state.sliceDoc(from, to);

      if (selectedText) {
        view.dispatch({
          changes: { from, to, insert: `**${selectedText}**` },
          selection: { anchor: from + 2, head: to + 2 },
        });
      } else {
        view.dispatch({
          changes: { from, insert: '****' },
          selection: { anchor: from + 2 },
        });
      }
      return true;
    },
  },
  // Ctrl+I for italic
  {
    key: 'Ctrl-i',
    run: (view) => {
      const { from, to } = view.state.selection.main;
      const selectedText = view.state.sliceDoc(from, to);

      if (selectedText) {
        view.dispatch({
          changes: { from, to, insert: `*${selectedText}*` },
          selection: { anchor: from + 1, head: to + 1 },
        });
      } else {
        view.dispatch({
          changes: { from, insert: '**' },
          selection: { anchor: from + 1 },
        });
      }
      return true;
    },
  },
  // Ctrl+` for inline code
  {
    key: 'Ctrl-`',
    run: (view) => {
      const { from, to } = view.state.selection.main;
      const selectedText = view.state.sliceDoc(from, to);

      if (selectedText) {
        view.dispatch({
          changes: { from, to, insert: `\`${selectedText}\`` },
          selection: { anchor: from + 1, head: to + 1 },
        });
      } else {
        view.dispatch({
          changes: { from, insert: '``' },
          selection: { anchor: from + 1 },
        });
      }
      return true;
    },
  },
  // Ctrl+K for link
  {
    key: 'Ctrl-k',
    run: (view) => {
      const { from, to } = view.state.selection.main;
      const selectedText = view.state.sliceDoc(from, to);

      if (selectedText) {
        view.dispatch({
          changes: { from, to, insert: `[${selectedText}](url)` },
          selection: { anchor: to + 3, head: to + 6 },
        });
      } else {
        view.dispatch({
          changes: { from, insert: '[](url)' },
          selection: { anchor: from + 1 },
        });
      }
      return true;
    },
  },
]);

export function getKeymapExtensions() {
  return [
    history(),
    keymap.of([
      ...defaultKeymap,
      ...historyKeymap,
      indentWithTab,
    ]),
    markdownKeymap,
  ];
}

export function getMarkdownExtension() {
  return markdown({
    codeLanguages: languages,
  });
}
