# カーソル移動 実装計画

## 概要

Composing 状態の未確定文字列内でカーソルを左右に移動し、カーソル位置での文字挿入・削除を可能にする。

## 現状分析

### 現在の `InputState` 構造

```rust
pub struct InputState {
    output: String,   // 確定したひらがな（例: "かん"）
    pending: String,  // 未確定ローマ字（例: "k"）
}
```

- 表示テキスト = `output + pending`
- 文字入力は常に末尾に追加
- Backspace は常に末尾から削除
- カーソル位置の概念なし

### 目標の状態

```
表示: [output_before] [pending] [output_after]
       ~~~~~~~~~~~~~~  ~~~~~~~~  ~~~~~~~~~~~~~
       カーソル前の     入力中    カーソル後の
       確定ひらがな     ローマ字  確定ひらがな
```

- カーソルが末尾のとき（デフォルト）: `output_before = 全output`, `output_after = ""`（既存動作と同一）
- カーソルを左に移動: pending を flush → カーソル位置で output を分割
- 新しい文字入力: `output_before` の末尾に追加（pending が変換されたら `output_before` に入る）
- Backspace: カーソル前の1文字を削除

## 設計

### InputState の変更

```rust
pub struct InputState {
    output_before: String,  // カーソル前の確定ひらがな
    output_after: String,   // カーソル後の確定ひらがな
    pending: String,        // 未確定ローマ字バッファ（常に output_before と output_after の間）
}
```

**メソッド変更:**

| メソッド | 現在 | 変更後 |
|---------|------|--------|
| `output()` | `&self.output` | `output_before + output_after` を結合して返す |
| `pending()` | `&self.pending` | 変更なし |
| `feed_char()` | `output` 末尾に追加 | `output_before` 末尾に追加 |
| `flush()` | `output` 末尾に追加 | `output_before` 末尾に追加 |
| `backspace()` | 末尾から削除 | `pending` → `output_before` の末尾から削除 |
| `reset()` | 両方クリア | 3つ全てクリア |
| `is_empty()` | 両方が空 | 3つ全てが空 |

**新規メソッド:**

| メソッド | 説明 |
|---------|------|
| `move_left()` | カーソルを1文字左に移動。pending があれば先に flush。`output_before` 末尾から `output_after` 先頭へ1文字移動 |
| `move_right()` | カーソルを1文字右に移動。pending があれば先に flush。`output_after` 先頭から `output_before` 末尾へ1文字移動 |
| `cursor_pos()` | カーソル位置を返す（`output_before.chars().count() + pending.chars().count()`） |
| `display()` | 表示用文字列を返す（`output_before + pending + output_after`） |
| `output_before()` | カーソル前の確定ひらがなを返す |
| `output_after()` | カーソル後の確定ひらがなを返す |

### EngineCommand の追加

```rust
pub enum EngineCommand {
    // ... 既存 ...
    CursorLeft,   // カーソルを左に移動
    CursorRight,  // カーソルを右に移動
}
```

### EngineOutput の変更

```rust
pub struct EngineOutput {
    pub committed: String,
    pub display: String,
    pub candidate_index: Option<usize>,
    pub cursor_pos: usize,  // 新規: display 内のカーソル位置（文字数）
}
```

### key_mapping の変更

```
VK_LEFT  → EngineCommand::CursorLeft
VK_RIGHT → EngineCommand::CursorRight
```

Emacs プリセット:
```
Ctrl+B → EngineCommand::CursorLeft
Ctrl+F → EngineCommand::CursorRight
```

### エンジン状態ごとの振る舞い

| 状態 | CursorLeft | CursorRight |
|------|-----------|-------------|
| Direct | 無視（empty_output） | 無視（empty_output） |
| Composing | `input.move_left()` → composing_output | `input.move_right()` → composing_output |
| Converting | 無視（converting_output） | 無視（converting_output） |

### デバッグ出力

`eprintln!` を使い、状態変化を標準エラー出力にログする。テスト時は `--nocapture` で確認可能。

**InputState:**
```rust
eprintln!("[InputState::feed_char] ch='{}' before='{}'|pending='{}'|after='{}'",
    ch, self.output_before, self.pending, self.output_after);
```

**InputState カーソル移動:**
```rust
eprintln!("[InputState::move_left] before='{}'|pending='{}'|after='{}' → before='{}'|after='{}'",
    before_before, before_pending, before_after, self.output_before, self.output_after);
```

**Engine:**
```rust
eprintln!("[Engine::process] state={:?} command={:?}", self.state, command);
eprintln!("[Engine::process] → state={:?} output={:?}", self.state, &result);
```

デバッグ出力は常に `eprintln!` で出力する（`cfg(debug_assertions)` は使わない）。
完成が近づいてから除外を検討する。

## 実装フェーズ

---

