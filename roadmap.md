# z_memo 超詳細開発ロードマップ v2.0

## 概要

**技術スタック**: Tauri v2 + Svelte 5 + Rust

**コンセプト**: ショートカット一発で即起動、書いて閉じたら必ず保存される超高速Markdownメモアプリ

**設計方針**: SOLID原則に基づく拡張可能なアーキテクチャ

---

## 設計原則

### SOLID原則の適用

| 原則 | 適用箇所 |
|------|----------|
| **S** (単一責任) | 各モジュールは1つの責務のみ。NoteRepository はデータアクセスのみ、NoteService はビジネスロジックのみ |
| **O** (開放/閉鎖) | プラグインシステムで機能拡張可能、コア変更不要 |
| **L** (リスコフ置換) | trait ベースの抽象化で実装を交換可能 |
| **I** (インターフェース分離) | 細分化された trait（Storage, Index, Settings） |
| **D** (依存性逆転) | 上位モジュールは抽象に依存、DI コンテナで注入 |

### 採用デザインパターン

| パターン | 用途 |
|----------|------|
| **Repository** | データアクセス層の抽象化（NoteRepository trait） |
| **Observer/EventBus** | イベント駆動アーキテクチャ、プラグインフック |
| **Factory** | Note, Settings の生成を一元化 |
| **Strategy** | ファイル名生成、保存戦略の交換可能性 |
| **Command** | ユーザー操作のカプセル化、Undo/Redo対応 |
| **Plugin/Extension** | 機能拡張の仕組み |
| **Dependency Injection** | テスト容易性、疎結合 |

---

## ファイル名規則

**優先順位**:
1. 本文の最初の H1（`#`）または H2（`##`）の内容 → ファイル名として使用
2. H1/H2が存在しない場合 → uid（ULID）を使用

**例**:
- `# 買い物リスト` → `買い物リスト.md`
- `## 2026年の目標` → `2026年の目標.md`
- 見出しなし → `01J1Z9P6V9WQ7H9QXGQ2K5J1ZC.md`

**ファイル名のサニタイズ**:
- 禁止文字（`/ \ : * ? " < > |`）は `_` に置換
- 空白は維持（OSが許容する範囲）
- 最大長: 200文字（文字数ベース、バイト数ではない）
- 超過時: 196文字に切り詰め + `...` 付与
- 重複時: `ファイル名_2.md`, `ファイル名_3.md` のように連番付与

---

## Phase 0: プロジェクト基盤・常駐・トグル表示

### 0.1 プロジェクト構造の整備

#### 0.1.1 Rust側ディレクトリ構造（SOLID準拠）

```
src-tauri/
├── src/
│   ├── main.rs                    # エントリポイント
│   ├── lib.rs                     # Tauriアプリ初期化、DI設定
│   ├── app_state.rs               # アプリケーション状態管理
│   │
│   ├── domain/                    # ドメイン層（ビジネスロジック）
│   │   ├── mod.rs
│   │   ├── note.rs                # Note エンティティ
│   │   ├── settings.rs            # Settings エンティティ
│   │   └── events.rs              # ドメインイベント定義
│   │
│   ├── traits/                    # インターフェース定義（依存性逆転）
│   │   ├── mod.rs
│   │   ├── storage.rs             # Storage trait
│   │   ├── repository.rs          # Repository trait
│   │   ├── filename_strategy.rs   # ファイル名生成 Strategy trait
│   │   └── event_bus.rs           # EventBus trait
│   │
│   ├── infrastructure/            # インフラ層（実装詳細）
│   │   ├── mod.rs
│   │   ├── file_storage.rs        # Storage trait の実装
│   │   ├── file_repository.rs     # Repository trait の実装
│   │   ├── heading_filename.rs    # H1/H2ファイル名戦略
│   │   ├── toml_settings.rs       # TOML設定永続化
│   │   └── event_bus_impl.rs      # EventBus 実装
│   │
│   ├── services/                  # アプリケーションサービス層
│   │   ├── mod.rs
│   │   ├── note_service.rs        # メモCRUDロジック
│   │   ├── settings_service.rs    # 設定管理ロジック
│   │   └── autosave_service.rs    # 自動保存ロジック
│   │
│   ├── platform/                  # プラットフォーム固有機能
│   │   ├── mod.rs
│   │   ├── hotkey.rs              # グローバルホットキー
│   │   ├── window.rs              # ウィンドウ制御
│   │   ├── tray.rs                # システムトレイ
│   │   └── file_watcher.rs        # ファイル監視
│   │
│   ├── commands/                  # Tauriコマンド（薄いアダプタ層）
│   │   ├── mod.rs
│   │   ├── note_commands.rs
│   │   ├── settings_commands.rs
│   │   └── window_commands.rs
│   │
│   └── plugin/                    # プラグインシステム
│       ├── mod.rs
│       ├── loader.rs              # プラグインローダー
│       ├── manifest.rs            # マニフェスト解析
│       ├── api.rs                 # プラグインAPI
│       ├── registry.rs            # 拡張ポイント登録
│       └── sandbox.rs             # セキュリティサンドボックス
│
├── Cargo.toml
└── tauri.conf.json
```

#### 0.1.2 Svelte側ディレクトリ構造（SOLID準拠）

```
src/
├── routes/
│   ├── +layout.svelte             # 共通レイアウト
│   ├── +layout.ts
│   └── +page.svelte               # メインページ
│
├── lib/
│   ├── components/                # プレゼンテーション層
│   │   ├── Editor.svelte
│   │   ├── MenuDrawer.svelte
│   │   ├── NoteList.svelte
│   │   ├── NoteListItem.svelte
│   │   ├── Settings.svelte
│   │   ├── MenuToggle.svelte
│   │   ├── ErrorToast.svelte
│   │   └── slots/                 # プラグイン用UIスロット
│   │       ├── SidebarSlot.svelte
│   │       ├── EditorToolbarSlot.svelte
│   │       └── SettingsPanelSlot.svelte
│   │
│   ├── stores/                    # 状態管理
│   │   ├── note.ts                # 現在のメモ状態
│   │   ├── notes.ts               # メモ一覧
│   │   ├── settings.ts            # 設定
│   │   ├── ui.ts                  # UI状態
│   │   └── plugins.ts             # プラグイン状態
│   │
│   ├── services/                  # ビジネスロジック
│   │   ├── noteService.ts         # メモ操作サービス
│   │   ├── settingsService.ts     # 設定サービス
│   │   └── commandService.ts      # コマンド実行サービス
│   │
│   ├── tauri/                     # Tauri通信層
│   │   ├── commands.ts            # コマンド呼び出し
│   │   └── events.ts              # イベントリスナー
│   │
│   ├── events/                    # イベントバス
│   │   ├── eventBus.ts            # 中央イベントバス
│   │   ├── types.ts               # イベント型定義
│   │   └── hooks.ts               # プラグインフック
│   │
│   ├── editor/                    # エディタ関連
│   │   ├── setup.ts               # CodeMirror設定
│   │   ├── extensions/            # 拡張機能
│   │   │   ├── livePreview.ts
│   │   │   ├── keymaps.ts
│   │   │   └── registry.ts        # 拡張登録
│   │   └── themes/
│   │       ├── tokyoNight.ts
│   │       └── light.ts
│   │
│   ├── plugin/                    # プラグインシステム
│   │   ├── PluginHost.svelte      # プラグインUI統合
│   │   ├── api.ts                 # フロントエンドAPI
│   │   ├── loader.ts              # プラグインローダー
│   │   └── types.ts               # プラグイン型定義
│   │
│   ├── utils/
│   │   ├── debounce.ts
│   │   ├── markdown.ts
│   │   └── filename.ts
│   │
│   └── styles/
│       ├── themes/
│       │   ├── tokyo-night.css
│       │   └── light.css
│       └── global.css
│
└── app.html
```

#### 0.1.3 Cargo.toml 依存関係

```toml
[package]
name = "z_memo"
version = "0.1.0"
edition = "2021"

[dependencies]
tauri = { version = "2", features = ["tray-icon", "devtools"] }
tauri-plugin-opener = "2"
tauri-plugin-global-shortcut = "2"
tauri-plugin-fs = "2"
tauri-plugin-dialog = "2"
tauri-plugin-store = "2"
serde = { version = "1", features = ["derive"] }
serde_json = "1"
serde_yaml = "0.9"
toml = "0.8"
ulid = "1"
chrono = { version = "0.4", features = ["serde"] }
thiserror = "2"
anyhow = "1"
regex = "1"
dirs = "5"
notify = "6"                       # ファイル監視
parking_lot = "0.12"               # 高速Mutex
once_cell = "1"                    # 遅延初期化
tracing = "0.1"                    # ロギング
tracing-subscriber = "0.3"

[dev-dependencies]
tempfile = "3"
mockall = "0.13"                   # モック生成
```

#### 0.1.4 tauri.conf.json

```json
{
  "$schema": "https://schema.tauri.app/config/2",
  "productName": "z_memo",
  "version": "0.1.0",
  "identifier": "com.zmemo.app",
  "build": {
    "beforeDevCommand": "bun run dev",
    "devUrl": "http://localhost:1420",
    "beforeBuildCommand": "bun run build",
    "frontendDist": "../build"
  },
  "app": {
    "withGlobalTauri": true,
    "windows": [
      {
        "title": "z_memo",
        "width": 800,
        "height": 600,
        "visible": false,
        "skipTaskbar": false,
        "decorations": true,
        "resizable": true
      }
    ],
    "trayIcon": {
      "iconPath": "icons/icon.png",
      "iconAsTemplate": true
    },
    "security": {
      "csp": null
    }
  },
  "plugins": {
    "global-shortcut": {},
    "fs": {
      "scope": ["$DOCUMENT/*", "$HOME/*", "$APPCONFIG/*"]
    },
    "dialog": {},
    "store": {}
  },
  "bundle": {
    "active": true,
    "targets": "all",
    "icon": [
      "icons/32x32.png",
      "icons/128x128.png",
      "icons/128x128@2x.png",
      "icons/icon.icns",
      "icons/icon.ico"
    ]
  }
}
```

**完了条件**: ディレクトリ構造が整い、依存関係がインストールされ、`bun run tauri dev` でエラーなく起動

---

### 0.2 コアインターフェース定義（依存性逆転の原則）

#### 0.2.1 traits/storage.rs - Storage trait

```rust
use std::path::{Path, PathBuf};
use thiserror::Error;

#[derive(Error, Debug)]
pub enum StorageError {
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),
    #[error("Directory not found: {0}")]
    DirectoryNotFound(PathBuf),
    #[error("File not found: {0}")]
    FileNotFound(PathBuf),
    #[error("Atomic write failed: {0}")]
    AtomicWriteFailed(String),
}

/// ストレージ抽象化（単一責任: ファイルI/Oのみ）
pub trait Storage: Send + Sync {
    /// アトミックにファイルを保存
    fn save_atomic(&self, path: &Path, content: &str) -> Result<(), StorageError>;

    /// ファイルを読み込み
    fn load(&self, path: &Path) -> Result<String, StorageError>;

    /// ファイルを削除
    fn delete(&self, path: &Path) -> Result<(), StorageError>;

    /// ファイル存在確認
    fn exists(&self, path: &Path) -> bool;

    /// ディレクトリ内のファイル一覧
    fn list_files(&self, dir: &Path, extension: &str) -> Result<Vec<PathBuf>, StorageError>;
}
```

#### 0.2.2 traits/repository.rs - Repository trait

```rust
use crate::domain::note::{Note, NoteMetadata};
use thiserror::Error;

#[derive(Error, Debug)]
pub enum RepositoryError {
    #[error("Storage error: {0}")]
    Storage(#[from] crate::traits::storage::StorageError),
    #[error("Note not found: {0}")]
    NotFound(String),
    #[error("Parse error: {0}")]
    ParseError(String),
    #[error("Duplicate filename: {0}")]
    DuplicateFilename(String),
}

/// メモリポジトリ抽象化（単一責任: メモのCRUD）
pub trait NoteRepository: Send + Sync {
    /// 新規メモを作成
    fn create(&self, note: &Note) -> Result<String, RepositoryError>;

    /// UIDでメモを取得
    fn find_by_uid(&self, uid: &str) -> Result<Option<Note>, RepositoryError>;

    /// ファイル名でメモを取得
    fn find_by_filename(&self, filename: &str) -> Result<Option<Note>, RepositoryError>;

    /// メモを更新（ファイル名変更対応）
    fn update(&self, note: &Note, old_filename: Option<&str>) -> Result<String, RepositoryError>;

    /// メモを削除
    fn delete(&self, uid: &str) -> Result<(), RepositoryError>;

    /// 全メモのメタデータを取得（更新日時降順）
    fn list_all(&self) -> Result<Vec<NoteMetadata>, RepositoryError>;
}
```

#### 0.2.3 traits/filename_strategy.rs - Strategy パターン

```rust
/// ファイル名生成戦略（Strategy パターン）
pub trait FilenameStrategy: Send + Sync {
    /// コンテンツからファイル名を生成
    fn generate(&self, content: &str, uid: &str) -> String;

    /// ファイル名をサニタイズ
    fn sanitize(&self, name: &str) -> String;
}
```

#### 0.2.4 traits/event_bus.rs - Observer パターン

```rust
use std::any::Any;
use std::sync::Arc;

/// イベント型マーカー
pub trait Event: Send + Sync + 'static {
    fn event_name(&self) -> &'static str;
    fn as_any(&self) -> &dyn Any;
}

/// イベントハンドラ
pub type EventHandler = Arc<dyn Fn(&dyn Event) + Send + Sync>;

/// イベントバス（Observer パターン）
pub trait EventBus: Send + Sync {
    /// イベント購読
    fn subscribe(&self, event_name: &str, handler: EventHandler) -> SubscriptionId;

    /// 購読解除
    fn unsubscribe(&self, id: SubscriptionId);

    /// イベント発火
    fn emit(&self, event: &dyn Event);

    /// 非同期イベント発火
    fn emit_async(&self, event: Box<dyn Event>);
}

#[derive(Clone, Copy, PartialEq, Eq, Hash)]
pub struct SubscriptionId(pub u64);
```

**完了条件**: 全 trait が定義され、コンパイルが通る

---

### 0.3 ドメインモデル定義

#### 0.3.1 domain/note.rs

