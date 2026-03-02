# Phase 4 実装計画: TSF (Text Services Framework) 連携

## 概要

Phase 3（変換エンジン、125テスト全パス）の上に、Windows の Text Services Framework (TSF) と連携する
COM DLL を構築する。ConversionEngine を Windows IME として動作させ、任意のアプリケーションで
ローマ字→ひらがな→漢字変換を使えるようにする。

TDD サイクル（Red → Green → Refactor）を厳守し、プラットフォーム非依存のロジックは
ユニットテストで検証する。Windows 固有の COM/TSF コードは `#[cfg(windows)]` で分離し、
手動テスト手順を明記する。

---

## アーキテクチャ設計

### レイヤー構成

```
┌─────────────────────────────────────────────────┐
│  Windows アプリケーション (メモ帳等)                │
└───────────────┬─────────────────────────────────┘
                │ TSF API
┌───────────────▼─────────────────────────────────┐
│  DLL エントリポイント (lib.rs)                      │
│  DllGetClassObject / DllRegisterServer ...        │
├─────────────────────────────────────────────────┤
│  ClassFactory (class_factory.rs)                  │
│  IClassFactory → TextService 生成                 │
├─────────────────────────────────────────────────┤
│  TextService (text_service.rs)                    │
│  ITfTextInputProcessorEx                          │
│  ├── ITfKeyEventSink (キーイベント処理)              │
│  └── Composition 管理 (確定/未確定テキスト)           │
├─────────────────────────────────────────────────┤
│  KeyMapping (key_mapping.rs)  ← TDD 可能          │
│  VirtualKey + 修飾キー → EngineCommand             │
├─────────────────────────────────────────────────┤
│  ConversionEngine (engine.rs) ← 既存              │
│  InputState / Dictionary / CandidateList          │
└─────────────────────────────────────────────────┘
```

### プラットフォーム分離方針

| コード | プラットフォーム | テスト方法 |
|--------|----------------|-----------|
| `key_mapping.rs` | 非依存 | `cargo test` (TDD) |
| `guids.rs` | 非依存 | 定数のみ |
| `text_service.rs` | Windows 専用 | Windows での手動テスト |
| `class_factory.rs` | Windows 専用 | Windows での手動テスト |
| `registry.rs` | Windows 専用 | Windows での手動テスト |
| DLL エクスポート | Windows 専用 | `regsvr32` で動作確認 |

- Windows 専用コードは `#[cfg(windows)]` で囲む
- `windows` crate は `[target.'cfg(windows)'.dependencies]` で追加
- Linux 上でも `cargo build` と `cargo test` が通る状態を維持

### ファイル構成

```
src/
├── lib.rs              # EDIT: DLL エクスポート + mod 宣言追加
├── key_mapping.rs      # NEW: VirtualKey → EngineCommand 変換 (TDD)
├── guids.rs            # NEW: CLSID, Profile GUID 定義
├── text_service.rs     # NEW: TextService + ITfTextInputProcessorEx (#[cfg(windows)])
├── class_factory.rs    # NEW: ClassFactory + IClassFactory (#[cfg(windows)])
├── registry.rs         # NEW: COM/TSF レジストリ登録 (#[cfg(windows)])
├── engine.rs           # 既存（変更なし）
├── candidate.rs        # 既存（変更なし）
├── dictionary.rs       # 既存（変更なし）
├── input_state.rs      # 既存（変更なし）
├── romaji.rs           # 既存（変更なし）
├── katakana.rs         # 既存（変更なし）
└── main.rs             # 既存（変更なし）
installer/
└── install.ps1         # NEW: インストール/アンインストール用 PowerShell スクリプト
```

---

## ステップ 1: プロジェクト設定

### 1-1. Cargo.toml の更新

```toml
[package]
name = "enpitsu"
version = "0.1.0"
edition = "2024"

[lib]
crate-type = ["cdylib", "rlib"]

[dependencies]
encoding_rs = "0.8"

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

- `crate-type = ["cdylib", "rlib"]`: DLL (cdylib) とライブラリ (rlib) の両方を生成
- `windows` crate は Windows ターゲットのみ依存に追加

### 1-2. 空ファイルの作成

- `src/guids.rs` を空ファイルとして作成
- `src/key_mapping.rs` を空ファイルとして作成
- `src/text_service.rs` を空ファイルとして作成
- `src/class_factory.rs` を空ファイルとして作成
- `src/registry.rs` を空ファイルとして作成

### 1-3. lib.rs にモジュール登録

```rust
pub mod guids;
pub mod key_mapping;

