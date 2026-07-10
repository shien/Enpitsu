# Phase 7.5: Emacs キーバインドの追加（プリセット + 設定で変更可能）

> **状態: ✅ 完了（2026-07 時点）**
> `keybind_preset`（`none` / `minimal` / `emacs`）と個別上書きを `src/config.rs` に、
> Ctrl+キー → `EngineCommand` 変換を `src/key_mapping.rs` に実装済み。

## 目標

Emacs スタイルのキーバインド（Ctrl+キー）を変換エンジンに追加し、
ホームポジションから手を離さずに変換操作できるようにする。
プリセット方式と個別設定を組み合わせ、ユーザーが好みに応じてカスタマイズできるようにする。

## 背景

現在のキーバインドは矢印キーや Enter/Escape など、ホームポジションから離れたキーに依存している。
日本語 IME のユーザーには Emacs キーバインドを好む層が多く、SKK 自体も Emacs 発祥のため親和性が高い。
Ctrl+キーの組み合わせで変換・確定・キャンセル等の操作を可能にすることで、入力効率を大幅に向上させる。

ただし、`Ctrl+N`（新規）、`Ctrl+P`（印刷）、`Ctrl+H`（置換）などは Windows アプリケーションの
標準ショートカットと競合する。そのため、プリセットと個別設定の2段階でキーバインドを管理する。

### プリセット方式

`config.toml` で `keybind_preset` を指定し、キーバインドのベースを選択する。
プリセットをベースに、`[keybind]` セクションで個別のキーを上書きできる。

| プリセット | 説明 | 有効なキー |
|-----------|------|-----------|
| `"none"` | Ctrl+キー割り当てなし（デフォルト） | なし |
| `"minimal"` | 競合の少ないキーのみ | `Ctrl+J`=確定, `Ctrl+G`=キャンセル, `Ctrl+M`=確定 |
| `"emacs"` | Emacs フルセット | `Ctrl+J/G/N/P/H/M` すべて有効 |

**デフォルトは `"none"`**。Emacs キーバインドを使いたいユーザーが明示的に有効化する。

### 各プリセットの詳細

#### `"none"` プリセット（デフォルト）

Ctrl+キーの割り当てなし。従来と同じ動作。

| キーバインド | 割り当て |
|-------------|---------|
| `Ctrl+J` | なし |
| `Ctrl+G` | なし |
| `Ctrl+N` | なし |
| `Ctrl+P` | なし |
| `Ctrl+H` | なし |
| `Ctrl+M` | なし |

#### `"minimal"` プリセット

Windows 標準ショートカットと競合しにくいキーのみ有効。

| キーバインド | 割り当て | 競合リスク |
|-------------|---------|-----------|
| `Ctrl+J` | Commit (確定) | 低 — 標準の割り当てなし |
| `Ctrl+G` | Cancel (キャンセル) | 低 — 一部エディタで行ジャンプ |
| `Ctrl+M` | Commit (確定) | 低 — Enter と同等 |
| `Ctrl+N` | なし | 中 — 新規ウィンドウ |
| `Ctrl+P` | なし | 中 — 印刷 |
| `Ctrl+H` | なし | 中 — 置換ダイアログ |

#### `"emacs"` プリセット

Emacs キーバインドをフルに有効化。OS のショートカットを上書きする。

| キーバインド | 割り当て | 上書きされる OS 操作 |
|-------------|---------|-------------------|
| `Ctrl+J` | Commit (確定) | — |
| `Ctrl+G` | Cancel (キャンセル) | 行ジャンプ（一部エディタ） |
| `Ctrl+N` | NextCandidate (次の候補) | 新規ウィンドウ |
| `Ctrl+P` | PrevCandidate (前の候補) | 印刷 |
| `Ctrl+H` | Backspace (1文字削除) | 置換ダイアログ |
| `Ctrl+M` | Commit (確定) | — |

### 設定ファイルの構造