```rust
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use ulid::Ulid;

/// メモエンティティ
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Note {
    pub uid: String,
    pub content: String,
    pub filename: Option<String>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

impl Note {
    /// 新規メモを作成（Factory メソッド）
    pub fn new() -> Self {
        let now = Utc::now();
        let uid = Ulid::new().to_string();

        Self {
            uid: uid.clone(),
            content: Self::generate_front_matter(&uid, &now, &now),
            filename: None,
            created_at: now,
            updated_at: now,
        }
    }

    /// UIDを指定して作成
    pub fn with_uid(uid: String) -> Self {
        let now = Utc::now();
        Self {
            uid: uid.clone(),
            content: Self::generate_front_matter(&uid, &now, &now),
            filename: None,
            created_at: now,
            updated_at: now,
        }
    }

    /// Front Matter を生成
    fn generate_front_matter(uid: &str, created: &DateTime<Utc>, updated: &DateTime<Utc>) -> String {
        format!(
            "---\nuid: \"{}\"\ncreated_at: \"{}\"\nupdated_at: \"{}\"\n---\n\n",
            uid,
            created.to_rfc3339(),
            updated.to_rfc3339()
        )
    }

    /// コンテンツを更新
    pub fn update_content(&mut self, content: String) {
        self.content = content;
        self.updated_at = Utc::now();
    }

    /// Front Matter の updated_at を更新
    pub fn refresh_timestamp(&mut self) {
        use regex::Regex;

        self.updated_at = Utc::now();
        let re = Regex::new(r#"updated_at:\s*"[^"]+""#).unwrap();
        self.content = re
            .replace(&self.content, format!(r#"updated_at: "{}""#, self.updated_at.to_rfc3339()))
            .to_string();
    }
}

/// メモのメタデータ（一覧表示用）
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct NoteMetadata {
    pub uid: String,
    pub display_name: String,
    pub filename: String,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}
```

#### 0.3.2 domain/events.rs - ドメインイベント

```rust
use crate::traits::event_bus::Event;
use std::any::Any;

/// メモ作成イベント
#[derive(Debug, Clone)]
pub struct NoteCreated {
    pub uid: String,
    pub filename: String,
}

impl Event for NoteCreated {
    fn event_name(&self) -> &'static str { "note:created" }
    fn as_any(&self) -> &dyn Any { self }
}

/// メモ更新イベント
#[derive(Debug, Clone)]
pub struct NoteUpdated {
    pub uid: String,
    pub filename: String,
    pub old_filename: Option<String>,
}

impl Event for NoteUpdated {
    fn event_name(&self) -> &'static str { "note:updated" }
    fn as_any(&self) -> &dyn Any { self }
}

/// メモ削除イベント
#[derive(Debug, Clone)]
pub struct NoteDeleted {
    pub uid: String,
    pub filename: String,
}

impl Event for NoteDeleted {
    fn event_name(&self) -> &'static str { "note:deleted" }
    fn as_any(&self) -> &dyn Any { self }
}

/// メモ保存前イベント（プラグインがキャンセル可能）
#[derive(Debug, Clone)]
pub struct NoteBeforeSave {
    pub uid: String,
    pub content: String,
}

impl Event for NoteBeforeSave {
    fn event_name(&self) -> &'static str { "note:before_save" }
    fn as_any(&self) -> &dyn Any { self }
}

/// ウィンドウ表示イベント
#[derive(Debug, Clone)]
pub struct WindowShown;

impl Event for WindowShown {
    fn event_name(&self) -> &'static str { "window:shown" }
    fn as_any(&self) -> &dyn Any { self }
}

/// ウィンドウ非表示イベント
#[derive(Debug, Clone)]
pub struct WindowHidden;

impl Event for WindowHidden {
    fn event_name(&self) -> &'static str { "window:hidden" }
    fn as_any(&self) -> &dyn Any { self }
}

/// アプリ起動完了イベント
#[derive(Debug, Clone)]
pub struct AppReady;

impl Event for AppReady {
    fn event_name(&self) -> &'static str { "app:ready" }
    fn as_any(&self) -> &dyn Any { self }
}

/// アプリ終了前イベント
#[derive(Debug, Clone)]
pub struct AppBeforeQuit;

impl Event for AppBeforeQuit {
    fn event_name(&self) -> &'static str { "app:before_quit" }
    fn as_any(&self) -> &dyn Any { self }
}
```

**完了条件**: ドメインモデルとイベントが定義され、コンパイルが通る

---

### 0.4 システムトレイ常駐

#### 0.4.1 トレイアイコン作成

- `src-tauri/icons/` にトレイ用アイコン配置
- 16x16, 32x32 PNG（macOSテンプレート対応）

#### 0.4.2 platform/tray.rs

```rust
use tauri::{
    tray::{MouseButton, MouseButtonState, TrayIconBuilder, TrayIconEvent},
    menu::{Menu, MenuItem},
    App, AppHandle, Manager,
};
use crate::platform::window::toggle_window;

pub fn setup_tray(app: &mut App) -> Result<(), Box<dyn std::error::Error>> {
    let open_item = MenuItem::with_id(app, "open", "開く", true, None::<&str>)?;
    let quit_item = MenuItem::with_id(app, "quit", "終了", true, None::<&str>)?;

    let menu = Menu::with_items(app, &[&open_item, &quit_item])?;

    let _tray = TrayIconBuilder::new()
        .icon(app.default_window_icon().unwrap().clone())
        .menu(&menu)
        .menu_on_left_click(false)
        .on_menu_event(|app, event| {
            match event.id.as_ref() {
                "open" => {
                    if let Some(window) = app.get_webview_window("main") {
                        let _ = window.show();
                        let _ = window.set_focus();
                    }
                }
                "quit" => {
                    app.exit(0);
                }
                _ => {}
            }
        })
        .on_tray_icon_event(|tray, event| {
            if let TrayIconEvent::Click {
                button: MouseButton::Left,
                button_state: MouseButtonState::Up,
                ..
            } = event {
                let app = tray.app_handle();
                toggle_window(app);
            }
        })
        .build(app)?;

    Ok(())
}
```

#### 0.4.3 ウィンドウ閉じる挙動変更 - lib.rs

```rust
use tauri::{WindowEvent, Manager};

// setup 内で
.on_window_event(|window, event| {
    if let WindowEvent::CloseRequested { api, .. } = event {
        // 閉じるのを防止して非表示に
        api.prevent_close();

        // 保存リクエストを送信
        let _ = window.emit("request-save-before-hide", ());
    }
})
```

**完了条件**:
- トレイアイコンが表示される
- ウィンドウを閉じても終了せず、トレイに残る
- トレイメニューから「終了」で完全終了

---

### 0.5 グローバルホットキー

#### 0.5.1 platform/hotkey.rs

```rust
use tauri::{AppHandle, Manager};
use tauri_plugin_global_shortcut::{Code, GlobalShortcutExt, Modifiers, Shortcut, ShortcutState};
use crate::platform::window::toggle_window;
use tracing::{info, error};

pub fn register_hotkey(app: &AppHandle) -> Result<(), Box<dyn std::error::Error>> {
    let shortcut = Shortcut::new(Some(Modifiers::CONTROL | Modifiers::SHIFT), Code::Space);

    app.global_shortcut().on_shortcut(shortcut, |app, _shortcut, event| {
        if event.state == ShortcutState::Pressed {
            info!("Hotkey pressed: Ctrl+Shift+Space");
            toggle_window(app);
        }
    })?;

    info!("Global hotkey registered: Ctrl+Shift+Space");
    Ok(())
}

/// ホットキー登録失敗時のフォールバック
pub fn register_hotkey_with_fallback(app: &AppHandle) {
    if let Err(e) = register_hotkey(app) {
        error!("Failed to register primary hotkey: {}", e);

        // フォールバック: Alt+Space を試す
        let fallback = Shortcut::new(Some(Modifiers::ALT), Code::Space);
        if let Err(e2) = app.global_shortcut().on_shortcut(fallback, |app, _, event| {
            if event.state == ShortcutState::Pressed {
                toggle_window(app);
            }
        }) {
            error!("Failed to register fallback hotkey: {}", e2);
            // フロントエンドにエラー通知
            if let Some(window) = app.get_webview_window("main") {
                let _ = window.emit("hotkey-registration-failed", ());
            }
        } else {
            info!("Fallback hotkey registered: Alt+Space");
        }
    }
}
```

#### 0.5.2 platform/window.rs

```rust
use tauri::{AppHandle, Manager, WebviewWindow};
use crate::app_state::AppStateManager;
use crate::domain::events::{WindowShown, WindowHidden};
use std::time::Duration;

pub fn toggle_window(app: &AppHandle) {
    let Some(window) = app.get_webview_window("main") else {
        return;
    };

    let is_visible = window.is_visible().unwrap_or(false);

    if is_visible {
        // 保存リクエストを送信（フロントエンドが保存後に hide_window を呼ぶ）
        let state_manager = app.state::<AppStateManager>();
        if state_manager.transition_to_saving() {
            let _ = window.emit("request-save-before-hide", ());
        }
    } else {
        show_window(&window);

        // イベント発火
        if let Some(event_bus) = app.try_state::<crate::infrastructure::EventBusImpl>() {
            event_bus.emit(&WindowShown);
        }
    }
}

pub fn show_window(window: &WebviewWindow) {
    let _ = window.show();
    let _ = window.set_focus();

    // 一時的に最前面に（フォーカス奪取用）
    let _ = window.set_always_on_top(true);
    std::thread::spawn({
        let window = window.clone();
        move || {
            std::thread::sleep(Duration::from_millis(100));
            let _ = window.set_always_on_top(false);
        }
    });

    // フロントエンドに通知
    let _ = window.emit("window-shown", ());
}

pub fn hide_window(window: &WebviewWindow) {
    let _ = window.hide();

    // イベント発火
    if let Some(app) = window.app_handle().try_state::<crate::infrastructure::EventBusImpl>() {
        app.emit(&WindowHidden);
    }
}
```

**完了条件**: 3OS全てでホットキーによるウィンドウ表示/非表示が動作

---

### 0.6 アプリケーション状態管理

#### 0.6.1 app_state.rs

```rust
use parking_lot::Mutex;
use std::sync::Arc;
use crate::domain::settings::Settings;
use crate::traits::{
    repository::NoteRepository,
    storage::Storage,
    event_bus::EventBus,
    filename_strategy::FilenameStrategy,
};

/// アプリケーション状態（列挙型）
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WindowState {
    Hidden,
    Visible,
    Saving,
}

/// 状態マネージャー
pub struct AppStateManager {
    window_state: Mutex<WindowState>,
}

impl AppStateManager {
    pub fn new() -> Self {
        Self {
            window_state: Mutex::new(WindowState::Hidden),
        }
    }

    pub fn get_window_state(&self) -> WindowState {
        *self.window_state.lock()
    }

    pub fn set_visible(&self) {
        *self.window_state.lock() = WindowState::Visible;
    }

    pub fn set_hidden(&self) {
        *self.window_state.lock() = WindowState::Hidden;
    }

    /// Saving 状態への遷移（Visible からのみ許可）
    pub fn transition_to_saving(&self) -> bool {
        let mut state = self.window_state.lock();
        if *state == WindowState::Visible {
            *state = WindowState::Saving;
            true
        } else {
            false
        }
    }

    /// 保存完了処理
    pub fn complete_save(&self, success: bool) {
        let mut state = self.window_state.lock();
        if *state == WindowState::Saving {
            *state = if success { WindowState::Hidden } else { WindowState::Visible };
        }
    }
}

/// 依存性コンテナ（DI）
pub struct AppContainer {
    pub settings: Arc<Mutex<Settings>>,
    pub storage: Arc<dyn Storage>,
    pub repository: Arc<dyn NoteRepository>,
    pub event_bus: Arc<dyn EventBus>,
    pub filename_strategy: Arc<dyn FilenameStrategy>,
    pub state_manager: Arc<AppStateManager>,
}

impl AppContainer {
    /// 本番環境用のコンテナを構築
    pub fn production(settings: Settings) -> Self {
        use crate::infrastructure::*;

        let settings = Arc::new(Mutex::new(settings));
        let storage: Arc<dyn Storage> = Arc::new(FileStorage::new());
        let event_bus: Arc<dyn EventBus> = Arc::new(EventBusImpl::new());
        let filename_strategy: Arc<dyn FilenameStrategy> = Arc::new(HeadingFilenameStrategy::new());

        let repository: Arc<dyn NoteRepository> = Arc::new(FileNoteRepository::new(
            storage.clone(),
            filename_strategy.clone(),
            settings.clone(),
        ));

        Self {
            settings,
            storage,
            repository,
            event_bus,
            filename_strategy,
            state_manager: Arc::new(AppStateManager::new()),
        }
    }

    /// テスト用のコンテナを構築（モック注入可能）
    #[cfg(test)]
    pub fn test(
        storage: Arc<dyn Storage>,
        repository: Arc<dyn NoteRepository>,
        event_bus: Arc<dyn EventBus>,
    ) -> Self {
        Self {
            settings: Arc::new(Mutex::new(Settings::default())),
            storage,
            repository,
            event_bus,
            filename_strategy: Arc::new(crate::infrastructure::HeadingFilenameStrategy::new()),
            state_manager: Arc::new(AppStateManager::new()),
        }
    }
}
```

**完了条件**: アプリケーション状態が適切に管理され、DI コンテナが動作

---

### 0.7 CI/CDパイプライン

#### 0.7.1 .github/workflows/build.yml

```yaml
name: Build

on:
  push:
    branches: [main]
  pull_request:
    branches: [main]

jobs:
  build:
    strategy:
      fail-fast: false
      matrix:
        platform: [macos-latest, ubuntu-22.04, windows-latest]
    runs-on: ${{ matrix.platform }}

    steps:
      - uses: actions/checkout@v4

      - name: Setup Bun
        uses: oven-sh/setup-bun@v2

      - name: Setup Rust
        uses: dtolnay/rust-toolchain@stable

      - name: Install dependencies (Linux)
        if: matrix.platform == 'ubuntu-22.04'
        run: |
          sudo apt-get update
          sudo apt-get install -y libwebkit2gtk-4.1-dev libappindicator3-dev librsvg2-dev patchelf

      - name: Install frontend dependencies
        run: bun install

      - name: Run Rust tests
        run: cargo test --manifest-path src-tauri/Cargo.toml

      - name: Build
        run: bun run tauri build

      - name: Upload artifacts
        uses: actions/upload-artifact@v4
        with:
          name: binaries-${{ matrix.platform }}
          path: src-tauri/target/release/bundle/
```

---

**Phase 0 完了条件チェックリスト**:
- [ ] プロジェクト構造が整備されている（SOLID準拠）
- [ ] コアインターフェース（trait）が定義されている
- [ ] 3OSでビルドが通る（CI緑）
- [ ] トレイ常駐が動作する
- [ ] ホットキーでshow/hideトグルが動作する
- [ ] 表示時に最前面＋フォーカスが当たる
- [ ] ×ボタンで終了せず非表示になる
- [ ] DI コンテナが動作する

