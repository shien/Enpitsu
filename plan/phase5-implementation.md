# Phase 5 実装計画: 候補ウィンドウ UI

## 概要

Phase 4（TSF 連携、143テスト全パス）の上に、変換候補をポップアップウィンドウで表示する機能を構築する。
候補ウィンドウは Win32 API で実装し、キーボードの数字キー (1-9) による直接選択機能もエンジンに追加する。

TDD サイクル（Red → Green → Refactor）を厳守する。
プラットフォーム非依存のロジック（`SelectCandidate` コマンド、数字キーマッピング）は
ユニットテストで検証し、Windows 固有の UI コード（`candidate_window.rs`）は
`#[cfg(windows)]` で分離して手動テストで検証する。

---

## アーキテクチャ設計

### レイヤー構成

```
┌─────────────────────────────────────────────────┐
│  Windows アプリケーション (メモ帳等)                │
└───────────────┬─────────────────────────────────┘
                │ TSF API
┌───────────────▼─────────────────────────────────┐
│  TextService (text_service.rs)                    │
│  ├── ITfKeyEventSink (キーイベント処理)              │
│  ├── Composition 管理 (確定/未確定テキスト)           │
│  └── CandidateWindow 制御 ← NEW                  │
├─────────────────────────────────────────────────┤
│  CandidateWindow (candidate_window.rs) ← NEW      │
│  Win32 ポップアップウィンドウ                         │
│  ├── ウィンドウ作成・表示・非表示                      │
│  ├── 候補リスト描画（番号付き）                       │
│  └── カーソル位置追従                               │
├─────────────────────────────────────────────────┤
│  KeyMapping (key_mapping.rs) ← EDIT               │
│  VirtualKey + 修飾キー + エンジン状態                │
│  → EngineCommand (SelectCandidate 追加)           │
├─────────────────────────────────────────────────┤
│  ConversionEngine (engine.rs) ← EDIT              │
│  SelectCandidate(usize) コマンド追加                │
│  CandidateList::select_at(usize) 追加             │
├─────────────────────────────────────────────────┤
│  CandidateList (candidate.rs) ← EDIT              │
│  select_at(index) メソッド追加                      │
└─────────────────────────────────────────────────┘
```

### プラットフォーム分離方針

| コード | プラットフォーム | テスト方法 |
|--------|----------------|-----------|
| `candidate.rs` (select_at 追加) | 非依存 | `cargo test` (TDD) |
| `engine.rs` (SelectCandidate 追加) | 非依存 | `cargo test` (TDD) |
| `key_mapping.rs` (数字キー追加) | 非依存 | `cargo test` (TDD) |
| `candidate_window.rs` (NEW) | Windows 専用 | Windows での手動テスト |
| `text_service.rs` (候補ウィンドウ連携) | Windows 専用 | Windows での手動テスト |

### ファイル構成

```
src/
├── candidate.rs        # EDIT: select_at(index) メソッド追加
├── engine.rs           # EDIT: SelectCandidate コマンド追加
├── key_mapping.rs      # EDIT: 数字キーマッピング追加 (エンジン状態対応)
├── candidate_window.rs # NEW: 候補ウィンドウ UI (#[cfg(windows)])
├── text_service.rs     # EDIT: 候補ウィンドウの表示制御追加
├── lib.rs              # EDIT: pub mod candidate_window; 追加
├── class_factory.rs    # 既存（変更なし）
├── registry.rs         # 既存（変更なし）
├── guids.rs            # 既存（変更なし）
├── dictionary.rs       # 既存（変更なし）
├── input_state.rs      # 既存（変更なし）
├── romaji.rs           # 既存（変更なし）
├── katakana.rs         # 既存（変更なし）
└── main.rs             # 既存（変更なし）
```

---

## ステップ 1: CandidateList に select_at を追加 (TDD)

