# 検索機能 実装ロードマップ

## 高速化戦略まとめ

調査結果から、以下の最適化を採用します。

### 採用する高速化技術

| 技術 | 効果 | 実装コスト |
|------|------|-----------|
| **nucleo高レベルAPI** | バックグラウンドスレッドプール + ロックフリー | 中 |
| **rayon並列処理** | ファイル読み込み並列化 | 低 |
| **memmap2** | ファイルI/O 3-5倍高速化 | 低 |
| **本文先頭のみ検索** | 読み込み量削減 | 低 |
| **ASCII高速パス** | 90%のテキストで高速化 | nucleoが自動対応 |

### アーキテクチャ概要

```
┌──────────────────────────────────────────────────────────┐
│                    SearchService                          │
├──────────────────────────────────────────────────────────┤
│  ┌─────────────┐    ┌─────────────┐    ┌─────────────┐  │
│  │ IndexCache  │ ←→ │   Nucleo    │ ←→ │   Rayon     │  │
│  │ (メタデータ)│    │  Matcher    │    │ (並列I/O)  │  │
│  └─────────────┘    └─────────────┘    └─────────────┘  │
│         ↑                                     ↓          │
│         │              memmap2                │          │
│         └──────────── (高速読み込み) ─────────┘          │
└──────────────────────────────────────────────────────────┘
```

---

## Phase 0: 準備 (15分)

### 0.1 依存クレート追加

```toml
# src-tauri/Cargo.toml に追加

[dependencies]
# Fuzzy検索（高レベルAPIは使わず、matcher直接使用でシンプルに）
nucleo-matcher = "0.3"

# 並列処理
rayon = "1.10"

# 高速ファイル読み込み
memmap2 = "0.9"
```

### 0.2 モジュール構造作成

```bash
# 新規ファイル作成
touch src-tauri/src/domain/search.rs
touch src-tauri/src/services/search_service.rs
```

**完了条件**: `cargo check` が通る

---

## Phase 1: ドメイン層 (20分)

### 1.1 検索結果の型定義

**ファイル**: `src-tauri/src/domain/search.rs`

```rust
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

/// マッチ位置（バイト単位）
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
    /// text内でのマッチ開始位置
    pub match_start: u32,
    /// text内でのマッチ終了位置
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
```

### 1.2 domain/mod.rs に追加

```rust
mod search;
pub use search::*;
```

**完了条件**: 型定義がコンパイル通る

---

## Phase 2: 検索サービス実装 (60分)

### 2.1 SearchService 基本構造

**ファイル**: `src-tauri/src/services/search_service.rs`