---

## Phase 1: インフラ層実装・設定永続化

### 1.1 Storage 実装

#### 1.1.1 infrastructure/file_storage.rs

```rust
use crate::traits::storage::{Storage, StorageError};
use std::fs::{self, File};
use std::io::Write;
use std::path::{Path, PathBuf};
use tracing::{debug, error};

/// ファイルシステムベースの Storage 実装
pub struct FileStorage;

impl FileStorage {
    pub fn new() -> Self {
        Self
    }
}

impl Storage for FileStorage {
    fn save_atomic(&self, path: &Path, content: &str) -> Result<(), StorageError> {
        // 親ディレクトリを作成
        if let Some(parent) = path.parent() {
            if !parent.exists() {
                fs::create_dir_all(parent)?;
            }
        }

        // 一時ファイルパス
        let tmp_path = path.with_extension("tmp");

        // 1. 一時ファイルに書き込み
        {
            let mut file = File::create(&tmp_path)?;
            file.write_all(content.as_bytes())?;

            // 2. fsync で確実にディスクへ
            file.sync_all()?;
        }

        // 3. rename でアトミックに置換
        fs::rename(&tmp_path, path).map_err(|e| {
            // クリーンアップ
            let _ = fs::remove_file(&tmp_path);
            StorageError::AtomicWriteFailed(e.to_string())
        })?;

        debug!("Saved file: {:?}", path);
        Ok(())
    }

    fn load(&self, path: &Path) -> Result<String, StorageError> {
        if !path.exists() {
            return Err(StorageError::FileNotFound(path.to_path_buf()));
        }
        Ok(fs::read_to_string(path)?)
    }

    fn delete(&self, path: &Path) -> Result<(), StorageError> {
        if path.exists() {
            fs::remove_file(path)?;
            debug!("Deleted file: {:?}", path);
        }
        Ok(())
    }

    fn exists(&self, path: &Path) -> bool {
        path.exists()
    }

    fn list_files(&self, dir: &Path, extension: &str) -> Result<Vec<PathBuf>, StorageError> {
        if !dir.exists() {
            fs::create_dir_all(dir)?;
            return Ok(Vec::new());
        }

        let mut files = Vec::new();
        for entry in fs::read_dir(dir)? {
            let entry = entry?;
            let path = entry.path();
            if path.extension().map_or(false, |ext| ext == extension) {
                files.push(path);
            }
        }

        Ok(files)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[test]
    fn test_atomic_save() {
        let dir = tempdir().unwrap();
        let storage = FileStorage::new();
        let path = dir.path().join("test.md");

        storage.save_atomic(&path, "# Test").unwrap();

        assert!(path.exists());
        assert_eq!(storage.load(&path).unwrap(), "# Test");
    }

    #[test]
    fn test_atomic_save_overwrites() {
        let dir = tempdir().unwrap();
        let storage = FileStorage::new();
        let path = dir.path().join("test.md");

        storage.save_atomic(&path, "# First").unwrap();
        storage.save_atomic(&path, "# Second").unwrap();

        assert_eq!(storage.load(&path).unwrap(), "# Second");
    }
}
```

### 1.2 FilenameStrategy 実装

#### 1.2.1 infrastructure/heading_filename.rs

```rust
use crate::traits::filename_strategy::FilenameStrategy;
use regex::Regex;
use once_cell::sync::Lazy;

static HEADING_REGEX: Lazy<Regex> = Lazy::new(|| {
    Regex::new(r"^#{1,2}\s+(.+)$").unwrap()
});

static FORBIDDEN_CHARS: &[char] = &['/', '\\', ':', '*', '?', '"', '<', '>', '|'];

/// H1/H2 見出しをファイル名にする戦略
pub struct HeadingFilenameStrategy;

impl HeadingFilenameStrategy {
    pub fn new() -> Self {
        Self
    }

    /// Front Matter をスキップして本文を取得
    fn skip_front_matter<'a>(&self, content: &'a str) -> &'a str {
        if !content.starts_with("---") {
            return content;
        }

        // "---\n" の後から検索開始
        let search_start = match content.find('\n') {
            Some(i) => i + 1,
            None => return content,
        };

        // 終了の "---" を探す
        if let Some(end_pos) = content[search_start..].find("\n---") {
            let body_start = search_start + end_pos + 4; // "\n---" の長さ
            // 先頭の改行をスキップ
            content[body_start..].trim_start_matches('\n')
        } else {
            content
        }
    }

    /// 最初の H1/H2 見出しを抽出
    fn extract_first_heading(&self, content: &str) -> Option<String> {
        let body = self.skip_front_matter(content);

        for line in body.lines() {
            let trimmed = line.trim();
            if let Some(caps) = HEADING_REGEX.captures(trimmed) {
                return Some(caps[1].trim().to_string());
            }
        }

        None
    }
}

impl FilenameStrategy for HeadingFilenameStrategy {
    fn generate(&self, content: &str, uid: &str) -> String {
        let name = self.extract_first_heading(content)
            .map(|h| self.sanitize(&h))
            .filter(|s| !s.is_empty())
            .unwrap_or_else(|| uid.to_string());

        format!("{}.md", name)
    }

    fn sanitize(&self, name: &str) -> String {
        // 禁止文字を置換
        let sanitized: String = name
            .chars()
            .map(|c| if FORBIDDEN_CHARS.contains(&c) { '_' } else { c })
            .collect();

        // 文字数ベースで切り詰め（UTF-8安全）
        let char_count = sanitized.chars().count();
        if char_count > 200 {
            let truncated: String = sanitized.chars().take(196).collect();
            format!("{}...", truncated)
        } else {
            sanitized
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_generate_with_h1() {
        let strategy = HeadingFilenameStrategy::new();
        let content = "---\nuid: \"test\"\n---\n\n# My Note Title\n\nContent";
        assert_eq!(strategy.generate(content, "uid123"), "My Note Title.md");
    }

    #[test]
    fn test_generate_with_h2() {
        let strategy = HeadingFilenameStrategy::new();
        let content = "---\nuid: \"test\"\n---\n\n## Second Level\n\nContent";
        assert_eq!(strategy.generate(content, "uid123"), "Second Level.md");
    }

    #[test]
    fn test_generate_fallback_to_uid() {
        let strategy = HeadingFilenameStrategy::new();
        let content = "---\nuid: \"test\"\n---\n\nNo heading here";
        assert_eq!(strategy.generate(content, "uid123"), "uid123.md");
    }

    #[test]
    fn test_sanitize_forbidden_chars() {
        let strategy = HeadingFilenameStrategy::new();
        assert_eq!(strategy.sanitize("Hello/World"), "Hello_World");
        assert_eq!(strategy.sanitize("File:Name"), "File_Name");
        assert_eq!(strategy.sanitize("Test?<>|"), "Test____");
    }

    #[test]
    fn test_sanitize_japanese() {
        let strategy = HeadingFilenameStrategy::new();
        assert_eq!(strategy.sanitize("日本語のタイトル"), "日本語のタイトル");
    }

    #[test]
    fn test_sanitize_long_name() {
        let strategy = HeadingFilenameStrategy::new();
        let long_name = "あ".repeat(250);
        let result = strategy.sanitize(&long_name);
        assert!(result.chars().count() <= 200);
        assert!(result.ends_with("..."));
    }
}
```

### 1.3 設定管理

#### 1.3.1 domain/settings.rs

```rust
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Settings {
    pub window: WindowSettings,
    pub storage: StorageSettings,
    pub editor: EditorSettings,
    pub theme: ThemeSettings,
    pub hotkey: HotkeySettings,
    pub autosave: AutosaveSettings,
    pub behavior: BehaviorSettings,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WindowSettings {
    pub x: Option<i32>,
    pub y: Option<i32>,
    pub width: u32,
    pub height: u32,
    pub is_maximized: bool,
    pub monitor_id: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StorageSettings {
    pub directory: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EditorSettings {
    pub font_family: String,
    pub font_size: u32,
    pub line_height: f32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ThemeSettings {
    pub name: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HotkeySettings {
    pub toggle: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AutosaveSettings {
    pub enabled: bool,
    pub delay_ms: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BehaviorSettings {
    pub restore_last_note: bool,
}

impl Default for Settings {
    fn default() -> Self {
        let documents_dir = dirs::document_dir()
            .unwrap_or_else(|| PathBuf::from("."))
            .join("z_memo");

        Self {
            window: WindowSettings {
                x: None,
                y: None,
                width: 800,
                height: 600,
                is_maximized: false,
                monitor_id: None,
            },
            storage: StorageSettings {
                directory: documents_dir.to_string_lossy().to_string(),
            },
            editor: EditorSettings {
                font_family: "system-ui".to_string(),
                font_size: 14,
                line_height: 1.6,
            },
            theme: ThemeSettings {
                name: "tokyo-night".to_string(),
            },
            hotkey: HotkeySettings {
                toggle: "Ctrl+Shift+Space".to_string(),
            },
            autosave: AutosaveSettings {
                enabled: true,
                delay_ms: 2000,
            },
            behavior: BehaviorSettings {
                restore_last_note: false,
            },
        }
    }
}
```

#### 1.3.2 infrastructure/toml_settings.rs

```rust
use crate::domain::settings::Settings;
use std::fs;
use std::path::PathBuf;
use thiserror::Error;
use tracing::{debug, info, warn};

#[derive(Error, Debug)]
pub enum SettingsError {
    #[error("Config directory not found")]
    ConfigDirNotFound,
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),
    #[error("TOML parse error: {0}")]
    TomlParse(#[from] toml::de::Error),
    #[error("TOML serialize error: {0}")]
    TomlSerialize(#[from] toml::ser::Error),
}

/// 設定ファイルパスを取得
pub fn config_path() -> Result<PathBuf, SettingsError> {
    dirs::config_dir()
        .map(|p| p.join("z_memo").join("config.toml"))
        .ok_or(SettingsError::ConfigDirNotFound)
}

/// 設定を読み込み
pub fn load_settings() -> Result<Settings, SettingsError> {
    let path = config_path()?;

    if path.exists() {
        let content = fs::read_to_string(&path)?;
        let settings: Settings = toml::from_str(&content)?;
        debug!("Loaded settings from {:?}", path);
        Ok(settings)
    } else {
        info!("Settings file not found, creating default");
        let settings = Settings::default();
        save_settings(&settings)?;
        Ok(settings)
    }
}

/// 設定を保存
pub fn save_settings(settings: &Settings) -> Result<(), SettingsError> {
    let path = config_path()?;

    // ディレクトリを作成
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }

    let content = toml::to_string_pretty(settings)?;
    fs::write(&path, content)?;

    debug!("Saved settings to {:?}", path);
    Ok(())
}
```

### 1.4 Repository 実装

#### 1.4.1 infrastructure/file_repository.rs

```rust
use crate::domain::note::{Note, NoteMetadata};
use crate::domain::settings::Settings;
use crate::traits::{
    filename_strategy::FilenameStrategy,
    repository::{NoteRepository, RepositoryError},
    storage::Storage,
};
use chrono::{DateTime, Utc};
use parking_lot::Mutex;
use regex::Regex;
use serde::Deserialize;
use std::path::PathBuf;
use std::sync::Arc;
use tracing::{debug, warn};

pub struct FileNoteRepository {
    storage: Arc<dyn Storage>,
    filename_strategy: Arc<dyn FilenameStrategy>,
    settings: Arc<Mutex<Settings>>,
}

impl FileNoteRepository {
    pub fn new(
        storage: Arc<dyn Storage>,
        filename_strategy: Arc<dyn FilenameStrategy>,
        settings: Arc<Mutex<Settings>>,
    ) -> Self {
        Self {
            storage,
            filename_strategy,
            settings,
        }
    }

    fn get_directory(&self) -> PathBuf {
        PathBuf::from(&self.settings.lock().storage.directory)
    }

    fn resolve_duplicate(&self, filename: &str, directory: &PathBuf) -> String {
        let name_without_ext = filename.trim_end_matches(".md");
        let mut final_name = filename.to_string();
        let mut counter = 2;

        while self.storage.exists(&directory.join(&final_name)) {
            final_name = format!("{}_{}.md", name_without_ext, counter);
            counter += 1;
        }

        final_name
    }

    fn parse_note(&self, path: &PathBuf) -> Result<Note, RepositoryError> {
        let content = self.storage.load(path)?;
        let filename = path.file_name()
            .and_then(|n| n.to_str())
            .map(String::from);

        let front_matter = self.parse_front_matter(&content)
            .ok_or_else(|| RepositoryError::ParseError("Invalid front matter".to_string()))?;

        Ok(Note {
            uid: front_matter.uid,
            content,
            filename,
            created_at: front_matter.created_at,
            updated_at: front_matter.updated_at,
        })
    }

    fn parse_front_matter(&self, content: &str) -> Option<FrontMatter> {
        if !content.starts_with("---") {
            return None;
        }

        let parts: Vec<&str> = content.splitn(3, "---").collect();
        if parts.len() < 3 {
            return None;
        }

        serde_yaml::from_str(parts[1].trim()).ok()
    }

    fn extract_display_name(&self, content: &str, uid: &str) -> String {
        // Front Matter の title を優先
        if let Some(fm) = self.parse_front_matter(content) {
            if let Some(title) = fm.title {
                return title;
            }
        }

        // H1/H2 を抽出
        let heading_regex = Regex::new(r"^#{1,2}\s+(.+)$").unwrap();
        let body = self.skip_front_matter(content);

        for line in body.lines() {
            if let Some(caps) = heading_regex.captures(line.trim()) {
                return caps[1].trim().to_string();
            }
        }

        // フォールバック: uid
        uid.to_string()
    }

    fn skip_front_matter<'a>(&self, content: &'a str) -> &'a str {
        if !content.starts_with("---") {
            return content;
        }

        let search_start = content.find('\n').map(|i| i + 1).unwrap_or(3);
        if let Some(end_pos) = content[search_start..].find("\n---") {
            content[search_start + end_pos + 4..].trim_start_matches('\n')
        } else {
            content
        }
    }
}

#[derive(Debug, Deserialize)]
struct FrontMatter {
    uid: String,
    created_at: DateTime<Utc>,
    updated_at: DateTime<Utc>,
    title: Option<String>,
}

impl NoteRepository for FileNoteRepository {
    fn create(&self, note: &Note) -> Result<String, RepositoryError> {
        let directory = self.get_directory();
        let filename = self.filename_strategy.generate(&note.content, &note.uid);
        let final_filename = self.resolve_duplicate(&filename, &directory);

        let path = directory.join(&final_filename);
        self.storage.save_atomic(&path, &note.content)?;

        debug!("Created note: {}", final_filename);
        Ok(final_filename)
    }

    fn find_by_uid(&self, uid: &str) -> Result<Option<Note>, RepositoryError> {
        let directory = self.get_directory();
        let files = self.storage.list_files(&directory, "md")?;

        for path in files {
            if let Ok(note) = self.parse_note(&path) {
                if note.uid == uid {
                    return Ok(Some(note));
                }
            }
        }

        Ok(None)
    }

    fn find_by_filename(&self, filename: &str) -> Result<Option<Note>, RepositoryError> {
        let path = self.get_directory().join(filename);
        if self.storage.exists(&path) {
            Ok(Some(self.parse_note(&path)?))
        } else {
            Ok(None)
        }
    }

    fn update(&self, note: &Note, old_filename: Option<&str>) -> Result<String, RepositoryError> {
        let directory = self.get_directory();
        let new_filename = self.filename_strategy.generate(&note.content, &note.uid);

        // 古いファイルを削除（ファイル名が変わった場合）
        if let Some(old) = old_filename {
            if old != new_filename {
                let old_path = directory.join(old);
                if self.storage.exists(&old_path) {
                    self.storage.delete(&old_path)?;
                }
            }
        }

        // 重複解決（自分自身は除外）
        let final_filename = if old_filename == Some(&new_filename) {
            new_filename
        } else {
            self.resolve_duplicate(&new_filename, &directory)
        };

        let path = directory.join(&final_filename);
        self.storage.save_atomic(&path, &note.content)?;

        debug!("Updated note: {}", final_filename);
        Ok(final_filename)
    }

    fn delete(&self, uid: &str) -> Result<(), RepositoryError> {
        if let Some(note) = self.find_by_uid(uid)? {
            if let Some(filename) = note.filename {
                let path = self.get_directory().join(filename);
                self.storage.delete(&path)?;
                debug!("Deleted note: {}", uid);
            }
        }
        Ok(())
    }

    fn list_all(&self) -> Result<Vec<NoteMetadata>, RepositoryError> {
        let directory = self.get_directory();
        let files = self.storage.list_files(&directory, "md")?;

        let mut notes = Vec::new();
        for path in files {
            if let Ok(note) = self.parse_note(&path) {
                let display_name = self.extract_display_name(&note.content, &note.uid);
                notes.push(NoteMetadata {
                    uid: note.uid,
                    display_name,
                    filename: note.filename.unwrap_or_default(),
                    created_at: note.created_at,
                    updated_at: note.updated_at,
                });
            }
        }

        // 更新日時で降順ソート
        notes.sort_by(|a, b| b.updated_at.cmp(&a.updated_at));

        Ok(notes)
    }
}
```