数字キーによる候補の直接選択を実現するため、`CandidateList` にインデックス指定で
候補を選択するメソッドを追加する。

### 1-1. Red: select_at のテストを書く

`src/candidate.rs` のテストに以下を追加:

```rust
// === select_at ===

#[test]
fn select_at_valid_index() {
    let cl = CandidateList::new(vec![
        "漢字".to_string(),
        "感じ".to_string(),
        "幹事".to_string(),
    ]);
    assert_eq!(cl.select_at(1), Some("感じ".to_string()));
}

#[test]
fn select_at_first() {
    let cl = CandidateList::new(vec!["漢字".to_string(), "感じ".to_string()]);
    assert_eq!(cl.select_at(0), Some("漢字".to_string()));
}

#[test]
fn select_at_last() {
    let cl = CandidateList::new(vec!["漢字".to_string(), "感じ".to_string()]);
    assert_eq!(cl.select_at(1), Some("感じ".to_string()));
}

#[test]
fn select_at_out_of_bounds() {
    let cl = CandidateList::new(vec!["漢字".to_string()]);
    assert_eq!(cl.select_at(5), None);
}

#[test]
fn select_at_on_empty() {
    let cl = CandidateList::new(vec![]);
    assert_eq!(cl.select_at(0), None);
}
```

**動作確認:** `cargo test` で新しいテストが**失敗する**こと（Red）

### 1-2. Green: select_at を実装

```rust
/// 指定インデックスの候補を返す。範囲外なら None。
pub fn select_at(&self, index: usize) -> Option<String> {
    self.candidates.get(index).cloned()
}
```

**動作確認:** `cargo test` で全テスト（既存 143 + 新規 5 = 148 テスト）がパスすること（Green）

### 1-3. Refactor

**動作確認:** `cargo test` 全パス、`cargo clippy` 警告なし、`cargo fmt -- --check` 差分なし

---

## ステップ 2: エンジンに SelectCandidate コマンドを追加 (TDD)

数字キー (1-9) で候補を直接選択・確定するコマンドをエンジンに追加する。

### 2-1. Red: SelectCandidate のテストを書く

`src/engine.rs` のテストに以下を追加:

```rust
// === SelectCandidate ===

#[test]
fn select_candidate_in_converting() {
    let mut engine = test_engine();
    for ch in "kanji".chars() {
        engine.process(EngineCommand::InsertChar(ch));
    }
    engine.process(EngineCommand::Convert);
    // index=1 → "感じ" を直接選択
    let output = engine.process(EngineCommand::SelectCandidate(1));
    assert_eq!(output.committed, "感じ");
    assert_eq!(engine.state(), EngineState::Direct);
}

#[test]
fn select_candidate_first() {
    let mut engine = test_engine();
    for ch in "kanji".chars() {
        engine.process(EngineCommand::InsertChar(ch));
    }
    engine.process(EngineCommand::Convert);
    let output = engine.process(EngineCommand::SelectCandidate(0));
    assert_eq!(output.committed, "漢字");
    assert_eq!(engine.state(), EngineState::Direct);
}

#[test]
fn select_candidate_out_of_bounds() {
    let mut engine = test_engine();
    for ch in "kanji".chars() {
        engine.process(EngineCommand::InsertChar(ch));
    }
    engine.process(EngineCommand::Convert);
    // 範囲外 → Converting 状態を維持（何もしない）
    let output = engine.process(EngineCommand::SelectCandidate(99));
    assert_eq!(engine.state(), EngineState::Converting);
    assert!(output.committed.is_empty());
}

#[test]
fn select_candidate_in_direct_is_noop() {
    let mut engine = test_engine();
    let output = engine.process(EngineCommand::SelectCandidate(0));
    assert_eq!(engine.state(), EngineState::Direct);
    assert!(output.committed.is_empty());
}

#[test]
fn select_candidate_in_composing_is_noop() {
    let mut engine = test_engine();
    engine.process(EngineCommand::InsertChar('k'));
    let output = engine.process(EngineCommand::SelectCandidate(0));
    assert_eq!(engine.state(), EngineState::Composing);
    assert!(output.committed.is_empty());
}
```

