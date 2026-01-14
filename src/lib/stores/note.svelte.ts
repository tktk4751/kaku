// ノートストア (Svelte 5 runes)
import { createNote, saveNote, loadNote, listNotes, deleteNote } from '$lib/services/api';
import type { NoteDto, NoteListItemDto } from '$lib/types';

// 現在のノート状態
let currentNote = $state<NoteDto | null>(null);
let noteList = $state<NoteListItemDto[]>([]);
let isSaving = $state(false);
let isDirty = $state(false);
let saveError = $state<string | null>(null);

// 自動保存タイマー
let autosaveTimer: ReturnType<typeof setTimeout> | null = null;

// 即時保存のデバウンス時間（ミリ秒）
const AUTOSAVE_DELAY_MS = 50;

export function useNoteStore() {
  return {
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
      } catch (e) {
        saveError = String(e);
        throw e;
      }
    },

    async load(uid: string) {
      try {
        currentNote = await loadNote(uid);
        isDirty = false;
        saveError = null;
      } catch (e) {
        saveError = String(e);
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
        saveError = String(e);
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
        saveError = String(e);
        throw e;
      }
    },
  };
}

// シングルトンインスタンス
export const noteStore = useNoteStore();
