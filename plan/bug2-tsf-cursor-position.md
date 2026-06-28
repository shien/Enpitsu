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

1. `EditAction::SetText` を `SetText { text: String, cursor_pos: usize }` に変更する
2. `EditAction::CommitAndCompose` を `CommitAndCompose { committed: String, display: String, cursor_pos: usize }` に変更する
3. `update_composition` で `output.cursor_pos` を各 `EditAction` に渡す
4. `write_text` の後、`set_cursor_position(ec, cursor_pos)` を呼ぶ
   - `SetText` の場合: テキスト設定後にカーソル位置を設定
   - `CommitAndCompose` の場合: 2回目の `write_text(display)` の後にカーソル位置を設定
5. `set_cursor_position` の実装:
   - Composition の Range を取得し `Clone` で新しい Range を作成
   - `Collapse(ec, TF_ANCHOR_START)` で Range を先頭に collapse（0 幅にする）
   - `ShiftEnd(ec, cursor_pos as i32, &halt_cond, &mut shifted)` で終了位置を移動
   - `Collapse(ec, TF_ANCHOR_END)` で再度 collapse して 0 幅のキャレット位置にする
   - `ITfContext::SetSelection` で `TF_SELECTION` を設定
     - `range`: 上記の Range（0 幅 = キャレット）
     - `style.ase`: `TF_AE_END`
     - `style.fInterimChar`: `FALSE`
   - **注意:** 2回目の `Collapse(TF_ANCHOR_END)` が重要。これを省くと ShiftEnd 後に Range に幅が残り、テキスト選択状態になる可能性がある

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

            // 1. Range を先頭に collapse（0 幅にする）
            cloned.Collapse(ec, TF_ANCHOR_START)?;

            // 2. 終了アンカーを cursor_pos 文字分移動
            let mut shifted: i32 = 0;
            let halt_cond = TF_HALTCOND::default();
            cloned.ShiftEnd(ec, cursor_pos as i32, &halt_cond, &mut shifted)?;

            // 3. 終了位置で再度 collapse → 0 幅のキャレットにする
            //    これを省くと Range に幅が残り選択状態になる
            cloned.Collapse(ec, TF_ANCHOR_END)?;

            // 4. SetSelection でキャレット位置を反映
            let selection = TF_SELECTION {
                range: Some(cloned),
                style: TF_SELECTIONSTYLE {
                    ase: TF_AE_END,
                    fInterimChar: FALSE,
                },
            };
            self.context.SetSelection(ec, &[selection])?;
        }
    }
    Ok(())
}
```

**注意:**
- `Collapse(TF_ANCHOR_END)` が 0 幅キャレットの確保に必須。省略すると ShiftEnd 後に Range 幅が cursor_pos 分残る。
- `ShiftEnd` の戻り値 `shifted` が `cursor_pos` より小さい場合、テキスト末尾に到達している。デバッグログで `shifted != cursor_pos` を検出する。
- `ShiftStart` / `ShiftEnd` はバイト単位ではなくコードポイント単位で動作する。UTF-16 のサロゲートペアの扱いに注意が必要だが、日本語の基本的な文字（ひらがな・カタカナ・漢字）は BMP 内なので 1 コードポイント = 1 UTF-16 ユニット。

#### Phase 2: 実装

1. `EditAction::SetText` を `SetText { text: String, cursor_pos: usize }` に変更
2. `EditAction::CommitAndCompose` を `CommitAndCompose { committed: String, display: String, cursor_pos: usize }` に変更
3. `update_composition` で `output.cursor_pos` を `SetText` と `CommitAndCompose` に渡す
4. `DoEditSession` 内:
   - `SetText` の後に `set_cursor_position(ec, cursor_pos)` を呼ぶ
   - `CommitAndCompose` の 2 回目の `write_text(display)` の後に `set_cursor_position(ec, cursor_pos)` を呼ぶ
5. `set_cursor_position` メソッドを `EditSession` に追加

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
