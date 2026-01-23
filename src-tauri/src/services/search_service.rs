//! 高速部分文字列検索サービス
//!
//! # 最適化
//!
//! - **rayon**: ファイル読み込みの並列化
//! - **memmap2**: メモリマップによる高速ファイルI/O
//! - **本文先頭検索**: 最初の4KBのみ検索（高速化）

use crate::domain::{ContentPreview, MatchRange, SearchError, SearchResult};
use crate::traits::{NoteListItem, NoteRepository};
use memmap2::Mmap;
use rayon::prelude::*;
use std::fs::File;
use std::path::Path;
use std::sync::Arc;

/// 本文検索の最大バイト数（高速化のため先頭のみ）
const MAX_CONTENT_SEARCH_BYTES: usize = 4096;

/// プレビューの前後文字数
const PREVIEW_CONTEXT_CHARS: usize = 30;

/// デフォルトの検索結果上限
const DEFAULT_LIMIT: usize = 50;

/// 検索サービス
pub struct SearchService {
    repository: Arc<dyn NoteRepository>,
}

impl SearchService {
    pub fn new(repository: Arc<dyn NoteRepository>) -> Self {
        Self { repository }
    }

    /// タイトルでノートを検索（完全一致）
    ///
    /// Wiki linkの解決に使用
    pub fn find_by_title(&self, title: &str) -> Result<Option<NoteListItem>, SearchError> {
        let title_lower = title.to_lowercase();
        let notes = self.repository.list_all()?;

        Ok(notes.into_iter().find(|n| n.title.to_lowercase() == title_lower))
    }

    /// 部分文字列検索を実行
    ///
    /// # Arguments
    /// * `query` - 検索クエリ
    /// * `limit` - 最大結果数
    ///
    /// # Performance
    /// - 並列ファイル読み込み (rayon)
    /// - メモリマップ (memmap2)
    /// - 本文は先頭4KBのみ検索
    pub fn search(
        &self,
        query: &str,
        limit: Option<usize>,
    ) -> Result<Vec<SearchResult>, SearchError> {
        let limit = limit.unwrap_or(DEFAULT_LIMIT).min(100);

        // 空クエリは空結果
        let query = query.trim();
        if query.is_empty() {
            return Ok(Vec::new());
        }

        // 1. 全ノートのメタデータ取得
        let notes = self.repository.list_all()?;

        // 2. クエリを小文字に変換（case-insensitive検索）
        let query_lower = query.to_lowercase();

        // 3. 並列検索実行
        let mut results: Vec<SearchResult> = notes
            .par_iter()
            .filter_map(|note| Self::match_note(&query_lower, note))
            .collect();

        // 4. スコア降順ソート
        results.sort_by(|a, b| b.score.cmp(&a.score));

        // 5. 上限適用
        results.truncate(limit);

        Ok(results)
    }

    /// 単一ノートのマッチング（部分文字列検索）
    fn match_note(query_lower: &str, note: &NoteListItem) -> Option<SearchResult> {
        let title_lower = note.title.to_lowercase();

        // タイトルマッチング
        let title_matched = title_lower.contains(query_lower);

        // 本文マッチング（memmap + 先頭のみ）
        let (content_matched, content_preview) =
            Self::match_content(query_lower, &note.path).unwrap_or((false, None));

        // マッチなしならスキップ
        if !title_matched && !content_matched {
            return None;
        }

        // スコア計算（タイトルマッチを優先: 1000点、本文マッチ: 100点）
        let mut score = 0u32;
        if title_matched {
            score += 1000;
        }
        if content_matched {
            score += 100;
        }

        // マッチ位置を抽出
        let title_matches = if title_matched {
            Self::extract_match_ranges(query_lower, &note.title)
        } else {
            Vec::new()
        };

        Some(SearchResult {
            uid: note.uid.clone(),
            title: note.title.clone(),
            score,
            title_matches,
            content_preview,
        })
    }

