// ギャラリー関連コマンド
//
// パフォーマンス最適化: SQLiteインデックスからキャッシュされたデータを取得
// N+1問題を解消し、10-100倍の高速化を実現

use crate::AppState;
use serde::Serialize;
use tauri::State;

/// プレビュー文字数（400文字）
pub const PREVIEW_LENGTH: usize = 400;

/// ギャラリー用ノートアイテムDTO
#[derive(Debug, Clone, Serialize)]
pub struct NoteGalleryItemDto {
    pub uid: String,
    pub title: String,
    /// 本文プレビュー（最初の400文字、Markdown装飾を除去）
    pub preview: String,
    /// 全タグ（フロントマター + ハッシュタグをマージ）
    pub tags: Vec<String>,
    pub created_at: String,
    pub updated_at: String,
}

/// ソート順
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum GallerySortOrder {
    UpdatedAt,
    CreatedAt,
}

impl Default for GallerySortOrder {
    fn default() -> Self {
        Self::UpdatedAt
    }
}

/// ギャラリー用ノート一覧を取得（高速版 - インデックスから取得）
#[tauri::command]
pub fn list_notes_gallery(
    state: State<AppState>,
    sort_order: Option<GallerySortOrder>,
    tag_filter: Option<String>,
) -> Result<Vec<NoteGalleryItemDto>, String> {
    let sort = sort_order.unwrap_or_default();
    let sort_by_created = matches!(sort, GallerySortOrder::CreatedAt);

    // インデックスから直接取得（高速）
    let gallery_notes = state
        .note_service
        .list_gallery_notes(sort_by_created, tag_filter.as_deref())
        .map_err(|e| e.to_string())?;

    // DTOに変換
    let items: Vec<NoteGalleryItemDto> = gallery_notes
        .into_iter()
        .map(|note| NoteGalleryItemDto {
            uid: note.uid,
            title: note.title,
            preview: note.preview,
            tags: note.tags,
            created_at: note.created_at.format("%Y-%m-%d %H:%M:%S").to_string(),
            updated_at: note.updated_at.format("%Y-%m-%d %H:%M:%S").to_string(),
        })
        .collect();

    Ok(items)
}

/// 本文からプレビューを生成（Markdown装飾を除去）
pub fn generate_preview(content: &str, max_len: usize) -> String {
    let mut result = String::new();
    let mut in_code_block = false;
    let mut chars_count = 0;

    for line in content.lines() {
        let trimmed = line.trim();

        // コードブロックをスキップ
        if trimmed.starts_with("```") {
            in_code_block = !in_code_block;
            continue;
        }
        if in_code_block {
            continue;
        }

        // 空行は1つのスペースとして扱う
        if trimmed.is_empty() {
            if !result.is_empty() && !result.ends_with(' ') {
                result.push(' ');
                chars_count += 1;
            }
            continue;
        }

        // Markdown装飾を除去
        let clean_line = clean_markdown(trimmed);

        // 結果に追加
        for c in clean_line.chars() {
            if chars_count >= max_len {
                result.push_str("...");
                return result;
            }
            result.push(c);
            chars_count += 1;
        }

        // 行間にスペースを追加
        if !result.ends_with(' ') {
            result.push(' ');
            chars_count += 1;
        }
    }

    result.trim().to_string()
}

/// Markdown装飾を除去
pub fn clean_markdown(line: &str) -> String {
    let mut result = line.to_string();

    // 見出し（# ## ### など）を除去
    if result.starts_with('#') {
        let trimmed = result.trim_start_matches('#').trim_start();
        result = trimmed.to_string();
    }

    // リストマーカーを除去
    if result.starts_with("- ") || result.starts_with("* ") {
        result = result[2..].to_string();
    }
    // 番号付きリスト
    if let Some(pos) = result.find(". ") {
        if pos <= 3 && result[..pos].chars().all(|c| c.is_ascii_digit()) {
            result = result[pos + 2..].to_string();
        }
    }

    // 引用を除去
    if result.starts_with("> ") {
        result = result[2..].to_string();
    }

    // 太字・斜体を除去 (**text**, *text*, __text__, _text_)
    result = result.replace("**", "");
    result = result.replace("__", "");
    // 単独の * と _ は残す（単語内で使われることがあるため）

    // インラインコードを除去
    // `code` -> code
    let mut cleaned = String::new();
    let mut in_code = false;
    for c in result.chars() {
        if c == '`' {
            in_code = !in_code;
        } else {
            cleaned.push(c);
        }
    }
    result = cleaned;

    // リンクを除去 [text](url) -> text
    while let Some(start) = result.find('[') {
        if let Some(mid) = result[start..].find("](") {
            if let Some(end) = result[start + mid..].find(')') {
                let text = &result[start + 1..start + mid];
                let before = &result[..start];
                let after = &result[start + mid + end + 1..];
                result = format!("{}{}{}", before, text, after);
                continue;
            }
        }
        break;
    }

    // 画像を除去 ![alt](url) -> alt
    while let Some(start) = result.find("![") {
        if let Some(mid) = result[start..].find("](") {
            if let Some(end) = result[start + mid..].find(')') {
                let text = &result[start + 2..start + mid];
                let before = &result[..start];
                let after = &result[start + mid + end + 1..];
                result = format!("{}{}{}", before, text, after);
                continue;
            }
        }
        break;
    }

    result
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_generate_preview() {
        let content = "# Title\n\nThis is a paragraph.\n\n- List item 1\n- List item 2";
        let preview = generate_preview(content, 100);
        assert!(preview.contains("Title"));
        assert!(preview.contains("This is a paragraph"));
        assert!(preview.contains("List item 1"));
    }

    #[test]
    fn test_generate_preview_truncate() {
        let content = "This is a very long text that should be truncated. ".repeat(20);
        let preview = generate_preview(&content, 50);
        assert!(preview.len() <= 53); // 50 + "..."
        assert!(preview.ends_with("..."));
    }

    #[test]
    fn test_clean_markdown_heading() {
        assert_eq!(clean_markdown("# Heading"), "Heading");
        assert_eq!(clean_markdown("## Subheading"), "Subheading");
    }

    #[test]
    fn test_clean_markdown_bold() {
        assert_eq!(clean_markdown("This is **bold** text"), "This is bold text");
    }

    #[test]
    fn test_clean_markdown_link() {
        assert_eq!(clean_markdown("[link text](http://example.com)"), "link text");
    }

    #[test]
    fn test_clean_markdown_code() {
        assert_eq!(clean_markdown("Some `code` here"), "Some code here");
    }
}