### 1.5 EventBus 実装

#### 1.5.1 infrastructure/event_bus_impl.rs

```rust
use crate::traits::event_bus::{Event, EventBus, EventHandler, SubscriptionId};
use parking_lot::RwLock;
use std::collections::HashMap;
use std::sync::atomic::{AtomicU64, Ordering};
use tracing::debug;

pub struct EventBusImpl {
    handlers: RwLock<HashMap<String, Vec<(SubscriptionId, EventHandler)>>>,
    next_id: AtomicU64,
}

impl EventBusImpl {
    pub fn new() -> Self {
        Self {
            handlers: RwLock::new(HashMap::new()),
            next_id: AtomicU64::new(1),
        }
    }
}

impl EventBus for EventBusImpl {
    fn subscribe(&self, event_name: &str, handler: EventHandler) -> SubscriptionId {
        let id = SubscriptionId(self.next_id.fetch_add(1, Ordering::SeqCst));

        let mut handlers = self.handlers.write();
        handlers
            .entry(event_name.to_string())
            .or_insert_with(Vec::new)
            .push((id, handler));

        debug!("Subscribed to event: {} (id: {})", event_name, id.0);
        id
    }

    fn unsubscribe(&self, id: SubscriptionId) {
        let mut handlers = self.handlers.write();
        for handlers_list in handlers.values_mut() {
            handlers_list.retain(|(sub_id, _)| *sub_id != id);
        }
        debug!("Unsubscribed: {}", id.0);
    }

    fn emit(&self, event: &dyn Event) {
        let handlers = self.handlers.read();
        if let Some(handlers_list) = handlers.get(event.event_name()) {
            debug!("Emitting event: {} ({} handlers)", event.event_name(), handlers_list.len());
            for (_, handler) in handlers_list {
                handler(event);
            }
        }
    }

    fn emit_async(&self, event: Box<dyn Event>) {
        let handlers = self.handlers.read().clone();
        std::thread::spawn(move || {
            if let Some(handlers_list) = handlers.get(event.event_name()) {
                for (_, handler) in handlers_list {
                    handler(event.as_ref());
                }
            }
        });
    }
}
```

**Phase 1 完了条件チェックリスト**:
- [ ] Storage trait 実装が動作
- [ ] FilenameStrategy が正しくファイル名を生成
- [ ] 設定ファイルが正しいパスに生成される
- [ ] Repository がCRUD操作を正しく実行
- [ ] EventBus がイベントを正しく配信
- [ ] 全ての単体テストが通る

---

## Phase 2: UI骨格（編集画面・メニュー・設定）

### 2.1 グローバルスタイル・テーマ

#### 2.1.1 Tokyo Night カラーパレット: styles/themes/tokyo-night.css

```css
:root[data-theme="tokyo-night"] {
  /* Background */
  --bg-primary: #1a1b26;
  --bg-secondary: #16161e;
  --bg-tertiary: #1f2335;
  --bg-highlight: #292e42;

  /* Foreground */
  --fg-primary: #c0caf5;
  --fg-secondary: #a9b1d6;
  --fg-muted: #565f89;
  --fg-dark: #414868;

  /* Accent */
  --accent-blue: #7aa2f7;
  --accent-cyan: #7dcfff;
  --accent-purple: #bb9af7;
  --accent-green: #9ece6a;
  --accent-orange: #ff9e64;
  --accent-red: #f7768e;
  --accent-yellow: #e0af68;

  /* UI Elements */
  --border-color: #292e42;
  --scrollbar-track: #1a1b26;
  --scrollbar-thumb: #414868;

  /* Editor specific */
  --editor-bg: var(--bg-primary);
  --editor-line-highlight: #1e2030;
  --editor-selection: #364a82;
  --editor-cursor: var(--accent-blue);
}
```

#### 2.1.2 Light テーマ: styles/themes/light.css

```css
:root[data-theme="light"] {
  --bg-primary: #f5f5f5;
  --bg-secondary: #ffffff;
  --bg-tertiary: #e8e8e8;
  --bg-highlight: #d4d4d4;

  --fg-primary: #1a1a1a;
  --fg-secondary: #4a4a4a;
  --fg-muted: #8a8a8a;
  --fg-dark: #6a6a6a;

  --accent-blue: #0066cc;
  --accent-cyan: #0891b2;
  --accent-purple: #7c3aed;
  --accent-green: #16a34a;
  --accent-orange: #ea580c;
  --accent-red: #dc2626;
  --accent-yellow: #ca8a04;

  --border-color: #d4d4d4;
  --scrollbar-track: #f5f5f5;
  --scrollbar-thumb: #a3a3a3;

  --editor-bg: var(--bg-secondary);
  --editor-line-highlight: #f0f0f0;
  --editor-selection: #add6ff;
  --editor-cursor: var(--accent-blue);
}
```

#### 2.1.3 グローバルスタイル: styles/global.css

```css
@import './themes/tokyo-night.css';
@import './themes/light.css';

* {
  box-sizing: border-box;
  margin: 0;
  padding: 0;
}

html, body {
  height: 100%;
  overflow: hidden;
  font-family: var(--font-family, system-ui, -apple-system, sans-serif);
  font-size: var(--font-size, 14px);
  line-height: var(--line-height, 1.6);
  background-color: var(--bg-primary);
  color: var(--fg-primary);
}

::-webkit-scrollbar {
  width: 8px;
}

::-webkit-scrollbar-track {
  background: var(--scrollbar-track);
}

::-webkit-scrollbar-thumb {
  background: var(--scrollbar-thumb);
  border-radius: 4px;
}

::-webkit-scrollbar-thumb:hover {
  background: var(--fg-muted);
}
```

### 2.2 イベントバス（フロントエンド）

#### 2.2.1 lib/events/eventBus.ts

```typescript
type EventCallback<T = unknown> = (data: T) => void;

interface Subscription {
  id: number;
  callback: EventCallback;
}

class EventBus {
  private handlers: Map<string, Subscription[]> = new Map();
  private nextId = 1;

  /**
   * イベントを購読
   */
  on<T>(event: string, callback: EventCallback<T>): () => void {
    const id = this.nextId++;
    const subscription: Subscription = { id, callback: callback as EventCallback };

    if (!this.handlers.has(event)) {
      this.handlers.set(event, []);
    }
    this.handlers.get(event)!.push(subscription);

    // 購読解除関数を返す
    return () => this.off(event, id);
  }

  /**
   * 一度だけ購読
   */
  once<T>(event: string, callback: EventCallback<T>): () => void {
    const unsubscribe = this.on<T>(event, (data) => {
      unsubscribe();
      callback(data);
    });
    return unsubscribe;
  }

  /**
   * 購読解除
   */
  private off(event: string, id: number): void {
    const handlers = this.handlers.get(event);
    if (handlers) {
      const index = handlers.findIndex(h => h.id === id);
      if (index !== -1) {
        handlers.splice(index, 1);
      }
    }
  }

  /**
   * イベント発火
   */
  emit<T>(event: string, data?: T): void {
    const handlers = this.handlers.get(event);
    if (handlers) {
      handlers.forEach(h => h.callback(data));
    }
  }

  /**
   * 全購読解除
   */
  clear(): void {
    this.handlers.clear();
  }
}

// シングルトンインスタンス
export const eventBus = new EventBus();

// イベント名定数（型安全）
export const Events = {
  // Note events
  NOTE_CREATED: 'note:created',
  NOTE_UPDATED: 'note:updated',
  NOTE_DELETED: 'note:deleted',
  NOTE_CONTENT_CHANGED: 'note:content_changed',

  // UI events
  MENU_OPEN: 'ui:menu_open',
  MENU_CLOSE: 'ui:menu_close',
  SETTINGS_OPEN: 'ui:settings_open',
  SETTINGS_CLOSE: 'ui:settings_close',

  // Editor events
  EDITOR_READY: 'editor:ready',
  EDITOR_FOCUS: 'editor:focus',

  // Save events
  SAVE_REQUESTED: 'save:requested',
  SAVE_COMPLETED: 'save:completed',
  SAVE_FAILED: 'save:failed',
} as const;
```

#### 2.2.2 lib/events/types.ts

```typescript
import type { NoteMetadata } from '$lib/stores/notes';

// イベントペイロード型定義
export interface NoteCreatedPayload {
  uid: string;
  filename: string;
}

export interface NoteUpdatedPayload {
  uid: string;
  filename: string;
  oldFilename?: string;
}

export interface NoteDeletedPayload {
  uid: string;
  filename: string;
}

export interface NoteContentChangedPayload {
  uid: string;
  content: string;
  isDirty: boolean;
}

export interface SaveCompletedPayload {
  uid: string;
  filename: string;
}

export interface SaveFailedPayload {
  uid: string;
  error: string;
}
```

### 2.3 Svelte Stores（状態管理）

#### 2.3.1 stores/settings.ts

```typescript
import { writable, derived } from 'svelte/store';

export interface Settings {
  window: {
    x: number | null;
    y: number | null;
    width: number;
    height: number;
    isMaximized: boolean;
  };
  storage: {
    directory: string;
  };
  editor: {
    fontFamily: string;
    fontSize: number;
    lineHeight: number;
  };
  theme: {
    name: string;
  };
  hotkey: {
    toggle: string;
  };
  autosave: {
    enabled: boolean;
    delayMs: number;
  };
  behavior: {
    restoreLastNote: boolean;
  };
}

const defaultSettings: Settings = {
  window: { x: null, y: null, width: 800, height: 600, isMaximized: false },
  storage: { directory: '' },
  editor: { fontFamily: 'system-ui', fontSize: 14, lineHeight: 1.6 },
  theme: { name: 'tokyo-night' },
  hotkey: { toggle: 'Ctrl+Shift+Space' },
  autosave: { enabled: true, delayMs: 2000 },
  behavior: { restoreLastNote: false },
};

export const settings = writable<Settings>(defaultSettings);

// 派生ストア: テーマ名のみ
export const themeName = derived(settings, $s => $s.theme.name);

// 派生ストア: エディタ設定のみ
export const editorSettings = derived(settings, $s => $s.editor);
```

#### 2.3.2 stores/note.ts

```typescript
import { writable, get } from 'svelte/store';
import { eventBus, Events } from '$lib/events/eventBus';

export interface Note {
  uid: string;
  content: string;
  filename: string | null;
  isDirty: boolean;
  createdAt: string;
  updatedAt: string;
}

function createNoteStore() {
  const { subscribe, set, update } = writable<Note | null>(null);

  return {
    subscribe,

    /**
     * メモを読み込み
     */
    load(note: Note) {
      set({ ...note, isDirty: false });
    },

    /**
     * 新規メモを作成（Rust側で生成したデータを使用）
     */
    setNew(data: { uid: string; content: string; createdAt: string; updatedAt: string }) {
      set({
        uid: data.uid,
        content: data.content,
        filename: null,
        isDirty: false,
        createdAt: data.createdAt,
        updatedAt: data.updatedAt,
      });
    },

    /**
     * コンテンツを更新
     */
    updateContent(content: string) {
      update(note => {
        if (note) {
          const updated = { ...note, content, isDirty: true };
          eventBus.emit(Events.NOTE_CONTENT_CHANGED, {
            uid: note.uid,
            content,
            isDirty: true,
          });
          return updated;
        }
        return note;
      });
    },

    /**
     * 保存完了をマーク
     */
    markSaved(filename: string) {
      update(note => {
        if (note) {
          return {
            ...note,
            filename,
            isDirty: false,
            updatedAt: new Date().toISOString(),
          };
        }
        return note;
      });
    },

    /**
     * クリア
     */
    clear() {
      set(null);
    },

    /**
     * 現在の値を取得（非リアクティブ）
     */
    get() {
      return get({ subscribe });
    },
  };
}

export const currentNote = createNoteStore();
```

#### 2.3.3 stores/notes.ts

```typescript
import { writable } from 'svelte/store';

export interface NoteMetadata {
  uid: string;
  displayName: string;
  filename: string;
  createdAt: string;
  updatedAt: string;
}

export const notes = writable<NoteMetadata[]>([]);
```

#### 2.3.4 stores/ui.ts

