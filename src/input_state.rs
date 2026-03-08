//! 入力状態管理。
//!
//! ローマ字を1文字ずつ受け取り、逐次的にひらがなへ変換する。
//! バッファと確定済み出力を保持する。

use crate::romaji;

/// 入力状態を管理する構造体。
#[derive(Debug, Clone)]
pub struct InputState {
    /// カーソル前の確定したひらがな出力
    output_before: String,
    /// カーソル後の確定したひらがな出力
    output_after: String,
    /// まだ確定していないローマ字バッファ（常に output_before と output_after の間）
    pending: String,
}

impl InputState {
    /// 新しい InputState を作成する。
    pub fn new() -> Self {
        Self {
            output_before: String::new(),
            output_after: String::new(),
            pending: String::new(),
        }
    }

    /// 1文字入力する。確定したひらがながあれば output_before に追加される。
    pub fn feed_char(&mut self, ch: char) {
        self.pending.push(ch);
        let result = romaji::convert(&self.pending);
        self.output_before.push_str(&result.output);
        self.pending = result.pending;
        #[cfg(debug_assertions)]
        eprintln!(
            "[InputState::feed_char] ch='{}' before='{}'|pending='{}'|after='{}'",
            ch, self.output_before, self.pending, self.output_after
        );
    }

    /// 未確定バッファを確定する（末尾の "n" → "ん"）。
    pub fn flush(&mut self) {
        if self.pending == "n" {
            self.output_before.push('ん');
            self.pending.clear();
        } else if !self.pending.is_empty() {
            self.output_before.push_str(&self.pending);
            self.pending.clear();
        }
        #[cfg(debug_assertions)]
        eprintln!(
            "[InputState::flush] before='{}'|pending='{}'|after='{}'",
            self.output_before, self.pending, self.output_after
        );
    }

    /// バッファと出力をクリアする。
    pub fn reset(&mut self) {
        self.output_before.clear();
        self.output_after.clear();
        self.pending.clear();
    }

    /// 確定済みの出力を返す（output_before + output_after）。
    pub fn output(&self) -> String {
        format!("{}{}", self.output_before, self.output_after)
    }

    /// カーソル前の確定済み出力を返す。
    pub fn output_before(&self) -> &str {
        &self.output_before
    }

    /// カーソル後の確定済み出力を返す。
    pub fn output_after(&self) -> &str {
        &self.output_after
    }

    /// 未確定のバッファを返す。
    pub fn pending(&self) -> &str {
        &self.pending
    }

    /// 表示用文字列を返す（output_before + pending + output_after）。
    pub fn display(&self) -> String {
        format!(
            "{}{}{}",
            self.output_before, self.pending, self.output_after
        )
    }

    /// 末尾の1文字を削除する。pending があれば pending から、なければ output_before から削除。
    pub fn backspace(&mut self) {
        if !self.pending.is_empty() {
            self.pending.pop();
        } else {
            self.output_before.pop();
        }
        #[cfg(debug_assertions)]
        eprintln!(
            "[InputState::backspace] before='{}'|pending='{}'|after='{}'",
            self.output_before, self.pending, self.output_after
        );
    }

    /// カーソル位置を返す（output_before の文字数 + pending の文字数）。
    pub fn cursor_pos(&self) -> usize {
        self.output_before.chars().count() + self.pending.chars().count()
    }

    /// カーソルを1文字左に移動する。pending があれば先に flush する。
    pub fn move_left(&mut self) {
        if !self.pending.is_empty() {
            self.flush();
        }
        if let Some(ch) = self.output_before.pop() {
            self.output_after.insert(0, ch);
        }
        #[cfg(debug_assertions)]
        eprintln!(
            "[InputState::move_left] before='{}'|pending='{}'|after='{}'",
            self.output_before, self.pending, self.output_after
        );
    }

