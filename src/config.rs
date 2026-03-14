//! 設定ファイルのパースとデフォルト値。
//!
//! `%APPDATA%\enpitsu\config.toml` から設定を読み込む。
//! 簡易 TOML パーサー（key = value 形式のサブセット）。

use std::path::Path;

use crate::engine::EngineCommand;
use crate::key_mapping::{CtrlKeyConfig, KeybindPreset};

/// 設定エラー。
#[derive(Debug)]
pub enum ConfigError {
    /// ファイル I/O エラー。
    Io(std::io::Error),
    /// パースエラー。
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
    pub keybind_preset: KeybindPreset,
    pub keybind: CtrlKeyConfig,
}

impl Config {
    /// デフォルト設定を返す。
    pub fn default_config() -> Self {
        Self {
            toggle_key: ToggleKey::ZenkakuHankaku,
            system_dict_path: None,
            auto_learn: true,
            keybind_preset: KeybindPreset::None,
            keybind: CtrlKeyConfig::default(),
        }
    }

    /// TOML ファイルから設定を読み込む。ファイルが存在しない場合はデフォルト設定を返す。
    pub fn load(path: &Path) -> Result<Self, ConfigError> {
        if !path.exists() {
            return Ok(Self::default_config());
        }
        let text = std::fs::read_to_string(path)?;
        Self::parse(&text)
    }

    /// TOML テキストから設定をパースする。
    pub fn parse(text: &str) -> Result<Self, ConfigError> {
        let mut config = Self::default_config();
        let mut keybind_overrides: Vec<(&str, &str)> = Vec::new();

        for line in text.lines() {
            let line = line.trim();
            if line.is_empty() || line.starts_with('#') || line.starts_with('[') {
                continue;
            }
            let Some((key, value)) = line.split_once('=') else {
                continue;
            };
            let key = key.trim();
            let value = value.trim().trim_matches('"');
            match key {
                "toggle_key" => {
                    config.toggle_key = parse_toggle_key(value)?;
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
                "keybind_preset" => {
                    config.keybind_preset = parse_preset(value)?;
                }
                k if k.starts_with("ctrl_") => {
                    keybind_overrides.push((k, value));
                }
                _ => {} // 未知のキーは無視
            }
        }

        // プリセットからベースを生成し、個別の上書きを適用
        config.keybind = CtrlKeyConfig::from_preset(&config.keybind_preset);
        for (key, value) in keybind_overrides {
            let cmd = parse_command(value)?;
            match key {
                "ctrl_b" => config.keybind.ctrl_b = cmd,
                "ctrl_d" => config.keybind.ctrl_d = cmd,
                "ctrl_f" => config.keybind.ctrl_f = cmd,
                "ctrl_g" => config.keybind.ctrl_g = cmd,
                "ctrl_h" => config.keybind.ctrl_h = cmd,
                "ctrl_j" => config.keybind.ctrl_j = cmd,
                "ctrl_m" => config.keybind.ctrl_m = cmd,
                "ctrl_n" => config.keybind.ctrl_n = cmd,
                "ctrl_p" => config.keybind.ctrl_p = cmd,
                _ => {} // 未知の ctrl_* は無視
            }
        }

        Ok(config)
    }

    /// デフォルト設定ファイルの内容を生成する。
    pub fn default_toml() -> String {
        r#"# Enpitsu 設定ファイル

[general]
# 入力モード切り替えキー: "zenkaku-hankaku" | "ctrl-space" | "alt-tilde"
toggle_key = "zenkaku-hankaku"
# キーバインドプリセット: "none" | "minimal" | "emacs"
keybind_preset = "none"

[dictionary]
# システム辞書パス（空の場合は DLL と同じディレクトリの dict/ を使用）
system_dict_path = ""

[behavior]
# 候補選択後に自動的に学習するか
auto_learn = true

# [keybind]
# プリセットをベースに個別のキーを上書きする。
# 値: commit, cancel, next, prev, backspace, convert, none
# ctrl_j = "commit"
# ctrl_g = "cancel"
"#
        .to_string()
    }
}

fn parse_toggle_key(value: &str) -> Result<ToggleKey, ConfigError> {
    match value {
        "zenkaku-hankaku" => Ok(ToggleKey::ZenkakuHankaku),
        "ctrl-space" => Ok(ToggleKey::CtrlSpace),
        "alt-tilde" => Ok(ToggleKey::AltTilde),
        _ => Err(ConfigError::Parse(format!("不明な toggle_key: {value}"))),
    }
}

fn parse_preset(value: &str) -> Result<KeybindPreset, ConfigError> {
    match value {
        "none" => Ok(KeybindPreset::None),
        "minimal" => Ok(KeybindPreset::Minimal),
        "emacs" => Ok(KeybindPreset::Emacs),
        _ => Err(ConfigError::Parse(format!(
            "不正なプリセット名: {value} (none, minimal, emacs のいずれか)"
        ))),
    }
}

fn parse_command(value: &str) -> Result<Option<EngineCommand>, ConfigError> {
    match value {
        "commit" => Ok(Some(EngineCommand::Commit)),
        "cancel" => Ok(Some(EngineCommand::Cancel)),
        "next" => Ok(Some(EngineCommand::NextCandidate)),
        "prev" => Ok(Some(EngineCommand::PrevCandidate)),
        "backspace" => Ok(Some(EngineCommand::Backspace)),
        "convert" => Ok(Some(EngineCommand::Convert)),
        "cursor_left" => Ok(Some(EngineCommand::CursorLeft)),
        "cursor_right" => Ok(Some(EngineCommand::CursorRight)),
        "delete" => Ok(Some(EngineCommand::Delete)),
        "none" => Ok(None),
        _ => Err(ConfigError::Parse(format!("不正なコマンド名: {value}"))),
    }
}

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
        assert_eq!(config.keybind_preset, KeybindPreset::None);
        assert_eq!(config.keybind, CtrlKeyConfig::default());
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

