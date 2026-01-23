// Tauri API ラッパー
//
// This module provides type-safe wrappers around Tauri IPC commands.
// All functions that can fail return Result<T, AppError> for consistent error handling.

import { invoke } from '@tauri-apps/api/core';
import type {
  NoteDto,
  NoteListItemDto,
  SearchResultDto,
  BacklinkDto,
  Settings,
  Result,
  AppError,
  NoteGalleryItemDto,
  GallerySortOrder,
  NoteTagsDto,
} from '$lib/types';
import { ok, err, parseAppError } from '$lib/types';

// ===== Safe API Wrapper =====

/**
 * Wraps a Tauri invoke call with Result-based error handling.
 * Converts unknown errors into structured AppError.
 */
async function safeInvoke<T>(cmd: string, args?: Record<string, unknown>): Promise<Result<T, AppError>> {
  try {
    const value = await invoke<T>(cmd, args);
    return ok(value);
  } catch (e) {
    return err(parseAppError(e));
  }
}

// ===== Note Operations =====

/** Create a new note in memory (not saved to disk yet) */
export async function createNote(): Promise<NoteDto> {
  return await invoke('create_note');
}

/** Create a new note with Result-based error handling */
export async function createNoteSafe(): Promise<Result<NoteDto, AppError>> {
  return safeInvoke('create_note');
}

/** Save note content to disk */
export async function saveNote(uid: string, content: string): Promise<void> {
  return await invoke('save_note', { uid, content });
}

/** Save note with Result-based error handling */
export async function saveNoteSafe(uid: string, content: string): Promise<Result<void, AppError>> {
  return safeInvoke('save_note', { uid, content });
}

/** Load a note by UID */
export async function loadNote(uid: string): Promise<NoteDto> {
  return await invoke('load_note', { uid });
}

/** Load note with Result-based error handling */
export async function loadNoteSafe(uid: string): Promise<Result<NoteDto, AppError>> {
  return safeInvoke('load_note', { uid });
}

/** Delete a note by UID */
export async function deleteNote(uid: string): Promise<void> {
  return await invoke('delete_note', { uid });
}

/** Delete note with Result-based error handling */
export async function deleteNoteSafe(uid: string): Promise<Result<void, AppError>> {
  return safeInvoke('delete_note', { uid });
}

/** List all notes (sorted by updated_at descending) */
export async function listNotes(): Promise<NoteListItemDto[]> {
  return await invoke('list_notes');
}

/** List notes with Result-based error handling */
export async function listNotesSafe(): Promise<Result<NoteListItemDto[], AppError>> {
  return safeInvoke('list_notes');
}

// ===== Search =====

/** Search notes with fuzzy matching */
export async function searchNotes(query: string, limit?: number): Promise<SearchResultDto[]> {
  return await invoke('search_notes', { query, limit });
}

/** Search with Result-based error handling */
export async function searchNotesSafe(query: string, limit?: number): Promise<Result<SearchResultDto[], AppError>> {
  return safeInvoke('search_notes', { query, limit });
}

/** Resolve wiki link - find note by title or create new */
export async function resolveWikiLink(title: string): Promise<NoteDto> {
  return await invoke('resolve_wiki_link', { title });
}

/** Resolve wiki link with Result-based error handling */
export async function resolveWikiLinkSafe(title: string): Promise<Result<NoteDto, AppError>> {
  return safeInvoke('resolve_wiki_link', { title });
}

// ===== Backlinks =====

/** Get backlinks for a note by UID */
export async function getBacklinks(uid: string): Promise<BacklinkDto[]> {
  return await invoke('get_backlinks', { uid });
}

/** Get backlinks with Result-based error handling */
export async function getBacklinksSafe(uid: string): Promise<Result<BacklinkDto[], AppError>> {
  return safeInvoke('get_backlinks', { uid });
}

/** Rebuild backlink index */
export async function rebuildBacklinkIndex(): Promise<void> {
  return await invoke('rebuild_backlink_index');
}

