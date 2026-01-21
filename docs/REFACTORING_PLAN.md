# z_memo リファクタリング計画書
## SOLID原則とデザインパターンによるクリーンアーキテクチャ化

---

## ⚠️ レビュー結果: 発見された問題と修正

### 🔴 重大な問題 (挙動変更を引き起こす)

| # | 箇所 | 問題 | 影響 |
|---|------|------|------|
| 1 | Phase 2.1 Store分離 | `updateContent()` から autosave スケジューリングが消失 | **autosave が動かなくなる** |
| 2 | Phase 2.2 API層 | 関数シグネチャ変更 (`saveNote(uid, content)` → `saveNote({uid, content})`) | **全呼び出し元がコンパイルエラー** |
| 3 | Phase 1.3 PlatformManager | `show_window()` で `mark_window_visible()` 未呼び出し | **ウィンドウ可視状態の追跡が壊れる** |
| 4 | Phase 1.3 PlatformManager | `hide_window()` でジオメトリ保存なし | **ウィンドウ位置が保存されない** |
| 5 | Phase 1.3 PlatformManager | `create-new-note` イベント発火なし | **ホットキーで新規ノートが作成されない** |

### 🟡 中程度の問題 (移行漏れ)

| # | 箇所 | 問題 |
|---|------|------|
| 6 | 計画書全体 | コンポーネントの import 変更が記載されていない |
| 7 | Phase 2.1 | `Editor.svelte` の `noteStore.updateContent()` 呼び出し変更が未記載 |
| 8 | Phase 2.1 | `+page.svelte` の全 `noteStore.xxx()` 呼び出し変更が未記載 |

---

## 現状分析サマリー

### 良い点 ✓
- **ドメイン層が明確**: `domain/` に純粋なエンティティ
- **トレイトによる抽象化**: `Storage`, `NoteRepository`, `EventBus`, `FilenameStrategy`
- **依存性逆転**: バックエンドは `Arc<dyn Trait>` で注入
- **イベント駆動**: `DomainEvent` + `EventBus` パターン
- **アトミック書き込み**: データ安全性を確保

### 改善点 ⚠
- **AppState がモノリシック**: 全サービスを1つの構造体で保持
- **コマンドハンドラが1ファイル**: 337行、14コマンドが混在
- **Settings Service が多責務**: 永続化 + 状態管理 + イベント発火
- **フロントエンド Store が複合的**: データ + アクション + 副作用
- **lib.rs に setup ロジックが集中**: 358行

---

## Phase 1: バックエンド構造改善（基盤整備）

### 1.1 コマンドハンドラのモジュール分割

**現状**: `commands/mod.rs` (337行) に全コマンド

**目標構造**:
```
src-tauri/src/commands/
├── mod.rs              # re-export + DTO定義
├── note.rs             # create, save, load, delete, list (5)
├── settings.rs         # get, update (2)
├── window.rs           # geometry, hide, maximize (4)
└── hotkey.rs           # update, get_current (2)
```

**適用パターン**:
- **Single Responsibility**: 各モジュールは1つの関心事
- **Facade**: `mod.rs` が公開インターフェース

**実装**:

```rust
// commands/mod.rs
mod note;
mod settings;
mod window;
mod hotkey;

// DTOs remain here (shared)
pub use note::*;
pub use settings::*;
pub use window::*;
pub use hotkey::*;

// DTOs
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NoteDto { ... }

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NoteListItemDto { ... }

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SettingsUpdateDto { ... }
```

```rust
// commands/note.rs
use super::{NoteDto, NoteListItemDto};
use crate::AppState;
use tauri::State;

#[tauri::command]
pub fn create_note(state: State<AppState>) -> Result<NoteDto, String> { ... }

#[tauri::command]
pub fn save_note(state: State<AppState>, uid: String, content: String) -> Result<(), String> { ... }

#[tauri::command]
pub fn load_note(state: State<AppState>, uid: String) -> Result<NoteDto, String> { ... }

#[tauri::command]
pub fn delete_note(state: State<AppState>, uid: String) -> Result<(), String> { ... }

#[tauri::command]
pub fn list_notes(state: State<AppState>) -> Result<Vec<NoteListItemDto>, String> { ... }
```

```rust
// commands/settings.rs
use crate::AppState;
use super::SettingsUpdateDto;
use tauri::State;

#[tauri::command]
pub fn get_settings(state: State<AppState>) -> crate::domain::Settings { ... }

#[tauri::command]
pub fn update_settings(state: State<AppState>, settings: SettingsUpdateDto) -> Result<(), String> { ... }
```

