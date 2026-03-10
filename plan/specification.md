# Enpitsu 実装済みフェーズ仕様書

Phase 1〜4 の設計・実装内容をまとめたドキュメント。
各フェーズの詳細な実装計画は完了済みのため、ここに仕様として集約する。

---

## Phase 1: ローマ字→かな変換

### 概要

ローマ字入力からひらがな・カタカナを正確に生成する変換エンジン。

### モジュール

| ファイル | 内容 |
|---------|------|
| `src/romaji.rs` | ローマ字→ひらがな変換 (`convert()`) |
| `src/katakana.rs` | ひらがな→カタカナ変換 (`to_katakana()`) |
| `src/input_state.rs` | 逐次入力状態管理 (`InputState`) |
| `src/main.rs` | CLI デモ |

### 主要機能

- **変換テーブル**: 五十音、濁音、半濁音、拗音、促音、「ん」処理、小文字かな (`xa`, `la` 系)、外来語 (`fa`, `va` 系)、句読点 (`,`→`、`, `.`→`。`)、長音記号 (`-`→`ー`)
- **カタカナ変換**: Unicode オフセット方式 (U+3041〜U+3096 → U+30A1〜U+30F6)
- **InputState**: `feed_char(ch)` で逐次入力、`flush()` で未確定バッファ確定、`reset()` でクリア、`backspace()` で1文字削除、`is_empty()` で空判定

### テスト

- 初期テスト 32 件 → 拡張後 68 件
- romaji.rs, katakana.rs, input_state.rs それぞれにユニットテスト

---

## Phase 2: SKK 辞書の読み込みと検索

### 概要

SKK 辞書ファイルを読み込み、ひらがなの読みから変換候補（漢字）を検索する。

### モジュール

| ファイル | 内容 |
|---------|------|
| `src/dictionary.rs` | 辞書パーサー・検索 |
| `tests/fixtures/test_dict.txt` | テスト用 UTF-8 辞書 |

### 主要機能

- **辞書形式**: SKK-JISYO 形式 (`かんじ /漢字/感じ/幹事/`)
- **パーサー** (`parse_line`): コメント行 (`;`) スキップ、空行スキップ、アノテーション (`;` 以降) 除去、送り仮名付きエントリ対応
- **Dictionary 構造体**: `HashMap<String, Vec<String>>` ベース
  - `load_from_file(path)`: UTF-8 / EUC-JP 自動判定 (`encoding_rs`)
  - `lookup(reading)`: 完全一致検索
  - `lookup_prefix(prefix)`: 前方一致検索
- **エンコーディング**: UTF-8 優先、失敗時 EUC-JP (`encoding_rs::EUC_JP`)

### テスト

- 16 件追加 (合計 84 件): パーサー 6 件、検索 3 件、ファイル読み込み 3 件、前方一致 3 件、EUC-JP 1 件

### 依存クレート

- `encoding_rs = "0.8"`

---

## Phase 3: 変換エンジンの統合

### 概要

ローマ字入力→漢字候補の表示まで、一連の変換パイプラインを統合。SKK 方式の変換操作を実装。

### モジュール

| ファイル | 内容 |
|---------|------|
| `src/candidate.rs` | 候補リスト管理 (`CandidateList`) |
| `src/engine.rs` | 変換エンジン (`ConversionEngine`) |

### 状態遷移

```
         InsertChar            Convert              Commit
Direct ──────────→ Composing ──────────→ Converting ──────────→ Direct
                   ↑      │              ↑    │ ↑              │
                   │      │ Commit       │    │ │              │
                   │      ╰──→ Direct    │    │ │ Next/Prev    │
                   │         Cancel      │    ╰─╯              │
                   ╰─────────────────────╯                     │
                          Cancel                               │
```

| 状態 | InsertChar | Convert | Next/Prev | Commit | Cancel | Backspace |
|------|-----------|---------|-----------|--------|--------|-----------|
| Direct | → Composing | 無視 | 無視 | 無視 | 無視 | 無視 |
| Composing | 文字追加 | → Converting | 無視 | ひらがな確定 → Direct | 破棄 → Direct | 1文字削除 |
| Converting | 無視 | Next と同じ | 候補移動 | 候補確定 → Direct | → Composing | 無視 |

### 主要型

- **EngineState**: `Direct`, `Composing`, `Converting`
- **EngineCommand**: `InsertChar(char)`, `Convert`, `NextCandidate`, `PrevCandidate`, `Commit`, `Cancel`, `Backspace`
- **EngineOutput**: `committed` (確定文字列), `display` (表示用), `candidates` (候補リスト), `candidate_index` (選択位置)
- **CandidateList**: `new()`, `current()`, `next()`, `prev()`, `select()`, ラップアラウンド対応

