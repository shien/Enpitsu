# Phase 9: インストーラ整備（Windows / Linux）

## 目標

Enpitsu を「ビルド済み成果物を所定の場所に配置し、すぐ使える状態にする」ためのインストーラを
Windows / Linux の両方に用意する。インストール・アンインストールが各 1 コマンドで完結し、
辞書・設定ファイルも含めてセットアップされる状態をゴールとする。

## 背景

現在の `installer/install.ps1` は `target/release/enpitsu.dll` をその場で `regsvr32` 登録する
開発者向けスクリプトであり、以下の課題がある:

- DLL がビルドディレクトリに置かれたまま登録されるため、`cargo clean` や再ビルドで壊れる
- 辞書 (`dict/SKK-JISYO.L`) と設定 (`%APPDATA%\enpitsu\config.toml`) の配置が手作業
- Linux 向けのインストール手段が存在しない

### プラットフォームごとのスコープ

| プラットフォーム | インストール対象 | 備考 |
|----------------|----------------|------|
| Windows | TSF IME (DLL) + 辞書 + 設定 | 既存 `install.ps1` を「所定ディレクトリへ配置してから登録する」方式に拡充 |
| Linux | CLI (`enpitsu` バイナリ) + 辞書 + 設定 | XDG Base Directory 準拠で配置 |

> **注意:** Linux にはまだ IME エンジン（fcitx5 / IBus 連携）が存在しないため、
> このフェーズの Linux インストーラがセットアップするのは **CLI デモと辞書・設定**である。
> fcitx5 / IBus 対応は将来フェーズ（エンジン実装込み）で扱い、その際も本フェーズで整備する
> パス規約（XDG）とインストーラ構成をそのまま流用できるようにしておく。

### 設計方針

- **設定テンプレートの単一ソース化。** デフォルト設定は `Config::default_toml()`（実装済み）を
  唯一のソースとし、インストーラはスクリプト内にテンプレートを重複記述しない。
  CLI に `--init-config` サブコマンドを追加し、両インストーラから呼び出す。
- **パス解決の共通化。** 現在 `text_service.rs` 内に Windows 専用で埋め込まれている
  `get_appdata_path` を、クロスプラットフォームな `paths` モジュールへ切り出す。
  Windows は `%APPDATA%\enpitsu\`、Linux は XDG (`$XDG_CONFIG_HOME` / `$XDG_DATA_HOME`) に従う。
- **スクリプトはシンプルに保つ。** Windows は PowerShell、Linux は POSIX 互換の bash スクリプト。
  MSI (WiX) や .deb / .rpm パッケージ化は本フェーズのスコープ外（将来拡張）。

## 前提

- Phase 6 の `Config` / `Config::default_toml()` / ユーザー辞書が実装済みであること
- `installer/install.ps1`（regsvr32 登録・解除）が動作していること
- CLI デモ (`src/main.rs`) が `--dict` / `--user-dict` オプションで動作していること

## タスク

### 9.1 テストの追加（paths モジュール / CLI 設定解決）

TDD に従い、まずテストを書いてから実装を行う。

#### paths モジュールのテスト（`src/paths.rs`）

- [ ] `config_dir_uses_xdg_config_home` — Linux: `$XDG_CONFIG_HOME` 設定時に `$XDG_CONFIG_HOME/enpitsu` を返す
- [ ] `config_dir_falls_back_to_home` — Linux: `$XDG_CONFIG_HOME` 未設定時に `~/.config/enpitsu` を返す
- [ ] `data_dir_uses_xdg_data_home` — Linux: `$XDG_DATA_HOME` 設定時に `$XDG_DATA_HOME/enpitsu` を返す
- [ ] `data_dir_falls_back_to_home` — Linux: 未設定時に `~/.local/share/enpitsu` を返す
- [ ] `config_dir_uses_appdata` — Windows: `%APPDATA%\enpitsu` を返す（`#[cfg(windows)]` テスト）
- [ ] `config_file_path_is_config_toml` — `config_file()` が `<config_dir>/config.toml` を返す
- [ ] `default_dict_path_under_data_dir` — `default_dict_file()` が `<data_dir>/dict/SKK-JISYO.L` を返す
- [ ] `user_dict_path_under_data_dir` — `user_dict_file()` が `<data_dir>/user_dict.txt` を返す

環境変数に依存するテストは、環境変数を引数で受け取る内部関数
（例: `config_dir_from(xdg: Option<&str>, home: &str)`）に対して書き、
環境変数の読み取りは薄いラッパーに閉じ込める（テストの並列実行で env が干渉しないようにする）。

#### 設定初期化ヘルパーのテスト（`src/config.rs`）

