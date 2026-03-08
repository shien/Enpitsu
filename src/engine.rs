//! 変換エンジン。
//!
//! ローマ字入力 → ひらがな変換 → 辞書検索 → 候補選択 → 確定
//! の一連の変換パイプラインを管理する。

use crate::candidate::CandidateList;
use crate::dictionary::Dictionary;
use crate::input_state::InputState;
use crate::user_dictionary::UserDictionary;

/// エンジンの状態。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EngineState {
    /// 直接入力（変換なし）
    Direct,
    /// ローマ字→かな変換中（未確定文字列あり）
    Composing,
    /// 候補選択中
    Converting,
}

/// エンジンへのコマンド。
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum EngineCommand {
    /// 文字入力
    InsertChar(char),
    /// 変換開始 (Space)
    Convert,
    /// 次の候補
    NextCandidate,
    /// 前の候補
    PrevCandidate,
    /// 確定 (Enter)
    Commit,
    /// キャンセル (Escape)
    Cancel,
    /// 1文字削除
    Backspace,
    /// カーソルを左に移動
    CursorLeft,
    /// カーソルを右に移動
    CursorRight,
    /// カーソル後ろの1文字を削除
    Delete,
}

/// エンジンの処理結果。
#[derive(Debug, Clone)]
pub struct EngineOutput {
    /// 確定してアプリケーションに渡すテキスト
    pub committed: String,
    /// 現在の表示用テキスト（未確定文字列 or 選択中の候補）
    pub display: String,
    /// 候補リスト内の選択インデックス
    pub candidate_index: Option<usize>,
    /// display 内のカーソル位置（文字数）
    pub cursor_pos: usize,
}

/// 変換エンジン。
pub struct ConversionEngine {
    state: EngineState,
    input: InputState,
    dict: Option<Dictionary>,
    user_dict: Option<UserDictionary>,
    candidates: Option<CandidateList>,
    /// 変換時の読み（ひらがな）を保持する。
    reading: String,
}

impl ConversionEngine {
    /// 新しい変換エンジンを作成する。
    pub fn new(dict: Option<Dictionary>) -> Self {
        Self {
            state: EngineState::Direct,
            input: InputState::new(),
            dict,
            user_dict: None,
            candidates: None,
            reading: String::new(),
        }
    }

    /// ユーザー辞書付きの変換エンジンを作成する。
    pub fn new_with_user_dict(dict: Option<Dictionary>, user_dict: Option<UserDictionary>) -> Self {
        Self {
            state: EngineState::Direct,
            input: InputState::new(),
            dict,
            user_dict,
            candidates: None,
            reading: String::new(),
        }
    }

    /// ユーザー辞書の可変参照を返す。
    pub fn user_dict_mut(&mut self) -> Option<&mut UserDictionary> {
        self.user_dict.as_mut()
    }

    /// 現在の状態を返す。
    pub fn state(&self) -> EngineState {
        self.state
    }

    /// 変換候補リストを返す（Converting 状態のとき Some）。
    pub fn candidates(&self) -> Option<&[String]> {
        self.candidates.as_ref().map(|cl| cl.candidates())
    }

    /// 変換時の読み（ひらがな）を返す。
    pub fn reading(&self) -> &str {
        &self.reading
    }