```rust
//! 高速ファジー検索サービス
//!
//! # 最適化
//!
//! - **nucleo-matcher**: skim比6倍高速なfuzzy matching
//! - **rayon**: ファイル読み込みの並列化
//! - **memmap2**: メモリマップによる高速ファイルI/O
//! - **本文先頭検索**: 最初の1KBのみ検索（高速化）

use crate::domain::{SearchResult, SearchError, MatchRange, ContentPreview};
use crate::traits::{NoteRepository, NoteListItem, Storage};
use memmap2::Mmap;
use nucleo_matcher::{Matcher, Config};
use nucleo_matcher::pattern::{Pattern, CaseMatching, Normalization, AtomKind};
use nucleo_matcher::Utf32Str;
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
    pub fn search(&self, query: &str, limit: Option<usize>) -> Result<Vec<SearchResult>, SearchError> {
        let limit = limit.unwrap_or(DEFAULT_LIMIT).min(100);

        // 空クエリは空結果
        let query = query.trim();
        if query.is_empty() {
            return Ok(Vec::new());
        }

        // 1. 全ノートのメタデータ取得
        let notes = self.repository.list_all()?;

        // 2. パターン準備（スレッドローカルでMatcherを作成するため、ここでは文字列のみ保持）
        let query_chars: Vec<char> = query.chars().collect();

        // 3. 並列検索実行
        let mut results: Vec<SearchResult> = notes
            .par_iter()
            .filter_map(|note| {
                // スレッドローカルでMatcherを作成（Matcherはスレッドセーフではない）
                let mut matcher = Matcher::new(Config::DEFAULT);
                let pattern = Pattern::new(
                    &query_chars.iter().collect::<String>(),
                    CaseMatching::Ignore,
                    Normalization::Smart,
                    AtomKind::Fuzzy,
                );

                self.match_note(&mut matcher, &pattern, note)
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
        &self,
        matcher: &mut Matcher,
        pattern: &Pattern,
        note: &NoteListItem,
    ) -> Option<SearchResult> {
        let mut title_indices = Vec::new();
        let mut content_indices = Vec::new();

        // タイトルマッチング
        let title_score = {
            let title_utf32 = Utf32Str::new(&note.title, &mut title_indices);
            pattern.score(title_utf32, matcher)
        };

        // 本文マッチング（memmap + 先頭のみ）
        let (content_score, content_preview) = self
            .match_content(matcher, pattern, &note.path, &mut content_indices)
            .unwrap_or((None, None));

        // スコア計算（タイトルを2倍重視）
        let title_pts = title_score.unwrap_or(0) as u32 * 2;
        let content_pts = content_score.unwrap_or(0) as u32;
        let total_score = title_pts + content_pts;

        if total_score == 0 {
            return None;
        }

        // マッチ位置を抽出
        let title_matches = self.extract_match_ranges(matcher, pattern, &note.title);

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
        &self,
        matcher: &mut Matcher,
        pattern: &Pattern,
        path: &Path,
        indices: &mut Vec<char>,
    ) -> Result<(Option<u16>, Option<ContentPreview>), std::io::Error> {
        // ファイルをmemmap
        let file = File::open(path)?;
        let mmap = unsafe { Mmap::map(&file)? };

        // Front matterをスキップ
        let content = self.skip_front_matter(&mmap);

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
        let utf32 = Utf32Str::new(content_str, indices);
        let score = pattern.score(utf32, matcher);

        if score.is_none() || score == Some(0) {
            return Ok((None, None));
        }

        // プレビュー生成
        let preview = self.generate_preview(matcher, pattern, content_str);

        Ok((score, preview))
    }

    /// Front matter (---で囲まれた部分) をスキップ
    fn skip_front_matter<'a>(&self, content: &'a [u8]) -> &'a [u8] {
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
    fn extract_match_ranges(
        &self,
        matcher: &mut Matcher,
        pattern: &Pattern,
        text: &str,
    ) -> Vec<MatchRange> {
        let mut indices = Vec::new();
        let mut match_indices = Vec::new();

        let utf32 = Utf32Str::new(text, &mut indices);
        pattern.indices(utf32, matcher, &mut match_indices);

        // 連続するインデックスをマージしてレンジに変換
        let mut ranges = Vec::new();
        let chars: Vec<char> = text.chars().collect();

        if match_indices.is_empty() {
            return ranges;
        }

        match_indices.sort();

        let mut start = match_indices[0] as usize;
        let mut end = start;

        for &idx in &match_indices[1..] {
            let idx = idx as usize;
            if idx == end + 1 {
                end = idx;
            } else {
                // バイト位置に変換
                let byte_start = chars[..start].iter().map(|c| c.len_utf8()).sum::<usize>();
                let byte_end = chars[..=end].iter().map(|c| c.len_utf8()).sum::<usize>();
                ranges.push(MatchRange {
                    start: byte_start as u32,
                    end: byte_end as u32,
                });
                start = idx;
                end = idx;
            }
        }

        // 最後のレンジ
        let byte_start = chars[..start].iter().map(|c| c.len_utf8()).sum::<usize>();
        let byte_end = chars[..=end.min(chars.len() - 1)]
            .iter()
            .map(|c| c.len_utf8())
            .sum::<usize>();
        ranges.push(MatchRange {
            start: byte_start as u32,
            end: byte_end as u32,
        });

        ranges
    }

    /// プレビューテキスト生成
    fn generate_preview(
        &self,
        matcher: &mut Matcher,
        pattern: &Pattern,
        content: &str,
    ) -> Option<ContentPreview> {
        let mut indices = Vec::new();
        let mut match_indices = Vec::new();

        let utf32 = Utf32Str::new(content, &mut indices);
        pattern.indices(utf32, matcher, &mut match_indices);

        if match_indices.is_empty() {
            return None;
        }

        // 最初のマッチ位置を中心にプレビュー
        let first_match = match_indices[0] as usize;
        let chars: Vec<char> = content.chars().collect();

        let preview_start = first_match.saturating_sub(PREVIEW_CONTEXT_CHARS);
        let preview_end = (first_match + PREVIEW_CONTEXT_CHARS).min(chars.len());

        let preview_chars: String = chars[preview_start..preview_end].iter().collect();

        // プレビュー内でのマッチ位置を再計算
        let match_in_preview = first_match - preview_start;
        let match_char = chars.get(first_match)?;

        Some(ContentPreview {
            text: if preview_start > 0 {
                format!("...{}", preview_chars)
            } else {
                preview_chars
            },
            match_start: (match_in_preview + if preview_start > 0 { 3 } else { 0 }) as u32,
            match_end: (match_in_preview + if preview_start > 0 { 3 } else { 0 } + 1) as u32,
        })
    }
}
```

