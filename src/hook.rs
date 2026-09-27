use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc::SyncSender;
use std::sync::OnceLock;
use windows_sys::Win32::Foundation::{HINSTANCE, LPARAM, LRESULT, WPARAM};
use windows_sys::Win32::UI::WindowsAndMessaging::{
    CallNextHookEx, GetMessageW, SetWindowsHookExW, UnhookWindowsHookEx,
    HC_ACTION, HHOOK, KBDLLHOOKSTRUCT, MSG,
    WH_KEYBOARD_LL, WM_KEYDOWN, WM_KEYUP, WM_SYSKEYDOWN, WM_SYSKEYUP,
};
use crate::audio::KeyType;

const VK_BACK: u16 = 0x08;
const VK_RETURN: u16 = 0x0D;
const VK_SPACE: u16 = 0x20;

pub struct KeyAudioEvent {
    pub key_type: KeyType,
    pub is_press: bool,
    pub pan: f32,
}

static EVENT_SENDER: OnceLock<SyncSender<KeyAudioEvent>> = OnceLock::new();
static KEY_STATE: [AtomicBool; 256] = {
    const INIT: AtomicBool = AtomicBool::new(false);
    [INIT; 256]
};

fn vk_to_pan(vk: u16) -> f32 {
    match vk {
        // Far left keys: Tab, Caps, LShift, LCtrl, Escape, `, 1, Q, A, Z
        0x1B | 0xC0 | 0x31 | 0x51 | 0x41 | 0x5A | 0x09 | 0x14 | 0xA0 | 0xA2 => -0.40,
        // Mid-left keys: 2, 3, W, E, S, D, X, C
        0x32 | 0x33 | 0x57 | 0x45 | 0x53 | 0x44 | 0x58 | 0x43 => -0.22,
        // Center keys: 4, 5, 6, R, T, Y, F, G, H, V, B, N, Space
        0x34 | 0x35 | 0x36 | 0x52 | 0x54 | 0x59 | 0x46 | 0x47 | 0x48 | 0x56 | 0x42 | 0x4E | 0x20 => 0.0,
        // Mid-right keys: 7, 8, U, I, J, K, M, ,
        0x37 | 0x38 | 0x55 | 0x49 | 0x4A | 0x4B | 0x4D | 0xBC => 0.22,
        // Far right keys: 9, 0, -, =, O, P, [, ], L, ;, ', Enter, Backspace, RShift, Arrows
        _ => 0.40,
    }
}

unsafe extern "system" fn keyboard_hook_proc(
    n_code: i32,
    w_param: WPARAM,
    l_param: LPARAM,
) -> LRESULT {
    if n_code == HC_ACTION as i32 {
        let is_press = w_param == WM_KEYDOWN as WPARAM || w_param == WM_SYSKEYDOWN as WPARAM;
        let is_release = w_param == WM_KEYUP as WPARAM || w_param == WM_SYSKEYUP as WPARAM;

        if is_press || is_release {
            let kbd = *(l_param as *const KBDLLHOOKSTRUCT);
            let vk = (kbd.vkCode as usize) & 0xFF;

            let should_trigger = if is_press {
                // If the key is already pressed, this is an OS auto-repeat.
                // In real physical switches, holding down doesn't re-trigger a physical click.
                !KEY_STATE[vk].swap(true, Ordering::Relaxed)
            } else {
                KEY_STATE[vk].store(false, Ordering::Relaxed);
                true
            };

            if should_trigger {
                let vk_code = kbd.vkCode as u16;
                let key_type = match vk_code {
                    VK_SPACE => KeyType::Space,
                    VK_RETURN => KeyType::Enter,
                    VK_BACK => KeyType::Backspace,
                    _ => KeyType::Regular,
                };
                let pan = vk_to_pan(vk_code);

                if let Some(sender) = EVENT_SENDER.get() {
                    let _ = sender.try_send(KeyAudioEvent { key_type, is_press, pan });
                }
            }
        }
    }

    CallNextHookEx(std::ptr::null_mut(), n_code, w_param, l_param)
}

pub struct KeyboardHookGuard {
    hook: HHOOK,
}

impl Drop for KeyboardHookGuard {
    fn drop(&mut self) {
        if !self.hook.is_null() {
            unsafe {
                UnhookWindowsHookEx(self.hook);
            }
            println!("[Keeboy] Keyboard hook uninstalled.");
        }
    }
}

pub fn start_keyboard_hook(sender: SyncSender<KeyAudioEvent>) -> std::thread::JoinHandle<()> {
    let _ = EVENT_SENDER.set(sender);

    std::thread::spawn(|| {
        unsafe {
            let hook = SetWindowsHookExW(
                WH_KEYBOARD_LL,
                Some(keyboard_hook_proc),
                0 as HINSTANCE,
                0,
            );

            if hook.is_null() {
                eprintln!("[Keeboy] ERROR: Failed to install low-level keyboard hook!");
                return;
            }

            println!("[Keeboy] Low-level keyboard hook installed successfully.");
            let _guard = KeyboardHookGuard { hook };

            // Windows message loop required to keep the hook active
            let mut msg: MSG = std::mem::zeroed();
            while GetMessageW(&mut msg, std::ptr::null_mut(), 0, 0) > 0 {
                // Low-level hooks are dispatched directly by the OS in this thread's message loop
            }
        }
    })
}