### テスト

- 39 件追加 (合計 125 件): CandidateList 11 件、InputState backspace 5 件、状態遷移 9 件、候補操作 9 件、統合テスト 5 件

---

## Phase 4: TSF (Text Services Framework) 連携

### 概要

Windows の TSF に IME として登録し、任意のアプリケーションでローマ字→ひらがな→漢字変換を使えるようにする。

### アーキテクチャ

```
┌──────────────────────────────────────┐
│  Windows アプリケーション (メモ帳等)    │
└──────────┬───────────────────────────┘
           │ TSF API
┌──────────▼───────────────────────────┐
│  DLL エントリポイント (lib.rs)          │
├──────────────────────────────────────┤
│  ClassFactory (class_factory.rs)     │
├──────────────────────────────────────┤
│  TextService (text_service.rs)       │
│  ├── ITfKeyEventSink                 │
│  └── Composition 管理                │
├──────────────────────────────────────┤
│  KeyMapping (key_mapping.rs) ← TDD   │
├──────────────────────────────────────┤
│  ConversionEngine (engine.rs) ← 既存  │
└──────────────────────────────────────┘
```

### モジュール

| ファイル | プラットフォーム | 内容 |
|---------|----------------|------|
| `src/key_mapping.rs` | 非依存 | VirtualKey → EngineCommand 変換 (TDD) |
| `src/guids.rs` | 非依存 | CLSID, Profile GUID 定義 |
| `src/text_service.rs` | Windows 専用 | ITfTextInputProcessorEx, ITfKeyEventSink, Composition 管理 |
| `src/class_factory.rs` | Windows 専用 | COM ClassFactory |
| `src/registry.rs` | Windows 専用 | COM/TSF レジストリ登録 |
| `installer/install.ps1` | Windows 専用 | インストール/アンインストール用スクリプト |

### プラットフォーム分離

- Windows 専用コードは `#[cfg(windows)]` で囲む
- `windows` crate は `[target.'cfg(windows)'.dependencies]` で追加
- Linux 上でも `cargo build` と `cargo test` が通る状態を維持

### KeyMapping

- IME オフ → 全キー `None` (アプリに素通し)
- Ctrl / Alt 押下中 → `None` (Ctrl+キープリセット対応を除く)
- VK_A〜VK_Z → `InsertChar` (Shift で大文字)
- VK_SPACE → `Convert`, VK_RETURN → `Commit`, VK_ESCAPE → `Cancel`, VK_BACK → `Backspace`
- VK_DOWN → `NextCandidate`, VK_UP → `PrevCandidate`
- OEM キー: `-`, `.`, `,` → 対応する `InsertChar`

### Composition 管理

- `update_composition`: EngineOutput に基づいて Composition を更新
- `ensure_composition`: 未開始なら `InsertTextAtSelection` + `StartComposition` で開始
- `write_text`: `ITfRange::SetText` でテキスト更新
- `commit_composition`: テキスト確定 + Composition 終了
- `ITfCompositionSink`: 外部からの Composition 終了を処理

### テスト

- 18 件追加 (合計 143 件): KeyMapping 18 件 (TDD)
- Windows 固有コードは手動テスト (メモ帳 + DebugView)

### 依存クレート (追加)

```toml
[target.'cfg(windows)'.dependencies]
windows = { version = "0.58", features = [
    "implement",
    "Win32_Foundation",
    "Win32_System_Com",
    "Win32_System_LibraryLoader",
    "Win32_System_Registry",
    "Win32_UI_Input_KeyboardAndMouse",
    "Win32_UI_TextServices",
] }
```

---

## 追加実装 (Phase 4 以降)

### ユーザー辞書 (`src/user_dictionary.rs`)

- 学習結果の永続化 (`%APPDATA%\enpitsu\user_dict.txt`)
- 変換確定時に自動学習、ユーザー辞書の候補をシステム辞書より優先

### 設定ファイル (`src/config.rs`)

- `%APPDATA%\enpitsu\config.toml` からパース
- IME トグルキー設定 (`toggle_key`)
- Ctrl+キープリセット (`keybind_preset`: none / emacs)

### IME トグルキー

- Ctrl+Space / 半角全角 / Alt+` で IME オン/オフ切り替え
- 設定ファイルで変更可能

### Ctrl+キープリセット (Emacs)

- `Ctrl+G` → Cancel, `Ctrl+H` → Backspace, `Ctrl+M` → Commit
- `Ctrl+F` → ForwardChar, `Ctrl+B` → BackwardChar
- `Ctrl+A` → BeginningOfLine, `Ctrl+E` → EndOfLine, `Ctrl+K` → KillLine