    /// コマンドを処理し、結果を返す。
    pub fn process(&mut self, command: EngineCommand) -> EngineOutput {
        #[cfg(debug_assertions)]
        eprintln!(
            "[Engine::process] state={:?} command={:?}",
            self.state, command
        );

        let result = match (&self.state, &command) {
            // === Direct ===
            (EngineState::Direct, EngineCommand::InsertChar(ch)) => {
                self.input.feed_char(*ch);
                self.state = EngineState::Composing;
                self.composing_output()
            }
            (EngineState::Direct, _) => self.empty_output(),

            // === Composing ===
            (EngineState::Composing, EngineCommand::InsertChar(ch)) => {
                self.input.feed_char(*ch);
                self.composing_output()
            }
            (EngineState::Composing, EngineCommand::Convert) => self.do_convert(),
            (EngineState::Composing, EngineCommand::Commit) => {
                self.input.flush();
                let committed = self.input.output().to_string();
                self.input.reset();
                self.state = EngineState::Direct;
                EngineOutput {
                    committed,
                    display: String::new(),
                    candidate_index: None,
                    cursor_pos: 0,
                }
            }
            (EngineState::Composing, EngineCommand::Cancel) => {
                self.input.reset();
                self.state = EngineState::Direct;
                self.empty_output()
            }
            (EngineState::Composing, EngineCommand::Backspace) => {
                self.input.backspace();
                if self.input.is_empty() {
                    self.state = EngineState::Direct;
                }
                self.composing_output()
            }
            (EngineState::Composing, EngineCommand::CursorLeft) => {
                self.input.move_left();
                self.composing_output()
            }
            (EngineState::Composing, EngineCommand::CursorRight) => {
                self.input.move_right();
                self.composing_output()
            }
            (EngineState::Composing, EngineCommand::Delete) => {
                self.input.delete();
                if self.input.is_empty() {
                    self.state = EngineState::Direct;
                }
                self.composing_output()
            }
            (EngineState::Composing, _) => self.composing_output(),

            // === Converting ===
            (EngineState::Converting, EngineCommand::NextCandidate)
            | (EngineState::Converting, EngineCommand::Convert) => {
                if let Some(ref mut cl) = self.candidates {
                    cl.next();
                }
                self.converting_output()
            }
            (EngineState::Converting, EngineCommand::PrevCandidate) => {
                if let Some(ref mut cl) = self.candidates {
                    cl.prev();
                }
                self.converting_output()
            }
            (EngineState::Converting, EngineCommand::Commit) => {
                let committed = self
                    .candidates
                    .as_ref()
                    .and_then(|cl| cl.select())
                    .unwrap_or_default();
                // ユーザー辞書に学習データを記録
                if let Some(ref mut ud) = self.user_dict
                    && !committed.is_empty()
                    && !self.reading.is_empty()
                {
                    ud.record(&self.reading, &committed);
                }
                self.candidates = None;
                self.input.reset();
                self.state = EngineState::Direct;
                EngineOutput {
                    committed,
                    display: String::new(),
                    candidate_index: None,
                    cursor_pos: 0,
                }
            }
            (EngineState::Converting, EngineCommand::Cancel) => {
                self.candidates = None;
                self.state = EngineState::Composing;
                self.composing_output()
            }
            (EngineState::Converting, EngineCommand::InsertChar(ch)) => {
                // 現在の候補を確定し、新しい文字で Composing を開始する
                let committed = self
                    .candidates
                    .as_ref()
                    .and_then(|cl| cl.select())
                    .unwrap_or_default();
                // ユーザー辞書に学習データを記録
                if let Some(ref mut ud) = self.user_dict
                    && !committed.is_empty()
                    && !self.reading.is_empty()
                {
                    ud.record(&self.reading, &committed);
                }
                self.candidates = None;
                self.input.reset();
                self.input.feed_char(*ch);
                self.state = EngineState::Composing;
                let composing = self.composing_output();
                EngineOutput {
                    committed,
                    display: composing.display,
                    candidate_index: None,
                    cursor_pos: composing.cursor_pos,
                }
            }
            (EngineState::Converting, EngineCommand::Backspace) => {
                // 変換をキャンセルして Composing に戻る（Cancel と同じ動作）
                self.candidates = None;
                self.state = EngineState::Composing;
                self.composing_output()
            }
            // Converting 状態でのカーソル移動・Delete は無視
            (EngineState::Converting, _) => self.converting_output(),
        };

        #[cfg(debug_assertions)]
        eprintln!(
            "[Engine::process] → state={:?} cursor_pos={} display='{}'",
            self.state, result.cursor_pos, result.display
        );

        result
    }

