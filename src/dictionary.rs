//! SKK 辞書の読み込みと検索。
//!
//! SKK 辞書ファイルをパースし、ひらがなの読みから
//! 変換候補（漢字）を検索する。

use std::collections::BTreeMap;
use std::path::Path;

/// 辞書操作で発生するエラー。
#[derive(Debug)]
pub enum DictionaryError {
    /// ファイル I/O エラー。
    Io(std::io::Error),
}

impl std::fmt::Display for DictionaryError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            DictionaryError::Io(e) => write!(f, "辞書ファイルの読み込みエラー: {e}"),
        }
    }
}

impl From<std::io::Error> for DictionaryError {
    fn from(e: std::io::Error) -> Self {
        DictionaryError::Io(e)
    }
}

/// 読みから候補リストへのマッピングを保持する辞書。
pub struct Dictionary {
    entries: BTreeMap<String, Vec<String>>,
}

impl Default for Dictionary {
    fn default() -> Self {
        Self::new()
    }
}

impl Dictionary {
    /// 空の辞書を作成する。
    pub fn new() -> Self {
        Self {
            entries: BTreeMap::new(),
        }
    }

    /// 辞書ファイルから読み込む。UTF-8 / EUC-JP を自動判定する。
    pub fn load_from_file(path: &Path) -> Result<Self, DictionaryError> {
        let bytes = std::fs::read(path)?;

        // UTF-8 として解釈を試み、失敗したら EUC-JP でデコード
        let text = match std::str::from_utf8(&bytes) {
            Ok(s) => s.to_string(),
            Err(_) => {
                let (cow, _, _) = encoding_rs::EUC_JP.decode(&bytes);
                cow.into_owned()
            }
        };

        let mut dict = Self::new();
        for line in text.lines() {
            if let Some((reading, candidates)) = parse_line(line) {
                let entry = dict.entries.entry(reading).or_default();
                for c in candidates {
                    if !entry.contains(&c) {
                        entry.push(c);
                    }
                }
            }
        }

        Ok(dict)
    }

    /// 読みから候補を検索する。
    pub fn lookup(&self, reading: &str) -> Option<&[String]> {
        self.entries.get(reading).map(|v| v.as_slice())
    }

    /// 前方一致検索。指定のプレフィクスで始まる読みとその候補を返す。
    ///
    /// BTreeMap の range を使い、プレフィクスに一致する範囲のみを走査する。
    pub fn lookup_prefix(&self, prefix: &str) -> Vec<(&str, &[String])> {
        // BTreeMap は sorted なので range で効率的に前方一致検索できる。
        // prefix の末尾文字をインクリメントして上限を作る。
        let start = prefix.to_string();
        let end = prefix_end_bound(prefix);
        let iter: Box<dyn Iterator<Item = _>> = match &end {
            Some(end) => Box::new(self.entries.range(start..end.clone())),
            None => Box::new(self.entries.range(start..)),
        };
        iter.filter(|(reading, _)| reading.starts_with(prefix))
            .map(|(reading, candidates)| (reading.as_str(), candidates.as_slice()))
            .collect()
    }
}

/// プレフィクスの「次の文字列」を返す（range 検索の上限用）。
///
/// 末尾の文字をインクリメントして上限を作る。
/// インクリメントできない場合（char::MAX）は None を返す。
fn prefix_end_bound(prefix: &str) -> Option<String> {
    let mut chars: Vec<char> = prefix.chars().collect();
    // 末尾の文字をインクリメント
    while let Some(last) = chars.pop() {
        if let Some(next) = char::from_u32(last as u32 + 1) {
            chars.push(next);
            return Some(chars.into_iter().collect());
        }
        // char::MAX の場合は桁上がり → 1つ前の文字をインクリメント
    }
    None
}

