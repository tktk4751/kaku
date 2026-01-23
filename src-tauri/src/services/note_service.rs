use crate::domain::{DomainEvent, Note};
use crate::infrastructure::GalleryNote;
use crate::traits::{EventBus, NoteListItem, NoteRepository, RepositoryError};
use std::sync::Arc;

/// ノートサービス（ビジネスロジック層）
pub struct NoteService {
    repository: Arc<dyn NoteRepository>,
    event_bus: Arc<dyn EventBus>,
}

impl NoteService {
    pub fn new(repository: Arc<dyn NoteRepository>, event_bus: Arc<dyn EventBus>) -> Self {
        Self {
            repository,
            event_bus,
        }
    }

    /// 新規メモを作成（ファイルは保存しない、メモリ上のみ）
    pub fn create_note(&self) -> Result<Note, RepositoryError> {
        let note = Note::new();
        // ファイルは保存しない - 内容が入力されたときに初めて保存

        self.event_bus.emit(DomainEvent::NoteCreated {
            uid: note.metadata.uid.clone(),
        });

        Ok(note)
    }

    /// メモを保存
    pub fn save_note(&self, note: &Note) -> Result<(), RepositoryError> {
        self.repository.save(note)?;

        self.event_bus.emit(DomainEvent::SaveCompleted {
            uid: note.metadata.uid.clone(),
        });

        Ok(())
    }

    /// メモをロード
    pub fn load_note(&self, uid: &str) -> Result<Note, RepositoryError> {
        let note = self.repository.load(uid)?;

        self.event_bus.emit(DomainEvent::NoteLoaded {
            uid: uid.to_string(),
        });

        Ok(note)
    }

    /// メモを削除
    pub fn delete_note(&self, uid: &str) -> Result<(), RepositoryError> {
        self.repository.delete(uid)?;

        self.event_bus.emit(DomainEvent::NoteDeleted {
            uid: uid.to_string(),
        });

        Ok(())
    }

    /// 全メモ一覧を取得
    pub fn list_notes(&self) -> Result<Vec<NoteListItem>, RepositoryError> {
        self.repository.list_all()
    }

    /// ギャラリー用ノート一覧を取得（高速キャッシュ版）
    pub fn list_gallery_notes(
        &self,
        sort_by_created: bool,
        tag_filter: Option<&str>,
    ) -> Result<Vec<GalleryNote>, RepositoryError> {
        self.repository.list_gallery(sort_by_created, tag_filter)
    }
}