- [ ] `init_config_creates_file` — 存在しないパスに `Config::default_toml()` の内容でファイルが作られる
- [ ] `init_config_creates_parent_dirs` — 親ディレクトリがなければ作成される
- [ ] `init_config_does_not_overwrite` — 既存ファイルがある場合は上書きせず、その旨を返す
- [ ] `init_config_result_roundtrip` — 生成されたファイルを `Config::load` で読めてデフォルト値と一致する

**動作確認:**
- Red: 上記テストを追加し `cargo test paths` / `cargo test init_config` が**失敗する**ことを確認
- Green 後: `cargo test` で全テストがパスすること

### 9.2 paths モジュールの実装

デフォルトパス解決を `src/paths.rs` に切り出し、クロスプラットフォーム化する。

- [ ] `src/paths.rs` を新規作成し、`lib.rs` に `pub mod paths;` を追加
- [ ] `config_dir()` / `data_dir()` / `config_file()` / `default_dict_file()` / `user_dict_file()` を実装
  - Windows: いずれも `%APPDATA%\enpitsu\` 基点（既存動作を維持）
  - Linux: 設定は `$XDG_CONFIG_HOME/enpitsu`（既定 `~/.config/enpitsu`）、
    データ（辞書・ユーザー辞書）は `$XDG_DATA_HOME/enpitsu`（既定 `~/.local/share/enpitsu`）
- [ ] `text_service.rs` の `get_appdata_path` を `paths` モジュール利用に置き換える
  （`config.toml` / `user_dict.txt` のパスが従来と同一であることを維持）

```rust
/// 設定ディレクトリを返す。
/// Windows: %APPDATA%\enpitsu / Linux: $XDG_CONFIG_HOME/enpitsu (既定 ~/.config/enpitsu)
pub fn config_dir() -> PathBuf { /* ... */ }

/// データディレクトリ（辞書・ユーザー辞書の配置先）を返す。
/// Windows: %APPDATA%\enpitsu / Linux: $XDG_DATA_HOME/enpitsu (既定 ~/.local/share/enpitsu)
pub fn data_dir() -> PathBuf { /* ... */ }
```

**動作確認:**
- `cargo test` で 9.1 の paths テストを含む全テストがパスすること
- `cargo clippy -- -D warnings` / `cargo fmt -- --check` がクリーンであること
- Windows 環境: `cargo build --release` 後、IME が従来どおり
  `%APPDATA%\enpitsu\config.toml` を読むこと（メモ帳 + DebugView で `[Enpitsu]` の
  `Loading config from:` ログを確認）

### 9.3 CLI のデフォルトパス対応と `--init-config`

CLI デモがインストール済みの設定・辞書を自動で見つけられるようにする。
これにより Linux インストーラの成果物（設定・辞書）が実際に CLI から使われる。

- [ ] `--init-config`: `paths::config_file()` に `Config::default_toml()` を書き出して終了する
  （既存ファイルは上書きしない。作成したパス、または既存の旨を表示する）
- [ ] 起動時に `paths::config_file()` から `Config` を読み込む（無ければデフォルト値）
- [ ] 辞書の解決順: `--dict` 引数 > `config.system_dict_path` > `paths::default_dict_file()`
- [ ] ユーザー辞書の解決順: `--user-dict` 引数 > （`auto_learn` 有効時）`paths::user_dict_file()`
- [ ] どの辞書・設定を読んだか（または見つからなかったか）を起動時に stderr へ表示する

**動作確認:**
- `cargo test` で全テストがパスすること
- `cargo run -- --init-config` で `~/.config/enpitsu/config.toml` が生成され、
  再実行時に「既に存在する」旨が表示されること
- `~/.local/share/enpitsu/dict/SKK-JISYO.L` を置いた状態で `cargo run`（引数なし）を実行し、
  辞書が自動で読み込まれて漢字候補が出ること
- `--dict tests/fixtures/test_dict.txt` 指定時は引数が優先されること

### 9.4 Windows インストーラの拡充（`installer/install.ps1`）

「配置してから登録する」方式に変更し、辞書・設定も含めてセットアップする。

- [ ] インストール先を `%ProgramFiles%\Enpitsu`（`-InstallDir` で変更可能）とする
  - `enpitsu.dll` をコピーし、**コピー先の DLL** を `regsvr32 /s` で登録する
  - リポジトリの `dict/` にファイルがあれば `<InstallDir>\dict\` へコピーする
    （`text_service.rs` の `load_default_dict` は DLL と同じディレクトリの `dict/` を参照するため、
    これでシステム辞書が有効になる）
- [ ] `target/release/enpitsu.exe --init-config` を呼び出して
  `%APPDATA%\enpitsu\config.toml` を生成する（既存なら何もしない）
- [ ] アンインストール (`-Uninstall`):
  - インストール先 DLL を `regsvr32 /u /s` で解除し、`<InstallDir>` の削除を試みる
    （DLL がロード中で削除できない場合は再起動後の手動削除を案内する）
  - `%APPDATA%\enpitsu\`（設定・ユーザー辞書）は既定で残す。`-Purge` 指定時のみ削除する
- [ ] 旧方式（`target\release` 直登録）からの移行: インストール時に旧パスの登録が残っていれば
  先に `regsvr32 /u` を試みる
- [ ] 管理者権限チェックを追加し、権限がない場合は日本語メッセージで案内して終了する

**動作確認:**
- Windows 環境で `cargo build --release` → 管理者 PowerShell で `.\installer\install.ps1` を実行し:
  - `%ProgramFiles%\Enpitsu\enpitsu.dll` と `dict\` が配置されること
  - `%APPDATA%\enpitsu\config.toml` が生成されること
  - 設定 → 言語のオプションから Enpitsu を追加し、メモ帳でローマ字→かな→漢字変換が動くこと
  - DebugView で `[Enpitsu]` ログに設定・辞書の読み込みが出ること
- `.\installer\install.ps1 -Uninstall` 実行後、IME 一覧から Enpitsu が消えること
  （`-Purge` なしなら `%APPDATA%\enpitsu\` が残ること、`-Purge` ありなら消えること）
- 二重インストール（再実行）がエラーにならず上書き更新になること

### 9.5 Linux インストーラの新規作成（`installer/install.sh`）

CLI・辞書・設定を XDG 準拠で配置する bash スクリプトを作成する。

- [ ] `installer/install.sh` を新規作成（`#!/usr/bin/env bash`, `set -euo pipefail`）
- [ ] インストール（既定はユーザーローカル、root 不要）:
  - `target/release/enpitsu` を `~/.local/bin/enpitsu` へコピー（`--prefix <dir>` で変更可能）
  - リポジトリの `dict/` にファイルがあれば `${XDG_DATA_HOME:-$HOME/.local/share}/enpitsu/dict/` へコピー
  - インストールしたバイナリで `enpitsu --init-config` を実行し設定を生成
  - `~/.local/bin` が `PATH` に含まれない場合は警告を表示する
