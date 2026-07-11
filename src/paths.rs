//! クロスプラットフォームなデフォルトパス解決。
//!
//! 設定・辞書・ユーザー辞書の既定の配置場所を OS ごとに解決する。
//!
//! - **Windows:** 設定・データとも `%APPDATA%\enpitsu\` を基点とする（従来動作を維持）。
//! - **Linux:** XDG Base Directory 仕様に従う。
//!   - 設定: `$XDG_CONFIG_HOME/enpitsu`（既定 `~/.config/enpitsu`）
//!   - データ（辞書・ユーザー辞書）: `$XDG_DATA_HOME/enpitsu`（既定 `~/.local/share/enpitsu`）
//!
//! 環境変数の読み取りは各公開関数の薄いラッパーに閉じ込め、パス組み立てのロジックは
//! 環境変数を引数で受け取る内部関数（`*_from`）に切り出してテスト可能にしている。

use std::path::PathBuf;

/// アプリケーション用サブディレクトリ名。
const APP_DIR: &str = "enpitsu";

// === 設定ディレクトリ ===

/// 設定ディレクトリを返す。
///
/// Windows: `%APPDATA%\enpitsu` / Linux: `$XDG_CONFIG_HOME/enpitsu`（既定 `~/.config/enpitsu`）。
pub fn config_dir() -> PathBuf {
    #[cfg(windows)]
    {
        appdata_dir_from(std::env::var("APPDATA").ok().as_deref())
    }
    #[cfg(not(windows))]
    {
        let home = std::env::var("HOME").unwrap_or_else(|_| ".".to_string());
        config_dir_from(std::env::var("XDG_CONFIG_HOME").ok().as_deref(), &home)
    }
}

/// データディレクトリ（辞書・ユーザー辞書の配置先）を返す。
///
/// Windows: `%APPDATA%\enpitsu` / Linux: `$XDG_DATA_HOME/enpitsu`（既定 `~/.local/share/enpitsu`）。
pub fn data_dir() -> PathBuf {
    #[cfg(windows)]
    {
        appdata_dir_from(std::env::var("APPDATA").ok().as_deref())
    }
    #[cfg(not(windows))]
    {
        let home = std::env::var("HOME").unwrap_or_else(|_| ".".to_string());
        data_dir_from(std::env::var("XDG_DATA_HOME").ok().as_deref(), &home)
    }
}

// === 個別ファイルパス ===

/// 設定ファイル `config.toml` のパスを返す。
pub fn config_file() -> PathBuf {
    config_dir().join("config.toml")
}

/// 既定のシステム辞書ファイル `dict/SKK-JISYO.L` のパスを返す。
pub fn default_dict_file() -> PathBuf {
    data_dir().join("dict").join("SKK-JISYO.L")
}

/// ユーザー辞書ファイル `user_dict.txt` のパスを返す。
pub fn user_dict_file() -> PathBuf {
    data_dir().join("user_dict.txt")
}

// === 内部ヘルパー（環境変数を引数で受け取りテスト可能にする） ===

/// Windows: `%APPDATA%\enpitsu` を組み立てる。`APPDATA` 未設定時はカレントを基点にする。
#[cfg(windows)]
fn appdata_dir_from(appdata: Option<&str>) -> PathBuf {
    let base = appdata.filter(|s| !s.is_empty()).unwrap_or(".");
    PathBuf::from(base).join(APP_DIR)
}

/// Linux: 設定ディレクトリを組み立てる。
/// `XDG_CONFIG_HOME` が非空ならそれを、未設定・空なら `~/.config` を基点にする。
#[cfg(not(windows))]
fn config_dir_from(xdg_config_home: Option<&str>, home: &str) -> PathBuf {
    match xdg_config_home {
        Some(x) if !x.is_empty() => PathBuf::from(x).join(APP_DIR),
        _ => PathBuf::from(home).join(".config").join(APP_DIR),
    }
}

/// Linux: データディレクトリを組み立てる。
/// `XDG_DATA_HOME` が非空ならそれを、未設定・空なら `~/.local/share` を基点にする。
#[cfg(not(windows))]
fn data_dir_from(xdg_data_home: Option<&str>, home: &str) -> PathBuf {
    match xdg_data_home {
        Some(x) if !x.is_empty() => PathBuf::from(x).join(APP_DIR),
        _ => PathBuf::from(home)
            .join(".local")
            .join("share")
            .join(APP_DIR),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // === Linux: 設定ディレクトリ ===

    #[cfg(not(windows))]
    #[test]
    fn config_dir_uses_xdg_config_home() {
        let dir = config_dir_from(Some("/custom/config"), "/home/alice");
        assert_eq!(dir, PathBuf::from("/custom/config/enpitsu"));
    }

    #[cfg(not(windows))]
    #[test]
    fn config_dir_falls_back_to_home() {
        // 未設定
        let dir = config_dir_from(None, "/home/alice");
        assert_eq!(dir, PathBuf::from("/home/alice/.config/enpitsu"));
        // 空文字列も未設定扱い
        let dir_empty = config_dir_from(Some(""), "/home/alice");
        assert_eq!(dir_empty, PathBuf::from("/home/alice/.config/enpitsu"));
    }

    // === Linux: データディレクトリ ===

    #[cfg(not(windows))]
    #[test]
    fn data_dir_uses_xdg_data_home() {
        let dir = data_dir_from(Some("/custom/data"), "/home/alice");
        assert_eq!(dir, PathBuf::from("/custom/data/enpitsu"));
    }

    #[cfg(not(windows))]
    #[test]
    fn data_dir_falls_back_to_home() {
        let dir = data_dir_from(None, "/home/alice");
        assert_eq!(dir, PathBuf::from("/home/alice/.local/share/enpitsu"));
        let dir_empty = data_dir_from(Some(""), "/home/alice");
        assert_eq!(dir_empty, PathBuf::from("/home/alice/.local/share/enpitsu"));
    }

    // === Windows: 設定ディレクトリ ===

    #[cfg(windows)]
    #[test]
    fn config_dir_uses_appdata() {
        let dir = appdata_dir_from(Some(r"C:\Users\alice\AppData\Roaming"));
        assert_eq!(
            dir,
            PathBuf::from(r"C:\Users\alice\AppData\Roaming").join("enpitsu")
        );
    }

    // === 個別ファイルパス（環境非依存の関係性を検証） ===

    #[test]
    fn config_file_path_is_config_toml() {
        assert_eq!(config_file(), config_dir().join("config.toml"));
        assert!(config_file().ends_with("config.toml"));
    }

    #[test]
    fn default_dict_path_under_data_dir() {
        assert_eq!(
            default_dict_file(),
            data_dir().join("dict").join("SKK-JISYO.L")
        );
        assert!(default_dict_file().ends_with("SKK-JISYO.L"));
    }

    #[test]
    fn user_dict_path_under_data_dir() {
        assert_eq!(user_dict_file(), data_dir().join("user_dict.txt"));
        assert!(user_dict_file().ends_with("user_dict.txt"));
    }
}
