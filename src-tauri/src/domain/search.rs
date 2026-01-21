//! 検索ドメインモデル

/// 検索結果
#[derive(Debug, Clone)]
pub struct SearchResult {
    /// ノートUID
    pub uid: String,
    /// ノートタイトル
    pub title: String,
    /// マッチスコア (0-65535)
    pub score: u32,
    /// タイトル内のマッチ位置
    pub title_matches: Vec<MatchRange>,
    /// 本文マッチのプレビュー
    pub content_preview: Option<ContentPreview>,
}

/// マッチ位置（文字単位）
#[derive(Debug, Clone)]
pub struct MatchRange {
    pub start: u32,
    pub end: u32,
}

/// 本文プレビュー
#[derive(Debug, Clone)]
pub struct ContentPreview {
    /// プレビューテキスト（マッチ箇所の前後）
    pub text: String,
    /// text内でのマッチ開始位置（文字単位）
    pub match_start: u32,
    /// text内でのマッチ終了位置（文字単位）
    pub match_end: u32,
}

/// 検索エラー
#[derive(Debug, thiserror::Error)]
pub enum SearchError {
    #[error("Repository error: {0}")]
    Repository(#[from] crate::traits::RepositoryError),

    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),
}
