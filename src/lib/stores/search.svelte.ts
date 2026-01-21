// 検索ストア (Svelte 5 runes)
//
// 高速ファジー検索の状態管理
// - デバウンス付き入力
// - Rust側のnucleo-matcherで検索実行

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