    /// カーソルを1文字右に移動する。pending があれば先に flush する。
    pub fn move_right(&mut self) {
        if !self.pending.is_empty() {
            self.flush();
        }
        if !self.output_after.is_empty() {
            let ch = self.output_after.remove(0);
            self.output_before.push(ch);
        }
        #[cfg(debug_assertions)]
        eprintln!(
            "[InputState::move_right] before='{}'|pending='{}'|after='{}'",
            self.output_before, self.pending, self.output_after
        );
    }

    /// カーソル後ろの1文字を削除する。pending があれば先に flush する。
    pub fn delete(&mut self) {
        if !self.pending.is_empty() {
            self.flush();
        }
        if !self.output_after.is_empty() {
            self.output_after.remove(0);
        }
        #[cfg(debug_assertions)]
        eprintln!(
            "[InputState::delete] before='{}'|pending='{}'|after='{}'",
            self.output_before, self.pending, self.output_after
        );
    }

    /// 出力と pending の両方が空かどうか。
    pub fn is_empty(&self) -> bool {
        self.output_before.is_empty() && self.output_after.is_empty() && self.pending.is_empty()
    }
}

impl Default for InputState {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // === 基本的な逐次入力 ===

    #[test]
    fn feed_single_vowel() {
        let mut state = InputState::new();
        state.feed_char('a');
        assert_eq!(state.output(), "あ");
        assert_eq!(state.pending(), "");
    }

    #[test]
    fn feed_consonant_then_vowel() {
        let mut state = InputState::new();
        state.feed_char('k');
        assert_eq!(state.output(), "");
        assert_eq!(state.pending(), "k");
        state.feed_char('a');
        assert_eq!(state.output(), "か");
        assert_eq!(state.pending(), "");
    }

    #[test]
    fn feed_sequence_aiueo() {
        let mut state = InputState::new();
        for ch in "aiueo".chars() {
            state.feed_char(ch);
        }
        assert_eq!(state.output(), "あいうえお");
        assert_eq!(state.pending(), "");
    }

    // === 促音 ===

    #[test]
    fn feed_sokuon() {
        let mut state = InputState::new();
        for ch in "kakko".chars() {
            state.feed_char(ch);
        }
        assert_eq!(state.output(), "かっこ");
    }

    // === 「ん」処理 ===

    #[test]
    fn feed_nn() {
        let mut state = InputState::new();
        state.feed_char('n');
        state.feed_char('n');
        assert_eq!(state.output(), "ん");
        assert_eq!(state.pending(), "n");
    }

    #[test]
    fn feed_n_before_consonant() {
        let mut state = InputState::new();
        for ch in "kanta".chars() {
            state.feed_char(ch);
        }
        assert_eq!(state.output(), "かんた");
    }

    // === flush ===

    #[test]
    fn flush_trailing_n() {
        let mut state = InputState::new();
        for ch in "kan".chars() {
            state.feed_char(ch);
        }
        assert_eq!(state.output(), "か");
        assert_eq!(state.pending(), "n");
        state.flush();
        assert_eq!(state.output(), "かん");
        assert_eq!(state.pending(), "");
    }

    #[test]
    fn flush_empty_pending() {
        let mut state = InputState::new();
        for ch in "ka".chars() {
            state.feed_char(ch);
        }
        state.flush();
        assert_eq!(state.output(), "か");
        assert_eq!(state.pending(), "");
    }

    // === reset ===

    #[test]
    fn reset_clears_all() {
        let mut state = InputState::new();
        for ch in "ka".chars() {
            state.feed_char(ch);
        }
        state.reset();
        assert_eq!(state.output(), "");
        assert_eq!(state.pending(), "");
    }

    // === convert() との一致確認 ===

    #[test]
    fn matches_batch_convert() {
        let input = "konnichiwa";
        let batch = romaji::convert(input);

        let mut state = InputState::new();
        for ch in input.chars() {
            state.feed_char(ch);
        }
        state.flush();

        // flush 後の output は convert の output + pending を確定した結果と一致する
        assert_eq!(state.output(), batch.output);
    }

