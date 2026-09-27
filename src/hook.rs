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
}

static EVENT_SENDER: OnceLock<SyncSender<KeyAudioEvent>> = OnceLock::new();
static KEY_STATE: [AtomicBool; 256] = {
    const INIT: AtomicBool = AtomicBool::new(false);
    [INIT; 256]
};

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
                let key_type = match kbd.vkCode as u16 {
                    VK_SPACE => KeyType::Space,
                    VK_RETURN => KeyType::Enter,
                    VK_BACK => KeyType::Backspace,
                    _ => KeyType::Regular,
                };

                if let Some(sender) = EVENT_SENDER.get() {
                    let _ = sender.try_send(KeyAudioEvent { key_type, is_press });
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
