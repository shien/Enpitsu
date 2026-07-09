---
name: add-romaji
description: ローマ字→ひらがな変換テーブル (src/romaji.rs の ROMAJI_TABLE) に新しいエントリを追加する。「〜という入力を〜に変換したい」「ローマ字表記を追加したい」というタスクで使う。
---

# add-romaji — 変換テーブルへのエントリ追加

`src/romaji.rs` の `ROMAJI_TABLE` に新しいローマ字→ひらがなエントリを追加する手順。
**テーブルにエントリを追加したら、対応するテストも必ず追加する**（CLAUDE.md のルール）。

## 前提知識

- `ROMAJI_TABLE` は長さ別のセクションに分かれている: `// 3文字のエントリ` → `// 2文字のエントリ` → `// 1文字のエントリ`。**長いエントリを先に**配置する必要がある（長い順マッチのため）。
- `FULL_MATCH_MAP`（完全一致）と `PREFIX_SET`（接頭辞＝入力途中判定）は `ROMAJI_TABLE` から `LazyLock` で自動生成される。**テーブルに追加するだけでよく、他のデータ構造の変更は不要。**
- 促音（子音重ね→っ）と撥音（n+子音→ん）はテーブルではなく `convert()` 内のロジックで処理される。

## 手順（TDD）

1. **Red — テストを先に書く。** `src/romaji.rs` 末尾の `#[cfg(test)] mod tests` に追加する。
   - 既存のカテゴリコメント（例: `// === ふぁ行 ===`）に該当があればそこへ、なければ新しいカテゴリコメントを作る。
   - テスト名は挙動を表す `snake_case`（例: `fa_row`, `sokuon_kk`）。
   - 単独変換に加え、**単語レベルのテスト**（`// === 単語レベルのテスト ===` セクション）や、既存エントリとの接頭辞衝突が疑われる場合はそのケースも書く。

   ```rust
   #[test]
   fn thi_entry() {
       let result = convert("thi");
       assert_eq!(result.output, "てぃ");
       assert_eq!(result.pending, "");
   }
   ```

   `cargo test thi_entry` で**失敗する**ことを確認する。

2. **Green — エントリを追加する。** 文字数に合ったセクションに `RomajiEntry` を追加する。

   ```rust
   RomajiEntry {
       romaji: "thi",
       hiragana: "てぃ",
   },
   ```

   `cargo test` で全テストが通ることを確認する。

3. **確認 — 接頭辞の影響をチェックする。** 新エントリの接頭辞（例: `thi` なら `t`, `th`）が `PREFIX_SET` に入るため、短いエントリの確定タイミングが変わる可能性がある。その接頭辞で終わる入力の `pending` を検証するテストが既存にあるか確認し、なければ追加する。

4. **仕上げ:** `/verify` スキル（`cargo test` / `cargo clippy -- -D warnings` / `cargo fmt -- --check`）を実行する。README.md のローマ字入力表に載せるべきエントリなら README も更新する。