#[cfg(windows)]
pub mod class_factory;
#[cfg(windows)]
pub mod registry;
#[cfg(windows)]
pub mod text_service;
```

**動作確認:**
- `cargo build` がエラーなく完了すること
- `cargo test` で既存の 125 テストが全パスすること

---

## ステップ 2: GUID 定義

### 2-1. guids.rs に定数を定義

IME の識別に必要な GUID を定義する。

```rust
//! Enpitsu の CLSID および Profile GUID。

/// IME の CLSID (COM クラス識別子)。
/// `uuidgen` で生成した一意な値。
/// {B5F7E5D1-7A3C-4E8B-9F2A-1D6C8E4A3B7F}
pub const CLSID_TEXT_SERVICE: [u8; 16] = [
    0xB5, 0xF7, 0xE5, 0xD1, 0x7A, 0x3C, 0x4E, 0x8B,
    0x9F, 0x2A, 0x1D, 0x6C, 0x8E, 0x4A, 0x3B, 0x7F,
];

/// TSF プロファイル GUID (入力プロファイル識別子)。
/// {A2C9D4E6-5B8F-4A1C-8D3E-7F0B2C5A6E9D}
pub const GUID_PROFILE: [u8; 16] = [
    0xA2, 0xC9, 0xD4, 0xE6, 0x5B, 0x8F, 0x4A, 0x1C,
    0x8D, 0x3E, 0x7F, 0x0B, 0x2C, 0x5A, 0x6E, 0x9D,
];
```

Windows 専用の `GUID` 変換関数:

```rust
#[cfg(windows)]
use windows::core::GUID;

#[cfg(windows)]
pub fn clsid_text_service() -> GUID {
    GUID::from_bytes(&CLSID_TEXT_SERVICE)
}

#[cfg(windows)]
pub fn guid_profile() -> GUID {
    GUID::from_bytes(&GUID_PROFILE)
}
```

**動作確認:**
- `cargo build` がエラーなく完了すること
- `cargo test` で既存テストが全パスすること

---

## ステップ 3: KeyMapping モジュール (TDD)

キーボードの仮想キーコード（VK_*）と修飾キー状態から `EngineCommand` への変換を行う。
このモジュールはプラットフォーム非依存で、完全に TDD で実装する。

### 3-1. Red: KeyMapping のテストを書く

`src/key_mapping.rs` に以下のテストを追加:

```rust
#[cfg(test)]
mod tests {
    use super::*;
    use crate::engine::EngineCommand;

    // === アルファベットキー ===

    #[test]
    fn alphabet_key_lowercase() {
        // IME オン + 修飾なし → InsertChar('a')
        let cmd = map_key(VK_A, &Modifiers::none(), true);
        assert_eq!(cmd, Some(EngineCommand::InsertChar('a')));
    }

    #[test]
    fn alphabet_key_all_letters() {
        // A-Z 全てが対応する小文字に変換される
        for vk in VK_A..=VK_Z {
            let cmd = map_key(vk, &Modifiers::none(), true);
            let expected_char = (b'a' + (vk - VK_A) as u8) as char;
            assert_eq!(cmd, Some(EngineCommand::InsertChar(expected_char)));
        }
    }

    // === 特殊キー ===

    #[test]
    fn space_key_converts() {
        let cmd = map_key(VK_SPACE, &Modifiers::none(), true);
        assert_eq!(cmd, Some(EngineCommand::Convert));
    }

    #[test]
    fn enter_key_commits() {
        let cmd = map_key(VK_RETURN, &Modifiers::none(), true);
        assert_eq!(cmd, Some(EngineCommand::Commit));
    }

    #[test]
    fn escape_key_cancels() {
        let cmd = map_key(VK_ESCAPE, &Modifiers::none(), true);
        assert_eq!(cmd, Some(EngineCommand::Cancel));
    }

    #[test]
    fn backspace_key() {
        let cmd = map_key(VK_BACK, &Modifiers::none(), true);
        assert_eq!(cmd, Some(EngineCommand::Backspace));
    }