    /// 変換を実行する。候補があれば Converting へ、なければひらがな確定。
    fn do_convert(&mut self) -> EngineOutput {
        self.input.flush();
        let hiragana = self.input.output().to_string();
        self.reading = hiragana.clone();

        // ユーザー辞書とシステム辞書の候補をマージ
        let merged = self.merge_candidates(&hiragana);

        if merged.is_empty() {
            // 候補なし → ひらがなを確定
            let committed = hiragana;
            self.input.reset();
            self.state = EngineState::Direct;
            EngineOutput {
                committed,
                display: String::new(),
                candidate_index: None,
                cursor_pos: 0,
            }
        } else {
            let cl = CandidateList::new(merged);
            let display = cl.current().unwrap_or("").to_string();
            let cursor_pos = display.chars().count();
            let idx = cl.index();
            self.candidates = Some(cl);
            self.state = EngineState::Converting;
            EngineOutput {
                committed: String::new(),
                display,
                candidate_index: Some(idx),
                cursor_pos,
            }
        }
    }

    /// ユーザー辞書とシステム辞書の候補をマージする。
    /// ユーザー辞書の候補を先頭に配置し、システム辞書の候補のうち
    /// ユーザー辞書に含まれないものを後ろに追加する。
    fn merge_candidates(&self, reading: &str) -> Vec<String> {
        let user_cands: Vec<String> = self
            .user_dict
            .as_ref()
            .and_then(|ud| ud.lookup(reading))
            .map(|c| c.to_vec())
            .unwrap_or_default();

        let system_cands: Vec<String> = self
            .dict
            .as_ref()
            .and_then(|d| d.lookup(reading))
            .map(|c| c.iter().map(|s| s.to_string()).collect())
            .unwrap_or_default();

        let mut merged = user_cands;
        for c in system_cands {
            if !merged.contains(&c) {
                merged.push(c);
            }
        }
        merged
    }

    /// Composing 状態の EngineOutput を組み立てる。
    fn composing_output(&self) -> EngineOutput {
        let display = self.input.display();
        let cursor_pos = self.input.cursor_pos();
        EngineOutput {
            committed: String::new(),
            display,
            candidate_index: None,
            cursor_pos,
        }
    }

    /// Converting 状態の EngineOutput を組み立てる。
    fn converting_output(&self) -> EngineOutput {
        match &self.candidates {
            Some(cl) => {
                let display = cl.current().unwrap_or("").to_string();
                let cursor_pos = display.chars().count();
                EngineOutput {
                    committed: String::new(),
                    display,
                    candidate_index: Some(cl.index()),
                    cursor_pos,
                }
            }
            None => self.empty_output(),
        }
    }