    /// 本文マッチング（memmap使用）
    fn match_content(
        query_lower: &str,
        path: &Path,
    ) -> Result<(bool, Option<ContentPreview>), std::io::Error> {
        // ファイルをmemmap
        let file = File::open(path)?;
        let metadata = file.metadata()?;

        // 空ファイルはスキップ
        if metadata.len() == 0 {
            return Ok((false, None));
        }

        let mmap = unsafe { Mmap::map(&file)? };

        // Front matterをスキップ
        let content = Self::skip_front_matter(&mmap);

        // 先頭N バイトのみ検索
        let search_bytes = &content[..content.len().min(MAX_CONTENT_SEARCH_BYTES)];

        // UTF-8としてデコード（無効な場合はスキップ）
        let content_str = match std::str::from_utf8(search_bytes) {
            Ok(s) => s,
            Err(e) => {
                // 有効な部分のみ使用
                let valid_up_to = e.valid_up_to();
                if valid_up_to == 0 {
                    return Ok((false, None));
                }
                unsafe { std::str::from_utf8_unchecked(&search_bytes[..valid_up_to]) }
            }
        };

        // 部分文字列検索
        let content_lower = content_str.to_lowercase();
        if !content_lower.contains(query_lower) {
            return Ok((false, None));
        }

        // プレビュー生成
        let preview = Self::generate_preview(query_lower, content_str);

        Ok((true, preview))
    }

    /// Front matter (---で囲まれた部分) をスキップ
    fn skip_front_matter(content: &[u8]) -> &[u8] {
        if content.starts_with(b"---") {
            // 2つ目の---を探す
            if let Some(end_pos) = content[3..]
                .windows(4)
                .position(|w| w.starts_with(b"\n---"))
            {
                let skip_to = 3 + end_pos + 4; // "---" + position + "\n---"
                if skip_to < content.len() {
                    return &content[skip_to..];
                }
            }
        }
        content
    }

    /// マッチ位置の抽出（部分文字列検索）
    fn extract_match_ranges(query_lower: &str, text: &str) -> Vec<MatchRange> {
        let text_lower = text.to_lowercase();
        let mut ranges = Vec::new();

        // 全てのマッチ位置を検索
        let mut search_start = 0;
        while let Some(byte_pos) = text_lower[search_start..].find(query_lower) {
            let absolute_byte_pos = search_start + byte_pos;

            // バイト位置から文字位置に変換
            let char_start = text[..absolute_byte_pos].chars().count() as u32;
            let char_end = char_start + query_lower.chars().count() as u32;

            ranges.push(MatchRange {
                start: char_start,
                end: char_end,
            });

            // 次の検索位置（1文字分進める）
            search_start = absolute_byte_pos + text_lower[absolute_byte_pos..].chars().next().map_or(1, |c| c.len_utf8());
        }

        ranges
    }

    /// プレビューテキスト生成（部分文字列検索）
    fn generate_preview(query_lower: &str, content: &str) -> Option<ContentPreview> {
        let content_lower = content.to_lowercase();

        // 最初のマッチ位置を検索
        let byte_pos = content_lower.find(query_lower)?;

        // バイト位置から文字位置に変換
        let first_match = content[..byte_pos].chars().count();
        let match_len = query_lower.chars().count();
        let chars: Vec<char> = content.chars().collect();

        let preview_start = first_match.saturating_sub(PREVIEW_CONTEXT_CHARS);
        let preview_end = (first_match + match_len + PREVIEW_CONTEXT_CHARS).min(chars.len());

        let preview_chars: String = chars[preview_start..preview_end].iter().collect();

        // プレビュー内でのマッチ位置を再計算
        let match_in_preview = first_match - preview_start;

        let prefix = if preview_start > 0 { "..." } else { "" };
        let suffix = if preview_end < chars.len() { "..." } else { "" };

        Some(ContentPreview {
            text: format!("{}{}{}", prefix, preview_chars, suffix),
            match_start: (match_in_preview + prefix.len()) as u32,
            match_end: (match_in_preview + prefix.len() + match_len) as u32,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_skip_front_matter() {
        let content = b"---\nuid: test\nupdated_at: 2024\n---\n\nHello World";
        let result = SearchService::skip_front_matter(content);
        // skip_front_matter skips past "---" + front matter + "\n---", leaving "\n\nHello World"
        assert!(result.starts_with(b"\n\nHello"));
    }

    #[test]
    fn test_skip_front_matter_no_front_matter() {
        let content = b"Hello World";
        let result = SearchService::skip_front_matter(content);
        assert_eq!(result, content);
    }

    #[test]
    fn test_skip_front_matter_incomplete() {
        let content = b"---\nuid: test\nno closing";
        let result = SearchService::skip_front_matter(content);
        assert_eq!(result, content);
    }
}
