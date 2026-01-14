use std::path::{Path, PathBuf};
use thiserror::Error;

/// ストレージ抽象化（依存性逆転原則）
pub trait Storage: Send + Sync {
    /// アトミック保存（tmp → rename パターン）
    fn save_atomic(&self, path: &Path, content: &str) -> Result<(), StorageError>;

    /// ファイル読み込み
    fn load(&self, path: &Path) -> Result<String, StorageError>;

    /// ファイル削除
    fn delete(&self, path: &Path) -> Result<(), StorageError>;

    /// ファイル存在確認
    fn exists(&self, path: &Path) -> bool;

    /// 指定拡張子のファイル一覧を取得
    fn list_files(&self, dir: &Path, extension: &str) -> Result<Vec<PathBuf>, StorageError>;
}

/// ストレージエラー
#[derive(Debug, Error)]
pub enum StorageError {
    #[error("ファイルが見つかりません: {0}")]
    NotFound(PathBuf),
    #[error("IOエラー: {0}")]
    Io(#[from] std::io::Error),
    #[error("パーミッションエラー: {0}")]
    PermissionDenied(PathBuf),
    #[error("ディレクトリ作成エラー: {0}")]
    CreateDirFailed(PathBuf),
}
