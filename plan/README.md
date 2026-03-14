# Enpitsu 開発計画

Windows 向け日本語入力システム (IME) を段階的に構築する。
各フェーズは独立して動作・テスト可能な状態で完了する。

## 現在の状態

- [x] Phase 1: ローマ字→かな変換 (68 テスト)
- [x] Phase 2: SKK 辞書の読み込みと検索 (84 テスト)
- [x] Phase 3: 変換エンジンの統合 (125 テスト)
- [x] Phase 4: TSF 連携 (143+ テスト)

実装済みフェーズの仕様: [specification.md](./specification.md)

## フェーズ一覧

| フェーズ | 内容 | 成果物 | 状態 |
|---------|------|--------|------|
| Phase 1-4 | ローマ字→かな→TSF 連携 | [仕様書](./specification.md) | 完了 |
| [Phase 5](./phase5-candidate-ui.md) | 候補ウィンドウ UI | 変換候補をポップアップ表示 | 未着手 |
| [Phase 6](./phase6-polish.md) | 仕上げ・インストーラー | 配布可能な状態 | 未着手 |
| [Phase 7](./phase7-mecab.md) | MeCab/形態素解析による高機能変換 | 連文節変換・予測変換 | 未着手 |
| [Phase 7.5](./phase7.5-emacs-keybind.md) | Emacs キーバインドの追加 | Ctrl+キーによるホームポジション操作 | 未着手 |
| [Phase 8](./phase8-ai-dict.md) | AI 辞書・設定生成 | AI で生成した辞書・テーブルによるオフライン高機能化 | 未着手 |

## 調査事項

- [OnTestKeyDown が呼ばれない問題](./investigation-ontestkeydown.md)

## 方針

- 各フェーズ完了時に `cargo test` が全て通ること
- フェーズ内のタスクにはそれぞれテストを含める
- 先のフェーズに依存しない部分から着手する