```typescript
import { writable, derived } from 'svelte/store';

interface UIState {
  isMenuOpen: boolean;
  isSettingsOpen: boolean;
  errorMessage: string | null;
  isLoading: boolean;
}

const initialState: UIState = {
  isMenuOpen: false,
  isSettingsOpen: false,
  errorMessage: null,
  isLoading: false,
};

function createUIStore() {
  const { subscribe, set, update } = writable<UIState>(initialState);

  return {
    subscribe,

    openMenu() {
      update(s => ({ ...s, isMenuOpen: true }));
    },

    closeMenu() {
      update(s => ({ ...s, isMenuOpen: false }));
    },

    toggleMenu() {
      update(s => ({ ...s, isMenuOpen: !s.isMenuOpen }));
    },

    openSettings() {
      update(s => ({ ...s, isSettingsOpen: true, isMenuOpen: false }));
    },

    closeSettings() {
      update(s => ({ ...s, isSettingsOpen: false }));
    },

    showError(message: string) {
      update(s => ({ ...s, errorMessage: message }));
    },

    clearError() {
      update(s => ({ ...s, errorMessage: null }));
    },

    setLoading(loading: boolean) {
      update(s => ({ ...s, isLoading: loading }));
    },
  };
}

export const ui = createUIStore();

// 個別の派生ストア（コンポーネント用）
export const isMenuOpen = derived(ui, $ui => $ui.isMenuOpen);
export const isSettingsOpen = derived(ui, $ui => $ui.isSettingsOpen);
export const errorMessage = derived(ui, $ui => $ui.errorMessage);
```

### 2.4 Tauri 通信層

#### 2.4.1 lib/tauri/commands.ts

```typescript
import { invoke } from '@tauri-apps/api/core';
import type { Settings } from '$lib/stores/settings';
import type { NoteMetadata } from '$lib/stores/notes';

// 型定義
export interface SaveNoteRequest {
  uid: string;
  content: string;
  currentFilename: string | null;
}

export interface SaveNoteResponse {
  filename: string;
  path: string;
}

export interface NewNoteData {
  uid: string;
  content: string;
  createdAt: string;
  updatedAt: string;
}

// Settings commands
export async function loadSettings(): Promise<Settings> {
  return invoke('get_settings');
}

export async function saveSettings(settings: Settings): Promise<void> {
  return invoke('save_settings', { settings });
}

// Note commands
export async function createNewNote(): Promise<NewNoteData> {
  return invoke('create_new_note');
}

export async function saveNote(request: SaveNoteRequest): Promise<SaveNoteResponse> {
  return invoke('save_note', { request });
}

export async function loadNoteByFilename(filename: string): Promise<string> {
  return invoke('load_note_by_filename', { filename });
}

export async function deleteNote(filename: string): Promise<void> {
  return invoke('delete_note', { filename });
}

export async function getNoteList(): Promise<NoteMetadata[]> {
  return invoke('get_note_list');
}

// Window commands
export async function hideWindowAfterSave(success: boolean): Promise<void> {
  return invoke('hide_window_after_save', { success });
}

// Directory selection
export async function selectDirectory(): Promise<string | null> {
  return invoke('select_save_directory');
}
```

#### 2.4.2 lib/tauri/events.ts

```typescript
import { listen, type UnlistenFn } from '@tauri-apps/api/event';
import { currentNote } from '$lib/stores/note';
import { notes } from '$lib/stores/notes';
import { ui } from '$lib/stores/ui';
import { eventBus, Events } from '$lib/events/eventBus';
import { saveNote, hideWindowAfterSave, getNoteList } from './commands';

let unlisten: UnlistenFn[] = [];

export async function setupTauriListeners(): Promise<void> {
  // ウィンドウ表示イベント
  unlisten.push(
    await listen('window-shown', () => {
      eventBus.emit(Events.EDITOR_FOCUS);
    })
  );

  // 保存リクエスト（非表示前）
  unlisten.push(
    await listen('request-save-before-hide', async () => {
      const note = currentNote.get();

      if (!note || !note.isDirty) {
        await hideWindowAfterSave(true);
        return;
      }

      try {
        const result = await saveNote({
          uid: note.uid,
          content: note.content,
          currentFilename: note.filename,
        });

        currentNote.markSaved(result.filename);
        eventBus.emit(Events.SAVE_COMPLETED, { uid: note.uid, filename: result.filename });
        await hideWindowAfterSave(true);
      } catch (error) {
        console.error('Save failed:', error);
        ui.showError('保存に失敗しました。再度お試しください。');
        eventBus.emit(Events.SAVE_FAILED, { uid: note.uid, error: String(error) });
        await hideWindowAfterSave(false);
      }
    })
  );

  // ファイル変更通知
  unlisten.push(
    await listen('notes-changed', async () => {
      const list = await getNoteList();
      notes.set(list);
    })
  );

  // ホットキー登録失敗
  unlisten.push(
    await listen('hotkey-registration-failed', () => {
      ui.showError('ホットキーの登録に失敗しました。他のアプリと競合している可能性があります。');
    })
  );
}

export function cleanupTauriListeners(): void {
  unlisten.forEach(fn => fn());
  unlisten = [];
}
```

### 2.5 コンポーネント実装

#### 2.5.1 routes/+layout.svelte

```svelte
<script lang="ts">
  import { onMount, onDestroy } from 'svelte';
  import { settings, themeName, editorSettings } from '$lib/stores/settings';
  import { loadSettings } from '$lib/tauri/commands';
  import { setupTauriListeners, cleanupTauriListeners } from '$lib/tauri/events';
  import '$lib/styles/global.css';

  onMount(async () => {
    // 設定読み込み
    const loaded = await loadSettings();
    settings.set(loaded);

    // Tauriイベントリスナー設定
    await setupTauriListeners();
  });

  onDestroy(() => {
    cleanupTauriListeners();
  });

  // テーマ・フォント設定の反映
  $effect(() => {
    document.documentElement.setAttribute('data-theme', $themeName);
  });

  $effect(() => {
    const root = document.documentElement;
    root.style.setProperty('--font-family', $editorSettings.fontFamily);
    root.style.setProperty('--font-size', `${$editorSettings.fontSize}px`);
    root.style.setProperty('--line-height', String($editorSettings.lineHeight));
  });
</script>

<slot />
```

#### 2.5.2 routes/+page.svelte

```svelte
<script lang="ts">
  import Editor from '$lib/components/Editor.svelte';
  import MenuDrawer from '$lib/components/MenuDrawer.svelte';
  import MenuToggle from '$lib/components/MenuToggle.svelte';
  import Settings from '$lib/components/Settings.svelte';
  import ErrorToast from '$lib/components/ErrorToast.svelte';
  import { ui, isMenuOpen, isSettingsOpen } from '$lib/stores/ui';
</script>

<div class="app-container">
  <header class="app-header">
    <MenuToggle onclick={() => ui.toggleMenu()} isOpen={$isMenuOpen} />
  </header>

  <main class="app-main">
    {#if $isSettingsOpen}
      <Settings onclose={() => ui.closeSettings()} />
    {:else}
      <Editor />
    {/if}
  </main>

  <MenuDrawer
    isOpen={$isMenuOpen}
    onclose={() => ui.closeMenu()}
    onOpenSettings={() => ui.openSettings()}
  />

  <ErrorToast />
</div>

<style>
  .app-container {
    display: flex;
    flex-direction: column;
    height: 100vh;
    background: var(--bg-primary);
  }

  .app-header {
    position: fixed;
    top: 0;
    left: 0;
    z-index: 100;
    padding: 8px;
  }

  .app-main {
    flex: 1;
    overflow: hidden;
    padding-top: 40px;
  }
</style>
```

**Phase 2 完了条件チェックリスト**:
- [ ] Tokyo Night テーマが適用される
- [ ] ハンバーガーメニューでドロワーが開閉する
- [ ] メモ一覧が表示される（空状態も含む）
- [ ] 設定画面が開ける
- [ ] 保存ディレクトリをフォルダピッカーで選択できる
- [ ] フォント設定が即座に反映される
- [ ] テーマ切り替えが動作する
- [ ] イベントバスが正しく動作する

---

## Phase 3: ライブプレビューエディタ

### 3.1 CodeMirror セットアップ

#### 3.1.1 依存関係

```bash
bun add @codemirror/state @codemirror/view @codemirror/commands @codemirror/language @codemirror/lang-markdown @lezer/markdown @lezer/highlight
```

#### 3.1.2 lib/editor/setup.ts

```typescript
import { EditorState, type Extension } from '@codemirror/state';
import {
  EditorView,
  keymap,
  lineNumbers,
  highlightActiveLine,
  drawSelection,
} from '@codemirror/view';
import { defaultKeymap, history, historyKeymap } from '@codemirror/commands';
import { markdown, markdownLanguage } from '@codemirror/lang-markdown';
import { syntaxHighlighting, HighlightStyle } from '@codemirror/language';
import { tags } from '@lezer/highlight';
import { livePreviewPlugin } from './extensions/livePreview';
import { appKeymap } from './extensions/keymaps';
import { tokyoNightTheme } from './themes/tokyoNight';
import type { Settings } from '$lib/stores/settings';

// Tokyo Night ハイライトスタイル
const highlightStyles = HighlightStyle.define([
  { tag: tags.heading1, color: '#bb9af7', fontWeight: 'bold', fontSize: '1.6em' },
  { tag: tags.heading2, color: '#7aa2f7', fontWeight: 'bold', fontSize: '1.4em' },
  { tag: tags.heading3, color: '#7dcfff', fontWeight: 'bold', fontSize: '1.2em' },
  { tag: tags.heading4, color: '#7dcfff', fontWeight: 'bold', fontSize: '1.1em' },
  { tag: tags.strong, color: '#ff9e64', fontWeight: 'bold' },
  { tag: tags.emphasis, color: '#c0caf5', fontStyle: 'italic' },
  { tag: tags.link, color: '#7aa2f7', textDecoration: 'underline' },
  { tag: tags.url, color: '#565f89' },
  { tag: tags.monospace, color: '#9ece6a' },
  { tag: tags.quote, color: '#565f89', fontStyle: 'italic' },
  { tag: tags.list, color: '#7dcfff' },
]);

export interface EditorConfig {
  settings: Settings;
  onChange: (content: string) => void;
}

export function createEditorExtensions(config: EditorConfig): Extension[] {
  const { settings, onChange } = config;

  return [
    lineNumbers(),
    highlightActiveLine(),
    drawSelection(),
    history(),
    markdown({ base: markdownLanguage }),
    syntaxHighlighting(highlightStyles),
    livePreviewPlugin(),
    tokyoNightTheme,
    keymap.of([...defaultKeymap, ...historyKeymap]),
    appKeymap,
    EditorView.updateListener.of((update) => {
      if (update.docChanged) {
        onChange(update.state.doc.toString());
      }
    }),
    EditorView.theme({
      '&': {
        height: '100%',
        fontSize: `${settings.editor.fontSize}px`,
        fontFamily: settings.editor.fontFamily,
      },
      '.cm-content': {
        lineHeight: String(settings.editor.lineHeight),
        padding: '16px 24px',
      },
      '.cm-line': {
        padding: '0 4px',
      },
      '.cm-scroller': {
        overflow: 'auto',
      },
    }),
  ];
}

export function createEditorState(doc: string, extensions: Extension[]): EditorState {
  return EditorState.create({ doc, extensions });
}
```

### 3.2 ライブプレビュープラグイン

#### 3.2.1 lib/editor/extensions/livePreview.ts

```typescript
import {
  ViewPlugin,
  Decoration,
  type DecorationSet,
  EditorView,
  type ViewUpdate,
} from '@codemirror/view';
import { syntaxTree } from '@codemirror/language';
import { RangeSetBuilder } from '@codemirror/state';

// デコレーションマーク
const boldMark = Decoration.mark({ class: 'cm-bold-content' });
const italicMark = Decoration.mark({ class: 'cm-italic-content' });
const codeMark = Decoration.mark({ class: 'cm-code-content' });
const linkMark = Decoration.mark({ class: 'cm-link-content' });

export function livePreviewPlugin() {
  return ViewPlugin.fromClass(
    class {
      decorations: DecorationSet;

      constructor(view: EditorView) {
        this.decorations = this.buildDecorations(view);
      }

      update(update: ViewUpdate) {
        if (update.docChanged || update.viewportChanged || update.selectionSet) {
          this.decorations = this.buildDecorations(update.view);
        }
      }

      buildDecorations(view: EditorView): DecorationSet {
        const builder = new RangeSetBuilder<Decoration>();
        const cursorPos = view.state.selection.main.head;

        syntaxTree(view.state).iterate({
          enter: (node) => {
            const { from, to } = node;
            const lineStart = view.state.doc.lineAt(from).from;
            const lineEnd = view.state.doc.lineAt(to).to;
            const cursorOnLine = cursorPos >= lineStart && cursorPos <= lineEnd;

            // カーソル行はMarkdown記法を表示（編集モード）
            if (cursorOnLine) return;

            switch (node.name) {
              case 'ATXHeading1':
              case 'ATXHeading2':
              case 'ATXHeading3':
              case 'ATXHeading4':
              case 'ATXHeading5':
              case 'ATXHeading6': {
                const level = parseInt(node.name.slice(-1));
                const hashEnd = from + level + 1;
                if (hashEnd <= to) {
                  builder.add(from, Math.min(hashEnd, to), Decoration.replace({}));
                }
                break;
              }

              case 'StrongEmphasis': {
                const text = view.state.sliceDoc(from, to);
                if (text.startsWith('**') && text.endsWith('**') && text.length > 4) {
                  builder.add(from, from + 2, Decoration.replace({}));
                  builder.add(to - 2, to, Decoration.replace({}));
                  builder.add(from + 2, to - 2, boldMark);
                }
                break;
              }

              case 'Emphasis': {
                const text = view.state.sliceDoc(from, to);
                if (text.length > 2 &&
                    (text.startsWith('*') || text.startsWith('_')) &&
                    (text.endsWith('*') || text.endsWith('_'))) {
                  builder.add(from, from + 1, Decoration.replace({}));
                  builder.add(to - 1, to, Decoration.replace({}));
                  builder.add(from + 1, to - 1, italicMark);
                }
                break;
              }

              case 'InlineCode': {
                const text = view.state.sliceDoc(from, to);
                if (text.startsWith('`') && text.endsWith('`') && text.length > 2) {
                  builder.add(from, from + 1, Decoration.replace({}));
                  builder.add(to - 1, to, Decoration.replace({}));
                  builder.add(from + 1, to - 1, codeMark);
                }
                break;
              }

              case 'Link': {
                const text = view.state.sliceDoc(from, to);
                const match = text.match(/^\[([^\]]+)\]\(([^)]+)\)$/);
                if (match) {
                  const textStart = from + 1;
                  const textEnd = from + 1 + match[1].length;
                  builder.add(from, textStart, Decoration.replace({}));
                  builder.add(textEnd, to, Decoration.replace({}));
                  builder.add(textStart, textEnd, linkMark);
                }
                break;
              }
            }
          },
        });

        return builder.finish();
      }
    },
    {
      decorations: (v) => v.decorations,
    }
  );
}
```

