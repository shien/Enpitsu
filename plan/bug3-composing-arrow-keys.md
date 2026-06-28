# Bug 3: Composing 状態で上下矢印キーが消費される (Medium)

## 概要

Composing 状態で VK_UP / VK_DOWN が NextCandidate / PrevCandidate にマッピングされるが、候補が存在しないため何もせずキーを消費する。

## 現象

- 未確定テキスト入力中に上下矢印キーが効かない
- アプリケーション側のカーソル上下移動ができない

## 原因

`key_mapping::map_key()` は Composing/Converting の区別なく VK_UP → PrevCandidate、VK_DOWN → NextCandidate を返す。engine.rs では `(EngineState::Composing, _) => self.composing_output()` の wildcard で catch し、キーを消費するが何も変化しない。

## 対応方針

Bug 1 と同じ仕組みで解決する。`OnTestKeyDown` でエンジンの状態をチェックし、Composing 状態では NextCandidate / PrevCandidate を消費しない。

### 対象ファイル

- `src/text_service.rs` — `OnTestKeyDown`, `OnKeyDown`
- `src/key_mapping.rs` — ヘルパー関数追加（Bug 1 と共通）

### 実装詳細

Bug 1 の `OnTestKeyDown` 修正に追加で:

1. Composing 状態の場合:
   - `map_key` が NextCandidate / PrevCandidate を返したら `FALSE` を返す（消費しない）
   - 他のコマンドは従来通り消費する
2. `OnKeyDown` でも同様のガード

### 方法の選択肢

**方法 A: OnTestKeyDown で状態×コマンドをチェック**

```rust
let engine = self.engine.lock().unwrap();
let state = engine.state();
drop(engine);

match (state, &command) {
    (EngineState::Direct, _) if !is_character_key(...) => FALSE,
    (EngineState::Composing, Some(EngineCommand::NextCandidate)) => FALSE,
    (EngineState::Composing, Some(EngineCommand::PrevCandidate)) => FALSE,
    _ => TRUE,
}
```

**方法 B: map_key に状態を渡す**

`map_key` のシグネチャを変更して `EngineState` も受け取り、Composing 状態では VK_UP/DOWN を None にする。ただしエンジンの状態を key_mapping に持ち込むことになり、責務の分離が崩れる。

→ **方法 A** を採用する。`OnTestKeyDown` で判定するのが TSF 層の責務として自然。

### TDD 手順

#### Phase 1: Red — テストを書く

text_service.rs は Windows 依存のため直接のユニットテストは困難。ただし、判定ロジックをヘルパー関数に抽出すればテスト可能:

```rust
pub fn should_consume_key(
    state: EngineState,
    command: &Option<EngineCommand>,
) -> bool
```

- Composing + NextCandidate → false
- Composing + PrevCandidate → false
- Converting + NextCandidate → true
- Converting + PrevCandidate → true
- Composing + CursorLeft → true
- Composing + InsertChar → true

**動作確認:** `cargo test` で新しいテストが失敗することを確認する。

#### Phase 2: Green — 実装

1. `should_consume_key` をヘルパーとして実装
2. `OnTestKeyDown` と `OnKeyDown` で使用

**動作確認:** `cargo test` で全テストが通ることを確認する。

#### Phase 3: Refactor

- Bug 1 の Direct 状態ガードと統合する

**動作確認:** `cargo test`、`cargo clippy`、`cargo fmt -- --check`。

### Composition アクティブ時の整合性リスク

Composing 状態で上下矢印キーをアプリに渡す場合、TSF の Composition はアクティブなままである。アプリ側でカーソルが上下に移動すると、IME が保持している Composition 範囲とアプリ側の編集位置がずれるリスクがある。

**検証方針:**
- キーを渡す前に Composition を commit/cancel するべきか、維持したまま渡してよいか
- メモ帳・ブラウザ等の主要アプリで実際にどう振る舞うかを手動テストで確認する
- TSF の仕様上、Composition がアクティブなときに SetSelection でカーソルを Composition 外に移動させると Composition が破棄される可能性がある → この場合はアプリ側の TSF 実装依存

**安全策:** Composing 中の上下矢印を FALSE で返す前に、未確定テキストを自動確定（Commit）してから FALSE を返す方針も検討する。ただしユーザーが意図しない確定が発生するため、まずは FALSE のみ返して実機で挙動を確認し、問題があれば自動確定を導入する。

### 手動テスト (Windows)

1. メモ帳で「かんじ」を入力中（Composing、変換前）
2. 上矢印キーを押す → カーソルが上の行に移動する（IME が消費しない）
3. 下矢印キーを押す → カーソルが下の行に移動する
4. Space で変換（Converting）→ 上下矢印で候補が切り替わる（従来通り動作）
5. **整合性確認:** 上記 2, 3 の後、未確定テキストの状態がどうなるか確認する
   - Composition が維持されているか
   - 未確定テキストの表示位置がずれていないか
   - 再度文字入力したとき正しく動作するか
