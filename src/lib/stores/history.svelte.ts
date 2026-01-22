// ノート閲覧履歴の管理

interface HistoryState {
  /** 履歴スタック */
  stack: string[];
  /** 現在の位置（インデックス） */
  currentIndex: number;
}

function createHistoryStore() {
  let state = $state<HistoryState>({
    stack: [],
    currentIndex: -1,
  });

  // 最大履歴数
  const MAX_HISTORY = 10;

  return {
    /** 現在のUID */
    get currentUid(): string | null {
      if (state.currentIndex >= 0 && state.currentIndex < state.stack.length) {
        return state.stack[state.currentIndex];
      }
      return null;
    },

    /** 戻れるかどうか */
    get canGoBack(): boolean {
      return state.currentIndex > 0;
    },

    /** 進めるかどうか */
    get canGoForward(): boolean {
      return state.currentIndex < state.stack.length - 1;
    },

    /** ノートを開いた時に履歴に追加 */
    push(uid: string) {
      // 同じノートを連続で追加しない
      if (state.stack[state.currentIndex] === uid) {
        return;
      }

      // 現在位置より先の履歴を削除（新しい分岐を作る）
      const newStack = state.stack.slice(0, state.currentIndex + 1);

      // 新しいUIDを追加
      newStack.push(uid);

      // 最大数を超えたら古いものを削除
      if (newStack.length > MAX_HISTORY) {
        newStack.shift();
      }

      state = {
        stack: newStack,
        currentIndex: newStack.length - 1,
      };
    },

    /** 戻る - 前のノートのUIDを返す */
    goBack(): string | null {
      if (!this.canGoBack) {
        return null;
      }

      state = {
        ...state,
        currentIndex: state.currentIndex - 1,
      };

      return state.stack[state.currentIndex];
    },

    /** 進む - 次のノートのUIDを返す */
    goForward(): string | null {
      if (!this.canGoForward) {
        return null;
      }

      state = {
        ...state,
        currentIndex: state.currentIndex + 1,
      };

      return state.stack[state.currentIndex];
    },

    /** 履歴をクリア */
    clear() {
      state = {
        stack: [],
        currentIndex: -1,
      };
    },

    /** 特定のUIDを履歴から削除（ノート削除時） */
    remove(uid: string) {
      const newStack = state.stack.filter((u) => u !== uid);
      let newIndex = state.currentIndex;

      // インデックスを調整
      const removedBefore = state.stack.slice(0, state.currentIndex + 1).filter((u) => u === uid).length;
      newIndex -= removedBefore;

      // インデックスが範囲外にならないように調整
      if (newIndex >= newStack.length) {
        newIndex = newStack.length - 1;
      }
      if (newIndex < 0 && newStack.length > 0) {
        newIndex = 0;
      }

      state = {
        stack: newStack,
        currentIndex: newIndex,
      };
    },
  };
}

export const historyStore = createHistoryStore();
