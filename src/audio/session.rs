//! Windows Volume Mixer integration.
//!
//! Keeboy already gets its own slider in the mixer simply by rendering audio, but the
//! session is unlabelled by default. This names it and points it at the embedded keycap
//! icon so it's obvious which slider controls typing sounds.
//!
//! The Core Audio COM interfaces are declared by hand (windows-sys exposes interfaces as
//! opaque pointers) to avoid pulling in the much heavier `windows` crate.

use std::ffi::c_void;
use windows_sys::core::{GUID, HRESULT, PCWSTR, PWSTR};
use windows_sys::Win32::System::Com::{
    CoCreateInstance, CoInitializeEx, CLSCTX_ALL, COINIT_MULTITHREADED,
};
use windows_sys::Win32::System::LibraryLoader::GetModuleFileNameW;

const CLSID_MM_DEVICE_ENUMERATOR: GUID = GUID::from_u128(0xBCDE0395_E52F_467C_8E3D_C4579291692E);
const IID_IMM_DEVICE_ENUMERATOR: GUID = GUID::from_u128(0xA95664D2_9614_4F35_A746_DE8DB63617E6);
const IID_IAUDIO_SESSION_MANAGER2: GUID = GUID::from_u128(0x77AA99A0_1BD6_484F_8BC7_2C654C9A9B6F);

/// eRender
const DATA_FLOW_RENDER: i32 = 0;
/// eMultimedia
const ROLE_MULTIMEDIA: i32 = 1;

const S_OK: HRESULT = 0;

#[repr(C)]
struct IUnknownVtbl {
    query_interface:
        unsafe extern "system" fn(*mut c_void, *const GUID, *mut *mut c_void) -> HRESULT,
    add_ref: unsafe extern "system" fn(*mut c_void) -> u32,
    release: unsafe extern "system" fn(*mut c_void) -> u32,
}

#[repr(C)]
struct IMMDeviceEnumeratorVtbl {
    base: IUnknownVtbl,
    enum_audio_endpoints:
        unsafe extern "system" fn(*mut c_void, i32, u32, *mut *mut c_void) -> HRESULT,
    get_default_audio_endpoint:
        unsafe extern "system" fn(*mut c_void, i32, i32, *mut *mut c_void) -> HRESULT,
    // Remaining methods are unused and therefore not declared.
}

#[repr(C)]
struct IMMDeviceVtbl {
    base: IUnknownVtbl,
    activate: unsafe extern "system" fn(
        *mut c_void,
        *const GUID,
        u32,
        *const c_void,
        *mut *mut c_void,
    ) -> HRESULT,
}

#[repr(C)]
struct IAudioSessionManager2Vtbl {
    base: IUnknownVtbl,
    get_audio_session_control:
        unsafe extern "system" fn(*mut c_void, *const GUID, u32, *mut *mut c_void) -> HRESULT,
    // Remaining methods are unused.
}

#[repr(C)]
struct IAudioSessionControlVtbl {
    base: IUnknownVtbl,
    get_state: unsafe extern "system" fn(*mut c_void, *mut i32) -> HRESULT,
    get_display_name: unsafe extern "system" fn(*mut c_void, *mut PWSTR) -> HRESULT,
    set_display_name: unsafe extern "system" fn(*mut c_void, PCWSTR, *const GUID) -> HRESULT,
    get_icon_path: unsafe extern "system" fn(*mut c_void, *mut PWSTR) -> HRESULT,
    set_icon_path: unsafe extern "system" fn(*mut c_void, PCWSTR, *const GUID) -> HRESULT,
}

/// Owned COM interface pointer; releases on drop.
struct ComPtr(*mut c_void);

impl ComPtr {
    fn as_raw(&self) -> *mut c_void {
        self.0
    }

    /// # Safety
    /// `T` must match the actual vtable layout of the interface this pointer holds.
    unsafe fn vtbl<T>(&self) -> *const T {
        *(self.0 as *const *const T)
    }
}