/// SKK 辞書の1行をパースする。
///
/// 読みと候補リストを返す。コメント行・空行は None。
fn parse_line(line: &str) -> Option<(String, Vec<String>)> {
    let line = line.trim();
    if line.is_empty() || line.starts_with(';') {
        return None;
    }

    // 読みと候補部分を最初の空白文字（スペースまたはタブ）で分割
    let split_pos = line.find([' ', '\t'])?;
    let (reading, rest) = (&line[..split_pos], line[split_pos..].trim_start());
    let reading = reading.trim();
    if reading.is_empty() {
        return None;
    }

    // '/' で区切って候補を抽出し、アノテーション（';' 以降）を除去
    let candidates: Vec<String> = rest
        .split('/')
        .filter(|s| !s.is_empty())
        .map(|s| match s.find(';') {
            Some(pos) => s[..pos].to_string(),
            None => s.to_string(),
        })
        .filter(|s| !s.is_empty())
        .collect();

    if candidates.is_empty() {
        return None;
    }

    Some((reading.to_string(), candidates))
}

#[cfg(test)]
mod tests {
    use super::*;

    // === 行パーサー ===

    #[test]
    fn parse_normal_entry() {
        let result = parse_line("かんじ /漢字/感じ/幹事/").unwrap();
        assert_eq!(result.0, "かんじ");
        assert_eq!(result.1, vec!["漢字", "感じ", "幹事"]);
    }

    #[test]
    fn parse_single_candidate() {
        let result = parse_line("にほん /日本/").unwrap();
        assert_eq!(result.0, "にほん");
        assert_eq!(result.1, vec!["日本"]);
    }

    #[test]
    fn parse_annotation() {
        // アノテーション（;以降）は除去して候補のみ返す
        let result = parse_line("にほん /日本;country/二本/").unwrap();
        assert_eq!(result.0, "にほん");
        assert_eq!(result.1, vec!["日本", "二本"]);
    }

    #[test]
    fn parse_comment_line() {
        assert!(parse_line(";; これはコメント").is_none());
    }

    #[test]
    fn parse_empty_line() {
        assert!(parse_line("").is_none());
    }

    #[test]
    fn parse_tab_separated_entry() {
        // タブ区切りの辞書行もパースできること
        let result = parse_line("かんじ\t/漢字/感じ/").unwrap();
        assert_eq!(result.0, "かんじ");
        assert_eq!(result.1, vec!["漢字", "感じ"]);
    }

    #[test]
    fn parse_okurigana_entry() {
        // 送り仮名付きエントリもそのまま保持
        let result = parse_line("おおきi /大き/").unwrap();
        assert_eq!(result.0, "おおきi");
        assert_eq!(result.1, vec!["大き"]);
    }

    // === Dictionary 構造体 ===

    fn sample_dict() -> Dictionary {
        let mut dict = Dictionary::new();
        dict.entries.insert(
            "かんじ".to_string(),
            vec!["漢字".to_string(), "感じ".to_string(), "幹事".to_string()],
        );
        dict.entries
            .insert("にほん".to_string(), vec!["日本".to_string()]);
        dict
    }

    #[test]
    fn lookup_found() {
        let dict = sample_dict();
        let result = dict.lookup("かんじ").unwrap();
        assert_eq!(result, &["漢字", "感じ", "幹事"]);
    }

    #[test]
    fn lookup_not_found() {
        let dict = sample_dict();
        assert!(dict.lookup("そんざいしない").is_none());
    }

    #[test]
    fn lookup_single_candidate() {
        let dict = sample_dict();
        let result = dict.lookup("にほん").unwrap();
        assert_eq!(result, &["日本"]);
    }

    // === ファイル読み込み ===

    #[test]
    fn load_from_utf8_file() {
        let dict = Dictionary::load_from_file(Path::new("tests/fixtures/test_dict.txt")).unwrap();
        let result = dict.lookup("かんじ").unwrap();
        assert_eq!(result, &["漢字", "感じ", "幹事"]);
    }

