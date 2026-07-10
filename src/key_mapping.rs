//! 仮想キーコードから EngineCommand への変換。
//!
//! Windows の仮想キーコード (VK_*) と修飾キー状態を受け取り、
//! ConversionEngine に渡す EngineCommand に変換する。
//! プラットフォーム非依存のため、どの OS でもテスト可能。

use crate::engine::{EngineCommand, EngineState};

// === 仮想キーコード定数 ===

pub const VK_BACK: u16 = 0x08;
pub const VK_RETURN: u16 = 0x0D;
pub const VK_SHIFT: u16 = 0x10;
pub const VK_CONTROL: u16 = 0x11;
pub const VK_MENU: u16 = 0x12; // Alt
pub const VK_ESCAPE: u16 = 0x1B;
pub const VK_SPACE: u16 = 0x20;
pub const VK_LEFT: u16 = 0x25;
pub const VK_UP: u16 = 0x26;
pub const VK_RIGHT: u16 = 0x27;
pub const VK_DOWN: u16 = 0x28;
pub const VK_DELETE: u16 = 0x2E;
pub const VK_0: u16 = 0x30;
pub const VK_9: u16 = 0x39;
pub const VK_A: u16 = 0x41;
pub const VK_B: u16 = 0x42;
pub const VK_D: u16 = 0x44;
pub const VK_F: u16 = 0x46;
pub const VK_G: u16 = 0x47;
pub const VK_H: u16 = 0x48;
pub const VK_J: u16 = 0x4A;
pub const VK_M: u16 = 0x4D;
pub const VK_N: u16 = 0x4E;
pub const VK_P: u16 = 0x50;
pub const VK_Z: u16 = 0x5A;
pub const VK_F1: u16 = 0x70;
pub const VK_F7: u16 = 0x76;
pub const VK_OEM_COMMA: u16 = 0xBC;
pub const VK_OEM_MINUS: u16 = 0xBD;
pub const VK_OEM_PERIOD: u16 = 0xBE;
pub const VK_KANJI: u16 = 0x19;
pub const VK_OEM_3: u16 = 0xC0; // ` (backtick/tilde)
pub const VK_OEM_AUTO: u16 = 0xF3; // 半角/全角 が送ることがある VK
pub const VK_OEM_ENLW: u16 = 0xF4; // 半角/全角 が送ることがある VK

// === TSF 予約キー（PreserveKey）の修飾子ビット ===
// msctf.h の TF_MOD_* と同じ値。プラットフォーム非依存に扱うため定数で持つ。

pub const TF_MOD_ALT: u32 = 0x0001;
pub const TF_MOD_CONTROL: u32 = 0x0002;
pub const TF_MOD_SHIFT: u32 = 0x0004;

/// TSF 予約キー登録に使う仕様（プラットフォーム非依存）。
///
/// `modifiers` は msctf.h の TF_MOD_* と同じビット値を用いる。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PreservedKeySpec {
    pub vk: u16,
    pub modifiers: u32,
}

/// 修飾キーの状態。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Modifiers {
    pub shift: bool,
    pub ctrl: bool,
    pub alt: bool,
}

impl Modifiers {
    pub fn none() -> Self {
        Self {
            shift: false,
            ctrl: false,
            alt: false,
        }
    }

    pub fn shift() -> Self {
        Self {
            shift: true,
            ctrl: false,
            alt: false,
        }
    }

    pub fn ctrl() -> Self {
        Self {
            shift: false,
            ctrl: true,
            alt: false,
        }
    }

    pub fn alt() -> Self {
        Self {
            shift: false,
            ctrl: false,
            alt: true,
        }
    }

    pub fn ctrl_alt() -> Self {
        Self {
            shift: false,
            ctrl: true,
            alt: true,
        }
    }
}

// === キーバインドプリセット ===

/// キーバインドプリセット。
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum KeybindPreset {
    /// Ctrl+キー割り当てなし（デフォルト）
    None,
    /// 競合の少ないキーのみ (Ctrl+J, Ctrl+G, Ctrl+M)
    Minimal,
    /// Emacs フルセット (Ctrl+J/G/N/P/H/M)
    Emacs,
}

/// Ctrl+キーの割り当て設定。
/// 各フィールドが None の場合、そのキーは OS に処理を委ねる。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CtrlKeyConfig {
    pub ctrl_b: Option<EngineCommand>,
    pub ctrl_d: Option<EngineCommand>,
    pub ctrl_f: Option<EngineCommand>,
    pub ctrl_g: Option<EngineCommand>,
    pub ctrl_h: Option<EngineCommand>,
    pub ctrl_j: Option<EngineCommand>,
    pub ctrl_m: Option<EngineCommand>,
    pub ctrl_n: Option<EngineCommand>,
    pub ctrl_p: Option<EngineCommand>,
}