### 2.2 services/mod.rs に追加

```rust
mod search_service;
pub use search_service::SearchService;
```

**完了条件**: SearchService がコンパイル通る

---

## Phase 3: コマンド層 (20分)

### 3.1 DTO定義

**ファイル**: `src-tauri/src/commands/mod.rs` に追加

```rust
/// 検索結果DTO
#[derive(Serialize)]
pub struct SearchResultDto {
    pub uid: String,
    pub title: String,
    pub score: u32,
    pub title_matches: Vec<MatchRangeDto>,
    pub content_preview: Option<ContentPreviewDto>,
}

#[derive(Serialize)]
pub struct MatchRangeDto {
    pub start: u32,
    pub end: u32,
}

#[derive(Serialize)]
pub struct ContentPreviewDto {
    pub text: String,
    pub match_start: u32,
    pub match_end: u32,
}

// 変換実装
impl From<crate::domain::SearchResult> for SearchResultDto {
    fn from(r: crate::domain::SearchResult) -> Self {
        Self {
            uid: r.uid,
            title: r.title,
            score: r.score,
            title_matches: r.title_matches.into_iter().map(|m| MatchRangeDto {
                start: m.start,
                end: m.end,
            }).collect(),
            content_preview: r.content_preview.map(|p| ContentPreviewDto {
                text: p.text,
                match_start: p.match_start,
                match_end: p.match_end,
            }),
        }
    }
}
```

### 3.2 検索コマンド

**ファイル**: `src-tauri/src/commands/note.rs` に追加

```rust
/// ノートを検索
///
/// # Performance
/// - nucleo fuzzy matching (skim比6倍高速)
/// - rayon並列ファイル読み込み
/// - memmap2高速I/O
#[tauri::command]
pub fn search_notes(
    state: State<AppState>,
    query: String,
    limit: Option<usize>,
) -> Result<Vec<SearchResultDto>, String> {
    // クエリ長制限（DoS防止）
    if query.len() > 200 {
        return Err("Query too long (max 200 chars)".to_string());
    }

    state
        .search_service
        .search(&query, limit)
        .map(|results| results.into_iter().map(SearchResultDto::from).collect())
        .map_err(|e| e.to_string())
}
```

### 3.3 AppState に SearchService 追加

**ファイル**: `src-tauri/src/app_state.rs`

```rust
pub struct AppState {
    pub note_service: NoteService,
    pub settings_service: Arc<SettingsService>,
    pub search_service: SearchService,  // 追加
    // ...
}
```

### 3.4 main.rs でコマンド登録

```rust
.invoke_handler(tauri::generate_handler![
    // 既存コマンド...
    commands::search_notes,  // 追加
])
```

**完了条件**: Tauriアプリがビルド＆起動する

---

## Phase 4: Frontend 型定義 (10分)

### 4.1 型追加

**ファイル**: `src/lib/types/index.ts` に追加

```typescript
// 検索結果
export interface SearchResultDto {
  uid: string;
  title: string;
  score: number;
  title_matches: MatchRange[];
  content_preview: ContentPreview | null;
}

export interface MatchRange {
  start: number;
  end: number;
}

export interface ContentPreview {
  text: string;
  match_start: number;
  match_end: number;
}
```

### 4.2 API関数追加

**ファイル**: `src/lib/services/api.ts` に追加