    #[test]
    fn down_arrow_next_candidate() {
        let cmd = map_key(VK_DOWN, &Modifiers::none(), true);
        assert_eq!(cmd, Some(EngineCommand::NextCandidate));
    }

    #[test]
    fn up_arrow_prev_candidate() {
        let cmd = map_key(VK_UP, &Modifiers::none(), true);
        assert_eq!(cmd, Some(EngineCommand::PrevCandidate));
    }

    // === IME オフ ===

    #[test]
    fn ime_off_returns_none() {
        // IME がオフのときは全てのキーを None (アプリに素通し)
        let cmd = map_key(VK_A, &Modifiers::none(), false);
        assert_eq!(cmd, None);
    }

    #[test]
    fn ime_off_space_returns_none() {
        let cmd = map_key(VK_SPACE, &Modifiers::none(), false);
        assert_eq!(cmd, None);
    }

    // === 修飾キー ===

    #[test]
    fn ctrl_key_returns_none() {
        // Ctrl が押されているときは IME で処理しない
        let cmd = map_key(VK_A, &Modifiers::ctrl(), true);
        assert_eq!(cmd, None);
    }

    #[test]
    fn alt_key_returns_none() {
        // Alt が押されているときは IME で処理しない
        let cmd = map_key(VK_A, &Modifiers::alt(), true);
        assert_eq!(cmd, None);
    }

    #[test]
    fn shift_alphabet_uppercase() {
        // Shift+A → InsertChar('A')（大文字）
        // SKK 方式ではシフト入力が変換開始を意味するが、Phase 4 では大文字をそのまま渡す
        let cmd = map_key(VK_A, &Modifiers::shift(), true);
        assert_eq!(cmd, Some(EngineCommand::InsertChar('A')));
    }

    // === 処理しないキー ===

    #[test]
    fn function_keys_return_none() {
        let cmd = map_key(VK_F1, &Modifiers::none(), true);
        assert_eq!(cmd, None);
    }

    #[test]
    fn number_keys_return_none() {
        // 数字キーは IME で処理しない（Direct 入力）
        let cmd = map_key(VK_0, &Modifiers::none(), true);
        assert_eq!(cmd, None);
    }

    // === マイナス・句読点 ===

    #[test]
    fn minus_key_inserts_minus() {
        let cmd = map_key(VK_OEM_MINUS, &Modifiers::none(), true);
        assert_eq!(cmd, Some(EngineCommand::InsertChar('-')));
    }

    #[test]
    fn period_key_inserts_period() {
        let cmd = map_key(VK_OEM_PERIOD, &Modifiers::none(), true);
        assert_eq!(cmd, Some(EngineCommand::InsertChar('.')));
    }

    #[test]
    fn comma_key_inserts_comma() {
        let cmd = map_key(VK_OEM_COMMA, &Modifiers::none(), true);
        assert_eq!(cmd, Some(EngineCommand::InsertChar(',')));
    }
}
```

公開 API:

```rust
/// Windows 仮想キーコード (VK_*) の定数。
/// プラットフォーム非依存にするため自前で定義する。
pub const VK_BACK: u16 = 0x08;
pub const VK_RETURN: u16 = 0x0D;
pub const VK_ESCAPE: u16 = 0x1B;
pub const VK_SPACE: u16 = 0x20;
// ... (A-Z, 0-9, F1-F12, 矢印, OEM)

/// 修飾キーの状態。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Modifiers {
    pub shift: bool,
    pub ctrl: bool,
    pub alt: bool,
}

impl Modifiers {
    pub fn none() -> Self;
    pub fn shift() -> Self;
    pub fn ctrl() -> Self;
    pub fn alt() -> Self;
}

