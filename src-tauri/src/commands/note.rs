// ノート関連コマンド
//
// SOLID: Input Validation
// フロントエンドからの入力を信頼せず、バックエンドで検証する

use super::{NoteDto, NoteListItemDto, SearchResultDto};
use crate::AppState;
use tauri::State;

/// クエリ長制限（DoS防止）
const MAX_QUERY_LENGTH: usize = 200;

// ===== 入力検証 =====

/// UID検証（タイムスタンプ形式のみ）
///
/// アプリで生成されるUIDはタイムスタンプ形式（数字のみ、14-20文字）
/// 例: "2026011418102637208"
///
/// # セキュリティ
///
/// - 数字のみ許可（パストラバーサル防止）
/// - 長さ制限: 14-26文字（タイムスタンプ + ナノ秒）
fn validate_uid(uid: &str) -> Result<(), String> {
    // 長さチェック（タイムスタンプは14-26文字）
    if uid.len() < 14 || uid.len() > 26 {
        return Err(format!(
            "Invalid UID length: {} characters (expected 14-26)",
            uid.len()
        ));
    }

    // 数字のみ許可
    for c in uid.chars() {
        if !c.is_ascii_digit() {
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

    note.update_content(content.clone());
    state
        .note_service
        .save_note(&note)
        .map_err(|e| e.to_string())?;

    // バックリンクインデックスを更新
    let title = note.metadata.title.clone().unwrap_or_default();
    state.backlink_service.update_note(&note.metadata.uid, &title, &content);

    Ok(())
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
        .map_err(|e| e.to_string())?;

    // バックリンクインデックスから削除
    state.backlink_service.remove_note(&uid);

    Ok(())
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

/// ノートを検索
///
/// # Performance
/// - nucleo fuzzy matching (skim比6倍高速)
/// - rayon並列ファイル読み込み
/// - memmap2高速I/O
#[tauri::command]
pub fn search_notes(
    state: State<AppState>,
    query: String,
    limit: Option<usize>,
) -> Result<Vec<SearchResultDto>, String> {
    // クエリ長制限（DoS防止）
    if query.len() > MAX_QUERY_LENGTH {
        return Err(format!(
            "Query too long: {} chars (max {} chars)",
            query.len(),
            MAX_QUERY_LENGTH
        ));
    }

    state
        .search_service
        .search(&query, limit)
        .map(|results| results.into_iter().map(SearchResultDto::from).collect())
        .map_err(|e| e.to_string())
}

/// Wiki linkを解決（タイトルからノートを検索、なければ作成）
#[tauri::command]
pub fn resolve_wiki_link(
    state: State<AppState>,
    title: String,
) -> Result<NoteDto, String> {
    // タイトル長制限
    if title.len() > 200 {
        return Err("Title too long".to_string());
    }

    // タイトルでノートを検索
    if let Some(note_item) = state
        .search_service
        .find_by_title(&title)
        .map_err(|e| e.to_string())?
    {
        // 既存ノートをロード
        return state
            .note_service
            .load_note(&note_item.uid)
            .map(NoteDto::from)
            .map_err(|e| e.to_string());
    }

    // ノートが見つからない場合は新規作成
    let note = crate::domain::Note::with_title(&title);
    state
        .note_service
        .save_note(&note)
        .map_err(|e| e.to_string())?;

    // バックリンクインデックスに追加
    state.backlink_service.update_note(&note.metadata.uid, &title, &note.content);

    Ok(NoteDto::from(note))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_validate_uid_valid() {
        // タイムスタンプ形式（アプリ内生成）
        assert!(validate_uid("2026011418102637208").is_ok());
        assert!(validate_uid("20260114181236685512").is_ok());
        assert!(validate_uid("20260114181026").is_ok());
    }

    #[test]
    fn test_validate_uid_too_short() {
        // 14文字未満
        assert!(validate_uid("1234567890123").is_err());
        assert!(validate_uid("").is_err());
    }

    #[test]
    fn test_validate_uid_too_long() {
        // 26文字を超える場合はエラー
        let long_uid = "1".repeat(27);
        assert!(validate_uid(&long_uid).is_err());
        // 26文字以下はOK
        let max_uid = "1".repeat(26);
        assert!(validate_uid(&max_uid).is_ok());
    }

    #[test]
    fn test_validate_uid_invalid_chars() {
        // 英字（数字のみ許可）
        assert!(validate_uid("2026011418102a").is_err());
        // 特殊文字
        assert!(validate_uid("20260114181026/").is_err());
        assert!(validate_uid("20260114181026.").is_err());
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
