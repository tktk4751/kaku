# ノート検索機能 設計仕様書

## 1. 概要

サイドバーにファジー検索機能を追加し、タイトルと本文から高速にノートを検索できるようにする。

### 1.1 目標
- **超高速検索**: 数百件のノートでも瞬時に結果表示
- **曖昧検索**: タイポや部分一致でもヒット（fuzzy matching）
- **コンテキスト表示**: マッチ箇所の前後をプレビュー表示

### 1.2 技術選定

| 項目 | 選定 | 理由 |
|------|------|------|
| Fuzzy検索エンジン | **nucleo** | skim比6倍高速、Helix editor実績、Unicode対応 |
| 実行場所 | Rust (Backend) | ファイルアクセス効率、並列処理 |
| インデックス方式 | オンデマンド読み込み | シンプル、数百件なら十分高速 |

---

## 2. UI仕様

### 2.1 検索バー配置

```
┌─────────────────────────────────┐
│  Notes                     [+]  │  ← ヘッダー（既存）
├─────────────────────────────────┤
│  🔍 [検索...              ]     │  ← 新規: 検索入力欄
├─────────────────────────────────┤
│  📝 マッチしたタイトル          │
│     ...前後のコンテキスト...    │  ← マッチ箇所プレビュー
├─────────────────────────────────┤
│  📝 別のノート                  │
│     ...プレビュー...            │
└─────────────────────────────────┘
```

### 2.2 検索バーの振る舞い

| 項目 | 仕様 |
|------|------|
| 位置 | サイドバーヘッダー直下 |
| プレースホルダー | "Search..." |
| デバウンス | 150ms（入力停止後に検索実行） |
| クリアボタン | 入力があるときのみ表示（×アイコン） |
| キーボード | `Escape`で検索クリア、`Enter`で最初の結果を選択 |
| フォーカス | サイドバー開時に自動フォーカス（オプション） |

### 2.3 検索結果の表示

| 項目 | 仕様 |
|------|------|
| ソート順 | スコア降順（マッチ度が高い順） |
| 表示件数 | 最大50件（パフォーマンス考慮） |
| タイトル | マッチ部分をハイライト |
| プレビュー | マッチ箇所の前後30文字程度を表示 |
| 空結果 | "No results found" メッセージ |

### 2.4 ハイライト表示

マッチした文字は視覚的に強調:

```css
.search-match {
  background: var(--accent-yellow-dim);
  color: var(--fg-primary);
  border-radius: 2px;
}
```

---

## 3. データ構造

### 3.1 検索リクエスト (Frontend → Backend)

```typescript
// Frontend: src/lib/services/api.ts
interface SearchRequest {
  query: string;       // 検索クエリ
  limit?: number;      // 最大件数（デフォルト: 50）
}
```

### 3.2 検索結果 (Backend → Frontend)

```typescript
// Frontend: src/lib/types/index.ts
interface SearchResultDto {
  uid: string;                    // ノートUID
  title: string;                  // ノートタイトル
  score: number;                  // マッチスコア（0-1000）
  title_matches: MatchRange[];    // タイトル内のマッチ位置
  content_matches: ContentMatch[]; // 本文内のマッチ情報
}

interface MatchRange {
  start: number;  // 開始位置（UTF-16 code unit）
  end: number;    // 終了位置
}

interface ContentMatch {
  text: string;           // マッチ箇所の前後テキスト（60文字程度）
  match_start: number;    // text内でのマッチ開始位置
  match_end: number;      // text内でのマッチ終了位置
}
```

### 3.3 Rust側データ構造

```rust
// src-tauri/src/commands/mod.rs
#[derive(Serialize)]
pub struct SearchResultDto {
    pub uid: String,
    pub title: String,
    pub score: u32,
    pub title_matches: Vec<MatchRangeDto>,
    pub content_matches: Vec<ContentMatchDto>,
}

#[derive(Serialize)]
pub struct MatchRangeDto {
    pub start: usize,
    pub end: usize,
}

#[derive(Serialize)]
pub struct ContentMatchDto {
    pub text: String,
    pub match_start: usize,
    pub match_end: usize,
}
```

---

## 4. API仕様

### 4.1 Tauriコマンド

```rust
// src-tauri/src/commands/note.rs

/// ノートを検索
///
/// # Arguments
/// * `query` - 検索クエリ（空文字の場合は全件返却）
/// * `limit` - 最大結果数（デフォルト: 50、最大: 100）
///
/// # Returns
/// スコア降順でソートされた検索結果
#[tauri::command]
pub fn search_notes(
    state: State<AppState>,
    query: String,
    limit: Option<usize>,
) -> Result<Vec<SearchResultDto>, String>
```

