<script lang="ts">
  import { onMount, onDestroy } from 'svelte';
  import { noteStore, _internal as noteStoreInternal } from '$lib/stores/note.svelte';
  import { settingsStore } from '$lib/stores/settings.svelte';
  import { historyStore } from '$lib/stores/history.svelte';
  import { prepareHide } from '$lib/services/api';
  import { matchShortcut } from '$lib/utils/shortcuts';
  import Editor from '$lib/components/Editor.svelte';
  import Sidebar from '$lib/components/Sidebar.svelte';
  import TitleBar from '$lib/components/TitleBar.svelte';
  import SettingsModal from '$lib/components/SettingsModal.svelte';
  import CommandPalette from '$lib/components/CommandPalette.svelte';
  import HomeView from '$lib/components/HomeView.svelte';
  import TagEditDialog from '$lib/components/TagEditDialog.svelte';
  import { homeStore } from '$lib/stores/home.svelte';

  let sidebarOpen = $state(false);
  let settingsOpen = $state(false);
  let commandPaletteOpen = $state(false);
  let tagEditOpen = $state(false);
  let unlistenVisibility: (() => void) | null = null;
  let unlistenCreateNote: (() => void) | null = null;
  let unlistenMouseNav: (() => void) | null = null;

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

    // Setup mouse back/forward navigation
    unlistenMouseNav = setupMouseNavigation();
  });

  onDestroy(() => {
    unlistenVisibility?.();
    unlistenCreateNote?.();
    unlistenMouseNav?.();
    window.removeEventListener('beforeunload', handleBeforeUnload);
    // Cleanup autosave timer to prevent memory leaks
    noteStoreInternal.cleanup();
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

  function closeCommandPalette() {
    commandPaletteOpen = false;
  }

  async function handlePaletteSelect(uid: string) {
    await handleNoteSelect(uid);
  }

  function handleKeydown(event: KeyboardEvent) {
    const shortcuts = settingsStore.settings.shortcuts ?? {
      new_note: 'Ctrl+N',
      toggle_sidebar: 'Ctrl+M',
      open_settings: 'Ctrl+,',
      command_palette: 'Ctrl+P',
      history_back: 'Ctrl+H',
      history_forward: 'Ctrl+L',
      save_note: 'Ctrl+S',
      find_in_note: 'Ctrl+F',
      backlink_panel: 'Ctrl+Shift+B',
    };

    // Home view shortcut (Ctrl+Shift+A) - works everywhere
    if (matchShortcut(event, 'Ctrl+Shift+A')) {
      event.preventDefault();
      openHome();
      return;
    }

    // Tag edit shortcut (Ctrl+T) when not in home view
    if (matchShortcut(event, 'Ctrl+T') && !homeStore.isVisible && noteStore.currentNote) {
      event.preventDefault();
      tagEditOpen = true;
      return;
    }

    // Command palette shortcut (works everywhere)
    if (matchShortcut(event, shortcuts.command_palette)) {
      event.preventDefault();
      commandPaletteOpen = !commandPaletteOpen;
      return;
    }

    // Don't handle other shortcuts when command palette or home view is open
    if (commandPaletteOpen || homeStore.isVisible) {
      return;
    }

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
    // History back
    if (matchShortcut(event, shortcuts.history_back)) {
      event.preventDefault();
      handleGoBack();
      return;
    }
    // History forward
    if (matchShortcut(event, shortcuts.history_forward)) {
      event.preventDefault();
      handleGoForward();
      return;
    }
  }

  // Handle mouse back/forward buttons (uses native listener to avoid scroll interference)
  function setupMouseNavigation() {
    function handleMouseUp(event: MouseEvent) {
      // Button 3 = Back, Button 4 = Forward
      // Use mouseup instead of mousedown to avoid scroll interference
      if (event.button === 3) {
        event.preventDefault();
        handleGoBack();
      } else if (event.button === 4) {
        event.preventDefault();
        handleGoForward();
      }
    }
    window.addEventListener('mouseup', handleMouseUp);
    return () => window.removeEventListener('mouseup', handleMouseUp);
  }

  // Go back in history
  async function handleGoBack() {
    if (!historyStore.canGoBack) return;

    // Save current note before navigating
    if (noteStore.isDirty && noteStore.currentNote) {
      noteStore.cancelAutosave();
      await noteStore.save();
    }

    const uid = historyStore.goBack();
    if (uid) {
      await noteStore.load(uid, { skipHistory: true });
    }
  }

  // Go forward in history
  async function handleGoForward() {
    if (!historyStore.canGoForward) return;

    // Save current note before navigating
    if (noteStore.isDirty && noteStore.currentNote) {
      noteStore.cancelAutosave();
      await noteStore.save();
    }

    const uid = historyStore.goForward();
    if (uid) {
      await noteStore.load(uid, { skipHistory: true });
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

  function openHome() {
    closeSidebar();
    homeStore.show();
  }

  async function handleNavigateFromHome(uid: string) {
    await handleNoteSelect(uid);
  }

  function closeTagEdit() {
    tagEditOpen = false;
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
    onOpenHome={openHome}
  />

  <main class="main-content">
    <Editor />
  </main>

  {#if settingsOpen}
    <SettingsModal onClose={closeSettings} />
  {/if}

  {#if commandPaletteOpen}
    <CommandPalette onSelect={handlePaletteSelect} onClose={closeCommandPalette} />
  {/if}

  <HomeView onNavigateToNote={handleNavigateFromHome} />

  {#if tagEditOpen && noteStore.currentNote}
    <TagEditDialog
      bind:open={tagEditOpen}
      noteUid={noteStore.currentNote.uid}
      onClose={closeTagEdit}
      onSave={() => noteStore.refreshList()}
    />
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