/// 仮想キーコードと修飾キー状態を EngineCommand に変換する。
///
/// - `vk`: Windows 仮想キーコード
/// - `modifiers`: 修飾キーの状態
/// - `ime_on`: IME がオンかどうか
///
/// 戻り値: 対応する EngineCommand。処理しないキーの場合は None。
pub fn map_key(vk: u16, modifiers: &Modifiers, ime_on: bool) -> Option<EngineCommand>;
```

- **動作確認:** `cargo test` で新しいテストが**失敗する**こと（Red）

### 3-2. Green: KeyMapping を実装

実装方針:
- `ime_on` が `false` なら常に `None` を返す
- `ctrl` または `alt` が押されていたら `None` を返す
- VK コードに応じて:
  - `VK_A..=VK_Z`: Shift なしなら小文字、Shift ありなら大文字で `InsertChar`
  - `VK_SPACE`: `Convert`
  - `VK_RETURN`: `Commit`
  - `VK_ESCAPE`: `Cancel`
  - `VK_BACK`: `Backspace`
  - `VK_DOWN`: `NextCandidate`
  - `VK_UP`: `PrevCandidate`
  - `VK_OEM_MINUS`: `InsertChar('-')`
  - `VK_OEM_PERIOD`: `InsertChar('.')`
  - `VK_OEM_COMMA`: `InsertChar(',')`
  - その他: `None`

- **動作確認:** `cargo test` で全テスト（既存 125 + KeyMapping 約 17 テスト = 約 142 テスト）がパスすること（Green）

### 3-3. Refactor

- **動作確認:** `cargo test` 全パス、`cargo clippy` 警告なし、`cargo fmt -- --check` 差分なし

---

## ステップ 4: COM ClassFactory (Windows 専用)

### 4-1. class_factory.rs の実装

`#[cfg(windows)]` で囲んだ ClassFactory を実装する。

```rust
//! COM ClassFactory。DllGetClassObject から呼ばれ、TextService を生成する。

use windows::core::*;
use windows::Win32::System::Com::*;

use crate::text_service::TextService;

#[implement(IClassFactory)]
pub struct ClassFactory;

impl IClassFactory_Impl for ClassFactory_Impl {
    fn CreateInstance(
        &self,
        punkouter: Option<&IUnknown>,
        riid: *const GUID,
        ppvobject: *mut *mut core::ffi::c_void,
    ) -> Result<()> {
        if punkouter.is_some() {
            return Err(Error::from(CLASS_E_NOAGGREGATION));
        }
        unsafe {
            let service: IUnknown = TextService::new().into();
            service.query(riid, ppvobject).ok()
        }
    }

    fn LockServer(&self, _flock: BOOL) -> Result<()> {
        Ok(())
    }
}
```

**動作確認:**
- Windows 環境で `cargo build` がエラーなく完了すること
- Linux 環境では `#[cfg(windows)]` により無視され、`cargo build` と `cargo test` が通ること

---

## ステップ 5: DLL エントリポイント (Windows 専用)

### 5-1. lib.rs に DLL エクスポート関数を追加

```rust
#[cfg(windows)]
mod dll_exports {
    use windows::core::*;
    use windows::Win32::Foundation::*;
    use windows::Win32::System::Com::*;
    use crate::class_factory::ClassFactory;
    use crate::guids;

    static mut DLL_INSTANCE: HMODULE = HMODULE(std::ptr::null_mut());

    /// DLL ロード時に呼ばれる。
    #[unsafe(no_mangle)]
    unsafe extern "system" fn DllMain(
        hinstance: HMODULE,
        reason: u32,
        _reserved: *mut core::ffi::c_void,
    ) -> BOOL {
        if reason == 1 {
            // DLL_PROCESS_ATTACH
            unsafe { DLL_INSTANCE = hinstance; }
        }
        TRUE
    }

    /// COM オブジェクトファクトリを返す。
    #[unsafe(no_mangle)]
    unsafe extern "system" fn DllGetClassObject(
        rclsid: *const GUID,
        riid: *const GUID,
        ppv: *mut *mut core::ffi::c_void,
    ) -> HRESULT {
        let rclsid = unsafe { &*rclsid };
        if *rclsid != guids::clsid_text_service() {
            return CLASS_E_CLASSNOTAVAILABLE;
        }
        let factory: IClassFactory = ClassFactory.into();
        unsafe { factory.query(riid, ppv) }
    }

    /// DLL がアンロード可能か返す。
    #[unsafe(no_mangle)]
    extern "system" fn DllCanUnloadNow() -> HRESULT {
        S_FALSE // 常にロード状態を維持（簡易実装）
    }

    /// COM サーバーをレジストリに登録する。
    #[unsafe(no_mangle)]
    unsafe extern "system" fn DllRegisterServer() -> HRESULT {
        match crate::registry::register_server(unsafe { DLL_INSTANCE }) {
            Ok(()) => S_OK,
            Err(_) => SELFREG_E_CLASS,
        }
    }

    /// COM サーバーのレジストリ登録を解除する。
    #[unsafe(no_mangle)]
    unsafe extern "system" fn DllUnregisterServer() -> HRESULT {
        match crate::registry::unregister_server() {
            Ok(()) => S_OK,
            Err(_) => SELFREG_E_CLASS,
        }
    }
}
```

