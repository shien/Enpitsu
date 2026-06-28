# Bug 1: Direct 状態でキーが消費される (Critical)

## 概要

IME がオンで Direct 状態（未入力）のとき、矢印キー・Backspace・Enter・Space・Escape・Delete が IME に消費され、アプリケーションに届かない。

## 現象

- IME オン中、テキスト入力していないのにカーソル移動できない
- Backspace でアプリ側の文字を消せない
- Enter で改行できない
- Space で空白を入力できない
- Escape がアプリに届かない
- Delete キーがアプリに届かない

## 原因

`key_mapping::map_key()` は `ime_on == true` なら以下のキーに対して常に `Some(command)` を返す:

- VK_LEFT → CursorLeft
- VK_RIGHT → CursorRight
- VK_UP → PrevCandidate
- VK_DOWN → NextCandidate
- VK_DELETE → Delete
- VK_BACK → Backspace
- VK_RETURN → Commit
- VK_ESCAPE → Cancel
- VK_SPACE → Convert

`text_service.rs` の `OnTestKeyDown` は `map_key()` が `Some` を返すと `TRUE` を返す（＝キーを消費）。しかしエンジンの Direct 状態では `(EngineState::Direct, _) => self.empty_output()` で何もしない。結果、キーは消費されるが何も起きない。

## 対応方針 (案B)

`OnTestKeyDown` でエンジンの状態をチェックし、Direct 状態では InsertChar になるキーのみ消費する。

### 対象ファイル

- `src/text_service.rs` — `OnTestKeyDown` と `OnKeyDown`

### 実装詳細

1. `OnTestKeyDown` 内でエンジンの状態を取得する
2. Direct 状態の場合、文字入力キー（A-Z, 0-9, 句読点）のみ `TRUE` を返す
3. Composing/Converting 状態の場合は従来通り `map_key()` の結果に従う
4. `OnKeyDown` でも同様のガード: Direct 状態で InsertChar 以外のコマンドなら `FALSE` を返す

### ヘルパー関数

`key_mapping.rs` に以下を追加:

```rust
pub fn is_character_key(vk: u16, modifiers: &Modifiers) -> bool
```

文字入力キー（InsertChar を生成するキー）かどうかを判定する。`map_key` 内のロジックと重複を避けるため、この関数を `map_key` 内部でも利用することを検討する。

### TDD 手順

#### Phase 1: Red — テストを書く

1. `key_mapping.rs` に `is_character_key` のテストを追加:
   - アルファベットキー (VK_A) → true
   - 数字キー (VK_0) → true
   - 句読点 (VK_OEM_PERIOD) → true
   - VK_SPACE → false
   - VK_RETURN → false
   - VK_BACK → false
   - VK_LEFT → false
   - VK_ESCAPE → false
   - VK_DELETE → false
   - VK_UP → false
   - VK_DOWN → false
   - Ctrl+A → false（Ctrl 押下時は文字キーではない）
   - Alt+A → false

2. `text_service.rs` のテストは Windows 環境依存のため手動テストで確認する

**動作確認:** `cargo test` で新しいテストが失敗することを確認する。

#### Phase 2: Green — 最小限の実装

1. `key_mapping.rs` に `is_character_key` を実装
2. `text_service.rs` の `OnTestKeyDown` を修正:
   - エンジンの状態を lock して取得
   - `EngineState::Direct` のとき、`is_character_key` が false なら `FALSE` を返す
3. `OnKeyDown` にも同様のガードを追加:
   - `map_key` が `Some` を返しても、Direct 状態で InsertChar 以外なら `FALSE` を返す

**動作確認:** `cargo test` で全テストが通ることを確認する。

#### Phase 3: Refactor

- `OnTestKeyDown` と `OnKeyDown` の判定ロジックが重複していないか確認
- デバッグログに状態ガードの結果を追加

**動作確認:** `cargo test`、`cargo clippy`、`cargo fmt -- --check`。

### 手動テスト (Windows)

1. メモ帳で IME をオンにする
2. 何も入力していない状態で以下を確認:
   - 矢印キーでカーソルが移動する
   - Backspace で手前の文字が消える
   - Enter で改行される
   - Space で空白が入力される
   - Delete で後ろの文字が消える
3. 文字入力（Composing 状態）では従来通り動作する