impl Drop for ComPtr {
    fn drop(&mut self) {
        unsafe {
            if !self.0.is_null() {
                let vtbl = self.vtbl::<IUnknownVtbl>();
                ((*vtbl).release)(self.0);
            }
        }
    }
}

fn to_wide(s: &str) -> Vec<u16> {
    s.encode_utf16().chain(std::iter::once(0)).collect()
}

/// Full path of the running executable, used as the mixer icon source.
fn exe_path() -> Option<String> {
    unsafe {
        let mut buf = [0u16; 512];
        let len = GetModuleFileNameW(std::ptr::null_mut(), buf.as_mut_ptr(), buf.len() as u32);
        if len == 0 || len as usize >= buf.len() {
            return None;
        }
        Some(String::from_utf16_lossy(&buf[..len as usize]))
    }
}

/// Labels this process's audio session so the Windows Volume Mixer shows "Keeboy" with
/// the app icon instead of a bare executable name.
///
/// Call this from the thread that owns the audio output stream, after the stream exists.
/// Failure is non-fatal: the slider still works, it just keeps the default label.
pub fn label_mixer_session(display_name: &str) -> Result<(), HRESULT> {
    unsafe {
        // cpal may already have initialised COM on this thread; a differing mode returns
        // RPC_E_CHANGED_MODE, which is fine for our purposes.
        let _ = CoInitializeEx(std::ptr::null(), COINIT_MULTITHREADED as u32);

        let mut enumerator: *mut c_void = std::ptr::null_mut();
        let hr = CoCreateInstance(
            &CLSID_MM_DEVICE_ENUMERATOR,
            std::ptr::null_mut(),
            CLSCTX_ALL,
            &IID_IMM_DEVICE_ENUMERATOR,
            &mut enumerator,
        );
        if hr != S_OK || enumerator.is_null() {
            return Err(hr);
        }
        let enumerator = ComPtr(enumerator);

        let mut device: *mut c_void = std::ptr::null_mut();
        let hr = ((*enumerator.vtbl::<IMMDeviceEnumeratorVtbl>()).get_default_audio_endpoint)(
            enumerator.as_raw(),
            DATA_FLOW_RENDER,
            ROLE_MULTIMEDIA,
            &mut device,
        );
        if hr != S_OK || device.is_null() {
            return Err(hr);
        }
        let device = ComPtr(device);

        let mut manager: *mut c_void = std::ptr::null_mut();
        let hr = ((*device.vtbl::<IMMDeviceVtbl>()).activate)(
            device.as_raw(),
            &IID_IAUDIO_SESSION_MANAGER2,
            CLSCTX_ALL,
            std::ptr::null(),
            &mut manager,
        );
        if hr != S_OK || manager.is_null() {
            return Err(hr);
        }
        let manager = ComPtr(manager);

        // A null GUID asks for this process's own default session.
        let mut control: *mut c_void = std::ptr::null_mut();
        let hr = ((*manager.vtbl::<IAudioSessionManager2Vtbl>()).get_audio_session_control)(
            manager.as_raw(),
            std::ptr::null(),
            0,
            &mut control,
        );
        if hr != S_OK || control.is_null() {
            return Err(hr);
        }
        let control = ComPtr(control);
        let vtbl = control.vtbl::<IAudioSessionControlVtbl>();

        let name = to_wide(display_name);
        let hr = ((*vtbl).set_display_name)(control.as_raw(), name.as_ptr(), std::ptr::null());
        if hr != S_OK {
            return Err(hr);
        }

        // ",0" selects the executable's first embedded icon (the keycap).
        if let Some(path) = exe_path() {
            let icon = to_wide(&format!("{},0", path));
            let _ = ((*vtbl).set_icon_path)(control.as_raw(), icon.as_ptr(), std::ptr::null());
        }

        Ok(())
    }
}