    #[test]
    fn load_skips_comments_and_empty_lines() {
        let dict = Dictionary::load_from_file(Path::new("tests/fixtures/test_dict.txt")).unwrap();
        // コメント行・空行はエントリとして登録されない
        assert!(dict.lookup(";; テスト用 SKK 辞書").is_none());
        assert!(dict.lookup("").is_none());
    }

    #[test]
    fn load_annotation_stripped() {
        let dict = Dictionary::load_from_file(Path::new("tests/fixtures/test_dict.txt")).unwrap();
        // "にほん /日本;country/二本/" → アノテーション除去
        let result = dict.lookup("にほん").unwrap();
        assert_eq!(result, &["日本", "二本"]);
    }

    #[test]
    fn load_nonexistent_file() {
        let result = Dictionary::load_from_file(Path::new("nonexistent.txt"));
        assert!(result.is_err());
    }

    // === 前方一致検索 ===

    #[test]
    fn lookup_prefix_found() {
        let dict = Dictionary::load_from_file(Path::new("tests/fixtures/test_dict.txt")).unwrap();
        // "かん" → "かん", "かんじ", "かんこく" がヒット
        let results = dict.lookup_prefix("かん");
        let readings: Vec<&str> = results.iter().map(|(r, _)| *r).collect();
        assert!(readings.contains(&"かん"));
        assert!(readings.contains(&"かんじ"));
        assert!(readings.contains(&"かんこく"));
        assert_eq!(results.len(), 3);
    }

    #[test]
    fn lookup_prefix_no_match() {
        let dict = Dictionary::load_from_file(Path::new("tests/fixtures/test_dict.txt")).unwrap();
        let results = dict.lookup_prefix("zzz");
        assert!(results.is_empty());
    }

    #[test]
    fn lookup_prefix_exact_match_included() {
        let dict = Dictionary::load_from_file(Path::new("tests/fixtures/test_dict.txt")).unwrap();
        // "かんじ" 自身も前方一致にヒットする
        let results = dict.lookup_prefix("かんじ");
        let readings: Vec<&str> = results.iter().map(|(r, _)| *r).collect();
        assert!(readings.contains(&"かんじ"));
    }

    // === 候補の重複排除 ===

    #[test]
    fn dedup_candidates() {
        // 同じ読みが複数行にあっても候補が重複しないこと
        let content = "かんじ /漢字/感じ/\nかんじ /漢字/幹事/\n";
        let dir = std::env::temp_dir().join("enpitsu_test_dedup");
        std::fs::create_dir_all(&dir).unwrap();
        let path = dir.join("test_dedup.dict");
        std::fs::write(&path, content).unwrap();

        let dict = Dictionary::load_from_file(&path).unwrap();
        let result = dict.lookup("かんじ").unwrap();
        // "漢字" は1回だけ登場する
        assert_eq!(result, &["漢字", "感じ", "幹事"]);

        let _ = std::fs::remove_file(&path);
    }

    // === EUC-JP 対応 ===

    #[test]
    fn load_from_eucjp_file() {
        // EUC-JP エンコードのテスト辞書を一時ファイルに生成
        let content = "かんじ /漢字/感じ/\nにほん /日本/\n";
        let (encoded, _, _) = encoding_rs::EUC_JP.encode(content);

        let dir = std::env::temp_dir().join("enpitsu_test");
        std::fs::create_dir_all(&dir).unwrap();
        let path = dir.join("test_eucjp.dict");
        std::fs::write(&path, &*encoded).unwrap();

        let dict = Dictionary::load_from_file(&path).unwrap();
        let result = dict.lookup("かんじ").unwrap();
        assert_eq!(result, &["漢字", "感じ"]);
        let result = dict.lookup("にほん").unwrap();
        assert_eq!(result, &["日本"]);

        // クリーンアップ
        let _ = std::fs::remove_file(&path);
    }
}
