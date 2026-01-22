//! バックリンク関連コマンド

use crate::domain::BacklinkInfo;
use crate::AppState;
use serde::{Deserialize, Serialize};
use tauri::State;

/// バックリンク DTO
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BacklinkDto {
    pub uid: String,
    pub title: String,
    pub context: String,
}

impl From<BacklinkInfo> for BacklinkDto {
    fn from(info: BacklinkInfo) -> Self {
        Self {
            uid: info.source_uid,
            title: info.source_title,
            context: info.context,
        }
    }
}

/// 指定ノートへのバックリンクを取得
#[tauri::command]
pub fn get_backlinks(state: State<AppState>, uid: String) -> Result<Vec<BacklinkDto>, String> {
    let backlinks = state.backlink_service.get_backlinks_for_uid(&uid);
    Ok(backlinks.into_iter().map(BacklinkDto::from).collect())
}

/// バックリンクインデックスを再構築
#[tauri::command]
pub fn rebuild_backlink_index(state: State<AppState>) -> Result<(), String> {
    state
        .backlink_service
        .rebuild_index()
        .map_err(|e| e.to_string())
}
