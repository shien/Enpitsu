//! TSF TextService。IME のメインオブジェクト。
//!
//! `ITfTextInputProcessorEx` と `ITfKeyEventSink` を実装し、
//! Windows の TSF フレームワークと ConversionEngine を接続する。

use std::sync::{Arc, Mutex};

use windows::Win32::Foundation::*;
use windows::Win32::UI::Input::KeyboardAndMouse::GetKeyState;
use windows::Win32::UI::TextServices::*;
use windows::core::*;

use crate::config::Config;
use crate::dictionary::Dictionary;
use crate::engine::{ConversionEngine, EngineOutput};
use crate::key_mapping::{self, CtrlKeyConfig, Modifiers};
use crate::user_dictionary::UserDictionary;

// === EditSession ===

/// EditSession 内で実行するアクション。
enum EditAction {
    /// Composition を開始/更新してテキストを設定する。
    SetText(String),
    /// テキストを確定して Composition を終了する。
    CommitText(String),
    /// テキストを確定して Composition を終了し、直後に新しい Composition を開始する。
    /// Converting 中の InsertChar で候補確定と新規入力を同一セッションで処理する。
    CommitAndCompose { committed: String, display: String },
    /// Composition を終了する。
    EndComposition,
}

/// TSF の EditSession。テキスト操作は全て EditSession コールバック内で行う。
///
/// `RequestEditSession` に渡すと、TSF が適切なタイミングで
/// `DoEditSession` を呼び出し、edit cookie を提供する。
/// テキスト挿入・範囲操作はこの edit cookie を使って行う必要がある。
#[implement(ITfEditSession)]
struct EditSession {
    context: ITfContext,
    composition: Arc<Mutex<Option<ITfComposition>>>,
    action: EditAction,
}

impl EditSession {
    /// Composition が未開始なら、現在のカーソル位置で開始する。
    fn ensure_composition(&self, ec: u32) -> Result<()> {
        let mut comp = self.composition.lock().unwrap();
        if comp.is_some() {
            return Ok(());
        }

        unsafe {
            // カーソル位置の範囲を取得（テキストは挿入しない）
            let insert: ITfInsertAtSelection = self.context.cast()?;
            let range = insert.InsertTextAtSelection(ec, TF_IAS_QUERYONLY, &[])?;

            // その範囲で Composition を開始
            let ctx_comp: ITfContextComposition = self.context.cast()?;
            let new_comp = ctx_comp.StartComposition(ec, &range, None)?;

            *comp = Some(new_comp);
        }
        Ok(())
    }

    /// Composition 範囲のテキストを設定する。
    fn write_text(&self, ec: u32, text: &str) -> Result<()> {
        let comp = self.composition.lock().unwrap();
        if let Some(ref composition) = *comp {
            unsafe {
                let range = composition.GetRange()?;
                let wide: Vec<u16> = text.encode_utf16().collect();
                range.SetText(ec, 0, &wide)?;
            }
        }
        Ok(())
    }

    /// Composition を終了し、参照をクリアする。
    fn finish_composition(&self, ec: u32) -> Result<()> {
        let mut comp = self.composition.lock().unwrap();
        if let Some(composition) = comp.take() {
            unsafe {
                composition.EndComposition(ec)?;
            }
        }
        Ok(())
    }
}

impl ITfEditSession_Impl for EditSession_Impl {
    fn DoEditSession(&self, ec: u32) -> Result<()> {
        match &self.action {
            EditAction::SetText(text) => {
                self.ensure_composition(ec)?;
                self.write_text(ec, text)?;
            }
            EditAction::CommitText(text) => {
                self.ensure_composition(ec)?;
                self.write_text(ec, text)?;
                self.finish_composition(ec)?;
            }
            EditAction::CommitAndCompose { committed, display } => {
                // 1. 現在の候補を確定して Composition を終了
                self.ensure_composition(ec)?;
                self.write_text(ec, committed)?;
                self.finish_composition(ec)?;
                // 2. 新しい Composition を開始して次の入力を表示
                self.ensure_composition(ec)?;
                self.write_text(ec, display)?;
            }
            EditAction::EndComposition => {
                self.finish_composition(ec)?;
            }
        }
        Ok(())
    }
}

