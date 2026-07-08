//! TSF TextService。IME のメインオブジェクト。
//!
//! `ITfTextInputProcessorEx` と `ITfKeyEventSink` を実装し、
//! Windows の TSF フレームワークと ConversionEngine を接続する。

use std::mem::ManuallyDrop;
use std::sync::{Arc, Mutex};

use windows::Win32::Foundation::*;
use windows::Win32::UI::Input::KeyboardAndMouse::GetKeyState;
use windows::Win32::UI::TextServices::*;
use windows::core::*;

use crate::config::{Config, ToggleKey};
use crate::dictionary::Dictionary;
use crate::engine::{ConversionEngine, EngineCommand, EngineOutput, EngineState};
use crate::guids;
use crate::key_mapping::{self, CtrlKeyConfig, Modifiers};
use crate::user_dictionary::UserDictionary;

/// デバッグログを OutputDebugStringW で出力する（UTF-8 文字化け防止）。
#[cfg(windows)]
fn debug_log(msg: &str) {
    use windows::Win32::System::Diagnostics::Debug::OutputDebugStringW;
    let formatted = format!("[Enpitsu] {}", msg);
    let wide: Vec<u16> = formatted.encode_utf16().chain(std::iter::once(0)).collect();
    unsafe {
        OutputDebugStringW(windows::core::PCWSTR(wide.as_ptr()));
    }
}

// === CompositionSink ===

/// TSF が Composition を外部から終了したときに通知を受けるシンク。
///
/// `StartComposition` に渡す必須パラメータ。
/// アプリ側のリセット等で Composition が終了されたとき、内部状態をクリアする。
#[implement(ITfCompositionSink)]
struct CompositionSink {
    composition: Arc<Mutex<Option<ITfComposition>>>,
}

impl ITfCompositionSink_Impl for CompositionSink_Impl {
    fn OnCompositionTerminated(
        &self,
        _ecwrite: u32,
        _pcomposition: Option<&ITfComposition>,
    ) -> Result<()> {
        debug_log("OnCompositionTerminated called");
        *self.composition.lock().unwrap() = None;
        Ok(())
    }
}

// === EditSession ===