```rust
// commands/window.rs
use crate::AppState;
use tauri::{AppHandle, Manager, State};

#[tauri::command]
pub fn save_window_geometry(app: AppHandle, state: State<AppState>) -> Result<(), String> { ... }

#[tauri::command]
pub async fn prepare_hide(app: AppHandle, state: State<'_, AppState>, uid: Option<String>, content: Option<String>) -> Result<(), String> { ... }

#[tauri::command]
pub fn set_last_note_uid(state: State<AppState>, uid: Option<String>) -> Result<(), String> { ... }

#[tauri::command]
pub fn quit_app(app: AppHandle) { ... }

#[tauri::command]
pub fn hide_window(app: AppHandle, state: State<AppState>) -> Result<(), String> { ... }

#[tauri::command]
pub fn toggle_maximize(app: AppHandle) -> Result<(), String> { ... }
```

```rust
// commands/hotkey.rs
use crate::AppState;
use tauri::State;

#[tauri::command]
pub fn update_hotkey(state: State<AppState>, hotkey: String) -> Result<(), String> { ... }

#[tauri::command]
pub fn get_current_hotkey(state: State<AppState>) -> String { ... }
```

---

### 1.2 Settings Repository パターンの導入

**現状**: `SettingsService` が永続化と状態管理を両方担当

**目標**: Repository Pattern で永続化を分離

**適用原則**:
- **Single Responsibility**: Repository = 永続化, Service = ビジネスロジック
- **Dependency Inversion**: Service は Repository トレイトに依存

**実装**:

```rust
// traits/settings_repository.rs
use crate::domain::{Settings, SettingsError};

pub trait SettingsRepository: Send + Sync {
    fn load(&self) -> Result<Settings, SettingsError>;
    fn save(&self, settings: &Settings) -> Result<(), SettingsError>;
}
```

```rust
// infrastructure/file_settings_repository.rs
use crate::domain::{Settings, SettingsError};
use crate::traits::SettingsRepository;
use std::path::PathBuf;

pub struct FileSettingsRepository {
    config_path: PathBuf,
}

impl FileSettingsRepository {
    pub fn new() -> Self {
        Self {
            config_path: Settings::config_path(),
        }
    }
}

impl SettingsRepository for FileSettingsRepository {
    fn load(&self) -> Result<Settings, SettingsError> {
        Settings::load_from_file(&self.config_path)
    }

    fn save(&self, settings: &Settings) -> Result<(), SettingsError> {
        settings.save_to_file(&self.config_path)
    }
}
```

```rust
// services/settings_service.rs (リファクタリング後)
use crate::domain::{DomainEvent, Settings, SettingsError, WindowGeometry};
use crate::traits::{EventBus, SettingsRepository};
use parking_lot::RwLock;
use std::sync::Arc;

pub struct SettingsService {
    repository: Arc<dyn SettingsRepository>,
    settings: RwLock<Settings>,
    event_bus: Arc<dyn EventBus>,
}

impl SettingsService {
    pub fn new(
        repository: Arc<dyn SettingsRepository>,
        event_bus: Arc<dyn EventBus>,
    ) -> Self {
        let settings = repository.load().unwrap_or_default();
        Self {
            repository,
            settings: RwLock::new(settings),
            event_bus,
        }
    }

    pub fn get(&self) -> Settings {
        self.settings.read().clone()
    }

    pub fn update<F>(&self, f: F) -> Result<(), SettingsError>
    where
        F: FnOnce(&mut Settings),
    {
        {
            let mut settings = self.settings.write();
            f(&mut settings);
            self.repository.save(&settings)?;
        }
        self.event_bus.emit(DomainEvent::SettingsChanged);
        Ok(())
    }

    // ... other methods unchanged
}
```

---

### 1.3 Platform Manager Facade

**現状**: `platform/` の各モジュールが独立、`lib.rs` で直接呼び出し

**目標**: Facade Pattern で統一インターフェース

**適用パターン**:
- **Facade**: 複雑なサブシステムに統一インターフェース
- **Strategy**: プラットフォーム固有の実装を切り替え

> ⚠️ **修正済み**: 以下の問題を修正
> - `mark_window_visible()` / `mark_window_hidden()` の呼び出し追加
> - ジオメトリ保存機能を分離（呼び出し元が制御）
> - 注意: `create-new-note` イベントは Facade ではなく呼び出し元（`lib.rs`）で発火

**実装**:

```rust
// platform/manager.rs
use super::{WindowManager, hyprland, mark_window_hidden, mark_window_visible, is_window_visible};
use crate::domain::WindowGeometry;

/// プラットフォーム操作の統一インターフェース
///
/// 注意: このFacadeは純粋なウィンドウ操作のみを担当。
/// イベント発火（create-new-note等）は呼び出し元が行う。
pub struct PlatformManager;

impl PlatformManager {
    /// ウィンドウを表示（可視状態を追跡）
    pub fn show_window<R: tauri::Runtime>(window: &tauri::WebviewWindow<R>) -> Result<(), String> {
        #[cfg(target_os = "linux")]
        {
            if hyprland::is_hyprland() {
                let _ = window.show();
                std::thread::sleep(std::time::Duration::from_millis(50));
                mark_window_visible();  // ← 修正: 可視状態を追跡
                return Ok(());
            }
        }
        window.show().map_err(|e| e.to_string())?;
        mark_window_visible();  // ← 修正: 可視状態を追跡
        Ok(())
    }

    /// ウィンドウを非表示（可視状態を追跡）
    /// 注意: ジオメトリ保存は呼び出し元が事前に行うこと
    pub fn hide_window<R: tauri::Runtime>(window: &tauri::WebviewWindow<R>) -> Result<(), String> {
        #[cfg(target_os = "linux")]
        {
            if hyprland::is_hyprland() {
                hyprland::move_offscreen("kaku");
                mark_window_hidden();
                return Ok(());
            }
        }
        window.hide().map_err(|e| e.to_string())?;
        mark_window_hidden();
        Ok(())
    }

    /// ウィンドウジオメトリを取得（プラットフォーム対応）
    pub fn get_geometry<R: tauri::Runtime>(window: &tauri::WebviewWindow<R>) -> Result<WindowGeometry, String> {
        #[cfg(target_os = "linux")]
        {
            if hyprland::is_hyprland() {
                let mut geometry = WindowManager::get_geometry(window)
                    .map_err(|e| e.to_string())?;
                if let Some((x, y)) = hyprland::get_window_position("kaku") {
                    // オフスクリーン位置は無視
                    if x >= -5000 && y >= -5000 {
                        geometry.x = x;
                        geometry.y = y;
                    }
                }
                return Ok(geometry);
            }
        }
        WindowManager::get_geometry(window).map_err(|e| e.to_string())
    }

    /// ウィンドウ位置を設定
    pub fn set_position(x: i32, y: i32) {
        #[cfg(target_os = "linux")]
        {
            if hyprland::is_hyprland() {
                hyprland::set_window_position("kaku", x, y);
                return;
            }
        }
        // X11/Windows/macOS: Tauri handles this
    }

    /// 現在ウィンドウが可視かどうか
    pub fn is_visible() -> bool {
        is_window_visible()
    }

    /// デフォルトウィンドウ位置を計算
    #[cfg(target_os = "linux")]
    pub fn calculate_default_position(width: u32, height: u32) -> (i32, i32) {
        hyprland::calculate_default_position(width, height)
            .unwrap_or((100, 50))
    }

    #[cfg(not(target_os = "linux"))]
    pub fn calculate_default_position(_width: u32, _height: u32) -> (i32, i32) {
        (100, 50)
    }
}
```

**呼び出し例（lib.rs toggle_window_from_ipc）**:
```rust
// ジオメトリ保存 → 非表示の順序を維持
if PlatformManager::is_visible() {
    // 1. ジオメトリを保存（呼び出し元の責務）
    if let Ok(geometry) = PlatformManager::get_geometry(&window) {
        let _ = state.settings_service.update_window_geometry(geometry);
    }
    // 2. ウィンドウを非表示
    PlatformManager::hide_window(&window)?;
} else {
    // 1. ウィンドウを表示
    PlatformManager::show_window(&window)?;
    // 2. 位置を復元（呼び出し元の責務）
    PlatformManager::set_position(x, y);
    // 3. イベント発火（呼び出し元の責務）
    let _ = window.emit("create-new-note", ());
}
```

---

### 1.4 統一エラー型

**現状**: 各モジュールが独自エラー型、コマンドで `.map_err(|e| e.to_string())`

**目標**: 階層的エラー型で変換を一元化

**適用パターン**:
- **Layered Error Handling**: 各層が自身のエラー型を持つ
- **Error Translation**: コマンド層で統一的に変換

**実装**:

```rust
// commands/error.rs
use crate::domain::{NoteParseError, SettingsError};
use crate::traits::{RepositoryError, StorageError};
use thiserror::Error;

#[derive(Debug, Error)]
pub enum CommandError {
    #[error("Note error: {0}")]
    Note(#[from] RepositoryError),

    #[error("Settings error: {0}")]
    Settings(#[from] SettingsError),

    #[error("Storage error: {0}")]
    Storage(#[from] StorageError),

    #[error("Window error: {0}")]
    Window(String),

    #[error("Not found: {0}")]
    NotFound(String),
}

impl From<CommandError> for String {
    fn from(e: CommandError) -> Self {
        e.to_string()
    }
}

// Usage in commands:
// #[tauri::command]
// pub fn create_note(state: State<AppState>) -> Result<NoteDto, CommandError> { ... }
```

---

### 1.5 lib.rs の分割

**現状**: `lib.rs` (358行) に AppState + setup ロジック

**目標**: 責務分離

**実装**:

```
src-tauri/src/
├── lib.rs              # pub mod declarations + run()
├── app_state.rs        # AppState struct + impl
└── setup/
    ├── mod.rs          # setup orchestration
    ├── window.rs       # window configuration
    └── platform.rs     # platform-specific setup
```

