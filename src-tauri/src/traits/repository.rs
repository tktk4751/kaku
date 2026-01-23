use crate::domain::Note;
use crate::infrastructure::GalleryNote;
use std::path::PathBuf;
use thiserror::Error;

/// ノートリポジトリ抽象化（Repositoryパターン）
pub trait NoteRepository: Send + Sync {
    /// メモを保存
    fn save(&self, note: &Note) -> Result<PathBuf, RepositoryError>;

    /// メモをロード
    fn load(&self, uid: &str) -> Result<Note, RepositoryError>;

    /// メモを削除
    fn delete(&self, uid: &str) -> Result<(), RepositoryError>;

    /// 全メモの一覧を取得（メタデータのみ）
    fn list_all(&self) -> Result<Vec<NoteListItem>, RepositoryError>;

    /// UIDからファイルパスを取得
    fn get_path(&self, uid: &str) -> Option<PathBuf>;

    /// ギャラリー用ノート一覧を取得（高速キャッシュ版）
    fn list_gallery(
        &self,
        sort_by_created: bool,
        tag_filter: Option<&str>,
    ) -> Result<Vec<GalleryNote>, RepositoryError>;
}

/// メモ一覧アイテム
#[derive(Debug, Clone)]
pub struct NoteListItem {
    pub uid: String,
    pub title: String,
    pub path: PathBuf,
    pub updated_at: chrono::DateTime<chrono::Utc>,
}

/// リポジトリエラー
///
/// # エラーコンテキスト
///
/// 各バリアントは操作のコンテキスト情報を含み、
/// デバッグやエラーレポートに役立ちます。
#[derive(Debug, Error)]
pub enum RepositoryError {
    /// 指定されたUIDのメモが見つからない
    #[error("メモが見つかりません: uid={uid}")]
    NotFound {
        uid: String,
    },
    /// ストレージ層でのエラー
    #[error("ストレージエラー: {context} - {source}")]
    Storage {
        context: String,
        #[source]
        source: crate::traits::StorageError,
    },
    /// ファイル内容のパースエラー
    #[error("パースエラー: {context} (path={path:?})")]
    Parse {
        context: String,
        path: Option<PathBuf>,
    },
    /// ファイル名生成の失敗
    #[error("ファイル名生成エラー: {reason}")]
    FilenameGeneration {
        reason: String,
    },
}

impl RepositoryError {
    /// NotFoundエラーを作成
    pub fn not_found(uid: impl Into<String>) -> Self {
        Self::NotFound { uid: uid.into() }
    }

    /// StorageErrorからの変換（コンテキスト付き）
    pub fn storage(context: impl Into<String>, source: crate::traits::StorageError) -> Self {
        Self::Storage {
            context: context.into(),
            source,
        }
    }

    /// パースエラーを作成
    pub fn parse(context: impl Into<String>, path: Option<PathBuf>) -> Self {
        Self::Parse {
            context: context.into(),
            path,
        }
    }
}

// StorageError からの自動変換（後方互換性）
impl From<crate::traits::StorageError> for RepositoryError {
    fn from(err: crate::traits::StorageError) -> Self {
        Self::Storage {
            context: "storage operation".to_string(),
            source: err,
        }
    }
}
