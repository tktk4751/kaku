<script lang="ts">
  import { onMount, onDestroy } from 'svelte';
  import { untrack } from 'svelte';
  import { noteStore } from '$lib/stores/note.svelte';
  import { settingsStore } from '$lib/stores/settings.svelte';
  import { createEditor, setEditorContent, focusEditor, updateNoteTitles } from '$lib/editor/setup';
  import { resolveWikiLink, getBacklinks } from '$lib/services/api';
  import { matchShortcut } from '$lib/utils/shortcuts';
  import type { BacklinkDto } from '$lib/types';
  import type { EditorView } from '@codemirror/view';
  import FindBar from './FindBar.svelte';
  import BacklinkPanel from './BacklinkPanel.svelte';

  interface Props {
    onNavigateToNote?: (uid: string) => void;
  }

  let { onNavigateToNote }: Props = $props();

  let editorContainer: HTMLDivElement;
  let editorView = $state<EditorView | null>(null);
  let isUpdatingFromStore = false;
  let showFindBar = $state(false);
  let showBacklinks = $state(false);
  let backlinks = $state<BacklinkDto[]>([]);

  // Track previous settings (plain variables, not reactive)
  let prevTheme: string | null = null;
  let prevFontSize: number | null = null;
  let prevLineHeight: number | null = null;
  let prevShowLineNumbers: boolean | null = null;

  onMount(() => {
    initEditor();
    // Update note titles for autocomplete
    updateNoteTitles(noteStore.noteList);
  });

  // Update note titles when noteList changes
  $effect(() => {
    updateNoteTitles(noteStore.noteList);
  });

  // Handle wiki link navigation
  async function handleWikiLinkClick(title: string) {
    try {
      // Save current note first if dirty
      if (noteStore.isDirty && noteStore.currentNote) {
        noteStore.cancelAutosave();
        await noteStore.save();
      }

      // Resolve wiki link (find or create note)
      const note = await resolveWikiLink(title);

      // Refresh note list in case a new note was created
      await noteStore.refreshList();

      // Navigate to the note
      if (onNavigateToNote) {
        onNavigateToNote(note.uid);
      } else {
        // Fallback: load directly
        await noteStore.load(note.uid);
      }
    } catch (e) {
      console.error('Failed to resolve wiki link:', e);
    }
  }

  onDestroy(() => {
    editorView?.destroy();
  });

  function initEditor() {
    if (!editorContainer) return;

    const theme = settingsStore.settings.theme;
    const fontSize = settingsStore.settings.editor.font_size;
    const lineHeight = settingsStore.settings.editor.line_height;
    const showLineNumbers = settingsStore.settings.editor.show_line_numbers;

    editorView = createEditor({
      parent: editorContainer,
      doc: noteStore.currentNote?.content ?? '',
      theme,
      fontSize,
      lineHeight,
      showLineNumbers,
      onChange: (content) => {
        if (!isUpdatingFromStore) {
          noteStore.updateContent(content);
        }
      },
      onWikiLinkClick: handleWikiLinkClick,
    });

    // Store initial values
    prevTheme = theme;
    prevFontSize = fontSize;
    prevLineHeight = lineHeight;
    prevShowLineNumbers = showLineNumbers;

    focusEditor(editorView);
  }

  // Recreate editor when settings change
  $effect(() => {
    const currentTheme = settingsStore.settings.theme;
    const currentFontSize = settingsStore.settings.editor.font_size;
    const currentLineHeight = settingsStore.settings.editor.line_height;
    const currentShowLineNumbers = settingsStore.settings.editor.show_line_numbers;

    // Skip if not initialized yet
    if (prevTheme === null) return;

    const settingsChanged =
      currentTheme !== prevTheme ||
      currentFontSize !== prevFontSize ||
      currentLineHeight !== prevLineHeight ||
      currentShowLineNumbers !== prevShowLineNumbers;

    if (settingsStore.isLoaded && editorContainer && editorView && settingsChanged) {
      untrack(() => {
        const currentContent = editorView!.state.doc.toString();
        editorView!.destroy();

        editorView = createEditor({
          parent: editorContainer,
          doc: currentContent,
          theme: currentTheme,
          fontSize: currentFontSize,
          lineHeight: currentLineHeight,
          showLineNumbers: currentShowLineNumbers,
          onChange: (content) => {
            if (!isUpdatingFromStore) {
              noteStore.updateContent(content);
            }
          },
          onWikiLinkClick: handleWikiLinkClick,
        });

        // Update previous values
        prevTheme = currentTheme;
        prevFontSize = currentFontSize;
        prevLineHeight = currentLineHeight;
        prevShowLineNumbers = currentShowLineNumbers;

        // Re-focus editor after recreation
        focusEditor(editorView);
      });
    }
  });

  // Update editor content when note changes (e.g., loading different note)
  $effect(() => {
    const content = noteStore.currentNote?.content;
    if (editorView && content !== undefined) {
      const currentContent = editorView.state.doc.toString();
      if (currentContent !== content) {
        isUpdatingFromStore = true;
        setEditorContent(editorView, content);
        isUpdatingFromStore = false;
      }
    }
  });

  // Handle keyboard shortcuts
  function handleKeydown(event: KeyboardEvent) {
    const shortcuts = settingsStore.settings.shortcuts ?? {
      save_note: 'Ctrl+S',
      find_in_note: 'Ctrl+F',
      backlink_panel: 'Ctrl+Shift+B',
    };

    if (matchShortcut(event, shortcuts.save_note)) {
      event.preventDefault();
      noteStore.save();
    } else if (matchShortcut(event, shortcuts.find_in_note)) {
      event.preventDefault();
      showFindBar = !showFindBar;
    } else if (matchShortcut(event, shortcuts.backlink_panel)) {
      event.preventDefault();
      toggleBacklinks();
    }
  }

  // Toggle backlink panel
  async function toggleBacklinks() {
    if (showBacklinks) {
      showBacklinks = false;
      return;
    }

    if (!noteStore.currentNote) return;

    try {
      backlinks = await getBacklinks(noteStore.currentNote.uid);
      showBacklinks = true;
    } catch (e) {
      console.error('Failed to get backlinks:', e);
    }
  }

  function handleCloseBacklinks() {
    showBacklinks = false;
    if (editorView) {
      editorView.focus();
    }
  }

  async function handleBacklinkSelect(uid: string) {
    // Save current note first if dirty
    if (noteStore.isDirty && noteStore.currentNote) {
      noteStore.cancelAutosave();
      await noteStore.save();
    }

    // Navigate to the note
    if (onNavigateToNote) {
      onNavigateToNote(uid);
    } else {
      await noteStore.load(uid);
    }
  }

  function handleCloseFindBar() {
    showFindBar = false;
  }

  // Ensure editor focus on click - but not when clicking find bar or backlink panel
  function handleClick(e: MouseEvent) {
    const target = e.target as HTMLElement;
    if (target.closest('.find-bar') || target.closest('.backlink-panel')) {
      return;
    }
    if (editorView) {
      editorView.focus();
    }
  }