```rust
// app_state.rs
use crate::infrastructure::*;
use crate::services::*;
use std::sync::Arc;

pub struct AppState {
    pub note_service: NoteService,
    pub settings_service: Arc<SettingsService>,
    pub event_bus: Arc<EventBusImpl>,
}

impl AppState {
    pub fn new() -> Self {
        let event_bus = Arc::new(EventBusImpl::new());
        let settings_repository = Arc::new(FileSettingsRepository::new());
        let settings_service = Arc::new(SettingsService::new(
            settings_repository,
            event_bus.clone(),
        ));

        let storage = Arc::new(FileStorage::new());
        let filename_strategy = Arc::new(HeadingFilenameStrategy::new());
        let repository = Arc::new(FileNoteRepository::new(
            storage,
            filename_strategy,
            settings_service.clone(),
        ));

        let note_service = NoteService::new(repository, event_bus.clone());

        Self {
            note_service,
            settings_service,
            event_bus,
        }
    }
}
```

---

## Phase 2: フロントエンド構造改善

### 2.1 Store のデータ/アクション分離

**現状**: `note.svelte.ts` がデータ + アクション + 副作用を混在

**目標**: CQRS-lite パターンで分離

**適用パターン**:
- **CQRS (Command Query Responsibility Segregation)**: 読み取りと更新を分離
- **Single Responsibility**: データストアとアクションサービス

> ⚠️ **修正済み**: autosave スケジューリングの問題を修正
> - **問題**: 元の案では `updateContent()` から autosave が消失
> - **解決策A**: `updateContent()` に `scheduleAutosave` オプションを追加
> - **解決策B（採用）**: 既存 API を維持し、内部実装のみ分離

**実装（解決策B: 後方互換性を維持）**:

```typescript
// stores/note.svelte.ts - 既存のインターフェースを維持
import type { NoteDto, NoteListItemDto } from '$lib/types';
import { createNote, saveNote, loadNote, listNotes, deleteNote } from '$lib/services/api';

// ===== 内部データ層（外部非公開）=====
let currentNote = $state<NoteDto | null>(null);
let noteList = $state<NoteListItemDto[]>([]);
let isSaving = $state(false);
let isDirty = $state(false);
let saveError = $state<string | null>(null);
let autosaveTimer: ReturnType<typeof setTimeout> | null = null;

const AUTOSAVE_DELAY_MS = 50;

// ===== 内部データ操作（テスト用にエクスポート可能）=====
export const _internal = {
  setCurrentNote(note: NoteDto | null) { currentNote = note; },
  setNoteList(list: NoteListItemDto[]) { noteList = list; },
  setDirty(dirty: boolean) { isDirty = dirty; },
  setSaving(saving: boolean) { isSaving = saving; },
  setError(error: string | null) { saveError = error; },
};

// ===== 公開API（既存と完全互換）=====
export function useNoteStore() {
  return {
    // Getters（変更なし）
    get currentNote() { return currentNote; },
    get noteList() { return noteList; },
    get isSaving() { return isSaving; },
    get isDirty() { return isDirty; },
    get saveError() { return saveError; },

    // Actions（既存と同じシグネチャ）
    async createNew() {
      try {
        currentNote = await createNote();
        isDirty = false;
        saveError = null;
        await saveNote(currentNote.uid, currentNote.content);
        await this.refreshList();
      } catch (e) {
        saveError = String(e);
        throw e;
      }
    },

    async load(uid: string) {
      try {
        currentNote = await loadNote(uid);
        isDirty = false;
        saveError = null;
      } catch (e) {
        saveError = String(e);
        throw e;
      }
    },

    async save() {
      if (!currentNote || isSaving) return;

      isSaving = true;
      saveError = null;

      try {
        await saveNote(currentNote.uid, currentNote.content);
        isDirty = false;
        await this.refreshList();
      } catch (e) {
        saveError = String(e);
        throw e;
      } finally {
        isSaving = false;
      }
    },

    async deleteIfEmpty() {
      if (!currentNote) return false;

      const contentIsEmpty = !currentNote.content.trim();
      if (contentIsEmpty) {
        try {
          await deleteNote(currentNote.uid);
          await this.refreshList();
          return true;
        } catch (e) {
          console.log('Note deletion skipped:', e);
        }
      }
      return false;
    },

    // ⚠️ 重要: autosave スケジューリングを維持
    updateContent(content: string) {
      if (!currentNote) return;
      if (currentNote.content !== content) {
        currentNote = { ...currentNote, content };
        isDirty = true;
        this.scheduleAutosave();  // ← 既存の挙動を維持
      }
    },

    scheduleAutosave() {
      if (autosaveTimer) {
        clearTimeout(autosaveTimer);
      }
      autosaveTimer = setTimeout(() => {
        this.save().catch(console.error);
      }, AUTOSAVE_DELAY_MS);
    },

    cancelAutosave() {
      if (autosaveTimer) {
        clearTimeout(autosaveTimer);
        autosaveTimer = null;
      }
    },

    async refreshList() {
      try {
        noteList = await listNotes();
      } catch (e) {
        console.error('Failed to refresh note list:', e);
      }
    },

    async delete(uid: string) {
      try {
        await deleteNote(uid);

        if (currentNote?.uid === uid) {
          await this.refreshList();
          if (noteList.length > 0) {
            await this.load(noteList[0].uid);
          } else {
            await this.createNew();
          }
        } else {
          await this.refreshList();
        }

        saveError = null;
      } catch (e) {
        saveError = String(e);
        throw e;
      }
    },
  };
}

export const noteStore = useNoteStore();
```

