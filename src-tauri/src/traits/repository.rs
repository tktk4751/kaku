use crate::domain::Note;
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
#[derive(Debug, Error)]
pub enum RepositoryError {
    #[error("メモが見つかりません: {0}")]
    NotFound(String),
    #[error("ストレージエラー: {0}")]
    Storage(#[from] crate::traits::StorageError),
    #[error("パースエラー: {0}")]
    Parse(String),
    #[error("ファイル名生成エラー: {0}")]
    FilenameGeneration(String),
}
