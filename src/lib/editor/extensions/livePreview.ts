import {
  ViewPlugin,
  Decoration,
  type DecorationSet,
  EditorView,
  type ViewUpdate,
  WidgetType,
} from '@codemirror/view';
import { syntaxTree } from '@codemirror/language';
import { RangeSetBuilder } from '@codemirror/state';

// Decoration marks for live preview
const boldMark = Decoration.mark({ class: 'cm-bold-content' });
const italicMark = Decoration.mark({ class: 'cm-italic-content' });
const codeMark = Decoration.mark({ class: 'cm-code-content' });
const linkMark = Decoration.mark({ class: 'cm-link-content' });
const heading1Mark = Decoration.mark({ class: 'cm-heading-1' });
const heading2Mark = Decoration.mark({ class: 'cm-heading-2' });
const heading3Mark = Decoration.mark({ class: 'cm-heading-3' });
const heading4Mark = Decoration.mark({ class: 'cm-heading-4' });
const heading5Mark = Decoration.mark({ class: 'cm-heading-5' });
const strikethroughMark = Decoration.mark({ class: 'cm-strikethrough-content' });
const blockquoteMark = Decoration.mark({ class: 'cm-blockquote-content' });
const listBulletMark = Decoration.mark({ class: 'cm-list-bullet' });

interface DecorationEntry {
  from: number;
  to: number;
  decoration: Decoration;
}

// Widget for checkbox
class CheckboxWidget extends WidgetType {
  constructor(readonly checked: boolean, readonly pos: number) {
    super();
  }

  eq(other: CheckboxWidget) {
    return other.checked === this.checked && other.pos === this.pos;
  }

  toDOM(view: EditorView) {
    const wrap = document.createElement('span');
    wrap.className = 'cm-checkbox-widget';

    const checkbox = document.createElement('input');
    checkbox.type = 'checkbox';
    checkbox.checked = this.checked;
    checkbox.className = 'cm-task-checkbox';

    checkbox.addEventListener('mousedown', (e) => {
      e.preventDefault();
      const pos = this.pos;
      const line = view.state.doc.lineAt(pos);
      const lineText = line.text;

      // Find the checkbox pattern and toggle it
      const checkboxMatch = lineText.match(/^(\s*[-*+]\s*)\[([ xX])\]/);
      if (checkboxMatch) {
        const bracketStart = line.from + checkboxMatch[1].length;
        const newChar = this.checked ? ' ' : 'x';

        view.dispatch({
          changes: {
            from: bracketStart + 1,
            to: bracketStart + 2,
            insert: newChar
          }
        });
      }
    });

    wrap.appendChild(checkbox);
    return wrap;
  }

  ignoreEvent() {
    return false;
  }
}

// Widget for horizontal rule
class HorizontalRuleWidget extends WidgetType {
  toDOM() {
    const hr = document.createElement('div');
    hr.className = 'cm-hr-widget';
    return hr;
  }

  eq() {
    return true;
  }
}

// Widget for list bullet
class ListBulletWidget extends WidgetType {
  constructor(readonly bulletType: 'ul' | 'ol', readonly number?: number) {
    super();
  }

  eq(other: ListBulletWidget) {
    return other.bulletType === this.bulletType && other.number === this.number;
  }

  toDOM() {
    const span = document.createElement('span');
    span.className = `cm-list-marker cm-list-marker-${this.bulletType}`;

    if (this.bulletType === 'ul') {
      span.textContent = '•';
    } else if (this.number !== undefined) {
      span.textContent = `${this.number}.`;
    }

    return span;
  }
}