**変更点のポイント**:
1. **外部インターフェース変更なし**: `noteStore.xxx()` の呼び出しは全て既存のまま
2. **内部分離**: `_internal` オブジェクトでデータ操作を分離（テスト時に有用）
3. **autosave 維持**: `updateContent()` は引き続き `scheduleAutosave()` を呼ぶ
4. **コンポーネント変更不要**: `Editor.svelte`, `+page.svelte` はそのまま動作

<details>
<summary>❌ 廃止された提案（参考用）: noteData + noteActions 分離</summary>

以下の案は **autosave 消失問題** により採用されませんでした。
`Editor.svelte` が `noteStore.updateContent(content)` のみを呼び出しており、
分離すると `scheduleAutosave()` が呼ばれなくなるためです。

```typescript
// ❌ 廃止: この分離は挙動変更を引き起こす
// noteData.updateContent() + noteActions.scheduleAutosave() を
// 全ての呼び出し元で明示的に呼ぶ必要があり、変更漏れのリスクが高い
```

</details>

---

### 2.2 API Layer の型安全性強化

**現状**: `api.ts` が薄い関数ラッパー

**目標**: Command/Query パターンで型安全性向上

**適用パターン**:
- **Command Pattern**: 操作をオブジェクトとしてカプセル化
- **Type Safety**: Request/Response を明示的に型付け

> ⚠️ **修正済み**: 関数シグネチャの問題を修正
> - **問題**: 元の案では `saveNote(uid, content)` → `saveNote({uid, content})` に変更
> - **影響**: 全ての呼び出し元がコンパイルエラーになる
> - **解決策**: **既存の関数シグネチャを維持**し、内部で型安全にする

**実装（後方互換性を維持）**:

```typescript
// services/api/types.ts - Request/Response 型定義（内部使用）
export interface CreateNoteRequest {}
export interface SaveNoteRequest { uid: string; content: string; }
export interface LoadNoteRequest { uid: string; }
export interface DeleteNoteRequest { uid: string; }

// これらは内部ドキュメント用。外部APIは既存シグネチャを維持。
```

```typescript
// services/api/commands/note.ts
import { invoke } from '@tauri-apps/api/core';
import type { NoteDto, NoteListItemDto } from '$lib/types';

// ⚠️ 既存シグネチャを維持（挙動変更なし）
export async function createNote(): Promise<NoteDto> {
  return invoke<NoteDto>('create_note');
}

// ⚠️ 既存シグネチャを維持: saveNote(uid, content) ← オブジェクトではない
export async function saveNote(uid: string, content: string): Promise<void> {
  return invoke('save_note', { uid, content });
}

// ⚠️ 既存シグネチャを維持: loadNote(uid) ← オブジェクトではない
export async function loadNote(uid: string): Promise<NoteDto> {
  return invoke<NoteDto>('load_note', { uid });
}

// ⚠️ 既存シグネチャを維持: deleteNote(uid) ← オブジェクトではない
export async function deleteNote(uid: string): Promise<void> {
  return invoke('delete_note', { uid });
}

export async function listNotes(): Promise<NoteListItemDto[]> {
  return invoke<NoteListItemDto[]>('list_notes');
}
```

**変更点のポイント**:
1. **関数シグネチャ変更なし**: `saveNote(uid, content)` のまま
2. **型情報追加**: Request/Response 型は内部ドキュメントとして定義
3. **呼び出し元変更不要**: `note.svelte.ts`, `+page.svelte` はそのまま動作

```typescript
// services/api/commands/settings.ts
import { invoke } from '@tauri-apps/api/core';
import type { Settings, ThemeName, ThemeMode } from '$lib/types';

export interface GetSettingsRequest {}
export interface UpdateSettingsRequest {
  theme?: ThemeName;
  theme_mode?: ThemeMode;
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

export async function getSettings(): Promise<Settings> {
  return invoke<Settings>('get_settings');
}

export async function updateSettings(request: UpdateSettingsRequest): Promise<void> {
  return invoke('update_settings', { settings: request });
}
```

```typescript
// services/api/index.ts - 統一エクスポート
export * from './commands/note';
export * from './commands/settings';
export * from './commands/window';
export * from './commands/hotkey';
```

---

### 2.3 Editor Factory パターン

**現状**: `createEditor()` が設定、拡張機能、テーマを1つの関数で処理

**目標**: Builder Pattern で構成を分離

**適用パターン**:
- **Builder Pattern**: 複雑なオブジェクト構築を段階的に
- **Factory Method**: エディタインスタンス生成