### フェーズ 1: InputState のリファクタリング（output → output_before + output_after）

既存テストが全て通る状態を維持しつつ、内部構造を変更する。

#### タスク 1.1: Red — output_before / output_after のテスト追加

新しいメソッド `output_before()`, `output_after()`, `display()` のテストを追加。
カーソル移動はまだ実装しないので、初期状態では `output_after` は常に空。

**テスト:**
- `output_before_equals_output_initially`: `output_before()` が `output()` と一致
- `output_after_empty_initially`: `output_after()` が空
- `display_equals_output_plus_pending`: `display()` が `output() + pending()` と一致

**動作確認:**
- `cargo test` → 新テストが**失敗する**ことを確認

#### タスク 1.2: Green — InputState 内部構造を変更

- `output` フィールドを `output_before` + `output_after` に分割
- `output()` は `output_before + output_after` を返す
- `output_before()`, `output_after()`, `display()` メソッド追加
- デバッグ出力を `feed_char`, `backspace`, `flush`, `reset` に追加
- 全既存テスト + 新テストが通ること

**動作確認:**
- `cargo test` → **全テスト通過**

#### タスク 1.3: Refactor

- `cargo clippy` / `cargo fmt -- --check` でコード品質確認

**動作確認:**
- `cargo test && cargo clippy && cargo fmt -- --check` → 全てパス

---

### フェーズ 2: InputState にカーソル移動を追加

#### タスク 2.1: Red — move_left / move_right のテスト追加

**テスト:**
- `move_left_moves_one_char`: "かき" → move_left → before="か", after="き"
- `move_left_at_beginning_does_nothing`: 空 → move_left → 変化なし
- `move_left_flushes_pending`: "k" pending → move_left → pending flush → before="", after="k"（"k" が output_after に）
- `move_left_flushes_trailing_n`: "kan" → move_left → "かん" flush → before="か", after="ん"
- `move_right_moves_one_char`: before="か", after="き" → move_right → before="かき", after=""
- `move_right_at_end_does_nothing`: before="かき", after="" → move_right → 変化なし
- `move_right_flushes_pending`: pending="k" → move_right → flush → 移動
- `cursor_pos_at_end`: "かき" → cursor_pos = 2
- `cursor_pos_after_move_left`: "かき" → move_left → cursor_pos = 1
- `feed_char_inserts_at_cursor`: "かき" → move_left → 'a' → display="かあき"（pending なし）
- `feed_char_with_pending_at_cursor`: "かき" → move_left → 'k' → display="かkき"（pending="k"）
- `backspace_at_cursor_removes_before`: "かき" → move_left → backspace → display="き"

**動作確認:**
- `cargo test` → 新テストが**失敗する**ことを確認

#### タスク 2.2: Green — move_left / move_right 実装

- `move_left()`: pending があれば flush → `output_before` 末尾の1文字を `output_after` 先頭に移動
- `move_right()`: pending があれば flush → `output_after` 先頭の1文字を `output_before` 末尾に移動
- `cursor_pos()`: `output_before.chars().count() + pending.chars().count()`
- `backspace()`: pending 空 & output_before 非空 → output_before の末尾を削除
- デバッグ出力を `move_left`, `move_right` に追加

**動作確認:**
- `cargo test` → **全テスト通過**

#### タスク 2.3: Refactor

**動作確認:**
- `cargo test && cargo clippy && cargo fmt -- --check` → 全てパス

---

### フェーズ 3: EngineCommand / EngineOutput の拡張

#### タスク 3.1: Red — エンジンのカーソル移動テスト追加

**テスト:**
- `cursor_left_in_composing_moves_cursor`: Composing 中に CursorLeft → cursor_pos が減る
- `cursor_right_in_composing_moves_cursor`: CursorLeft 後に CursorRight → cursor_pos が戻る
- `cursor_left_in_direct_is_noop`: Direct 状態では無視
- `cursor_right_in_direct_is_noop`: Direct 状態では無視
- `cursor_left_in_converting_is_noop`: Converting 状態では無視
- `insert_at_cursor_in_composing`: カーソル移動後に文字入力 → 正しい位置に挿入
- `backspace_at_cursor_in_composing`: カーソル移動後に Backspace → カーソル前を削除
- `cursor_pos_in_output`: `EngineOutput.cursor_pos` が正しい値を返す

**動作確認:**
- `cargo test` → 新テストが**失敗する**ことを確認

#### タスク 3.2: Green — Engine にカーソル移動を実装

- `EngineCommand` に `CursorLeft`, `CursorRight` を追加
- `EngineOutput` に `cursor_pos: usize` を追加（全 EngineOutput 生成箇所を更新）
- `process()` の Composing マッチに CursorLeft/CursorRight を追加
- Direct / Converting では無視
- Engine の `process()` にデバッグ出力を追加