export function livePreviewPlugin() {
  return ViewPlugin.fromClass(
    class {
      decorations: DecorationSet;

      constructor(view: EditorView) {
        this.decorations = this.buildDecorations(view);
      }

      update(update: ViewUpdate) {
        if (update.docChanged || update.viewportChanged || update.selectionSet) {
          this.decorations = this.buildDecorations(update.view);
        }
      }

      buildDecorations(view: EditorView): DecorationSet {
        const cursorPos = view.state.selection.main.head;

        // Performance optimization: only process visible viewport
        const { from: viewportFrom, to: viewportTo } = view.viewport;

        // Collect decorations first, then sort by position
        const decorations: DecorationEntry[] = [];

        // Track processed lines for line-level decorations
        const processedLines = new Set<number>();

        syntaxTree(view.state).iterate({
          from: viewportFrom,
          to: viewportTo,
          enter: (node) => {
            const { from, to } = node;
            const line = view.state.doc.lineAt(from);
            const lineStart = line.from;
            const lineEnd = view.state.doc.lineAt(to).to;
            const cursorOnLine = cursorPos >= lineStart && cursorPos <= lineEnd;

            // Show markdown syntax on cursor line (edit mode)
            if (cursorOnLine) return;

            switch (node.name) {
              case 'ATXHeading1': {
                const text = view.state.sliceDoc(from, to);
                const spaceIndex = text.indexOf(' ');
                if (spaceIndex > 0) {
                  const hashEnd = from + spaceIndex + 1;
                  const contentStart = hashEnd;
                  if (contentStart < to) {
                    decorations.push({ from, to: hashEnd, decoration: Decoration.replace({}) });
                    decorations.push({ from: contentStart, to, decoration: heading1Mark });
                  }
                }
                break;
              }

              case 'ATXHeading2': {
                const text = view.state.sliceDoc(from, to);
                const spaceIndex = text.indexOf(' ');
                if (spaceIndex > 0) {
                  const hashEnd = from + spaceIndex + 1;
                  const contentStart = hashEnd;
                  if (contentStart < to) {
                    decorations.push({ from, to: hashEnd, decoration: Decoration.replace({}) });
                    decorations.push({ from: contentStart, to, decoration: heading2Mark });
                  }
                }
                break;
              }

              case 'ATXHeading3': {
                const text = view.state.sliceDoc(from, to);
                const spaceIndex = text.indexOf(' ');
                if (spaceIndex > 0) {
                  const hashEnd = from + spaceIndex + 1;
                  const contentStart = hashEnd;
                  if (contentStart < to) {
                    decorations.push({ from, to: hashEnd, decoration: Decoration.replace({}) });
                    decorations.push({ from: contentStart, to, decoration: heading3Mark });
                  }
                }
                break;
              }

              case 'ATXHeading4': {
                const text = view.state.sliceDoc(from, to);
                const spaceIndex = text.indexOf(' ');
                if (spaceIndex > 0) {
                  const hashEnd = from + spaceIndex + 1;
                  const contentStart = hashEnd;
                  if (contentStart < to) {
                    decorations.push({ from, to: hashEnd, decoration: Decoration.replace({}) });
                    decorations.push({ from: contentStart, to, decoration: heading4Mark });
                  }
                }
                break;
              }

              case 'ATXHeading5':
              case 'ATXHeading6': {
                const text = view.state.sliceDoc(from, to);
                const spaceIndex = text.indexOf(' ');
                if (spaceIndex > 0) {
                  const hashEnd = from + spaceIndex + 1;
                  const contentStart = hashEnd;
                  if (contentStart < to) {
                    decorations.push({ from, to: hashEnd, decoration: Decoration.replace({}) });
                    decorations.push({ from: contentStart, to, decoration: heading5Mark });
                  }
                }
                break;
              }

              case 'StrongEmphasis': {
                const text = view.state.sliceDoc(from, to);
                if (text.startsWith('**') && text.endsWith('**') && text.length > 4) {
                  decorations.push({ from, to: from + 2, decoration: Decoration.replace({}) });
                  decorations.push({ from: from + 2, to: to - 2, decoration: boldMark });
                  decorations.push({ from: to - 2, to, decoration: Decoration.replace({}) });
                }
                break;
              }

              case 'Emphasis': {
                const text = view.state.sliceDoc(from, to);
                if (text.length > 2 &&
                    (text.startsWith('*') || text.startsWith('_')) &&
                    (text.endsWith('*') || text.endsWith('_'))) {
                  decorations.push({ from, to: from + 1, decoration: Decoration.replace({}) });
                  decorations.push({ from: from + 1, to: to - 1, decoration: italicMark });
                  decorations.push({ from: to - 1, to, decoration: Decoration.replace({}) });
                }
                break;
              }

              case 'Strikethrough': {
                const text = view.state.sliceDoc(from, to);
                if (text.startsWith('~~') && text.endsWith('~~') && text.length > 4) {
                  decorations.push({ from, to: from + 2, decoration: Decoration.replace({}) });
                  decorations.push({ from: from + 2, to: to - 2, decoration: strikethroughMark });
                  decorations.push({ from: to - 2, to, decoration: Decoration.replace({}) });
                }
                break;
              }

              case 'InlineCode': {
                const text = view.state.sliceDoc(from, to);
                if (text.startsWith('`') && text.endsWith('`') && text.length > 2) {
                  decorations.push({ from, to: from + 1, decoration: Decoration.replace({}) });
                  decorations.push({ from: from + 1, to: to - 1, decoration: codeMark });
                  decorations.push({ from: to - 1, to, decoration: Decoration.replace({}) });
                }
                break;
              }

              case 'Link': {
                const text = view.state.sliceDoc(from, to);
                const match = text.match(/^\[([^\]]+)\]\(([^)]+)\)$/);
                if (match) {
                  const textStart = from + 1;
                  const textEnd = from + 1 + match[1].length;
                  decorations.push({ from, to: textStart, decoration: Decoration.replace({}) });
                  decorations.push({ from: textStart, to: textEnd, decoration: linkMark });
                  decorations.push({ from: textEnd, to, decoration: Decoration.replace({}) });
                }
                break;
              }

              case 'HorizontalRule': {
                // Replace the entire horizontal rule with a widget
                decorations.push({
                  from,
                  to,
                  decoration: Decoration.replace({
                    widget: new HorizontalRuleWidget()
                  })
                });
                break;
              }

              case 'Blockquote': {
                // Mark the blockquote line
                if (!processedLines.has(line.number)) {
                  processedLines.add(line.number);
                  const lineText = line.text;
                  const quoteMatch = lineText.match(/^(\s*>+\s*)/);
                  if (quoteMatch) {
                    const markerEnd = line.from + quoteMatch[1].length;
                    // Add line decoration for blockquote styling
                    decorations.push({
                      from: line.from,
                      to: line.from,
                      decoration: Decoration.line({ class: 'cm-blockquote-line' })
                    });
                    // Hide the > marker
                    decorations.push({
                      from: line.from,
                      to: markerEnd,
                      decoration: Decoration.replace({})
                    });
                  }
                }
                break;
              }

              case 'ListItem': {
                // Handle task lists (checkboxes) and regular lists
                const lineText = line.text;

                // Check for task list: - [ ] or - [x]
                const taskMatch = lineText.match(/^(\s*)([-*+])\s*\[([ xX])\]\s*/);
                if (taskMatch) {
                  const isChecked = taskMatch[3].toLowerCase() === 'x';
                  const markerStart = line.from + taskMatch[1].length;
                  const markerEnd = line.from + taskMatch[0].length;

                  // Replace the bullet and checkbox syntax with a widget
                  decorations.push({
                    from: markerStart,
                    to: markerEnd,
                    decoration: Decoration.replace({
                      widget: new CheckboxWidget(isChecked, line.from)
                    })
                  });

                  // Add line class for checked items
                  if (isChecked) {
                    decorations.push({
                      from: line.from,
                      to: line.from,
                      decoration: Decoration.line({ class: 'cm-task-checked' })
                    });
                  }
                  break;
                }

                // Check for unordered list: -, *, +
                const ulMatch = lineText.match(/^(\s*)([-*+])\s+/);
                if (ulMatch) {
                  const markerStart = line.from + ulMatch[1].length;
                  const markerEnd = line.from + ulMatch[0].length;

                  decorations.push({
                    from: markerStart,
                    to: markerEnd,
                    decoration: Decoration.replace({
                      widget: new ListBulletWidget('ul')
                    })
                  });
                  break;
                }

                // Check for ordered list: 1., 2., etc.
                const olMatch = lineText.match(/^(\s*)(\d+)\.\s+/);
                if (olMatch) {
                  const markerStart = line.from + olMatch[1].length;
                  const markerEnd = line.from + olMatch[0].length;
                  const number = parseInt(olMatch[2], 10);

                  decorations.push({
                    from: markerStart,
                    to: markerEnd,
                    decoration: Decoration.replace({
                      widget: new ListBulletWidget('ol', number)
                    })
                  });
                  break;
                }
                break;
              }

              case 'BulletList':
              case 'OrderedList': {
                // Parent containers, decorations handled at ListItem level
                break;
              }
            }
          },
        });

        // Sort decorations by 'from' position (required by RangeSetBuilder)
        // For same position, line decorations should come first
        decorations.sort((a, b) => {
          if (a.from !== b.from) return a.from - b.from;
          // Line decorations have from === to
          const aIsLine = a.from === a.to;
          const bIsLine = b.from === b.to;
          if (aIsLine && !bIsLine) return -1;
          if (!aIsLine && bIsLine) return 1;
          return a.to - b.to;
        });

        // Build sorted decoration set
        const builder = new RangeSetBuilder<Decoration>();
        for (const { from, to, decoration } of decorations) {
          builder.add(from, to, decoration);
        }

        return builder.finish();
      }
    },
    {
      decorations: (v) => v.decorations,
    }
  );
}