**実装**:

```typescript
// editor/builders/ExtensionBuilder.ts
import type { Extension } from '@codemirror/state';
import { lineNumbers, highlightActiveLine, drawSelection } from '@codemirror/view';
import { syntaxHighlighting, defaultHighlightStyle, bracketMatching } from '@codemirror/language';

export interface ExtensionOptions {
  showLineNumbers: boolean;
}

export class ExtensionBuilder {
  build(options: ExtensionOptions): Extension[] {
    const extensions: Extension[] = [
      highlightActiveLine(),
      drawSelection(),
      bracketMatching(),
      syntaxHighlighting(defaultHighlightStyle, { fallback: true }),
    ];

    if (options.showLineNumbers) {
      extensions.unshift(lineNumbers());
    }

    return extensions;
  }
}
```

```typescript
// editor/builders/KeymapBuilder.ts
import type { Extension } from '@codemirror/state';
import { getKeymapExtensions, getMarkdownExtension } from '../extensions/keymaps';

export class KeymapBuilder {
  build(): Extension[] {
    return [
      getMarkdownExtension(),
      ...getKeymapExtensions(),
    ];
  }
}
```

```typescript
// editor/builders/ThemeBuilder.ts
import type { Extension } from '@codemirror/state';
import { EditorView } from '@codemirror/view';
import { tokyoNightTheme } from '../themes/tokyoNight';
import type { ThemeName } from '$lib/types';

export interface ThemeOptions {
  theme: ThemeName;
  fontSize: number;
  lineHeight: number;
}

export class ThemeBuilder {
  build(options: ThemeOptions): Extension[] {
    return [
      // Base theme (CSS variables)
      tokyoNightTheme,

      // Font settings
      EditorView.theme({
        '&': { fontSize: `${options.fontSize}px` },
        '.cm-content': { lineHeight: String(options.lineHeight) },
        '.cm-line': { lineHeight: String(options.lineHeight) },
      }),
    ];
  }
}
```

```typescript
// editor/EditorFactory.ts
import { EditorState, type Extension } from '@codemirror/state';
import { EditorView } from '@codemirror/view';
import { ExtensionBuilder, type ExtensionOptions } from './builders/ExtensionBuilder';
import { KeymapBuilder } from './builders/KeymapBuilder';
import { ThemeBuilder, type ThemeOptions } from './builders/ThemeBuilder';
import { livePreviewPlugin } from './extensions/livePreview';
import type { ThemeName } from '$lib/types';

export interface EditorConfig {
  parent: HTMLElement;
  doc?: string;
  onChange?: (content: string) => void;
  theme?: ThemeName;
  fontSize?: number;
  lineHeight?: number;
  showLineNumbers?: boolean;
}

export class EditorFactory {
  private extensionBuilder: ExtensionBuilder;
  private keymapBuilder: KeymapBuilder;
  private themeBuilder: ThemeBuilder;

  constructor() {
    this.extensionBuilder = new ExtensionBuilder();
    this.keymapBuilder = new KeymapBuilder();
    this.themeBuilder = new ThemeBuilder();
  }

  create(config: EditorConfig): EditorView {
    const {
      parent,
      doc = '',
      onChange,
      theme = 'tokyo-night',
      fontSize = 14,
      lineHeight = 1.6,
      showLineNumbers = true,
    } = config;

    const extensions = this.buildExtensions({
      showLineNumbers,
      theme,
      fontSize,
      lineHeight,
      onChange,
    });

    const state = EditorState.create({ doc, extensions });
    return new EditorView({ state, parent });
  }

  private buildExtensions(options: {
    showLineNumbers: boolean;
    theme: ThemeName;
    fontSize: number;
    lineHeight: number;
    onChange?: (content: string) => void;
  }): Extension[] {
    const extensions: Extension[] = [
      // Core features
      ...this.extensionBuilder.build({ showLineNumbers: options.showLineNumbers }),

      // Keymaps
      ...this.keymapBuilder.build(),

      // Live preview
      livePreviewPlugin(),

      // Theme
      ...this.themeBuilder.build({
        theme: options.theme,
        fontSize: options.fontSize,
        lineHeight: options.lineHeight,
      }),

      // Editor behavior
      EditorView.lineWrapping,
      EditorState.allowMultipleSelections.of(true),
      EditorView.editable.of(true),
    ];

    // Change listener
    if (options.onChange) {
      extensions.push(
        EditorView.updateListener.of((update) => {
          if (update.docChanged) {
            options.onChange!(update.state.doc.toString());
          }
        })
      );
    }

    return extensions;
  }
}

// Singleton instance
export const editorFactory = new EditorFactory();

// Convenience function (backward compatible)
export function createEditor(config: EditorConfig): EditorView {
  return editorFactory.create(config);
}
```

---

### 2.4 コンポーネントの Handler Object Pattern

**現状**: 複数のコールバックを個別 props として渡す

**目標**: 関連するコールバックをオブジェクトにまとめる

