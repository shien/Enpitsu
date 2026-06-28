# Bug 2: TSF にカーソル位置が反映されない (Critical)

## 概要

`EngineOutput.cursor_pos` をエンジンは正しく計算しているが、`text_service.rs` の `update_composition` では使っていない。TSF の Composition 範囲内のカーソル位置を設定していないため、左右矢印キーで内部カーソルは移動するが画面上は常に末尾にカーソルが表示される。

## 現象

- Composing 中に左右矢印キーを押しても、メモ帳上のカーソルは常に未確定テキストの末尾に表示される
- カーソル位置での文字挿入や削除はエンジン内部では正しく動くが、ユーザーにはカーソル位置が見えない

## 原因

`text_service.rs` の `update_composition` → `write_text` は `range.SetText(ec, 0, &wide)` でテキストを設定するだけで、`ITfContext::SetSelection` を呼んでいない。TSF の SetText 後、カーソルは範囲の末尾に配置されるため、`cursor_pos` が反映されない。

## 対応方針

`write_text` の後にカーソル位置を `cursor_pos` に設定する。

### 対象ファイル

- `src/text_service.rs` — `EditSession`, `write_text`, `update_composition`

### 実装詳細

1. `EditAction::SetText` に `cursor_pos: usize` を追加する
2. `write_text` の後、`set_cursor_position(ec, cursor_pos)` を呼ぶ
3. `set_cursor_position` の実装:
   - Composition の Range から `Clone` で新しい Range を作成
   - `ShiftStart` で Range の開始位置を cursor_pos 文字分移動
   - Range の終了位置も同じ位置に設定（カーソル = 0幅の選択）
   - `ITfContext::SetSelection` で `TF_SELECTION` を設定
     - `range`: 上記の Range
     - `style.ase`: `TF_AE_END`
     - `style.fInterimChar`: `FALSE`

### TDD 手順

#### Phase 1: 設計確認

TSF API の利用方法を確認する:

```rust
fn set_cursor_position(&self, ec: u32, cursor_pos: usize) -> Result<()> {
    let comp = self.composition.lock().unwrap();
    if let Some(ref composition) = *comp {
        unsafe {
            let range = composition.GetRange()?;
            let cloned = range.Clone()?;
            // Range の開始位置を先頭にリセット
            cloned.Collapse(ec, TF_ANCHOR_START)?;
            // cursor_pos 文字分移動
            let shifted = cloned.ShiftEnd(ec, cursor_pos as i32, ...)?;
            cloned.ShiftStart(ec, cursor_pos as i32, ...)?;

            let mut selection = TF_SELECTION {
                range: Some(cloned),
                style: TF_SELECTIONSTYLE {
                    ase: TF_AE_END,
                    fInterimChar: FALSE,
                },
            };
            let context: ITfContext = ...;
            context.SetSelection(ec, &[selection])?;
        }
    }
    Ok(())
}
```

**注意:** `ShiftStart` / `ShiftEnd` はバイト単位ではなくコードポイント単位で動作する。UTF-16 のサロゲートペアの扱いに注意が必要だが、日本語の基本的な文字（ひらがな・カタカナ・漢字）は BMP 内なので 1 コードポイント = 1 UTF-16 ユニット。

#### Phase 2: 実装

1. `EditAction::SetText` を `SetText { text: String, cursor_pos: usize }` に変更
2. `update_composition` で `cursor_pos` を `EditAction` に渡す
3. `DoEditSession` 内で `SetText` の後に `set_cursor_position` を呼ぶ
4. `set_cursor_position` メソッドを `EditSession` に追加

**動作確認:** `cargo build` が通ることを確認（TSF API は Windows 環境でのみコンパイル可能）。

#### Phase 3: Refactor

- デバッグログを追加: `set_cursor_position: pos={cursor_pos}`
- エラーハンドリング: SetSelection 失敗時はログを出力して続行

**動作確認:** `cargo build`、`cargo clippy`、`cargo fmt -- --check`。

### 手動テスト (Windows)

1. メモ帳で「かきくけこ」を入力中（Composing）
2. 左矢印キーを押す → カーソルが「け」と「こ」の間に移動することを確認
3. さらに左矢印 → 「く」と「け」の間にカーソルが移動
4. 右矢印 → カーソルが「け」と「こ」の間に戻る
5. カーソル位置で文字入力 → 正しい位置に挿入される
6. Ctrl+B/F（Emacs プリセット時）でも同様に動作する

### 備考

- Bug 1 の修正が先に必要。Direct 状態で矢印キーが消費されない状態にしてから、Composing 状態でのカーソル位置表示を修正する。
- CommitAndCompose アクションでは新しい Composition の cursor_pos も設定する必要がある。