公開 API への追加:

```rust
pub enum EngineCommand {
    // ... 既存のコマンド ...
    /// 指定インデックスの候補を直接選択・確定 (数字キー)
    SelectCandidate(usize),
}
```

**動作確認:** `cargo test` で新しいテストが**失敗する**こと（Red）

### 2-2. Green: SelectCandidate を実装

`engine.rs` の `process()` メソッドに以下を追加:

```rust
// Converting 状態に追加
(EngineState::Converting, EngineCommand::SelectCandidate(idx)) => {
    let committed = self
        .candidates
        .as_ref()
        .and_then(|cl| cl.select_at(*idx));
    match committed {
        Some(text) => {
            self.candidates = None;
            self.input.reset();
            self.state = EngineState::Direct;
            EngineOutput {
                committed: text,
                display: String::new(),
                candidates: None,
                candidate_index: None,
            }
        }
        None => {
            // 範囲外のインデックス → 何もしない
            self.converting_output()
        }
    }
}
```

Direct / Composing 状態では既存の catch-all パターンが処理するため追加不要:
- `(EngineState::Direct, _) => self.empty_output()`
- `(EngineState::Composing, _) => self.composing_output()`

**動作確認:** `cargo test` で全テスト（148 + 新規 5 = 153 テスト）がパスすること（Green）

### 2-3. Refactor

**動作確認:** `cargo test` 全パス、`cargo clippy` 警告なし、`cargo fmt -- --check` 差分なし

---

## ステップ 3: キーマッピングに数字キーと状態対応を追加 (TDD)

Converting 状態のときのみ数字キー (1-9) を `SelectCandidate` にマッピングする。
`map_key` のシグネチャにエンジン状態パラメータを追加する。

### 3-1. 設計方針

現状の `map_key(vk, modifiers, ime_on)` に `is_converting: bool` パラメータを追加する。

```rust
pub fn map_key(
    vk: u16,
    modifiers: &Modifiers,
    ime_on: bool,
    is_converting: bool,
) -> Option<EngineCommand>
```

- `is_converting` が `true` かつ VK_1..=VK_9 → `SelectCandidate(index)` (1キー→index 0、9キー→index 8)
- `is_converting` が `false` のときは数字キーは `None`（アプリに素通し）
- 既存のキーマッピングは変更なし

### 3-2. Red: 数字キーマッピングのテストを書く

`src/key_mapping.rs` のテストに以下を追加:

```rust
// === VK 定数 ===
// VK_1..VK_9 の定数を追加（VK_0 は既存）
pub const VK_1: u16 = 0x31;
pub const VK_9: u16 = 0x39;
```

```rust
// === 数字キーによる候補選択 ===

#[test]
fn number_key_1_selects_candidate_when_converting() {
    let cmd = map_key(VK_1, &Modifiers::none(), true, true);
    assert_eq!(cmd, Some(EngineCommand::SelectCandidate(0)));
}

#[test]
fn number_key_9_selects_candidate_when_converting() {
    let cmd = map_key(VK_9, &Modifiers::none(), true, true);
    assert_eq!(cmd, Some(EngineCommand::SelectCandidate(8)));
}

#[test]
fn number_key_5_selects_candidate_when_converting() {
    let cmd = map_key(0x35, &Modifiers::none(), true, true);  // VK_5 = 0x35
    assert_eq!(cmd, Some(EngineCommand::SelectCandidate(4)));
}

#[test]
fn number_key_not_converting_returns_none() {
    let cmd = map_key(VK_1, &Modifiers::none(), true, false);
    assert_eq!(cmd, None);
}

#[test]
fn number_key_0_always_returns_none() {
    // 0キーは候補選択に使わない（1-9のみ）
    let cmd = map_key(VK_0, &Modifiers::none(), true, true);
    assert_eq!(cmd, None);
}
```

