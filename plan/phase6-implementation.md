# Phase 6 実装計画: 仕上げ・インストーラー

## 概要

Phase 4（TSF 連携、143テスト全パス）の上に、一般ユーザーが簡単にインストール・使用できる
仕上げを行う。ユーザー辞書の永続化、設定ファイルの読み込み、インストーラースクリプトの改善、
品質向上（エラーハンドリング・ログ出力）、ドキュメント整備を実施する。

TDD サイクル（Red → Green → Refactor）を厳守する。
プラットフォーム非依存のロジック（ユーザー辞書、設定パーサー）はユニットテストで検証し、
Windows 固有のコード（パス解決、インストーラー）は手動テストで検証する。

> **Note:** Phase 5（候補ウィンドウ UI）は Phase 6 の前提条件ではない。
> Phase 5 は Windows 専用 UI 機能であり、Phase 6 で扱うロジック（ユーザー辞書・設定・品質向上）
> はコアエンジン層に対する変更であるため、独立して実装できる。

---

## アーキテクチャ設計

### レイヤー構成

```
┌─────────────────────────────────────────────────┐
│  Windows アプリケーション (メモ帳等)                │
└───────────────┬─────────────────────────────────┘
                │ TSF API
┌───────────────▼─────────────────────────────────┐
│  TextService (text_service.rs) — 既存             │
├─────────────────────────────────────────────────┤
│  ConversionEngine (engine.rs) ← EDIT             │
│  ├── ユーザー辞書連携                               │
│  └── 確定時に学習データを UserDictionary へ記録      │
├─────────────────────────────────────────────────┤
│  UserDictionary (user_dictionary.rs) ← NEW       │
│  ├── エントリの保存・読み込み (%APPDATA%)            │
│  ├── 学習: 選択候補の優先度上げ                      │
│  └── 辞書ファイルの永続化 (SKK 形式)                │
├─────────────────────────────────────────────────┤
│  Config (config.rs) ← NEW                        │
│  ├── config.toml のパース                          │
│  ├── キーバインドカスタマイズ                         │
│  └── デフォルト設定の生成                            │
├─────────────────────────────────────────────────┤
│  Dictionary (dictionary.rs) ← 既存                │
│  InputState / CandidateList — 既存                │
└─────────────────────────────────────────────────┘
```

### プラットフォーム分離方針

| コード | プラットフォーム | テスト方法 |
|--------|----------------|-----------|
| `user_dictionary.rs` (NEW) | 非依存 | `cargo test` (TDD) |
| `config.rs` (NEW) | 非依存 | `cargo test` (TDD) |
| `engine.rs` (ユーザー辞書連携) | 非依存 | `cargo test` (TDD) |
| `dictionary.rs` (書き出しメソッド追加) | 非依存 | `cargo test` (TDD) |
| `installer/install.ps1` (改善) | Windows 専用 | Windows での手動テスト |
| ドキュメント (README.md 等) | 非依存 | 目視確認 |

### ファイル構成

```
src/
├── user_dictionary.rs  # NEW: ユーザー辞書の管理（学習・永続化）
├── config.rs           # NEW: 設定ファイルのパース・デフォルト値
├── engine.rs           # EDIT: ユーザー辞書連携
├── dictionary.rs       # EDIT: save_to_file() メソッド追加
├── lib.rs              # EDIT: pub mod user_dictionary; pub mod config; 追加
├── main.rs             # EDIT: --user-dict オプション追加
├── candidate.rs        # 既存（変更なし）
├── input_state.rs      # 既存（変更なし）
├── romaji.rs           # 既存（変更なし）
├── katakana.rs         # 既存（変更なし）
├── key_mapping.rs      # 既存（変更なし）
├── guids.rs            # 既存（変更なし）
├── text_service.rs     # EDIT: ユーザー辞書・設定の読み込み
├── class_factory.rs    # 既存（変更なし）
├── registry.rs         # 既存（変更なし）
installer/
├── install.ps1         # EDIT: 辞書ファイル同梱・設定ディレクトリ作成
README.md               # NEW: プロジェクト概要・インストール手順・使い方
```

---

## ステップ 1: UserDictionary — エントリの保存・読み込み (TDD)

ユーザーが変換で確定した結果を記録し、次回以降の変換で優先表示するための
ユーザー辞書を実装する。システム辞書と同じ SKK 形式で保存する。

### 1-1. 設計方針

```rust
/// ユーザー辞書。確定結果を学習し、候補の優先順位を変更する。
///
/// SKK 形式で保存・読み込みする。
/// エントリは HashMap<読み, Vec<候補>> で管理し、
/// Vec の先頭が最も優先度の高い候補。
pub struct UserDictionary {
    entries: HashMap<String, Vec<String>>,
    dirty: bool,  // 保存が必要か
}
```

公開 API:

```rust
impl UserDictionary {
    /// 空のユーザー辞書を作成する。
    pub fn new() -> Self;

    /// ファイルからユーザー辞書を読み込む。ファイルが存在しない場合は空の辞書を返す。
    pub fn load(path: &Path) -> Result<Self, DictionaryError>;

    /// ユーザー辞書をファイルに保存する。
    pub fn save(&self, path: &Path) -> Result<(), DictionaryError>;

    /// 学習: 読みと候補を記録する。既存エントリの場合は先頭に移動（優先度上げ）。
    pub fn record(&mut self, reading: &str, candidate: &str);

    /// 読みから候補を検索する。
    pub fn lookup(&self, reading: &str) -> Option<&[String]>;

    /// 保存が必要かどうか。
    pub fn is_dirty(&self) -> bool;
}
```

### 1-2. Red: UserDictionary の基本テストを書く

`src/user_dictionary.rs` を新規作成し、テストを先に書く:

```rust
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

        // クリーンアップ
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

        // クリーンアップ
        let _ = std::fs::remove_file(&path);
    }

    #[test]
    fn lookup_not_found() {
        let ud = UserDictionary::new();
        assert!(ud.lookup("そんざいしない").is_none());
    }
}
```

**動作確認:** `cargo test` で新しいテストが**失敗する**こと（Red）

### 1-3. Green: UserDictionary を実装

```rust
//! ユーザー辞書の管理。
//!
//! ユーザーが変換で確定した結果を学習し、次回以降の変換で
//! 候補の優先順位を変更する。SKK 形式で保存・読み込みする。

use std::collections::HashMap;
use std::path::Path;

use crate::dictionary::DictionaryError;

/// ユーザー辞書。
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
            let split_pos = match line.find([' ', '\t']) {
                Some(pos) => pos,
                None => continue,
            };
            let (reading, rest) = (&line[..split_pos], line[split_pos..].trim_start());
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
        // 親ディレクトリがなければ作成
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
        // 既存の同じ候補を削除
        entry.retain(|c| c != candidate);
        // 先頭に挿入
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
```

**動作確認:** `cargo test` で全テスト（既存 143 + 新規 9 = 152 テスト）がパスすること（Green）

### 1-4. Refactor

**動作確認:** `cargo test` 全パス、`cargo clippy` 警告なし、`cargo fmt -- --check` 差分なし

---

## ステップ 2: エンジンにユーザー辞書連携を追加 (TDD)

ConversionEngine に UserDictionary を組み込み、変換時にシステム辞書とユーザー辞書を
マージした候補を返し、確定時にユーザー辞書へ学習データを記録する。

### 2-1. 設計方針

候補マージのルール:
1. ユーザー辞書の候補を先頭に配置
2. システム辞書の候補のうち、ユーザー辞書に含まれないものを後ろに追加
3. 重複を排除

```
ユーザー辞書: ["感じ"]
システム辞書: ["漢字", "感じ", "幹事"]
マージ結果:   ["感じ", "漢字", "幹事"]
```

ConversionEngine の変更:

```rust
pub struct ConversionEngine {
    state: EngineState,
    input: InputState,
    dict: Option<Dictionary>,
    user_dict: Option<UserDictionary>,     // NEW
    candidates: Option<CandidateList>,
    last_reading: Option<String>,           // NEW: 確定時の学習に使用
}
```

### 2-2. Red: ユーザー辞書連携テストを書く

`src/engine.rs` のテストに以下を追加:

```rust
// === ユーザー辞書連携 ===

fn test_engine_with_user_dict() -> ConversionEngine {
    let dict = Dictionary::load_from_file(Path::new("tests/fixtures/test_dict.txt")).unwrap();
    let mut user_dict = UserDictionary::new();
    user_dict.record("かんじ", "感じ");  // "感じ" をユーザー辞書に登録
    ConversionEngine::new_with_user_dict(Some(dict), Some(user_dict))
}

#[test]
fn user_dict_candidates_first() {
    let mut engine = test_engine_with_user_dict();
    for ch in "kanji".chars() {
        engine.process(EngineCommand::InsertChar(ch));
    }
    let output = engine.process(EngineCommand::Convert);
    // ユーザー辞書の "感じ" が先頭、続いてシステム辞書の残り
    assert_eq!(output.display, "感じ");
    let candidates = output.candidates.unwrap();
    assert_eq!(candidates[0], "感じ");
    assert!(candidates.contains(&"漢字".to_string()));
    assert!(candidates.contains(&"幹事".to_string()));
}

#[test]
fn user_dict_no_duplicate() {
    let mut engine = test_engine_with_user_dict();
    for ch in "kanji".chars() {
        engine.process(EngineCommand::InsertChar(ch));
    }
    let output = engine.process(EngineCommand::Convert);
    let candidates = output.candidates.unwrap();
    // "感じ" が重複していないこと
    let count = candidates.iter().filter(|c| c.as_str() == "感じ").count();
    assert_eq!(count, 1);
}

#[test]
fn commit_records_to_user_dict() {
    let dict = Dictionary::load_from_file(Path::new("tests/fixtures/test_dict.txt")).unwrap();
    let user_dict = UserDictionary::new();
    let mut engine = ConversionEngine::new_with_user_dict(Some(dict), Some(user_dict));

    for ch in "kanji".chars() {
        engine.process(EngineCommand::InsertChar(ch));
    }
    engine.process(EngineCommand::Convert);
    engine.process(EngineCommand::NextCandidate); // → "感じ"
    engine.process(EngineCommand::Commit);

    // 2回目: ユーザー辞書の学習で "感じ" が先頭になる
    for ch in "kanji".chars() {
        engine.process(EngineCommand::InsertChar(ch));
    }
    let output = engine.process(EngineCommand::Convert);
    assert_eq!(output.display, "感じ");
}

#[test]
fn engine_without_user_dict_unchanged() {
    // ユーザー辞書なしの場合は既存の動作と同じ
    let mut engine = test_engine();
    for ch in "kanji".chars() {
        engine.process(EngineCommand::InsertChar(ch));
    }
    let output = engine.process(EngineCommand::Convert);
    assert_eq!(output.display, "漢字");
}

#[test]
fn user_dict_only_no_system_dict() {
    let mut user_dict = UserDictionary::new();
    user_dict.record("かんじ", "感じ");
    let mut engine = ConversionEngine::new_with_user_dict(None, Some(user_dict));

    for ch in "kanji".chars() {
        engine.process(EngineCommand::InsertChar(ch));
    }
    let output = engine.process(EngineCommand::Convert);
    assert_eq!(output.display, "感じ");
    let candidates = output.candidates.unwrap();
    assert_eq!(candidates, vec!["感じ"]);
}
```

**動作確認:** `cargo test` で新しいテストが**失敗する**こと（Red）

### 2-3. Green: ユーザー辞書連携を実装

`engine.rs` に以下の変更を加える:

```rust
use crate::user_dictionary::UserDictionary;

pub struct ConversionEngine {
    state: EngineState,
    input: InputState,
    dict: Option<Dictionary>,
    user_dict: Option<UserDictionary>,
    candidates: Option<CandidateList>,
    last_reading: Option<String>,
}

impl ConversionEngine {
    /// 新しい変換エンジンを作成する（後方互換）。
    pub fn new(dict: Option<Dictionary>) -> Self {
        Self {
            state: EngineState::Direct,
            input: InputState::new(),
            dict,
            user_dict: None,
            candidates: None,
            last_reading: None,
        }
    }

    /// ユーザー辞書付きの変換エンジンを作成する。
    pub fn new_with_user_dict(
        dict: Option<Dictionary>,
        user_dict: Option<UserDictionary>,
    ) -> Self {
        Self {
            state: EngineState::Direct,
            input: InputState::new(),
            dict,
            user_dict,
            candidates: None,
            last_reading: None,
        }
    }

    /// ユーザー辞書の可変参照を返す。
    pub fn user_dict_mut(&mut self) -> Option<&mut UserDictionary> {
        self.user_dict.as_mut()
    }
}
```

`do_convert()` を変更してユーザー辞書を優先:

```rust
fn do_convert(&mut self) -> EngineOutput {
    self.input.flush();
    let hiragana = self.input.output().to_string();

    // ユーザー辞書とシステム辞書の候補をマージ
    let user_cands: Vec<String> = self
        .user_dict
        .as_ref()
        .and_then(|ud| ud.lookup(&hiragana))
        .map(|c| c.to_vec())
        .unwrap_or_default();

    let system_cands: Vec<String> = self
        .dict
        .as_ref()
        .and_then(|d| d.lookup(&hiragana))
        .map(|c| c.iter().map(|s| s.to_string()).collect())
        .unwrap_or_default();

    // ユーザー辞書 → システム辞書（重複排除）
    let mut merged = user_cands;
    for c in system_cands {
        if !merged.contains(&c) {
            merged.push(c);
        }
    }

    if merged.is_empty() {
        // 候補なし → ひらがなを確定
        self.last_reading = None;
        let committed = hiragana;
        self.input.reset();
        self.state = EngineState::Direct;
        EngineOutput {
            committed,
            display: String::new(),
            candidates: None,
            candidate_index: None,
        }
    } else {
        self.last_reading = Some(hiragana);
        let cl = CandidateList::new(merged);
        // ... 既存の Converting 遷移処理 ...
    }
}
```

Commit 時の学習処理:

```rust
(EngineState::Converting, EngineCommand::Commit) => {
    let committed = self
        .candidates
        .as_ref()
        .and_then(|cl| cl.select())
        .unwrap_or_default();

    // ユーザー辞書に学習データを記録
    if let (Some(ref mut ud), Some(ref reading)) =
        (&mut self.user_dict, &self.last_reading)
    {
        if !committed.is_empty() {
            ud.record(reading, &committed);
        }
    }

    self.candidates = None;
    self.last_reading = None;
    self.input.reset();
    self.state = EngineState::Direct;
    EngineOutput {
        committed,
        display: String::new(),
        candidates: None,
        candidate_index: None,
    }
}
```

**動作確認:** `cargo test` で全テスト（152 + 新規 5 = 157 テスト）がパスすること（Green）

### 2-4. Refactor

- `do_convert()` の候補マージロジックをヘルパーメソッド `merge_candidates()` に抽出
- `last_reading` のクリアを Cancel 時にも追加

**動作確認:** `cargo test` 全パス、`cargo clippy` 警告なし、`cargo fmt -- --check` 差分なし

---

## ステップ 3: Config — 設定ファイルのパースとデフォルト値 (TDD)

`%APPDATA%\enpitsu\config.toml` から設定を読み込む。
TOML パーサーは外部クレートを使わず、簡易実装（key = value 形式）とする。

### 3-1. 設計方針

設定項目:

```toml
# Enpitsu 設定ファイル

[general]
# 入力モード切り替えキー: "zenkaku-hankaku" | "ctrl-space" | "alt-tilde"
toggle_key = "zenkaku-hankaku"

[dictionary]
# システム辞書パス（空の場合は DLL と同じディレクトリの dict/ を使用）
system_dict_path = ""

[behavior]
# 候補選択後に自動的に学習するか
auto_learn = true
```

公開 API:

```rust
/// アプリケーション設定。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Config {
    pub toggle_key: ToggleKey,
    pub system_dict_path: Option<String>,
    pub auto_learn: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ToggleKey {
    ZenkakuHankaku,
    CtrlSpace,
    AltTilde,
}

impl Config {
    /// デフォルト設定を返す。
    pub fn default_config() -> Self;

    /// TOML ファイルから設定を読み込む。
    pub fn load(path: &Path) -> Result<Self, ConfigError>;

    /// デフォルト設定ファイルの内容を生成する。
    pub fn default_toml() -> String;
}
```

### 3-2. Red: Config のテストを書く

`src/config.rs` を新規作成し、テストを先に書く:

```rust
#[cfg(test)]
mod tests {
    use super::*;

    // === デフォルト設定 ===

    #[test]
    fn default_config_values() {
        let config = Config::default_config();
        assert_eq!(config.toggle_key, ToggleKey::ZenkakuHankaku);
        assert_eq!(config.system_dict_path, None);
        assert!(config.auto_learn);
    }

    // === TOML パース ===

    #[test]
    fn parse_complete_config() {
        let toml = r#"
[general]
toggle_key = "ctrl-space"

[dictionary]
system_dict_path = "C:\dict\SKK-JISYO.L"

[behavior]
auto_learn = false
"#;
        let config = Config::parse(toml).unwrap();
        assert_eq!(config.toggle_key, ToggleKey::CtrlSpace);
        assert_eq!(
            config.system_dict_path,
            Some(r"C:\dict\SKK-JISYO.L".to_string())
        );
        assert!(!config.auto_learn);
    }

    #[test]
    fn parse_partial_config_uses_defaults() {
        let toml = r#"
[general]
toggle_key = "alt-tilde"
"#;
        let config = Config::parse(toml).unwrap();
        assert_eq!(config.toggle_key, ToggleKey::AltTilde);
        // 未指定の項目はデフォルト値
        assert_eq!(config.system_dict_path, None);
        assert!(config.auto_learn);
    }

    #[test]
    fn parse_empty_config_returns_defaults() {
        let config = Config::parse("").unwrap();
        assert_eq!(config, Config::default_config());
    }

    #[test]
    fn parse_comments_and_blank_lines() {
        let toml = r#"
# これはコメント
[general]
# コメント行
toggle_key = "zenkaku-hankaku"
"#;
        let config = Config::parse(toml).unwrap();
        assert_eq!(config.toggle_key, ToggleKey::ZenkakuHankaku);
    }

    #[test]
    fn parse_invalid_toggle_key() {
        let toml = r#"
[general]
toggle_key = "invalid-key"
"#;
        let result = Config::parse(toml);
        assert!(result.is_err());
    }

    // === ファイル読み込み ===

    #[test]
    fn load_nonexistent_returns_defaults() {
        let config = Config::load(Path::new("/tmp/nonexistent_config.toml")).unwrap();
        assert_eq!(config, Config::default_config());
    }

    #[test]
    fn load_and_save_roundtrip() {
        let dir = std::env::temp_dir().join("enpitsu_test_config");
        std::fs::create_dir_all(&dir).unwrap();
        let path = dir.join("config_test.toml");

        let toml = Config::default_toml();
        std::fs::write(&path, &toml).unwrap();
        let config = Config::load(&path).unwrap();
        assert_eq!(config, Config::default_config());

        // クリーンアップ
        let _ = std::fs::remove_file(&path);
    }

    // === default_toml ===

    #[test]
    fn default_toml_is_parsable() {
        let toml = Config::default_toml();
        let config = Config::parse(&toml).unwrap();
        assert_eq!(config, Config::default_config());
    }
}
```

