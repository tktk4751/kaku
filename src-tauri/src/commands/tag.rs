// タグ関連コマンド

use crate::AppState;
use tauri::State;
use serde::Serialize;

/// タグ更新リクエスト検証
fn validate_tags(tags: &[String]) -> Result<(), String> {
    // タグ数制限
    if tags.len() > 50 {
        return Err("Too many tags (max 50)".to_string());
    }

    // 各タグの検証
    for tag in tags {
        if tag.len() > 100 {
            return Err(format!("Tag too long: {} (max 100 chars)", tag));
        }
        if tag.is_empty() {
            return Err("Empty tag not allowed".to_string());
        }
    }

    Ok(())
}

/// ノートのタグを更新（フロントマター）
#[tauri::command]
pub fn update_note_tags(
    state: State<AppState>,
    uid: String,
    tags: Vec<String>,
) -> Result<(), String> {
    // 入力検証
    super::note::validate_uid(&uid)?;
    validate_tags(&tags)?;

    // ノートをロード
    let mut note = state
        .note_service
        .load_note(&uid)
        .map_err(|e| e.to_string())?;

    // タグを更新
    note.update_tags(tags);

    // 保存
    state
        .note_service
        .save_note(&note)
        .map_err(|e| e.to_string())?;

    Ok(())
}

/// 全タグを取得（オートコンプリート用）
#[tauri::command]
pub fn get_all_tags(state: State<AppState>) -> Result<Vec<String>, String> {
    // 全ノートからタグを収集
    let notes = state
        .note_service
        .list_notes()
        .map_err(|e| e.to_string())?;

    let mut all_tags: Vec<String> = Vec::new();

    for note_item in notes {
        // 各ノートをロードしてタグを取得
        if let Ok(note) = state.note_service.load_note(&note_item.uid) {
            for tag in note.all_tags() {
                if !all_tags.contains(&tag) {
                    all_tags.push(tag);
                }
            }
        }
    }

    // アルファベット順にソート
    all_tags.sort();

    Ok(all_tags)
}

/// ノートのタグを取得
#[tauri::command]
pub fn get_note_tags(state: State<AppState>, uid: String) -> Result<NoteTagsDto, String> {
    super::note::validate_uid(&uid)?;

    let note = state
        .note_service
        .load_note(&uid)
        .map_err(|e| e.to_string())?;

    Ok(NoteTagsDto {
        frontmatter_tags: note.tags().to_vec(),
        hashtags: note.extract_hashtags(),
        all_tags: note.all_tags(),
    })
}

/// ノートのタグDTO
#[derive(Debug, Clone, Serialize)]
pub struct NoteTagsDto {
    /// フロントマターのタグ
    pub frontmatter_tags: Vec<String>,
    /// 本文から抽出したハッシュタグ
    pub hashtags: Vec<String>,
    /// 全タグ（マージ済み）
    pub all_tags: Vec<String>,
}