    #[test]
    fn matches_batch_convert_toukyou() {
        let input = "toukyou";
        let batch = romaji::convert(input);

        let mut state = InputState::new();
        for ch in input.chars() {
            state.feed_char(ch);
        }
        state.flush();

        assert_eq!(state.output(), batch.output);
    }

    // === backspace ===

    #[test]
    fn backspace_removes_pending() {
        let mut state = InputState::new();
        state.feed_char('k');
        assert_eq!(state.pending(), "k");
        state.backspace();
        assert_eq!(state.pending(), "");
        assert_eq!(state.output(), "");
    }

    #[test]
    fn backspace_removes_output_char() {
        let mut state = InputState::new();
        state.feed_char('k');
        state.feed_char('a');
        assert_eq!(state.output(), "か");
        assert_eq!(state.pending(), "");
        state.backspace();
        assert_eq!(state.output(), "");
        assert_eq!(state.pending(), "");
    }

    #[test]
    fn backspace_on_empty_does_nothing() {
        let mut state = InputState::new();
        state.backspace();
        assert_eq!(state.output(), "");
        assert_eq!(state.pending(), "");
    }

    #[test]
    fn backspace_multi_char_output() {
        let mut state = InputState::new();
        for ch in "ka".chars() {
            state.feed_char(ch);
        }
        for ch in "ki".chars() {
            state.feed_char(ch);
        }
        assert_eq!(state.output(), "かき");
        state.backspace();
        assert_eq!(state.output(), "か");
    }

    // === output_before / output_after / display ===

    #[test]
    fn output_before_equals_output_initially() {
        let mut state = InputState::new();
        for ch in "ka".chars() {
            state.feed_char(ch);
        }
        assert_eq!(state.output_before(), state.output());
    }

    #[test]
    fn output_after_empty_initially() {
        let mut state = InputState::new();
        for ch in "ka".chars() {
            state.feed_char(ch);
        }
        assert_eq!(state.output_after(), "");
    }

    #[test]
    fn display_equals_output_plus_pending() {
        let mut state = InputState::new();
        for ch in "kak".chars() {
            state.feed_char(ch);
        }
        let expected = format!("{}{}", state.output(), state.pending());
        assert_eq!(state.display(), expected);
    }

    // === move_left ===

    #[test]
    fn move_left_moves_one_char() {
        let mut state = InputState::new();
        for ch in "kaki".chars() {
            state.feed_char(ch);
        }
        // output_before="かき", output_after=""
        state.move_left();
        assert_eq!(state.output_before(), "か");
        assert_eq!(state.output_after(), "き");
    }

    #[test]
    fn move_left_at_beginning_does_nothing() {
        let mut state = InputState::new();
        state.move_left();
        assert_eq!(state.output_before(), "");
        assert_eq!(state.output_after(), "");
    }

    #[test]
    fn move_left_flushes_pending() {
        let mut state = InputState::new();
        state.feed_char('k');
        assert_eq!(state.pending(), "k");
        state.move_left();
        // flush: "k" はそのまま確定 → output="k" → move_left → before="", after="k"
        assert_eq!(state.pending(), "");
        assert_eq!(state.output_before(), "");
        assert_eq!(state.output_after(), "k");
    }

    #[test]
    fn move_left_flushes_trailing_n() {
        let mut state = InputState::new();
        for ch in "kan".chars() {
            state.feed_char(ch);
        }
        // output_before="か", pending="n"
        state.move_left();
        // flush "n"→"ん": output="かん" → move_left → before="か", after="ん"
        assert_eq!(state.pending(), "");
        assert_eq!(state.output_before(), "か");
        assert_eq!(state.output_after(), "ん");
    }

    // === move_right ===

    #[test]
    fn move_right_moves_one_char() {
        let mut state = InputState::new();
        for ch in "kaki".chars() {
            state.feed_char(ch);
        }
        state.move_left(); // before="か", after="き"
        state.move_right(); // before="かき", after=""
        assert_eq!(state.output_before(), "かき");
        assert_eq!(state.output_after(), "");
    }