**動作確認:** `cargo test` で新しいテストが**失敗する**こと（Red）

### 3-3. Green: Config を実装

```rust
//! 設定ファイルのパースとデフォルト値。
//!
//! `%APPDATA%\enpitsu\config.toml` から設定を読み込む。
//! 簡易 TOML パーサー（key = value 形式のサブセット）。

use std::path::Path;

/// 設定エラー。
#[derive(Debug)]
pub enum ConfigError {
    Io(std::io::Error),
    Parse(String),
}

impl From<std::io::Error> for ConfigError {
    fn from(e: std::io::Error) -> Self {
        ConfigError::Io(e)
    }
}

impl std::fmt::Display for ConfigError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ConfigError::Io(e) => write!(f, "設定ファイルの読み込みエラー: {e}"),
            ConfigError::Parse(msg) => write!(f, "設定ファイルのパースエラー: {msg}"),
        }
    }
}

/// 入力モード切り替えキー。
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ToggleKey {
    ZenkakuHankaku,
    CtrlSpace,
    AltTilde,
}

/// アプリケーション設定。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Config {
    pub toggle_key: ToggleKey,
    pub system_dict_path: Option<String>,
    pub auto_learn: bool,
}

impl Config {
    pub fn default_config() -> Self {
        Self {
            toggle_key: ToggleKey::ZenkakuHankaku,
            system_dict_path: None,
            auto_learn: true,
        }
    }

    pub fn load(path: &Path) -> Result<Self, ConfigError> {
        if !path.exists() {
            return Ok(Self::default_config());
        }
        let text = std::fs::read_to_string(path)?;
        Self::parse(&text)
    }

    pub fn parse(text: &str) -> Result<Self, ConfigError> {
        let mut config = Self::default_config();
        for line in text.lines() {
            let line = line.trim();
            if line.is_empty() || line.starts_with('#') || line.starts_with('[') {
                continue;
            }
            if let Some((key, value)) = line.split_once('=') {
                let key = key.trim();
                let value = value.trim().trim_matches('"');
                match key {
                    "toggle_key" => {
                        config.toggle_key = match value {
                            "zenkaku-hankaku" => ToggleKey::ZenkakuHankaku,
                            "ctrl-space" => ToggleKey::CtrlSpace,
                            "alt-tilde" => ToggleKey::AltTilde,
                            _ => return Err(ConfigError::Parse(
                                format!("不明な toggle_key: {value}")
                            )),
                        };
                    }
                    "system_dict_path" => {
                        config.system_dict_path = if value.is_empty() {
                            None
                        } else {
                            Some(value.to_string())
                        };
                    }
                    "auto_learn" => {
                        config.auto_learn = value == "true";
                    }
                    _ => {} // 未知のキーは無視
                }
            }
        }
        Ok(config)
    }

    pub fn default_toml() -> String {
        r#"# Enpitsu 設定ファイル

[general]
# 入力モード切り替えキー: "zenkaku-hankaku" | "ctrl-space" | "alt-tilde"
toggle_key = "zenkaku-hankaku"

[dictionary]
# システム辞書パス（空の場合は DLL と同じディレクトリの dict/ を使用）
system_dict_path = ""

[behavior]
# 候補選択後に自動的に学習するか
auto_learn = true
"#.to_string()
    }
}
```

**動作確認:** `cargo test` で全テスト（157 + 新規 9 = 166 テスト）がパスすること（Green）

### 3-4. Refactor

**動作確認:** `cargo test` 全パス、`cargo clippy` 警告なし、`cargo fmt -- --check` 差分なし

---

## ステップ 4: lib.rs と Cargo.toml の更新

### 4-1. lib.rs にモジュール追加

```rust
pub mod config;
pub mod user_dictionary;
```

### 4-2. Cargo.toml の確認

今回の Phase 6 では新しい外部クレートは不要。
`encoding_rs` と `windows` クレートのみ。

**動作確認:**
- `cargo build` がエラーなく完了すること
- `cargo test` で全 166 テストがパスすること

---

## ステップ 5: CLI デモの拡張

`main.rs` にユーザー辞書と設定ファイルの対応を追加する。