// === TextService ===

#[implement(ITfTextInputProcessorEx, ITfTextInputProcessor, ITfKeyEventSink)]
pub struct TextService {
    thread_mgr: Mutex<Option<ITfThreadMgr>>,
    client_id: Mutex<u32>,
    engine: Mutex<ConversionEngine>,
    ime_on: Mutex<bool>,
    composition: Arc<Mutex<Option<ITfComposition>>>,
    ctrl_config: CtrlKeyConfig,
}

impl TextService {
    pub fn new() -> Self {
        // 設定ファイルの読み込み
        let config_path = get_appdata_path("config.toml");
        let config = Config::load(&config_path).unwrap_or_else(|_| Config::default_config());

        // システム辞書の読み込み
        let dict = if let Some(ref path) = config.system_dict_path {
            Dictionary::load_from_file(std::path::Path::new(path)).ok()
        } else {
            Self::load_default_dict()
        };

        // ユーザー辞書の読み込み
        let user_dict_path = get_appdata_path("user_dict.txt");
        let user_dict = if config.auto_learn {
            UserDictionary::load(&user_dict_path).ok()
        } else {
            None
        };

        let ctrl_config = config.keybind.clone();

        Self {
            thread_mgr: Mutex::new(None),
            client_id: Mutex::new(0),
            engine: Mutex::new(ConversionEngine::new_with_user_dict(dict, user_dict)),
            ime_on: Mutex::new(false),
            composition: Arc::new(Mutex::new(None)),
            ctrl_config,
        }
    }

    fn load_default_dict() -> Option<Dictionary> {
        let dll_dir = Self::dll_directory()?;
        let dict_path = dll_dir.join("dict").join("SKK-JISYO.L");
        Dictionary::load_from_file(&dict_path).ok()
    }

    /// DLL の配置ディレクトリを取得する。
    ///
    /// `DllMain` で記録した HMODULE から `GetModuleFileNameW` で DLL パスを解決する。
    /// ホストプロセス（notepad.exe 等）ではなく DLL 自身のパスが返る。
    fn dll_directory() -> Option<std::path::PathBuf> {
        use std::ffi::OsString;
        use std::os::windows::ffi::OsStringExt;
        use windows::Win32::System::LibraryLoader::GetModuleFileNameW;

        let hmodule = crate::dll_exports::dll_instance();
        if hmodule.0.is_null() {
            return None;
        }
        let mut buf = [0u16; 260];
        let len = unsafe { GetModuleFileNameW(Some(hmodule), &mut buf) } as usize;
        if len == 0 {
            return None;
        }
        let path = OsString::from_wide(&buf[..len]);
        std::path::Path::new(&path)
            .parent()
            .map(|p| p.to_path_buf())
    }

    /// EngineOutput に基づいて EditSession を発行し、Composition を更新する。
    fn update_composition(&self, context: &ITfContext, output: &EngineOutput) -> Result<()> {
        let action = if !output.committed.is_empty() && !output.display.is_empty() {
            // 候補確定と新規入力が同時に発生（例: Converting 中の InsertChar）
            EditAction::CommitAndCompose {
                committed: output.committed.clone(),
                display: output.display.clone(),
            }
        } else if !output.committed.is_empty() {
            EditAction::CommitText(output.committed.clone())
        } else if !output.display.is_empty() {
            EditAction::SetText(output.display.clone())
        } else {
            // 表示も確定テキストもない場合、Composition がなければ何もしない
            if self.composition.lock().unwrap().is_none() {
                return Ok(());
            }
            EditAction::EndComposition
        };

        let session: ITfEditSession = EditSession {
            context: context.clone(),
            composition: Arc::clone(&self.composition),
            action,
        }
        .into();

        let tid = *self.client_id.lock().unwrap();
        unsafe {
            let _session_hr =
                context.RequestEditSession(tid, &session, TF_ES_READWRITE | TF_ES_SYNC)?;
        }

        Ok(())
    }
}

// --- ITfTextInputProcessorEx ---

