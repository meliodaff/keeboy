use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use windows_sys::Win32::Foundation::{CloseHandle, HANDLE, HWND, LPARAM, LRESULT, POINT, WPARAM, ERROR_ALREADY_EXISTS};
use windows_sys::Win32::System::LibraryLoader::GetModuleHandleW;
use windows_sys::Win32::System::Threading::CreateMutexW;
use windows_sys::Win32::UI::Shell::{
    Shell_NotifyIconW, NIF_ICON, NIF_MESSAGE, NIF_TIP, NIM_ADD, NIM_DELETE, NOTIFYICONDATAW,
};
use windows_sys::Win32::UI::WindowsAndMessaging::{
    AppendMenuW, CreatePopupMenu, CreateWindowExW, DefWindowProcW, DestroyMenu,
    DestroyWindow, DispatchMessageW, GetCursorPos, GetMessageW, GetSystemMetrics,
    LoadIconW, LoadImageW, MessageBoxW, PostMessageW, PostQuitMessage, RegisterClassW,
    SetForegroundWindow, TrackPopupMenuEx, TranslateMessage, IDI_APPLICATION, IMAGE_ICON,
    LR_DEFAULTCOLOR, MB_ICONINFORMATION, MB_OK, MB_SETFOREGROUND, MB_TOPMOST, MF_CHECKED,
    MF_POPUP, MF_SEPARATOR, MF_STRING, MF_UNCHECKED, MSG, SM_CXICON, SM_CXSMICON, SM_CYICON,
    SM_CYSMICON, TPM_NONOTIFY, TPM_RETURNCMD, TPM_RIGHTBUTTON, WM_APP, WM_DESTROY,
    WM_LBUTTONUP, WM_NULL, WM_RBUTTONUP, WNDCLASSW,
};

use crate::audio::AudioState;

const WM_TRAYICON: u32 = WM_APP + 100;
const TRAY_ICON_ID: u32 = 1001;

/// Icon resource ID embedded by build.rs (see assets/keeboy.rc).
const IDI_KEEBOY: u32 = 1;

// Menu IDs
const CMD_MUTE: u32 = 2001;
const CMD_EXIT: u32 = 2002;
const CMD_VOL_BASE: u32 = 2100; // 2100..2199 master volume steps
const CMD_RELEASE_BASE: u32 = 2300; // 2300..2399 key-release volume steps
const CMD_PACK_VOL_BASE: u32 = 2400; // 2400..2499 current-profile trim steps
const CMD_PACK_BASE: u32 = 2200; // 2200..2299 for soundpacks

/// Master volume steps offered in the tray menu.
const MASTER_STEPS: [u32; 8] = [10, 25, 40, 50, 65, 75, 100, 125];
/// Key-release (upstroke) steps. 0 silences release clicks but keeps keypresses.
const RELEASE_STEPS: [u32; 6] = [0, 25, 50, 75, 100, 125];
/// Per-profile trim steps.
const PACK_STEPS: [u32; 5] = [50, 75, 100, 125, 150];

static RUNNING: AtomicBool = AtomicBool::new(true);

fn to_wide_chars(s: &str) -> Vec<u16> {
    s.encode_utf16().chain(std::iter::once(0)).collect()
}

/// Named-mutex guard that keeps a second copy of Keeboy from installing a second
/// keyboard hook (which would double every keystroke sound).
pub struct SingleInstance(HANDLE);

impl SingleInstance {
    pub fn acquire(name: &str) -> Option<Self> {
        unsafe {
            let wide = to_wide_chars(name);
            let handle = CreateMutexW(std::ptr::null(), 1, wide.as_ptr());
            if handle.is_null() {
                // Can't tell — let the app run rather than blocking it outright.
                return Some(SingleInstance(std::ptr::null_mut()));
            }
            if windows_sys::Win32::Foundation::GetLastError() == ERROR_ALREADY_EXISTS {
                CloseHandle(handle);
                return None;
            }
            Some(SingleInstance(handle))
        }
    }
}