### 5-1. コマンドラインオプション追加

```
enpitsu [--dict <path>] [--user-dict <path>] [--config <path>]
```

### 5-2. 実装

```rust
fn main() {
    let args: Vec<String> = std::env::args().collect();

    // --dict オプション
    let dict = /* 既存 */;

    // --user-dict オプション
    let user_dict = if let Some(pos) = args.iter().position(|a| a == "--user-dict") {
        let Some(path) = args.get(pos + 1) else {
            eprintln!("エラー: --user-dict の後にパスを指定してください");
            std::process::exit(1);
        };
        match UserDictionary::load(Path::new(path)) {
            Ok(ud) => {
                eprintln!("ユーザー辞書を読み込みました: {path}");
                Some(ud)
            }
            Err(e) => {
                eprintln!("ユーザー辞書の読み込みに失敗: {e}");
                None
            }
        }
    } else {
        None
    };

    let mut engine = ConversionEngine::new_with_user_dict(dict, user_dict);
    // ... 既存のループ ...
}
```

**動作確認:**
- `cargo run -- --dict tests/fixtures/test_dict.txt` で既存の動作が変わらないこと
- `cargo run -- --dict tests/fixtures/test_dict.txt --user-dict /tmp/ud.txt` でユーザー辞書が使えること

---

## ステップ 6: インストーラーの改善 (Windows 専用)

### 6-1. install.ps1 の拡張

既存の `installer/install.ps1` に以下の機能を追加:

```powershell
# インストール処理に追加:
# 1. %APPDATA%\enpitsu\ ディレクトリの作成
# 2. デフォルト config.toml の配置
# 3. dict/ ディレクトリの辞書ファイルを DLL と同じ場所にコピー

$appDataDir = Join-Path $env:APPDATA "enpitsu"
if (-not (Test-Path $appDataDir)) {
    New-Item -ItemType Directory -Path $appDataDir -Force
    Write-Host "設定ディレクトリを作成しました: $appDataDir"
}

# デフォルト設定ファイルの配置（上書きしない）
$configPath = Join-Path $appDataDir "config.toml"
if (-not (Test-Path $configPath)) {
    # デフォルト設定を書き込む
    Set-Content -Path $configPath -Value @"
# Enpitsu 設定ファイル

[general]
toggle_key = "zenkaku-hankaku"

[dictionary]
system_dict_path = ""

[behavior]
auto_learn = true
"@
    Write-Host "デフォルト設定ファイルを配置しました: $configPath"
}
```

### 6-2. アンインストール処理の拡張

```powershell
# アンインストール時に辞書と設定は残す（ユーザーデータ保護）
# ただし -PurgeData フラグで完全削除も可能にする

param(
    [switch]$Uninstall,
    [switch]$PurgeData
)

if ($Uninstall) {
    # ... 既存の DLL 登録解除 ...

    if ($PurgeData) {
        $appDataDir = Join-Path $env:APPDATA "enpitsu"
        if (Test-Path $appDataDir) {
            Remove-Item -Recurse -Force $appDataDir
            Write-Host "ユーザーデータを削除しました: $appDataDir"
        }
    } else {
        Write-Host "ユーザーデータは保持されます: $appDataDir"
        Write-Host "完全に削除するには -PurgeData フラグを使用してください"
    }
}
```

**動作確認:**
- Windows 環境でインストーラーを実行し、`%APPDATA%\enpitsu\` が作成されること
- `config.toml` がデフォルト内容で配置されること
- アンインストール後もユーザーデータが残ること
- `-PurgeData` フラグでユーザーデータが削除されること

---

## ステップ 7: TextService のユーザー辞書・設定連携 (Windows 専用)

### 7-1. TextService の ActivateEx でファイル読み込み

```rust
fn ActivateEx(&self, ptim: Option<&ITfThreadMgr>, tid: u32, _flags: u32) -> Result<()> {
    // ... 既存の処理 ...

    // 設定ファイルの読み込み
    let config_path = get_appdata_path("config.toml");
    let config = Config::load(&config_path).unwrap_or_else(|_| Config::default_config());

    // ユーザー辞書の読み込み
    let user_dict_path = get_appdata_path("user_dict.txt");
    let user_dict = UserDictionary::load(&user_dict_path).unwrap_or_default();

    // エンジンにユーザー辞書を設定
    // ...

    Ok(())
}

