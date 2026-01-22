//! バックリンクサービス
//!
//! ウィキリンクのインデックスを管理し、バックリンクの検索を提供する

use crate::domain::{extract_context, extract_wiki_links, BacklinkInfo, SearchError};
use crate::traits::NoteRepository;
use parking_lot::RwLock;
use std::collections::{HashMap, HashSet};
use std::sync::Arc;

/// コンテキスト抽出の文字数
const CONTEXT_CHARS: usize = 40;

/// バックリンクインデックス
///
/// target_title (lowercase) -> source_uids
struct BacklinkIndex {
    /// タイトル -> リンク元UIDのセット
    links: HashMap<String, HashSet<String>>,
    /// UID -> タイトル (キャッシュ)
    titles: HashMap<String, String>,
    /// UID -> コンテンツ (キャッシュ)
    contents: HashMap<String, String>,
}

impl BacklinkIndex {
    fn new() -> Self {
        Self {
            links: HashMap::new(),
            titles: HashMap::new(),
            contents: HashMap::new(),
        }
    }

    /// ノートのリンクをインデックスに追加
    fn index_note(&mut self, uid: &str, title: &str, content: &str) {
        // 古いリンクを削除
        self.remove_links_from(uid);

        // タイトルとコンテンツをキャッシュ
        self.titles.insert(uid.to_string(), title.to_string());
        self.contents.insert(uid.to_string(), content.to_string());

        // 新しいリンクを追加
        let links = extract_wiki_links(content);
        for link in links {
            let target_key = link.title.to_lowercase();
            self.links
                .entry(target_key)
                .or_default()
                .insert(uid.to_string());
        }
    }

    /// ノートからのリンクを削除
    fn remove_links_from(&mut self, uid: &str) {
        // 全てのターゲットからこのUIDを削除
        for sources in self.links.values_mut() {
            sources.remove(uid);
        }
        // 空のエントリを削除
        self.links.retain(|_, sources| !sources.is_empty());
    }

    /// タイトルに対するバックリンクを取得
    fn get_backlinks(&self, title: &str) -> Vec<BacklinkInfo> {
        let target_key = title.to_lowercase();

        let Some(source_uids) = self.links.get(&target_key) else {
            return Vec::new();
        };

        source_uids
            .iter()
            .filter_map(|uid| {
                let source_title = self.titles.get(uid)?.clone();
                let content = self.contents.get(uid)?;

                // リンクの位置を見つけてコンテキストを抽出
                let links = extract_wiki_links(content);
                let position = links
                    .iter()
                    .find(|l| l.title.to_lowercase() == target_key)
                    .map(|l| l.position)
                    .unwrap_or(0);

                let context = extract_context(content, position, CONTEXT_CHARS);

                Some(BacklinkInfo {
                    source_uid: uid.clone(),
                    source_title,
                    context,
                })
            })
            .collect()
    }

    /// UIDに対するバックリンクを取得
    fn get_backlinks_for_uid(&self, uid: &str) -> Vec<BacklinkInfo> {
        // UIDからタイトルを取得
        let Some(title) = self.titles.get(uid) else {
            return Vec::new();
        };
        self.get_backlinks(title)
    }
}

/// バックリンクサービス
pub struct BacklinkService {
    index: RwLock<BacklinkIndex>,
    repository: Arc<dyn NoteRepository>,
}

impl BacklinkService {
    pub fn new(repository: Arc<dyn NoteRepository>) -> Self {
        Self {
            index: RwLock::new(BacklinkIndex::new()),
            repository,
        }
    }

    /// 全ノートからインデックスを再構築
    pub fn rebuild_index(&self) -> Result<(), SearchError> {
        let notes = self.repository.list_all()?;
        let mut index = self.index.write();

        // インデックスをクリア
        *index = BacklinkIndex::new();

        // 各ノートをインデックス
        for note_item in &notes {
            if let Ok(note) = self.repository.load(&note_item.uid) {
                index.index_note(&note_item.uid, &note_item.title, &note.content);
            }
        }

        println!(
            "[BacklinkService] Rebuilt index: {} notes, {} link targets",
            notes.len(),
            index.links.len()
        );

        Ok(())
    }

    /// ノート保存時にインデックスを更新
    pub fn update_note(&self, uid: &str, title: &str, content: &str) {
        let mut index = self.index.write();
        index.index_note(uid, title, content);
    }

    /// ノート削除時にインデックスから削除
    pub fn remove_note(&self, uid: &str) {
        let mut index = self.index.write();
        index.remove_links_from(uid);
        index.titles.remove(uid);
        index.contents.remove(uid);
    }

    /// タイトルに対するバックリンクを取得
    pub fn get_backlinks(&self, title: &str) -> Vec<BacklinkInfo> {
        let index = self.index.read();
        index.get_backlinks(title)
    }

    /// UIDに対するバックリンクを取得
    pub fn get_backlinks_for_uid(&self, uid: &str) -> Vec<BacklinkInfo> {
        let index = self.index.read();
        index.get_backlinks_for_uid(uid)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // モックリポジトリは統合テストで使用
}
