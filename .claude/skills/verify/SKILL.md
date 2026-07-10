---
name: verify
description: Enpitsu の品質ゲートを実行する。コードを変更した後・コミットする前に必ず使う。cargo test / clippy / fmt --check を順に実行し、全て通ることを確認する。
---

# verify — 品質ゲート

コード変更後・コミット前に、このリポジトリで必須の確認を一括で行う。

## 手順

以下を順に実行し、**すべて成功する**ことを確認する。

```sh
cargo test
cargo clippy -- -D warnings
cargo fmt -- --check
```

1. **`cargo test`** — 全テストが通ること。1件でも失敗したらコミットしてはならない（CLAUDE.md のルール）。
2. **`cargo clippy -- -D warnings`** — warning を含めて指摘ゼロであること。
3. **`cargo fmt -- --check`** — フォーマット差分がないこと。差分が出たら `cargo fmt` を実行して修正し、再度 `cargo fmt -- --check` で確認する。

## 失敗したとき

- **テスト失敗**: 失敗したテストの出力を読み、実装を修正する。テスト自体を安易に変更・削除しない。期待値が仕様変更で正しく変わった場合のみテストを更新する。
- **clippy 指摘**: 指摘どおりに修正する。`#[allow(...)]` での抑制は最終手段とし、理由をコメントで残す。
- **fmt 差分**: `cargo fmt` で自動修正する。手で整形しない。

## 補足

- ランタイム動作も確認したい場合は CLI デモを使う: `echo "kanji" | cargo run` あるいは `cargo run -- --dict tests/fixtures/test_dict.txt`。
- Windows 固有部分（TSF/COM）は `#[cfg(windows)]` で分離されており、Linux/macOS では変換ロジックのテストのみ実行される。Windows 固有コードを変更した場合は、この環境ではコンパイルが通ることまでしか確認できない旨を報告に含めること。