impl CtrlKeyConfig {
    /// プリセットからキーバインド設定を生成する。
    pub fn from_preset(preset: &KeybindPreset) -> Self {
        match preset {
            KeybindPreset::None => Self {
                ctrl_b: None,
                ctrl_d: None,
                ctrl_f: None,
                ctrl_g: None,
                ctrl_h: None,
                ctrl_j: None,
                ctrl_m: None,
                ctrl_n: None,
                ctrl_p: None,
            },
            KeybindPreset::Minimal => Self {
                ctrl_b: None,
                ctrl_d: None,
                ctrl_f: None,
                ctrl_g: Some(EngineCommand::Cancel),
                ctrl_h: None,
                ctrl_j: Some(EngineCommand::Commit),
                ctrl_m: Some(EngineCommand::Commit),
                ctrl_n: None,
                ctrl_p: None,
            },
            KeybindPreset::Emacs => Self {
                ctrl_b: Some(EngineCommand::CursorLeft),
                ctrl_d: Some(EngineCommand::Delete),
                ctrl_f: Some(EngineCommand::CursorRight),
                ctrl_g: Some(EngineCommand::Cancel),
                ctrl_h: Some(EngineCommand::Backspace),
                ctrl_j: Some(EngineCommand::Commit),
                ctrl_m: Some(EngineCommand::Commit),
                ctrl_n: Some(EngineCommand::NextCandidate),
                ctrl_p: Some(EngineCommand::PrevCandidate),
            },
        }
    }
}

impl Default for CtrlKeyConfig {
    fn default() -> Self {
        Self::from_preset(&KeybindPreset::None)
    }
}

/// 仮想キーコードと修飾キー状態を EngineCommand に変換する。
///
/// - `vk`: Windows 仮想キーコード
/// - `modifiers`: 修飾キーの状態
/// - `ime_on`: IME がオンかどうか
/// - `ctrl_config`: Ctrl+キーの割り当て設定
///
/// 戻り値: 対応する EngineCommand。処理しないキーの場合は None。
pub fn map_key(
    vk: u16,
    modifiers: &Modifiers,
    ime_on: bool,
    ctrl_config: &CtrlKeyConfig,
) -> Option<EngineCommand> {
    if !ime_on {
        return None;
    }

    // Alt は常に OS に委ねる
    if modifiers.alt {
        return None;
    }

    // Ctrl+キー: 設定に基づくマッピング（Ctrl+Shift は OS に委ねる）
    if modifiers.ctrl {
        if modifiers.shift {
            return None;
        }
        return map_ctrl_key(vk, ctrl_config);
    }

    match vk {
        VK_A..=VK_Z => {
            let base = (b'a' + (vk - VK_A) as u8) as char;
            let ch = if modifiers.shift {
                base.to_ascii_uppercase()
            } else {
                base
            };
            Some(EngineCommand::InsertChar(ch))
        }
        VK_0..=VK_9 if !modifiers.shift => {
            let ch = (b'0' + (vk - VK_0) as u8) as char;
            Some(EngineCommand::InsertChar(ch))
        }
        VK_SPACE => Some(EngineCommand::Convert),
        VK_F7 => Some(EngineCommand::ConvertKatakana),
        VK_RETURN => Some(EngineCommand::Commit),
        VK_ESCAPE => Some(EngineCommand::Cancel),
        VK_BACK => Some(EngineCommand::Backspace),
        VK_LEFT => Some(EngineCommand::CursorLeft),
        VK_RIGHT => Some(EngineCommand::CursorRight),
        VK_DOWN => Some(EngineCommand::NextCandidate),
        VK_UP => Some(EngineCommand::PrevCandidate),
        VK_DELETE => Some(EngineCommand::Delete),
        VK_OEM_MINUS => Some(EngineCommand::InsertChar('-')),
        VK_OEM_PERIOD => Some(EngineCommand::InsertChar('.')),
        VK_OEM_COMMA => Some(EngineCommand::InsertChar(',')),
        _ => None,
    }
}

/// 文字入力（`InsertChar`）を生成するキーかどうかを判定する。
///
/// Direct 状態で消費すべきキーを判別するために使う。
/// `map_key` の `InsertChar` を返す分岐と同じ条件で判定する。
/// Ctrl / Alt 押下時は文字入力キーではない。
pub fn is_character_key(vk: u16, modifiers: &Modifiers) -> bool {
    if modifiers.ctrl || modifiers.alt {
        return false;
    }
    match vk {
        VK_A..=VK_Z => true,
        VK_0..=VK_9 => !modifiers.shift,
        VK_OEM_MINUS | VK_OEM_PERIOD | VK_OEM_COMMA => true,
        _ => false,
    }
}

