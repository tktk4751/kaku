// フロントエンド型定義

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
  new_note: string;
  toggle_sidebar: string;
  open_settings: string;
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
