// ノートストア (Svelte 5 runes)
//
// SOLID: Single Responsibility
// - データ層（内部状態）とアクション層（公開API）を分離
// - 外部インターフェースは変更なし（後方互換性維持）

import { createNote, saveNote, loadNote, listNotes, deleteNote } from '$lib/services/api';
import { historyStore } from '$lib/stores/history.svelte';
import type { NoteDto, NoteListItemDto, AppError } from '$lib/types';
import { parseAppError } from '$lib/types';

interface LoadOptions {
  /** Skip adding to history (used when navigating via back/forward) */
  skipHistory?: boolean;
}

// ===== 内部データ層（外部非公開）=====
let currentNote = $state<NoteDto | null>(null);
let noteList = $state<NoteListItemDto[]>([]);
let isSaving = $state(false);
let isDirty = $state(false);
let saveError = $state<AppError | null>(null);

// 自動保存タイマー
let autosaveTimer: ReturnType<typeof setTimeout> | null = null;

// 即時保存のデバウンス時間（ミリ秒）
const AUTOSAVE_DELAY_MS = 50;

// ===== 内部データ操作（テスト用にエクスポート）=====
// これにより、ユニットテストで状態を直接操作できる
export const _internal = {
  setCurrentNote(note: NoteDto | null) { currentNote = note; },
  setNoteList(list: NoteListItemDto[]) { noteList = list; },
  setDirty(dirty: boolean) { isDirty = dirty; },
  setSaving(saving: boolean) { isSaving = saving; },
  setError(error: AppError | null) { saveError = error; },
  getAutosaveTimer() { return autosaveTimer; },
  /** Cleanup function - call when store is no longer needed */
  cleanup() {
    if (autosaveTimer) {
      clearTimeout(autosaveTimer);
      autosaveTimer = null;
    }
  },
};

// ===== 公開API（既存と完全互換）=====
export function useNoteStore() {
  return {
    // Getters（変更なし）
    get currentNote() { return currentNote; },
    get noteList() { return noteList; },
    get isSaving() { return isSaving; },
    get isDirty() { return isDirty; },
    get saveError() { return saveError; },

    async createNew() {
      try {
        currentNote = await createNote();
        isDirty = false;
        saveError = null;
        // 新規ノートを即座にファイルとして保存
        await saveNote(currentNote.uid, currentNote.content);
        await this.refreshList();
        // 履歴に追加
        historyStore.push(currentNote.uid);
      } catch (e) {
        saveError = parseAppError(e);
        throw e;
      }
    },

    async load(uid: string, options?: LoadOptions) {
      try {
        currentNote = await loadNote(uid);
        isDirty = false;
        saveError = null;
        // 履歴に追加（skipHistoryが指定されていない場合）
        if (!options?.skipHistory) {
          historyStore.push(uid);
        }
      } catch (e) {
        saveError = parseAppError(e);
        throw e;
      }
    },

    async save() {
      if (!currentNote || isSaving) return;

      isSaving = true;
      saveError = null;

      try {
        // 空でも保存する（削除は非表示時に行う）
        await saveNote(currentNote.uid, currentNote.content);
        isDirty = false;
        await this.refreshList();
      } catch (e) {
        saveError = parseAppError(e);
        throw e;
      } finally {
        isSaving = false;
      }
    },

    // 空のノートを削除（非表示時に呼び出す）
    async deleteIfEmpty() {
      if (!currentNote) return false;

      const contentIsEmpty = !currentNote.content.trim();
      if (contentIsEmpty) {
        try {
          await deleteNote(currentNote.uid);
          await this.refreshList();
          return true;
        } catch (e) {
          console.log('Note deletion skipped:', e);
        }
      }
      return false;
    },

    updateContent(content: string) {
      if (!currentNote) return;
      if (currentNote.content !== content) {
        currentNote = { ...currentNote, content };
        isDirty = true;
        this.scheduleAutosave();
      }
    },

    scheduleAutosave() {
      if (autosaveTimer) {
        clearTimeout(autosaveTimer);
      }
      autosaveTimer = setTimeout(() => {
        this.save().catch(console.error);
      }, AUTOSAVE_DELAY_MS);
    },

    cancelAutosave() {
      if (autosaveTimer) {
        clearTimeout(autosaveTimer);
        autosaveTimer = null;
      }
    },

    async refreshList() {
      try {
        noteList = await listNotes();
      } catch (e) {
        console.error('Failed to refresh note list:', e);
      }
    },

    async delete(uid: string) {
      try {
        await deleteNote(uid);

        // 履歴から削除
        historyStore.remove(uid);

        // If we deleted the current note, load another one
        if (currentNote?.uid === uid) {
          await this.refreshList();
          if (noteList.length > 0) {
            await this.load(noteList[0].uid);
          } else {
            await this.createNew();
          }
        } else {
          await this.refreshList();
        }

        saveError = null;
      } catch (e) {
        saveError = parseAppError(e);
        throw e;
      }
    },
  };
}

// シングルトンインスタンス
export const noteStore = useNoteStore();
