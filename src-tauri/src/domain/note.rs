use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

/// メモのメタデータ（YAML front matter）
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct NoteMetadata {
    pub uid: String,
    pub title: Option<String>,
    pub tags: Vec<String>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

impl NoteMetadata {
    pub fn new() -> Self {
        let now = Utc::now();
        // 時間ベースのUID: YYYYMMDDHHmmss形式 + ナノ秒の下6桁
        let uid = format!(
            "{}{}",
            now.format("%Y%m%d%H%M%S"),
            now.timestamp_subsec_nanos() % 1_000_000
        );
        Self {
            uid,
            title: None,
            tags: Vec::new(),
            created_at: now,
            updated_at: now,
        }
    }

    /// 指定UIDでメタデータを作成
    pub fn with_uid(uid: String) -> Self {
        let now = Utc::now();
        Self {
            uid,
            title: None,
            tags: Vec::new(),
            created_at: now,
            updated_at: now,
        }
    }

    /// YAML front matterをパース
    pub fn from_yaml(yaml: &str) -> Result<Self, serde_yaml_error::Error> {
        // カスタムパーサーを使用（serde_yaml非依存）
        let mut uid = None;
        let mut title = None;
        let mut tags = Vec::new();
        let mut created_at = None;
        let mut updated_at = None;
        let mut in_tags = false;

        for line in yaml.lines() {
            let line_trimmed = line.trim();

            // タグ配列の処理
            if in_tags {
                if line_trimmed.starts_with("- ") {
                    let tag = line_trimmed.trim_start_matches("- ").trim().to_string();
                    if !tag.is_empty() {
                        tags.push(tag);
                    }
                    continue;
                } else if !line_trimmed.is_empty() && !line.starts_with(' ') && !line.starts_with('\t') {
                    // 新しいフィールドが始まった
                    in_tags = false;
                }
            }

            if line_trimmed.starts_with("uid:") {
                uid = Some(line_trimmed.trim_start_matches("uid:").trim().to_string());
            } else if line_trimmed.starts_with("title:") {
                let value = line_trimmed.trim_start_matches("title:").trim();
                if !value.is_empty() {
                    title = Some(value.to_string());
                }
            } else if line_trimmed.starts_with("tags:") {
                let inline_value = line_trimmed.trim_start_matches("tags:").trim();
                // インライン配列形式: tags: [tag1, tag2]
                if inline_value.starts_with('[') && inline_value.ends_with(']') {
                    let inner = &inline_value[1..inline_value.len()-1];
                    tags = inner.split(',')
                        .map(|s| s.trim().to_string())
                        .filter(|s| !s.is_empty())
                        .collect();
                } else if inline_value.is_empty() {
                    // 複数行形式
                    in_tags = true;
                }
            } else if line_trimmed.starts_with("created_at:") {
                let value = line_trimmed.trim_start_matches("created_at:").trim();
                created_at = Self::parse_datetime(value);
            } else if line_trimmed.starts_with("updated_at:") {
                let value = line_trimmed.trim_start_matches("updated_at:").trim();
                updated_at = Self::parse_datetime(value);
            }
        }

        match (uid, created_at, updated_at) {
            (Some(uid), Some(created_at), Some(updated_at)) => Ok(Self {
                uid,
                title,
                tags,
                created_at,
                updated_at,
            }),
            _ => Err(serde_yaml_error::Error::InvalidFormat),
        }
    }

    /// 日時をパース（新形式 "YYYY-MM-DD HH:MM:SS" と旧形式 RFC3339 の両方に対応）
    fn parse_datetime(value: &str) -> Option<DateTime<Utc>> {
        // 新形式: "2025-12-15 02:42:38"
        if let Ok(naive) = chrono::NaiveDateTime::parse_from_str(value, "%Y-%m-%d %H:%M:%S") {
            return Some(naive.and_utc());
        }
        // 旧形式: RFC3339 (後方互換性)
        DateTime::parse_from_rfc3339(value)
            .ok()
            .map(|dt| dt.with_timezone(&Utc))
    }

    /// 日時を読みやすい形式にフォーマット
    fn format_datetime(dt: &DateTime<Utc>) -> String {
        dt.format("%Y-%m-%d %H:%M:%S").to_string()
    }

    /// YAML front matterに変換
    pub fn to_yaml(&self) -> String {
        let title_line = match &self.title {
            Some(t) => format!("title: {}\n", t),
            None => String::new(),
        };
        let tags_line = if self.tags.is_empty() {
            String::new()
        } else {
            format!("tags:\n{}", self.tags.iter().map(|t| format!("  - {}", t)).collect::<Vec<_>>().join("\n")) + "\n"
        };
        format!(
            "uid: {}\n{}{}created_at: {}\nupdated_at: {}",
            self.uid,
            title_line,
            tags_line,
            Self::format_datetime(&self.created_at),
            Self::format_datetime(&self.updated_at)
        )
    }
}

impl Default for NoteMetadata {
    fn default() -> Self {
        Self::new()
    }
}

/// メモエンティティ
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Note {
    pub metadata: NoteMetadata,
    pub content: String,
    #[serde(skip)]
    pub is_dirty: bool,
}

impl Note {
    /// 新規メモを作成
    pub fn new() -> Self {
        Self {
            metadata: NoteMetadata::new(),
            content: String::new(),
            is_dirty: false,
        }
    }

    /// 指定UIDでメモを作成
    pub fn with_uid(uid: String) -> Self {
        Self {
            metadata: NoteMetadata::with_uid(uid),
            content: String::new(),
            is_dirty: false,
        }
    }

    /// タイトル付きで新規メモを作成
    pub fn with_title(title: &str) -> Self {
        let mut note = Self::new();
        note.content = format!("# {}\n\n", title);
        note.metadata.title = Some(title.to_string());
        note.is_dirty = true;
        note
    }

    /// ファイル内容からパース（front matterあり）
    pub fn from_file_content(content: &str) -> Result<Self, NoteParseError> {
        if !content.starts_with("---\n") {
            return Err(NoteParseError::MissingFrontMatter);
        }

        // Front matterの終了位置を検索（最初の---の後から）
        let search_start = 4; // "---\n".len()
        let end_marker = content[search_start..]
            .find("\n---")
            .ok_or(NoteParseError::InvalidFrontMatter)?;

        let yaml_content = &content[search_start..search_start + end_marker];
        let metadata = NoteMetadata::from_yaml(yaml_content)
            .map_err(|_| NoteParseError::InvalidFrontMatter)?;

        // 本文を取得（---の後の改行をスキップ）
        let body_start = search_start + end_marker + 4; // "\n---".len()
        let body = if body_start < content.len() {
            content[body_start..].trim_start_matches('\n').to_string()
        } else {
            String::new()
        };

        Ok(Self {
            metadata,
            content: body,
            is_dirty: false,
        })
    }

    /// ファイル内容がfront matterを持つかチェック
    pub fn has_front_matter(content: &str) -> bool {
        content.starts_with("---\n") && content[4..].contains("\n---")
    }

    /// ファイル保存用の完全な内容を生成
    pub fn to_file_content(&self) -> String {
        format!("---\n{}\n---\n\n{}", self.metadata.to_yaml(), self.content)
    }

    /// 本文の最初のH1またはH2見出しを抽出
    pub fn extract_heading(&self) -> Option<String> {
        for line in self.content.lines() {
            let trimmed = line.trim();
            if let Some(h1) = trimmed.strip_prefix("# ") {
                return Some(h1.trim().to_string());
            }
            if let Some(h2) = trimmed.strip_prefix("## ") {
                return Some(h2.trim().to_string());
            }
        }
        None
    }

    /// UIDを取得
    pub fn uid(&self) -> &str {
        &self.metadata.uid
    }

    /// コンテンツを更新
    pub fn update_content(&mut self, content: String) {
        if self.content != content {
            self.content = content;
            self.metadata.updated_at = Utc::now();
            // タイトルを見出しから更新
            self.metadata.title = self.extract_heading();
            self.is_dirty = true;
        }
    }

    /// 保存完了をマーク
    pub fn mark_saved(&mut self) {
        self.is_dirty = false;
    }

    /// フロントマターのタグを取得
    pub fn tags(&self) -> &[String] {
        &self.metadata.tags
    }

    /// タグを更新
    pub fn update_tags(&mut self, tags: Vec<String>) {
        self.metadata.tags = tags;
        self.metadata.updated_at = Utc::now();
        self.is_dirty = true;
    }

    /// 本文からハッシュタグを抽出
    pub fn extract_hashtags(&self) -> Vec<String> {
        let mut hashtags = Vec::new();
        let re = regex::Regex::new(r"(?:^|\s)#([a-zA-Z0-9_\-\u3040-\u309F\u30A0-\u30FF\u4E00-\u9FFF]+)").unwrap();
        for cap in re.captures_iter(&self.content) {
            let tag = cap[1].to_lowercase();
            if !hashtags.contains(&tag) {
                hashtags.push(tag);
            }
        }
        hashtags
    }

    /// 全タグを取得（フロントマター + ハッシュタグをマージ、重複排除）
    pub fn all_tags(&self) -> Vec<String> {
        let mut all = self.metadata.tags.clone();
        for tag in self.extract_hashtags() {
            if !all.iter().any(|t| t.to_lowercase() == tag.to_lowercase()) {
                all.push(tag);
            }
        }
        all.sort();
        all
    }
}

impl Default for Note {
    fn default() -> Self {
        Self::new()
    }
}

/// メモパースエラー
#[derive(Debug, thiserror::Error)]
pub enum NoteParseError {
    #[error("Front matterが見つかりません")]
    MissingFrontMatter,
    #[error("Front matterの形式が不正です")]
    InvalidFrontMatter,
}

/// YAML パースエラー（軽量実装用）
pub mod serde_yaml_error {
    #[derive(Debug)]
    pub enum Error {
        InvalidFormat,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_new_note() {
        let note = Note::new();
        assert!(!note.metadata.uid.is_empty());
        assert!(note.content.is_empty());
        assert!(!note.is_dirty);
    }

    #[test]
    fn test_note_roundtrip() {
        let mut note = Note::new();
        note.update_content("# テストメモ\n\nこれはテストです。".to_string());

        let file_content = note.to_file_content();
        let parsed = Note::from_file_content(&file_content).unwrap();

        assert_eq!(note.metadata.uid, parsed.metadata.uid);
        assert_eq!(note.metadata.title, Some("テストメモ".to_string()));
        assert_eq!(parsed.metadata.title, Some("テストメモ".to_string()));
        assert_eq!(note.content, parsed.content);
    }

    #[test]
    fn test_extract_heading_h1() {
        let mut note = Note::new();
        note.content = "# 買い物リスト\n\n- 牛乳\n- パン".to_string();
        assert_eq!(note.extract_heading(), Some("買い物リスト".to_string()));
    }

    #[test]
    fn test_extract_heading_h2() {
        let mut note = Note::new();
        note.content = "## 2026年の目標\n\n目標を書く".to_string();
        assert_eq!(note.extract_heading(), Some("2026年の目標".to_string()));
    }

    #[test]
    fn test_extract_heading_none() {
        let mut note = Note::new();
        note.content = "見出しなしのメモ".to_string();
        assert_eq!(note.extract_heading(), None);
    }

    #[test]
    fn test_update_content_marks_dirty() {
        let mut note = Note::new();
        assert!(!note.is_dirty);

        note.update_content("新しい内容".to_string());
        assert!(note.is_dirty);

        note.mark_saved();
        assert!(!note.is_dirty);
    }
}
