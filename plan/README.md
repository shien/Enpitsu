# Enpitsu 開発計画

Windows 向け日本語入力システム (IME) を段階的に構築する。
各フェーズは独立して動作・テスト可能な状態で完了する。

## 現在の状態

- [x] Cargo プロジェクト初期化済み
- [x] ローマ字→ひらがな変換モジュール (`src/romaji.rs`) + テスト 32件

## フェーズ一覧

| フェーズ | 内容 | 成果物 |
|---------|------|--------|
| [Phase 1](./phase1-romaji.md) | ローマ字→かな変換の完成 | CLI で動作確認できるレベル |
| [Phase 2](./phase2-dictionary.md) | SKK 辞書の読み込みと検索 | かな→漢字の変換候補を返せる |
| [Phase 3](./phase3-engine.md) | 変換エンジンの統合 | ローマ字入力→漢字候補の一連の流れ |
| [Phase 4](./phase4-tsf.md) | TSF (Text Services Framework) 連携 | Windows にIMEとして登録・最小動作 |
| [Phase 5](./phase5-candidate-ui.md) | 候補ウィンドウ UI | 変換候補をポップアップ表示 |
| [Phase 6](./phase6-polish.md) | 仕上げ・インストーラー | 配布可能な状態 |
| [Phase 7](./phase7-mecab.md) | MeCab/形態素解析による高機能変換 | 連文節変換・予測変換 |
| [Phase 7.5](./phase7.5-emacs-keybind.md) | Emacs キーバインドの追加 | Ctrl+キーによるホームポジション操作 |
| [Phase 8](./phase8-ai-dict.md) | AI 辞書・設定生成 | AI で生成した辞書・テーブルによるオフライン高機能化 |

## 方針

- 各フェーズ完了時に `cargo test` が全て通ること
- フェーズ内のタスクにはそれぞれテストを含める
- 先のフェーズに依存しない部分から着手する
