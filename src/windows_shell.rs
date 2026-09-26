/// windows_shell

use std::path::Path;

#[cfg(target_os = "windows")]
use std::os::windows::ffi::OsStrExt;

#[cfg(target_os = "windows")]
use windows_sys::Win32::UI::Shell::{
    SHOpenWithDialog,
    OPENASINFO,
    OAIF_EXEC,
};

#[cfg(target_os = "windows")]
pub fn open_with(path: &Path) -> Result<(), String> {
    let wide_path: Vec<u16> = path
        .as_os_str()
        .encode_wide()
        .chain(std::iter::once(0))
        .collect();

    let info = OPENASINFO {
        pcszFile: wide_path.as_ptr(),
        pcszClass: std::ptr::null(),
        oaifInFlags: OAIF_EXEC,
    };

    let hr = unsafe {
        SHOpenWithDialog(
            std::ptr::null_mut(),
            &info,
        )
    };

    if hr == 0 {
        Ok(())
    } else {
        Err(format!(
            "SHOpenWithDialog failed: HRESULT=0x{:08X}",
            hr as u32
        ))
    }
    
}


/// Windows以外では未対応
#[cfg(not(target_os = "windows"))]
pub fn open_with(_path: &Path) -> Result<(), String> {
    Err("「プログラムから開く」はWindowsでのみ利用できます".to_string())
}

