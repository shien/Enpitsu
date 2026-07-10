# Enpitsu 開発計画

Windows 向け日本語入力システム (IME) を段階的に構築する。
各フェーズは独立して動作・テスト可能な状態で完了する。

## 現在の状態

全テスト: **293 passed**（`cargo test`）

- [x] Phase 1: ローマ字→かな変換
- [x] Phase 2: SKK 辞書の読み込みと検索
- [x] Phase 3: 変換エンジンの統合
- [x] Phase 4: TSF 連携
- [~] Phase 5: 候補選択（`candidate.rs`）— 候補ナビ・**インライン表示**は実装済み。ポップアップ候補ウィンドウは未実装
- [x] Phase 6: 仕上げ・インストーラー（ユーザー辞書・設定・`installer/install.ps1`）
- [x] Phase 7.5: Emacs キーバインド（`keybind_preset`: none/minimal/emacs）
- [ ] Phase 7: MeCab/形態素解析による連文節変換（**未着手**）
- [ ] Phase 8: AI 辞書・設定生成（**未着手**）
- [ ] Phase 9: インストーラ整備（Windows / Linux）（**未着手**）

> **メモ:** Phase 7（連文節変換）は規模が大きいため後回しにし、先に Phase 7.5（Emacs
> キーバインド）を実装した。番号順と実装順が前後している点に注意。

実装済みフェーズの仕様: [specification.md](./specification.md)

## フェーズ一覧

| フェーズ | 内容 | 成果物 | 状態 |
|---------|------|--------|------|
| Phase 1-4 | ローマ字→かな→TSF 連携 | [仕様書](./specification.md) | ✅ 完了 |
| [Phase 5](./phase5-candidate-ui.md) | 候補選択 | 候補ナビ・インライン表示（ポップアップ窓は未実装） | ⚠️ 一部完了 |
| [Phase 6](./phase6-polish.md) | 仕上げ・インストーラー | 配布可能な状態（CI 未整備） | ✅ ほぼ完了 |
| [Phase 7.5](./phase7.5-emacs-keybind.md) | Emacs キーバインドの追加 | Ctrl+キーによるホームポジション操作 | ✅ 完了 |
| [Phase 7](./phase7-mecab.md) | MeCab/形態素解析による高機能変換 | 連文節変換・予測変換 | ⬜ 未着手 |
| [Phase 8](./phase8-ai-dict.md) | AI 辞書・設定生成 | AI で生成した辞書・テーブルによるオフライン高機能化 | ⬜ 未着手 |
| [Phase 9](./phase9-installer.md) | インストーラ整備 | Windows/Linux 両対応のインストールスクリプト・XDG 準拠パス | ⬜ 未着手 |

### Phase 5 の残タスク

候補の内部管理（`CandidateList`）・キー操作・composition へのインライン表示は実装済み。
以下のポップアップ UI 系は未実装（現状は SKK 的なインライン確定で代替）:

- ポップアップ候補ウィンドウ（`candidate_window.rs` / `ITfCandidateListUIElement`）
- カーソル追従表示・DPI スケーリング・フォント調整（5.1 / 5.4）

### Phase 6 の残タスク

`installer/install.ps1`・ユーザー辞書・設定は実装済み。以下は未対応:

- CI 設定（GitHub Actions）— `.github/workflows/` が未作成
- `tracing` crate によるログ（現状は `OutputDebugString` の `[Enpitsu]` ログを使用）

## 調査事項

- [OnTestKeyDown が呼ばれない問題](./investigation-ontestkeydown.md)

## 方針

- 各フェーズ完了時に `cargo test` が全て通ること
- フェーズ内のタスクにはそれぞれテストを含める
- 先のフェーズに依存しない部分から着手する
