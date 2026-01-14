use crate::domain::Note;
use std::path::Path;

/// ファイル名生成戦略（Strategyパターン）
pub trait FilenameStrategy: Send + Sync {
    /// メモからファイル名を生成（拡張子なし）
    fn generate(&self, note: &Note, existing_files: &[&Path]) -> String;
}
