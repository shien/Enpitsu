# 調査: OnTestKeyDown が呼ばれない問題

## 事象

- **発生環境:** Windows 11, メモ帳 (notepad.exe)
- **確認方法:** DebugView (Sysinternals), フィルタなし
- **症状:** DebugView のログに `OnTestKeyDown` のエントリが一切出ない
  - `OnKeyDown` は正常に呼ばれ、文字入力は動作している
  - `AdviseKeyEventSink` は成功 (ログに記録あり)

## ログ抜粋

```
[Enpitsu] ActivateEx: AdviseKeyEventSink succeeded
[Enpitsu] OnKeyDown: vk=0x41, command=InsertChar('a')     ← OnTestKeyDown のログなし
[Enpitsu] OnKeyDown: output committed='', display='あ'
```

## コード状態

- `OnTestKeyDown` の先頭に `debug_log("OnTestKeyDown ENTERED: ...")` あり
- `debug_log` は `OutputDebugStringW` で出力
- `OnTestKeyDown` → `OnKeyDown` は数秒間隔 (キー入力ごと)

## 考えられる原因

1. **DebugView のログ欠落**: `OutputDebugStringW` のカーネルデバッグバッファが小さく、`OnTestKeyDown` → `OnKeyDown` の短い間隔でログが欠落する可能性。ただし、キー入力の間隔は数秒あるため可能性は低い。
2. **COM vtable の問題**: `windows-rs` のマクロ生成で vtable の順序が不正な可能性。ただし `OnKeyDown` は正常に呼ばれているため、可能性は低い。
3. **OnTestKeyDown 内でのパニック**: 最初の `debug_log` 呼び出し前にパニックする箇所はない (format マクロと `wparam.0` のみ)。

## 現状の影響

- IME は正常に動作している（ローマ字→ひらがな変換、確定が可能）
- `OnKeyDown` 内でも未マップキーは `FALSE` を返しているため、キーの素通しは動作する
- 致命的な問題ではないが、TSF の仕様上 `OnTestKeyDown` が呼ばれるべき

## 今後の対応案

- [ ] ファイルログ (`%APPDATA%\enpitsu\debug.log`) を `debug_log` に併用し、`OutputDebugStringW` の欠落かどうかを切り分ける
- [ ] `OnTestKeyDown` 内にカウンター (AtomicU64) を置き、`OnKeyDown` 内でカウント値をログ出力して呼び出し回数を比較する
- [ ] 優先度: 低（現状 IME は動作しているため）
