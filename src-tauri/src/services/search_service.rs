//! 高速ファジー検索サービス
//!
//! # 最適化
//!
//! - **nucleo-matcher**: skim比6倍高速なfuzzy matching
//! - **rayon**: ファイル読み込みの並列化
//! - **memmap2**: メモリマップによる高速ファイルI/O
//! - **本文先頭検索**: 最初の4KBのみ検索（高速化）

use crate::domain::{ContentPreview, MatchRange, SearchError, SearchResult};
use crate::traits::{NoteListItem, NoteRepository};
use memmap2::Mmap;
use nucleo_matcher::pattern::{AtomKind, CaseMatching, Normalization, Pattern};
use nucleo_matcher::{Config, Matcher, Utf32Str};
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

    /// ファジー検索を実行
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

        // 2. クエリ文字列を保持
        let query_string = query.to_string();

        // 3. 並列検索実行
        let mut results: Vec<SearchResult> = notes
            .par_iter()
            .filter_map(|note| {
                // スレッドローカルでMatcherを作成（Matcherはスレッドセーフではない）
                let mut matcher = Matcher::new(Config::DEFAULT);
                let pattern = Pattern::new(
                    &query_string,
                    CaseMatching::Ignore,
                    Normalization::Smart,
                    AtomKind::Fuzzy,
                );

                Self::match_note(&mut matcher, &pattern, note)
            })
            .collect();

        // 4. スコア降順ソート
        results.sort_by(|a, b| b.score.cmp(&a.score));

        // 5. 上限適用
        results.truncate(limit);

        Ok(results)
    }

    /// 単一ノートのマッチング
    fn match_note(
        matcher: &mut Matcher,
        pattern: &Pattern,
        note: &NoteListItem,
    ) -> Option<SearchResult> {
        let mut buf = Vec::new();

        // タイトルマッチング
        let title_score = {
            let title_utf32 = Utf32Str::new(&note.title, &mut buf);
            pattern.score(title_utf32, matcher)
        };

        // 本文マッチング（memmap + 先頭のみ）
        let (content_score, content_preview) =
            Self::match_content(matcher, pattern, &note.path).unwrap_or((None, None));

        // スコア計算（タイトルを2倍重視）
        let title_pts = title_score.unwrap_or(0) as u32 * 2;
        let content_pts = content_score.unwrap_or(0);
        let total_score = title_pts + content_pts;

        if total_score == 0 {
            return None;
        }

        // マッチ位置を抽出
        let title_matches = Self::extract_match_ranges(matcher, pattern, &note.title);

        Some(SearchResult {
            uid: note.uid.clone(),
            title: note.title.clone(),
            score: total_score,
            title_matches,
            content_preview,
        })
    }

    /// 本文マッチング（memmap使用）
    fn match_content(
        matcher: &mut Matcher,
        pattern: &Pattern,
        path: &Path,
    ) -> Result<(Option<u32>, Option<ContentPreview>), std::io::Error> {
        // ファイルをmemmap
        let file = File::open(path)?;
        let metadata = file.metadata()?;

        // 空ファイルはスキップ
        if metadata.len() == 0 {
            return Ok((None, None));
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
                    return Ok((None, None));
                }
                unsafe { std::str::from_utf8_unchecked(&search_bytes[..valid_up_to]) }
            }
        };

        // マッチング
        let mut buf = Vec::new();
        let utf32 = Utf32Str::new(content_str, &mut buf);
        let score = pattern.score(utf32, matcher);

        if score.is_none() || score == Some(0) {
            return Ok((None, None));
        }

        // プレビュー生成
        let preview = Self::generate_preview(matcher, pattern, content_str);

        Ok((score, preview))
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

    /// マッチ位置の抽出
    fn extract_match_ranges(matcher: &mut Matcher, pattern: &Pattern, text: &str) -> Vec<MatchRange> {
        let mut buf = Vec::new();
        let mut match_indices: Vec<u32> = Vec::new();

        let utf32 = Utf32Str::new(text, &mut buf);
        pattern.indices(utf32, matcher, &mut match_indices);

        if match_indices.is_empty() {
            return Vec::new();
        }

        // 連続するインデックスをマージしてレンジに変換
        let mut ranges = Vec::new();
        match_indices.sort();

        let mut start = match_indices[0];
        let mut end = start;

        for &idx in &match_indices[1..] {
            if idx == end + 1 {
                end = idx;
            } else {
                ranges.push(MatchRange { start, end: end + 1 });
                start = idx;
                end = idx;
            }
        }

        // 最後のレンジ
        ranges.push(MatchRange { start, end: end + 1 });

        ranges
    }

    /// プレビューテキスト生成
    fn generate_preview(
        matcher: &mut Matcher,
        pattern: &Pattern,
        content: &str,
    ) -> Option<ContentPreview> {
        let mut buf = Vec::new();
        let mut match_indices: Vec<u32> = Vec::new();

        let utf32 = Utf32Str::new(content, &mut buf);
        pattern.indices(utf32, matcher, &mut match_indices);

        if match_indices.is_empty() {
            return None;
        }

        match_indices.sort();

        // 最初のマッチ位置を中心にプレビュー
        let first_match = match_indices[0] as usize;
        let chars: Vec<char> = content.chars().collect();

        let preview_start = first_match.saturating_sub(PREVIEW_CONTEXT_CHARS);
        let preview_end = (first_match + PREVIEW_CONTEXT_CHARS + 1).min(chars.len());

        let preview_chars: String = chars[preview_start..preview_end].iter().collect();

        // プレビュー内でのマッチ位置を再計算
        let match_in_preview = first_match - preview_start;

        // 連続するマッチをカウント
        let mut match_len = 1usize;
        for i in 1..match_indices.len() {
            if match_indices[i] == match_indices[i - 1] + 1
                && (match_indices[i] as usize) < preview_end
            {
                match_len += 1;
            } else {
                break;
            }
        }

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
        assert!(result.starts_with(b"\nHello"));
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