```toml
# config.toml

[general]
# プリセット: "none" | "minimal" | "emacs"
keybind_preset = "none"

[keybind]
# プリセットをベースに、個別のキーを上書きする。
# 値は: commit, cancel, next, prev, backspace, convert, none
# このセクションを省略するとプリセットがそのまま適用される。
# ctrl_j = "commit"
# ctrl_n = "none"
```

**適用ルール:**
1. `keybind_preset` でベースとなるキーバインドを決定する
2. `[keybind]` セクションで指定されたキーのみ上書きする
3. `[keybind]` セクション未指定のキーはプリセットの値を維持する
4. `"none"` を指定するとそのキーの割り当てを無効化する

**Phase 7.5 のスコープ:** `Ctrl+J`, `Ctrl+G`, `Ctrl+N`, `Ctrl+P`, `Ctrl+H`, `Ctrl+M` の6つ。
カーソル移動系 (`Ctrl+A`, `Ctrl+E`, `Ctrl+F`, `Ctrl+B`) は Composing 状態でのカーソル移動機能と
合わせて将来フェーズで実装する。

## 前提

- Phase 3 の `EngineCommand` 体系が実装済みであること
- `key_mapping.rs` の `map_key` 関数が VirtualKey → EngineCommand の変換を担当していること
- `Modifiers` 構造体が `ctrl` フラグを持っていること
- Phase 6 の `Config` 構造体と TOML パーサーが設計済みであること

## タスク

### 7.5.1 CtrlKeyConfig 構造体とプリセットの定義

Ctrl+キーの割り当てを保持する構造体とプリセットを定義する。

- [ ] `CtrlKeyConfig` 構造体を `key_mapping.rs` に定義する
- [ ] 各フィールドは `Option<EngineCommand>` 型（`None` = OS に委ねる）
- [ ] `KeybindPreset` 列挙型を定義: `None`, `Minimal`, `Emacs`
- [ ] `CtrlKeyConfig::from_preset(preset)` で各プリセットの設定を生成
- [ ] `Default` トレイトは `KeybindPreset::None`（全て無効）を返す

```rust
/// キーバインドプリセット。
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum KeybindPreset {
    /// Ctrl+キー割り当てなし（デフォルト）
    None,
    /// 競合の少ないキーのみ (Ctrl+J, Ctrl+G, Ctrl+M)
    Minimal,
    /// Emacs フルセット (Ctrl+J/G/N/P/H/M)
    Emacs,
}

/// Ctrl+キーの割り当て設定。
/// 各フィールドが None の場合、そのキーは OS に処理を委ねる。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CtrlKeyConfig {
    pub ctrl_g: Option<EngineCommand>,
    pub ctrl_h: Option<EngineCommand>,
    pub ctrl_j: Option<EngineCommand>,
    pub ctrl_m: Option<EngineCommand>,
    pub ctrl_n: Option<EngineCommand>,
    pub ctrl_p: Option<EngineCommand>,
}

impl CtrlKeyConfig {
    /// プリセットからキーバインド設定を生成する。
    pub fn from_preset(preset: &KeybindPreset) -> Self {
        match preset {
            KeybindPreset::None => Self {
                ctrl_g: None,
                ctrl_h: None,
                ctrl_j: None,
                ctrl_m: None,
                ctrl_n: None,
                ctrl_p: None,
            },
            KeybindPreset::Minimal => Self {
                ctrl_g: Some(EngineCommand::Cancel),
                ctrl_h: None,
                ctrl_j: Some(EngineCommand::Commit),
                ctrl_m: Some(EngineCommand::Commit),
                ctrl_n: None,
                ctrl_p: None,
            },
            KeybindPreset::Emacs => Self {
                ctrl_g: Some(EngineCommand::Cancel),
                ctrl_h: Some(EngineCommand::Backspace),
                ctrl_j: Some(EngineCommand::Commit),
                ctrl_m: Some(EngineCommand::Commit),
                ctrl_n: Some(EngineCommand::NextCandidate),
                ctrl_p: Some(EngineCommand::PrevCandidate),
            },
        }
    }
}

impl Default for CtrlKeyConfig {
    fn default() -> Self {
        Self::from_preset(&KeybindPreset::None)
    }
}
```

