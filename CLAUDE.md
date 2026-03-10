# CLAUDE.md — Enpitsu

This file provides guidance for AI assistants (including Claude Code) working in this repository.

## Project Overview

**Enpitsu** is a Windows 向け日本語入力システム (IME) written in Rust.

- **License:** MIT (Copyright 2026 shien)
- **Default branch:** `main`

## Repository Structure

```
Enpitsu/
├── Cargo.toml         # Rust package manifest (encoding_rs, windows)
├── src/
│   ├── lib.rs         # Crate root (module declarations + DLL exports)
│   ├── romaji.rs      # ローマ字 → ひらがな変換
│   ├── katakana.rs    # ひらがな → カタカナ変換
│   ├── input_state.rs # 入力状態管理 (逐次入力、backspace)
│   ├── dictionary.rs  # SKK 辞書読み込み・検索
│   ├── user_dictionary.rs # ユーザー辞書管理（学習・永続化）
│   ├── candidate.rs   # 変換候補リスト管理
│   ├── engine.rs      # 変換エンジン (状態機械・コマンド処理・ユーザー辞書連携)
│   ├── key_mapping.rs # VirtualKey → EngineCommand 変換 (Ctrl+キー プリセット対応)
│   ├── config.rs      # 設定ファイルのパース・デフォルト値 (keybind_preset 含む)
│   ├── guids.rs       # CLSID, Profile GUID 定義
│   ├── text_service.rs    # TSF TextService (Windows 専用、設定・ユーザー辞書連携)
│   ├── class_factory.rs   # COM ClassFactory (Windows 専用)
│   ├── registry.rs        # COM/TSF レジストリ登録 (Windows 専用)
│   └── main.rs        # CLI デモ (変換エンジン対応、--user-dict オプション)
├── tests/
│   └── fixtures/
│       └── test_dict.txt  # テスト用 SKK 辞書
├── installer/
│   └── install.ps1    # Windows インストール/アンインストール
├── dict/              # 辞書ファイル配置先 (.gitignore で除外)
├── plan/              # 開発計画ドキュメント
├── LICENSE
└── CLAUDE.md
```

## Development Setup

```sh
git clone <repo-url>
cd Enpitsu
cargo build
```

## Common Commands

| Command | Description |
|---------|-------------|
| `cargo build` | プロジェクトをビルドする |
| `cargo test` | 全テストを実行する |
| `cargo test -- --nocapture` | テスト実行時に stdout を表示する |
| `cargo test <test_name>` | 特定のテストのみ実行する |
| `cargo clippy` | lint チェックを実行する |
| `cargo fmt` | コードをフォーマットする |
| `cargo fmt -- --check` | フォーマット差分があるかチェックする（CI向け） |
| `cargo run` | CLI デモを起動する（ローマ字→かな変換） |
| `cargo run -- --dict <path>` | 辞書指定で CLI デモを起動する |
| `cargo run -- --dict <path> --user-dict <path>` | 辞書・ユーザー辞書指定で CLI デモを起動する |
| `cargo build --release` | リリースビルド（Windows DLL 生成） |

## Windows IME インストール

Windows 環境でのみ:

```powershell
cargo build --release
.\installer\install.ps1          # インストール（管理者権限）
.\installer\install.ps1 -Uninstall  # アンインストール
```

## Windows IME 手動テスト

- **テスト対象アプリ:** メモ帳（notepad.exe）を使用する。
- **デバッグログ確認:** DebugView（Sysinternals）をフィルタなしで使用し、`[Enpitsu]` のログを確認する。

## Code Conventions

- **Language:** Rust (Edition 2024)
- **Formatting:** `rustfmt` (デフォルト設定)
- **Linting:** `clippy`
- **Testing:** `cargo test` (Rust 標準のテストフレームワーク)

## Testing Rules

以下のルールに従ってテストを書くこと。

### テスト実行

- コードを変更したら `cargo test` で全テストが通ることを必ず確認する。
- テストが失敗した状態でコミットしてはならない。

### テストの配置

- **ユニットテスト**は、対象モジュールと同じファイル内の `#[cfg(test)] mod tests { ... }` ブロックに書く。
- **結合テスト**（将来必要になった場合）は `tests/` ディレクトリに配置する。

### テストの書き方

- テスト関数名は `snake_case` で、テスト対象の挙動を簡潔に表す名前にする。
  - 良い例: `fn sokuon_kk()`, `fn n_before_consonant()`
  - 悪い例: `fn test1()`, `fn it_works()`
- テストはカテゴリごとにコメントで区切る（例: `// === 促音（っ） ===`）。
- `assert_eq!` を基本とし、期待値を右辺に書く: `assert_eq!(actual, expected)`。
- 新しい機能を追加するときは、最低限以下のテストを含める:
  - 正常系（基本的な入力）
  - エッジケース（空入力、境界値など）
  - 既知の特殊ケース（その機能固有の注意点）

### テスト駆動開発 (TDD)

このプロジェクトではテスト駆動開発を採用する。以下のサイクルを守ること:

1. **Red**: まず失敗するテストを書く。期待する振る舞いをテストで定義する。
   - 動作確認: `cargo test` を実行し、新しいテストが**失敗する**ことを確認する。
2. **Green**: テストが通る最小限の実装を書く。
   - 動作確認: `cargo test` を実行し、新しいテストを含む**全テストが通る**ことを確認する。
3. **Refactor**: テストが通る状態を維持しながらコードを整理する。
   - 動作確認: `cargo test` で全テストが通ることを確認し、`cargo clippy` と `cargo fmt -- --check` で品質を確認する。

- **実装よりテストを先に書く。** 新しい関数・機能を追加するときは、必ずテストを先に書いてから実装に取り掛かる。
- テストを書く前にいきなり実装コードを書いてはならない。
- バグを修正するときは、そのバグを再現するテストを先に書く（回帰テスト）。
- 変換テーブルにエントリを追加したら、対応するテストも追加する。
- **計画時に動作確認方法を明記する。** 各フェーズ（Red / Green / Refactor）で何をどう確認するかを、作業開始前に計画として書き出すこと。
- **計画ドキュメントの各タスクに動作確認を書く。** `plan/` フォルダの各フェーズファイルでは、タスクごとに「**動作確認:**」セクションを設け、具体的な確認手順（実行するコマンド、期待する結果、手動確認の内容）を明記すること。

## Git Workflow

- The default branch is `main`.
- Feature branches should use descriptive names (e.g., `feat/romaji-to-kana`, `fix/input-lag`).
- Write clear, concise commit messages that explain the "why" behind changes.

## Guidelines for AI Assistants

- **Read before modifying.** Always read existing files before proposing changes.
- **Keep changes minimal.** Only modify what is necessary for the task at hand.
- **Do not over-engineer.** Avoid adding abstractions, utilities, or features beyond what is explicitly requested.
- **Preserve existing conventions.** Match the style, formatting, and patterns already present in the codebase.
- **No unnecessary files.** Do not create documentation, config files, or boilerplate unless explicitly asked.
- **Security first.** Do not introduce command injection, XSS, SQL injection, or other common vulnerabilities.
- **Run tests.** Before committing, always run `cargo test` and confirm all tests pass.
- **Update this file.** When new tooling, structure, or conventions are added to the project, update this CLAUDE.md to reflect the current state.
