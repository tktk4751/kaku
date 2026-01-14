import { EditorView } from '@codemirror/view';

// Minimal theme - colors are applied via CSS variables in Editor.svelte
export const tokyoNightTheme = EditorView.theme(
  {
    '.cm-lineNumbers .cm-gutterElement': {
      padding: '0 8px 0 16px',
    },
    '.cm-cursor, .cm-dropCursor': {
      borderLeftWidth: '2px',
    },
    '.cm-gutters': {
      border: 'none',
    },
    // Live preview custom classes - structure only, colors via CSS
    '.cm-bold-content': {
      fontWeight: 'bold',
    },
    '.cm-italic-content': {
      fontStyle: 'italic',
    },
    '.cm-code-content': {
      padding: '1px 4px',
      borderRadius: '3px',
      fontFamily: 'monospace',
    },
    '.cm-link-content': {
      textDecoration: 'underline',
      cursor: 'pointer',
    },
    '.cm-heading-1': {
      fontSize: '1.6em',
      fontWeight: 'bold',
    },
    '.cm-heading-2': {
      fontSize: '1.4em',
      fontWeight: 'bold',
    },
    '.cm-heading-3': {
      fontSize: '1.2em',
      fontWeight: 'bold',
    },
    '.cm-heading-4': {
      fontSize: '1.1em',
      fontWeight: 'bold',
    },
    '.cm-heading-5': {
      fontSize: '1.0em',
      fontWeight: 'bold',
    },
    // Strikethrough
    '.cm-strikethrough-content': {
      textDecoration: 'line-through',
    },
    // Horizontal rule widget
    '.cm-hr-widget': {
      display: 'block',
      height: '2px',
      margin: '16px 0',
    },
    // Blockquote
    '.cm-blockquote-line': {
      borderLeft: '3px solid',
      paddingLeft: '12px',
      marginLeft: '4px',
    },
    // List markers
    '.cm-list-marker': {
      display: 'inline-block',
      minWidth: '1.5em',
      textAlign: 'center',
      fontWeight: 'bold',
      marginRight: '4px',
    },
    // Checkbox
    '.cm-checkbox-widget': {
      display: 'inline-flex',
      alignItems: 'center',
      marginRight: '6px',
    },
    '.cm-task-checkbox': {
      width: '16px',
      height: '16px',
      cursor: 'pointer',
      margin: '0',
      borderRadius: '3px',
    },
  },
  { dark: true }
);
