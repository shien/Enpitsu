//! COM サーバーと TSF プロファイルのレジストリ登録。

use std::ffi::OsString;
use std::os::windows::ffi::OsStringExt;

use windows::Win32::Foundation::*;
use windows::Win32::System::Com::*;
use windows::Win32::System::LibraryLoader::GetModuleFileNameW;
use windows::Win32::System::Registry::*;
use windows::Win32::UI::TextServices::*;
use windows::core::*;

use crate::guids;

const IME_DISPLAY_NAME: &str = "Enpitsu";
const LANGID_JAPANESE: u16 = 0x0411;

/// COM サーバーをレジストリに登録する。
pub fn register_server(dll_instance: HMODULE) -> Result<()> {
    unsafe { CoInitializeEx(None, COINIT_APARTMENTTHREADED).ok()? };
    let result = register_server_inner(dll_instance);
    unsafe { CoUninitialize() };
    result
}

fn register_server_inner(dll_instance: HMODULE) -> Result<()> {
    let dll_path = get_dll_path(dll_instance)?;
    let clsid = guids::clsid_text_service();
    let clsid_str = guid_to_string(&clsid);

    register_clsid(&clsid_str, &dll_path)?;
    register_profile(&clsid)?;
    register_categories(&clsid)?;

    Ok(())
}

/// COM サーバーのレジストリ登録を解除する。
pub fn unregister_server() -> Result<()> {
    unsafe { CoInitializeEx(None, COINIT_APARTMENTTHREADED).ok()? };
    let result = unregister_server_inner();
    unsafe { CoUninitialize() };
    result
}

fn unregister_server_inner() -> Result<()> {
    let clsid = guids::clsid_text_service();
    let clsid_str = guid_to_string(&clsid);

    unregister_categories(&clsid)?;
    unregister_profile(&clsid)?;
    unregister_clsid(&clsid_str)?;

    Ok(())
}

/// DLL のフルパスを取得する。
fn get_dll_path(dll_instance: HMODULE) -> Result<String> {
    let mut buf = [0u16; 260];
    let len = unsafe { GetModuleFileNameW(dll_instance, &mut buf) } as usize;
    if len == 0 {
        return Err(Error::from_win32());
    }
    let path = OsString::from_wide(&buf[..len]);
    path.into_string().map_err(|_| Error::from_hresult(E_FAIL))
}

/// GUID を "{...}" 形式の文字列に変換する。
fn guid_to_string(guid: &GUID) -> String {
    format!(
        "{{{:08X}-{:04X}-{:04X}-{:02X}{:02X}-{:02X}{:02X}{:02X}{:02X}{:02X}{:02X}}}",
        guid.data1,
        guid.data2,
        guid.data3,
        guid.data4[0],
        guid.data4[1],
        guid.data4[2],
        guid.data4[3],
        guid.data4[4],
        guid.data4[5],
        guid.data4[6],
        guid.data4[7],
    )
}

/// CLSID をレジストリに登録する。
fn register_clsid(clsid_str: &str, dll_path: &str) -> Result<()> {
    let key_path = format!("CLSID\\{clsid_str}\\InProcServer32");
    let hkey = unsafe {
        let mut hkey = HKEY::default();
        RegCreateKeyExW(
            HKEY_CLASSES_ROOT,
            &HSTRING::from(&key_path),
            0,
            None,
            REG_OPTION_NON_VOLATILE,
            KEY_WRITE,
            None,
            &mut hkey,
            None,
        )
        .ok()?;
        hkey
    };

    unsafe {
        let wide_path: Vec<u16> = dll_path.encode_utf16().chain(std::iter::once(0)).collect();
        RegSetValueExW(
            hkey,
            None,
            0,
            REG_SZ,
            Some(std::slice::from_raw_parts(
                wide_path.as_ptr() as *const u8,
                wide_path.len() * 2,
            )),
        )
        .ok()?;

        let threading = "Apartment\0";
        let wide_threading: Vec<u16> = threading.encode_utf16().collect();
        RegSetValueExW(
            hkey,
            &HSTRING::from("ThreadingModel"),
            0,
            REG_SZ,
            Some(std::slice::from_raw_parts(
                wide_threading.as_ptr() as *const u8,
                wide_threading.len() * 2,
            )),
        )
        .ok()?;

        RegCloseKey(hkey).ok()?;
    }

    Ok(())
}

/// TSF プロファイルを登録する。
fn register_profile(clsid: &GUID) -> Result<()> {
    let profiles: ITfInputProcessorProfiles =
        unsafe { CoCreateInstance(&CLSID_TF_InputProcessorProfiles, None, CLSCTX_INPROC_SERVER)? };

    unsafe {
        profiles.Register(clsid)?;

        let display_name: Vec<u16> = IME_DISPLAY_NAME
            .encode_utf16()
            .chain(std::iter::once(0))
            .collect();
        profiles.AddLanguageProfile(
            clsid,
            LANGID_JAPANESE,
            &guids::guid_profile(),
            &display_name,
            &[],                 // icon file
            0,                   // icon index
        )?;
    }

    Ok(())
}

/// TSF カテゴリを登録する。
fn register_categories(clsid: &GUID) -> Result<()> {
    let category_mgr: ITfCategoryMgr =
        unsafe { CoCreateInstance(&CLSID_TF_CategoryMgr, None, CLSCTX_INPROC_SERVER)? };

    unsafe {
        category_mgr.RegisterCategory(clsid, &GUID_TFCAT_TIP_KEYBOARD, clsid)?;
    }

    Ok(())
}

/// CLSID をレジストリから解除する。
fn unregister_clsid(clsid_str: &str) -> Result<()> {
    let key_path = format!("CLSID\\{clsid_str}");
    unsafe {
        let _ = RegDeleteTreeW(HKEY_CLASSES_ROOT, &HSTRING::from(&key_path));
    }
    Ok(())
}

/// TSF プロファイルを解除する。
fn unregister_profile(clsid: &GUID) -> Result<()> {
    let profiles: ITfInputProcessorProfiles =
        unsafe { CoCreateInstance(&CLSID_TF_InputProcessorProfiles, None, CLSCTX_INPROC_SERVER)? };

    unsafe {
        let _ = profiles.RemoveLanguageProfile(clsid, LANGID_JAPANESE, &guids::guid_profile());
        let _ = profiles.Unregister(clsid);
    }

    Ok(())
}

/// TSF カテゴリを解除する。
fn unregister_categories(clsid: &GUID) -> Result<()> {
    let category_mgr: ITfCategoryMgr =
        unsafe { CoCreateInstance(&CLSID_TF_CategoryMgr, None, CLSCTX_INPROC_SERVER)? };

    unsafe {
        let _ = category_mgr.UnregisterCategory(clsid, &GUID_TFCAT_TIP_KEYBOARD, clsid);
    }

    Ok(())
}