既存テストも `is_converting: false` の第4引数を追加して更新する必要がある。

**動作確認:** `cargo test` で新しいテストが**失敗する**こと（Red）

### 3-3. Green: map_key にパラメータ追加と数字キー処理を実装

```rust
pub fn map_key(vk: u16, modifiers: &Modifiers, ime_on: bool, is_converting: bool) -> Option<EngineCommand> {
    if !ime_on {
        return None;
    }

    if modifiers.ctrl || modifiers.alt {
        return None;
    }

    // Converting 状態で数字キー 1-9 → SelectCandidate
    if is_converting {
        if let VK_1..=VK_9 = vk {
            let index = (vk - VK_1) as usize;
            return Some(EngineCommand::SelectCandidate(index));
        }
    }

    match vk {
        VK_A..=VK_Z => {
            // ... 既存のまま ...
        }
        // ... 既存のまま ...
    }
}
```

### 3-4. 呼び出し元の更新

`text_service.rs` の `OnTestKeyDown` / `OnKeyDown` で `map_key` 呼び出しを更新:

```rust
// engine の状態を取得して map_key に渡す
let engine = self.engine.lock().unwrap();
let is_converting = engine.state() == EngineState::Converting;
let command = key_mapping::map_key(vk, &modifiers, ime_on, is_converting);
```

**動作確認:** `cargo test` で全テスト（153 + 新規 5 = 158 テスト）がパスすること（Green）

### 3-5. Refactor

**動作確認:** `cargo test` 全パス、`cargo clippy` 警告なし、`cargo fmt -- --check` 差分なし

---

## ステップ 4: Cargo.toml の更新

候補ウィンドウの作成と描画に必要な Windows features を追加する。

### 4-1. windows crate の features に追加

```toml
[target.'cfg(windows)'.dependencies]
windows = { version = "0.58", features = [
    "implement",
    "Win32_Foundation",
    "Win32_Graphics_Gdi",                  # NEW: GDI 描画
    "Win32_System_Com",
    "Win32_System_LibraryLoader",
    "Win32_System_Registry",
    "Win32_UI_HiDpi",                      # NEW: DPI スケーリング
    "Win32_UI_Input_KeyboardAndMouse",
    "Win32_UI_TextServices",
    "Win32_UI_WindowsAndMessaging",        # NEW: ウィンドウ作成
] }
```

### 4-2. lib.rs にモジュール追加

```rust
#[cfg(windows)]
pub mod candidate_window;
```

**動作確認:**
- `cargo build` がエラーなく完了すること
- `cargo test` で既存の 158 テストが全パスすること

---

## ステップ 5: 候補ウィンドウの基本実装 (Windows 専用)

`src/candidate_window.rs` に Win32 API を使った候補ウィンドウを実装する。

### 5-1. ウィンドウクラス登録とウィンドウ作成

```rust
//! 候補ウィンドウ UI。
//!
//! 変換候補をポップアップウィンドウで表示する。
//! Win32 API を使った独自ウィンドウ実装。

use std::sync::Mutex;

use windows::Win32::Foundation::*;
use windows::Win32::Graphics::Gdi::*;
use windows::Win32::UI::WindowsAndMessaging::*;
use windows::core::*;

const WINDOW_CLASS_NAME: &str = "enpitsu_candidate";

/// 候補ウィンドウの状態。
struct CandidateWindowState {
    candidates: Vec<String>,
    selected_index: usize,
}

/// 候補ウィンドウ。
pub struct CandidateWindow {
    hwnd: HWND,
    state: Mutex<CandidateWindowState>,
}
```

