//! ユーザー辞書の管理。
//!
//! ユーザーが変換で確定した結果を学習し、次回以降の変換で
//! 候補の優先順位を変更する。SKK 形式で保存・読み込みする。

use std::collections::HashMap;
use std::path::Path;

use crate::dictionary::DictionaryError;

/// ユーザー辞書。確定結果を学習し、候補の優先順位を変更する。
///
/// SKK 形式で保存・読み込みする。
/// エントリは HashMap<読み, Vec<候補>> で管理し、
/// Vec の先頭が最も優先度の高い候補。
pub struct UserDictionary {
    entries: HashMap<String, Vec<String>>,
    dirty: bool,
}

impl UserDictionary {
    /// 空のユーザー辞書を作成する。
    pub fn new() -> Self {
        Self {
            entries: HashMap::new(),
            dirty: false,
        }
    }

    /// ファイルからユーザー辞書を読み込む。ファイルが存在しない場合は空の辞書を返す。
    pub fn load(path: &Path) -> Result<Self, DictionaryError> {
        if !path.exists() {
            return Ok(Self::new());
        }
        let text = std::fs::read_to_string(path)?;
        let mut entries = HashMap::new();
        for line in text.lines() {
            let line = line.trim();
            if line.is_empty() || line.starts_with(';') {
                continue;
            }
            let Some(split_pos) = line.find([' ', '\t']) else {
                continue;
            };
            let reading = &line[..split_pos];
            let rest = line[split_pos..].trim_start();
            let candidates: Vec<String> = rest
                .split('/')
                .filter(|s| !s.is_empty())
                .map(|s| s.to_string())
                .collect();
            if !candidates.is_empty() {
                entries.insert(reading.to_string(), candidates);
            }
        }
        Ok(Self {
            entries,
            dirty: false,
        })
    }

    /// ユーザー辞書をファイルに保存する。
    pub fn save(&mut self, path: &Path) -> Result<(), DictionaryError> {
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        let mut lines: Vec<String> = Vec::new();
        lines.push(";; Enpitsu ユーザー辞書".to_string());
        let mut sorted_keys: Vec<&String> = self.entries.keys().collect();
        sorted_keys.sort();
        for reading in sorted_keys {
            let candidates = &self.entries[reading];
            let cands = candidates
                .iter()
                .map(|s| s.as_str())
                .collect::<Vec<_>>()
                .join("/");
            lines.push(format!("{reading} /{cands}/"));
        }
        std::fs::write(path, lines.join("\n") + "\n")?;
        self.dirty = false;
        Ok(())
    }

    /// 学習: 読みと候補を記録する。既存エントリの場合は先頭に移動（優先度上げ）。
    pub fn record(&mut self, reading: &str, candidate: &str) {
        let entry = self.entries.entry(reading.to_string()).or_default();
        entry.retain(|c| c != candidate);
        entry.insert(0, candidate.to_string());
        self.dirty = true;
    }

    /// 読みから候補を検索する。
    pub fn lookup(&self, reading: &str) -> Option<&[String]> {
        self.entries
            .get(reading)
            .filter(|v| !v.is_empty())
            .map(|v| v.as_slice())
    }

    /// 保存が必要かどうか。
    pub fn is_dirty(&self) -> bool {
        self.dirty
    }
}

impl Default for UserDictionary {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // === 基本操作 ===

    #[test]
    fn new_is_empty() {
        let ud = UserDictionary::new();
        assert!(ud.lookup("かんじ").is_none());
        assert!(!ud.is_dirty());
    }

    #[test]
    fn record_and_lookup() {
        let mut ud = UserDictionary::new();
        ud.record("かんじ", "漢字");
        let result = ud.lookup("かんじ").unwrap();
        assert_eq!(result, &["漢字"]);
        assert!(ud.is_dirty());
    }

    #[test]
    fn record_multiple_candidates() {
        let mut ud = UserDictionary::new();
        ud.record("かんじ", "漢字");
        ud.record("かんじ", "感じ");
        let result = ud.lookup("かんじ").unwrap();
        assert_eq!(result, &["感じ", "漢字"]);
    }

    // === 学習（優先度変更） ===

    #[test]
    fn record_existing_moves_to_front() {
        let mut ud = UserDictionary::new();
        ud.record("かんじ", "漢字");
        ud.record("かんじ", "感じ");
        ud.record("かんじ", "幹事");
        // この時点: ["幹事", "感じ", "漢字"]
        // "漢字" を再度選択 → 先頭に移動
        ud.record("かんじ", "漢字");
        let result = ud.lookup("かんじ").unwrap();
        assert_eq!(result[0], "漢字");
    }

    #[test]
    fn record_same_candidate_no_duplicate() {
        let mut ud = UserDictionary::new();
        ud.record("かんじ", "漢字");
        ud.record("かんじ", "漢字");
        let result = ud.lookup("かんじ").unwrap();
        assert_eq!(result.len(), 1);
        assert_eq!(result, &["漢字"]);
    }

    // === 保存・読み込み ===

    #[test]
    fn save_and_load() {
        let dir = std::env::temp_dir().join("enpitsu_test_ud");
        std::fs::create_dir_all(&dir).unwrap();
        let path = dir.join("user_dict_test.txt");

        let mut ud = UserDictionary::new();
        ud.record("かんじ", "漢字");
        ud.record("かんじ", "感じ");
        ud.record("にほん", "日本");
        ud.save(&path).unwrap();

        let loaded = UserDictionary::load(&path).unwrap();
        let result = loaded.lookup("かんじ").unwrap();
        assert_eq!(result, &["感じ", "漢字"]);
        let result = loaded.lookup("にほん").unwrap();
        assert_eq!(result, &["日本"]);

        let _ = std::fs::remove_file(&path);
    }

    #[test]
    fn load_nonexistent_returns_empty() {
        let loaded = UserDictionary::load(Path::new("/tmp/nonexistent_ud.txt")).unwrap();
        assert!(loaded.lookup("かんじ").is_none());
    }

    #[test]
    fn save_clears_dirty_flag() {
        let dir = std::env::temp_dir().join("enpitsu_test_ud");
        std::fs::create_dir_all(&dir).unwrap();
        let path = dir.join("user_dict_dirty_test.txt");

        let mut ud = UserDictionary::new();
        ud.record("かんじ", "漢字");
        assert!(ud.is_dirty());
        ud.save(&path).unwrap();
        assert!(!ud.is_dirty());

        let _ = std::fs::remove_file(&path);
    }

    #[test]
    fn lookup_not_found() {
        let ud = UserDictionary::new();
        assert!(ud.lookup("そんざいしない").is_none());
    }
}
