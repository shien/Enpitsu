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

/// Windows Store / immersive アプリ対応を示すカテゴリ。
/// このカテゴリを登録しないと Windows 10/11 の設定アプリがキーボードを一覧から除外する。
const GUID_TFCAT_TIPCAP_IMMERSIVESUPPORT: GUID = GUID {
    data1: 0x13A016DF,
    data2: 0x560B,
    data3: 0x46CD,
    data4: [0x94, 0x7A, 0x4C, 0x3A, 0xF1, 0xE0, 0xE3, 0x5D],
};

/// システムトレイ対応を示すカテゴリ。
const GUID_TFCAT_TIPCAP_SYSTRAYSUPPORT: GUID = GUID {
    data1: 0x25504FB4,
    data2: 0x7BAB,
    data3: 0x4BC1,
    data4: [0x9C, 0x69, 0xCF, 0x81, 0x89, 0x0F, 0x0E, 0xF5],
};

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
///
/// Windows 8 以降は `ITfInputProcessorProfileMgr::RegisterProfile` を使う。
/// 旧 API (`ITfInputProcessorProfiles::AddLanguageProfile`) では
/// Windows 設定アプリでキーボード追加が永続化されない問題がある。
fn register_profile(clsid: &GUID) -> Result<()> {
    use windows::Win32::Foundation::TRUE;
    use windows::Win32::UI::Input::KeyboardAndMouse::HKL;

    let profile_mgr: ITfInputProcessorProfileMgr =
        unsafe { CoCreateInstance(&CLSID_TF_InputProcessorProfiles, None, CLSCTX_INPROC_SERVER)? };

    unsafe {
        let display_name: Vec<u16> = IME_DISPLAY_NAME.encode_utf16().collect();
        profile_mgr.RegisterProfile(
            clsid,
            LANGID_JAPANESE,
            &guids::guid_profile(),
            &display_name,  // description
            &[],            // icon file (none)
            0,              // icon index
            HKL::default(), // no substitute keyboard
            0,              // preferred layout
            TRUE,           // enabled by default
            0,              // flags
        )?;
    }

    Ok(())
}

/// TSF カテゴリを登録する。
///
/// `GUID_TFCAT_TIP_KEYBOARD` に加えて、`GUID_TFCAT_TIPCAP_IMMERSIVESUPPORT` と
/// `GUID_TFCAT_TIPCAP_SYSTRAYSUPPORT` を登録する。これらがないと Windows 10/11 の
/// 設定アプリがキーボードを一覧に表示しない（追加しても消える）。
fn register_categories(clsid: &GUID) -> Result<()> {
    let category_mgr: ITfCategoryMgr =
        unsafe { CoCreateInstance(&CLSID_TF_CategoryMgr, None, CLSCTX_INPROC_SERVER)? };

    unsafe {
        category_mgr.RegisterCategory(clsid, &GUID_TFCAT_TIP_KEYBOARD, clsid)?;
        category_mgr.RegisterCategory(clsid, &GUID_TFCAT_TIPCAP_IMMERSIVESUPPORT, clsid)?;
        category_mgr.RegisterCategory(clsid, &GUID_TFCAT_TIPCAP_SYSTRAYSUPPORT, clsid)?;
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
    let profile_mgr: ITfInputProcessorProfileMgr =
        unsafe { CoCreateInstance(&CLSID_TF_InputProcessorProfiles, None, CLSCTX_INPROC_SERVER)? };

    unsafe {
        let _ = profile_mgr.UnregisterProfile(clsid, LANGID_JAPANESE, &guids::guid_profile(), 0);
    }

    Ok(())
}

/// TSF カテゴリを解除する。
fn unregister_categories(clsid: &GUID) -> Result<()> {
    let category_mgr: ITfCategoryMgr =
        unsafe { CoCreateInstance(&CLSID_TF_CategoryMgr, None, CLSCTX_INPROC_SERVER)? };

    unsafe {
        let _ = category_mgr.UnregisterCategory(clsid, &GUID_TFCAT_TIP_KEYBOARD, clsid);
        let _ = category_mgr.UnregisterCategory(clsid, &GUID_TFCAT_TIPCAP_IMMERSIVESUPPORT, clsid);
        let _ = category_mgr.UnregisterCategory(clsid, &GUID_TFCAT_TIPCAP_SYSTRAYSUPPORT, clsid);
    }

    Ok(())
}