**動作確認:**
- `cargo test` でプリセットとデフォルト値のテスト（7.5.2）がパスすること
- `cargo clippy` がクリーンであること

### 7.5.2 テストの追加

TDD に従い、まずテストを書いてから実装を行う。

#### プリセットのテスト

- [ ] `preset_none_all_disabled` — `None` プリセットで全キーが無効であること
- [ ] `preset_minimal_only_safe_keys` — `Minimal` で `Ctrl+J/G/M` のみ有効、`Ctrl+N/P/H` は無効
- [ ] `preset_emacs_all_enabled` — `Emacs` で全6キーが有効であること
- [ ] `default_is_none_preset` — `Default` が `None` プリセットと一致すること

#### map_key のテスト（Emacs プリセット）

- [ ] `emacs_ctrl_j_commits` — `Ctrl+J` が `EngineCommand::Commit` を返す
- [ ] `emacs_ctrl_g_cancels` — `Ctrl+G` が `EngineCommand::Cancel` を返す
- [ ] `emacs_ctrl_n_next_candidate` — `Ctrl+N` が `EngineCommand::NextCandidate` を返す
- [ ] `emacs_ctrl_p_prev_candidate` — `Ctrl+P` が `EngineCommand::PrevCandidate` を返す
- [ ] `emacs_ctrl_h_backspace` — `Ctrl+H` が `EngineCommand::Backspace` を返す
- [ ] `emacs_ctrl_m_commits` — `Ctrl+M` が `EngineCommand::Commit` を返す

#### map_key のテスト（Minimal プリセット）

- [ ] `minimal_ctrl_j_commits` — `Ctrl+J` が `Commit` を返す
- [ ] `minimal_ctrl_n_returns_none` — `Ctrl+N` が `None` を返す（OS に委ねる）
- [ ] `minimal_ctrl_p_returns_none` — `Ctrl+P` が `None` を返す
- [ ] `minimal_ctrl_h_returns_none` — `Ctrl+H` が `None` を返す

#### map_key のテスト（None プリセット）

- [ ] `none_preset_ctrl_j_returns_none` — `Ctrl+J` が `None` を返す

#### map_key のテスト（共通）

- [ ] `ctrl_other_returns_none` — 未設定キー（`Ctrl+A` 等）が `None` を返す
- [ ] `ctrl_alt_returns_none` — `Ctrl+Alt+J` が `None` を返す

#### カスタム設定のテスト（プリセット + 個別上書き）

- [ ] `emacs_override_ctrl_n_none` — Emacs プリセットで `ctrl_n` を `None` に上書き可能
- [ ] `none_override_ctrl_j_commit` — None プリセットで `ctrl_j` を `Commit` に個別追加可能
- [ ] `minimal_override_ctrl_h_backspace` — Minimal プリセットで `ctrl_h` を `Backspace` に追加可能

#### TOML パースのテスト

- [ ] `parse_preset_emacs` — `keybind_preset = "emacs"` が正しくパースされること
- [ ] `parse_preset_minimal` — `keybind_preset = "minimal"` が正しくパースされること
- [ ] `parse_preset_none` — `keybind_preset = "none"` が正しくパースされること
- [ ] `parse_preset_missing_defaults_to_none` — `keybind_preset` 省略時は `"none"` になること
- [ ] `parse_preset_invalid_errors` — 不正なプリセット名がエラーになること
- [ ] `parse_keybind_override` — プリセット + `[keybind]` セクションの上書きが正しいこと
- [ ] `parse_keybind_none_value` — `"none"` が `None` にパースされること
- [ ] `parse_keybind_invalid_command_errors` — 不正なコマンド名がエラーになること