- [ ] アンインストール (`--uninstall`):
  - バイナリと data ディレクトリの辞書を削除。設定・ユーザー辞書は既定で残し、
    `--purge` 指定時のみ `config` / `data` ディレクトリごと削除する
- [ ] `--help` で使用方法（日本語）を表示する

```sh
# 使用例
./installer/install.sh              # ~/.local 以下にインストール
./installer/install.sh --uninstall  # アンインストール（設定は残す）
./installer/install.sh --uninstall --purge  # 設定・ユーザー辞書も削除
```

**動作確認:**
- `bash -n installer/install.sh` で構文エラーがないこと（`shellcheck` があれば併用する）
- クリーンな環境変数でのインストール検証（一時ディレクトリを HOME に見立てる）:
  ```sh
  cargo build --release
  export TESTHOME=$(mktemp -d)
  HOME=$TESTHOME XDG_DATA_HOME= XDG_CONFIG_HOME= ./installer/install.sh
  test -x $TESTHOME/.local/bin/enpitsu
  test -f $TESTHOME/.config/enpitsu/config.toml
  ```
  の各コマンドが成功すること
- 同環境で `--uninstall` 後にバイナリが消え、`config.toml` が残ること。
  `--uninstall --purge` 後は `~/.config/enpitsu` / `~/.local/share/enpitsu` も消えること
- 実環境でインストール後、`enpitsu`（引数なし）が設定・辞書を自動検出して起動すること

### 9.6 辞書ダウンロード補助（オプション）

`dict/` は `.gitignore` 対象のため、ユーザーは SKK 辞書を別途入手する必要がある。
両インストーラに辞書ダウンロードオプションを追加して手間を減らす。

- [ ] `install.ps1 -DownloadDict` / `install.sh --download-dict` を追加
- [ ] SKK 辞書の公式配布 URL（`https://skk-dev.github.io/dict/SKK-JISYO.L.gz`）から
  HTTPS でダウンロードし、展開してインストール先の `dict/` に配置する
- [ ] ダウンロード失敗時は手動配置の手順（URL と配置先パス）を表示して継続する
  （辞書なしでもローマ字→かな変換は動作するため、インストール自体は失敗させない）

**動作確認:**
- `./installer/install.sh --download-dict` 実行後、
  `~/.local/share/enpitsu/dict/SKK-JISYO.L` が配置され、`enpitsu` で漢字変換候補が出ること