#### 3.2.2 lib/editor/extensions/keymaps.ts

```typescript
import { keymap } from '@codemirror/view';
import { eventBus, Events } from '$lib/events/eventBus';

export const appKeymap = keymap.of([
  {
    key: 'Mod-s',
    run: () => {
      eventBus.emit(Events.SAVE_REQUESTED);
      return true;
    },
  },
  {
    key: 'Mod-n',
    run: () => {
      document.dispatchEvent(new CustomEvent('new-note'));
      return true;
    },
  },
]);
```

### 3.3 エディタテーマ

#### 3.3.1 lib/editor/themes/tokyoNight.ts

```typescript
import { EditorView } from '@codemirror/view';

export const tokyoNightTheme = EditorView.theme(
  {
    '&': {
      backgroundColor: '#1a1b26',
      color: '#c0caf5',
    },
    '.cm-content': {
      caretColor: '#7aa2f7',
    },
    '.cm-cursor, .cm-dropCursor': {
      borderLeftColor: '#7aa2f7',
    },
    '&.cm-focused .cm-selectionBackground, .cm-selectionBackground, .cm-content ::selection': {
      backgroundColor: '#364a82',
    },
    '.cm-gutters': {
      backgroundColor: '#16161e',
      color: '#565f89',
      border: 'none',
    },
    '.cm-activeLineGutter': {
      backgroundColor: '#1f2335',
    },
    '.cm-activeLine': {
      backgroundColor: '#1e2030',
    },
    // ライブプレビュー用カスタムクラス
    '.cm-bold-content': {
      fontWeight: 'bold',
      color: '#ff9e64',
    },
    '.cm-italic-content': {
      fontStyle: 'italic',
    },
    '.cm-code-content': {
      backgroundColor: '#1f2335',
      padding: '1px 4px',
      borderRadius: '3px',
      fontFamily: 'monospace',
      color: '#9ece6a',
    },
    '.cm-link-content': {
      color: '#7aa2f7',
      textDecoration: 'underline',
      cursor: 'pointer',
    },
  },
  { dark: true }
);
```

### 3.4 Editor コンポーネント

#### 3.4.1 lib/components/Editor.svelte

```svelte
<script lang="ts">
  import { onMount, onDestroy } from 'svelte';
  import { EditorView } from '@codemirror/view';
  import { currentNote } from '$lib/stores/note';
  import { settings } from '$lib/stores/settings';
  import { eventBus, Events } from '$lib/events/eventBus';
  import { createEditorExtensions, createEditorState } from '$lib/editor/setup';
  import { createNewNote, saveNote } from '$lib/tauri/commands';

  let editorContainer: HTMLDivElement;
  let view: EditorView | null = null;

  onMount(async () => {
    // 新規メモを作成
    if (!$currentNote) {
      const data = await createNewNote();
      currentNote.setNew(data);
    }

    // エディタ作成
    const extensions = createEditorExtensions({
      settings: $settings,
      onChange: (content) => {
        currentNote.updateContent(content);
      },
    });

    const state = createEditorState($currentNote?.content ?? '', extensions);
    view = new EditorView({
      state,
      parent: editorContainer,
    });

    view.focus();
    eventBus.emit(Events.EDITOR_READY);

    // 保存リクエストハンドラ
    const unsubscribeSave = eventBus.on(Events.SAVE_REQUESTED, async () => {
      const note = currentNote.get();
      if (note?.isDirty) {
        try {
          const result = await saveNote({
            uid: note.uid,
            content: note.content,
            currentFilename: note.filename,
          });
          currentNote.markSaved(result.filename);
          eventBus.emit(Events.SAVE_COMPLETED, { uid: note.uid, filename: result.filename });
        } catch (error) {
          eventBus.emit(Events.SAVE_FAILED, { uid: note.uid, error: String(error) });
        }
      }
    });

    // フォーカスリクエストハンドラ
    const unsubscribeFocus = eventBus.on(Events.EDITOR_FOCUS, () => {
      view?.focus();
    });

    return () => {
      unsubscribeSave();
      unsubscribeFocus();
    };
  });

  onDestroy(() => {
    view?.destroy();
  });

  // メモが変更されたらエディタ内容を更新
  $effect(() => {
    if (view && $currentNote) {
      const currentContent = view.state.doc.toString();
      if (currentContent !== $currentNote.content) {
        view.dispatch({
          changes: {
            from: 0,
            to: currentContent.length,
            insert: $currentNote.content,
          },
        });
      }
    }
  });
</script>

<div class="editor-wrapper" bind:this={editorContainer}></div>

<style>
  .editor-wrapper {
    height: 100%;
    overflow: hidden;
  }

  :global(.cm-editor) {
    height: 100%;
    background: var(--editor-bg);
  }

  :global(.cm-gutters) {
    background: var(--bg-secondary);
    border-right: 1px solid var(--border-color);
  }

  :global(.cm-activeLineGutter) {
    background: var(--bg-highlight);
  }

  :global(.cm-activeLine) {
    background: var(--editor-line-highlight);
  }

  :global(.cm-selectionBackground) {
    background: var(--editor-selection) !important;
  }

  :global(.cm-cursor) {
    border-left-color: var(--editor-cursor);
  }
</style>
```

**Phase 3 完了条件チェックリスト**:
- [ ] CodeMirror 6 ベースのエディタが動作する
- [ ] 見出し（# ## ###）が大きく表示される
- [ ] 太字（**text**）が太字で表示される
- [ ] 斜体（*text*）が斜体で表示される
- [ ] インラインコード（`code`）が装飾される
- [ ] リンク（[text](url)）がリンクスタイルで表示される
- [ ] カーソル行はMarkdown記法が表示される（編集可能）
- [ ] カーソル外の行は装飾のみ表示される（プレビュー）
- [ ] 入力遅延が体感できない（< 16ms）

---

## Phase 4: Tauriコマンド実装・保存仕様

### 4.1 NoteService（アプリケーションサービス層）

#### 4.1.1 services/note_service.rs

```rust
use crate::domain::note::{Note, NoteMetadata};
use crate::domain::events::{NoteCreated, NoteUpdated, NoteDeleted, NoteBeforeSave};
use crate::traits::{
    repository::NoteRepository,
    event_bus::EventBus,
};
use std::sync::Arc;
use tracing::{debug, info};

/// メモサービス（ビジネスロジック層）
pub struct NoteService {
    repository: Arc<dyn NoteRepository>,
    event_bus: Arc<dyn EventBus>,
}

impl NoteService {
    pub fn new(
        repository: Arc<dyn NoteRepository>,
        event_bus: Arc<dyn EventBus>,
    ) -> Self {
        Self { repository, event_bus }
    }

    /// 新規メモを作成
    pub fn create_note(&self) -> Result<Note, String> {
        let note = Note::new();
        let filename = self.repository.create(&note).map_err(|e| e.to_string())?;

        self.event_bus.emit(&NoteCreated {
            uid: note.uid.clone(),
            filename: filename.clone(),
        });

        info!("Created new note: {}", note.uid);
        Ok(Note { filename: Some(filename), ..note })
    }

    /// メモを保存
    pub fn save_note(
        &self,
        uid: &str,
        content: &str,
        current_filename: Option<&str>,
    ) -> Result<String, String> {
        // 保存前イベント発火
        self.event_bus.emit(&NoteBeforeSave {
            uid: uid.to_string(),
            content: content.to_string(),
        });

        let mut note = self.repository
            .find_by_uid(uid)
            .map_err(|e| e.to_string())?
            .ok_or_else(|| format!("Note not found: {}", uid))?;

        let old_filename = note.filename.clone();
        note.update_content(content.to_string());
        note.refresh_timestamp();

        let new_filename = self.repository
            .update(&note, current_filename)
            .map_err(|e| e.to_string())?;

        self.event_bus.emit(&NoteUpdated {
            uid: uid.to_string(),
            filename: new_filename.clone(),
            old_filename,
        });

        debug!("Saved note: {} -> {}", uid, new_filename);
        Ok(new_filename)
    }

    /// メモを削除
    pub fn delete_note(&self, uid: &str) -> Result<(), String> {
        let note = self.repository
            .find_by_uid(uid)
            .map_err(|e| e.to_string())?
            .ok_or_else(|| format!("Note not found: {}", uid))?;

        let filename = note.filename.clone().unwrap_or_default();

        self.repository.delete(uid).map_err(|e| e.to_string())?;

        self.event_bus.emit(&NoteDeleted {
            uid: uid.to_string(),
            filename,
        });

        info!("Deleted note: {}", uid);
        Ok(())
    }

    /// メモ一覧を取得
    pub fn list_notes(&self) -> Result<Vec<NoteMetadata>, String> {
        self.repository.list_all().map_err(|e| e.to_string())
    }

    /// ファイル名でメモを取得
    pub fn get_note_by_filename(&self, filename: &str) -> Result<Option<Note>, String> {
        self.repository.find_by_filename(filename).map_err(|e| e.to_string())
    }
}
```

### 4.2 Tauriコマンド

#### 4.2.1 commands/note_commands.rs

```rust
use crate::app_state::AppContainer;
use crate::domain::note::NoteMetadata;
use crate::services::note_service::NoteService;
use serde::{Deserialize, Serialize};
use tauri::State;

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct NewNoteData {
    pub uid: String,
    pub content: String,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SaveNoteRequest {
    pub uid: String,
    pub content: String,
    pub current_filename: Option<String>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SaveNoteResponse {
    pub filename: String,
    pub path: String,
}

#[tauri::command]
pub async fn create_new_note(container: State<'_, AppContainer>) -> Result<NewNoteData, String> {
    let service = NoteService::new(
        container.repository.clone(),
        container.event_bus.clone(),
    );

    let note = service.create_note()?;

    Ok(NewNoteData {
        uid: note.uid,
        content: note.content,
        created_at: note.created_at.to_rfc3339(),
        updated_at: note.updated_at.to_rfc3339(),
    })
}

#[tauri::command]
pub async fn save_note(
    request: SaveNoteRequest,
    container: State<'_, AppContainer>,
) -> Result<SaveNoteResponse, String> {
    let service = NoteService::new(
        container.repository.clone(),
        container.event_bus.clone(),
    );

    let filename = service.save_note(
        &request.uid,
        &request.content,
        request.current_filename.as_deref(),
    )?;

    let settings = container.settings.lock();
    let path = std::path::PathBuf::from(&settings.storage.directory)
        .join(&filename)
        .to_string_lossy()
        .to_string();

    Ok(SaveNoteResponse { filename, path })
}

#[tauri::command]
pub async fn get_note_list(container: State<'_, AppContainer>) -> Result<Vec<NoteMetadata>, String> {
    let service = NoteService::new(
        container.repository.clone(),
        container.event_bus.clone(),
    );

    service.list_notes()
}

#[tauri::command]
pub async fn load_note_by_filename(
    filename: String,
    container: State<'_, AppContainer>,
) -> Result<String, String> {
    let service = NoteService::new(
        container.repository.clone(),
        container.event_bus.clone(),
    );

    let note = service.get_note_by_filename(&filename)?
        .ok_or_else(|| format!("Note not found: {}", filename))?;

    Ok(note.content)
}

#[tauri::command]
pub async fn delete_note(
    filename: String,
    container: State<'_, AppContainer>,
) -> Result<(), String> {
    let service = NoteService::new(
        container.repository.clone(),
        container.event_bus.clone(),
    );

    // ファイル名からuidを取得
    let note = service.get_note_by_filename(&filename)?
        .ok_or_else(|| format!("Note not found: {}", filename))?;

    service.delete_note(&note.uid)
}
```

#### 4.2.2 commands/window_commands.rs

```rust
use crate::app_state::AppStateManager;
use tauri::{State, WebviewWindow};

#[tauri::command]
pub async fn hide_window_after_save(
    window: WebviewWindow,
    state: State<'_, AppStateManager>,
    success: bool,
) -> Result<(), String> {
    state.complete_save(success);

    if success {
        window.hide().map_err(|e| e.to_string())?;
    }

    Ok(())
}
```

**Phase 4 完了条件チェックリスト**:
- [ ] 新規メモにFront Matter（uid, created_at, updated_at）が自動挿入される
- [ ] H1/H2があればその内容がファイル名になる
- [ ] H1/H2がなければuidがファイル名になる
- [ ] ファイル名に使えない文字がサニタイズされる
- [ ] 同名ファイルがある場合は連番が付与される
- [ ] 保存はアトミック（tmp → rename）で行われる
- [ ] ホットキー再押下時、保存完了するまで非表示にならない
- [ ] ×ボタン押下時、保存完了するまで非表示にならない
- [ ] 保存失敗時、エラーが表示され画面が維持される
- [ ] Ctrl+S で手動保存ができる

---

## Phase 5: 堅牢性・快適性向上

### 5.1 自動保存サービス

#### 5.1.1 services/autosave_service.ts

```typescript
import { get } from 'svelte/store';
import { currentNote } from '$lib/stores/note';
import { settings } from '$lib/stores/settings';
import { saveNote } from '$lib/tauri/commands';
import { eventBus, Events } from '$lib/events/eventBus';

class AutosaveService {
  private timer: ReturnType<typeof setTimeout> | null = null;
  private isEnabled = true;

  /**
   * 自動保存をスケジュール
   */
  schedule(): void {
    if (!this.isEnabled) return;

    this.cancel();

    const { autosave } = get(settings);
    if (!autosave.enabled) return;

    this.timer = setTimeout(async () => {
      await this.save();
    }, autosave.delayMs);
  }

  /**
   * スケジュールをキャンセル
   */
  cancel(): void {
    if (this.timer) {
      clearTimeout(this.timer);
      this.timer = null;
    }
  }

  /**
   * 即座に保存
   */
  async save(): Promise<boolean> {
    const note = get(currentNote);
    if (!note?.isDirty) return true;

    try {
      const result = await saveNote({
        uid: note.uid,
        content: note.content,
        currentFilename: note.filename,
      });

      currentNote.markSaved(result.filename);
      eventBus.emit(Events.SAVE_COMPLETED, { uid: note.uid, filename: result.filename });
      return true;
    } catch (error) {
      console.warn('Autosave failed:', error);
      // 自動保存の失敗は静かに処理（次の機会を待つ）
      return false;
    }
  }

  /**
   * 有効/無効を設定
   */
  setEnabled(enabled: boolean): void {
    this.isEnabled = enabled;
    if (!enabled) {
      this.cancel();
    }
  }

  /**
   * クリーンアップ
   */
  destroy(): void {
    this.cancel();
  }
}

export const autosaveService = new AutosaveService();

// コンテンツ変更時に自動保存をスケジュール
eventBus.on(Events.NOTE_CONTENT_CHANGED, () => {
  autosaveService.schedule();
});
```