**動作確認:**
- Red: 新規テストを追加し `cargo test` で失敗することを確認
- Green: 実装後 `cargo test` で全テストがパスすること

### 7.5.3 map_key の設定対応

`map_key` に `CtrlKeyConfig` を受け取る引数を追加し、設定に基づいて Ctrl+キーを処理する。

- [ ] `map_key` のシグネチャを変更: `ctrl_config: &CtrlKeyConfig` 引数を追加
- [ ] Ctrl が押されている場合、`CtrlKeyConfig` から対応するコマンドを検索して返す
- [ ] Ctrl+Alt の場合は引き続き `None` を返す（AltGr 対策）
- [ ] Alt のみの場合も引き続き `None` を返す
- [ ] 既存の呼び出し箇所を更新する（`text_service.rs`, `main.rs`）

```rust
/// 仮想キーコードと修飾キー状態を EngineCommand に変換する。
pub fn map_key(
    vk: u16,
    modifiers: &Modifiers,
    ime_on: bool,
    ctrl_config: &CtrlKeyConfig,
) -> Option<EngineCommand> {
    if !ime_on {
        return None;
    }

    // Alt は常に OS に委ねる
    if modifiers.alt {
        return None;
    }

    // Ctrl+キー: 設定に基づくマッピング
    if modifiers.ctrl {
        return map_ctrl_key(vk, ctrl_config);
    }

    // 通常キー（既存ロジック）
    match vk {
        // ...
    }
}

fn map_ctrl_key(vk: u16, config: &CtrlKeyConfig) -> Option<EngineCommand> {
    match vk {
        VK_G => config.ctrl_g.clone(),
        VK_H => config.ctrl_h.clone(),
        VK_J => config.ctrl_j.clone(),
        VK_M => config.ctrl_m.clone(),
        VK_N => config.ctrl_n.clone(),
        VK_P => config.ctrl_p.clone(),
        _ => None,
    }
}
```

**動作確認:**
- `cargo test` で全テストがパスすること
- `cargo clippy` と `cargo fmt -- --check` がクリーンであること

### 7.5.4 TOML パース対応

Phase 6 の `Config` 構造体に `keybind_preset` と `keybind` を追加し、
プリセット + 個別上書きのパースを実装する。

- [ ] `Config` に `pub keybind_preset: KeybindPreset` フィールドを追加
- [ ] `Config` に `pub keybind: CtrlKeyConfig` フィールドを追加（プリセット適用後の最終結果）
- [ ] パース処理: まず `keybind_preset` からベースを生成し、`[keybind]` セクションで上書き
- [ ] コマンド名文字列 ↔ `EngineCommand` の変換ヘルパーを用意

```rust
// プリセット名 → KeybindPreset の変換
fn parse_preset(value: &str) -> Result<KeybindPreset, ConfigError> {
    match value {
        "none" => Ok(KeybindPreset::None),
        "minimal" => Ok(KeybindPreset::Minimal),
        "emacs" => Ok(KeybindPreset::Emacs),
        _ => Err(ConfigError::Parse(
            format!("不正なプリセット名: {value} (none, minimal, emacs のいずれか)")
        )),
    }
}

// コマンド名 → EngineCommand の変換
fn parse_command(value: &str) -> Result<Option<EngineCommand>, ConfigError> {
    match value {
        "commit" => Ok(Some(EngineCommand::Commit)),
        "cancel" => Ok(Some(EngineCommand::Cancel)),
        "next" => Ok(Some(EngineCommand::NextCandidate)),
        "prev" => Ok(Some(EngineCommand::PrevCandidate)),
        "backspace" => Ok(Some(EngineCommand::Backspace)),
        "convert" => Ok(Some(EngineCommand::Convert)),
        "none" => Ok(None),
        _ => Err(ConfigError::Parse(
            format!("不正なコマンド名: {value}")
        )),
    }
}

// パース処理のフロー
fn parse_keybind(text: &str) -> Result<CtrlKeyConfig, ConfigError> {
    // 1. keybind_preset をパース（デフォルト: "none"）
    let preset = parse_preset(/* ... */)?;
    let mut config = CtrlKeyConfig::from_preset(&preset);

    // 2. [keybind] セクションで個別上書き
    // ctrl_j = "cancel" → config.ctrl_j = Some(EngineCommand::Cancel)
    // ctrl_n = "none"   → config.ctrl_n = None

    Ok(config)
}
```