// ===== Settings =====

/** Get current settings */
export async function getSettings(): Promise<Settings> {
  return await invoke('get_settings');
}

/** Get settings with Result-based error handling */
export async function getSettingsSafe(): Promise<Result<Settings, AppError>> {
  return safeInvoke('get_settings');
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
  // Shortcuts
  shortcut_new_note?: string;
  shortcut_toggle_sidebar?: string;
  shortcut_open_settings?: string;
  shortcut_command_palette?: string;
  shortcut_history_back?: string;
  shortcut_history_forward?: string;
  shortcut_save_note?: string;
  shortcut_find_in_note?: string;
  shortcut_backlink_panel?: string;
}

/** Update settings */
export async function updateSettings(settings: SettingsUpdate): Promise<void> {
  return await invoke('update_settings', { settings });
}

/** Update settings with Result-based error handling */
export async function updateSettingsSafe(settings: SettingsUpdate): Promise<Result<void, AppError>> {
  return safeInvoke('update_settings', { settings });
}

// ===== Window Management =====

/** Save current window geometry to settings */
export async function saveWindowGeometry(): Promise<void> {
  return await invoke('save_window_geometry');
}

/** Prepare for window hide (save note, delete if empty) */
export async function prepareHide(uid?: string, content?: string): Promise<void> {
  return await invoke('prepare_hide', { uid, content });
}

/** Set the last opened note UID for restoration */
export async function setLastNoteUid(uid: string | null): Promise<void> {
  return await invoke('set_last_note_uid', { uid });
}

// ===== Hotkey =====

/** Update the global hotkey */
export async function updateHotkey(hotkey: string): Promise<void> {
  return await invoke('update_hotkey', { hotkey });
}

/** Update hotkey with Result-based error handling */
export async function updateHotkeySafe(hotkey: string): Promise<Result<void, AppError>> {
  return safeInvoke('update_hotkey', { hotkey });
}

/** Get the current global hotkey */
export async function getCurrentHotkey(): Promise<string> {
  return await invoke('get_current_hotkey');
}

// ===== Gallery =====

/** List notes for gallery view */
export async function listNotesGallery(
  sortOrder?: GallerySortOrder,
  tagFilter?: string
): Promise<NoteGalleryItemDto[]> {
  return await invoke('list_notes_gallery', {
    sortOrder: sortOrder ?? 'updated_at',
    tagFilter,
  });
}

/** List notes for gallery with Result-based error handling */
export async function listNotesGallerySafe(
  sortOrder?: GallerySortOrder,
  tagFilter?: string
): Promise<Result<NoteGalleryItemDto[], AppError>> {
  return safeInvoke('list_notes_gallery', {
    sortOrder: sortOrder ?? 'updated_at',
    tagFilter,
  });
}

// ===== Tags =====

/** Get all tags across all notes */
export async function getAllTags(): Promise<string[]> {
  return await invoke('get_all_tags');
}

/** Get all tags with Result-based error handling */
export async function getAllTagsSafe(): Promise<Result<string[], AppError>> {
  return safeInvoke('get_all_tags');
}

/** Get tags for a specific note */
export async function getNoteTags(uid: string): Promise<NoteTagsDto> {
  return await invoke('get_note_tags', { uid });
}

/** Get note tags with Result-based error handling */
export async function getNoteTagsSafe(uid: string): Promise<Result<NoteTagsDto, AppError>> {
  return safeInvoke('get_note_tags', { uid });
}

/** Update tags for a note */
export async function updateNoteTags(uid: string, tags: string[]): Promise<void> {
  return await invoke('update_note_tags', { uid, tags });
}

/** Update note tags with Result-based error handling */
export async function updateNoteTagsSafe(uid: string, tags: string[]): Promise<Result<void, AppError>> {
  return safeInvoke('update_note_tags', { uid, tags });
}