impl ITfTextInputProcessorEx_Impl for TextService_Impl {
    fn ActivateEx(&self, ptim: Option<&ITfThreadMgr>, tid: u32, _flags: u32) -> Result<()> {
        let thread_mgr = ptim.ok_or(E_INVALIDARG)?.clone();

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
}

// --- ITfTextInputProcessor ---

impl ITfTextInputProcessor_Impl for TextService_Impl {
    fn Activate(&self, ptim: Option<&ITfThreadMgr>, tid: u32) -> Result<()> {
        self.ActivateEx(ptim, tid, 0)
    }

    fn Deactivate(&self) -> Result<()> {
        let thread_mgr = self.thread_mgr.lock().unwrap().take();
        let tid = *self.client_id.lock().unwrap();

        if let Some(thread_mgr) = thread_mgr {
            if let Ok(keystroke_mgr) = thread_mgr.cast::<ITfKeystrokeMgr>() {
                unsafe {
                    let _ = keystroke_mgr.UnadviseKeyEventSink(tid);
                }
            }
        }

        // ユーザー辞書の保存
        let mut engine = self.engine.lock().unwrap();
        if let Some(ud) = engine.user_dict_mut() {
            if ud.is_dirty() {
                let path = get_appdata_path("user_dict.txt");
                let _ = ud.save(&path);
            }
        }
        drop(engine);

        *self.ime_on.lock().unwrap() = false;
        // EditSession なしでは EndComposition(ec) を呼べないため、参照のみ解放する。
        // TSF は TIP の Deactivate 時にアクティブな Composition を自動終了する。
        *self.composition.lock().unwrap() = None;
        Ok(())
    }
}

// --- ITfKeyEventSink ---

impl ITfKeyEventSink_Impl for TextService_Impl {
    fn OnSetFocus(&self, _fforeground: BOOL) -> Result<()> {
        Ok(())
    }

    fn OnTestKeyDown(
        &self,
        _pic: Option<&ITfContext>,
        wparam: WPARAM,
        _lparam: LPARAM,
    ) -> Result<BOOL> {
        let ime_on = *self.ime_on.lock().unwrap();
        let modifiers = modifiers_from_keyboard_state();
        let vk = wparam.0 as u16;

        match key_mapping::map_key(vk, &modifiers, ime_on, &self.ctrl_config) {
            Some(_) => Ok(TRUE),
            None => Ok(FALSE),
        }
    }

    fn OnKeyDown(&self, pic: Option<&ITfContext>, wparam: WPARAM, _lparam: LPARAM) -> Result<BOOL> {
        let ime_on = *self.ime_on.lock().unwrap();
        let modifiers = modifiers_from_keyboard_state();
        let vk = wparam.0 as u16;

        let Some(command) = key_mapping::map_key(vk, &modifiers, ime_on, &self.ctrl_config) else {
            return Ok(FALSE);
        };

        let mut engine = self.engine.lock().unwrap();
        let output = engine.process(command);
        drop(engine);

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
        Ok(FALSE)
    }

    fn OnKeyUp(&self, _pic: Option<&ITfContext>, _wparam: WPARAM, _lparam: LPARAM) -> Result<BOOL> {
        Ok(FALSE)
    }

    fn OnPreservedKey(
        &self,
        _pic: Option<&ITfContext>,
        _rguid: &GUID,
    ) -> Result<BOOL> {
        Ok(FALSE)
    }
}

/// %APPDATA%\enpitsu\ 以下のパスを返す。
fn get_appdata_path(filename: &str) -> std::path::PathBuf {
    let appdata = std::env::var("APPDATA").unwrap_or_else(|_| ".".to_string());
    std::path::PathBuf::from(appdata)
        .join("enpitsu")
        .join(filename)
}

/// キーボードの現在の修飾キー状態を取得する。
fn modifiers_from_keyboard_state() -> Modifiers {
    unsafe {
        Modifiers {
            shift: GetKeyState(key_mapping::VK_SHIFT as i32) < 0,
            ctrl: GetKeyState(key_mapping::VK_CONTROL as i32) < 0,
            alt: GetKeyState(key_mapping::VK_MENU as i32) < 0,
        }
    }
}