**動作確認:**
- `cargo test` で TOML パーステストがパスすること
- プリセット + 上書きの組み合わせが正しく動作すること

### 7.5.5 統合テスト（エンジン経由の動作確認）

`key_mapping` のユニットテストに加え、エンジン経由で Emacs キーバインドが正しく動作することを確認する。

#### Emacs プリセットでの動作

- [ ] Composing 状態で `Ctrl+J` → ひらがな確定 → Direct
- [ ] Converting 状態で `Ctrl+J` → 候補確定 → Direct
- [ ] Composing 状態で `Ctrl+G` → 入力破棄 → Direct
- [ ] Converting 状態で `Ctrl+G` → Composing に戻る
- [ ] Converting 状態で `Ctrl+N` / `Ctrl+P` → 候補移動
- [ ] Composing 状態で `Ctrl+H` → 1文字削除

#### None プリセットでの動作

- [ ] Composing 状態で `Ctrl+J` → `None`（未処理、OS に委ねる）

#### カスタム設定での動作

- [ ] Emacs プリセット + `ctrl_n = "none"` で `Ctrl+N` → `None` になること

**動作確認:**
- `cargo test` で統合テストがパスすること
- 既存の全テストが壊れていないこと

### 7.5.6 text_service.rs の Ctrl キー状態取得対応（Windows）

TSF 連携部分で Ctrl キーの押下状態を正しく `Modifiers` に反映させ、
設定から `CtrlKeyConfig` を読み込んで `map_key` に渡す。

- [ ] `text_service.rs` の `OnKeyDown` で `GetKeyState(VK_CONTROL)` を参照
- [ ] `Modifiers { ctrl: true, .. }` として `map_key` に渡す
- [ ] `Config` から `CtrlKeyConfig` を取得し `map_key` に渡す
- [ ] 手動テスト: `keybind_preset = "emacs"` で Ctrl+J による確定動作を確認
- [ ] 手動テスト: `config.toml` の `[keybind]` を変更して割り当てが変わることを確認

**動作確認:**
- `cargo build --release` がエラーなく完了すること（Windows 環境）
- `keybind_preset = "emacs"` で Ctrl+J 確定、Ctrl+G キャンセルが動作すること（手動確認）
- `keybind_preset = "none"` で Ctrl+キーが OS に委ねられることを確認（手動確認）
- `[keybind]` セクションの個別上書きが IME 再起動後に反映されること（手動確認）

## 実装順序

| 順序 | タスク | 依存 | 規模 |
|-----|--------|------|------|
| 1 | 7.5.2 テストの追加 (Red) | なし | 小 |
| 2 | 7.5.1 CtrlKeyConfig + プリセット定義 (Green) | 7.5.2 | 小 |
| 3 | 7.5.3 map_key の設定対応 (Green) | 7.5.1, 7.5.2 | 中 |
| 4 | 7.5.4 TOML パース対応 (Green) | 7.5.1 | 中 |
| 5 | 7.5.5 統合テスト | 7.5.3 | 小 |
| 6 | 7.5.6 TSF 連携（Windows のみ） | 7.5.3, 7.5.4 | 中 |

## 完了条件