**動作確認:**
- `cargo test` → **全テスト通過**

#### タスク 3.3: Refactor

**動作確認:**
- `cargo test && cargo clippy && cargo fmt -- --check` → 全てパス

---

### フェーズ 4: key_mapping の拡張

#### タスク 4.1: Red — キーマッピングテスト追加

**テスト:**
- `left_arrow_cursor_left`: VK_LEFT → CursorLeft
- `right_arrow_cursor_right`: VK_RIGHT → CursorRight
- `emacs_ctrl_b_cursor_left`: Emacs プリセットで Ctrl+B → CursorLeft
- `emacs_ctrl_f_cursor_right`: Emacs プリセットで Ctrl+F → CursorRight
- `minimal_ctrl_b_returns_none`: Minimal プリセットでは Ctrl+B → None
- `minimal_ctrl_f_returns_none`: Minimal プリセットでは Ctrl+F → None

**動作確認:**
- `cargo test` → 新テストが**失敗する**ことを確認

#### タスク 4.2: Green — キーマッピング実装

- `VK_LEFT` (0x25), `VK_RIGHT` (0x27) 定数を追加
- `map_key()` に VK_LEFT → CursorLeft, VK_RIGHT → CursorRight を追加
- `CtrlKeyConfig` に `ctrl_b`, `ctrl_f` フィールドを追加
- Emacs プリセット: `ctrl_b = CursorLeft`, `ctrl_f = CursorRight`
- Minimal / None プリセット: `ctrl_b = None`, `ctrl_f = None`
- `map_ctrl_key()` に VK_B, VK_F のマッピングを追加

**動作確認:**
- `cargo test` → **全テスト通過**

#### タスク 4.3: Refactor

**動作確認:**
- `cargo test && cargo clippy && cargo fmt -- --check` → 全てパス

---

### フェーズ 5: config.rs の拡張

#### タスク 5.1: Red — 設定テスト追加

**テスト:**
- `parse_ctrl_b_cursor_left`: `ctrl_b = "cursor_left"` → CursorLeft
- `parse_ctrl_f_cursor_right`: `ctrl_f = "cursor_right"` → CursorRight
- `emacs_preset_includes_ctrl_b_f`: Emacs プリセットで ctrl_b/ctrl_f がデフォルト設定される

**動作確認:**
- `cargo test` → 新テストが**失敗する**ことを確認

#### タスク 5.2: Green — 設定パーサー拡張

- `ctrl_b`, `ctrl_f` の設定キーを追加
- `"cursor_left"`, `"cursor_right"` の値をパース
- プリセットから `ctrl_b`, `ctrl_f` を初期化

**動作確認:**
- `cargo test` → **全テスト通過**

#### タスク 5.3: Refactor

**動作確認:**
- `cargo test && cargo clippy && cargo fmt -- --check` → 全てパス

---

### フェーズ 6: CLI デモ更新

#### タスク 6.1: main.rs のデバッグ表示更新

- エンジンの `cursor_pos` をデバッグ表示に含める
- カーソル位置を `_` や `|` で視覚的に表示するオプション

**動作確認:**
- `cargo run` で手動確認。カーソル位置が表示されること。
- `cargo test && cargo clippy && cargo fmt -- --check` → 全てパス

---

## テスト数見積もり

| フェーズ | 新規テスト数 | 累計テスト数（既存含む） |
|---------|------------|----------------------|
| フェーズ 1 | 3 | 既存 + 3 |
| フェーズ 2 | 12 | 既存 + 15 |
| フェーズ 3 | 8 | 既存 + 23 |
| フェーズ 4 | 6 | 既存 + 29 |
| フェーズ 5 | 3 | 既存 + 32 |
| フェーズ 6 | 0（手動確認） | 既存 + 32 |

## デバッグ方法

### テスト時のデバッグ出力

```sh
cargo test -- --nocapture
```

`eprintln!` のログが表示される。

### 特定テストのデバッグ

```sh
cargo test move_left -- --nocapture
```

### CLI デモでのデバッグ

```sh
cargo run -- --dict dict/SKK-JISYO.L
```

`eprintln!` のログが stderr に出力される。

### デバッグ出力の形式

```
[InputState::feed_char] ch='k' before='か'|pending=''|after='き' → before='か'|pending='k'|after='き'
[InputState::move_left] before='かき'|pending=''|after='' → before='か'|pending=''|after='き'
[InputState::move_right] before='か'|pending=''|after='き' → before='かき'|pending=''|after=''
[InputState::backspace] before='かき'|pending=''|after='' → before='か'|pending=''|after=''
[Engine::process] state=Composing command=CursorLeft
[Engine::process] → state=Composing cursor_pos=1 display='かき'
```

デバッグ出力は完成が近づくまで常に有効にしておく。除外は後で検討する。