impl Drop for SingleInstance {
    fn drop(&mut self) {
        unsafe {
            if !self.0.is_null() {
                CloseHandle(self.0);
            }
        }
    }
}

/// Shows a native message box. Needed because release builds have no console.
pub fn alert(title: &str, body: &str) {
    unsafe {
        let title_w = to_wide_chars(title);
        let body_w = to_wide_chars(body);
        MessageBoxW(
            std::ptr::null_mut(),
            body_w.as_ptr(),
            title_w.as_ptr(),
            MB_OK | MB_ICONINFORMATION | MB_SETFOREGROUND | MB_TOPMOST,
        );
    }
}

/// Loads the embedded keycap icon at the requested metric size, falling back to the
/// stock Windows application icon if the resource is missing.
unsafe fn load_app_icon(
    instance: windows_sys::Win32::Foundation::HMODULE,
    cx_metric: i32,
    cy_metric: i32,
) -> windows_sys::Win32::UI::WindowsAndMessaging::HICON {
    let handle = LoadImageW(
        instance,
        IDI_KEEBOY as *const u16, // MAKEINTRESOURCE
        IMAGE_ICON,
        GetSystemMetrics(cx_metric),
        GetSystemMetrics(cy_metric),
        LR_DEFAULTCOLOR,
    );
    if !handle.is_null() {
        return handle as _;
    }
    LoadIconW(std::ptr::null_mut(), IDI_APPLICATION)
}

unsafe extern "system" fn tray_window_proc(
    hwnd: HWND,
    msg: u32,
    w_param: WPARAM,
    l_param: LPARAM,
) -> LRESULT {
    match msg {
        WM_DESTROY => {
            PostQuitMessage(0);
            0
        }
        _ => DefWindowProcW(hwnd, msg, w_param, l_param),
    }
}

pub struct TrayIcon {
    hwnd: HWND,
    nid: NOTIFYICONDATAW,
}

impl Drop for TrayIcon {
    fn drop(&mut self) {
        unsafe {
            Shell_NotifyIconW(NIM_DELETE, &mut self.nid);
            if !self.hwnd.is_null() {
                DestroyWindow(self.hwnd);
            }
        }
    }
}

pub fn run_tray_loop(audio: Arc<AudioState>) {
    unsafe {
        let instance = GetModuleHandleW(std::ptr::null());
        let class_name = to_wide_chars("KeeboyTrayWindowClass");

        let wnd_class = WNDCLASSW {
            style: 0,
            lpfnWndProc: Some(tray_window_proc),
            cbClsExtra: 0,
            cbWndExtra: 0,
            hInstance: instance,
            hIcon: load_app_icon(instance, SM_CXICON, SM_CYICON),
            hCursor: 0 as _,
            hbrBackground: 0 as _,
            lpszMenuName: std::ptr::null(),
            lpszClassName: class_name.as_ptr(),
        };

        RegisterClassW(&wnd_class);

        let window_title = to_wide_chars("Keeboy Background Window");
        let hwnd = CreateWindowExW(
            0,
            class_name.as_ptr(),
            window_title.as_ptr(),
            0,
            0,
            0,
            0,
            0,
            0 as _,
            0 as _,
            instance,
            std::ptr::null(),
        );

        if hwnd.is_null() {
            eprintln!("[Keeboy] Failed to create tray background window.");
            return;
        }

        // Tray icons want the *small* system metric so they stay crisp.
        let icon = load_app_icon(instance, SM_CXSMICON, SM_CYSMICON);
        let mut nid: NOTIFYICONDATAW = std::mem::zeroed();
        nid.cbSize = std::mem::size_of::<NOTIFYICONDATAW>() as u32;
        nid.hWnd = hwnd;
        nid.uID = TRAY_ICON_ID;
        nid.uFlags = NIF_MESSAGE | NIF_TIP;
        nid.uCallbackMessage = WM_TRAYICON;

        if !icon.is_null() {
            nid.uFlags |= NIF_ICON;
            nid.hIcon = icon;
        }

        let tip_str = to_wide_chars("Keeboy - Mechanical Keyboard Sound");
        let copy_len = tip_str.len().min(nid.szTip.len());
        nid.szTip[..copy_len].copy_from_slice(&tip_str[..copy_len]);

        let mut success = Shell_NotifyIconW(NIM_ADD, &mut nid) != 0;
        if !success {
            // Fallback: Try with V2 struct size if Windows Shell is strict
            nid.cbSize = 936;
            success = Shell_NotifyIconW(NIM_ADD, &mut nid) != 0;
        }

        if !success {
            let err = windows_sys::Win32::Foundation::GetLastError();
            eprintln!("[Keeboy] Note: System tray icon could not be registered (Error {}). Hook and sound are still active.", err);
        } else {
            println!("[Keeboy] System Tray icon active.");
        }

        let _tray_guard = TrayIcon { hwnd, nid };

        let mut msg: MSG = std::mem::zeroed();
        while RUNNING.load(Ordering::Relaxed) && GetMessageW(&mut msg, 0 as _, 0, 0) > 0 {
            if msg.message == WM_TRAYICON {
                let event = msg.lParam as u32;
                if event == WM_RBUTTONUP || event == WM_LBUTTONUP {
                    show_context_menu(hwnd, &audio);
                }
            } else {
                TranslateMessage(&msg);
                DispatchMessageW(&msg);
            }
        }
    }
}