### 4.2 Frontend API

```typescript
// src/lib/services/api.ts

export async function searchNotes(
  query: string,
  limit?: number
): Promise<SearchResultDto[]> {
  return await invoke('search_notes', { query, limit });
}
```

---

## 5. 実装詳細

### 5.1 Rust検索サービス

```
src-tauri/src/
├── services/
│   ├── mod.rs
│   ├── note_service.rs
│   └── search_service.rs    ← 新規
├── commands/
│   ├── mod.rs
│   └── note.rs              ← search_notes追加
└── domain/
    └── search.rs            ← 新規（検索結果の型定義）
```

### 5.2 SearchService 設計

```rust
// src-tauri/src/services/search_service.rs

use nucleo_matcher::{Matcher, Config};
use nucleo_matcher::pattern::{Pattern, CaseMatching, Normalization};

pub struct SearchService {
    repository: Arc<dyn NoteRepository>,
    storage: Arc<dyn Storage>,
}

impl SearchService {
    /// 検索実行
    pub fn search(&self, query: &str, limit: usize) -> Result<Vec<SearchResult>, SearchError> {
        // 1. 全ノートのメタデータ取得
        let notes = self.repository.list_all()?;

        // 2. nucleo matcher初期化
        let mut matcher = Matcher::new(Config::DEFAULT);
        let pattern = Pattern::parse(query, CaseMatching::Ignore, Normalization::Smart);

        // 3. 各ノートをスコアリング
        let mut results: Vec<SearchResult> = notes
            .par_iter()  // rayon並列処理
            .filter_map(|note| self.match_note(&matcher, &pattern, note))
            .collect();

        // 4. スコア降順ソート
        results.sort_by(|a, b| b.score.cmp(&a.score));

        // 5. 上位N件を返却
        Ok(results.into_iter().take(limit).collect())
    }

    fn match_note(&self, matcher: &Matcher, pattern: &Pattern, item: &NoteListItem)
        -> Option<SearchResult>
    {
        // タイトルマッチング
        let title_score = pattern.score(Utf32Str::from(&item.title), &mut matcher);

        // 本文マッチング（ファイル読み込み）
        let content = self.storage.load(&item.path).ok()?;
        let content_score = pattern.score(Utf32Str::from(&content), &mut matcher);

        // スコア計算（タイトル優先）
        let total_score = title_score.unwrap_or(0) * 2 + content_score.unwrap_or(0);

        if total_score > 0 {
            Some(SearchResult {
                uid: item.uid.clone(),
                title: item.title.clone(),
                score: total_score,
                title_matches: self.extract_matches(pattern, &item.title),
                content_matches: self.extract_content_matches(pattern, &content),
            })
        } else {
            None
        }
    }
}
```

### 5.3 依存クレート追加

```toml
# src-tauri/Cargo.toml

[dependencies]
nucleo-matcher = "0.3"  # Fuzzy matching
rayon = "1.8"           # 並列処理（オプション）
```

### 5.4 Frontend検索ストア

```typescript
// src/lib/stores/search.svelte.ts

let searchQuery = $state('');
let searchResults = $state<SearchResultDto[]>([]);
let isSearching = $state(false);

let debounceTimer: ReturnType<typeof setTimeout> | null = null;

export function useSearchStore() {
  return {
    get query() { return searchQuery; },
    get results() { return searchResults; },
    get isSearching() { return isSearching; },
    get isActive() { return searchQuery.length > 0; },

    setQuery(query: string) {
      searchQuery = query;
      this.debouncedSearch();
    },

    clear() {
      searchQuery = '';
      searchResults = [];
    },

    debouncedSearch() {
      if (debounceTimer) clearTimeout(debounceTimer);
      debounceTimer = setTimeout(() => this.search(), 150);
    },

    async search() {
      if (!searchQuery.trim()) {
        searchResults = [];
        return;
      }

      isSearching = true;
      try {
        searchResults = await searchNotes(searchQuery);
      } catch (e) {
        console.error('Search failed:', e);
        searchResults = [];
      } finally {
        isSearching = false;
      }
    }
  };
}

export const searchStore = useSearchStore();
```

---

## 6. Sidebar変更

### 6.1 コンポーネント構造