**適用パターン**:
- **Parameter Object**: 関連パラメータをグループ化
- **Interface Segregation**: 必要なハンドラのみ要求

**実装**:

```typescript
// types/handlers.ts
export interface SidebarHandlers {
  onSelect: (uid: string) => void;
  onDelete: (uid: string) => Promise<void>;
  onClose: () => void;
}

export interface EditorHandlers {
  onChange: (content: string) => void;
  onSave: () => Promise<void>;
}

export interface SettingsHandlers {
  onClose: () => void;
  onSave: (settings: SettingsUpdate) => Promise<void>;
}
```

```svelte
<!-- Sidebar.svelte -->
<script lang="ts">
  import type { SidebarHandlers } from '$lib/types/handlers';

  interface Props {
    notes: NoteListItemDto[];
    selectedUid: string | null;
    handlers: SidebarHandlers;
  }

  let { notes, selectedUid, handlers }: Props = $props();
</script>
```

---

## Phase 3: アーキテクチャ拡張（オプション）

### 3.1 Plugin Architecture

**目標**: 拡張機能を追加可能なプラグインシステム

```rust
// traits/plugin.rs
use crate::domain::{Note, Settings, DomainEvent};

pub trait Plugin: Send + Sync {
    fn name(&self) -> &str;

    // Lifecycle hooks
    fn on_app_start(&self) -> Result<(), Box<dyn std::error::Error>> { Ok(()) }
    fn on_app_stop(&self) -> Result<(), Box<dyn std::error::Error>> { Ok(()) }

    // Note hooks
    fn on_note_created(&self, _note: &Note) -> Result<(), Box<dyn std::error::Error>> { Ok(()) }
    fn on_note_saved(&self, _note: &Note) -> Result<(), Box<dyn std::error::Error>> { Ok(()) }
    fn on_note_deleted(&self, _uid: &str) -> Result<(), Box<dyn std::error::Error>> { Ok(()) }

    // Settings hooks
    fn on_settings_changed(&self, _settings: &Settings) -> Result<(), Box<dyn std::error::Error>> { Ok(()) }
}
```

```rust
// infrastructure/plugin_manager.rs
use crate::traits::Plugin;

pub struct PluginManager {
    plugins: Vec<Box<dyn Plugin>>,
}

impl PluginManager {
    pub fn new() -> Self {
        Self { plugins: Vec::new() }
    }

    pub fn register(&mut self, plugin: Box<dyn Plugin>) {
        println!("[Plugin] Registered: {}", plugin.name());
        self.plugins.push(plugin);
    }

    pub fn notify_note_saved(&self, note: &crate::domain::Note) {
        for plugin in &self.plugins {
            if let Err(e) = plugin.on_note_saved(note) {
                eprintln!("[Plugin] {} error on note_saved: {}", plugin.name(), e);
            }
        }
    }
    // ... other notification methods
}
```

---

### 3.2 Unit of Work Pattern (将来の拡張)

複数の操作をトランザクション的に実行する場合に有用。

```rust
// traits/unit_of_work.rs
pub trait UnitOfWork {
    fn begin(&mut self);
    fn commit(&mut self) -> Result<(), Box<dyn std::error::Error>>;
    fn rollback(&mut self);
}
```

---

## 実装順序とマイルストーン

### Milestone 1: バックエンド基盤（1-2日）
1. ✅ `commands/` モジュール分割
2. ✅ `SettingsRepository` トレイト + 実装
3. ✅ `SettingsService` リファクタリング
4. ✅ テスト通過確認

### Milestone 2: バックエンド整理（1日）
1. ✅ `lib.rs` 分割 (`app_state.rs`, `setup/`)
2. ✅ `PlatformManager` Facade
3. ✅ `CommandError` 統一エラー型
4. ✅ テスト通過確認

### Milestone 3: フロントエンド分離（1-2日）
1. ✅ `noteData.svelte.ts` + `noteActions.ts` 分離
2. ✅ `settingsData.svelte.ts` + `settingsActions.ts` 分離
3. ✅ 動作確認

### Milestone 4: フロントエンド強化（1日）
1. ✅ API 型安全性強化
2. ✅ `EditorFactory` Builder パターン
3. ✅ Handler Object パターン適用
4. ✅ 全体テスト

### Milestone 5: オプション拡張
1. Plugin Architecture（必要に応じて）
2. Unit of Work（将来の複雑な操作用）

---

## ファイル変更サマリー（修正版）

> ⚠️ **重要**: 後方互換性を維持するため、フロントエンドのファイル構造変更は最小限に

