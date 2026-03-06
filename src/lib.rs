pub mod candidate;
pub mod config;
pub mod dictionary;
pub mod engine;
pub mod guids;
pub mod input_state;
pub mod katakana;
pub mod key_mapping;
pub mod romaji;
pub mod user_dictionary;

#[cfg(windows)]
pub mod class_factory;
#[cfg(windows)]
pub mod registry;
#[cfg(windows)]
pub mod text_service;

// === DLL エクスポート (Windows 専用) ===

#[cfg(windows)]
pub(crate) mod dll_exports {
    use std::sync::atomic::{AtomicIsize, Ordering};

    use windows::Win32::Foundation::*;
    use windows::Win32::System::Com::*;
    use windows::Win32::System::Ole::SELFREG_E_CLASS;
    use windows::core::*;

    use crate::class_factory::ClassFactory;
    use crate::guids;

    static DLL_INSTANCE: AtomicIsize = AtomicIsize::new(0);

    pub fn dll_instance() -> HMODULE {
        HMODULE(DLL_INSTANCE.load(Ordering::Relaxed) as *mut _)
    }

    /// DLL ロード時に呼ばれる。
    #[unsafe(no_mangle)]
    unsafe extern "system" fn DllMain(
        hinstance: HMODULE,
        reason: u32,
        _reserved: *mut core::ffi::c_void,
    ) -> BOOL {
        if reason == 1 {
            // DLL_PROCESS_ATTACH
            DLL_INSTANCE.store(hinstance.0 as isize, Ordering::Relaxed);
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
        S_FALSE
    }

    /// COM サーバーをレジストリに登録する。
    #[unsafe(no_mangle)]
    extern "system" fn DllRegisterServer() -> HRESULT {
        match crate::registry::register_server(dll_instance()) {
            Ok(()) => S_OK,
            Err(_) => SELFREG_E_CLASS,
        }
    }

    /// COM サーバーのレジストリ登録を解除する。
    #[unsafe(no_mangle)]
    extern "system" fn DllUnregisterServer() -> HRESULT {
        match crate::registry::unregister_server() {
            Ok(()) => S_OK,
            Err(_) => SELFREG_E_CLASS,
        }
    }
}