**動作確認:**
- Linux 環境: `cargo build` と `cargo test` が通ること（`#[cfg(windows)]` により無視）
- Windows 環境: `cargo build` で DLL が生成されること

---

## ステップ 6: TextService — ITfTextInputProcessorEx (Windows 専用)

### 6-1. text_service.rs の実装

```rust
//! TSF TextService。IME のメインオブジェクト。

use windows::core::*;
use windows::Win32::UI::TextServices::*;
use std::sync::Mutex;

use crate::engine::ConversionEngine;
use crate::dictionary::Dictionary;

#[implement(ITfTextInputProcessorEx, ITfKeyEventSink)]
pub struct TextService {
    thread_mgr: Mutex<Option<ITfThreadMgr>>,
    client_id: Mutex<u32>,
    engine: Mutex<ConversionEngine>,
    ime_on: Mutex<bool>,
}

impl TextService {
    pub fn new() -> Self {
        // dict/ ディレクトリから辞書を読み込み（なければ None）
        let dict = Self::load_default_dict();
        Self {
            thread_mgr: Mutex::new(None),
            client_id: Mutex::new(0),
            engine: Mutex::new(ConversionEngine::new(dict)),
            ime_on: Mutex::new(false),
        }
    }

    fn load_default_dict() -> Option<Dictionary> {
        // 実行ファイルと同じディレクトリの dict/SKK-JISYO.L を探す
        let exe_dir = std::env::current_exe().ok()?.parent()?.to_path_buf();
        let dict_path = exe_dir.join("dict").join("SKK-JISYO.L");
        Dictionary::load_from_file(&dict_path).ok()
    }
}
```

### 6-2. ITfTextInputProcessorEx の実装

```rust
impl ITfTextInputProcessorEx_Impl for TextService_Impl {
    fn ActivateEx(
        &self,
        ptim: Option<&ITfThreadMgr>,
        tid: u32,
        flags: u32,
    ) -> Result<()> {
        let thread_mgr = ptim.ok_or(E_INVALIDARG)?.clone();

        // KeyEventSink を登録
        let keystroke_mgr: ITfKeystrokeMgr = thread_mgr.cast()?;
        let self_sink: ITfKeyEventSink = self.cast()?;
        unsafe {
            keystroke_mgr.AdviseKeyEventSink(tid, &self_sink, TRUE)?;
        }

        *self.thread_mgr.lock().unwrap() = Some(thread_mgr);
        *self.client_id.lock().unwrap() = tid;
        *self.ime_on.lock().unwrap() = true;

        Ok(())
    }

    fn Deactivate(&self) -> Result<()> {
        let thread_mgr = self.thread_mgr.lock().unwrap().take();
        let tid = *self.client_id.lock().unwrap();

        if let Some(thread_mgr) = thread_mgr {
            let keystroke_mgr: ITfKeystrokeMgr = thread_mgr.cast()?;
            unsafe {
                let _ = keystroke_mgr.UnadviseKeyEventSink(tid);
            }
        }

        *self.ime_on.lock().unwrap() = false;
        Ok(())
    }
}

impl ITfTextInputProcessor_Impl for TextService_Impl {
    fn Activate(&self, ptim: Option<&ITfThreadMgr>, tid: u32) -> Result<()> {
        self.ActivateEx(ptim, tid, 0)
    }

    fn Deactivate(&self) -> Result<()> {
        ITfTextInputProcessorEx_Impl::Deactivate(self)
    }
}
```

**動作確認:**
- Linux 環境: `cargo build` と `cargo test` が通ること
- Windows 環境: `cargo build` がエラーなく完了すること

---

## ステップ 7: キーイベント処理 — ITfKeyEventSink (Windows 専用)

### 7-1. ITfKeyEventSink の実装

KeyMapping モジュール（ステップ 3 で TDD 済み）を使って、TSF のキーイベントを
ConversionEngine のコマンドに変換する。