### 新規作成
```
src-tauri/src/
├── commands/
│   ├── note.rs          (NEW - 既存コードを移動)
│   ├── settings.rs      (NEW - 既存コードを移動)
│   ├── window.rs        (NEW - 既存コードを移動)
│   ├── hotkey.rs        (NEW - 既存コードを移動)
│   └── error.rs         (NEW)
├── traits/
│   └── settings_repository.rs (NEW)
├── infrastructure/
│   └── file_settings_repository.rs (NEW)
├── platform/
│   └── manager.rs       (NEW - PlatformManager Facade)
├── app_state.rs         (NEW - AppState を分離)
└── setup/
    ├── mod.rs           (NEW)
    ├── window.rs        (NEW)
    └── platform.rs      (NEW)

src/lib/
├── editor/
│   ├── EditorFactory.ts (NEW - オプション)
│   └── builders/        (NEW - オプション)
└── types/
    └── handlers.ts      (NEW - オプション)
```

### 修正（既存ファイル）
```
src-tauri/src/
├── lib.rs               (MODIFIED - 簡略化、setup/ に分離)
├── commands/mod.rs      (MODIFIED - re-export + DTO定義)
├── services/settings_service.rs (MODIFIED - Repository 注入)
├── traits/mod.rs        (MODIFIED - SettingsRepository 追加)
└── platform/mod.rs      (MODIFIED - manager を追加)

src/lib/
├── stores/note.svelte.ts     (MODIFIED - 内部分離のみ、API変更なし)
├── stores/settings.svelte.ts (MODIFIED - 内部分離のみ、API変更なし)
├── services/api.ts           (MODIFIED - ファイル分割のみ、シグネチャ変更なし)
└── editor/setup.ts           (MODIFIED - Factory化はオプション)
```

### 変更しないファイル（後方互換性のため）
```
src/lib/
├── components/Editor.svelte   (変更不要 - noteStore.updateContent() のまま)
├── components/Sidebar.svelte  (変更不要)
└── routes/+page.svelte        (変更不要 - noteStore.xxx() のまま)
```

---

## リスク評価と軽減策

| リスク | 影響 | 軽減策 | 状態 |
|--------|------|--------|------|
| 既存テストの破損 | 中 | 各 Milestone でテスト実行 | - |
| Tauri コマンド登録の抜け | 高 | `invoke_handler` を慎重に更新 | - |
| Svelte 5 runes の挙動変化 | 中 | 段階的移行、動作確認 | - |
| 過度な抽象化 | 低 | YAGNI 原則を意識、必要な分だけ | - |
| **autosave 消失** | **高** | API変更なし、内部分離のみ | ✅ 修正済み |
| **API シグネチャ変更** | **高** | 既存シグネチャ維持 | ✅ 修正済み |
| **ウィンドウ状態追跡** | **高** | mark_window_visible/hidden 追加 | ✅ 修正済み |
| **イベント発火漏れ** | **高** | 呼び出し元の責務として明記 | ✅ 修正済み |

---

## 挙動変更なしの検証チェックリスト

リファクタリング後、以下の全てが既存と同じ挙動であることを確認:

### バックエンド (Rust)
- [ ] `bun run tauri dev` でビルド成功
- [ ] 全 Tauri コマンドが正常に呼び出せる
- [ ] 設定ファイル (`config.toml`) の読み書きが正常
- [ ] ノートの CRUD 操作が正常
- [ ] ホットキーでウィンドウトグルが動作
- [ ] トレイアイコンからの操作が動作
- [ ] ウィンドウ位置・サイズが保存/復元される

### フロントエンド (Svelte)
- [ ] エディタでの入力が即時反映される
- [ ] **50ms 後に autosave が発火する** ← 重要
- [ ] Ctrl+S で手動保存が動作
- [ ] サイドバーでノート切り替えが動作
- [ ] ノート削除が動作
- [ ] 設定変更が即時反映される（テーマ、フォントサイズ等）
- [ ] ホットキーで新規ノートが作成される

### 統合テスト
- [ ] ウィンドウ非表示時にジオメトリが保存される
- [ ] ウィンドウ表示時に `create-new-note` イベントが発火する
- [ ] 空のノートは非表示時に削除される
- [ ] `restore_last_note` 設定が正しく動作する

---

## 結論

このリファクタリング計画は、既存の良い設計をベースに SOLID 原則をさらに徹底し、将来の変更・拡張に強いコードベースを実現します。

**重要な設計原則**:
1. **後方互換性最優先**: 外部 API を変更しない
2. **段階的移行**: 一度に全てを変更しない
3. **テスト駆動**: 各ステップで挙動を検証

**主な改善点**:
1. **Single Responsibility**: コマンド分割（ファイル移動のみ）
2. **Open/Closed**: Plugin Architecture で拡張可能（オプション）
3. **Interface Segregation**: Repository 分離（バックエンドのみ）
4. **Dependency Inversion**: 全層でトレイト/インターフェース依存

段階的な実装により、リスクを最小限に抑えながら改善を進めることができます。

**⚠️ 修正履歴**:
- 2024-XX-XX: 初版作成
- 2024-XX-XX: レビューにより 5 件の重大な問題を修正
  - autosave 消失問題
  - API シグネチャ変更問題
  - ウィンドウ状態追跡問題
  - イベント発火漏れ問題
  - ジオメトリ保存問題
