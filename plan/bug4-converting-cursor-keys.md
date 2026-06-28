# Bug 4: Converting 状態でカーソル移動キーが消費される (Low)

## 概要

Converting（候補選択）状態で CursorLeft / CursorRight / Delete キーが wildcard match で消費され、何も起きない。

## 現象

- 候補選択中に左右矢印キーを押しても何も起きない
- Delete キーも同様に消費される
- アプリケーションに渡すべきかどうかは設計判断だが、現状は無意味にキーを消費している

## 原因

`engine.rs` の `(EngineState::Converting, _) => self.converting_output()` wildcard で CursorLeft / CursorRight / Delete が catch され、converting_output() を返す（変化なし）。`OnTestKeyDown` は `map_key` が `Some` を返すので `TRUE` を返し、キーを消費する。

## 対応方針

Bug 1, Bug 3 と同じ仕組みで解決する。`should_consume_key` ヘルパーに Converting 状態のルールを追加する。

### 対象ファイル

- `src/text_service.rs` — `should_consume_key` (Bug 1, 3 で作成するヘルパー)

### 実装詳細

`should_consume_key` に以下のルールを追加:

```rust
(EngineState::Converting, Some(EngineCommand::CursorLeft)) => false,
(EngineState::Converting, Some(EngineCommand::CursorRight)) => false,
(EngineState::Converting, Some(EngineCommand::Delete)) => false,
```

Converting 状態で消費すべきコマンド:
- NextCandidate (↓, Space) — 次の候補
- PrevCandidate (↑) — 前の候補
- Commit (Enter) — 確定
- Cancel (Escape) — キャンセル
- Backspace — キャンセルして Composing に戻る
- InsertChar — 自動確定して新規入力
- Convert — NextCandidate と同じ動作

Converting 状態で消費しないコマンド:
- CursorLeft — アプリに渡す
- CursorRight — アプリに渡す
- Delete — アプリに渡す

### TDD 手順

#### Phase 1: Red — テストを書く

`should_consume_key` のテストを追加:

- Converting + CursorLeft → false
- Converting + CursorRight → false
- Converting + Delete → false
- Converting + NextCandidate → true（既存動作確認）
- Converting + Commit → true（既存動作確認）

**動作確認:** `cargo test` で新しいテストが失敗することを確認する。

#### Phase 2: Green — 実装

`should_consume_key` にルールを追加する。

**動作確認:** `cargo test` で全テストが通ることを確認する。

#### Phase 3: Refactor

- ルールの網羅性を確認
- デバッグログに Converting 状態のキー通過を記録

**動作確認:** `cargo test`、`cargo clippy`、`cargo fmt -- --check`。

### 手動テスト (Windows)

1. メモ帳で「かんじ」を入力 → Space で変換（Converting 状態）
2. 左矢印キーを押す → アプリ側のカーソルが移動する（または候補ウィンドウ外の操作として適切に処理される）
3. 上下矢印で候補選択は従来通り動作する
4. Enter で確定は従来通り動作する

### 備考

- 優先度は Low。Bug 1 と Bug 3 の修正時に `should_consume_key` を拡張するだけなので追加コストは小さい。
- Converting 状態での左右矢印に「文節区切り変更」のような機能を将来追加する可能性がある。その場合はこのルールを見直す。
