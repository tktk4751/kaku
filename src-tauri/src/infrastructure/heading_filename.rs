use crate::domain::Note;
use crate::traits::FilenameStrategy;
use std::path::Path;

/// H1/H2見出しベースのファイル名生成戦略
pub struct HeadingFilenameStrategy;

impl HeadingFilenameStrategy {
    pub fn new() -> Self {
        Self
    }

    /// ファイル名をサニタイズ（禁止文字を置換）
    fn sanitize(name: &str) -> String {
        name.chars()
            .map(|c| match c {
                '/' | '\\' | ':' | '*' | '?' | '"' | '<' | '>' | '|' => '_',
                _ => c,
            })
            .collect()
    }

    /// 文字数ベースで切り詰め（UTF-8安全）
    fn truncate(name: &str, max_chars: usize) -> String {
        let char_count = name.chars().count();
        if char_count > max_chars {
            let truncated: String = name.chars().take(max_chars - 3).collect();
            format!("{}...", truncated)
        } else {
            name.to_string()
        }
    }

    /// 重複を避けるための連番付きファイル名を生成
    fn make_unique(base_name: &str, existing_files: &[&Path]) -> String {
        let existing_names: Vec<String> = existing_files
            .iter()
            .filter_map(|p| p.file_stem())
            .filter_map(|s| s.to_str())
            .map(|s| s.to_string())
            .collect();

        if !existing_names.contains(&base_name.to_string()) {
            return base_name.to_string();
        }

        // 連番を付与
        for i in 2..=999 {
            let candidate = format!("{}_{}", base_name, i);
            if !existing_names.contains(&candidate) {
                return candidate;
            }
        }

        // フォールバック（ほぼありえない）
        format!("{}_{}", base_name, chrono::Utc::now().format("%Y%m%d%H%M%S"))
    }
}

impl Default for HeadingFilenameStrategy {
    fn default() -> Self {
        Self::new()
    }
}

impl FilenameStrategy for HeadingFilenameStrategy {
    fn generate(&self, note: &Note, existing_files: &[&Path]) -> String {
        let base_name = match note.extract_heading() {
            Some(heading) => {
                let sanitized = Self::sanitize(&heading);
                Self::truncate(&sanitized, 200)
            }
            None => note.metadata.uid.clone(),
        };

        Self::make_unique(&base_name, existing_files)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::Note;
    use std::path::PathBuf;

    #[test]
    fn test_generate_from_h1() {
        let strategy = HeadingFilenameStrategy::new();
        let mut note = Note::new();
        note.content = "# 買い物リスト\n\nアイテム".to_string();

        let filename = strategy.generate(&note, &[]);
        assert_eq!(filename, "買い物リスト");
    }

    #[test]
    fn test_generate_fallback_to_uid() {
        let strategy = HeadingFilenameStrategy::new();
        let note = Note::new();

        let filename = strategy.generate(&note, &[]);
        assert_eq!(filename, note.metadata.uid);
    }

    #[test]
    fn test_sanitize_forbidden_chars() {
        let sanitized = HeadingFilenameStrategy::sanitize("test/file:name*?.md");
        assert_eq!(sanitized, "test_file_name__.md");
    }

    #[test]
    fn test_truncate_long_name() {
        let long_name = "あ".repeat(250);
        let truncated = HeadingFilenameStrategy::truncate(&long_name, 200);
        assert_eq!(truncated.chars().count(), 200);
        assert!(truncated.ends_with("..."));
    }

    #[test]
    fn test_make_unique() {
        let existing = vec![
            PathBuf::from("/path/to/テスト.md"),
            PathBuf::from("/path/to/テスト_2.md"),
        ];
        let refs: Vec<&Path> = existing.iter().map(|p| p.as_path()).collect();

        let unique = HeadingFilenameStrategy::make_unique("テスト", &refs);
        assert_eq!(unique, "テスト_3");
    }
}