- プリセット (`"none"`, `"minimal"`, `"emacs"`) でキーバインドのベースを選択できること
- デフォルトプリセットが `"none"`（全 Ctrl+キー無効）であること
- `"emacs"` プリセットで `Ctrl+J/G/N/P/H/M` が全て有効になること
- `"minimal"` プリセットで競合の少ない `Ctrl+J/G/M` のみ有効になること
- `[keybind]` セクションでプリセットの個別キーを上書きできること
- `"none"` を指定することで特定の Ctrl+キーの割り当てを無効化できること
- 既存のキーバインド（Enter, Escape, 矢印キー等）が引き続き動作すること
- Ctrl+未設定キーは `None` を返し、OS に処理を委ねること
- `cargo test` で全テストがパスすること
- `cargo clippy` と `cargo fmt -- --check` がクリーンであること

## ファイル構成 (変更対象)

```
src/
├── key_mapping.rs     # CtrlKeyConfig, KeybindPreset, map_key 設定対応（主な変更）
├── config.rs          # keybind_preset + [keybind] セクションのパース（Phase 6 と共同）
├── text_service.rs    # Ctrl キー状態の取得 + CtrlKeyConfig の受け渡し（Windows のみ）
├── engine.rs          # 変更なし（既存の EngineCommand をそのまま使用）
```

## 設定ファイル例

### デフォルト（Ctrl+キー割り当てなし）

```toml
[general]
keybind_preset = "none"
```

### Minimal プリセット（競合の少ないキーのみ）

```toml
[general]
keybind_preset = "minimal"
```

### Emacs プリセット（フルセット、OS ショートカットを上書き）

```toml
[general]
keybind_preset = "emacs"
```

### Emacs ベースで Ctrl+N/P だけ無効化

```toml
[general]
keybind_preset = "emacs"

[keybind]
ctrl_n = "none"
ctrl_p = "none"
```

### None ベースで Ctrl+J だけ有効化

```toml
[general]
keybind_preset = "none"

[keybind]
ctrl_j = "commit"
```

### 独自カスタマイズ（Ctrl+J/K で候補移動）

```toml
[general]
keybind_preset = "minimal"

[keybind]
ctrl_j = "next"
ctrl_k = "prev"          # ※ ctrl_k は将来拡張で対応
ctrl_m = "commit"
```

## 注意事項

- **EngineCommand の追加は不要。** 既存の `Commit`, `Cancel`, `NextCandidate`, `PrevCandidate`, `Backspace`, `Convert` をそのまま使う。新しいキーバインドは「入力経路の追加」であり、エンジンの変更は不要。
- **デフォルトは `"none"` プリセット。** 既存ユーザーへの影響を避けるため、明示的に有効化しない限り Ctrl+キーは全て OS に委ねる。
- **Alt+キーは対象外。** `modifiers.alt == true` の場合は引き続き `None` を返す。Alt キーの組み合わせは OS のメニューショートカットと競合するため。
- **Ctrl+Alt の組み合わせも対象外。** 一部の言語レイアウトでは `Ctrl+Alt` が `AltGr` として使われるため、`ctrl && alt` が同時に `true` の場合は `None` を返す。
- **Ctrl+C は対象外。** OS のコピー操作と競合するため、`ctrl_c` フィールドは設けない。
- **`InsertChar` は割り当て不可。** Ctrl+キーに文字入力を割り当てることは意味がないため、設定値に `"insert_char"` は含めない。
- **将来の拡張性:** カーソル移動（`Ctrl+A/E/F/B`）は `EngineCommand` の拡張が必要なため、Composing 状態でのカーソル移動機能と合わせて別フェーズで実装する。その際、`CtrlKeyConfig` にフィールドを追加し、プリセットにもカーソル移動用のバリエーションを追加すれば対応可能。
- **Phase 6 との連携:** `config.rs` の `Config` 構造体に `keybind_preset: KeybindPreset` と `keybind: CtrlKeyConfig` を追加する。Phase 6 未実装の場合は `CtrlKeyConfig::default()`（= `"none"` プリセット）をハードコードして使用し、Phase 6 実装時に `Config` から取得するよう切り替える。