設計方針:
- **WS_POPUP | WS_BORDER**: 枠なしポップアップ + 細い境界線
- **WS_EX_TOOLWINDOW | WS_EX_TOPMOST | WS_EX_NOACTIVATE**: タスクバー非表示、最前面、フォーカス奪取なし
- **ウィンドウクラス**: `enpitsu_candidate` という名前で登録
- **WndProc**: WM_PAINT で候補リストを描画

### 5-2. CandidateWindow の公開 API

```rust
impl CandidateWindow {
    /// 候補ウィンドウを作成する。非表示状態で初期化。
    pub fn new(hinstance: HMODULE) -> Result<Self>;

    /// 候補リストを更新し、指定位置にウィンドウを表示する。
    ///
    /// - `candidates`: 表示する候補のリスト
    /// - `selected_index`: 現在選択中のインデックス
    /// - `x`, `y`: 表示位置（スクリーン座標）
    pub fn show(&self, candidates: &[String], selected_index: usize, x: i32, y: i32);

    /// 候補の選択状態を更新する（ウィンドウは表示したまま）。
    pub fn update_selection(&self, selected_index: usize);

    /// ウィンドウを非表示にする。
    pub fn hide(&self);

    /// ウィンドウが表示中かどうか。
    pub fn is_visible(&self) -> bool;

    /// ウィンドウを破棄する。
    pub fn destroy(&self);
}
```

### 5-3. WM_PAINT での描画

```
┌──────────────┐
│ 1. 漢字   ◀  │  ← 選択中の候補をハイライト
│ 2. 感じ      │
│ 3. 幹事      │
└──────────────┘
```

描画方針:
- `BeginPaint` / `EndPaint` で描画コンテキストを取得
- `CreateFontW` で日本語フォント（Meiryo UI）を作成
- 各候補を `TextOutW` で番号付きで描画
- 選択中の行は背景色を変更（`SetBkColor` / `SetTextColor`）
- ウィンドウサイズは候補数とテキスト幅に基づいて自動計算（`GetTextExtentPoint32W`）

**動作確認:**
- Windows 環境で `cargo build` がエラーなく完了すること
- Linux 環境で `cargo build` と `cargo test` が通ること（`#[cfg(windows)]` により無視）

---

## ステップ 6: TextService と候補ウィンドウの統合 (Windows 専用)

### 6-1. TextService に CandidateWindow を追加

```rust
#[implement(ITfTextInputProcessorEx, ITfTextInputProcessor, ITfKeyEventSink)]
pub struct TextService {
    // ... 既存フィールド ...
    candidate_window: Mutex<Option<CandidateWindow>>,  // NEW
}
```

### 6-2. ActivateEx で候補ウィンドウを作成

```rust
fn ActivateEx(&self, ptim: Option<&ITfThreadMgr>, tid: u32, _flags: u32) -> Result<()> {
    // ... 既存の処理 ...

    // 候補ウィンドウを作成
    let hinstance = crate::dll_exports::dll_instance();
    if let Ok(window) = CandidateWindow::new(hinstance) {
        *self.candidate_window.lock().unwrap() = Some(window);
    }

    Ok(())
}
```

### 6-3. Deactivate で候補ウィンドウを破棄

```rust
fn Deactivate(&self) -> Result<()> {
    // ... 既存の処理 ...

    // 候補ウィンドウを破棄
    if let Some(window) = self.candidate_window.lock().unwrap().take() {
        window.destroy();
    }

    Ok(())
}
```

### 6-4. OnKeyDown で候補ウィンドウの表示制御

`OnKeyDown` で `engine.process()` の結果に基づいて候補ウィンドウを制御する:

```rust
fn OnKeyDown(&self, pic: Option<&ITfContext>, wparam: WPARAM, _lparam: LPARAM) -> Result<BOOL> {
    // ... 既存のキーマッピング + エンジン処理 ...

    // Composition の更新
    if let Some(context) = pic {
        self.update_composition(context, &output)?;
    }

    // 候補ウィンドウの制御
    self.update_candidate_window(pic, &output)?;

    Ok(TRUE)
}
```

