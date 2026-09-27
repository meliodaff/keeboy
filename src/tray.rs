use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use windows_sys::Win32::Foundation::{HWND, LPARAM, LRESULT, POINT, WPARAM};
use windows_sys::Win32::System::LibraryLoader::GetModuleHandleW;
use windows_sys::Win32::UI::Shell::{
    Shell_NotifyIconW, NIF_ICON, NIF_MESSAGE, NIF_TIP, NIM_ADD, NIM_DELETE, NOTIFYICONDATAW,
};
use windows_sys::Win32::UI::WindowsAndMessaging::{
    AppendMenuW, CreatePopupMenu, CreateWindowExW, DefWindowProcW, DestroyMenu,
    DestroyWindow, DispatchMessageW, GetCursorPos, GetMessageW, LoadIconW,
    PostMessageW, PostQuitMessage, RegisterClassW, SetForegroundWindow,
    TrackPopupMenuEx, TranslateMessage, IDI_APPLICATION, MF_CHECKED, MF_POPUP,
    MF_SEPARATOR, MF_STRING, MF_UNCHECKED, MSG, TPM_NONOTIFY, TPM_RETURNCMD,
    TPM_RIGHTBUTTON, WM_APP, WM_DESTROY, WM_LBUTTONUP, WM_NULL,
    WM_RBUTTONUP, WNDCLASSW,
};

use crate::audio::AudioState;

const WM_TRAYICON: u32 = WM_APP + 100;
const TRAY_ICON_ID: u32 = 1001;

// Menu IDs
const CMD_MUTE: u32 = 2001;
const CMD_EXIT: u32 = 2002;
const CMD_VOL_25: u32 = 2101;
const CMD_VOL_50: u32 = 2102;
const CMD_VOL_75: u32 = 2103;
const CMD_VOL_100: u32 = 2104;
const CMD_PACK_BASE: u32 = 2200; // 2200..2299 for soundpacks

static RUNNING: AtomicBool = AtomicBool::new(true);

fn to_wide_chars(s: &str) -> Vec<u16> {
    s.encode_utf16().chain(std::iter::once(0)).collect()
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
            hIcon: LoadIconW(0 as _, IDI_APPLICATION),
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

        let icon = LoadIconW(0 as _, IDI_APPLICATION);
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

    // 2. Volume Submenu
    let current_vol = audio.get_volume();
    let vol_options = [(25, CMD_VOL_25), (50, CMD_VOL_50), (75, CMD_VOL_75), (100, CMD_VOL_100)];
    for (vol, cmd) in vol_options {
        let flag = if current_vol == vol { MF_CHECKED } else { MF_UNCHECKED } | MF_STRING;
        let label = to_wide_chars(&format!("{}%", vol));
        AppendMenuW(volume_menu, flag, cmd as usize, label.as_ptr());
    }
    let vol_sub_title = to_wide_chars(&format!("Volume (Currently {}%)", current_vol));
    AppendMenuW(menu, MF_POPUP, volume_menu as usize, vol_sub_title.as_ptr());

    // Separator
    AppendMenuW(menu, MF_SEPARATOR, 0, std::ptr::null());

    // 3. Mute Toggle
    let mute_flag = if audio.is_muted() { MF_CHECKED } else { MF_UNCHECKED } | MF_STRING;
    let mute_title = to_wide_chars("Mute Sound");
    AppendMenuW(menu, mute_flag, CMD_MUTE as usize, mute_title.as_ptr());

    // Separator
    AppendMenuW(menu, MF_SEPARATOR, 0, std::ptr::null());

    // 4. Exit
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
        CMD_VOL_25 => {
            audio.set_volume(25);
            println!("[Keeboy] Volume set to 25%");
        }
        CMD_VOL_50 => {
            audio.set_volume(50);
            println!("[Keeboy] Volume set to 50%");
        }
        CMD_VOL_75 => {
            audio.set_volume(75);
            println!("[Keeboy] Volume set to 75%");
        }
        CMD_VOL_100 => {
            audio.set_volume(100);
            println!("[Keeboy] Volume set to 100%");
        }
        CMD_EXIT => {
            println!("[Keeboy] Exiting...");
            RUNNING.store(false, Ordering::SeqCst);
            PostQuitMessage(0);
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