```typescript
export async function searchNotes(
  query: string,
  limit?: number
): Promise<SearchResultDto[]> {
  return await invoke('search_notes', { query, limit });
}
```

**完了条件**: TypeScript型チェック通る

---

## Phase 5: 検索ストア (15分)

### 5.1 searchStore 作成

**ファイル**: `src/lib/stores/search.svelte.ts`

```typescript
// 検索ストア (Svelte 5 runes)

import { searchNotes } from '$lib/services/api';
import type { SearchResultDto } from '$lib/types';

// 内部状態
let query = $state('');
let results = $state<SearchResultDto[]>([]);
let isSearching = $state(false);
let error = $state<string | null>(null);

// デバウンスタイマー
let debounceTimer: ReturnType<typeof setTimeout> | null = null;
const DEBOUNCE_MS = 150;

export function useSearchStore() {
  return {
    // Getters
    get query() { return query; },
    get results() { return results; },
    get isSearching() { return isSearching; },
    get error() { return error; },
    get isActive() { return query.length > 0; },
    get hasResults() { return results.length > 0; },

    // Actions
    setQuery(newQuery: string) {
      query = newQuery;
      error = null;

      if (debounceTimer) {
        clearTimeout(debounceTimer);
      }

      if (!newQuery.trim()) {
        results = [];
        return;
      }

      debounceTimer = setTimeout(() => {
        this.executeSearch();
      }, DEBOUNCE_MS);
    },

    async executeSearch() {
      if (!query.trim()) {
        results = [];
        return;
      }

      isSearching = true;
      error = null;

      try {
        results = await searchNotes(query);
      } catch (e) {
        error = String(e);
        results = [];
      } finally {
        isSearching = false;
      }
    },

    clear() {
      query = '';
      results = [];
      error = null;
      if (debounceTimer) {
        clearTimeout(debounceTimer);
        debounceTimer = null;
      }
    },
  };
}

export const searchStore = useSearchStore();
```

**完了条件**: ストアがインポートできる

---

## Phase 6: UIコンポーネント (45分)

### 6.1 SearchInput コンポーネント

**ファイル**: `src/lib/components/SearchInput.svelte`

```svelte
<script lang="ts">
  interface Props {
    value: string;
    onInput: (value: string) => void;
    onClear: () => void;
    isSearching?: boolean;
  }

  let { value, onInput, onClear, isSearching = false }: Props = $props();
  let inputRef: HTMLInputElement;

  function handleInput(e: Event) {
    const target = e.target as HTMLInputElement;
    onInput(target.value);
  }

  function handleKeydown(e: KeyboardEvent) {
    if (e.key === 'Escape') {
      onClear();
      inputRef?.blur();
    }
  }

  export function focus() {
    inputRef?.focus();
  }
</script>

<div class="search-input">
  <svg class="search-icon" width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
    <circle cx="11" cy="11" r="8"></circle>
    <line x1="21" y1="21" x2="16.65" y2="16.65"></line>
  </svg>

  <input
    bind:this={inputRef}
    type="text"
    {value}
    placeholder="Search..."
    oninput={handleInput}
    onkeydown={handleKeydown}
    class:searching={isSearching}
  />

  {#if value}
    <button class="clear-btn" onclick={onClear} aria-label="Clear search">
      <svg width="12" height="12" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
        <line x1="18" y1="6" x2="6" y2="18"></line>
        <line x1="6" y1="6" x2="18" y2="18"></line>
      </svg>
    </button>
  {/if}
</div>

<style>
  .search-input {
    display: flex;
    align-items: center;
    gap: 8px;
    padding: 8px 12px;
    background: var(--bg-primary);
    border: 1px solid var(--border-color);
    border-radius: 6px;
    margin: 8px 12px;
  }

  .search-input:focus-within {
    border-color: var(--accent-blue);
  }

  .search-icon {
    flex-shrink: 0;
    color: var(--fg-muted);
  }

  input {
    flex: 1;
    border: none;
    background: transparent;
    color: var(--fg-primary);
    font-size: 13px;
    outline: none;
    min-width: 0;
  }

  input::placeholder {
    color: var(--fg-muted);
  }

  input.searching {
    opacity: 0.7;
  }

  .clear-btn {
    display: flex;
    align-items: center;
    justify-content: center;
    padding: 4px;
    color: var(--fg-muted);
    border-radius: 4px;
    transition: all 0.15s;
  }

  .clear-btn:hover {
    background: var(--bg-highlight);
    color: var(--fg-primary);
  }
</style>
```