```svelte
<!-- src/lib/components/Sidebar.svelte -->

<aside class="sidebar" class:open={isOpen}>
  <header class="sidebar-header">
    <h2>Notes</h2>
    <button class="icon-btn" onclick={onNewNote}>+</button>
  </header>

  <!-- 新規: 検索バー -->
  <div class="search-container">
    <SearchInput
      bind:value={searchStore.query}
      onClear={() => searchStore.clear()}
    />
  </div>

  <!-- 検索結果 or ノート一覧 -->
  {#if searchStore.isActive}
    <SearchResults
      results={searchStore.results}
      onSelect={handleNoteSelect}
    />
  {:else}
    <NoteList
      notes={noteStore.noteList}
      onSelect={handleNoteSelect}
    />
  {/if}

  <footer class="sidebar-footer">
    <!-- 設定ボタン -->
  </footer>
</aside>
```

### 6.2 SearchInputコンポーネント

```svelte
<!-- src/lib/components/SearchInput.svelte -->

<script lang="ts">
  interface Props {
    value: string;
    onClear: () => void;
  }

  let { value = $bindable(), onClear }: Props = $props();
  let inputRef: HTMLInputElement;

  function handleKeydown(e: KeyboardEvent) {
    if (e.key === 'Escape') {
      onClear();
      inputRef.blur();
    }
  }
</script>

<div class="search-input-container">
  <svg class="search-icon"><!-- 虫眼鏡 --></svg>
  <input
    bind:this={inputRef}
    bind:value
    type="text"
    placeholder="Search..."
    onkeydown={handleKeydown}
  />
  {#if value}
    <button class="clear-btn" onclick={onClear}>×</button>
  {/if}
</div>
```

### 6.3 SearchResultsコンポーネント

```svelte
<!-- src/lib/components/SearchResults.svelte -->

<script lang="ts">
  import type { SearchResultDto } from '$lib/types';
  import HighlightText from './HighlightText.svelte';

  interface Props {
    results: SearchResultDto[];
    onSelect: (uid: string) => void;
  }

  let { results, onSelect }: Props = $props();
</script>

<ul class="search-results">
  {#each results as result (result.uid)}
    <li class="search-result-item">
      <button onclick={() => onSelect(result.uid)}>
        <span class="result-title">
          <HighlightText
            text={result.title}
            matches={result.title_matches}
          />
        </span>
        {#if result.content_matches.length > 0}
          <span class="result-preview">
            <HighlightText
              text={result.content_matches[0].text}
              matches={[{
                start: result.content_matches[0].match_start,
                end: result.content_matches[0].match_end
              }]}
            />
          </span>
        {/if}
      </button>
    </li>
  {:else}
    <li class="no-results">No results found</li>
  {/each}
</ul>
```

---

## 7. パフォーマンス考慮

### 7.1 最適化ポイント

| 項目 | 対策 |
|------|------|
| デバウンス | 150msで入力をまとめる |
| 並列処理 | rayon でファイル読み込みを並列化 |
| 早期終了 | limit到達で打ち切り可能 |
| キャッシュ | ファイル内容は毎回読み込み（シンプル優先） |

### 7.2 期待性能

| ノート数 | 期待応答時間 |
|---------|-------------|
| ~100件 | < 50ms |
| ~500件 | < 100ms |
| ~1000件 | < 200ms |

---

## 8. 将来の拡張

### 8.1 Phase 2（オプション）
- [ ] メモリキャッシュ（検索インデックス永続化）
- [ ] タグ検索対応
- [ ] 日付フィルター

### 8.2 Phase 3（オプション）
- [ ] 全文検索インデックス（tantivy）
- [ ] 検索履歴
- [ ] 保存済み検索

---

## 9. 実装チェックリスト

### Backend (Rust)
- [ ] `nucleo-matcher` クレート追加
- [ ] `SearchService` 実装
- [ ] `search_notes` コマンド追加
- [ ] 検索結果DTO定義
- [ ] ユニットテスト

### Frontend (Svelte)
- [ ] `SearchResultDto` 型定義追加
- [ ] `searchNotes` API関数追加
- [ ] `searchStore` 実装
- [ ] `SearchInput` コンポーネント
- [ ] `SearchResults` コンポーネント
- [ ] `HighlightText` コンポーネント
- [ ] `Sidebar` に検索UI統合
- [ ] スタイリング

### テスト
- [ ] 空クエリでの動作
- [ ] 日本語検索
- [ ] 大量ノートでのパフォーマンス
- [ ] キーボードナビゲーション