```rust
impl ITfKeyEventSink_Impl for TextService_Impl {
    fn OnSetFocus(&self, _foreground: BOOL) -> Result<()> {
        Ok(())
    }

    fn OnTestKeyDown(
        &self,
        pic: Option<&ITfContext>,
        wparam: WPARAM,
        lparam: LPARAM,
    ) -> Result<BOOL> {
        let ime_on = *self.ime_on.lock().unwrap();
        let modifiers = Modifiers::from_keyboard_state();
        let vk = wparam.0 as u16;

        // このキーを処理するかどうかを返す
        match map_key(vk, &modifiers, ime_on) {
            Some(_) => Ok(TRUE),   // IME で処理する
            None => Ok(FALSE),     // アプリに素通し
        }
    }

    fn OnKeyDown(
        &self,
        pic: Option<&ITfContext>,
        wparam: WPARAM,
        lparam: LPARAM,
    ) -> Result<BOOL> {
        let ime_on = *self.ime_on.lock().unwrap();
        let modifiers = Modifiers::from_keyboard_state();
        let vk = wparam.0 as u16;

        let Some(command) = map_key(vk, &modifiers, ime_on) else {
            return Ok(FALSE);
        };

        let mut engine = self.engine.lock().unwrap();
        let output = engine.process(command);

        // Composition の更新 (ステップ 8 で実装)
        if let Some(context) = pic {
            self.update_composition(context, &output)?;
        }

        Ok(TRUE)
    }

    fn OnTestKeyUp(
        &self,
        _pic: Option<&ITfContext>,
        _wparam: WPARAM,
        _lparam: LPARAM,
    ) -> Result<BOOL> {
        Ok(FALSE) // KeyUp は処理しない
    }

    fn OnKeyUp(
        &self,
        _pic: Option<&ITfContext>,
        _wparam: WPARAM,
        _lparam: LPARAM,
    ) -> Result<BOOL> {
        Ok(FALSE)
    }
}
```

### 7-2. Modifiers::from_keyboard_state (Windows 専用)

```rust
#[cfg(windows)]
impl Modifiers {
    pub fn from_keyboard_state() -> Self {
        use windows::Win32::UI::Input::KeyboardAndMouse::GetKeyState;
        unsafe {
            Self {
                shift: GetKeyState(VK_SHIFT as i32) < 0,
                ctrl: GetKeyState(VK_CONTROL as i32) < 0,
                alt: GetKeyState(VK_MENU as i32) < 0,
            }
        }
    }
}
```

**動作確認:**
- Linux 環境: `cargo build` と `cargo test` が通ること
- Windows 環境: `cargo build` がエラーなく完了すること

---

## ステップ 8: Composition 管理 (Windows 専用)

### 8-1. TextService に Composition 関連メソッドを追加

TSF の Composition API を使って、未確定テキスト（下線付き）の表示と確定テキストの挿入を行う。

```rust
impl TextService {
    /// EngineOutput に基づいて Composition を更新する。
    fn update_composition(
        &self,
        context: &ITfContext,
        output: &EngineOutput,
    ) -> Result<()> {
        // 確定テキストがある場合: 挿入して Composition を終了
        if !output.committed.is_empty() {
            self.commit_text(context, &output.committed)?;
            return Ok(());
        }

        // 表示テキストがある場合: Composition を開始/更新
        if !output.display.is_empty() {
            self.set_composition_text(context, &output.display)?;
        } else {
            // 表示テキストもない場合: Composition を終了
            self.end_composition()?;
        }

        Ok(())
    }

    /// Composition を開始する（まだ開始していない場合）。
    fn start_composition(&self, context: &ITfContext) -> Result<()> {
        // ITfContextComposition を使って Composition を開始
        // ITfCompositionSink として自分自身を登録
        todo!("Composition 開始の詳細実装")
    }

    /// Composition テキストを設定する。
    fn set_composition_text(&self, context: &ITfContext, text: &str) -> Result<()> {
        // 現在の Composition 範囲のテキストを置換
        // 未確定テキスト用の表示属性（下線）を設定
        todo!("テキスト設定の詳細実装")
    }

    /// テキストを確定し、Composition を終了する。
    fn commit_text(&self, context: &ITfContext, text: &str) -> Result<()> {
        // テキストを挿入
        // Composition を終了
        todo!("テキスト確定の詳細実装")
    }

    /// Composition を終了する。
    fn end_composition(&self) -> Result<()> {
        todo!("Composition 終了の詳細実装")
    }
}
```