### 6.2 HighlightText コンポーネント

**ファイル**: `src/lib/components/HighlightText.svelte`

```svelte
<script lang="ts">
  import type { MatchRange } from '$lib/types';

  interface Props {
    text: string;
    matches: MatchRange[];
  }

  let { text, matches }: Props = $props();

  // マッチ位置でテキストを分割
  function getSegments(text: string, matches: MatchRange[]): Array<{ text: string; highlight: boolean }> {
    if (!matches.length) {
      return [{ text, highlight: false }];
    }

    const segments: Array<{ text: string; highlight: boolean }> = [];
    let lastEnd = 0;

    // バイト位置を文字位置に変換
    const bytes = new TextEncoder().encode(text);

    for (const match of matches) {
      // バイト位置から文字位置を計算
      const startChar = new TextDecoder().decode(bytes.slice(0, match.start)).length;
      const endChar = new TextDecoder().decode(bytes.slice(0, match.end)).length;

      if (startChar > lastEnd) {
        segments.push({
          text: text.slice(lastEnd, startChar),
          highlight: false,
        });
      }

      segments.push({
        text: text.slice(startChar, endChar),
        highlight: true,
      });

      lastEnd = endChar;
    }

    if (lastEnd < text.length) {
      segments.push({
        text: text.slice(lastEnd),
        highlight: false,
      });
    }

    return segments;
  }

  const segments = $derived(getSegments(text, matches));
</script>

<span class="highlight-text">
  {#each segments as segment}
    {#if segment.highlight}
      <mark class="match">{segment.text}</mark>
    {:else}
      {segment.text}
    {/if}
  {/each}
</span>

<style>
  .highlight-text {
    display: inline;
  }

  .match {
    background: var(--accent-yellow-dim, rgba(224, 175, 104, 0.3));
    color: var(--fg-primary);
    border-radius: 2px;
    padding: 0 1px;
  }
</style>
```

### 6.3 SearchResults コンポーネント

**ファイル**: `src/lib/components/SearchResults.svelte`

```svelte
<script lang="ts">
  import type { SearchResultDto } from '$lib/types';
  import HighlightText from './HighlightText.svelte';

  interface Props {
    results: SearchResultDto[];
    onSelect: (uid: string) => void;
    focusedIndex?: number;
  }

  let { results, onSelect, focusedIndex = -1 }: Props = $props();
</script>

<ul class="search-results" role="listbox">
  {#each results as result, index (result.uid)}
    <li
      class="result-item"
      class:focused={focusedIndex === index}
      role="option"
      aria-selected={focusedIndex === index}
    >
      <button onclick={() => onSelect(result.uid)}>
        <span class="result-title">
          <HighlightText text={result.title || 'Untitled'} matches={result.title_matches} />
        </span>
        {#if result.content_preview}
          <span class="result-preview">
            {result.content_preview.text}
          </span>
        {/if}
      </button>
    </li>
  {:else}
    <li class="no-results">No results found</li>
  {/each}
</ul>

<style>
  .search-results {
    flex: 1;
    overflow-y: auto;
    padding: 4px 8px;
  }

  .result-item {
    border-radius: 6px;
    transition: background 0.15s;
  }

  .result-item.focused,
  .result-item:hover {
    background: var(--bg-highlight);
  }

  .result-item button {
    display: flex;
    flex-direction: column;
    align-items: flex-start;
    width: 100%;
    padding: 10px 12px;
    text-align: left;
  }

  .result-title {
    font-size: 13px;
    font-weight: 500;
    color: var(--fg-primary);
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
    width: 100%;
  }

  .result-preview {
    font-size: 11px;
    color: var(--fg-muted);
    margin-top: 4px;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
    width: 100%;
  }

  .no-results {
    padding: 24px;
    text-align: center;
    color: var(--fg-muted);
    font-size: 13px;
  }
</style>
```

**完了条件**: 全コンポーネントが構文エラーなし

---

## Phase 7: Sidebar統合 (30分)

### 7.1 Sidebar.svelte 修正