        let _ = std::fs::remove_file(&path);
    }

    // === default_toml ===

    #[test]
    fn default_toml_is_parsable() {
        let toml = Config::default_toml();
        let config = Config::parse(&toml).unwrap();
        assert_eq!(config, Config::default_config());
    }

    // === keybind_preset パース ===

    #[test]
    fn parse_preset_emacs() {
        let toml = r#"
[general]
keybind_preset = "emacs"
"#;
        let config = Config::parse(toml).unwrap();
        assert_eq!(config.keybind_preset, KeybindPreset::Emacs);
        assert_eq!(
            config.keybind,
            CtrlKeyConfig::from_preset(&KeybindPreset::Emacs)
        );
    }

    #[test]
    fn parse_preset_minimal() {
        let toml = r#"
[general]
keybind_preset = "minimal"
"#;
        let config = Config::parse(toml).unwrap();
        assert_eq!(config.keybind_preset, KeybindPreset::Minimal);
        assert_eq!(
            config.keybind,
            CtrlKeyConfig::from_preset(&KeybindPreset::Minimal)
        );
    }

    #[test]
    fn parse_preset_none() {
        let toml = r#"
[general]
keybind_preset = "none"
"#;
        let config = Config::parse(toml).unwrap();
        assert_eq!(config.keybind_preset, KeybindPreset::None);
        assert_eq!(config.keybind, CtrlKeyConfig::default());
    }

    #[test]
    fn parse_preset_missing_defaults_to_none() {
        let toml = r#"
[general]
toggle_key = "zenkaku-hankaku"
"#;
        let config = Config::parse(toml).unwrap();
        assert_eq!(config.keybind_preset, KeybindPreset::None);
    }

    #[test]
    fn parse_preset_invalid_errors() {
        let toml = r#"
[general]
keybind_preset = "vim"
"#;
        let result = Config::parse(toml);
        assert!(result.is_err());
    }

    // === keybind 個別上書き ===

    #[test]
    fn parse_keybind_override() {
        let toml = r#"
[general]
keybind_preset = "emacs"

[keybind]
ctrl_n = "none"
ctrl_p = "none"
"#;
        let config = Config::parse(toml).unwrap();
        assert_eq!(config.keybind.ctrl_j, Some(EngineCommand::Commit));
        assert_eq!(config.keybind.ctrl_g, Some(EngineCommand::Cancel));
        assert_eq!(config.keybind.ctrl_n, None);
        assert_eq!(config.keybind.ctrl_p, None);
        assert_eq!(config.keybind.ctrl_h, Some(EngineCommand::Backspace));
    }

    #[test]
    fn parse_keybind_none_value() {
        let toml = r#"
[general]
keybind_preset = "minimal"

[keybind]
ctrl_j = "none"
"#;
        let config = Config::parse(toml).unwrap();
        assert_eq!(config.keybind.ctrl_j, None);
        // 他はプリセットの値を維持
        assert_eq!(config.keybind.ctrl_g, Some(EngineCommand::Cancel));
    }

    #[test]
    fn parse_keybind_invalid_command_errors() {
        let toml = r#"
[keybind]
ctrl_j = "invalid_command"
"#;
        let result = Config::parse(toml);
        assert!(result.is_err());
    }

    #[test]
    fn parse_none_preset_with_override() {
        let toml = r#"
[general]
keybind_preset = "none"

[keybind]
ctrl_j = "commit"
"#;
        let config = Config::parse(toml).unwrap();
        assert_eq!(config.keybind.ctrl_j, Some(EngineCommand::Commit));
        assert_eq!(config.keybind.ctrl_g, None);
    }

    // === カーソル移動・Delete 設定 ===

    #[test]
    fn parse_ctrl_b_cursor_left() {
        let toml = r#"
[keybind]
ctrl_b = "cursor_left"
"#;
        let config = Config::parse(toml).unwrap();
        assert_eq!(config.keybind.ctrl_b, Some(EngineCommand::CursorLeft));
    }

    #[test]
    fn parse_ctrl_f_cursor_right() {
        let toml = r#"
[keybind]
ctrl_f = "cursor_right"
"#;
        let config = Config::parse(toml).unwrap();
        assert_eq!(config.keybind.ctrl_f, Some(EngineCommand::CursorRight));
    }

    #[test]
    fn parse_ctrl_d_delete() {
        let toml = r#"
[keybind]
ctrl_d = "delete"
"#;
        let config = Config::parse(toml).unwrap();
        assert_eq!(config.keybind.ctrl_d, Some(EngineCommand::Delete));
    }

    #[test]
    fn emacs_preset_includes_ctrl_b_f_d() {
        let toml = r#"
[general]
keybind_preset = "emacs"
"#;
        let config = Config::parse(toml).unwrap();
        assert_eq!(config.keybind.ctrl_b, Some(EngineCommand::CursorLeft));
        assert_eq!(config.keybind.ctrl_f, Some(EngineCommand::CursorRight));
        assert_eq!(config.keybind.ctrl_d, Some(EngineCommand::Delete));
    }
}