    #[test]
    fn move_right_at_end_does_nothing() {
        let mut state = InputState::new();
        for ch in "kaki".chars() {
            state.feed_char(ch);
        }
        state.move_right();
        assert_eq!(state.output_before(), "かき");
        assert_eq!(state.output_after(), "");
    }

    #[test]
    fn move_right_flushes_pending() {
        let mut state = InputState::new();
        for ch in "kaki".chars() {
            state.feed_char(ch);
        }
        state.move_left(); // before="か", after="き"
        state.feed_char('k'); // before="か", pending="k", after="き"
        state.move_right();
        // flush "k"→"k": output_before="かk", then move "き" → before="かkき", after=""
        assert_eq!(state.pending(), "");
        assert_eq!(state.output_before(), "かkき");
        assert_eq!(state.output_after(), "");
    }

    // === delete ===

    #[test]
    fn delete_removes_char_after_cursor() {
        let mut state = InputState::new();
        for ch in "kaki".chars() {
            state.feed_char(ch);
        }
        state.move_left(); // before="か", after="き"
        state.delete();
        assert_eq!(state.display(), "か");
        assert_eq!(state.output_after(), "");
    }

    #[test]
    fn delete_at_end_does_nothing() {
        let mut state = InputState::new();
        for ch in "kaki".chars() {
            state.feed_char(ch);
        }
        state.delete();
        assert_eq!(state.display(), "かき");
    }

    #[test]
    fn delete_flushes_pending() {
        let mut state = InputState::new();
        for ch in "kaki".chars() {
            state.feed_char(ch);
        }
        state.feed_char('k'); // before="かき", pending="k", after=""
        state.move_left(); // flush → "かきk" → move_left → before="かき", after="k"
        state.delete(); // after の "k" を削除
        assert_eq!(state.pending(), "");
        assert_eq!(state.display(), "かき");
    }

    #[test]
    fn delete_empties_all() {
        let mut state = InputState::new();
        for ch in "ka".chars() {
            state.feed_char(ch);
        }
        state.move_left(); // before="", after="か"
        state.delete();
        assert_eq!(state.display(), "");
        assert!(state.is_empty());
    }

    // === cursor_pos ===

    #[test]
    fn cursor_pos_at_end() {
        let mut state = InputState::new();
        for ch in "kaki".chars() {
            state.feed_char(ch);
        }
        assert_eq!(state.cursor_pos(), 2); // "かき" → 2文字
    }

    #[test]
    fn cursor_pos_after_move_left() {
        let mut state = InputState::new();
        for ch in "kaki".chars() {
            state.feed_char(ch);
        }
        state.move_left();
        assert_eq!(state.cursor_pos(), 1); // "か|き"
    }

    // === カーソル位置での入力・削除 ===

    #[test]
    fn feed_char_inserts_at_cursor() {
        let mut state = InputState::new();
        for ch in "kaki".chars() {
            state.feed_char(ch);
        }
        state.move_left(); // before="か", after="き"
        state.feed_char('a'); // 'a' → "あ" が output_before に追加
        assert_eq!(state.display(), "かあき");
    }

    #[test]
    fn feed_char_with_pending_at_cursor() {
        let mut state = InputState::new();
        for ch in "kaki".chars() {
            state.feed_char(ch);
        }
        state.move_left(); // before="か", after="き"
        state.feed_char('k'); // pending="k"
        assert_eq!(state.display(), "かkき");
        assert_eq!(state.pending(), "k");
    }

    #[test]
    fn backspace_at_cursor_removes_before() {
        let mut state = InputState::new();
        for ch in "kaki".chars() {
            state.feed_char(ch);
        }
        state.move_left(); // before="か", after="き"
        state.backspace();
        assert_eq!(state.display(), "き");
        assert_eq!(state.output_before(), "");
        assert_eq!(state.output_after(), "き");
    }

    // === is_empty ===

    #[test]
    fn is_empty_after_input() {
        let mut state = InputState::new();
        assert!(state.is_empty());
        state.feed_char('k');
        assert!(!state.is_empty());
    }
}