```svelte
<script lang="ts">
  // 既存のimport...
  import { searchStore } from '$lib/stores/search.svelte';
  import SearchInput from './SearchInput.svelte';
  import SearchResults from './SearchResults.svelte';

  // ... 既存のコード ...

  // 検索結果選択時の処理
  function handleSearchSelect(uid: string) {
    searchStore.clear();
    onNoteSelect(uid);
  }
</script>

<!-- テンプレート -->
<aside class="sidebar" class:open={isOpen}>
  <header class="sidebar-header">
    <h2>Notes</h2>
    <button class="icon-btn" onclick={onNewNote}><!-- + icon --></button>
  </header>

  <!-- 検索バー（新規追加） -->
  <SearchInput
    value={searchStore.query}
    onInput={(v) => searchStore.setQuery(v)}
    onClear={() => searchStore.clear()}
    isSearching={searchStore.isSearching}
  />

  <!-- 検索結果 or ノート一覧 -->
  {#if searchStore.isActive}
    <SearchResults
      results={searchStore.results}
      onSelect={handleSearchSelect}
    />
  {:else}
    <!-- 既存のノート一覧 -->
    <ul class="note-list">
      <!-- ... -->
    </ul>
  {/if}

  <footer class="sidebar-footer">
    <!-- 設定ボタン -->
  </footer>
</aside>
```

**完了条件**: 検索バーが表示され、入力で検索が動作する

---

## Phase 8: テスト (30分)

### 8.1 Rust ユニットテスト

**ファイル**: `src-tauri/src/services/search_service.rs` にテスト追加

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_skip_front_matter() {
        let service = SearchService::new(/* ... */);

        let content = b"---\nuid: test\n---\n\nHello World";
        let result = service.skip_front_matter(content);
        assert!(result.starts_with(b"\nHello"));
    }

    #[test]
    fn test_empty_query_returns_empty() {
        // ...
    }

    #[test]
    fn test_fuzzy_matching() {
        // ...
    }
}
```

### 8.2 手動テスト項目

- [ ] 空クエリで空結果
- [ ] 日本語検索が動作
- [ ] タイポ入力でもマッチ（fuzzy）
- [ ] 大量ノート（100+）でも高速（< 100ms）
- [ ] Escapeキーでクリア
- [ ] 検索結果クリックでノート開く

**完了条件**: 全テスト項目パス

---

## 完成チェックリスト

### Backend
- [ ] `nucleo-matcher`, `rayon`, `memmap2` クレート追加
- [ ] `domain/search.rs` 型定義
- [ ] `services/search_service.rs` 実装
- [ ] `commands/note.rs` に `search_notes` 追加
- [ ] `AppState` に `SearchService` 追加
- [ ] ユニットテスト

### Frontend
- [ ] `types/index.ts` に検索型追加
- [ ] `api.ts` に `searchNotes` 追加
- [ ] `stores/search.svelte.ts` 作成
- [ ] `SearchInput.svelte` 作成
- [ ] `HighlightText.svelte` 作成
- [ ] `SearchResults.svelte` 作成
- [ ] `Sidebar.svelte` 統合

### 動作確認
- [ ] 検索バー表示
- [ ] 入力で検索実行（150msデバウンス）
- [ ] 結果リスト表示
- [ ] マッチ箇所ハイライト
- [ ] 本文プレビュー表示
- [ ] 結果クリックでノート選択
- [ ] Escapeでクリア
- [ ] 日本語対応
- [ ] パフォーマンス確認

---

## パフォーマンス目標

| ノート数 | 目標応答時間 | 根拠 |
|---------|-------------|------|
| 100件 | < 30ms | memmap + rayon並列 |
| 500件 | < 80ms | 先頭4KBのみ検索 |
| 1000件 | < 150ms | nucleo高速アルゴリズム |

---

## Sources

- [nucleo - GitHub](https://github.com/helix-editor/nucleo)
- [nucleo-matcher - docs.rs](https://docs.rs/nucleo-matcher)
- [Rayon Optimization Guide](https://gendignoux.com/blog/2024/11/18/rust-rayon-optimized.html)
- [memmap2 - crates.io](https://crates.io/crates/memmap2)
- [memmap2 Guide](https://generalistprogrammer.com/tutorials/memmap2-rust-crate-guide)
