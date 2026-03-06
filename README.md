# Enpitsu

Windows 向け日本語入力システム (IME)。

Rust で実装された SKK ライクな入力方式で、ローマ字 → ひらがな → 漢字変換を行う。
Text Services Framework (TSF) に対応し、メモ帳やブラウザなど Windows アプリケーション上で動作する。

## 特徴

- **ローマ字 → かな変換**: 逐次変換方式。入力と同時にひらがなに変換される
- **SKK 辞書対応**: EUC-JP / UTF-8 形式の SKK 辞書ファイルを使った漢字変換
- **ユーザー辞書・学習機能**: 選択した候補を記憶し、次回以降の変換で優先表示
- **カタカナ変換**: ひらがな → カタカナの自動変換
- **Emacs キーバインド**: プリセットで Ctrl+J/G/N/P/H/M を有効化可能
- **設定ファイル**: TOML 形式でキーバインドや辞書パスをカスタマイズ
- **プラットフォーム分離**: 変換ロジックはプラットフォーム非依存、Windows 固有部分は `#[cfg(windows)]` で分離

## インストール

### 必要環境

- Windows 10 以降
- [Rust](https://rustup.rs/) (ビルド時のみ)
- SKK 辞書ファイル (`SKK-JISYO.L` 等)

### ビルドとインストール

```powershell
# ビルド
cargo build --release

# インストール（管理者権限で実行）
.\installer\install.ps1

# アンインストール
.\installer\install.ps1 -Uninstall
# IME の DLL は各プロセスにロードされるため、アンインストール後は PC を再起動してください
```

インストール後、Windows の設定 → 時刻と言語 → 言語と地域 → 日本語 → 言語のオプション → インストールされているキーボード → キーボードの追加 から「Enpitsu」を追加する。

### 辞書ファイルの配置

DLL と同じディレクトリに `dict/SKK-JISYO.L` を配置する。
または `config.toml` の `system_dict_path` で辞書パスを指定する。

## 使い方

### 基本操作

1. タスクバーの言語バーから「Enpitsu」を選択して IME をオンにする
2. ローマ字を入力するとリアルタイムでひらがなに変換される
3. Space キーで漢字変換を開始
4. 候補を選んで Enter で確定
5. Escape でキャンセル

### 入力例

```
kanji  → かんじ → [Space] → 漢字 / 感じ / 幹事
toukyou → とうきょう → [Space] → 東京
nihon  → にほん → [Space] → 日本 / 二本
```

### 変換フロー

```
Direct (待機)
  ↓ 文字入力
Composing (ローマ字→かな変換中)
  ↓ Space
Converting (候補選択中)
  ↓ Enter
Direct (確定して待機に戻る)
```

## キーバインド

### 基本キー

| キー | 動作 |
|------|------|
| `a`-`z` | ローマ字入力 |
| Space | 変換開始（候補一覧を表示） |
| Enter | 確定（選択中の候補またはひらがなを挿入） |
| Escape | キャンセル（入力を破棄） |
| Backspace | 1文字削除 |
| ↑ | 前の候補 |
| ↓ | 次の候補 |

### Emacs キーバインド

`config.toml` で `keybind_preset` を設定すると、Ctrl+キーの組み合わせが使える。

#### プリセット一覧

| プリセット | 有効なキー | 用途 |
|-----------|-----------|------|
| `"none"` (デフォルト) | なし | Ctrl+キーは全て OS に委ねる |
| `"minimal"` | Ctrl+J, Ctrl+G, Ctrl+M | 競合の少ないキーのみ |
| `"emacs"` | Ctrl+J/G/M/N/P/H | Emacs フルセット |

#### Emacs プリセットのキー割り当て

| キー | 動作 | 備考 |
|------|------|------|
| Ctrl+J | 確定 | Enter と同じ |
| Ctrl+M | 確定 | Enter と同じ |
| Ctrl+G | キャンセル | Escape と同じ |
| Ctrl+N | 次の候補 | ↓ と同じ。OS の「新規」と競合 |
| Ctrl+P | 前の候補 | ↑ と同じ。OS の「印刷」と競合 |
| Ctrl+H | 1文字削除 | Backspace と同じ。OS の「置換」と競合 |

> Ctrl+N/P/H は Windows の標準ショートカットと競合する。
> 競合を避けたい場合は `keybind_preset = "minimal"` を使う。

#### キーの個別カスタマイズ

プリセットをベースに、`[keybind]` セクションで個別のキーを上書きできる。

```toml
[general]
keybind_preset = "emacs"

[keybind]
# Ctrl+N/P だけ無効にして OS ショートカットと共存
ctrl_n = "none"
ctrl_p = "none"
```

指定可能な値: `commit`, `cancel`, `next`, `prev`, `backspace`, `convert`, `none`

## ローマ字入力

標準的なローマ字入力に対応している。

### 五十音

| | a | i | u | e | o |
|---|---|---|---|---|---|
| - | あ | い | う | え | お |
| k | か | き | く | け | こ |
| s | さ | し (si/shi) | す | せ | そ |
| t | た | ち (ti/chi) | つ (tu/tsu) | て | と |
| n | な | に | ぬ | ね | の |
| h | は | ひ | ふ (hu/fu) | へ | ほ |
| m | ま | み | む | め | も |
| y | や | | ゆ | | よ |
| r | ら | り | る | れ | ろ |
| w | わ | | | | を |
| g | が | ぎ | ぐ | げ | ご |
| z | ざ | じ | ず | ぜ | ぞ |
| d | だ | ぢ | づ | で | ど |
| b | ば | び | ぶ | べ | ぼ |
| p | ぱ | ぴ | ぷ | ぺ | ぽ |

### 拗音・特殊音

| 入力 | 出力 | 入力 | 出力 | 入力 | 出力 |
|------|------|------|------|------|------|
| kya | きゃ | kyu | きゅ | kyo | きょ |
| sha | しゃ | shu | しゅ | sho | しょ |
| cha | ちゃ | chu | ちゅ | cho | ちょ |
| nya | にゃ | nyu | にゅ | nyo | にょ |
| hya | ひゃ | hyu | ひゅ | hyo | ひょ |
| gya | ぎゃ | gyu | ぎゅ | gyo | ぎょ |
| bya | びゃ | byu | びゅ | byo | びょ |
| pya | ぴゃ | pyu | ぴゅ | pyo | ぴょ |
| fa | ふぁ | fi | ふぃ | fe | ふぇ |

### 促音（っ）

子音を重ねると促音になる。

```
gakkou → がっこう
kitte  → きって
happyou → はっぴょう
```

### 撥音（ん）

- 子音の前の `n` は自動的に「ん」になる: `kanji` → かんじ
- 母音の前では `nn` と入力する: `konna` → こんな
- `n` 単体は入力途中として保持される

### 小文字

`x` または `l` を前置すると小文字になる。

| 入力 | 出力 | 入力 | 出力 |
|------|------|------|------|
| xtu / ltu | っ | xa / la | ぁ |
| xya / lya | ゃ | xyu / lyu | ゅ |
| xyo / lyo | ょ | xwa / lwa | ゎ |

## 設定

設定ファイルは `%APPDATA%\enpitsu\config.toml` に保存される。

### 設定項目一覧

| セクション | キー | 値 | デフォルト | 説明 |
|-----------|------|-----|----------|------|
| `[general]` | `toggle_key` | `"zenkaku-hankaku"` / `"ctrl-space"` / `"alt-tilde"` | `"zenkaku-hankaku"` | IME のオン/オフ切り替えキー |
| `[general]` | `keybind_preset` | `"none"` / `"minimal"` / `"emacs"` | `"none"` | Ctrl+キーのプリセット |
| `[dictionary]` | `system_dict_path` | ファイルパス | `""` (DLL 同梱) | システム辞書のパス |
| `[behavior]` | `auto_learn` | `true` / `false` | `true` | 候補選択時に自動学習するか |
| `[keybind]` | `ctrl_j` 等 | コマンド名 / `"none"` | プリセット依存 | 個別キーの上書き |

### 設定ファイル例

```toml
[general]
toggle_key = "zenkaku-hankaku"
keybind_preset = "emacs"

[dictionary]
system_dict_path = ""

[behavior]
auto_learn = true

[keybind]
ctrl_n = "none"
ctrl_p = "none"
```

### ユーザー辞書

ユーザー辞書は `%APPDATA%\enpitsu\user_dict.txt` に SKK 形式で自動保存される。
`auto_learn = true`（デフォルト）の場合、変換で候補を確定するたびに学習データが記録され、
次回以降の変換で選択した候補が優先表示される。

## CLI デモ

Windows 以外の環境でもローマ字→かな変換と辞書検索を試せる。

```sh
# ローマ字→かな変換のみ
cargo run

# 辞書を指定して漢字変換も有効化
cargo run -- --dict path/to/SKK-JISYO.L

# ユーザー辞書も指定（学習結果が保存される）
cargo run -- --dict path/to/SKK-JISYO.L --user-dict path/to/user_dict.txt
```

```
> kanji
  ひらがな: かんじ
  カタカナ: カンジ
  変換候補: 漢字 / 感じ / 幹事
  確定: 漢字
```

## 開発

```sh
cargo build          # ビルド
cargo test           # テスト実行（209テスト）
cargo clippy         # Lint
cargo fmt            # フォーマット
cargo run            # CLI デモ
```

TDD (テスト駆動開発) を採用している。新機能の追加やバグ修正は Red → Green → Refactor のサイクルで行う。

## ライセンス

MIT License - Copyright 2026 shien