unsafe fn show_context_menu(hwnd: HWND, audio: &Arc<AudioState>) {
    let mut pt: POINT = std::mem::zeroed();
    GetCursorPos(&mut pt);

    let menu = CreatePopupMenu();
    let soundpack_menu = CreatePopupMenu();
    let volume_menu = CreatePopupMenu();
    let release_menu = CreatePopupMenu();
    let pack_volume_menu = CreatePopupMenu();

    // 1. Soundpack Submenu
    let packs = audio.soundpack_names();
    let current_pack = audio.current_soundpack_index();

    for (i, name) in packs.iter().enumerate() {
        let flag = if i == current_pack { MF_CHECKED } else { MF_UNCHECKED } | MF_STRING;
        let wide_name = to_wide_chars(name);
        AppendMenuW(soundpack_menu, flag, (CMD_PACK_BASE + i as u32) as usize, wide_name.as_ptr());
    }

    let pack_sub_title = to_wide_chars("Switch Sound Profile");
    AppendMenuW(menu, MF_POPUP, soundpack_menu as usize, pack_sub_title.as_ptr());

    // 2. Master volume submenu
    let current_vol = audio.get_volume();
    for (i, step) in MASTER_STEPS.iter().enumerate() {
        let flag = if current_vol == *step { MF_CHECKED } else { MF_UNCHECKED } | MF_STRING;
        let label = to_wide_chars(&format!("{}%", step));
        AppendMenuW(volume_menu, flag, (CMD_VOL_BASE + i as u32) as usize, label.as_ptr());
    }
    let vol_sub_title = to_wide_chars(&format!("Master Volume (Currently {}%)", current_vol));
    AppendMenuW(menu, MF_POPUP, volume_menu as usize, vol_sub_title.as_ptr());

    // 3. Key-release volume submenu (independent of master)
    let current_release = audio.get_release_volume();
    for (i, step) in RELEASE_STEPS.iter().enumerate() {
        let flag = if current_release == *step { MF_CHECKED } else { MF_UNCHECKED } | MF_STRING;
        let label = if *step == 0 {
            to_wide_chars("Off (presses only)")
        } else {
            to_wide_chars(&format!("{}%", step))
        };
        AppendMenuW(release_menu, flag, (CMD_RELEASE_BASE + i as u32) as usize, label.as_ptr());
    }
    let release_title = if current_release == 0 {
        to_wide_chars("Key Release Volume (Off)")
    } else {
        to_wide_chars(&format!("Key Release Volume (Currently {}%)", current_release))
    };
    AppendMenuW(menu, MF_POPUP, release_menu as usize, release_title.as_ptr());

    // 4. Per-profile trim submenu, so profiles can be level-matched
    let current_pack_vol = audio.get_pack_volume(current_pack);
    for (i, step) in PACK_STEPS.iter().enumerate() {
        let flag = if current_pack_vol == *step { MF_CHECKED } else { MF_UNCHECKED } | MF_STRING;
        let label = to_wide_chars(&format!("{}%", step));
        AppendMenuW(pack_volume_menu, flag, (CMD_PACK_VOL_BASE + i as u32) as usize, label.as_ptr());
    }
    let pack_vol_title =
        to_wide_chars(&format!("This Profile's Volume (Currently {}%)", current_pack_vol));
    AppendMenuW(menu, MF_POPUP, pack_volume_menu as usize, pack_vol_title.as_ptr());

    // Separator
    AppendMenuW(menu, MF_SEPARATOR, 0, std::ptr::null());

    // 5. Mute Toggle
    let mute_flag = if audio.is_muted() { MF_CHECKED } else { MF_UNCHECKED } | MF_STRING;
    let mute_title = to_wide_chars("Mute Sound");
    AppendMenuW(menu, mute_flag, CMD_MUTE as usize, mute_title.as_ptr());

    // Separator
    AppendMenuW(menu, MF_SEPARATOR, 0, std::ptr::null());

    // 6. Exit
    let exit_title = to_wide_chars("Exit Keeboy");
    AppendMenuW(menu, MF_STRING, CMD_EXIT as usize, exit_title.as_ptr());

    // Windows requires SetForegroundWindow before TrackPopupMenu
    SetForegroundWindow(hwnd);

    let chosen_cmd = TrackPopupMenuEx(
        menu,
        TPM_RETURNCMD | TPM_RIGHTBUTTON | TPM_NONOTIFY,
        pt.x,
        pt.y,
        hwnd,
        std::ptr::null(),
    ) as u32;

    DestroyMenu(menu);
    PostMessageW(hwnd, WM_NULL, 0, 0);

    match chosen_cmd {
        CMD_MUTE => {
            let muted = audio.toggle_mute();
            println!("[Keeboy] Mute toggled: {}", if muted { "MUTED" } else { "ACTIVE" });
        }
        CMD_EXIT => {
            println!("[Keeboy] Exiting...");
            RUNNING.store(false, Ordering::SeqCst);
            PostQuitMessage(0);
        }
        cmd if cmd >= CMD_VOL_BASE && ((cmd - CMD_VOL_BASE) as usize) < MASTER_STEPS.len() => {
            let vol = MASTER_STEPS[(cmd - CMD_VOL_BASE) as usize];
            audio.set_volume(vol);
            println!("[Keeboy] Master volume set to {}%", vol);
        }
        cmd if cmd >= CMD_RELEASE_BASE
            && ((cmd - CMD_RELEASE_BASE) as usize) < RELEASE_STEPS.len() =>
        {
            let vol = RELEASE_STEPS[(cmd - CMD_RELEASE_BASE) as usize];
            audio.set_release_volume(vol);
            println!("[Keeboy] Key release volume set to {}%", vol);
        }
        cmd if cmd >= CMD_PACK_VOL_BASE
            && ((cmd - CMD_PACK_VOL_BASE) as usize) < PACK_STEPS.len() =>
        {
            let vol = PACK_STEPS[(cmd - CMD_PACK_VOL_BASE) as usize];
            let idx = audio.current_soundpack_index();
            audio.set_pack_volume(idx, vol);
            println!("[Keeboy] Profile volume set to {}%", vol);
        }
        cmd if cmd >= CMD_PACK_BASE && cmd < CMD_PACK_BASE + 100 => {
            let pack_idx = (cmd - CMD_PACK_BASE) as usize;
            audio.set_soundpack(pack_idx);
            let name = audio.soundpack_names().get(pack_idx).cloned().unwrap_or_default();
            println!("[Keeboy] Switch profile changed to: {}", name);
        }
        _ => {}
    }
}
