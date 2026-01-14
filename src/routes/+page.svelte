<script lang="ts">
  import { onMount, onDestroy } from 'svelte';
  import { noteStore } from '$lib/stores/note.svelte';
  import { settingsStore } from '$lib/stores/settings.svelte';
  import { prepareHide } from '$lib/services/api';
  import Editor from '$lib/components/Editor.svelte';
  import Sidebar from '$lib/components/Sidebar.svelte';
  import TitleBar from '$lib/components/TitleBar.svelte';
  import SettingsModal from '$lib/components/SettingsModal.svelte';

  let sidebarOpen = $state(false);
  let settingsOpen = $state(false);
  let unlistenVisibility: (() => void) | null = null;
  let unlistenCreateNote: (() => void) | null = null;

  // Save before window hides (global hotkey or close button)
  async function handleBeforeHide() {
    noteStore.cancelAutosave();

    if (!noteStore.currentNote) return;

    // 空のノートはファイルを削除（他のノートに切り替えない）
    const wasDeleted = await noteStore.deleteIfEmpty();
    if (wasDeleted) {
      return;
    }

    // 内容がある場合は保存処理
    try {
      await prepareHide(noteStore.currentNote.uid, noteStore.currentNote.content);
    } catch (e) {
      console.error('Failed to save before hide:', e);
    }
  }

  onMount(async () => {
    // Listen for visibility changes (Tauri window events)
    try {
      const { getCurrentWindow } = await import('@tauri-apps/api/window');
      const currentWindow = getCurrentWindow();
      unlistenVisibility = await currentWindow.onCloseRequested(async (event) => {
        await handleBeforeHide();
        // Let Tauri handle the close (which hides instead of closing)
      });
    } catch (e) {
      console.error('Failed to setup window listener:', e);
    }

    // Listen for window-shown event (from IPC/tray)
    try {
      const { listen } = await import('@tauri-apps/api/event');
      unlistenCreateNote = await listen('create-new-note', async () => {
        console.log('[Event] create-new-note received');
        await handleWindowShown();
      });
    } catch (e) {
      console.error('Failed to setup create-new-note listener:', e);
    }

    // Load settings and note list
    await settingsStore.load();
    await noteStore.refreshList();

    // Apply theme and mode
    document.documentElement.dataset.theme = settingsStore.settings.theme;
    document.documentElement.dataset.mode = settingsStore.settings.theme_mode;

    // Determine which note to load
    if (noteStore.noteList.length === 0) {
      // No notes exist, create new
      await noteStore.createNew();
    } else {
      // Check if we should restore last note
      const { restore_last_note, last_note_uid } = settingsStore.settings;
      let noteToLoad: string | null = null;

      if (restore_last_note && last_note_uid) {
        // Verify the last note still exists
        const exists = noteStore.noteList.some(n => n.uid === last_note_uid);
        if (exists) {
          noteToLoad = last_note_uid;
        }
      }

      // Fall back to most recent note
      if (!noteToLoad) {
        noteToLoad = noteStore.noteList[0].uid;
      }

      await noteStore.load(noteToLoad);
    }

    // Handle page unload (browser/app close)
    window.addEventListener('beforeunload', handleBeforeUnload);
  });

  onDestroy(() => {
    unlistenVisibility?.();
    unlistenCreateNote?.();
    window.removeEventListener('beforeunload', handleBeforeUnload);
  });

  function handleBeforeUnload(event: BeforeUnloadEvent) {
    if (noteStore.isDirty) {
      // Trigger sync save attempt
      handleBeforeHide();
      // Show confirmation dialog if content might be lost
      event.preventDefault();
    }
  }

  function toggleSidebar() {
    sidebarOpen = !sidebarOpen;
  }

  function closeSidebar() {
    sidebarOpen = false;
  }

  function openSettings() {
    settingsOpen = true;
  }

  function closeSettings() {
    settingsOpen = false;
  }

  function parseShortcut(shortcut: string): { ctrl: boolean; shift: boolean; alt: boolean; key: string } {
    const parts = shortcut.toLowerCase().split('+');
    return {
      ctrl: parts.includes('ctrl'),
      shift: parts.includes('shift'),
      alt: parts.includes('alt'),
      key: parts[parts.length - 1] === 'space' ? ' ' : parts[parts.length - 1],
    };
  }

  function matchShortcut(event: KeyboardEvent, shortcut: string): boolean {
    const parsed = parseShortcut(shortcut);
    const eventKey = event.key.toLowerCase();
    return (
      event.ctrlKey === parsed.ctrl &&
      event.shiftKey === parsed.shift &&
      event.altKey === parsed.alt &&
      eventKey === parsed.key
    );
  }

  function handleKeydown(event: KeyboardEvent) {
    // Don't handle shortcuts when settings modal is open (it has its own handler)
    if (settingsOpen) {
      // Only handle Escape to close settings
      if (event.key === 'Escape') {
        closeSettings();
      }
      return;
    }

    // Skip if typing in an input field (except for specific shortcuts)
    const target = event.target as HTMLElement;
    const isInput = target.tagName === 'INPUT' || target.tagName === 'TEXTAREA';

    // Escape to close sidebar (always works)
    if (event.key === 'Escape') {
      if (sidebarOpen) {
        closeSidebar();
      }
      return;
    }

    // Skip other shortcuts if typing in input
    if (isInput) return;

    const shortcuts = settingsStore.settings.shortcuts ?? {
      new_note: 'Ctrl+N',
      toggle_sidebar: 'Ctrl+M',
      open_settings: 'Ctrl+,',
    };

    // New note shortcut
    if (matchShortcut(event, shortcuts.new_note)) {
      event.preventDefault();
      handleNewNote();
      return;
    }
    // Toggle sidebar shortcut
    if (matchShortcut(event, shortcuts.toggle_sidebar)) {
      event.preventDefault();
      toggleSidebar();
      return;
    }
    // Open settings shortcut
    if (matchShortcut(event, shortcuts.open_settings)) {
      event.preventDefault();
      openSettings();
      return;
    }
  }

  async function handleNoteSelect(uid: string) {
    // Save current note before switching
    if (noteStore.isDirty && noteStore.currentNote) {
      noteStore.cancelAutosave();
      await noteStore.save();
    }
    await noteStore.load(uid);
    closeSidebar();
  }

  async function handleNewNote() {
    // Save current note before creating new
    if (noteStore.isDirty && noteStore.currentNote) {
      noteStore.cancelAutosave();
      await noteStore.save();
    }
    await noteStore.createNew();
    closeSidebar();
  }

  async function handleWindowShown() {
    // Refresh note list in case changes were made externally
    await noteStore.refreshList();

    // Check setting: restore last note or create new
    if (settingsStore.settings.restore_last_note) {
      // Keep the current note (don't create a new one)
      console.log('[WindowShown] restore_last_note enabled, keeping current note');
      return;
    }

    // Create new note (default behavior)
    console.log('[WindowShown] Creating new note');
    await handleNewNote();
  }

  async function handleDeleteNote(uid: string) {
    try {
      await noteStore.delete(uid);
    } catch (e) {
      console.error('Failed to delete note:', e);
    }
  }
</script>

<svelte:window onkeydown={handleKeydown} />

<div class="app">
  <TitleBar onClose={handleBeforeHide} onMenuClick={toggleSidebar} />

  <Sidebar
    isOpen={sidebarOpen}
    onClose={closeSidebar}
    onNoteSelect={handleNoteSelect}
    onNewNote={handleNewNote}
    onOpenSettings={openSettings}
    onDeleteNote={handleDeleteNote}
  />

  <main class="main-content">
    <Editor />
  </main>

  {#if settingsOpen}
    <SettingsModal onClose={closeSettings} />
  {/if}
</div>

<style>
  .app {
    position: absolute;
    top: 0;
    left: 0;
    right: 0;
    bottom: 0;
    display: flex;
    flex-direction: column;
    overflow: hidden;
    background-color: var(--bg-primary);
  }

  .main-content {
    flex: 1 1 0;
    min-height: 0;
    display: flex;
    flex-direction: column;
    overflow: hidden;
  }
</style>