### 6-5. update_candidate_window メソッド

```rust
impl TextService {
    fn update_candidate_window(
        &self,
        context: Option<&ITfContext>,
        output: &EngineOutput,
    ) -> Result<()> {
        let window_guard = self.candidate_window.lock().unwrap();
        let Some(window) = window_guard.as_ref() else {
            return Ok(());
        };

        match (&output.candidates, &output.candidate_index) {
            (Some(candidates), Some(index)) => {
                if !candidates.is_empty() {
                    // カーソル位置を取得して候補ウィンドウを表示
                    let (x, y) = self.get_composition_position(context)?;
                    window.show(candidates, *index, x, y);
                } else {
                    window.hide();
                }
            }
            _ => {
                // 候補がない（確定/キャンセル後）→ 非表示
                if window.is_visible() {
                    window.hide();
                }
            }
        }

        Ok(())
    }
}
```

### 6-6. カーソル位置の取得

TSF の `ITfContextView::GetTextExt()` を使って、Composition 範囲のスクリーン座標を取得する。

```rust
fn get_composition_position(&self, context: Option<&ITfContext>) -> Result<(i32, i32)> {
    // ITfContextView を取得
    // GetTextExt() で Composition 範囲の RECT を取得
    // RECT の左下をウィンドウ表示位置として返す
    // 取得できない場合はデフォルト位置 (0, 0) を返す
}
```

**動作確認:**
- Linux 環境: `cargo build` と `cargo test` が通ること
- Windows 環境: メモ帳で変換時に候補ウィンドウがカーソル付近に表示されること
- 候補選択中に数字キーで直接選択できること
- 確定/キャンセル時に候補ウィンドウが消えること

---

## ステップ 7: ウィンドウ表示制御の詳細 (Windows 専用)

### 7-1. 表示タイミングの制御

| イベント | 候補ウィンドウの動作 |
|---------|------------------|
| Convert (Space) → 候補あり | ウィンドウ表示 |
| NextCandidate (↓ / Space) | 選択ハイライト更新 |
| PrevCandidate (↑) | 選択ハイライト更新 |
| SelectCandidate (1-9) | ウィンドウ非表示（確定） |
| Commit (Enter) | ウィンドウ非表示（確定） |
| Cancel (Escape) | ウィンドウ非表示（キャンセル） |
| Convert → 候補なし | ウィンドウ非表示（ひらがな確定） |

### 7-2. フォーカス処理

`ITfKeyEventSink::OnSetFocus` で、アプリケーションがフォーカスを失った場合に
候補ウィンドウを非表示にする。

```rust
fn OnSetFocus(&self, fforeground: BOOL) -> Result<()> {
    if fforeground == FALSE {
        // フォーカスを失った → 候補ウィンドウを非表示
        let window_guard = self.candidate_window.lock().unwrap();
        if let Some(window) = window_guard.as_ref() {
            window.hide();
        }
    }
    Ok(())
}
```

**動作確認:**
- 変換開始→候補表示、確定→非表示、Escape→非表示の各操作を手動確認
- アプリ切り替え時にウィンドウが残らないことを手動確認

---

## ステップ 8: 見た目の調整 (Windows 専用)

### 8-1. 日本語フォント対応

```rust
fn create_font(height: i32) -> HFONT {
    unsafe {
        CreateFontW(
            height,         // 高さ
            0,              // 幅（自動）
            0, 0,           // 角度
            FW_NORMAL.0 as i32,
            0, 0, 0,        // italic, underline, strikeout
            SHIFTJIS_CHARSET.0 as u32,
            OUT_DEFAULT_PRECIS.0 as u32,
            CLIP_DEFAULT_PRECIS.0 as u32,
            CLEARTYPE_QUALITY.0 as u32,
            (FF_MODERN.0 | FIXED_PITCH.0) as u32,
            w!("Meiryo UI"),
        )
    }
}
```