**注意:** Composition 管理は TSF API の中で最も複雑な部分。
`ITfInsertAtSelection`、`ITfRange`、`ITfComposition` 等の複数のインターフェースを
組み合わせる必要がある。Windows 環境での段階的な実装・デバッグが必須。

**動作確認:**
- Linux 環境: `cargo build` と `cargo test` が通ること（`#[cfg(windows)]` + `todo!` で OK）
- Windows 環境: メモ帳で文字入力して未確定テキスト表示→確定のフロー確認

---

## ステップ 9: レジストリ登録 (Windows 専用)

### 9-1. registry.rs の実装

COM サーバーと TSF プロファイルの登録/解除を行う。

```rust
//! COM サーバーと TSF プロファイルのレジストリ登録。

use windows::core::*;
use windows::Win32::Foundation::*;
use windows::Win32::System::Registry::*;
use windows::Win32::UI::TextServices::*;

use crate::guids;

const IME_DISPLAY_NAME: &str = "Enpitsu";
const LANGID_JAPANESE: u16 = 0x0411;

/// COM サーバーをレジストリに登録する。
pub fn register_server(dll_instance: HMODULE) -> Result<()> {
    let dll_path = get_dll_path(dll_instance)?;
    let clsid_str = guid_to_registry_string(&guids::clsid_text_service());

    // 1. CLSID をレジストリに登録
    register_clsid(&clsid_str, &dll_path)?;

    // 2. TSF プロファイルを登録
    register_profile()?;

    // 3. カテゴリを登録 (GUID_TFCAT_TIP_KEYBOARD)
    register_categories()?;

    Ok(())
}

/// COM サーバーのレジストリ登録を解除する。
pub fn unregister_server() -> Result<()> {
    unregister_categories()?;
    unregister_profile()?;
    unregister_clsid()?;
    Ok(())
}

fn register_clsid(clsid_str: &str, dll_path: &str) -> Result<()> {
    // HKCR\CLSID\{...}\InProcServer32 に DLL パスを登録
    // ThreadingModel = "Apartment"
    todo!()
}

fn register_profile() -> Result<()> {
    // ITfInputProcessorProfiles を使ってプロファイル登録
    // 言語: 日本語 (0x0411)
    // アイコン: DLL 内リソース (将来追加)
    todo!()
}

fn register_categories() -> Result<()> {
    // ITfCategoryMgr を使ってカテゴリ登録
    // GUID_TFCAT_TIP_KEYBOARD: キーボード入力プロセッサとして登録
    todo!()
}
```

**動作確認:**
- Windows 環境で管理者権限で `regsvr32 target\debug\enpitsu.dll` を実行
- Windows の「設定 → 入力メソッド」に Enpitsu が表示されること

---

## ステップ 10: インストールスクリプト

### 10-1. installer/install.ps1 の作成

```powershell
# Enpitsu インストーラー
# 管理者権限で実行すること

param(
    [switch]$Uninstall
)

$ErrorActionPreference = "Stop"
$DllName = "enpitsu.dll"
$DllSource = Join-Path $PSScriptRoot "..\target\release\$DllName"

if ($Uninstall) {
    Write-Host "Enpitsu をアンインストールしています..."
    regsvr32 /u /s $DllSource
    Write-Host "完了。"
} else {
    if (-not (Test-Path $DllSource)) {
        Write-Host "エラー: DLL が見つかりません: $DllSource"
        Write-Host "先に 'cargo build --release' を実行してください。"
        exit 1
    }
    Write-Host "Enpitsu をインストールしています..."
    regsvr32 /s $DllSource
    Write-Host "完了。入力メソッドの設定から Enpitsu を追加してください。"
}
```

**動作確認:**
- Windows 環境で `.\installer\install.ps1` を管理者権限で実行
- `.\installer\install.ps1 -Uninstall` でアンインストール
- 入力メソッド一覧に Enpitsu が追加/削除されること

---

## ステップ 11: 最終確認

### 11-1. Linux 環境での確認

```sh
cargo test        # 既存 125 + KeyMapping 約 17 = 約 142 テストが全パス
cargo clippy      # 警告なし
cargo fmt -- --check  # フォーマット差分なし
cargo build       # エラーなし
```

### 11-2. Windows 環境での確認

```sh
cargo build --release    # DLL 生成
cargo test               # 全テストパス
```

手動テスト:

1. `regsvr32 target\release\enpitsu.dll` で登録
2. Windows の設定 → 入力メソッドに Enpitsu を追加
3. メモ帳を開き、Enpitsu を選択
4. ローマ字入力 → ひらがな変換を確認
5. Space で変換候補表示、Enter で確定を確認
6. `regsvr32 /u target\release\enpitsu.dll` で登録解除

### 11-3. CLAUDE.md の更新

- ファイル構成に新規ファイルを追記
- Windows ビルド手順を追記
- インストール手順を追記

### 11-4. コミット

- テスト全パスを確認した上でコミット

---

## 追加テスト一覧（予定）

| # | テスト名 | モジュール | 分類 | 内容 |
|---|---------|-----------|------|------|
| 1 | `alphabet_key_lowercase` | key_mapping | アルファベット | 小文字変換 |
| 2 | `alphabet_key_all_letters` | key_mapping | アルファベット | A-Z 全文字 |
| 3 | `space_key_converts` | key_mapping | 特殊キー | Space → Convert |
| 4 | `enter_key_commits` | key_mapping | 特殊キー | Enter → Commit |
| 5 | `escape_key_cancels` | key_mapping | 特殊キー | Escape → Cancel |
| 6 | `backspace_key` | key_mapping | 特殊キー | Backspace |
| 7 | `down_arrow_next_candidate` | key_mapping | 特殊キー | ↓ → NextCandidate |
| 8 | `up_arrow_prev_candidate` | key_mapping | 特殊キー | ↑ → PrevCandidate |
| 9 | `ime_off_returns_none` | key_mapping | IME オフ | IME オフ → None |
| 10 | `ime_off_space_returns_none` | key_mapping | IME オフ | IME オフ Space → None |
| 11 | `ctrl_key_returns_none` | key_mapping | 修飾キー | Ctrl+A → None |
| 12 | `alt_key_returns_none` | key_mapping | 修飾キー | Alt+A → None |
| 13 | `shift_alphabet_uppercase` | key_mapping | 修飾キー | Shift+A → 'A' |
| 14 | `function_keys_return_none` | key_mapping | 非対象キー | F1 → None |
| 15 | `number_keys_return_none` | key_mapping | 非対象キー | 数字 → None |
| 16 | `minus_key_inserts_minus` | key_mapping | 句読点 | マイナス → '-' |
| 17 | `period_key_inserts_period` | key_mapping | 句読点 | ピリオド → '.' |
| 18 | `comma_key_inserts_comma` | key_mapping | 句読点 | カンマ → ',' |

合計: 約 18 テスト追加（既存 125 + 新規 18 = 約 143 テスト）

Windows 専用コード（ステップ 4〜9）は `#[cfg(windows)]` で分離されており、
Linux 上のユニットテストには含まれない。Windows 環境での手動テストで検証する。

---

## 依存クレート（追加）

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

## 実装上の注意事項

### TSF の複雑さ

TSF は Windows の中でも特に複雑な API の一つ。以下の点に注意:

- **スレッドモデル**: TSF は STA (Single Thread Apartment) で動作する。
  `Mutex` を使う場合はデッドロックに注意。
- **Composition API**: テキスト挿入は `ITfInsertAtSelection` →
  `ITfRange` → `SetText` の手順が必要。
- **参照カウント**: COM オブジェクトの参照カウントは `windows` crate が自動管理するが、
  循環参照に注意。
- **デバッグ**: `OutputDebugString` や `log` crate でのデバッグ出力が有用。

### 段階的な動作確認

Phase 4 は Windows 環境でないと完全な動作確認ができないため、以下の順序で段階的に確認する:

1. **Linux**: `cargo build` + `cargo test` が通ること（KeyMapping テスト含む）
2. **Windows (ビルド)**: `cargo build` が通ること
3. **Windows (DLL 登録)**: `regsvr32` が成功すること
4. **Windows (IME 選択)**: 入力メソッド一覧に表示されること
5. **Windows (文字入力)**: メモ帳でローマ字→ひらがな変換が動作すること
6. **Windows (辞書変換)**: Space で漢字候補が表示されること

### 参考リソース

- Microsoft SampleIME (C++ 実装): TSF IME の参考実装
- `windows` crate ドキュメント: COM interface 実装パターン
- TSF Aware テスト: TSFPad で IME の動作確認
