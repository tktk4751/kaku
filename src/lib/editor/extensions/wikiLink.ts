import {
  ViewPlugin,
  Decoration,
  type DecorationSet,
  EditorView,
  type ViewUpdate,
  WidgetType,
} from '@codemirror/view';
import { RangeSetBuilder } from '@codemirror/state';

// Wiki link pattern: [[title]] or [[title|display]]
const WIKI_LINK_REGEX = /\[\[([^\]|]+)(?:\|([^\]]+))?\]\]/g;

// Decoration marks
const wikiLinkMark = Decoration.mark({ class: 'cm-wikilink' });

interface WikiLinkMatch {
  from: number;
  to: number;
  title: string;
  display: string;
}

// Widget for wiki link display text
class WikiLinkWidget extends WidgetType {
  constructor(
    readonly title: string,
    readonly display: string
  ) {
    super();
  }

  eq(other: WikiLinkWidget) {
    return other.title === this.title && other.display === this.display;
  }

  toDOM() {
    const span = document.createElement('span');
    span.className = 'cm-wikilink';
    span.textContent = this.display;
    span.dataset.wikiTitle = this.title;
    return span;
  }

  ignoreEvent() {
    return false;
  }
}

function findWikiLinks(text: string, offset: number = 0): WikiLinkMatch[] {
  const matches: WikiLinkMatch[] = [];
  let match;

  WIKI_LINK_REGEX.lastIndex = 0;
  while ((match = WIKI_LINK_REGEX.exec(text)) !== null) {
    const title = match[1].trim();
    const display = match[2]?.trim() || title;

    matches.push({
      from: offset + match.index,
      to: offset + match.index + match[0].length,
      title,
      display,
    });
  }

  return matches;
}

export function wikiLinkPlugin() {
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
        const { from: viewportFrom, to: viewportTo } = view.viewport;
        const builder = new RangeSetBuilder<Decoration>();

        // Process visible lines
        for (let pos = viewportFrom; pos < viewportTo; ) {
          const line = view.state.doc.lineAt(pos);
          const lineStart = line.from;
          const lineEnd = line.to;
          const cursorOnLine = cursorPos >= lineStart && cursorPos <= lineEnd;

          const matches = findWikiLinks(line.text, lineStart);

          for (const match of matches) {
            if (cursorOnLine) {
              // Show raw syntax with subtle styling
              builder.add(match.from, match.from + 2, Decoration.mark({ class: 'cm-wikilink-bracket' }));
              builder.add(match.from + 2, match.to - 2, wikiLinkMark);
              builder.add(match.to - 2, match.to, Decoration.mark({ class: 'cm-wikilink-bracket' }));
            } else {
              // Replace with styled display text
              builder.add(
                match.from,
                match.to,
                Decoration.replace({
                  widget: new WikiLinkWidget(match.title, match.display),
                })
              );
            }
          }

          pos = line.to + 1;
        }

        return builder.finish();
      }
    },
    {
      decorations: (v) => v.decorations,
    }
  );
}

// Click handler for wiki links
export function wikiLinkClickHandler(onNavigate: (title: string) => void) {
  return EditorView.domEventHandlers({
    click: (event, view) => {
      const target = event.target as HTMLElement;

      // Case 1: Clicked on widget (has data-wiki-title)
      if (target.classList.contains('cm-wikilink') && target.dataset.wikiTitle) {
        event.preventDefault();
        onNavigate(target.dataset.wikiTitle);
        return true;
      }

      // Case 2: Clicked on raw link text (cursor on line)
      // Check if we're clicking inside a wiki link pattern
      const pos = view.posAtCoords({ x: event.clientX, y: event.clientY });
      if (pos !== null) {
        const line = view.state.doc.lineAt(pos);
        const matches = findWikiLinks(line.text, line.from);

        for (const match of matches) {
          if (pos >= match.from && pos <= match.to) {
            event.preventDefault();
            onNavigate(match.title);
            return true;
          }
        }
      }

      return false;
    },
  });
}

// Export for use in autocomplete
export { findWikiLinks, WIKI_LINK_REGEX };