### 5.2 ファイル監視

#### 5.2.1 platform/file_watcher.rs

```rust
use notify::{Config, Event, EventKind, RecommendedWatcher, RecursiveMode, Watcher};
use std::path::PathBuf;
use std::sync::mpsc;
use std::time::{Duration, Instant};
use tauri::AppHandle;
use tracing::{debug, error, info};

pub struct FileWatcherHandle {
    _watcher: RecommendedWatcher,
}

pub fn start_file_watcher(
    app: AppHandle,
    directory: PathBuf,
) -> Result<FileWatcherHandle, notify::Error> {
    let (tx, rx) = mpsc::channel();

    let mut watcher = RecommendedWatcher::new(
        move |res: Result<Event, notify::Error>| {
            if let Ok(event) = res {
                let _ = tx.send(event);
            }
        },
        Config::default().with_poll_interval(Duration::from_secs(1)),
    )?;

    watcher.watch(&directory, RecursiveMode::NonRecursive)?;

    // イベント処理スレッド
    let app_handle = app.clone();
    std::thread::spawn(move || {
        let mut last_notify = Instant::now();
        const DEBOUNCE_MS: u64 = 500;

        while let Ok(event) = rx.recv() {
            match event.kind {
                EventKind::Create(_) | EventKind::Modify(_) | EventKind::Remove(_) => {
                    // デバウンス
                    if last_notify.elapsed() > Duration::from_millis(DEBOUNCE_MS) {
                        last_notify = Instant::now();
                        debug!("File change detected, notifying frontend");

                        if let Some(window) = app_handle.get_webview_window("main") {
                            let _ = window.emit("notes-changed", ());
                        }
                    }
                }
                _ => {}
            }
        }
    });

    info!("File watcher started for {:?}", directory);
    Ok(FileWatcherHandle { _watcher: watcher })
}
```

### 5.3 ウィンドウジオメトリ永続化

#### 5.3.1 platform/window_geometry.rs

```rust
use crate::domain::settings::{Settings, WindowSettings};
use parking_lot::Mutex;
use std::sync::Arc;
use std::time::{Duration, Instant};
use tauri::{PhysicalPosition, PhysicalSize, WebviewWindow};
use tracing::debug;

pub struct GeometryWatcher {
    last_change: Mutex<Instant>,
    pending: Mutex<Option<WindowSettings>>,
}

impl GeometryWatcher {
    pub fn new() -> Self {
        Self {
            last_change: Mutex::new(Instant::now()),
            pending: Mutex::new(None),
        }
    }

    /// ジオメトリ変更を記録
    pub fn on_geometry_change(&self, window: &WebviewWindow) {
        let position = window.outer_position().ok();
        let size = window.outer_size().ok();
        let is_maximized = window.is_maximized().unwrap_or(false);

        if let (Some(pos), Some(sz)) = (position, size) {
            let settings = WindowSettings {
                x: Some(pos.x),
                y: Some(pos.y),
                width: sz.width,
                height: sz.height,
                is_maximized,
                monitor_id: None,
            };

            *self.pending.lock() = Some(settings);
            *self.last_change.lock() = Instant::now();
        }
    }

    /// アイドル時に保存（500ms以上経過）
    pub fn flush_if_idle(&self, settings: &mut Settings) -> bool {
        let last = *self.last_change.lock();
        if last.elapsed() > Duration::from_millis(500) {
            if let Some(geometry) = self.pending.lock().take() {
                settings.window = geometry;
                return true;
            }
        }
        false
    }
}

/// 起動時にジオメトリを復元
pub fn restore_geometry(window: &WebviewWindow, settings: &WindowSettings) -> Result<(), String> {
    // 位置が有効か検証
    if let (Some(x), Some(y)) = (settings.x, settings.y) {
        if is_position_visible(x, y, settings.width, settings.height, window) {
            window
                .set_position(PhysicalPosition::new(x, y))
                .map_err(|e| e.to_string())?;
        } else {
            // 画面外の場合は中央に配置
            window.center().map_err(|e| e.to_string())?;
        }
    } else {
        window.center().map_err(|e| e.to_string())?;
    }

    window
        .set_size(PhysicalSize::new(settings.width, settings.height))
        .map_err(|e| e.to_string())?;

    if settings.is_maximized {
        window.maximize().map_err(|e| e.to_string())?;
    }

    debug!("Restored window geometry");
    Ok(())
}

fn is_position_visible(x: i32, y: i32, width: u32, height: u32, window: &WebviewWindow) -> bool {
    let monitors = match window.available_monitors() {
        Ok(m) => m,
        Err(_) => return true, // モニター情報が取れない場合はOKとする
    };

    let min_visible = 100i32;

    for monitor in monitors {
        let pos = monitor.position();
        let size = monitor.size();

        // ウィンドウの少なくとも一部がモニター内にあるか
        let monitor_right = pos.x + size.width as i32;
        let monitor_bottom = pos.y + size.height as i32;
        let window_right = x + width as i32;
        let window_bottom = y + height as i32;

        let overlap_x = (x < monitor_right) && (window_right > pos.x);
        let overlap_y = (y < monitor_bottom) && (window_bottom > pos.y);

        if overlap_x && overlap_y {
            // 最低限の可視領域があるか
            let visible_width = window_right.min(monitor_right) - x.max(pos.x);
            let visible_height = window_bottom.min(monitor_bottom) - y.max(pos.y);

            if visible_width >= min_visible && visible_height >= min_visible {
                return true;
            }
        }
    }

    false
}
```

### 5.4 一覧検索・キーボードショートカット

#### 5.4.1 lib/components/SearchableNoteList.svelte

```svelte
<script lang="ts">
  import NoteListItem from './NoteListItem.svelte';
  import type { NoteMetadata } from '$lib/stores/notes';

  interface Props {
    notes: NoteMetadata[];
    onselect: (uid: string) => void;
  }

  let { notes, onselect }: Props = $props();
  let searchQuery = $state('');

  let filteredNotes = $derived(
    searchQuery.trim()
      ? notes.filter(note =>
          note.displayName.toLowerCase().includes(searchQuery.toLowerCase())
        )
      : notes
  );
</script>

<div class="searchable-list">
  <div class="search-box">
    <svg width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
      <circle cx="11" cy="11" r="8"/>
      <line x1="21" y1="21" x2="16.65" y2="16.65"/>
    </svg>
    <input
      type="text"
      placeholder="検索..."
      bind:value={searchQuery}
    />
    {#if searchQuery}
      <button class="clear-btn" onclick={() => searchQuery = ''}>
        <svg width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
          <line x1="18" y1="6" x2="6" y2="18"/>
          <line x1="6" y1="6" x2="18" y2="18"/>
        </svg>
      </button>
    {/if}
  </div>

  <div class="note-list">
    {#if filteredNotes.length === 0}
      <div class="empty-state">
        {#if searchQuery}
          <p>検索結果がありません</p>
        {:else}
          <p>メモがありません</p>
          <p class="hint">新しいメモを作成しましょう</p>
        {/if}
      </div>
    {:else}
      {#each filteredNotes as note (note.uid)}
        <NoteListItem {note} onclick={() => onselect(note.uid)} />
      {/each}
    {/if}
  </div>
</div>

<style>
  .searchable-list {
    display: flex;
    flex-direction: column;
    height: 100%;
  }

  .search-box {
    display: flex;
    align-items: center;
    gap: 8px;
    padding: 8px 12px;
    margin: 8px;
    background: var(--bg-tertiary);
    border-radius: 6px;
  }

  .search-box svg {
    color: var(--fg-muted);
    flex-shrink: 0;
  }

  .search-box input {
    flex: 1;
    background: transparent;
    border: none;
    outline: none;
    color: var(--fg-primary);
    font-size: 14px;
  }

  .search-box input::placeholder {
    color: var(--fg-muted);
  }

  .clear-btn {
    display: flex;
    padding: 2px;
    background: transparent;
    border: none;
    color: var(--fg-muted);
    cursor: pointer;
  }

  .clear-btn:hover {
    color: var(--fg-primary);
  }

  .note-list {
    flex: 1;
    overflow-y: auto;
    padding: 0 8px;
  }

  .empty-state {
    padding: 24px 16px;
    text-align: center;
    color: var(--fg-muted);
  }

  .hint {
    font-size: 12px;
    margin-top: 4px;
  }
</style>
```

**Phase 5 完了条件チェックリスト**:
- [ ] 入力停止後2秒で自動保存される
- [ ] 外部でファイルが変更されると一覧が更新される
- [ ] 検索ボックスでメモをフィルタできる
- [ ] Ctrl+S で即座に保存できる
- [ ] Ctrl+N で新規メモを作成できる
- [ ] ウィンドウ位置・サイズが保存される
- [ ] 次回起動時に同じ位置・サイズで開く

---

## Phase 6: ポリッシュ・最終調整

### 6.1 起動時の挙動

- 起動時は新規メモで開始
- 前回開いていたメモを復元する（オプション、設定で切り替え可能）

### 6.2 メモ削除機能

確認ダイアログ付きの削除機能を実装。

### 6.3 エラーハンドリング強化

- すべてのRustコマンドにResult型を使用
- フロントエンドでのエラー境界
- 予期しないエラー時のリカバリ

### 6.4 アクセシビリティ

- キーボードナビゲーション対応
- スクリーンリーダー対応（aria属性）
- フォーカス管理

### 6.5 パフォーマンス最適化

- 大きなメモファイル（10MB+）の遅延読み込み
- 一覧の仮想スクロール（1000件以上の場合）
- メモリ使用量の監視

**Phase 6 完了条件チェックリスト**:
- [ ] 起動時に適切な状態で開始する
- [ ] メモを削除できる
- [ ] 予期しないエラーでクラッシュしない
- [ ] キーボードのみで操作できる
- [ ] 1000件のメモでも快適に動作する

---

## Phase 7: プラグインシステム（拡張可能性）

### 7.1 プラグインアーキテクチャ概要

```
プラグイン構造:
~/.config/z_memo/plugins/
├── my-plugin/
│   ├── plugin.toml          # マニフェスト
│   ├── main.js              # エントリポイント
│   └── styles.css           # オプション: カスタムCSS
```

### 7.2 プラグインマニフェスト

#### 7.2.1 plugin/manifest.rs

```rust
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PluginManifest {
    pub plugin: PluginInfo,
    pub permissions: PluginPermissions,
    pub hooks: Option<PluginHooks>,
    pub settings: Option<HashMap<String, SettingDefinition>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PluginInfo {
    pub id: String,
    pub name: String,
    pub version: String,
    pub description: Option<String>,
    pub author: Option<String>,
    pub api_version: String,  // z_memo API バージョン
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct PluginPermissions {
    #[serde(default)]
    pub filesystem: Vec<String>,  // "read", "write"
    #[serde(default)]
    pub network: bool,
    #[serde(default)]
    pub clipboard: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PluginHooks {
    pub on_note_save: Option<String>,
    pub on_note_open: Option<String>,
    pub on_app_ready: Option<String>,
    pub editor_extensions: Option<Vec<String>>,
    pub sidebar_panels: Option<Vec<String>>,
    pub settings_panels: Option<Vec<String>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SettingDefinition {
    pub r#type: String,  // "string", "number", "boolean", "select"
    pub default: serde_json::Value,
    pub label: String,
    pub description: Option<String>,
    pub options: Option<Vec<String>>,  // for "select" type
}
```

### 7.3 プラグインローダー

#### 7.3.1 plugin/loader.rs

```rust
use crate::plugin::manifest::PluginManifest;
use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};
use thiserror::Error;
use tracing::{debug, error, info, warn};

#[derive(Error, Debug)]
pub enum PluginError {
    #[error("Invalid manifest: {0}")]
    InvalidManifest(String),
    #[error("Incompatible API version: plugin requires {0}, app provides {1}")]
    IncompatibleApiVersion(String, String),
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),
    #[error("TOML parse error: {0}")]
    TomlParse(#[from] toml::de::Error),
}

pub struct PluginLoader {
    plugins_dir: PathBuf,
    loaded_plugins: HashMap<String, LoadedPlugin>,
    api_version: String,
}

pub struct LoadedPlugin {
    pub manifest: PluginManifest,
    pub path: PathBuf,
    pub enabled: bool,
}

impl PluginLoader {
    pub fn new(plugins_dir: PathBuf) -> Self {
        Self {
            plugins_dir,
            loaded_plugins: HashMap::new(),
            api_version: "1.0".to_string(),
        }
    }

    /// 全プラグインをスキャン・読み込み
    pub fn load_all(&mut self) -> Result<(), PluginError> {
        if !self.plugins_dir.exists() {
            fs::create_dir_all(&self.plugins_dir)?;
            return Ok(());
        }

        for entry in fs::read_dir(&self.plugins_dir)? {
            let entry = entry?;
            let path = entry.path();

            if path.is_dir() {
                match self.load_plugin(&path) {
                    Ok(plugin) => {
                        info!("Loaded plugin: {} v{}", plugin.manifest.plugin.name, plugin.manifest.plugin.version);
                        self.loaded_plugins.insert(plugin.manifest.plugin.id.clone(), plugin);
                    }
                    Err(e) => {
                        warn!("Failed to load plugin from {:?}: {}", path, e);
                    }
                }
            }
        }

        Ok(())
    }

    /// 単一プラグインを読み込み
    fn load_plugin(&self, path: &Path) -> Result<LoadedPlugin, PluginError> {
        let manifest_path = path.join("plugin.toml");
        let content = fs::read_to_string(&manifest_path)?;
        let manifest: PluginManifest = toml::from_str(&content)?;

        // APIバージョン互換性チェック
        if !self.is_api_compatible(&manifest.plugin.api_version) {
            return Err(PluginError::IncompatibleApiVersion(
                manifest.plugin.api_version.clone(),
                self.api_version.clone(),
            ));
        }

        Ok(LoadedPlugin {
            manifest,
            path: path.to_path_buf(),
            enabled: true,
        })
    }

    fn is_api_compatible(&self, required: &str) -> bool {
        // セマンティックバージョニングでメジャーバージョンをチェック
        let required_major: u32 = required.split('.').next()
            .and_then(|s| s.parse().ok())
            .unwrap_or(0);
        let api_major: u32 = self.api_version.split('.').next()
            .and_then(|s| s.parse().ok())
            .unwrap_or(0);

        required_major == api_major
    }

    /// 有効なプラグイン一覧を取得
    pub fn get_enabled_plugins(&self) -> Vec<&LoadedPlugin> {
        self.loaded_plugins.values().filter(|p| p.enabled).collect()
    }

    /// プラグインを有効化
    pub fn enable(&mut self, id: &str) -> bool {
        if let Some(plugin) = self.loaded_plugins.get_mut(id) {
            plugin.enabled = true;
            true
        } else {
            false
        }
    }

    /// プラグインを無効化
    pub fn disable(&mut self, id: &str) -> bool {
        if let Some(plugin) = self.loaded_plugins.get_mut(id) {
            plugin.enabled = false;
            true
        } else {
            false
        }
    }
}
```