/// エンジンの状態から、マップ済みキーを IME が消費すべきか判定する。
///
/// IME がキーを消費（`OnTestKeyDown`/`OnKeyDown` で `TRUE` を返す）すると、
/// そのキーはアプリケーションに渡らない。各状態で「実行しても何も起きない」
/// コマンドは消費せず、アプリに委ねることでカーソル移動・改行・空白入力などの
/// 通常操作を妨げないようにする。
///
/// - Direct: 文字入力キーのみ消費する。矢印・Backspace・Enter・Space・
///   Escape・Delete はアプリに渡す (Bug 1)。
/// - Composing: 候補ナビゲーション (`NextCandidate`/`PrevCandidate`) は候補が
///   存在しないため消費しない。上下矢印はアプリに渡す (Bug 3)。
/// - Converting: カーソル移動 (`CursorLeft`/`CursorRight`) と `Delete` は
///   消費しない。アプリに渡す (Bug 4)。
pub fn should_consume_key(
    state: EngineState,
    vk: u16,
    modifiers: &Modifiers,
    command: &Option<EngineCommand>,
) -> bool {
    let Some(command) = command else {
        return false;
    };
    match state {
        // Direct: 文字入力キーのみ消費し、それ以外はアプリに委ねる
        EngineState::Direct => is_character_key(vk, modifiers),
        // Composing: 候補ナビは消費しない（候補が無いため無意味）
        EngineState::Composing => !matches!(
            command,
            EngineCommand::NextCandidate | EngineCommand::PrevCandidate
        ),
        // Converting: カーソル移動・Delete は消費しない
        EngineState::Converting => !matches!(
            command,
            EngineCommand::CursorLeft | EngineCommand::CursorRight | EngineCommand::Delete
        ),
    }
}

/// 消費しない（アプリに渡す）キーを渡す前に、未確定入力を確定すべきか判定する。
///
/// `should_consume_key` が `false`（アプリに委ねる）を返すキーのうち、
/// Composition がアクティブな状態でカーソル移動・削除・候補ナビをアプリに渡すと、
/// IME が保持する Composition とアプリ側のキャレット位置がずれる。その状態で
/// 次の消費キーが来ると、古い候補が誤った位置に確定されてしまう。
///
/// これを防ぐため、以下のキーはアプリに渡す前に現在の入力を確定する:
/// - Composing で上下矢印（`NextCandidate`/`PrevCandidate`）
/// - Converting でカーソル移動（`CursorLeft`/`CursorRight`）・`Delete`
///
/// Direct 状態は Composition が無いため確定不要。マップされないキー
/// （修飾キー単体・ファンクションキー等）も確定しない。
pub fn should_commit_before_passthrough(
    state: EngineState,
    command: &Option<EngineCommand>,
) -> bool {
    let Some(command) = command else {
        return false;
    };
    matches!(
        (state, command),
        (EngineState::Composing, EngineCommand::NextCandidate)
            | (EngineState::Composing, EngineCommand::PrevCandidate)
            | (EngineState::Converting, EngineCommand::CursorLeft)
            | (EngineState::Converting, EngineCommand::CursorRight)
            | (EngineState::Converting, EngineCommand::Delete)
    )
}