/// EditSession 内で実行するアクション。
enum EditAction {
    /// Composition を開始/更新してテキストを設定し、カーソル位置を反映する。
    SetText { text: String, cursor_pos: usize },
    /// テキストを確定して Composition を終了する。
    CommitText(String),
    /// テキストを確定して Composition を終了し、直後に新しい Composition を開始する。
    /// Converting 中の InsertChar で候補確定と新規入力を同一セッションで処理する。
    CommitAndCompose {
        committed: String,
        display: String,
        cursor_pos: usize,
    },
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
            debug_log("ensure_composition: already active");
            return Ok(());
        }

        unsafe {
            // カーソル位置の範囲を取得（テキストは挿入しない）
            let insert: ITfInsertAtSelection = self.context.cast()?;
            debug_log("ensure_composition: InsertTextAtSelection (QUERYONLY)...");
            let range = insert.InsertTextAtSelection(ec, TF_IAS_QUERYONLY, &[])?;
            debug_log("ensure_composition: InsertTextAtSelection succeeded");

            // その範囲で Composition を開始
            let ctx_comp: ITfContextComposition = self.context.cast()?;
            let sink: ITfCompositionSink = CompositionSink {
                composition: Arc::clone(&self.composition),
            }
            .into();
            debug_log("ensure_composition: StartComposition...");
            let new_comp = ctx_comp.StartComposition(ec, &range, &sink)?;
            debug_log("ensure_composition: StartComposition succeeded");

            *comp = Some(new_comp);
        }
        Ok(())
    }

    /// Composition 範囲のテキストを設定する。
    fn write_text(&self, ec: u32, text: &str) -> Result<()> {
        let comp = self.composition.lock().unwrap();
        if let Some(ref composition) = *comp {
            unsafe {
                debug_log("write_text: GetRange...");
                let range = composition.GetRange()?;
                debug_log("write_text: GetRange succeeded");
                let wide: Vec<u16> = text.encode_utf16().collect();
                debug_log(&format!("write_text: SetText (len={})...", wide.len()));
                range.SetText(ec, 0, &wide)?;
                debug_log("write_text: SetText succeeded");
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

    /// Composition 範囲内のキャレット位置を `cursor_pos`（先頭からの文字数）に設定する。
    ///
    /// `SetText` はテキストを置換するだけでキャレットを移動しないため、
    /// `ITfContext::SetSelection` で明示的に 0 幅の選択（キャレット）を設定する。
    fn set_cursor_position(&self, ec: u32, cursor_pos: usize) -> Result<()> {
        let comp = self.composition.lock().unwrap();
        if let Some(ref composition) = *comp {
            unsafe {
                let range = composition.GetRange()?;
                let cloned = range.Clone()?;

                // 範囲を先頭に collapse（0 幅にする）
                cloned.Collapse(ec, TF_ANCHOR_START)?;

                // 終了アンカーを cursor_pos 文字分だけ前方へ移動
                let mut shifted: i32 = 0;
                let halt_cond = TF_HALTCOND::default();
                cloned.ShiftEnd(ec, cursor_pos as i32, &mut shifted, &halt_cond)?;
                if shifted != cursor_pos as i32 {
                    debug_log(&format!(
                        "set_cursor_position: shifted={} != cursor_pos={}",
                        shifted, cursor_pos
                    ));
                }

                // 終了位置で再度 collapse → 0 幅のキャレットにする
                cloned.Collapse(ec, TF_ANCHOR_END)?;

                let mut selection = TF_SELECTION {
                    range: ManuallyDrop::new(Some(cloned)),
                    style: TF_SELECTIONSTYLE {
                        ase: TF_AE_END,
                        fInterimChar: FALSE,
                    },
                };
                debug_log(&format!("set_cursor_position: pos={}", cursor_pos));
                // SetSelection は範囲を内部で AddRef するため、渡した後はこちらの
                // クローン参照を解放する必要がある。range は ManuallyDrop に move
                // されており自動 Drop されないので、呼び出し後に明示的に drop して
                // キー入力ごとの ITfRange リークを防ぐ。
                let result = self
                    .context
                    .SetSelection(ec, std::slice::from_ref(&selection));
                ManuallyDrop::drop(&mut selection.range);
                result?;
            }
        }
        Ok(())
    }
}

impl ITfEditSession_Impl for EditSession_Impl {
    fn DoEditSession(&self, ec: u32) -> Result<()> {
        debug_log(&format!("DoEditSession called, ec={}", ec));
        let result = match &self.action {
            EditAction::SetText { text, cursor_pos } => {
                debug_log(&format!("DoEditSession: SetText('{}')", text));
                self.ensure_composition(ec)
                    .and_then(|()| self.write_text(ec, text))
                    .and_then(|()| self.set_cursor_position(ec, *cursor_pos))
            }
            EditAction::CommitText(text) => {
                debug_log(&format!("DoEditSession: CommitText('{}')", text));
                // 確定文字列の末尾へキャレットを移動してから Composition を終了する。
                // これを省くと確定語の先頭にキャレットが戻る。
                let caret = text.chars().count();
                self.ensure_composition(ec)
                    .and_then(|()| self.write_text(ec, text))
                    .and_then(|()| self.set_cursor_position(ec, caret))
                    .and_then(|()| self.finish_composition(ec))
            }
            EditAction::CommitAndCompose {
                committed,
                display,
                cursor_pos,
            } => {
                debug_log(&format!(
                    "DoEditSession: CommitAndCompose('{}', '{}')",
                    committed, display
                ));
                // 確定部分の末尾へキャレットを移動してから終了し、
                // 続く新規 Composition が確定語の後ろから始まるようにする。
                let committed_caret = committed.chars().count();
                self.ensure_composition(ec)
                    .and_then(|()| self.write_text(ec, committed))
                    .and_then(|()| self.set_cursor_position(ec, committed_caret))
                    .and_then(|()| self.finish_composition(ec))
                    .and_then(|()| self.ensure_composition(ec))
                    .and_then(|()| self.write_text(ec, display))
                    .and_then(|()| self.set_cursor_position(ec, *cursor_pos))
            }
            EditAction::EndComposition => {
                debug_log("DoEditSession: EndComposition");
                self.finish_composition(ec)
            }
        };
        if let Err(ref e) = result {
            debug_log(&format!("DoEditSession FAILED: {:?}", e));
        } else {
            debug_log("DoEditSession completed successfully");
        }
        result
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
    toggle_key: ToggleKey,
    /// PreserveKey で登録済みの予約キー（GUID とキー仕様）。
    /// OnPreservedKey での照合と Deactivate 時の解除に使う。
    preserved: Mutex<Vec<(GUID, TF_PRESERVEDKEY)>>,
}

impl TextService {
    pub fn new() -> Self {
        debug_log("TextService::new() called");

        // 設定ファイルの読み込み
        let config_path = get_appdata_path("config.toml");
        debug_log(&format!("Loading config from: {:?}", config_path));
        let config = Config::load(&config_path).unwrap_or_else(|_| {
            debug_log("Config load failed, using defaults");
            Config::default_config()
        });

        // システム辞書の読み込み
        let dict = if let Some(ref path) = config.system_dict_path {
            debug_log(&format!("Loading system dict from config: {}", path));
            Dictionary::load_from_file(std::path::Path::new(path)).ok()
        } else {
            debug_log("Loading default dict from DLL directory");
            Self::load_default_dict()
        };
        debug_log(&format!("System dict loaded: {}", dict.is_some()));

        // ユーザー辞書の読み込み
        let user_dict_path = get_appdata_path("user_dict.txt");
        let user_dict = if config.auto_learn {
            UserDictionary::load(&user_dict_path).ok()
        } else {
            None
        };

        let ctrl_config = config.keybind.clone();
        let toggle_key = config.toggle_key.clone();

        debug_log("TextService::new() completed");
        Self {
            thread_mgr: Mutex::new(None),
            client_id: Mutex::new(0),
            engine: Mutex::new(ConversionEngine::new_with_user_dict(dict, user_dict)),
            ime_on: Mutex::new(false),
            composition: Arc::new(Mutex::new(None)),
            ctrl_config,
            toggle_key,
            preserved: Mutex::new(Vec::new()),
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
        let len = unsafe { GetModuleFileNameW(hmodule, &mut buf) } as usize;
        if len == 0 {
            return None;
        }
        let path = OsString::from_wide(&buf[..len]);
        std::path::Path::new(&path)
            .parent()
            .map(|p| p.to_path_buf())
    }

    /// IME のオン/オフを切り替える。オフにする際は未確定入力をキャンセルする。
    ///
    /// 予約キー（OnPreservedKey）と通常キー（OnKeyDown）の両経路から呼ばれる。
    fn handle_toggle(&self, pic: Option<&ITfContext>) -> Result<()> {
        let mut ime_on = self.ime_on.lock().unwrap();
        *ime_on = !*ime_on;
        let now_on = *ime_on;
        drop(ime_on);
        debug_log(&format!(
            "handle_toggle: IME toggled to {}",
            if now_on { "ON" } else { "OFF" }
        ));

        if !now_on {
            // IME をオフにする際、未確定入力をキャンセルする
            let mut engine = self.engine.lock().unwrap();
            let output = engine.process(EngineCommand::Cancel);
            drop(engine);
            if let Some(context) = pic {
                if let Err(e) = self.update_composition(context, &output) {
                    debug_log(&format!(
                        "handle_toggle: update_composition FAILED: {:?}",
                        e
                    ));
                    return Err(e);
                }
            }
        }
        Ok(())
    }

    /// EngineOutput に基づいて EditSession を発行し、Composition を更新する。
    fn update_composition(&self, context: &ITfContext, output: &EngineOutput) -> Result<()> {
        let action = if !output.committed.is_empty() && !output.display.is_empty() {
            // 候補確定と新規入力が同時に発生（例: Converting 中の InsertChar）
            EditAction::CommitAndCompose {
                committed: output.committed.clone(),
                display: output.display.clone(),
                cursor_pos: output.cursor_pos,
            }
        } else if !output.committed.is_empty() {
            EditAction::CommitText(output.committed.clone())
        } else if !output.display.is_empty() {
            EditAction::SetText {
                text: output.display.clone(),
                cursor_pos: output.cursor_pos,
            }
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
        debug_log(&format!(
            "update_composition: requesting edit session, tid={}",
            tid
        ));
        unsafe {
            let session_hr =
                context.RequestEditSession(tid, &session, TF_ES_READWRITE | TF_ES_SYNC)?;
            debug_log(&format!(
                "update_composition: RequestEditSession returned hr=0x{:08X}",
                session_hr.0
            ));
        }

        Ok(())
    }
}

// --- ITfTextInputProcessorEx ---

impl ITfTextInputProcessorEx_Impl for TextService_Impl {
    fn ActivateEx(&self, ptim: Option<&ITfThreadMgr>, tid: u32, _flags: u32) -> Result<()> {
        debug_log(&format!("ActivateEx called, tid={}", tid));

        let thread_mgr = ptim
            .ok_or_else(|| {
                debug_log("ActivateEx: ptim is None");
                E_INVALIDARG
            })?
            .clone();

        let keystroke_mgr: ITfKeystrokeMgr = thread_mgr.cast().map_err(|e| {
            debug_log(&format!("ActivateEx: ITfKeystrokeMgr cast failed: {:?}", e));
            e
        })?;
        let self_sink: ITfKeyEventSink = unsafe {
            self.cast().map_err(|e| {
                debug_log(&format!("ActivateEx: ITfKeyEventSink cast failed: {:?}", e));
                e
            })?
        };
        unsafe {
            keystroke_mgr
                .AdviseKeyEventSink(tid, &self_sink, TRUE)
                .map_err(|e| {
                    debug_log(&format!("ActivateEx: AdviseKeyEventSink failed: {:?}", e));
                    e
                })?;
        }

        debug_log("ActivateEx: AdviseKeyEventSink succeeded");

        // 入力モードトグルキー（半角/全角・Ctrl+Space 等）を予約キーとして登録する。
        // これがないと 半角/全角 のようなモード制御キーは OnKeyDown に配送されず、
        // トグルが機能しない。予約キーは OnPreservedKey に配送される。
        let specs = self.toggle_key.preserved_keys();
        let key_guids = guids::preservedkey_guids();
        let desc: Vec<u16> = "Enpitsu IME Toggle".encode_utf16().collect();
        let mut registered: Vec<(GUID, TF_PRESERVEDKEY)> = Vec::new();
        for (spec, guid) in specs.iter().zip(key_guids.iter()) {
            let key = TF_PRESERVEDKEY {
                uVKey: spec.vk as u32,
                uModifiers: spec.modifiers,
            };
            match unsafe { keystroke_mgr.PreserveKey(tid, guid, &key, &desc) } {
                Ok(()) => {
                    debug_log(&format!(
                        "ActivateEx: PreserveKey vk=0x{:02X} mod=0x{:X} registered",
                        spec.vk, spec.modifiers
                    ));
                    registered.push((*guid, key));
                }
                Err(e) => {
                    debug_log(&format!(
                        "ActivateEx: PreserveKey vk=0x{:02X} FAILED: {:?}",
                        spec.vk, e
                    ));
                }
            }
        }
        *self.preserved.lock().unwrap() = registered;

        *self.thread_mgr.lock().unwrap() = Some(thread_mgr);
        *self.client_id.lock().unwrap() = tid;
        *self.ime_on.lock().unwrap() = true;

        debug_log("ActivateEx completed successfully");
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
                    // 登録済みの予約キーを解除する。
                    for (guid, key) in self.preserved.lock().unwrap().drain(..) {
                        let _ = keystroke_mgr.UnpreserveKey(&guid, &key);
                    }
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
    fn OnSetFocus(&self, fforeground: BOOL) -> Result<()> {
        debug_log(&format!("OnSetFocus: fforeground={}", fforeground.0));
        Ok(())
    }

    fn OnTestKeyDown(
        &self,
        _pic: Option<&ITfContext>,
        wparam: WPARAM,
        _lparam: LPARAM,
    ) -> Result<BOOL> {
        debug_log(&format!("OnTestKeyDown ENTERED: wparam=0x{:02X}", wparam.0));
        let ime_on = *self.ime_on.lock().unwrap();
        let modifiers = modifiers_from_keyboard_state();
        let vk = wparam.0 as u16;

        // トグルキーは予約キーとして OnPreservedKey で処理されるため、ここでは扱わない。
        let result = if key_mapping::map_key(vk, &modifiers, ime_on, &self.ctrl_config).is_some() {
            // Direct 状態（未入力）では文字入力キー以外を消費せず、アプリに委ねる。
            // これがないと矢印・Backspace・Enter・Space 等が握り潰され、
            // カーソル移動や改行ができなくなる。
            let state = self.engine.lock().unwrap().state();
            state != EngineState::Direct || key_mapping::is_character_key(vk, &modifiers)
        } else {
            false
        };

        debug_log(&format!(
            "TEST vk=0x{:02X} ctrl={} ime={} eat={}",
            vk, modifiers.ctrl, ime_on, result
        ));

        Ok(if result { TRUE } else { FALSE })
    }

    fn OnKeyDown(&self, pic: Option<&ITfContext>, wparam: WPARAM, _lparam: LPARAM) -> Result<BOOL> {
        let modifiers = modifiers_from_keyboard_state();
        let vk = wparam.0 as u16;

        // トグルキー（半角/全角・Ctrl+Space 等）は予約キーとして登録され、
        // OnPreservedKey に配送される。ここで二重に処理すると 2 回トグルして
        // 元に戻ってしまうため、OnKeyDown ではトグル処理を行わない。

        let ime_on = *self.ime_on.lock().unwrap();

        let Some(command) = key_mapping::map_key(vk, &modifiers, ime_on, &self.ctrl_config) else {
            debug_log(&format!("OnKeyDown: vk=0x{:02X} not mapped, passing", vk));
            return Ok(FALSE);
        };

        debug_log(&format!(
            "OnKeyDown: vk=0x{:02X}, command={:?}",
            vk, command
        ));

        let mut engine = self.engine.lock().unwrap();
        // Direct 状態では文字入力キー以外は消費せずアプリに委ねる（OnTestKeyDown と整合）。
        if engine.state() == EngineState::Direct && !key_mapping::is_character_key(vk, &modifiers) {
            drop(engine);
            debug_log(&format!(
                "OnKeyDown: vk=0x{:02X} not consumed in Direct state, passing",
                vk
            ));
            return Ok(FALSE);
        }
        let output = engine.process(command);
        drop(engine);

        debug_log(&format!(
            "OnKeyDown: output committed='{}', display='{}'",
            output.committed, output.display
        ));

        if let Some(context) = pic {
            match self.update_composition(context, &output) {
                Ok(()) => debug_log("OnKeyDown: update_composition succeeded"),
                Err(e) => {
                    debug_log(&format!("OnKeyDown: update_composition FAILED: {:?}", e));
                    return Err(e);
                }
            }
        } else {
            debug_log("OnKeyDown: context is None, skipping composition update");
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

    // rguid は TSF 側が有効なポインタを渡す。トレイトのシグネチャは固定のため
    // unsafe を付けられず、null チェックの上で参照する。
    #[allow(clippy::not_unsafe_ptr_arg_deref)]
    fn OnPreservedKey(&self, pic: Option<&ITfContext>, rguid: *const GUID) -> Result<BOOL> {
        if rguid.is_null() {
            return Ok(FALSE);
        }
        let guid = unsafe { *rguid };
        let is_toggle = self
            .preserved
            .lock()
            .unwrap()
            .iter()
            .any(|(g, _)| *g == guid);
        if is_toggle {
            debug_log("OnPreservedKey: toggle key");
            self.handle_toggle(pic)?;
            Ok(TRUE)
        } else {
            debug_log("OnPreservedKey: unknown guid, passing");
            Ok(FALSE)
        }
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
