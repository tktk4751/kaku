//! バックリンク関連のドメインモデル

use serde::{Deserialize, Serialize};

/// バックリンク情報
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BacklinkInfo {
    /// リンク元ノートのUID
    pub source_uid: String,
    /// リンク元ノートのタイトル
    pub source_title: String,
    /// リンク周辺のコンテキスト（プレビュー）
    pub context: String,
}

/// ウィキリンクの抽出結果
#[derive(Debug, Clone)]
pub struct ExtractedLink {
    /// リンクのタイトル部分
    pub title: String,
    /// 表示テキスト（エイリアス）
    pub display: Option<String>,
    /// コンテンツ内での位置
    pub position: usize,
}

/// ウィキリンクを抽出
///
/// [[title]] と [[title|display]] 形式に対応
pub fn extract_wiki_links(content: &str) -> Vec<ExtractedLink> {
    let mut links = Vec::new();
    let mut chars = content.char_indices().peekable();

    while let Some((i, c)) = chars.next() {
        if c == '[' {
            if let Some((_, '[')) = chars.peek() {
                chars.next(); // consume second [
                let start = i;

                // Find the closing ]]
                let mut title = String::new();
                let mut display = None;
                let mut in_display = false;

                while let Some((_, c)) = chars.next() {
                    if c == ']' {
                        if let Some((_, ']')) = chars.peek() {
                            chars.next(); // consume second ]
                            if !title.is_empty() {
                                links.push(ExtractedLink {
                                    title: title.trim().to_string(),
                                    display: display.map(|s: String| s.trim().to_string()),
                                    position: start,
                                });
                            }
                            break;
                        }
                    } else if c == '|' && !in_display {
                        in_display = true;
                        display = Some(String::new());
                    } else if c == '\n' {
                        // Line break inside link - invalid, reset
                        break;
                    } else if in_display {
                        if let Some(ref mut d) = display {
                            d.push(c);
                        }
                    } else {
                        title.push(c);
                    }
                }
            }
        }
    }

    links
}

/// リンク周辺のコンテキストを抽出
pub fn extract_context(content: &str, position: usize, context_chars: usize) -> String {
    let chars: Vec<char> = content.chars().collect();
    let char_pos = content[..position.min(content.len())]
        .chars()
        .count();

    let start = char_pos.saturating_sub(context_chars);
    let end = (char_pos + context_chars + 20).min(chars.len()); // 20 for the link itself

    let mut result: String = chars[start..end].iter().collect();

    // Clean up: remove newlines and extra whitespace
    result = result.replace('\n', " ");
    while result.contains("  ") {
        result = result.replace("  ", " ");
    }

    // Add ellipsis if truncated
    let prefix = if start > 0 { "..." } else { "" };
    let suffix = if end < chars.len() { "..." } else { "" };

    format!("{}{}{}", prefix, result.trim(), suffix)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_extract_simple_link() {
        let content = "This is a [[Test Note]] in text.";
        let links = extract_wiki_links(content);

        assert_eq!(links.len(), 1);
        assert_eq!(links[0].title, "Test Note");
        assert!(links[0].display.is_none());
    }

    #[test]
    fn test_extract_aliased_link() {
        let content = "Check out [[Project X|the project]] for details.";
        let links = extract_wiki_links(content);

        assert_eq!(links.len(), 1);
        assert_eq!(links[0].title, "Project X");
        assert_eq!(links[0].display, Some("the project".to_string()));
    }

    #[test]
    fn test_extract_multiple_links() {
        let content = "See [[Note A]] and [[Note B|B]] for more.";
        let links = extract_wiki_links(content);

        assert_eq!(links.len(), 2);
        assert_eq!(links[0].title, "Note A");
        assert_eq!(links[1].title, "Note B");
        assert_eq!(links[1].display, Some("B".to_string()));
    }

    #[test]
    fn test_extract_no_links() {
        let content = "This text has no wiki links.";
        let links = extract_wiki_links(content);

        assert!(links.is_empty());
    }

    #[test]
    fn test_extract_incomplete_link() {
        let content = "This [[incomplete link should not match.";
        let links = extract_wiki_links(content);

        assert!(links.is_empty());
    }

    #[test]
    fn test_extract_context() {
        let content = "Some text before [[Test Link]] and some text after.";
        let context = extract_context(content, 17, 15);

        assert!(context.contains("Test Link"));
        assert!(context.contains("before"));
        assert!(context.contains("after"));
    }
}