- ネットワーク遮断状態で実行した場合、エラー案内を表示しつつインストール自体は完了すること
- Windows 環境: `-DownloadDict` で `<InstallDir>\dict\SKK-JISYO.L` が配置されること（手動確認）

### 9.7 ドキュメント更新

- [ ] `CLAUDE.md`: リポジトリ構造（`src/paths.rs`, `installer/install.sh`）、
  Common Commands（`--init-config`）、Linux インストール手順を追記
- [ ] `plan/README.md`: Phase 9 の行を追加し、完了時に状態を更新（`plan-status` Skill に従う）

**動作確認:**
- `CLAUDE.md` 記載のコマンドを実際に実行して手順どおり動くこと
- `plan/README.md` のリンク切れがないこと

## 実装順序

| 順序 | タスク | 依存 | 規模 |
|-----|--------|------|------|
| 1 | 9.1 テストの追加 (Red) | なし | 小 |
| 2 | 9.2 paths モジュール (Green) | 9.1 | 小 |
| 3 | 9.3 CLI デフォルトパス + `--init-config` (Green) | 9.1, 9.2 | 中 |
| 4 | 9.5 Linux インストーラ | 9.3 | 中 |
| 5 | 9.4 Windows インストーラ拡充 | 9.3（`--init-config` を利用） | 中 |
| 6 | 9.6 辞書ダウンロード補助（オプション） | 9.4, 9.5 | 小 |
| 7 | 9.7 ドキュメント更新 | 全タスク | 小 |

Linux インストーラを Windows より先に実装するのは、この開発環境（Linux）で
インストール〜動作確認まで自動で検証できるため。Windows インストーラは
Windows 実機での手動確認が必要になる。

## 完了条件

- `cargo test` で全テストがパスすること（paths / init_config テストを含む）
- `cargo clippy -- -D warnings` / `cargo fmt -- --check` がクリーンであること
- Linux: `./installer/install.sh` 一発で CLI・設定（・辞書）が配置され、
  `enpitsu` が引数なしで設定・辞書を自動検出して動作すること
- Linux: `--uninstall` / `--uninstall --purge` が仕様どおり動作すること
- Windows: `.\installer\install.ps1` 一発で `%ProgramFiles%\Enpitsu` への配置・TSF 登録・
  設定生成が完了し、メモ帳で変換が動作すること（手動確認）
- Windows: `-Uninstall` で登録解除・ファイル削除ができ、ユーザーデータの扱いが
  `-Purge` の有無に従うこと（手動確認）
- 既存の Windows IME の設定・ユーザー辞書パス（`%APPDATA%\enpitsu\`）が変わらないこと

## ファイル構成 (変更対象)

```
src/
├── paths.rs           # 新規: クロスプラットフォームなデフォルトパス解決
├── lib.rs             # paths モジュール宣言の追加
├── config.rs          # init_config ヘルパー（default_toml の書き出し）
├── main.rs            # --init-config / デフォルトパスからの設定・辞書読み込み
├── text_service.rs    # get_appdata_path → paths モジュールへの置き換え
installer/
├── install.ps1        # 配置 + 登録方式へ拡充、-Uninstall/-Purge/-DownloadDict
├── install.sh         # 新規: Linux 用インストーラ（XDG 準拠）
CLAUDE.md              # 構造・コマンド・インストール手順の追記
plan/README.md         # Phase 9 の追加
```

## 注意事項

- **Linux IME 連携は本フェーズのスコープ外。** fcitx5 / IBus のエンジン実装は別フェーズで扱う。
  本フェーズで決めるパス規約（XDG）は将来の IME エンジンでもそのまま使う前提で設計する。
- **既存 Windows ユーザーのパスを壊さない。** `%APPDATA%\enpitsu\config.toml` /
  `user_dict.txt` の場所は変更しない。paths モジュール化は純粋なリファクタリング。
- **辞書ファイルはリポジトリに含めない。** `dict/` は引き続き `.gitignore` 対象。
  インストーラはコピー（存在すれば）とダウンロード（オプション）のみ行う。
- **スクリプトに設定テンプレートを埋め込まない。** デフォルト設定の生成は必ず
  `--init-config`（= `Config::default_toml()`）経由にし、二重管理を避ける。
- **パッケージ化（MSI / .deb / .rpm / AUR）は将来拡張。** 需要が出た時点で
  本フェーズのスクリプトをベースに別フェーズとして計画する。
- **`install.sh` は破壊的操作を最小にする。** 削除対象は自身が配置したパスに限定し、
  `--purge` なしではユーザーデータ（設定・ユーザー辞書）を消さない。