/// Ctrl+キーを設定に基づいて EngineCommand に変換する。
fn map_ctrl_key(vk: u16, config: &CtrlKeyConfig) -> Option<EngineCommand> {
    match vk {
        VK_B => config.ctrl_b.clone(),
        VK_D => config.ctrl_d.clone(),
        VK_F => config.ctrl_f.clone(),
        VK_G => config.ctrl_g.clone(),
        VK_H => config.ctrl_h.clone(),
        VK_J => config.ctrl_j.clone(),
        VK_M => config.ctrl_m.clone(),
        VK_N => config.ctrl_n.clone(),
        VK_P => config.ctrl_p.clone(),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::engine::EngineCommand;

    // === プリセット ===

    #[test]
    fn preset_none_all_disabled() {
        let config = CtrlKeyConfig::from_preset(&KeybindPreset::None);
        assert_eq!(config.ctrl_j, None);
        assert_eq!(config.ctrl_g, None);
        assert_eq!(config.ctrl_n, None);
        assert_eq!(config.ctrl_p, None);
        assert_eq!(config.ctrl_h, None);
        assert_eq!(config.ctrl_m, None);
    }

    #[test]
    fn preset_minimal_only_safe_keys() {
        let config = CtrlKeyConfig::from_preset(&KeybindPreset::Minimal);
        assert_eq!(config.ctrl_j, Some(EngineCommand::Commit));
        assert_eq!(config.ctrl_g, Some(EngineCommand::Cancel));
        assert_eq!(config.ctrl_m, Some(EngineCommand::Commit));
        // 競合しやすいキーは無効
        assert_eq!(config.ctrl_n, None);
        assert_eq!(config.ctrl_p, None);
        assert_eq!(config.ctrl_h, None);
    }

    #[test]
    fn preset_emacs_all_enabled() {
        let config = CtrlKeyConfig::from_preset(&KeybindPreset::Emacs);
        assert_eq!(config.ctrl_j, Some(EngineCommand::Commit));
        assert_eq!(config.ctrl_g, Some(EngineCommand::Cancel));
        assert_eq!(config.ctrl_n, Some(EngineCommand::NextCandidate));
        assert_eq!(config.ctrl_p, Some(EngineCommand::PrevCandidate));
        assert_eq!(config.ctrl_h, Some(EngineCommand::Backspace));
        assert_eq!(config.ctrl_m, Some(EngineCommand::Commit));
    }

    #[test]
    fn default_is_none_preset() {
        let default = CtrlKeyConfig::default();
        let none = CtrlKeyConfig::from_preset(&KeybindPreset::None);
        assert_eq!(default, none);
    }

    // === map_key: Emacs プリセット ===

    #[test]
    fn emacs_ctrl_j_commits() {
        let config = CtrlKeyConfig::from_preset(&KeybindPreset::Emacs);
        let cmd = map_key(VK_J, &Modifiers::ctrl(), true, &config);
        assert_eq!(cmd, Some(EngineCommand::Commit));
    }

    #[test]
    fn emacs_ctrl_g_cancels() {
        let config = CtrlKeyConfig::from_preset(&KeybindPreset::Emacs);
        let cmd = map_key(VK_G, &Modifiers::ctrl(), true, &config);
        assert_eq!(cmd, Some(EngineCommand::Cancel));
    }

    #[test]
    fn emacs_ctrl_n_next_candidate() {
        let config = CtrlKeyConfig::from_preset(&KeybindPreset::Emacs);
        let cmd = map_key(VK_N, &Modifiers::ctrl(), true, &config);
        assert_eq!(cmd, Some(EngineCommand::NextCandidate));
    }

    #[test]
    fn emacs_ctrl_p_prev_candidate() {
        let config = CtrlKeyConfig::from_preset(&KeybindPreset::Emacs);
        let cmd = map_key(VK_P, &Modifiers::ctrl(), true, &config);
        assert_eq!(cmd, Some(EngineCommand::PrevCandidate));
    }

    #[test]
    fn emacs_ctrl_h_backspace() {
        let config = CtrlKeyConfig::from_preset(&KeybindPreset::Emacs);
        let cmd = map_key(VK_H, &Modifiers::ctrl(), true, &config);
        assert_eq!(cmd, Some(EngineCommand::Backspace));
    }

    #[test]
    fn emacs_ctrl_m_commits() {
        let config = CtrlKeyConfig::from_preset(&KeybindPreset::Emacs);
        let cmd = map_key(VK_M, &Modifiers::ctrl(), true, &config);
        assert_eq!(cmd, Some(EngineCommand::Commit));
    }

    // === map_key: Minimal プリセット ===

    #[test]
    fn minimal_ctrl_j_commits() {
        let config = CtrlKeyConfig::from_preset(&KeybindPreset::Minimal);
        let cmd = map_key(VK_J, &Modifiers::ctrl(), true, &config);
        assert_eq!(cmd, Some(EngineCommand::Commit));
    }

    #[test]
    fn minimal_ctrl_n_returns_none() {
        let config = CtrlKeyConfig::from_preset(&KeybindPreset::Minimal);
        let cmd = map_key(VK_N, &Modifiers::ctrl(), true, &config);
        assert_eq!(cmd, None);
    }

    #[test]
    fn minimal_ctrl_p_returns_none() {
        let config = CtrlKeyConfig::from_preset(&KeybindPreset::Minimal);
        let cmd = map_key(VK_P, &Modifiers::ctrl(), true, &config);
        assert_eq!(cmd, None);
    }

    #[test]
    fn minimal_ctrl_h_returns_none() {
        let config = CtrlKeyConfig::from_preset(&KeybindPreset::Minimal);
        let cmd = map_key(VK_H, &Modifiers::ctrl(), true, &config);
        assert_eq!(cmd, None);
    }

    // === map_key: None プリセット ===

    #[test]
    fn none_preset_ctrl_j_returns_none() {
        let config = CtrlKeyConfig::from_preset(&KeybindPreset::None);
        let cmd = map_key(VK_J, &Modifiers::ctrl(), true, &config);
        assert_eq!(cmd, None);
    }

    // === map_key: 共通 ===

    #[test]
    fn ctrl_other_returns_none() {
        let config = CtrlKeyConfig::from_preset(&KeybindPreset::Emacs);
        let cmd = map_key(VK_A, &Modifiers::ctrl(), true, &config);
        assert_eq!(cmd, None);
    }

    #[test]
    fn ctrl_alt_returns_none() {
        let config = CtrlKeyConfig::from_preset(&KeybindPreset::Emacs);
        let cmd = map_key(VK_J, &Modifiers::ctrl_alt(), true, &config);
        assert_eq!(cmd, None);
    }

    // === カスタム設定（プリセット + 個別上書き） ===

    #[test]
    fn emacs_override_ctrl_n_none() {
        let mut config = CtrlKeyConfig::from_preset(&KeybindPreset::Emacs);
        config.ctrl_n = None;
        let cmd = map_key(VK_N, &Modifiers::ctrl(), true, &config);
        assert_eq!(cmd, None);
        // 他のキーは影響なし
        let cmd = map_key(VK_J, &Modifiers::ctrl(), true, &config);
        assert_eq!(cmd, Some(EngineCommand::Commit));
    }

    #[test]
    fn none_override_ctrl_j_commit() {
        let mut config = CtrlKeyConfig::from_preset(&KeybindPreset::None);
        config.ctrl_j = Some(EngineCommand::Commit);
        let cmd = map_key(VK_J, &Modifiers::ctrl(), true, &config);
        assert_eq!(cmd, Some(EngineCommand::Commit));
        // 他は引き続き無効
        let cmd = map_key(VK_G, &Modifiers::ctrl(), true, &config);
        assert_eq!(cmd, None);
    }

    #[test]
    fn minimal_override_ctrl_h_backspace() {
        let mut config = CtrlKeyConfig::from_preset(&KeybindPreset::Minimal);
        config.ctrl_h = Some(EngineCommand::Backspace);
        let cmd = map_key(VK_H, &Modifiers::ctrl(), true, &config);
        assert_eq!(cmd, Some(EngineCommand::Backspace));
    }

    // === アルファベットキー ===

    #[test]
    fn alphabet_key_lowercase() {
        let config = CtrlKeyConfig::default();
        let cmd = map_key(VK_A, &Modifiers::none(), true, &config);
        assert_eq!(cmd, Some(EngineCommand::InsertChar('a')));
    }

    #[test]
    fn alphabet_key_all_letters() {
        let config = CtrlKeyConfig::default();
        for vk in VK_A..=VK_Z {
            let cmd = map_key(vk, &Modifiers::none(), true, &config);
            let expected_char = (b'a' + (vk - VK_A) as u8) as char;
            assert_eq!(cmd, Some(EngineCommand::InsertChar(expected_char)));
        }
    }

    // === 特殊キー ===

    #[test]
    fn space_key_converts() {
        let config = CtrlKeyConfig::default();
        let cmd = map_key(VK_SPACE, &Modifiers::none(), true, &config);
        assert_eq!(cmd, Some(EngineCommand::Convert));
    }

    #[test]
    fn enter_key_commits() {
        let config = CtrlKeyConfig::default();
        let cmd = map_key(VK_RETURN, &Modifiers::none(), true, &config);
        assert_eq!(cmd, Some(EngineCommand::Commit));
    }

    #[test]
    fn escape_key_cancels() {
        let config = CtrlKeyConfig::default();
        let cmd = map_key(VK_ESCAPE, &Modifiers::none(), true, &config);
        assert_eq!(cmd, Some(EngineCommand::Cancel));
    }

    #[test]
    fn f7_key_converts_katakana() {
        let config = CtrlKeyConfig::default();
        let cmd = map_key(VK_F7, &Modifiers::none(), true, &config);
        assert_eq!(cmd, Some(EngineCommand::ConvertKatakana));
    }

    #[test]
    fn f7_key_ime_off_returns_none() {
        let config = CtrlKeyConfig::default();
        let cmd = map_key(VK_F7, &Modifiers::none(), false, &config);
        assert_eq!(cmd, None);
    }

    #[test]
    fn backspace_key() {
        let config = CtrlKeyConfig::default();
        let cmd = map_key(VK_BACK, &Modifiers::none(), true, &config);
        assert_eq!(cmd, Some(EngineCommand::Backspace));
    }

    #[test]
    fn down_arrow_next_candidate() {
        let config = CtrlKeyConfig::default();
        let cmd = map_key(VK_DOWN, &Modifiers::none(), true, &config);
        assert_eq!(cmd, Some(EngineCommand::NextCandidate));
    }

    #[test]
    fn up_arrow_prev_candidate() {
        let config = CtrlKeyConfig::default();
        let cmd = map_key(VK_UP, &Modifiers::none(), true, &config);
        assert_eq!(cmd, Some(EngineCommand::PrevCandidate));
    }

    // === IME オフ ===

    #[test]
    fn ime_off_returns_none() {
        let config = CtrlKeyConfig::default();
        let cmd = map_key(VK_A, &Modifiers::none(), false, &config);
        assert_eq!(cmd, None);
    }

    #[test]
    fn ime_off_space_returns_none() {
        let config = CtrlKeyConfig::default();
        let cmd = map_key(VK_SPACE, &Modifiers::none(), false, &config);
        assert_eq!(cmd, None);
    }

    // === 修飾キー ===

    #[test]
    fn ctrl_key_with_default_returns_none() {
        // デフォルト (None プリセット) では Ctrl+A は None
        let config = CtrlKeyConfig::default();
        let cmd = map_key(VK_A, &Modifiers::ctrl(), true, &config);
        assert_eq!(cmd, None);
    }

    #[test]
    fn alt_key_returns_none() {
        let config = CtrlKeyConfig::default();
        let cmd = map_key(VK_A, &Modifiers::alt(), true, &config);
        assert_eq!(cmd, None);
    }

    #[test]
    fn shift_alphabet_uppercase() {
        let config = CtrlKeyConfig::default();
        let cmd = map_key(VK_A, &Modifiers::shift(), true, &config);
        assert_eq!(cmd, Some(EngineCommand::InsertChar('A')));
    }

    // === Ctrl+Shift は OS に委ねる ===

    #[test]
    fn ctrl_shift_n_returns_none() {
        // Ctrl+Shift+N はブラウザの新規シークレットウィンドウ等で使われる
        let config = CtrlKeyConfig::from_preset(&KeybindPreset::Emacs);
        let mods = Modifiers {
            shift: true,
            ctrl: true,
            alt: false,
        };
        let cmd = map_key(VK_N, &mods, true, &config);
        assert_eq!(cmd, None);
    }

    #[test]
    fn ctrl_shift_j_returns_none() {
        // Ctrl+Shift+J はブラウザの DevTools 等で使われる
        let config = CtrlKeyConfig::from_preset(&KeybindPreset::Emacs);
        let mods = Modifiers {
            shift: true,
            ctrl: true,
            alt: false,
        };
        let cmd = map_key(VK_J, &mods, true, &config);
        assert_eq!(cmd, None);
    }

    #[test]
    fn ctrl_shift_p_returns_none() {
        let config = CtrlKeyConfig::from_preset(&KeybindPreset::Emacs);
        let mods = Modifiers {
            shift: true,
            ctrl: true,
            alt: false,
        };
        let cmd = map_key(VK_P, &mods, true, &config);
        assert_eq!(cmd, None);
    }

    // === 処理しないキー ===

    #[test]
    fn function_keys_return_none() {
        let config = CtrlKeyConfig::default();
        let cmd = map_key(VK_F1, &Modifiers::none(), true, &config);
        assert_eq!(cmd, None);
    }

    // === 数字キー ===

    #[test]
    fn number_key_0() {
        let config = CtrlKeyConfig::default();
        let cmd = map_key(VK_0, &Modifiers::none(), true, &config);
        assert_eq!(cmd, Some(EngineCommand::InsertChar('0')));
    }

    #[test]
    fn number_key_9() {
        let config = CtrlKeyConfig::default();
        let cmd = map_key(VK_9, &Modifiers::none(), true, &config);
        assert_eq!(cmd, Some(EngineCommand::InsertChar('9')));
    }

    #[test]
    fn number_keys_all() {
        let config = CtrlKeyConfig::default();
        for vk in VK_0..=VK_9 {
            let cmd = map_key(vk, &Modifiers::none(), true, &config);
            let expected_char = (b'0' + (vk - VK_0) as u8) as char;
            assert_eq!(cmd, Some(EngineCommand::InsertChar(expected_char)));
        }
    }

    #[test]
    fn number_key_with_shift_returns_none() {
        // Shift+数字はシステムに処理を委ねる（! @ # 等）
        let config = CtrlKeyConfig::default();
        let cmd = map_key(VK_0, &Modifiers::shift(), true, &config);
        assert_eq!(cmd, None);
    }

    // === 句読点 ===

    #[test]
    fn minus_key_inserts_minus() {
        let config = CtrlKeyConfig::default();
        let cmd = map_key(VK_OEM_MINUS, &Modifiers::none(), true, &config);
        assert_eq!(cmd, Some(EngineCommand::InsertChar('-')));
    }

    #[test]
    fn period_key_inserts_period() {
        let config = CtrlKeyConfig::default();
        let cmd = map_key(VK_OEM_PERIOD, &Modifiers::none(), true, &config);
        assert_eq!(cmd, Some(EngineCommand::InsertChar('.')));
    }

    #[test]
    fn comma_key_inserts_comma() {
        let config = CtrlKeyConfig::default();
        let cmd = map_key(VK_OEM_COMMA, &Modifiers::none(), true, &config);
        assert_eq!(cmd, Some(EngineCommand::InsertChar(',')));
    }

    // === カーソル移動・Delete ===

    #[test]
    fn left_arrow_cursor_left() {
        let config = CtrlKeyConfig::default();
        let cmd = map_key(VK_LEFT, &Modifiers::none(), true, &config);
        assert_eq!(cmd, Some(EngineCommand::CursorLeft));
    }

    #[test]
    fn right_arrow_cursor_right() {
        let config = CtrlKeyConfig::default();
        let cmd = map_key(VK_RIGHT, &Modifiers::none(), true, &config);
        assert_eq!(cmd, Some(EngineCommand::CursorRight));
    }

    #[test]
    fn delete_key_delete() {
        let config = CtrlKeyConfig::default();
        let cmd = map_key(VK_DELETE, &Modifiers::none(), true, &config);
        assert_eq!(cmd, Some(EngineCommand::Delete));
    }

    #[test]
    fn emacs_ctrl_b_cursor_left() {
        let config = CtrlKeyConfig::from_preset(&KeybindPreset::Emacs);
        let cmd = map_key(VK_B, &Modifiers::ctrl(), true, &config);
        assert_eq!(cmd, Some(EngineCommand::CursorLeft));
    }

    #[test]
    fn emacs_ctrl_f_cursor_right() {
        let config = CtrlKeyConfig::from_preset(&KeybindPreset::Emacs);
        let cmd = map_key(VK_F, &Modifiers::ctrl(), true, &config);
        assert_eq!(cmd, Some(EngineCommand::CursorRight));
    }

    #[test]
    fn emacs_ctrl_d_delete() {
        let config = CtrlKeyConfig::from_preset(&KeybindPreset::Emacs);
        let cmd = map_key(VK_D, &Modifiers::ctrl(), true, &config);
        assert_eq!(cmd, Some(EngineCommand::Delete));
    }

    #[test]
    fn minimal_ctrl_b_returns_none() {
        let config = CtrlKeyConfig::from_preset(&KeybindPreset::Minimal);
        let cmd = map_key(VK_B, &Modifiers::ctrl(), true, &config);
        assert_eq!(cmd, None);
    }

    #[test]
    fn minimal_ctrl_f_returns_none() {
        let config = CtrlKeyConfig::from_preset(&KeybindPreset::Minimal);
        let cmd = map_key(VK_F, &Modifiers::ctrl(), true, &config);
        assert_eq!(cmd, None);
    }

    #[test]
    fn minimal_ctrl_d_returns_none() {
        let config = CtrlKeyConfig::from_preset(&KeybindPreset::Minimal);
        let cmd = map_key(VK_D, &Modifiers::ctrl(), true, &config);
        assert_eq!(cmd, None);
    }

    // === is_character_key ===

    #[test]
    fn is_character_key_alphabet() {
        assert!(is_character_key(VK_A, &Modifiers::none()));
        assert!(is_character_key(VK_Z, &Modifiers::none()));
        // Shift+アルファベットも文字入力キー（大文字）
        assert!(is_character_key(VK_A, &Modifiers::shift()));
    }

    #[test]
    fn is_character_key_digits() {
        assert!(is_character_key(VK_0, &Modifiers::none()));
        assert!(is_character_key(VK_9, &Modifiers::none()));
        // Shift+数字は記号なので文字入力キーではない
        assert!(!is_character_key(VK_0, &Modifiers::shift()));
    }

    #[test]
    fn is_character_key_punctuation() {
        assert!(is_character_key(VK_OEM_MINUS, &Modifiers::none()));
        assert!(is_character_key(VK_OEM_PERIOD, &Modifiers::none()));
        assert!(is_character_key(VK_OEM_COMMA, &Modifiers::none()));
    }

    #[test]
    fn is_character_key_non_character_keys() {
        assert!(!is_character_key(VK_SPACE, &Modifiers::none()));
        assert!(!is_character_key(VK_RETURN, &Modifiers::none()));
        assert!(!is_character_key(VK_BACK, &Modifiers::none()));
        assert!(!is_character_key(VK_LEFT, &Modifiers::none()));
        assert!(!is_character_key(VK_RIGHT, &Modifiers::none()));
        assert!(!is_character_key(VK_UP, &Modifiers::none()));
        assert!(!is_character_key(VK_DOWN, &Modifiers::none()));
        assert!(!is_character_key(VK_ESCAPE, &Modifiers::none()));
        assert!(!is_character_key(VK_DELETE, &Modifiers::none()));
    }

    #[test]
    fn is_character_key_with_ctrl_or_alt() {
        // Ctrl / Alt 押下時は文字入力キーではない
        assert!(!is_character_key(VK_A, &Modifiers::ctrl()));
        assert!(!is_character_key(VK_A, &Modifiers::alt()));
    }

    // IME トグルキーの検出は ToggleKey::matches に一本化した（config.rs のテスト参照）。

    // === should_consume_key: 状態に応じたキー消費判定 ===

    use EngineState::*;

    // マップされないキー（None）は消費しない
    #[test]
    fn consume_none_command_is_false() {
        assert!(!should_consume_key(
            Direct,
            VK_F1,
            &Modifiers::none(),
            &None
        ));
        assert!(!should_consume_key(
            Composing,
            VK_F1,
            &Modifiers::none(),
            &None
        ));
        assert!(!should_consume_key(
            Converting,
            VK_F1,
            &Modifiers::none(),
            &None
        ));
    }

    // Direct: 文字入力キーのみ消費 (Bug 1)
    #[test]
    fn consume_direct_character_key_is_true() {
        assert!(should_consume_key(
            Direct,
            VK_A,
            &Modifiers::none(),
            &Some(EngineCommand::InsertChar('a'))
        ));
    }

    #[test]
    fn consume_direct_arrow_is_false() {
        assert!(!should_consume_key(
            Direct,
            VK_LEFT,
            &Modifiers::none(),
            &Some(EngineCommand::CursorLeft)
        ));
    }

    #[test]
    fn consume_direct_enter_is_false() {
        assert!(!should_consume_key(
            Direct,
            VK_RETURN,
            &Modifiers::none(),
            &Some(EngineCommand::Commit)
        ));
    }

    // Composing: 上下矢印（候補ナビ）は消費しない (Bug 3)
    #[test]
    fn consume_composing_next_candidate_is_false() {
        assert!(!should_consume_key(
            Composing,
            VK_DOWN,
            &Modifiers::none(),
            &Some(EngineCommand::NextCandidate)
        ));
    }

    #[test]
    fn consume_composing_prev_candidate_is_false() {
        assert!(!should_consume_key(
            Composing,
            VK_UP,
            &Modifiers::none(),
            &Some(EngineCommand::PrevCandidate)
        ));
    }

    #[test]
    fn consume_composing_cursor_and_char_is_true() {
        assert!(should_consume_key(
            Composing,
            VK_LEFT,
            &Modifiers::none(),
            &Some(EngineCommand::CursorLeft)
        ));
        assert!(should_consume_key(
            Composing,
            VK_A,
            &Modifiers::none(),
            &Some(EngineCommand::InsertChar('a'))
        ));
    }

    // Converting: カーソル移動・Delete は消費しない (Bug 4)
    #[test]
    fn consume_converting_cursor_left_is_false() {
        assert!(!should_consume_key(
            Converting,
            VK_LEFT,
            &Modifiers::none(),
            &Some(EngineCommand::CursorLeft)
        ));
    }

    #[test]
    fn consume_converting_cursor_right_is_false() {
        assert!(!should_consume_key(
            Converting,
            VK_RIGHT,
            &Modifiers::none(),
            &Some(EngineCommand::CursorRight)
        ));
    }

    #[test]
    fn consume_converting_delete_is_false() {
        assert!(!should_consume_key(
            Converting,
            VK_DELETE,
            &Modifiers::none(),
            &Some(EngineCommand::Delete)
        ));
    }

    #[test]
    fn consume_converting_candidate_nav_is_true() {
        assert!(should_consume_key(
            Converting,
            VK_DOWN,
            &Modifiers::none(),
            &Some(EngineCommand::NextCandidate)
        ));
        assert!(should_consume_key(
            Converting,
            VK_RETURN,
            &Modifiers::none(),
            &Some(EngineCommand::Commit)
        ));
    }

    // === should_commit_before_passthrough: 渡す前に確定すべきか ===

    #[test]
    fn commit_before_passthrough_none_is_false() {
        assert!(!should_commit_before_passthrough(Direct, &None));
        assert!(!should_commit_before_passthrough(Composing, &None));
        assert!(!should_commit_before_passthrough(Converting, &None));
    }

    #[test]
    fn commit_before_passthrough_converting_cursor_and_delete() {
        assert!(should_commit_before_passthrough(
            Converting,
            &Some(EngineCommand::CursorLeft)
        ));
        assert!(should_commit_before_passthrough(
            Converting,
            &Some(EngineCommand::CursorRight)
        ));
        assert!(should_commit_before_passthrough(
            Converting,
            &Some(EngineCommand::Delete)
        ));
    }

    #[test]
    fn commit_before_passthrough_composing_candidate_nav() {
        assert!(should_commit_before_passthrough(
            Composing,
            &Some(EngineCommand::NextCandidate)
        ));
        assert!(should_commit_before_passthrough(
            Composing,
            &Some(EngineCommand::PrevCandidate)
        ));
    }

    #[test]
    fn commit_before_passthrough_consumed_keys_are_false() {
        // 消費するキーや Direct（Composition 無し）は確定対象外
        assert!(!should_commit_before_passthrough(
            Converting,
            &Some(EngineCommand::NextCandidate)
        ));
        assert!(!should_commit_before_passthrough(
            Composing,
            &Some(EngineCommand::CursorLeft)
        ));
        assert!(!should_commit_before_passthrough(
            Direct,
            &Some(EngineCommand::CursorLeft)
        ));
    }

    // 不変条件: 確定対象なら必ず非消費キーである
    #[test]
    fn commit_before_passthrough_implies_not_consumed() {
        // 確定対象となる (状態, コマンド, vk) の組
        let cases = [
            (Composing, EngineCommand::NextCandidate, VK_DOWN),
            (Composing, EngineCommand::PrevCandidate, VK_UP),
            (Converting, EngineCommand::CursorLeft, VK_LEFT),
            (Converting, EngineCommand::CursorRight, VK_RIGHT),
            (Converting, EngineCommand::Delete, VK_DELETE),
        ];
        for (state, command, vk) in cases {
            let cmd = Some(command.clone());
            assert!(should_commit_before_passthrough(state, &cmd));
            assert!(
                !should_consume_key(state, vk, &Modifiers::none(), &cmd),
                "確定対象 {:?}/{:?} は非消費キーであるべき",
                state,
                command
            );
        }
    }
}