### 8-2. ウィンドウサイズの自動調整

候補テキストの最大幅と候補数に基づいてウィンドウサイズを計算する。

```rust
fn calculate_window_size(
    hdc: HDC,
    candidates: &[String],
    font_height: i32,
) -> (i32, i32) {
    let padding = 4;
    let line_height = font_height + padding;

    // 最大テキスト幅を計算
    let mut max_width = 0;
    for (i, candidate) in candidates.iter().enumerate() {
        let text = format!("{}. {}", i + 1, candidate);
        let wide: Vec<u16> = text.encode_utf16().collect();
        let mut size = SIZE::default();
        unsafe { GetTextExtentPoint32W(hdc, &wide, &mut size); }
        if size.cx > max_width {
            max_width = size.cx;
        }
    }

    let width = max_width + padding * 4;   // 左右パディング
    let height = line_height * candidates.len() as i32 + padding * 2;  // 上下パディング

    (width, height)
}
```

### 8-3. DPI スケーリング対応

`Win32_UI_HiDpi` の `GetDpiForWindow` を使用してスケーリングファクターを取得し、
フォントサイズとウィンドウサイズに適用する。

```rust
fn get_dpi_scale(hwnd: HWND) -> f32 {
    use windows::Win32::UI::HiDpi::GetDpiForWindow;
    let dpi = unsafe { GetDpiForWindow(hwnd) };
    if dpi == 0 { 1.0 } else { dpi as f32 / 96.0 }
}
```

- 基本フォントサイズ: 16px (96 DPI 基準)
- DPI スケール適用: `(16.0 * dpi_scale) as i32`
- パディング等も同様にスケール

**動作確認:**
- 高 DPI 環境（150%, 200%）でフォントとウィンドウが適切にスケーリングされることを手動確認
- 候補の文字数が多い場合にウィンドウ幅が自動調整されることを手動確認
- 候補が9個以上のときのウィンドウサイズを手動確認

---

## ステップ 9: 最終確認

### 9-1. Linux 環境での確認

```sh
cargo test               # 既存 143 + 新規 15 = 158 テストが全パス
cargo clippy             # 警告なし
cargo fmt -- --check     # フォーマット差分なし
cargo build              # エラーなし
```

### 9-2. Windows 環境での確認

```sh
cargo build --release    # DLL 生成
cargo test               # 全テストパス
```

手動テスト:

1. `regsvr32 target\release\enpitsu.dll` で登録
2. メモ帳でローマ字入力 → Space で変換
3. 候補ウィンドウがカーソル付近にポップアップ表示されること
4. ↓ / Space で次の候補、↑ で前の候補に移動、ハイライトが更新されること
5. 数字キー (1-9) で候補を直接選択し確定できること
6. Enter で選択中の候補を確定、候補ウィンドウが消えること
7. Escape でキャンセル、候補ウィンドウが消えること
8. 候補なしの場合（辞書にない語）は候補ウィンドウが表示されないこと
9. アプリ切り替え時に候補ウィンドウが残らないこと
10. 高 DPI 環境でフォントとウィンドウサイズが適切であること

### 9-3. CLAUDE.md の更新

- ファイル構成に `candidate_window.rs` を追記
- Cargo.toml の features 変更を反映

### 9-4. コミット

- テスト全パスを確認した上でコミット

---

## 追加テスト一覧（予定）

