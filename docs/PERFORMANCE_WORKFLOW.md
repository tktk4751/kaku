# カードビュー高速化ワークフロー

## 概要
カードビュー(HomeView)のパフォーマンスを最適化するための実装計画

## 優先度別タスク

---

## P0: Rustインデックスキャッシュ [最重要]

### 問題
現在`list_notes_gallery`は毎回全ノートをファイルから読み込んでいる (N+1問題)

### 解決策
SQLiteベースの軽量インデックスを導入し、メタデータ・プレビュー・タグをキャッシュ

### 実装ステップ

```
P0-1. SQLiteインデックススキーマ定義
      - notes_index テーブル: uid, title, preview, tags_json, created_at, updated_at, file_hash

P0-2. インデックス管理モジュール作成
      - src-tauri/src/infrastructure/sqlite_index.rs
      - 初期化、クエリ、更新、削除メソッド

P0-3. ファイル変更検知
      - ファイルハッシュで変更を検知
      - 変更があった場合のみ再パース

P0-4. gallery.rsをインデックス使用に変更
      - list_notes_gallery() をインデックスクエリに置換

P0-5. ノート保存時のインデックス更新
      - save_note() 後にインデックス更新
```

### 期待効果
- 10ノート: 100ms → 5ms (20x高速化)
- 100ノート: 1000ms → 10ms (100x高速化)

---

## P1: 仮想スクロール実装

### 問題
大量のNoteCardコンポーネントが同時にDOMに存在

### 解決策
可視領域のカードのみレンダリングする仮想スクロール

### 実装ステップ

```
P1-1. VirtualList.svelteコンポーネント作成
      - 可視領域計算
      - スクロール位置追跡
      - アイテム高さ推定

P1-2. HomeViewに統合
      - masonry-gridをVirtualListに置換
      - カード高さの動的計算対応

P1-3. スクロール復元
      - 前回位置の記憶と復元
```

### 期待効果
- DOM要素数: 100個 → 10-15個
- メモリ使用量: 大幅削減
- 初期描画: 高速化

---

## P2: タグキャッシュ化

### 問題
`getAllTags()`が毎レンダリングで全アイテムを走査

### 解決策
タグ一覧を$derivedでキャッシュし、itemsが変わったときのみ再計算

### 実装ステップ

```
P2-1. homeStoreにallTags derivedを追加
      - items変更時のみ再計算
      - ソート済みで返却

P2-2. HomeViewでキャッシュを使用
      - getAllTags() → homeStore.allTags
```

### 期待効果
- 不要な再計算を排除
- CPU使用率低減

---

## P3: filteredItems最適化

### 問題
`$derived(() => {...})`は関数を返しており、毎回`filteredItems()`で実行される

### 解決策
`$derived.by()`を使用して値を直接キャッシュ

### 実装ステップ

```
P3-1. $derived.by()に変更
      - filteredItems() → filteredItems (値として)

P3-2. 呼び出し箇所を修正
      - filteredItems() → filteredItems
```

### 期待効果
- 依存関係が変わらない限り再計算しない
- 描画パフォーマンス改善

---

## P4: CSS contain最適化

### 問題
ブラウザが各カードのレイアウト・ペイント計算を毎フレーム実行

### 解決策
CSS `contain` プロパティで再計算範囲を限定

### 実装ステップ

```
P4-1. NoteCard.svelteにcontainを追加
      - contain: layout style paint
      - content-visibility: auto (画面外は描画スキップ)

P4-2. masonry-gridにcontainを追加
      - contain: layout style

P4-3. will-changeの適切な使用
      - ホバー時のみtransformをヒント
```

### 期待効果
- リフロー範囲の限定
- 画面外カードの描画スキップ
- GPU合成の最適化

---

## 実装順序

```
Phase 1 - クイック最適化 (即時実装):
├── P3: filteredItems最適化 (15分)
├── P4: CSS contain最適化 (20分)
├── P2: タグキャッシュ化 (30分)
└── 検証

Phase 2 - バックエンド最適化:
├── P0-1: SQLiteスキーマ定義
├── P0-2: インデックスモジュール作成
├── P0-3: ファイル変更検知
├── P0-4: gallery.rs修正
└── P0-5: 保存時更新

Phase 3 - フロントエンド最適化:
├── P1-1: VirtualList作成
├── P1-2: HomeView統合
├── P1-3: スクロール復元
└── 最終テスト
```

## 依存関係

```
P3 ─┬─► P4 ─┬─► P2 (独立して実装可能、順番に効果検証)
    │       │
P0 ─┴───────┴─► P1 (P0完了後にP1が最も効果的)
```

## 検証指標

| 指標 | 現状 | 目標 |
|------|------|------|
| 初期表示時間 | ~1000ms | <100ms |
| スクロールFPS | ~30fps | 60fps |
| メモリ使用量 | 高 | 50%削減 |
| リフロー範囲 | 全体 | カード単位 |

## 各フェーズの期待効果

| フェーズ | タスク | 効果 |
|----------|--------|------|
| Phase 1 | P3+P4+P2 | 体感20-30%改善 |
| Phase 2 | P0 | 10-100x高速化 (データ取得) |
| Phase 3 | P1 | 大量データ対応、メモリ削減 |
