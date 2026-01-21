// Tauri API ラッパー
import { invoke } from '@tauri-apps/api/core';
import type { NoteDto, NoteListItemDto, SearchResultDto, Settings } from '$lib/types';

export async function createNote(): Promise<NoteDto> {
  return await invoke('create_note');
}

export async function saveNote(uid: string, content: string): Promise<void> {
  return await invoke('save_note', { uid, content });
}

export async function loadNote(uid: string): Promise<NoteDto> {
  return await invoke('load_note', { uid });
}

export async function deleteNote(uid: string): Promise<void> {
  return await invoke('delete_note', { uid });
}

export async function listNotes(): Promise<NoteListItemDto[]> {
  return await invoke('list_notes');
}

export async function searchNotes(query: string, limit?: number): Promise<SearchResultDto[]> {
  return await invoke('search_notes', { query, limit });
}

export async function getSettings(): Promise<Settings> {
  return await invoke('get_settings');
}

export interface SettingsUpdate {
  theme?: 'tokyo-night' | 'kanagawa' | 'monokai' | 'gruvbox' | 'dracula' | 'catppuccin' | 'synthwave';
  theme_mode?: 'light' | 'dark';
  font_family?: string;
  font_size?: number;
  line_height?: number;
  show_line_numbers?: boolean;
  autosave_enabled?: boolean;
  autosave_delay_ms?: number;
  restore_last_note?: boolean;
  storage_directory?: string;
  shortcut_new_note?: string;
  shortcut_toggle_sidebar?: string;
  shortcut_open_settings?: string;
}

export async function updateSettings(settings: SettingsUpdate): Promise<void> {
  return await invoke('update_settings', { settings });
}

export async function saveWindowGeometry(): Promise<void> {
  return await invoke('save_window_geometry');
}

export async function prepareHide(uid?: string, content?: string): Promise<void> {
  return await invoke('prepare_hide', { uid, content });
}

export async function setLastNoteUid(uid: string | null): Promise<void> {
  return await invoke('set_last_note_uid', { uid });
}

export async function updateHotkey(hotkey: string): Promise<void> {
  return await invoke('update_hotkey', { hotkey });
}

export async function getCurrentHotkey(): Promise<string> {
  return await invoke('get_current_hotkey');
}
