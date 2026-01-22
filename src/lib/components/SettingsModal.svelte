<script lang="ts">
  import { onMount } from 'svelte';
  import { open } from '@tauri-apps/plugin-dialog';
  import { settingsStore } from '$lib/stores/settings.svelte';

  interface Props {
    onClose: () => void;
  }

  let { onClose }: Props = $props();

  import type { ThemeName, ThemeMode } from '$lib/types';

  // Local state for form
  let fontSize = $state(settingsStore.settings.editor.font_size);
  let lineHeight = $state(settingsStore.settings.editor.line_height);
  let showLineNumbers = $state(settingsStore.settings.editor.show_line_numbers);
  let theme = $state<ThemeName>(settingsStore.settings.theme);
  let themeMode = $state<ThemeMode>(settingsStore.settings.theme_mode);
  let storageDirectory = $state(settingsStore.settings.storage_directory);
  let restoreLastNote = $state(settingsStore.settings.restore_last_note);
  let hotkey = $state(settingsStore.settings.hotkey);
  let hotkeyError = $state('');

  // Shortcuts - Page level
  let shortcutNewNote = $state(settingsStore.settings.shortcuts?.new_note ?? 'Ctrl+N');
  let shortcutToggleSidebar = $state(settingsStore.settings.shortcuts?.toggle_sidebar ?? 'Ctrl+M');
  let shortcutOpenSettings = $state(settingsStore.settings.shortcuts?.open_settings ?? 'Ctrl+,');
  let shortcutCommandPalette = $state(settingsStore.settings.shortcuts?.command_palette ?? 'Ctrl+P');
  let shortcutHistoryBack = $state(settingsStore.settings.shortcuts?.history_back ?? 'Ctrl+H');
  let shortcutHistoryForward = $state(settingsStore.settings.shortcuts?.history_forward ?? 'Ctrl+L');
  // Shortcuts - Editor
  let shortcutSaveNote = $state(settingsStore.settings.shortcuts?.save_note ?? 'Ctrl+S');
  let shortcutFindInNote = $state(settingsStore.settings.shortcuts?.find_in_note ?? 'Ctrl+F');
  let shortcutBacklinkPanel = $state(settingsStore.settings.shortcuts?.backlink_panel ?? 'Ctrl+Shift+B');

  // テーマ一覧
  const themes: { id: ThemeName; name: string }[] = [
    { id: 'tokyo-night', name: 'Tokyo Night' },
    { id: 'kanagawa', name: 'Kanagawa' },
    { id: 'monokai', name: 'Monokai' },
    { id: 'gruvbox', name: 'Gruvbox' },
    { id: 'dracula', name: 'Dracula' },
    { id: 'catppuccin', name: 'Catppuccin' },
    { id: 'synthwave', name: 'Synthwave' },
  ];

  let modalElement: HTMLDivElement;
  let closeButton: HTMLButtonElement;
  let hotkeyInput: HTMLInputElement;

  onMount(async () => {
    closeButton?.focus();
    // 現在のホットキーを取得
    await settingsStore.refreshHotkey();
    hotkey = settingsStore.settings.hotkey;
    // ショートカットを更新
    const s = settingsStore.settings.shortcuts;
    shortcutNewNote = s?.new_note ?? 'Ctrl+N';
    shortcutToggleSidebar = s?.toggle_sidebar ?? 'Ctrl+M';
    shortcutOpenSettings = s?.open_settings ?? 'Ctrl+,';
    shortcutCommandPalette = s?.command_palette ?? 'Ctrl+P';
    shortcutHistoryBack = s?.history_back ?? 'Ctrl+H';
    shortcutHistoryForward = s?.history_forward ?? 'Ctrl+L';
    shortcutSaveNote = s?.save_note ?? 'Ctrl+S';
    shortcutFindInNote = s?.find_in_note ?? 'Ctrl+F';
    shortcutBacklinkPanel = s?.backlink_panel ?? 'Ctrl+Shift+B';
  });

  let isEditingHotkey = $state(false);
  let hotkeyInputValue = $state('');

  // Shortcut editing states
  let editingShortcut = $state<string | null>(null);
  let shortcutInputValue = $state('');

  function startEditingHotkey() {
    isEditingHotkey = true;
    hotkeyInputValue = hotkey;
    hotkeyError = '';
    setTimeout(() => hotkeyInput?.focus(), 0);
  }

  function handleHotkeyInputChange(event: Event) {
    const target = event.target as HTMLInputElement;
    hotkeyInputValue = target.value;
  }

  function handleHotkeyInputKeydown(event: KeyboardEvent) {
    if (event.key === 'Escape') {
      isEditingHotkey = false;
      hotkeyInputValue = '';
    } else if (event.key === 'Enter') {
      finishEditingHotkey();
    }
  }

  function finishEditingHotkey() {
    // 入力を正規化（例: "ctrl+shift+space" → "Ctrl+Shift+Space"）
    const normalized = hotkeyInputValue
      .split('+')
      .map(part => {
        const p = part.trim().toLowerCase();
        if (p === 'ctrl' || p === 'control') return 'Ctrl';
        if (p === 'shift') return 'Shift';
        if (p === 'alt') return 'Alt';
        if (p === 'super' || p === 'meta' || p === 'win') return 'Super';
        if (p === 'space') return 'Space';
        return p.toUpperCase();
      })
      .filter(p => p.length > 0)
      .join('+');

    if (normalized) {
      hotkey = normalized;
    }
    isEditingHotkey = false;
    hotkeyInputValue = '';
  }

  function handleHotkeyBlur() {
    setTimeout(() => {
      if (isEditingHotkey) {
        finishEditingHotkey();
      }
    }, 100);
  }

  function startEditingShortcut(name: string, currentValue: string) {
    editingShortcut = name;
    shortcutInputValue = currentValue;
  }

  function normalizeShortcut(value: string): string {
    return value
      .split('+')
      .map(part => {
        const p = part.trim().toLowerCase();
        if (p === 'ctrl' || p === 'control') return 'Ctrl';
        if (p === 'shift') return 'Shift';
        if (p === 'alt') return 'Alt';
        if (p === 'super' || p === 'meta' || p === 'win') return 'Super';
        if (p === 'space') return 'Space';
        if (p.length === 1) return p.toUpperCase();
        return p.charAt(0).toUpperCase() + p.slice(1);
      })
      .filter(p => p.length > 0)
      .join('+');
  }

  function finishEditingShortcut() {
    const normalized = normalizeShortcut(shortcutInputValue);
    if (normalized && editingShortcut) {
      switch (editingShortcut) {
        case 'new_note': shortcutNewNote = normalized; break;
        case 'toggle_sidebar': shortcutToggleSidebar = normalized; break;
        case 'open_settings': shortcutOpenSettings = normalized; break;
        case 'command_palette': shortcutCommandPalette = normalized; break;
        case 'history_back': shortcutHistoryBack = normalized; break;
        case 'history_forward': shortcutHistoryForward = normalized; break;
        case 'save_note': shortcutSaveNote = normalized; break;
        case 'find_in_note': shortcutFindInNote = normalized; break;
        case 'backlink_panel': shortcutBacklinkPanel = normalized; break;
      }
    }
    editingShortcut = null;
    shortcutInputValue = '';
  }

  function handleShortcutInputChange(event: Event) {
    const target = event.target as HTMLInputElement;
    shortcutInputValue = target.value;
  }

  function handleShortcutInputKeydown(event: KeyboardEvent) {
    if (event.key === 'Escape') {
      editingShortcut = null;
      shortcutInputValue = '';
    } else if (event.key === 'Enter') {
      finishEditingShortcut();
    }
  }

  function handleShortcutBlur() {
    setTimeout(() => {
      if (editingShortcut) {
        finishEditingShortcut();
      }
    }, 100);
  }

  function applyTheme(themeName: ThemeName, mode: ThemeMode) {
    document.documentElement.dataset.theme = themeName;
    document.documentElement.dataset.mode = mode;
  }

  async function handleModeChange(newMode: ThemeMode) {
    themeMode = newMode;
    applyTheme(theme, newMode);
    await settingsStore.setThemeMode(newMode);
  }

  async function handleThemeChange(newTheme: ThemeName) {
    theme = newTheme;
    applyTheme(newTheme, themeMode);
    await settingsStore.setTheme(newTheme);
  }

  async function handleBrowseDirectory() {
    const selected = await open({
      directory: true,
      multiple: false,
      defaultPath: storageDirectory || undefined,
      title: 'Select Storage Directory',
    });

    if (selected && typeof selected === 'string') {
      storageDirectory = selected;
    }
  }

  async function handleSave() {
    // Save editor settings if changed
    if (
      fontSize !== settingsStore.settings.editor.font_size ||
      lineHeight !== settingsStore.settings.editor.line_height ||
      showLineNumbers !== settingsStore.settings.editor.show_line_numbers
    ) {
      await settingsStore.setEditorSettings({
        font_size: fontSize,
        line_height: lineHeight,
        show_line_numbers: showLineNumbers,
      });
    }

    // Save storage directory if changed
    if (storageDirectory !== settingsStore.settings.storage_directory) {
      await settingsStore.setStorageDirectory(storageDirectory);
    }

    // Save restore last note setting if changed
    if (restoreLastNote !== settingsStore.settings.restore_last_note) {
      await settingsStore.setRestoreLastNote(restoreLastNote);
    }

    // Save hotkey if changed
    if (hotkey !== settingsStore.settings.hotkey) {
      try {
        await settingsStore.setHotkey(hotkey);
        hotkeyError = '';
      } catch (e) {
        hotkeyError = e instanceof Error ? e.message : 'Failed to save hotkey';
        return; // Don't close on error
      }
    }

    // Save shortcuts if changed
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
    if (
      shortcutNewNote !== shortcuts.new_note ||
      shortcutToggleSidebar !== shortcuts.toggle_sidebar ||
      shortcutOpenSettings !== shortcuts.open_settings ||
      shortcutCommandPalette !== shortcuts.command_palette ||
      shortcutHistoryBack !== shortcuts.history_back ||
      shortcutHistoryForward !== shortcuts.history_forward ||
      shortcutSaveNote !== shortcuts.save_note ||
      shortcutFindInNote !== shortcuts.find_in_note ||
      shortcutBacklinkPanel !== shortcuts.backlink_panel
    ) {
      await settingsStore.setShortcuts({
        new_note: shortcutNewNote,
        toggle_sidebar: shortcutToggleSidebar,
        open_settings: shortcutOpenSettings,
        command_palette: shortcutCommandPalette,
        history_back: shortcutHistoryBack,
        history_forward: shortcutHistoryForward,
        save_note: shortcutSaveNote,
        find_in_note: shortcutFindInNote,
        backlink_panel: shortcutBacklinkPanel,
      });
    }

    onClose();
  }

  function handleBackdropClick(event: MouseEvent) {
    if (event.target === event.currentTarget) {
      onClose();
    }
  }

  function handleKeydown(event: KeyboardEvent) {
    // Stop propagation to prevent other handlers from firing
    event.stopPropagation();

    if (event.key === 'Escape') {
      onClose();
    } else if (event.key === 'Tab' && modalElement) {
      // Focus trap
      const focusable = modalElement.querySelectorAll<HTMLElement>(
        'button:not([disabled]), input:not([disabled]), select:not([disabled]), [tabindex]:not([tabindex="-1"])'
      );
      const firstElement = focusable[0];
      const lastElement = focusable[focusable.length - 1];

      if (event.shiftKey && document.activeElement === firstElement) {
        event.preventDefault();
        lastElement?.focus();
      } else if (!event.shiftKey && document.activeElement === lastElement) {
        event.preventDefault();
        firstElement?.focus();
      }
    }
  }
