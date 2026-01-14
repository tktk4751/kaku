// 設定ストア (Svelte 5 runes)
import { getSettings, updateSettings, updateHotkey, getCurrentHotkey, type SettingsUpdate } from '$lib/services/api';
import type { Settings } from '$lib/types';

// デフォルト設定
const defaultSettings: Settings = {
  window: { x: 100, y: 100, width: 800, height: 600, is_maximized: false },
  storage_directory: '',
  editor: { font_family: 'system-ui', font_size: 14, line_height: 1.6, show_line_numbers: true },
  theme: 'tokyo-night',
  theme_mode: 'dark',
  hotkey: 'Ctrl+Shift+Space',
  shortcuts: { new_note: 'Ctrl+N', toggle_sidebar: 'Ctrl+M', open_settings: 'Ctrl+,' },
  autosave: { enabled: true, delay_ms: 2000 },
  restore_last_note: false,
  last_note_uid: null,
};

let settings = $state<Settings>(defaultSettings);
let isLoaded = $state(false);
let isSaving = $state(false);

export function useSettingsStore() {
  return {
    get settings() { return settings; },
    get isLoaded() { return isLoaded; },
    get isSaving() { return isSaving; },

    async load() {
      try {
        settings = await getSettings();
        isLoaded = true;
      } catch (e) {
        console.error('Failed to load settings:', e);
        settings = defaultSettings;
        isLoaded = true;
      }
    },

    async setTheme(theme: Settings['theme']) {
      if (settings.theme === theme) return;

      const previousTheme = settings.theme;
      settings = { ...settings, theme };

      try {
        isSaving = true;
        await updateSettings({ theme });
      } catch (e) {
        console.error('Failed to save theme:', e);
        settings = { ...settings, theme: previousTheme };
      } finally {
        isSaving = false;
      }
    },

    async setThemeMode(mode: Settings['theme_mode']) {
      if (settings.theme_mode === mode) return;

      const previousMode = settings.theme_mode;
      settings = { ...settings, theme_mode: mode };

      try {
        isSaving = true;
        await updateSettings({ theme_mode: mode });
      } catch (e) {
        console.error('Failed to save theme mode:', e);
        settings = { ...settings, theme_mode: previousMode };
      } finally {
        isSaving = false;
      }
    },

    async setEditorSettings(editorUpdate: Partial<Settings['editor']>) {
      const previousEditor = settings.editor;
      settings = {
        ...settings,
        editor: { ...settings.editor, ...editorUpdate },
      };

      const update: SettingsUpdate = {};
      if (editorUpdate.font_family !== undefined) {
        update.font_family = editorUpdate.font_family;
      }
      if (editorUpdate.font_size !== undefined) {
        update.font_size = editorUpdate.font_size;
      }
      if (editorUpdate.line_height !== undefined) {
        update.line_height = editorUpdate.line_height;
      }
      if (editorUpdate.show_line_numbers !== undefined) {
        update.show_line_numbers = editorUpdate.show_line_numbers;
      }

      try {
        isSaving = true;
        await updateSettings(update);
      } catch (e) {
        console.error('Failed to save editor settings:', e);
        settings = { ...settings, editor: previousEditor };
      } finally {
        isSaving = false;
      }
    },

    async setAutosave(enabled: boolean, delayMs?: number) {
      const previousAutosave = settings.autosave;
      settings = {
        ...settings,
        autosave: {
          enabled,
          delay_ms: delayMs ?? settings.autosave.delay_ms,
        },
      };

      try {
        isSaving = true;
        await updateSettings({
          autosave_enabled: enabled,
          autosave_delay_ms: delayMs,
        });
      } catch (e) {
        console.error('Failed to save autosave settings:', e);
        settings = { ...settings, autosave: previousAutosave };
      } finally {
        isSaving = false;
      }
    },

    async setStorageDirectory(directory: string) {
      const previousDirectory = settings.storage_directory;
      settings = { ...settings, storage_directory: directory };

      try {
        isSaving = true;
        await updateSettings({ storage_directory: directory });
      } catch (e) {
        console.error('Failed to save storage directory:', e);
        settings = { ...settings, storage_directory: previousDirectory };
      } finally {
        isSaving = false;
      }
    },

    async setRestoreLastNote(restore: boolean) {
      const previous = settings.restore_last_note;
      settings = { ...settings, restore_last_note: restore };

      try {
        isSaving = true;
        await updateSettings({ restore_last_note: restore });
      } catch (e) {
        console.error('Failed to save restore_last_note setting:', e);
        settings = { ...settings, restore_last_note: previous };
      } finally {
        isSaving = false;
      }
    },

    getEditorStyles() {
      return {
        fontFamily: settings.editor.font_family,
        fontSize: `${settings.editor.font_size}px`,
        lineHeight: settings.editor.line_height,
      };
    },

    async setHotkey(hotkey: string) {
      const previous = settings.hotkey;
      settings = { ...settings, hotkey };

      try {
        isSaving = true;
        await updateHotkey(hotkey);
      } catch (e) {
        console.error('Failed to save hotkey:', e);
        settings = { ...settings, hotkey: previous };
        throw e;
      } finally {
        isSaving = false;
      }
    },

    async refreshHotkey() {
      try {
        const currentHotkey = await getCurrentHotkey();
        settings = { ...settings, hotkey: currentHotkey };
      } catch (e) {
        console.error('Failed to get current hotkey:', e);
      }
    },

    async setShortcuts(shortcuts: Partial<Settings['shortcuts']>) {
      const previousShortcuts = settings.shortcuts;
      settings = {
        ...settings,
        shortcuts: { ...settings.shortcuts, ...shortcuts },
      };

      const update: SettingsUpdate = {};
      if (shortcuts.new_note !== undefined) {
        update.shortcut_new_note = shortcuts.new_note;
      }
      if (shortcuts.toggle_sidebar !== undefined) {
        update.shortcut_toggle_sidebar = shortcuts.toggle_sidebar;
      }
      if (shortcuts.open_settings !== undefined) {
        update.shortcut_open_settings = shortcuts.open_settings;
      }

      try {
        isSaving = true;
        await updateSettings(update);
      } catch (e) {
        console.error('Failed to save shortcuts:', e);
        settings = { ...settings, shortcuts: previousShortcuts };
      } finally {
        isSaving = false;
      }
    },
  };
}

// シングルトンインスタンス
export const settingsStore = useSettingsStore();