/// %APPDATA%\enpitsu\ 以下のパスを返す。
fn get_appdata_path(filename: &str) -> std::path::PathBuf {
    let appdata = std::env::var("APPDATA").unwrap_or_else(|_| ".".to_string());
    std::path::PathBuf::from(appdata).join("enpitsu").join(filename)
}
```

### 7-2. Deactivate でユーザー辞書を保存

```rust
fn Deactivate(&self) -> Result<()> {
    // ... 既存の処理 ...

    // ユーザー辞書の保存
    let mut engine = self.engine.lock().unwrap();
    if let Some(ud) = engine.user_dict_mut() {
        if ud.is_dirty() {
            let path = get_appdata_path("user_dict.txt");
            let _ = ud.save(&path);  // エラーは無視（ログで記録）
        }
    }

    Ok(())
}
```

**動作確認:**
- Windows 環境で IME を有効化し、`%APPDATA%\enpitsu\config.toml` が読み込まれること
- 変換を確定すると `%APPDATA%\enpitsu\user_dict.txt` が生成されること
- IME を無効化して再有効化すると、前回の学習結果が反映されること

---

## ステップ 8: README.md の作成

### 8-1. 内容

```markdown
# Enpitsu

Windows 向け日本語入力システム (IME)。

Rust で実装された SKK ライクな入力方式で、
ローマ字 → ひらがな → 漢字変換を行う。

## 特徴

- SKK 形式の辞書に対応
- ユーザー辞書による学習機能
- Text Services Framework (TSF) 対応
- 高速な変換処理

## インストール

### 必要環境

