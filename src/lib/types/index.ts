// フロントエンド型定義

// ===== Error Handling =====

/** Error codes matching Rust backend error types */
export type AppErrorCode =
  | 'NOT_FOUND'
  | 'SAVE_FAILED'
  | 'LOAD_FAILED'
  | 'DELETE_FAILED'
  | 'VALIDATION_ERROR'
  | 'PARSE_ERROR'
  | 'IO_ERROR'
  | 'SETTINGS_ERROR'
  | 'SEARCH_ERROR'
  | 'UNKNOWN';

/** Structured error type for consistent error handling */
export interface AppError {
  code: AppErrorCode;
  message: string;
  details?: string;
}

/** Result type for operations that can fail */
export type Result<T, E = AppError> =
  | { ok: true; value: T }
  | { ok: false; error: E };

/** Helper to create success result */
export function ok<T>(value: T): Result<T, never> {
  return { ok: true, value };
}

/** Helper to create error result */
export function err<E>(error: E): Result<never, E> {
  return { ok: false, error };
}

/** Parse error from Tauri invoke into AppError */
export function parseAppError(e: unknown): AppError {
  const message = e instanceof Error ? e.message : String(e);

  // Match common error patterns from Rust backend
  if (message.includes('not found') || message.includes('NotFound')) {
    return { code: 'NOT_FOUND', message };
  }
  if (message.includes('Invalid UID') || message.includes('too large') || message.includes('too long')) {
    return { code: 'VALIDATION_ERROR', message };
  }
  if (message.includes('parse') || message.includes('Parse')) {
    return { code: 'PARSE_ERROR', message };
  }
  if (message.includes('save') || message.includes('Save')) {
    return { code: 'SAVE_FAILED', message };
  }
  if (message.includes('load') || message.includes('Load')) {
    return { code: 'LOAD_FAILED', message };
  }
  if (message.includes('delete') || message.includes('Delete')) {
    return { code: 'DELETE_FAILED', message };
  }
  if (message.includes('search') || message.includes('Search')) {
    return { code: 'SEARCH_ERROR', message };
  }
  if (message.includes('settings') || message.includes('Settings')) {
    return { code: 'SETTINGS_ERROR', message };
  }
  if (message.includes('I/O') || message.includes('io::')) {
    return { code: 'IO_ERROR', message };
  }

  return { code: 'UNKNOWN', message };
}

// ===== Data Transfer Objects =====

export interface NoteDto {
  uid: string;
  content: string;
  created_at: string;
  updated_at: string;
  is_dirty: boolean;
}

export interface NoteListItemDto {
  uid: string;
  title: string;
  updated_at: string;
}

// 検索関連
export interface SearchResultDto {
  uid: string;
  title: string;
  score: number;
  title_matches: MatchRange[];
  content_preview: ContentPreview | null;
}

export interface MatchRange {
  start: number;
  end: number;
}

export interface ContentPreview {
  text: string;
  match_start: number;
  match_end: number;
}

export interface WindowGeometry {
  x: number;
  y: number;
  width: number;
  height: number;
  is_maximized: boolean;
}

export interface EditorSettings {
  font_family: string;
  font_size: number;
  line_height: number;
  show_line_numbers: boolean;
}

export interface AutosaveSettings {
  enabled: boolean;
  delay_ms: number;
}

export interface ShortcutSettings {
  // Page-level shortcuts
  new_note: string;
  toggle_sidebar: string;
  open_settings: string;
  command_palette: string;
  history_back: string;
  history_forward: string;
  // Editor shortcuts
  save_note: string;
  find_in_note: string;
  backlink_panel: string;
}

export type ThemeName = 'tokyo-night' | 'kanagawa' | 'monokai' | 'gruvbox' | 'dracula' | 'catppuccin' | 'synthwave';
export type ThemeMode = 'light' | 'dark';

export interface Settings {
  window: WindowGeometry;
  storage_directory: string;
  editor: EditorSettings;
  theme: ThemeName;
  theme_mode: ThemeMode;
  hotkey: string;
  shortcuts: ShortcutSettings;
  autosave: AutosaveSettings;
  restore_last_note: boolean;
  last_note_uid: string | null;
}

// バックリンク関連
export interface BacklinkDto {
  uid: string;
  title: string;
  context: string;
}