| # | テスト名 | モジュール | 分類 | 内容 |
|---|---------|-----------|------|------|
| 1 | `select_at_valid_index` | candidate | select_at | 有効インデックスで候補取得 |
| 2 | `select_at_first` | candidate | select_at | 先頭候補の取得 |
| 3 | `select_at_last` | candidate | select_at | 末尾候補の取得 |
| 4 | `select_at_out_of_bounds` | candidate | select_at | 範囲外で None |
| 5 | `select_at_on_empty` | candidate | select_at | 空リストで None |
| 6 | `select_candidate_in_converting` | engine | SelectCandidate | 候補直接選択・確定 |
| 7 | `select_candidate_first` | engine | SelectCandidate | 先頭候補の選択 |
| 8 | `select_candidate_out_of_bounds` | engine | SelectCandidate | 範囲外で状態維持 |
| 9 | `select_candidate_in_direct_is_noop` | engine | SelectCandidate | Direct では無視 |
| 10 | `select_candidate_in_composing_is_noop` | engine | SelectCandidate | Composing では無視 |
| 11 | `number_key_1_selects_candidate_when_converting` | key_mapping | 数字キー | 1キー → index 0 |
| 12 | `number_key_9_selects_candidate_when_converting` | key_mapping | 数字キー | 9キー → index 8 |
| 13 | `number_key_5_selects_candidate_when_converting` | key_mapping | 数字キー | 5キー → index 4 |
| 14 | `number_key_not_converting_returns_none` | key_mapping | 数字キー | 非変換時は None |
| 15 | `number_key_0_always_returns_none` | key_mapping | 数字キー | 0キーは None |

合計: 15 テスト追加（既存 143 + 新規 15 = 158 テスト）

Windows 専用コード（ステップ 5〜8）は `#[cfg(windows)]` で分離されており、
Linux 上のユニットテストには含まれない。Windows 環境での手動テストで検証する。

---

## 依存クレート（変更）

```toml
[target.'cfg(windows)'.dependencies]
windows = { version = "0.58", features = [
    "implement",
    "Win32_Foundation",
    "Win32_Graphics_Gdi",                  # NEW
    "Win32_System_Com",
    "Win32_System_LibraryLoader",
    "Win32_System_Registry",
    "Win32_UI_HiDpi",                      # NEW
    "Win32_UI_Input_KeyboardAndMouse",
    "Win32_UI_TextServices",
    "Win32_UI_WindowsAndMessaging",        # NEW
] }
```

---

## 実装上の注意事項

### Win32 ポップアップウィンドウ

- **WS_EX_NOACTIVATE**: フォーカスを奪わない。エディタのフォーカスを維持する。
- **WS_EX_TOPMOST**: 他のウィンドウの上に表示。
- **WS_EX_TOOLWINDOW**: タスクバーにアイコンが表示されない。
- ウィンドウクラスの登録は DLL ロード時（`DllMain` の `DLL_PROCESS_ATTACH`）ではなく、
  `ActivateEx` 時に行う（TSF のスレッドモデルに合わせる）。

### TSF カーソル位置の取得

- `ITfContextView::GetTextExt()` でテキスト範囲のスクリーン座標を取得する。
- Composition が未開始の場合は `ITfInsertAtSelection` で挿入位置を取得する。
- 座標取得に失敗した場合は、`GetCaretPos()` をフォールバックとして使用する。

### スレッド安全性

- 候補ウィンドウは TSF のスレッド（STA）で作成・操作する。
- `Mutex<CandidateWindowState>` でウィンドウの内部状態を保護する。
- Win32 ウィンドウメッセージ（WM_PAINT 等）は同じスレッドで処理されるため、
  ウィンドウ操作自体はスレッド安全。

### 段階的な動作確認

1. **Linux**: `cargo build` + `cargo test` が通ること（SelectCandidate + 数字キーテスト含む）
2. **Windows (ビルド)**: `cargo build` が通ること
3. **Windows (空ウィンドウ)**: 候補ウィンドウが表示されること（描画なし）
4. **Windows (描画)**: 候補リストが番号付きで表示されること
5. **Windows (操作)**: キーボード操作（↑↓、数字キー、Enter、Escape）が動作すること
6. **Windows (位置)**: カーソル付近に表示されること
7. **Windows (DPI)**: 高 DPI 環境でスケーリングが適切であること