### 7.4 プラグインAPI（フロントエンド）

#### 7.4.1 lib/plugin/api.ts

```typescript
import { eventBus, Events } from '$lib/events/eventBus';
import { currentNote } from '$lib/stores/note';
import { notes } from '$lib/stores/notes';
import { settings } from '$lib/stores/settings';
import { ui } from '$lib/stores/ui';
import { get } from 'svelte/store';

/**
 * プラグインに公開するAPI
 */
export interface ZMemoPluginAPI {
  // バージョン情報
  version: string;

  // Note操作
  notes: {
    getCurrent(): NoteData | null;
    getList(): NoteMetadata[];
    onContentChange(callback: (content: string) => void): () => void;
    onSave(callback: (note: NoteData) => void): () => void;
  };

  // UI操作
  ui: {
    showToast(message: string, type?: 'info' | 'error' | 'success'): void;
    showModal(options: ModalOptions): Promise<boolean>;
  };

  // 設定
  settings: {
    get<T>(key: string, defaultValue: T): T;
    set<T>(key: string, value: T): void;
    onChange(callback: (settings: PluginSettings) => void): () => void;
  };

  // イベント
  events: {
    on(event: string, callback: (...args: any[]) => void): () => void;
    emit(event: string, ...args: any[]): void;
  };

  // エディタ拡張
  editor: {
    registerExtension(extension: EditorExtension): void;
    insertText(text: string): void;
    getSelection(): string;
  };
}

interface NoteData {
  uid: string;
  content: string;
  filename: string | null;
}

interface NoteMetadata {
  uid: string;
  displayName: string;
  filename: string;
}

interface ModalOptions {
  title: string;
  message: string;
  confirmText?: string;
  cancelText?: string;
}

interface PluginSettings {
  [key: string]: unknown;
}

interface EditorExtension {
  name: string;
  extension: any; // CodeMirror Extension
}

/**
 * プラグインAPIを作成
 */
export function createPluginAPI(pluginId: string): ZMemoPluginAPI {
  const settingsNamespace = `plugin.${pluginId}`;

  return {
    version: '1.0',

    notes: {
      getCurrent() {
        const note = get(currentNote);
        if (!note) return null;
        return {
          uid: note.uid,
          content: note.content,
          filename: note.filename,
        };
      },

      getList() {
        return get(notes);
      },

      onContentChange(callback) {
        return eventBus.on(Events.NOTE_CONTENT_CHANGED, (data: any) => {
          callback(data.content);
        });
      },

      onSave(callback) {
        return eventBus.on(Events.SAVE_COMPLETED, (data: any) => {
          const note = get(currentNote);
          if (note) {
            callback({
              uid: note.uid,
              content: note.content,
              filename: data.filename,
            });
          }
        });
      },
    },

    ui: {
      showToast(message, type = 'info') {
        if (type === 'error') {
          ui.showError(message);
        } else {
          // TODO: 他のタイプのトースト実装
          console.log(`[${type}] ${message}`);
        }
      },

      async showModal(options) {
        return confirm(`${options.title}\n\n${options.message}`);
      },
    },

    settings: {
      get<T>(key: string, defaultValue: T): T {
        const allSettings = get(settings) as any;
        const pluginSettings = allSettings[settingsNamespace] || {};
        return pluginSettings[key] ?? defaultValue;
      },

      set<T>(key: string, value: T): void {
        settings.update(s => {
          const pluginSettings = (s as any)[settingsNamespace] || {};
          return {
            ...s,
            [settingsNamespace]: {
              ...pluginSettings,
              [key]: value,
            },
          };
        });
      },

      onChange(callback) {
        return settings.subscribe(s => {
          const pluginSettings = (s as any)[settingsNamespace] || {};
          callback(pluginSettings);
        });
      },
    },

    events: {
      on(event, callback) {
        return eventBus.on(`plugin:${pluginId}:${event}`, callback);
      },

      emit(event, ...args) {
        eventBus.emit(`plugin:${pluginId}:${event}`, ...args);
      },
    },

    editor: {
      registerExtension(extension) {
        eventBus.emit('editor:register_extension', extension);
      },

      insertText(text) {
        eventBus.emit('editor:insert_text', text);
      },

      getSelection() {
        // TODO: エディタから選択テキストを取得
        return '';
      },
    },
  };
}
```

### 7.5 プラグイン用UIスロット

#### 7.5.1 lib/components/slots/SidebarSlot.svelte

```svelte
<script lang="ts">
  import { pluginPanels } from '$lib/stores/plugins';

  // プラグインが登録したサイドバーパネルを表示
</script>

<div class="sidebar-slot">
  {#each $pluginPanels.sidebar as panel (panel.id)}
    <div class="plugin-panel">
      <h3>{panel.title}</h3>
      <div class="panel-content">
        <!-- プラグインがレンダリングしたコンテンツ -->
        {@html panel.content}
      </div>
    </div>
  {/each}
</div>

<style>
  .sidebar-slot {
    display: flex;
    flex-direction: column;
    gap: 8px;
  }

  .plugin-panel {
    padding: 12px;
    background: var(--bg-tertiary);
    border-radius: 6px;
  }

  .plugin-panel h3 {
    font-size: 12px;
    font-weight: 600;
    color: var(--fg-muted);
    margin-bottom: 8px;
    text-transform: uppercase;
  }
</style>
```

### 7.6 拡張ポイント一覧

| 拡張ポイント | 説明 | 登録方法 |
|-------------|------|---------|
| `editor.extensions` | CodeMirror拡張 | `api.editor.registerExtension()` |
| `sidebar.panels` | サイドバーパネル | マニフェストで定義 |
| `settings.panels` | 設定画面パネル | マニフェストで定義 |
| `hooks.on_note_save` | 保存前/後フック | マニフェストで定義 |
| `hooks.on_note_open` | メモ開くフック | マニフェストで定義 |
| `commands` | カスタムコマンド | `api.commands.register()` |
| `themes` | カスタムテーマ | マニフェストで定義 |

**Phase 7 完了条件チェックリスト**:
- [ ] プラグインマニフェストの読み込みが動作する
- [ ] プラグインAPI がフロントエンドで利用可能
- [ ] エディタ拡張を登録できる
- [ ] サイドバーパネルを追加できる
- [ ] 設定画面にプラグイン設定が表示される
- [ ] プラグインの有効/無効を切り替えられる

---

## テスト戦略

### 単体テスト（Rust）

```rust
#[cfg(test)]
mod tests {
    use super::*;
    use mockall::predicate::*;
    use tempfile::tempdir;

    // Storage モック
    mock! {
        Storage {}
        impl Storage for Storage {
            fn save_atomic(&self, path: &Path, content: &str) -> Result<(), StorageError>;
            fn load(&self, path: &Path) -> Result<String, StorageError>;
            fn delete(&self, path: &Path) -> Result<(), StorageError>;
            fn exists(&self, path: &Path) -> bool;
            fn list_files(&self, dir: &Path, ext: &str) -> Result<Vec<PathBuf>, StorageError>;
        }
    }

    #[test]
    fn test_note_service_create() {
        let mut mock_storage = MockStorage::new();
        mock_storage.expect_save_atomic().returning(|_, _| Ok(()));
        mock_storage.expect_exists().returning(|_| false);

        // ... テスト実装
    }

    #[test]
    fn test_filename_generation() {
        let strategy = HeadingFilenameStrategy::new();

        assert_eq!(
            strategy.generate("---\nuid: test\n---\n\n# タイトル", "uid123"),
            "タイトル.md"
        );

        assert_eq!(
            strategy.generate("---\nuid: test\n---\n\n本文のみ", "uid123"),
            "uid123.md"
        );
    }

    #[test]
    fn test_sanitize_unicode() {
        let strategy = HeadingFilenameStrategy::new();
        let long_name = "あ".repeat(250);
        let result = strategy.sanitize(&long_name);

        assert!(result.chars().count() <= 200);
        assert!(result.ends_with("..."));
    }
}
```

### 統合テスト

```rust
#[cfg(test)]
mod integration_tests {
    use super::*;
    use tempfile::tempdir;

    #[test]
    fn test_full_note_lifecycle() {
        let dir = tempdir().unwrap();
        let container = AppContainer::test_with_dir(dir.path());

        let service = NoteService::new(
            container.repository.clone(),
            container.event_bus.clone(),
        );

        // 作成
        let note = service.create_note().unwrap();
        assert!(!note.uid.is_empty());

        // 保存
        let filename = service.save_note(&note.uid, "# テスト", None).unwrap();
        assert_eq!(filename, "テスト.md");

        // 読み込み
        let loaded = service.get_note_by_filename(&filename).unwrap().unwrap();
        assert_eq!(loaded.uid, note.uid);

        // 削除
        service.delete_note(&note.uid).unwrap();
        assert!(service.get_note_by_filename(&filename).unwrap().is_none());
    }
}
```

### E2Eテスト（Playwright）

```typescript
// tests/e2e/basic.spec.ts
import { test, expect } from '@playwright/test';

test.describe('z_memo', () => {
  test('新規メモを作成して保存', async ({ page }) => {
    // エディタにフォーカス
    await page.locator('.cm-editor').click();

    // テキスト入力
    await page.keyboard.type('# テストメモ\n\n本文です。');

    // Ctrl+S で保存
    await page.keyboard.press('Control+s');

    // 保存完了を待機
    await expect(page.locator('.save-indicator')).toHaveText('保存完了');
  });

  test('メモ一覧から選択して開く', async ({ page }) => {
    // メニューを開く
    await page.locator('.menu-toggle').click();

    // メモを選択
    await page.locator('.note-item').first().click();

    // エディタに内容が表示される
    await expect(page.locator('.cm-content')).not.toBeEmpty();
  });
});
```

---

## リリースチェックリスト

### v0.1.0（MVP）
- [ ] Phase 0-4 完了
- [ ] Windows/macOS/Linux ビルド確認
- [ ] 基本機能の動作確認
- [ ] README更新

### v0.2.0（安定版）
- [ ] Phase 5 完了
- [ ] パフォーマンステスト合格
- [ ] ユーザーテストフィードバック反映

### v1.0.0（正式版）
- [ ] Phase 6 完了
- [ ] 全機能の単体テスト
- [ ] E2Eテスト
- [ ] ドキュメント完備
- [ ] 自動アップデート機能（オプション）

### v2.0.0（プラグイン対応）
- [ ] Phase 7 完了
- [ ] プラグインAPI安定化
- [ ] サンプルプラグイン作成
- [ ] プラグイン開発ドキュメント

---

## 設定ファイル完全仕様

```toml
# ~/.config/z_memo/config.toml

[window]
x = 100                    # ウィンドウX座標（ピクセル）
y = 100                    # ウィンドウY座標（ピクセル）
width = 800                # ウィンドウ幅
height = 600               # ウィンドウ高さ
is_maximized = false       # 最大化状態
monitor_id = "primary"     # モニターID（マルチモニター用）

[storage]
directory = "~/Documents/z_memo"  # 保存ディレクトリ

[editor]
font_family = "system-ui"  # フォントファミリ
font_size = 14             # フォントサイズ（px）
line_height = 1.6          # 行間

[theme]
name = "tokyo-night"       # テーマ名: "tokyo-night" | "light"

[hotkey]
toggle = "Ctrl+Shift+Space"  # トグルホットキー

[autosave]
enabled = true             # 自動保存有効
delay_ms = 2000            # 自動保存遅延（ミリ秒）

[behavior]
restore_last_note = false  # 起動時に前回のメモを復元

# プラグイン設定（namespace分離）
[plugin.my-plugin]
custom_setting = "value"
```

---

## 設計図: アーキテクチャ概要

```
┌─────────────────────────────────────────────────────────────────┐
│                        Presentation Layer                        │
│  ┌─────────────┐  ┌─────────────┐  ┌─────────────────────────┐  │
│  │   Svelte    │  │  Components │  │   Plugin UI Slots       │  │
│  │  Components │  │  (Editor,   │  │   (Sidebar, Settings)   │  │
│  │             │  │   Menu...)  │  │                         │  │
│  └─────────────┘  └─────────────┘  └─────────────────────────┘  │
├─────────────────────────────────────────────────────────────────┤
│                        Application Layer                         │
│  ┌─────────────┐  ┌─────────────┐  ┌─────────────────────────┐  │
│  │   Stores    │  │  Services   │  │   EventBus              │  │
│  │  (note,     │  │  (note,     │  │   (Observer Pattern)    │  │
│  │   settings) │  │   autosave) │  │                         │  │
│  └─────────────┘  └─────────────┘  └─────────────────────────┘  │
├─────────────────────────────────────────────────────────────────┤
│                         Domain Layer                             │
│  ┌─────────────┐  ┌─────────────┐  ┌─────────────────────────┐  │
│  │   Note      │  │  Settings   │  │   Domain Events         │  │
│  │  (Entity)   │  │  (Entity)   │  │   (NoteCreated, etc.)   │  │
│  └─────────────┘  └─────────────┘  └─────────────────────────┘  │
├─────────────────────────────────────────────────────────────────┤
│                      Infrastructure Layer                        │
│  ┌─────────────┐  ┌─────────────┐  ┌─────────────────────────┐  │
│  │  Storage    │  │ Repository  │  │   Tauri Commands        │  │
│  │  (File I/O) │  │  (CRUD)     │  │   (IPC Bridge)          │  │
│  └─────────────┘  └─────────────┘  └─────────────────────────┘  │
├─────────────────────────────────────────────────────────────────┤
│                        Plugin System                             │
│  ┌─────────────┐  ┌─────────────┐  ┌─────────────────────────┐  │
│  │   Loader    │  │    API      │  │   Extension Registry    │  │
│  │             │  │             │  │                         │  │
│  └─────────────┘  └─────────────┘  └─────────────────────────┘  │
└─────────────────────────────────────────────────────────────────┘
```