- Windows 10 以降
- [Rust](https://rustup.rs/) (ビルド時のみ)

### ビルドとインストール

\`\`\`powershell
# ビルド
cargo build --release

# インストール（管理者権限）
.\installer\install.ps1

# アンインストール
.\installer\install.ps1 -Uninstall
\`\`\`

## 使い方

1. インストール後、タスクバーの言語バーから「Enpitsu」を選択
2. ローマ字で入力し、Space で変換
3. 候補を選択して Enter で確定
4. Escape でキャンセル

### キーバインド

| キー | 動作 |
|------|------|
| Space | 変換 |
| Enter | 確定 |
| Escape | キャンセル |
| ↑ / ↓ | 候補の移動 |
| Backspace | 1文字削除 |

## 設定

設定ファイルは `%APPDATA%\enpitsu\config.toml` に保存される。

## 開発

\`\`\`sh
cargo build          # ビルド
cargo test           # テスト
cargo clippy         # Lint
cargo fmt            # フォーマット
cargo run            # CLI デモ
\`\`\`

## ライセンス

MIT License - Copyright 2026 shien
```

**動作確認:**
- README.md の手順に従ってビルド・インストールできることを確認（Windows 環境）
- Markdown が正しくレンダリングされることを GitHub で確認

---

## ステップ 9: CLAUDE.md の更新

### 9-1. ファイル構成の更新

`user_dictionary.rs` と `config.rs` を追記:

```
├── src/
│   ├── user_dictionary.rs # ユーザー辞書管理（学習・永続化）
│   ├── config.rs          # 設定ファイルのパース・デフォルト値
```

### 9-2. Common Commands の更新

```
| `cargo run -- --dict <path> --user-dict <path>` | CLI デモ（辞書・ユーザー辞書指定） |
```

**動作確認:**
- CLAUDE.md の内容がプロジェクトの現在の状態を正確に反映していること

---

## ステップ 10: 最終確認

### 10-1. Linux 環境での確認

```sh
cargo test               # 既存 143 + 新規 23 = 166 テストが全パス
cargo clippy             # 警告なし
cargo fmt -- --check     # フォーマット差分なし
cargo build              # エラーなし
```

### 10-2. Windows 環境での確認

```sh
cargo build --release    # DLL 生成
cargo test               # 全テストパス
```

手動テスト:

1. `installer/install.ps1` でインストール
2. `%APPDATA%\enpitsu\config.toml` が作成されること
3. メモ帳で変換し、候補を選択して確定
4. `%APPDATA%\enpitsu\user_dict.txt` が生成されること
5. IME を再起動して、前回の学習結果が反映されること
6. `config.toml` の `toggle_key` を変更して IME を再起動し、設定が反映されること
7. `install.ps1 -Uninstall` で DLL が登録解除されること
8. ユーザーデータ（辞書・設定）が残っていること
9. `install.ps1 -Uninstall -PurgeData` でユーザーデータも削除されること

### 10-3. コミット

- テスト全パスを確認した上でコミット

---

## 追加テスト一覧（予定）

| # | テスト名 | モジュール | 分類 | 内容 |
|---|---------|-----------|------|------|
| 1 | `new_is_empty` | user_dictionary | 基本操作 | 空の辞書の初期状態 |
| 2 | `record_and_lookup` | user_dictionary | 基本操作 | 記録と検索 |
| 3 | `record_multiple_candidates` | user_dictionary | 基本操作 | 複数候補の記録 |
| 4 | `record_existing_moves_to_front` | user_dictionary | 学習 | 既存候補の優先度上げ |
| 5 | `record_same_candidate_no_duplicate` | user_dictionary | 学習 | 同一候補の重複防止 |
| 6 | `save_and_load` | user_dictionary | 永続化 | 保存と読み込みの往復 |
| 7 | `load_nonexistent_returns_empty` | user_dictionary | 永続化 | 存在しないファイル |
| 8 | `save_clears_dirty_flag` | user_dictionary | 永続化 | dirty フラグのクリア |
| 9 | `lookup_not_found` | user_dictionary | 基本操作 | 存在しない読みの検索 |
| 10 | `user_dict_candidates_first` | engine | ユーザー辞書 | ユーザー辞書が先頭 |
| 11 | `user_dict_no_duplicate` | engine | ユーザー辞書 | 候補の重複排除 |
| 12 | `commit_records_to_user_dict` | engine | ユーザー辞書 | 確定時の学習記録 |
| 13 | `engine_without_user_dict_unchanged` | engine | ユーザー辞書 | 後方互換性 |
| 14 | `user_dict_only_no_system_dict` | engine | ユーザー辞書 | ユーザー辞書のみ |
| 15 | `default_config_values` | config | デフォルト | デフォルト設定値 |
| 16 | `parse_complete_config` | config | パース | 完全な設定のパース |
| 17 | `parse_partial_config_uses_defaults` | config | パース | 部分設定のパース |
| 18 | `parse_empty_config_returns_defaults` | config | パース | 空設定のパース |
| 19 | `parse_comments_and_blank_lines` | config | パース | コメント・空行の処理 |
| 20 | `parse_invalid_toggle_key` | config | パース | 不正な値のエラー |
| 21 | `load_nonexistent_returns_defaults` | config | ファイル | 存在しないファイル |
| 22 | `load_and_save_roundtrip` | config | ファイル | 保存→読み込みの往復 |
| 23 | `default_toml_is_parsable` | config | default_toml | 生成TOML のパース可能性 |

合計: 23 テスト追加（既存 143 + 新規 23 = 166 テスト）

Windows 専用コード（ステップ 6〜7）は `#[cfg(windows)]` で分離されており、
Linux 上のユニットテストには含まれない。Windows 環境での手動テストで検証する。

---

## 依存クレート（変更なし）

```toml
[dependencies]
encoding_rs = "0.8"

[target.'cfg(windows)'.dependencies]
windows = { version = "0.58", features = [
    "implement",
    "Win32_Foundation",
    "Win32_System_Com",
    "Win32_System_LibraryLoader",
    "Win32_System_Registry",
    "Win32_UI_Input_KeyboardAndMouse",
    "Win32_UI_TextServices",
] }
```

Phase 6 では新しい外部クレートは追加しない。
TOML パーサーは簡易実装とし、ユーザー辞書は既存の SKK 辞書パーサーを再利用する。

---

## 実装上の注意事項

### ユーザー辞書

- **ファイルロックなし**: 単一プロセス想定のため、ファイルロックは不要。
  IME の Deactivate 時にのみ保存する。
- **文字エンコーディング**: ユーザー辞書は常に UTF-8 で保存する（EUC-JP 不要）。
- **エントリ順序**: `HashMap` を使用するが、保存時はキーをソートして書き出す（差分管理しやすいため）。
- **パフォーマンス**: ユーザー辞書は小規模（数百〜数千エントリ）想定のため、
  `HashMap` で十分。

### 設定ファイル

- **簡易 TOML パーサー**: セクションヘッダー `[section]` は認識するが、
  ネストは不要。`key = "value"` 形式のみ対応。
- **未知のキー**: 無視する（将来の拡張に対応）。
- **ファイルなし**: 設定ファイルが存在しない場合はデフォルト設定を使用する。

### インストーラー

- **ユーザーデータ保護**: アンインストール時にユーザー辞書と設定ファイルは
  デフォルトで残す。`-PurgeData` フラグで完全削除。
- **設定ファイルの上書き**: インストール時、既存の `config.toml` は上書きしない。

### 後方互換性

- `ConversionEngine::new()` は既存のまま維持する。
  ユーザー辞書なしで動作する。
- `new_with_user_dict()` を追加して、ユーザー辞書を使う場合のコンストラクタとする。
- 既存の 143 テストはすべて変更なしでパスすること。

### 段階的な動作確認

1. **ステップ 1**: `cargo test` で UserDictionary のテスト 9 個がパス
2. **ステップ 2**: `cargo test` でエンジン連携テスト 5 個がパス
3. **ステップ 3**: `cargo test` で Config テスト 9 個がパス
4. **ステップ 4**: `cargo build` + `cargo test` で全 166 テストがパス
5. **ステップ 5**: CLI デモで辞書・ユーザー辞書が使えること
6. **ステップ 6-7**: Windows 環境で手動テスト
7. **ステップ 8-9**: ドキュメント確認
8. **ステップ 10**: 全テスト + clippy + fmt の最終確認