</script>

<svelte:window onkeydown={handleKeydown} />

<!-- svelte-ignore a11y_click_events_have_key_events -->
<!-- svelte-ignore a11y_no_static_element_interactions -->
<div class="editor-wrapper" onclick={handleClick}>
  {#if showFindBar}
    <FindBar {editorView} onClose={handleCloseFindBar} />
  {/if}
  <div class="editor-container" bind:this={editorContainer}></div>

  {#if showBacklinks}
    <BacklinkPanel
      {backlinks}
      onSelect={handleBacklinkSelect}
      onClose={handleCloseBacklinks}
    />
  {/if}
</div>

<style>
  .editor-wrapper {
    flex: 1 1 0;
    min-height: 0;
    display: flex;
    flex-direction: column;
    overflow: hidden;
    position: relative;
  }

  .editor-container {
    flex: 1 1 0;
    min-height: 0;
    /* Allow CodeMirror to handle its own scrolling */
    overflow: visible;
  }

  /* CodeMirror container styles */
  .editor-container :global(.cm-editor) {
    height: 100%;
    width: 100%;
    font-family: 'SF Mono', 'Fira Code', 'JetBrains Mono', 'Menlo', monospace;
    background-color: var(--editor-bg);
    color: var(--fg-primary);
  }

  .editor-container :global(.cm-scroller) {
    overflow: auto !important;
    height: 100%;
    padding: 12px 0 16px 0;
    /* Prevent scroll chaining to parent elements */
    overscroll-behavior: contain;
  }

  .editor-container :global(.cm-gutters) {
    padding-left: 8px;
    background-color: var(--bg-secondary);
    color: var(--fg-muted);
  }

  .editor-container :global(.cm-content) {
    padding: 0 20px 0 12px;
    min-height: 100%;
    caret-color: var(--editor-cursor);
  }

  .editor-container :global(.cm-line) {
    padding: 0 4px;
  }

  .editor-container :global(.cm-cursor),
  .editor-container :global(.cm-dropCursor) {
    border-left-color: var(--editor-cursor);
  }

  /* Active line highlight disabled - same as normal background */
  .editor-container :global(.cm-activeLine) {
    background-color: transparent;
  }

  .editor-container :global(.cm-activeLineGutter) {
    background-color: transparent;
  }

  /* Selection background - only show when focused */
  .editor-container :global(.cm-selectionBackground) {
    background-color: transparent !important;
  }

  .editor-container :global(.cm-focused .cm-selectionBackground),
  .editor-container :global(.cm-content ::selection) {
    background-color: var(--editor-selection) !important;
  }

  /* Hide drop cursor indicator */
  .editor-container :global(.cm-dropCursor) {
    border-left-color: transparent !important;
  }

  /* Hide placeholder */
  .editor-container :global(.cm-placeholder) {
    display: none !important;
  }

  /* Hide gap cursor */
  .editor-container :global(.cm-gap-cursor),
  .editor-container :global(.cm-gapcursor) {
    display: none !important;
  }

  /* Live preview colors are defined in global.css for proper cascading */

  /* Hide default CodeMirror search panel (we use custom FindBar) */
  .editor-container :global(.cm-panel.cm-search),
  .editor-container :global(.cm-search.cm-panel) {
    display: none !important;
  }

  /* Search panel styles (kept for reference) */
  .editor-container :global(.cm-search) {
    display: flex;
    flex-wrap: wrap;
    align-items: center;
    gap: 8px;
    padding: 8px 12px;
    background: var(--bg-secondary);
    border-bottom: 1px solid var(--border-color);
  }

  .editor-container :global(.cm-search label) {
    display: flex;
    align-items: center;
    gap: 4px;
    font-size: 12px;
    color: var(--fg-secondary);
  }

  .editor-container :global(.cm-search input[type="checkbox"]) {
    margin: 0;
    accent-color: var(--accent-blue);
  }

  .editor-container :global(.cm-search .cm-textfield) {
    background: var(--bg-primary);
    border: 1px solid var(--border-color);
    border-radius: 4px;
    color: var(--fg-primary);
    font-size: 13px;
    padding: 4px 8px;
    outline: none;
    min-width: 150px;
  }

  .editor-container :global(.cm-search .cm-textfield:focus) {
    border-color: var(--accent-blue);
  }

  .editor-container :global(.cm-search .cm-textfield::placeholder) {
    color: var(--fg-muted);
  }

  .editor-container :global(.cm-search button) {
    background: var(--bg-highlight);
    border: 1px solid var(--border-color);
    border-radius: 4px;
    color: var(--fg-secondary);
    font-size: 12px;
    padding: 4px 8px;
    cursor: pointer;
    transition: all 0.15s;
  }

  .editor-container :global(.cm-search button:hover) {
    background: var(--bg-primary);
    color: var(--fg-primary);
  }

  .editor-container :global(.cm-search button[name="close"]) {
    padding: 4px 6px;
  }

  .editor-container :global(.cm-search br) {
    display: none;
  }

  /* Search match highlight - all matches */
  .editor-container :global(.cm-searchMatch) {
    background: color-mix(in srgb, var(--accent-orange) 35%, transparent) !important;
    border-radius: 2px;
    box-shadow: 0 0 0 1px color-mix(in srgb, var(--accent-orange) 50%, transparent);
  }

  /* Search match highlight - current/selected match */
  .editor-container :global(.cm-searchMatch-selected),
  .editor-container :global(.cm-searchMatch.cm-searchMatch-selected) {
    background: color-mix(in srgb, var(--accent-orange) 60%, transparent) !important;
    border-radius: 2px;
    box-shadow: 0 0 0 2px var(--accent-orange);
  }

  /* Selection match highlight (when selecting text) */
  .editor-container :global(.cm-selectionMatch) {
    background: color-mix(in srgb, var(--accent-cyan) 25%, transparent) !important;
    border-radius: 2px;
  }

  /* Focus styles */
  .editor-container :global(.cm-editor.cm-focused) {
    outline: none;
  }

  /* Remove any dotted outlines */
  .editor-container :global(.cm-line:focus),
  .editor-container :global(.cm-content:focus) {
    outline: none !important;
  }
</style>