    /// 空の EngineOutput を返す。
    fn empty_output(&self) -> EngineOutput {
        EngineOutput {
            committed: String::new(),
            display: String::new(),
            candidate_index: None,
            cursor_pos: 0,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::Path;

    fn test_engine() -> ConversionEngine {
        let dict = Dictionary::load_from_file(Path::new("tests/fixtures/test_dict.txt")).unwrap();
        ConversionEngine::new(Some(dict))
    }

    fn engine_without_dict() -> ConversionEngine {
        ConversionEngine::new(None)
    }

    // === 初期状態 ===

    #[test]
    fn initial_state_is_direct() {
        let engine = test_engine();
        assert_eq!(engine.state(), EngineState::Direct);
    }

    // === Direct → Composing ===

    #[test]
    fn insert_char_transitions_to_composing() {
        let mut engine = test_engine();
        engine.process(EngineCommand::InsertChar('k'));
        assert_eq!(engine.state(), EngineState::Composing);
    }

    // === Composing → Converting ===

    #[test]
    fn convert_transitions_to_converting() {
        let mut engine = test_engine();
        for ch in "kanji".chars() {
            engine.process(EngineCommand::InsertChar(ch));
        }
        engine.process(EngineCommand::Convert);
        assert_eq!(engine.state(), EngineState::Converting);
        assert!(engine.candidates().is_some());
    }

    // === Converting → Direct (Commit) ===

    #[test]
    fn commit_in_converting_transitions_to_direct() {
        let mut engine = test_engine();
        for ch in "kanji".chars() {
            engine.process(EngineCommand::InsertChar(ch));
        }
        engine.process(EngineCommand::Convert);
        let output = engine.process(EngineCommand::Commit);
        assert_eq!(engine.state(), EngineState::Direct);
        assert!(!output.committed.is_empty());
    }

    // === Composing → Direct (Commit: ひらがな確定) ===

    #[test]
    fn commit_in_composing_confirms_hiragana() {
        let mut engine = test_engine();
        for ch in "kanji".chars() {
            engine.process(EngineCommand::InsertChar(ch));
        }
        let output = engine.process(EngineCommand::Commit);
        assert_eq!(engine.state(), EngineState::Direct);
        assert_eq!(output.committed, "かんじ");
    }

    // === Composing → Direct (Cancel) ===

    #[test]
    fn cancel_in_composing_discards_input() {
        let mut engine = test_engine();
        for ch in "ka".chars() {
            engine.process(EngineCommand::InsertChar(ch));
        }
        let output = engine.process(EngineCommand::Cancel);
        assert_eq!(engine.state(), EngineState::Direct);
        assert_eq!(output.committed, "");
        assert_eq!(output.display, "");
    }

    // === Converting → Composing (Cancel) ===

    #[test]
    fn cancel_in_converting_returns_to_composing() {
        let mut engine = test_engine();
        for ch in "kanji".chars() {
            engine.process(EngineCommand::InsertChar(ch));
        }
        engine.process(EngineCommand::Convert);
        let output = engine.process(EngineCommand::Cancel);
        assert_eq!(engine.state(), EngineState::Composing);
        assert_eq!(output.display, "かんじ");
    }

    // === Direct では無視されるコマンド ===

    #[test]
    fn convert_in_direct_is_noop() {
        let mut engine = test_engine();
        let output = engine.process(EngineCommand::Convert);
        assert_eq!(engine.state(), EngineState::Direct);
        assert_eq!(output.committed, "");
    }

    #[test]
    fn commit_in_direct_is_noop() {
        let mut engine = test_engine();
        let output = engine.process(EngineCommand::Commit);
        assert_eq!(engine.state(), EngineState::Direct);
        assert_eq!(output.committed, "");
    }

    // === 候補ナビゲーション ===

    #[test]
    fn next_candidate_moves_selection() {
        let mut engine = test_engine();
        for ch in "kanji".chars() {
            engine.process(EngineCommand::InsertChar(ch));
        }
        engine.process(EngineCommand::Convert);
        let output = engine.process(EngineCommand::NextCandidate);
        assert_eq!(engine.state(), EngineState::Converting);
        // "かんじ" → ["漢字", "感じ", "幹事"], next → index=1 "感じ"
        assert_eq!(output.candidate_index, Some(1));
    }

    #[test]
    fn prev_candidate_moves_selection() {
        let mut engine = test_engine();
        for ch in "kanji".chars() {
            engine.process(EngineCommand::InsertChar(ch));
        }
        engine.process(EngineCommand::Convert);
        engine.process(EngineCommand::NextCandidate); // index=1
        let output = engine.process(EngineCommand::PrevCandidate); // index=0
        assert_eq!(output.candidate_index, Some(0));
    }

    #[test]
    fn convert_in_converting_acts_as_next() {
        let mut engine = test_engine();
        for ch in "kanji".chars() {
            engine.process(EngineCommand::InsertChar(ch));
        }
        engine.process(EngineCommand::Convert); // index=0
        let output = engine.process(EngineCommand::Convert); // index=1
        assert_eq!(output.candidate_index, Some(1));
    }

    #[test]
    fn commit_in_converting_confirms_candidate() {
        let mut engine = test_engine();
        for ch in "kanji".chars() {
            engine.process(EngineCommand::InsertChar(ch));
        }
        engine.process(EngineCommand::Convert);
        engine.process(EngineCommand::NextCandidate);
        let output = engine.process(EngineCommand::Commit);
        assert_eq!(output.committed, "感じ");
        assert_eq!(engine.state(), EngineState::Direct);
    }

    // === Backspace ===

    #[test]
    fn backspace_in_composing_removes_char() {
        let mut engine = test_engine();
        for ch in "ka".chars() {
            engine.process(EngineCommand::InsertChar(ch));
        }
        // output="か", pending=""
        let output = engine.process(EngineCommand::Backspace);
        assert_eq!(output.display, "");
        assert_eq!(engine.state(), EngineState::Direct);
    }

    #[test]
    fn backspace_in_composing_with_pending() {
        let mut engine = test_engine();
        engine.process(EngineCommand::InsertChar('k'));
        // pending="k"
        let output = engine.process(EngineCommand::Backspace);
        assert_eq!(output.display, "");
        assert_eq!(engine.state(), EngineState::Direct);
    }

    #[test]
    fn backspace_in_composing_partial_removal() {
        // "kak" → output="か", pending="k" → backspace → output="か", pending=""
        let mut engine = test_engine();
        for ch in "kak".chars() {
            engine.process(EngineCommand::InsertChar(ch));
        }
        let output = engine.process(EngineCommand::Backspace);
        assert_eq!(output.display, "か");
        assert_eq!(engine.state(), EngineState::Composing);
    }

    // === Converting で InsertChar → 自動確定+新規入力 ===

    #[test]
    fn insert_char_in_converting_auto_commits() {
        // "kanji" → Convert → 'a' → 候補「漢字」が確定され、'a' の入力が開始
        let mut engine = test_engine();
        for ch in "kanji".chars() {
            engine.process(EngineCommand::InsertChar(ch));
        }
        engine.process(EngineCommand::Convert); // Converting, display="漢字"
        let output = engine.process(EngineCommand::InsertChar('a'));
        assert_eq!(output.committed, "漢字");
        assert_eq!(engine.state(), EngineState::Composing);
        assert_eq!(output.display, "あ");
    }

    #[test]
    fn insert_char_in_converting_after_next() {
        // 2番目の候補を選択中に文字入力 → 2番目の候補が確定
        let mut engine = test_engine();
        for ch in "kanji".chars() {
            engine.process(EngineCommand::InsertChar(ch));
        }
        engine.process(EngineCommand::Convert);
        engine.process(EngineCommand::NextCandidate); // "感じ"
        let output = engine.process(EngineCommand::InsertChar('k'));
        assert_eq!(output.committed, "感じ");
        assert_eq!(engine.state(), EngineState::Composing);
        assert_eq!(output.display, "k");
    }

    // === Converting で Backspace → Composing に戻る ===

    #[test]
    fn backspace_in_converting_returns_to_composing() {
        let mut engine = test_engine();
        for ch in "kanji".chars() {
            engine.process(EngineCommand::InsertChar(ch));
        }
        engine.process(EngineCommand::Convert);
        let output = engine.process(EngineCommand::Backspace);
        assert_eq!(engine.state(), EngineState::Composing);
        assert_eq!(output.display, "かんじ");
    }

    // === reading() getter ===

    #[test]
    fn reading_available_after_convert() {
        let mut engine = test_engine();
        for ch in "kanji".chars() {
            engine.process(EngineCommand::InsertChar(ch));
        }
        engine.process(EngineCommand::Convert);
        assert_eq!(engine.reading(), "かんじ");
    }

    // === 辞書なし / 候補なしでの変換 ===

    #[test]
    fn convert_without_dict_confirms_hiragana() {
        let mut engine = engine_without_dict();
        for ch in "kanji".chars() {
            engine.process(EngineCommand::InsertChar(ch));
        }
        let output = engine.process(EngineCommand::Convert);
        assert_eq!(engine.state(), EngineState::Direct);
        assert_eq!(output.committed, "かんじ");
    }

    #[test]
    fn convert_no_candidates_confirms_hiragana() {
        let mut engine = test_engine();
        for ch in "aaaaa".chars() {
            engine.process(EngineCommand::InsertChar(ch));
        }
        let output = engine.process(EngineCommand::Convert);
        assert_eq!(engine.state(), EngineState::Direct);
        assert_eq!(output.committed, "あああああ");
    }

    // === 統合テスト: 一連のフロー ===

    #[test]
    fn full_flow_kanji_convert_commit() {
        // "kanji" → Space → 1番目の候補「漢字」を確定
        let mut engine = test_engine();
        for ch in "kanji".chars() {
            engine.process(EngineCommand::InsertChar(ch));
        }
        let output = engine.process(EngineCommand::Convert);
        assert_eq!(output.display, "漢字");
        assert!(engine.candidates().is_some());

        let output = engine.process(EngineCommand::Commit);
        assert_eq!(output.committed, "漢字");
        assert_eq!(engine.state(), EngineState::Direct);
    }

    #[test]
    fn full_flow_select_second_candidate() {
        // "kanji" → Space → Next → 2番目の候補「感じ」を確定
        let mut engine = test_engine();
        for ch in "kanji".chars() {
            engine.process(EngineCommand::InsertChar(ch));
        }
        engine.process(EngineCommand::Convert);
        engine.process(EngineCommand::NextCandidate);
        let output = engine.process(EngineCommand::Commit);
        assert_eq!(output.committed, "感じ");
    }

    #[test]
    fn full_flow_cancel_and_re_edit() {
        // "kanji" → Space → Cancel → "ha" 追加 → Commit
        let mut engine = test_engine();
        for ch in "kanji".chars() {
            engine.process(EngineCommand::InsertChar(ch));
        }
        engine.process(EngineCommand::Convert);
        engine.process(EngineCommand::Cancel); // → Composing, display="かんじ"

        for ch in "ha".chars() {
            engine.process(EngineCommand::InsertChar(ch));
        }
        let output = engine.process(EngineCommand::Commit);
        assert_eq!(output.committed, "かんじは");
    }

    #[test]
    fn full_flow_consecutive_conversions() {
        // 1回目: "kanji" → Convert → Commit → "漢字"
        // 2回目: "nihon" → Convert → Commit → "日本"
        let mut engine = test_engine();
        for ch in "kanji".chars() {
            engine.process(EngineCommand::InsertChar(ch));
        }
        engine.process(EngineCommand::Convert);
        let output1 = engine.process(EngineCommand::Commit);
        assert_eq!(output1.committed, "漢字");

        for ch in "nihon".chars() {
            engine.process(EngineCommand::InsertChar(ch));
        }
        engine.process(EngineCommand::Convert);
        let output2 = engine.process(EngineCommand::Commit);
        assert_eq!(output2.committed, "日本");
    }

    #[test]
    fn display_updates_during_composing() {
        // 逐次入力中に display が更新される
        let mut engine = test_engine();
        let output = engine.process(EngineCommand::InsertChar('k'));
        assert_eq!(output.display, "k"); // pending

        let output = engine.process(EngineCommand::InsertChar('a'));
        assert_eq!(output.display, "か"); // output

        let output = engine.process(EngineCommand::InsertChar('n'));
        assert_eq!(output.display, "かn"); // output + pending
    }

    // === Emacs キーバインド統合テスト ===
    //
    // key_mapping のユニットテストに加え、エンジン経由で
    // Emacs キーバインドのコマンドが正しく動作することを確認する。
    // ここでは EngineCommand を直接渡すため key_mapping は経由しないが、
    // Ctrl+キーで発行されるコマンドがエンジンの各状態で正しく処理されることを検証する。

    #[test]
    fn emacs_ctrl_j_composing_commits_hiragana() {
        // Composing 状態で Commit (Ctrl+J 相当) → ひらがな確定 → Direct
        let mut engine = test_engine();
        for ch in "kanji".chars() {
            engine.process(EngineCommand::InsertChar(ch));
        }
        assert_eq!(engine.state(), EngineState::Composing);
        let output = engine.process(EngineCommand::Commit);
        assert_eq!(engine.state(), EngineState::Direct);
        assert_eq!(output.committed, "かんじ");
    }

    #[test]
    fn emacs_ctrl_j_converting_commits_candidate() {
        // Converting 状態で Commit (Ctrl+J 相当) → 候補確定 → Direct
        let mut engine = test_engine();
        for ch in "kanji".chars() {
            engine.process(EngineCommand::InsertChar(ch));
        }
        engine.process(EngineCommand::Convert);
        assert_eq!(engine.state(), EngineState::Converting);
        let output = engine.process(EngineCommand::Commit);
        assert_eq!(engine.state(), EngineState::Direct);
        assert_eq!(output.committed, "漢字");
    }

    #[test]
    fn emacs_ctrl_g_composing_cancels() {
        // Composing 状態で Cancel (Ctrl+G 相当) → 入力破棄 → Direct
        let mut engine = test_engine();
        for ch in "ka".chars() {
            engine.process(EngineCommand::InsertChar(ch));
        }
        assert_eq!(engine.state(), EngineState::Composing);
        let output = engine.process(EngineCommand::Cancel);
        assert_eq!(engine.state(), EngineState::Direct);
        assert_eq!(output.committed, "");
        assert_eq!(output.display, "");
    }

    #[test]
    fn emacs_ctrl_g_converting_returns_to_composing() {
        // Converting 状態で Cancel (Ctrl+G 相当) → Composing に戻る
        let mut engine = test_engine();
        for ch in "kanji".chars() {
            engine.process(EngineCommand::InsertChar(ch));
        }
        engine.process(EngineCommand::Convert);
        assert_eq!(engine.state(), EngineState::Converting);
        let output = engine.process(EngineCommand::Cancel);
        assert_eq!(engine.state(), EngineState::Composing);
        assert_eq!(output.display, "かんじ");
    }

    #[test]
    fn emacs_ctrl_n_p_converting_navigates() {
        // Converting 状態で NextCandidate/PrevCandidate (Ctrl+N/P 相当) → 候補移動
        let mut engine = test_engine();
        for ch in "kanji".chars() {
            engine.process(EngineCommand::InsertChar(ch));
        }
        engine.process(EngineCommand::Convert);
        // Ctrl+N → 次の候補
        let output = engine.process(EngineCommand::NextCandidate);
        assert_eq!(output.candidate_index, Some(1));
        assert_eq!(output.display, "感じ");
        // Ctrl+P → 前の候補に戻る
        let output = engine.process(EngineCommand::PrevCandidate);
        assert_eq!(output.candidate_index, Some(0));
        assert_eq!(output.display, "漢字");
    }

    #[test]
    fn emacs_ctrl_h_composing_backspace() {
        // Composing 状態で Backspace (Ctrl+H 相当) → 1文字削除
        let mut engine = test_engine();
        for ch in "kak".chars() {
            engine.process(EngineCommand::InsertChar(ch));
        }
        // output="か", pending="k"
        let output = engine.process(EngineCommand::Backspace);
        assert_eq!(output.display, "か");
        assert_eq!(engine.state(), EngineState::Composing);
    }

    // === ユーザー辞書連携 ===

    fn test_engine_with_user_dict() -> ConversionEngine {
        let dict = Dictionary::load_from_file(Path::new("tests/fixtures/test_dict.txt")).unwrap();
        let mut user_dict = UserDictionary::new();
        user_dict.record("かんじ", "感じ");
        ConversionEngine::new_with_user_dict(Some(dict), Some(user_dict))
    }

    #[test]
    fn user_dict_candidates_first() {
        let mut engine = test_engine_with_user_dict();
        for ch in "kanji".chars() {
            engine.process(EngineCommand::InsertChar(ch));
        }
        let output = engine.process(EngineCommand::Convert);
        // ユーザー辞書の "感じ" が先頭
        assert_eq!(output.display, "感じ");
        let candidates = engine.candidates().unwrap();
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
        engine.process(EngineCommand::Convert);
        let candidates = engine.candidates().unwrap();
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
        let candidates = engine.candidates().unwrap();
        assert_eq!(candidates, &["感じ"]);
    }

    // === カーソル移動 ===

    #[test]
    fn cursor_left_in_composing_moves_cursor() {
        let mut engine = test_engine();
        for ch in "kaki".chars() {
            engine.process(EngineCommand::InsertChar(ch));
        }
        let output = engine.process(EngineCommand::CursorLeft);
        assert_eq!(output.cursor_pos, 1); // "か|き"
        assert_eq!(output.display, "かき");
    }

    #[test]
    fn cursor_right_in_composing_moves_cursor() {
        let mut engine = test_engine();
        for ch in "kaki".chars() {
            engine.process(EngineCommand::InsertChar(ch));
        }
        engine.process(EngineCommand::CursorLeft); // "か|き"
        let output = engine.process(EngineCommand::CursorRight); // "かき|"
        assert_eq!(output.cursor_pos, 2);
    }

    #[test]
    fn cursor_left_in_direct_is_noop() {
        let mut engine = test_engine();
        let output = engine.process(EngineCommand::CursorLeft);
        assert_eq!(engine.state(), EngineState::Direct);
        assert_eq!(output.display, "");
    }

    #[test]
    fn cursor_right_in_direct_is_noop() {
        let mut engine = test_engine();
        let output = engine.process(EngineCommand::CursorRight);
        assert_eq!(engine.state(), EngineState::Direct);
        assert_eq!(output.display, "");
    }

    #[test]
    fn cursor_left_in_converting_is_noop() {
        let mut engine = test_engine();
        for ch in "kanji".chars() {
            engine.process(EngineCommand::InsertChar(ch));
        }
        engine.process(EngineCommand::Convert);
        let output = engine.process(EngineCommand::CursorLeft);
        assert_eq!(engine.state(), EngineState::Converting);
        assert!(output.candidate_index.is_some());
    }

    #[test]
    fn insert_at_cursor_in_composing() {
        let mut engine = test_engine();
        for ch in "kaki".chars() {
            engine.process(EngineCommand::InsertChar(ch));
        }
        engine.process(EngineCommand::CursorLeft); // "か|き"
        let output = engine.process(EngineCommand::InsertChar('a')); // "かあ|き"
        assert_eq!(output.display, "かあき");
    }

    #[test]
    fn backspace_at_cursor_in_composing() {
        let mut engine = test_engine();
        for ch in "kaki".chars() {
            engine.process(EngineCommand::InsertChar(ch));
        }
        engine.process(EngineCommand::CursorLeft); // "か|き"
        let output = engine.process(EngineCommand::Backspace); // "|き"
        assert_eq!(output.display, "き");
    }

    #[test]
    fn delete_at_cursor_in_composing() {
        let mut engine = test_engine();
        for ch in "kaki".chars() {
            engine.process(EngineCommand::InsertChar(ch));
        }
        engine.process(EngineCommand::CursorLeft); // "か|き"
        let output = engine.process(EngineCommand::Delete); // "か|"
        assert_eq!(output.display, "か");
    }

    #[test]
    fn delete_in_direct_is_noop() {
        let mut engine = test_engine();
        let output = engine.process(EngineCommand::Delete);
        assert_eq!(engine.state(), EngineState::Direct);
        assert_eq!(output.display, "");
    }

    #[test]
    fn delete_in_converting_is_noop() {
        let mut engine = test_engine();
        for ch in "kanji".chars() {
            engine.process(EngineCommand::InsertChar(ch));
        }
        engine.process(EngineCommand::Convert);
        let output = engine.process(EngineCommand::Delete);
        assert_eq!(engine.state(), EngineState::Converting);
        assert!(output.candidate_index.is_some());
    }

    #[test]
    fn delete_empties_composing_returns_to_direct() {
        let mut engine = test_engine();
        for ch in "ka".chars() {
            engine.process(EngineCommand::InsertChar(ch));
        }
        engine.process(EngineCommand::CursorLeft); // "|か"
        engine.process(EngineCommand::Delete); // "|"
        assert_eq!(engine.state(), EngineState::Direct);
    }

    #[test]
    fn cursor_pos_in_output() {
        let mut engine = test_engine();
        for ch in "kaki".chars() {
            engine.process(EngineCommand::InsertChar(ch));
        }
        let output = engine.process(EngineCommand::InsertChar('a')); // "かきあ|"
        assert_eq!(output.cursor_pos, 3);
    }
}
