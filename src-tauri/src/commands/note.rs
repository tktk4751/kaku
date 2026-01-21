// ノート関連コマンド
//
// SOLID: Input Validation
// フロントエンドからの入力を信頼せず、バックエンドで検証する

use super::{NoteDto, NoteListItemDto};
use crate::AppState;
use tauri::State;

// ===== 入力検証 =====

/// UID検証: タイムスタンプ形式（数字のみ、14-20文字）
///
/// 形式: YYYYMMDDHHmmss + ナノ秒下6桁
/// 例: "2026011418102637208" (19-20文字)
///
/// # セキュリティ
///
/// - 数字のみ許可（パストラバーサル防止）
/// - 長さ制限（DoS防止）
fn validate_uid(uid: &str) -> Result<(), String> {
    // 空チェック
    if uid.is_empty() {
        return Err("UID cannot be empty".to_string());
    }

    // 長さチェック（タイムスタンプ形式: 14-20文字）
    // 14文字: YYYYMMDDHHmmss
    // +6文字: ナノ秒の下6桁
    if uid.len() < 14 || uid.len() > 26 {
        return Err(format!("Invalid UID length: expected 14-26, got {}", uid.len()));
    }

    // 文字チェック（数字と英字のみ）
    for c in uid.chars() {
        if !c.is_ascii_alphanumeric() {
            return Err(format!("Invalid character in UID: '{}'", c));
        }
    }

    Ok(())
}

/// コンテンツサイズ検証（最大10MB）
const MAX_CONTENT_SIZE: usize = 10 * 1024 * 1024;

fn validate_content(content: &str) -> Result<(), String> {
    if content.len() > MAX_CONTENT_SIZE {
        return Err(format!(
            "Content too large: {} bytes (max {} bytes)",
            content.len(),
            MAX_CONTENT_SIZE
        ));
    }
    Ok(())
}

/// 新規メモを作成
#[tauri::command]
pub fn create_note(state: State<AppState>) -> Result<NoteDto, String> {
    state
        .note_service
        .create_note()
        .map(NoteDto::from)
        .map_err(|e| e.to_string())
}

/// メモを保存
#[tauri::command]
pub fn save_note(state: State<AppState>, uid: String, content: String) -> Result<(), String> {
    // 入力検証
    validate_uid(&uid)?;
    validate_content(&content)?;

    // 既存ノートをロード、なければ新規作成
    let mut note = match state.note_service.load_note(&uid) {
        Ok(n) => n,
        Err(_) => {
            // ノートが存在しない場合は新規作成（UIDを保持）
            crate::domain::Note::with_uid(uid)
        }
    };

    note.update_content(content);
    state
        .note_service
        .save_note(&note)
        .map_err(|e| e.to_string())
}

/// メモをロード
#[tauri::command]
pub fn load_note(state: State<AppState>, uid: String) -> Result<NoteDto, String> {
    validate_uid(&uid)?;

    state
        .note_service
        .load_note(&uid)
        .map(NoteDto::from)
        .map_err(|e| e.to_string())
}

/// メモを削除
#[tauri::command]
pub fn delete_note(state: State<AppState>, uid: String) -> Result<(), String> {
    validate_uid(&uid)?;

    state
        .note_service
        .delete_note(&uid)
        .map_err(|e| e.to_string())
}

/// 全メモ一覧を取得
#[tauri::command]
pub fn list_notes(state: State<AppState>) -> Result<Vec<NoteListItemDto>, String> {
    state
        .note_service
        .list_notes()
        .map(|items| items.into_iter().map(NoteListItemDto::from).collect())
        .map_err(|e| e.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_validate_uid_valid() {
        // 有効なタイムスタンプ形式（19-20文字）
        assert!(validate_uid("2026011418102637208").is_ok());
        assert!(validate_uid("20260114181236685512").is_ok());
        // 最小長（14文字: YYYYMMDDHHmmss）
        assert!(validate_uid("20260114181026").is_ok());
        // 26文字も許可（将来のULID対応）
        assert!(validate_uid("01ARZ3NDEKTSV4RRFFQ69G5FAV").is_ok());
    }

    #[test]
    fn test_validate_uid_empty() {
        assert!(validate_uid("").is_err());
    }

    #[test]
    fn test_validate_uid_wrong_length() {
        assert!(validate_uid("2026011").is_err()); // 短すぎ（<14）
        assert!(validate_uid("01ARZ3NDEKTSV4RRFFQ69G5FAVXXX").is_err()); // 長すぎ（>26）
    }

    #[test]
    fn test_validate_uid_invalid_chars() {
        // パス区切り文字
        assert!(validate_uid("20260114181026/x").is_err());
        assert!(validate_uid("20260114181026\\x").is_err());
        // 特殊文字
        assert!(validate_uid("20260114181026..").is_err());
        assert!(validate_uid("20260114181026--").is_err());
    }

    #[test]
    fn test_validate_content_valid() {
        assert!(validate_content("Hello, World!").is_ok());
        assert!(validate_content("").is_ok()); // 空は許可
        assert!(validate_content(&"a".repeat(1000)).is_ok());
    }

    #[test]
    fn test_validate_content_too_large() {
        let large_content = "a".repeat(MAX_CONTENT_SIZE + 1);
        assert!(validate_content(&large_content).is_err());
    }
}