</script>

<svelte:window onkeydown={handleKeydown} />

<!-- svelte-ignore a11y_click_events_have_key_events a11y_no_static_element_interactions -->
<div class="modal-backdrop" onclick={handleBackdropClick}>
  <div bind:this={modalElement} class="modal" role="dialog" aria-modal="true" aria-labelledby="settings-title">
    <header class="modal-header">
      <h2 id="settings-title">Settings</h2>
      <button bind:this={closeButton} class="close-btn" onclick={onClose} aria-label="Close">
        <svg width="20" height="20" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
          <line x1="18" y1="6" x2="6" y2="18"></line>
          <line x1="6" y1="6" x2="18" y2="18"></line>
        </svg>
      </button>
    </header>

    <div class="modal-content">
      <!-- Storage Section -->
      <section class="settings-section">
        <h3>Storage</h3>
        <div class="setting-row storage-row">
          <label for="storage-directory">Save Location</label>
          <div class="directory-input">
            <input
              type="text"
              id="storage-directory"
              bind:value={storageDirectory}
              readonly
              class="directory-path"
            />
            <button
              class="btn btn-browse"
              onclick={handleBrowseDirectory}
              disabled={settingsStore.isSaving}
            >
              Browse
            </button>
          </div>
        </div>
      </section>

      <!-- Theme Mode Section -->
      <section class="settings-section">
        <h3>Appearance</h3>
        <div class="mode-options">
          <button
            class="mode-option"
            class:active={themeMode === 'dark'}
            onclick={() => handleModeChange('dark')}
            disabled={settingsStore.isSaving}
          >
            <svg width="18" height="18" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
              <path d="M21 12.79A9 9 0 1 1 11.21 3 7 7 0 0 0 21 12.79z"></path>
            </svg>
            <span>Dark</span>
          </button>
          <button
            class="mode-option"
            class:active={themeMode === 'light'}
            onclick={() => handleModeChange('light')}
            disabled={settingsStore.isSaving}
          >
            <svg width="18" height="18" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
              <circle cx="12" cy="12" r="5"></circle>
              <line x1="12" y1="1" x2="12" y2="3"></line>
              <line x1="12" y1="21" x2="12" y2="23"></line>
              <line x1="4.22" y1="4.22" x2="5.64" y2="5.64"></line>
              <line x1="18.36" y1="18.36" x2="19.78" y2="19.78"></line>
              <line x1="1" y1="12" x2="3" y2="12"></line>
              <line x1="21" y1="12" x2="23" y2="12"></line>
              <line x1="4.22" y1="19.78" x2="5.64" y2="18.36"></line>
              <line x1="18.36" y1="5.64" x2="19.78" y2="4.22"></line>
            </svg>
            <span>Light</span>
          </button>
        </div>
      </section>

      <!-- Color Theme Section -->
      <section class="settings-section">
        <div class="setting-row">
          <label for="color-theme">Color Theme</label>
          <select
            id="color-theme"
            value={theme}
            onchange={(e) => handleThemeChange(e.currentTarget.value as ThemeName)}
            class="select-input"
            disabled={settingsStore.isSaving}
          >
            {#each themes as t}
              <option value={t.id}>{t.name}</option>
            {/each}
          </select>
        </div>
      </section>

      <!-- Editor Section -->
      <section class="settings-section">
        <h3>Editor</h3>

        <div class="setting-row">
          <label for="font-size">Font Size</label>
          <div class="input-group">
            <input
              type="number"
              id="font-size"
              bind:value={fontSize}
              min="10"
              max="24"
            />
            <span class="input-suffix">px</span>
          </div>
        </div>

        <div class="setting-row">
          <label for="line-height">Line Height</label>
          <div class="input-group">
            <input
              type="number"
              id="line-height"
              bind:value={lineHeight}
              min="1"
              max="3"
              step="0.1"
            />
          </div>
        </div>

        <div class="setting-row">
          <label for="show-line-numbers">Show Line Numbers</label>
          <label class="toggle">
            <input
              type="checkbox"
              id="show-line-numbers"
              bind:checked={showLineNumbers}
            />
            <span class="toggle-slider"></span>
          </label>
        </div>
      </section>

      <!-- Behavior Section -->
      <section class="settings-section">
        <h3>Behavior</h3>
        <div class="setting-row">
          <label for="restore-last-note">On window show</label>
          <select
            id="restore-last-note"
            value={restoreLastNote ? 'restore' : 'new'}
            onchange={(e) => restoreLastNote = e.currentTarget.value === 'restore'}
            class="select-input"
          >
            <option value="new">Create new note</option>
            <option value="restore">Restore last note</option>
          </select>
        </div>
      </section>

      <!-- Shortcuts Section -->
      <section class="settings-section">
        <h3>Keyboard Shortcuts</h3>
        <div class="shortcuts-list">
          <!-- Global hotkey -->
          <div class="shortcut-row editable">
            <span class="shortcut-desc">Toggle window (Global)</span>
            <div class="hotkey-input-wrapper">
              {#if isEditingHotkey}
                <input
                  bind:this={hotkeyInput}
                  type="text"
                  class="hotkey-input"
                  value={hotkeyInputValue}
                  placeholder="e.g., Ctrl+Shift+Space"
                  oninput={handleHotkeyInputChange}
                  onkeydown={handleHotkeyInputKeydown}
                  onblur={handleHotkeyBlur}
                />
              {:else}
                <button
                  class="hotkey-button"
                  onclick={startEditingHotkey}
                  disabled={settingsStore.isSaving}
                >
                  <kbd>{hotkey}</kbd>
                  <span class="edit-icon">Edit</span>
                </button>
              {/if}
            </div>
          </div>
          {#if hotkeyError}
            <div class="hotkey-error">{hotkeyError}</div>
          {/if}

          <!-- Page-level shortcuts -->
          {#each [
            { key: 'new_note', label: 'New note', value: shortcutNewNote },
            { key: 'toggle_sidebar', label: 'Toggle sidebar', value: shortcutToggleSidebar },
            { key: 'open_settings', label: 'Settings', value: shortcutOpenSettings },
            { key: 'command_palette', label: 'Command palette', value: shortcutCommandPalette },
            { key: 'history_back', label: 'History back', value: shortcutHistoryBack },
            { key: 'history_forward', label: 'History forward', value: shortcutHistoryForward },
            { key: 'save_note', label: 'Save note', value: shortcutSaveNote },
            { key: 'find_in_note', label: 'Find in note', value: shortcutFindInNote },
            { key: 'backlink_panel', label: 'Backlink panel', value: shortcutBacklinkPanel },
          ] as item (item.key)}
            <div class="shortcut-row editable">
              <span class="shortcut-desc">{item.label}</span>
              <div class="hotkey-input-wrapper">
                {#if editingShortcut === item.key}
                  <input
                    type="text"
                    class="hotkey-input"
                    value={shortcutInputValue}
                    placeholder="e.g., Ctrl+N"
                    oninput={handleShortcutInputChange}
                    onkeydown={handleShortcutInputKeydown}
                    onblur={handleShortcutBlur}
                  />
                {:else}
                  <button
                    class="hotkey-button"
                    onclick={() => startEditingShortcut(item.key, item.value)}
                    disabled={settingsStore.isSaving}
                  >
                    <kbd>{item.value}</kbd>
                    <span class="edit-icon">Edit</span>
                  </button>
                {/if}
              </div>
            </div>
          {/each}
        </div>
      </section>
    </div>

    <footer class="modal-footer">
      <button class="btn btn-secondary" onclick={onClose}>Cancel</button>
      <button class="btn btn-primary" onclick={handleSave} disabled={settingsStore.isSaving}>
        {settingsStore.isSaving ? 'Saving...' : 'Save'}
      </button>
    </footer>
  </div>
</div>

<style>
  .modal-backdrop {
    position: fixed;
    inset: 0;
    display: flex;
    align-items: center;
    justify-content: center;
    background: rgba(0, 0, 0, 0.6);
    z-index: 200;
  }

  .modal {
    width: 90%;
    max-width: 480px;
    max-height: 80vh;
    background: var(--bg-secondary);
    border-radius: 12px;
    border: 1px solid var(--border-color);
    display: flex;
    flex-direction: column;
    overflow: hidden;
  }

  .modal-header {
    display: flex;
    align-items: center;
    justify-content: space-between;
    padding: 16px 20px;
    background: var(--bg-secondary);
    border-bottom: 1px solid var(--border-color);
  }

  .modal-header h2 {
    font-size: 16px;
    font-weight: 600;
    color: var(--fg-primary);
  }

  .close-btn {
    display: flex;
    align-items: center;
    justify-content: center;
    width: 32px;
    height: 32px;
    border-radius: 6px;
    color: var(--fg-muted);
    transition: all 0.15s;
  }

  .close-btn:hover {
    background: var(--bg-highlight);
    color: var(--fg-primary);
  }

  .modal-content {
    flex: 1;
    min-height: 0;
    overflow-y: auto;
    overflow-x: hidden;
    padding: 20px;
    background: var(--bg-secondary);
  }

  .settings-section {
    margin-bottom: 24px;
  }

  .settings-section:last-child {
    margin-bottom: 0;
  }

  .settings-section h3 {
    font-size: 12px;
    font-weight: 600;
    text-transform: uppercase;
    letter-spacing: 0.05em;
    color: var(--fg-muted);
    margin-bottom: 12px;
  }

  /* Mode (Dark/Light) options */
  .mode-options {
    display: flex;
    gap: 8px;
  }

  .mode-option {
    flex: 1;
    display: flex;
    align-items: center;
    justify-content: center;
    gap: 8px;
    padding: 10px 16px;
    border-radius: 8px;
    border: 2px solid var(--border-color);
    background: var(--bg-tertiary);
    transition: all 0.15s;
  }

  .mode-option:hover:not(:disabled) {
    border-color: var(--fg-muted);
  }

  .mode-option:disabled {
    opacity: 0.6;
    cursor: not-allowed;
  }

  .mode-option.active {
    border-color: var(--accent-blue);
    background: var(--bg-highlight);
  }

  .mode-option span {
    font-size: 13px;
    font-weight: 500;
    color: var(--fg-secondary);
  }

  .mode-option.active span {
    color: var(--fg-primary);
  }

  .setting-row {
    display: flex;
    align-items: center;
    justify-content: space-between;
    margin-bottom: 12px;
  }

  .setting-row label {
    font-size: 13px;
    color: var(--fg-primary);
  }

  .storage-row {
    flex-direction: column;
    align-items: stretch;
    gap: 8px;
  }

  .directory-input {
    display: flex;
    gap: 8px;
  }

  .directory-path {
    flex: 1;
    padding: 8px 12px;
    background: var(--bg-tertiary);
    border: 1px solid var(--border-color);
    border-radius: 6px;
    color: var(--fg-secondary);
    font-size: 12px;
    font-family: 'SF Mono', 'Fira Code', monospace;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  .btn-browse {
    padding: 8px 12px;
    background: var(--bg-tertiary);
    border: 1px solid var(--border-color);
    border-radius: 6px;
    color: var(--fg-primary);
    font-size: 12px;
    white-space: nowrap;
    transition: all 0.15s;
  }

  .btn-browse:hover:not(:disabled) {
    background: var(--bg-highlight);
    border-color: var(--fg-muted);
  }

  .btn-browse:disabled {
    opacity: 0.6;
    cursor: not-allowed;
  }

  .input-group {
    display: flex;
    align-items: center;
    gap: 4px;
  }

  .input-group input {
    width: 72px;
    padding: 6px 10px;
    background: var(--bg-tertiary);
    border: 1px solid var(--border-color);
    border-radius: 6px;
    color: var(--fg-primary);
    font-size: 13px;
    text-align: right;
    -moz-appearance: textfield;
  }

  .input-group input::-webkit-outer-spin-button,
  .input-group input::-webkit-inner-spin-button {
    -webkit-appearance: none;
    margin: 0;
  }

  .input-group input:focus {
    outline: none;
    border-color: var(--accent-blue);
  }

  .input-suffix {
    font-size: 12px;
    color: var(--fg-muted);
  }

  .toggle {
    position: relative;
    display: inline-block;
    width: 40px;
    height: 22px;
    cursor: pointer;
  }

  .toggle input {
    opacity: 0;
    width: 0;
    height: 0;
  }

  .toggle-slider {
    position: absolute;
    inset: 0;
    background: var(--bg-highlight);
    border-radius: 22px;
    transition: all 0.2s;
  }

  .toggle-slider::before {
    content: '';
    position: absolute;
    width: 16px;
    height: 16px;
    left: 3px;
    bottom: 3px;
    background: var(--fg-muted);
    border-radius: 50%;
    transition: all 0.2s;
  }

  .toggle input:checked + .toggle-slider {
    background: var(--accent-blue);
  }

  .toggle input:checked + .toggle-slider::before {
    transform: translateX(18px);
    background: #fff;
  }

  .select-input {
    padding: 6px 32px 6px 10px;
    background: var(--bg-tertiary);
    border: 1px solid var(--border-color);
    border-radius: 6px;
    color: var(--fg-primary);
    font-size: 13px;
    cursor: pointer;
    appearance: none;
    -webkit-appearance: none;
    -moz-appearance: none;
    background-image: url("data:image/svg+xml,%3Csvg xmlns='http://www.w3.org/2000/svg' width='12' height='12' viewBox='0 0 24 24' fill='none' stroke='%23565f89' stroke-width='2'%3E%3Cpolyline points='6 9 12 15 18 9'%3E%3C/polyline%3E%3C/svg%3E");
    background-repeat: no-repeat;
    background-position: right 10px center;
  }

  .select-input:focus {
    outline: none;
    border-color: var(--accent-blue);
  }

  .select-input option {
    background: var(--bg-secondary);
    color: var(--fg-primary);
  }

  .shortcuts-list {
    display: flex;
    flex-direction: column;
    gap: 8px;
  }

  .shortcut-row {
    display: flex;
    align-items: center;
    justify-content: space-between;
    padding: 8px 0;
  }

  .shortcut-desc {
    font-size: 13px;
    color: var(--fg-secondary);
  }

  kbd {
    padding: 4px 8px;
    background: var(--bg-tertiary);
    border: 1px solid var(--border-color);
    border-radius: 4px;
    font-family: 'SF Mono', 'Fira Code', monospace;
    font-size: 11px;
    color: var(--fg-muted);
  }

  .shortcut-row.editable {
    padding: 8px 0;
  }

  .hotkey-input-wrapper {
    display: flex;
    align-items: center;
  }

  .hotkey-button {
    display: flex;
    align-items: center;
    gap: 8px;
    padding: 4px 8px;
    background: var(--bg-tertiary);
    border: 1px solid var(--border-color);
    border-radius: 4px;
    cursor: pointer;
    transition: all 0.15s;
  }

  .hotkey-button:hover:not(:disabled) {
    border-color: var(--accent-blue);
  }

  .hotkey-button:disabled {
    opacity: 0.6;
    cursor: not-allowed;
  }

  .hotkey-button kbd {
    border: none;
    padding: 0;
    background: transparent;
  }

  .hotkey-button .edit-icon {
    font-size: 10px;
    color: var(--fg-muted);
    opacity: 0;
    transition: opacity 0.15s;
  }

  .hotkey-button:hover .edit-icon {
    opacity: 1;
  }

  .hotkey-input {
    padding: 4px 8px;
    background: var(--bg-tertiary);
    border: 1px solid var(--accent-blue);
    border-radius: 4px;
    font-family: 'SF Mono', 'Fira Code', monospace;
    font-size: 11px;
    color: var(--fg-primary);
    outline: none;
    min-width: 120px;
  }

  .hotkey-input.recording {
    animation: pulse 1s infinite;
  }

  @keyframes pulse {
    0%, 100% { border-color: var(--accent-blue); }
    50% { border-color: var(--accent-purple); }
  }

  .hotkey-error {
    font-size: 12px;
    color: var(--accent-red, #f7768e);
    padding: 4px 0;
  }

  .modal-footer {
    display: flex;
    justify-content: flex-end;
    gap: 8px;
    padding: 16px 20px;
    background: var(--bg-secondary);
    border-top: 1px solid var(--border-color);
  }

  .btn {
    padding: 8px 16px;
    border-radius: 6px;
    font-size: 13px;
    font-weight: 500;
    transition: all 0.15s;
  }

  .btn:disabled {
    opacity: 0.6;
    cursor: not-allowed;
  }

  .btn-secondary {
    background: var(--bg-tertiary);
    color: var(--fg-secondary);
  }

  .btn-secondary:hover:not(:disabled) {
    background: var(--bg-highlight);
    color: var(--fg-primary);
  }

  .btn-primary {
    background: var(--accent-blue);
    color: #fff;
  }

  .btn-primary:hover:not(:disabled) {
    opacity: 0.9;
  }
</style>
