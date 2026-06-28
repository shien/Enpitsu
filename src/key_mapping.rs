//! 仮想キーコードから EngineCommand への変換。
//!
//! Windows の仮想キーコード (VK_*) と修飾キー状態を受け取り、
//! ConversionEngine に渡す EngineCommand に変換する。
//! プラットフォーム非依存のため、どの OS でもテスト可能。

use crate::engine::EngineCommand;

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
pub const VK_OEM_COMMA: u16 = 0xBC;
pub const VK_OEM_MINUS: u16 = 0xBD;
pub const VK_OEM_PERIOD: u16 = 0xBE;
pub const VK_KANJI: u16 = 0x19;
pub const VK_OEM_3: u16 = 0xC0; // ` (backtick/tilde)

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

/// Ctrl+Space のキー組み合わせかどうかを判定する。
pub fn is_ctrl_space(vk: u16, modifiers: &Modifiers) -> bool {
    vk == VK_SPACE && modifiers.ctrl && !modifiers.shift && !modifiers.alt
}

/// 半角/全角キーかどうかを判定する。
pub fn is_zenkaku_hankaku(vk: u16, modifiers: &Modifiers) -> bool {
    vk == VK_KANJI && !modifiers.ctrl && !modifiers.shift && !modifiers.alt
}

/// Alt+` のキー組み合わせかどうかを判定する。
pub fn is_alt_tilde(vk: u16, modifiers: &Modifiers) -> bool {
    vk == VK_OEM_3 && modifiers.alt && !modifiers.ctrl && !modifiers.shift
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

    // === IME トグルキー検出 ===

    #[test]
    fn ctrl_space_detected() {
        assert!(is_ctrl_space(VK_SPACE, &Modifiers::ctrl()));
    }

    #[test]
    fn space_without_ctrl_is_not_toggle() {
        assert!(!is_ctrl_space(VK_SPACE, &Modifiers::none()));
    }

    #[test]
    fn ctrl_space_with_shift_is_not_toggle() {
        let mods = Modifiers {
            shift: true,
            ctrl: true,
            alt: false,
        };
        assert!(!is_ctrl_space(VK_SPACE, &mods));
    }

    #[test]
    fn ctrl_space_with_alt_is_not_toggle() {
        assert!(!is_ctrl_space(VK_SPACE, &Modifiers::ctrl_alt()));
    }

    #[test]
    fn zenkaku_hankaku_detected() {
        assert!(is_zenkaku_hankaku(VK_KANJI, &Modifiers::none()));
    }

    #[test]
    fn zenkaku_hankaku_with_ctrl_is_not_toggle() {
        assert!(!is_zenkaku_hankaku(VK_KANJI, &Modifiers::ctrl()));
    }

    #[test]
    fn alt_tilde_detected() {
        assert!(is_alt_tilde(VK_OEM_3, &Modifiers::alt()));
    }

    #[test]
    fn tilde_without_alt_is_not_toggle() {
        assert!(!is_alt_tilde(VK_OEM_3, &Modifiers::none()));
    }
}
