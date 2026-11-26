//! System-wide keyboard access for Quick Accent: a low-level keyboard hook
//! (sees every key before applications do, and may hold one back) and typed
//! text injection.
//!
//! The hook runs on its own thread with a message loop, as Windows requires.
//! Its callback must stay fast: Windows silently drops hooks that take more
//! than about a second, so the handler only updates state and queues work.

use std::mem::size_of;
use std::sync::mpsc;
use std::sync::Mutex;
use std::thread::{self, JoinHandle};

use windows::Win32::Foundation::{LPARAM, LRESULT, WPARAM};
use windows::Win32::System::LibraryLoader::GetModuleHandleW;
use windows::Win32::System::Threading::GetCurrentThreadId;
use windows::Win32::UI::Input::KeyboardAndMouse::{
    GetKeyState, SendInput, INPUT, INPUT_0, INPUT_KEYBOARD, KEYBDINPUT, KEYBD_EVENT_FLAGS, KEYEVENTF_KEYUP,
    KEYEVENTF_UNICODE, VIRTUAL_KEY, VK_BACK, VK_CAPITAL,
};
use windows::Win32::UI::WindowsAndMessaging::{
    CallNextHookEx, GetMessageW, PostThreadMessageW, SetWindowsHookExW, UnhookWindowsHookEx, HC_ACTION,
    KBDLLHOOKSTRUCT, LLKHF_INJECTED, MSG, WH_KEYBOARD_LL, WM_KEYDOWN, WM_QUIT, WM_SYSKEYDOWN,
};

use crate::platform::{HookEvent, HookKey};

/// Returns true to keep the key from reaching applications.
pub type KeyHandler = Box<dyn Fn(HookEvent) -> bool + Send + Sync>;

/// One hook at a time; the callback (a plain function) reaches it here.
static HANDLER: Mutex<Option<KeyHandler>> = Mutex::new(None);

/// Removes the hook and stops its thread when dropped.
pub struct KeyboardHook {
    thread_id: u32,
    thread: Option<JoinHandle<()>>,
}

impl Drop for KeyboardHook {
    fn drop(&mut self) {
        // SAFETY: posting WM_QUIT to the hook thread's own message queue.
        if let Err(error) = unsafe { PostThreadMessageW(self.thread_id, WM_QUIT, WPARAM(0), LPARAM(0)) } {
            log::warn!("Cannot stop the keyboard hook: {error}");
        }
        if let Some(thread) = self.thread.take() {
            let _ = thread.join();
        }
        *HANDLER.lock().unwrap_or_else(|poisoned| poisoned.into_inner()) = None;
    }
}

pub fn install_keyboard_hook(handler: KeyHandler) -> Result<KeyboardHook, String> {
    *HANDLER.lock().unwrap_or_else(|poisoned| poisoned.into_inner()) = Some(handler);
    let (ready, started) = mpsc::channel::<Result<u32, String>>();
    let thread = thread::Builder::new()
        .name("keyboard-hook".into())
        .spawn(move || {
            // SAFETY: the hook is installed, pumped and removed on this thread.
            unsafe {
                let module = match GetModuleHandleW(None) {
                    Ok(module) => module,
                    Err(error) => {
                        let _ = ready.send(Err(format!("Cannot find the app module: {error}")));
                        return;
                    }
                };
                let hook = match SetWindowsHookExW(WH_KEYBOARD_LL, Some(hook_proc), Some(module.into()), 0) {
                    Ok(hook) => hook,
                    Err(error) => {
                        let _ = ready.send(Err(format!("Cannot listen to the keyboard: {error}")));
                        return;
                    }
                };
                let _ = ready.send(Ok(GetCurrentThreadId()));
                // Low-level hooks are called through this thread's message loop.
                let mut message = MSG::default();
                while GetMessageW(&mut message, None, 0, 0).as_bool() {}
                if let Err(error) = UnhookWindowsHookEx(hook) {
                    log::warn!("Cannot remove the keyboard hook: {error}");
                }
            }
        })
        .map_err(|error| format!("Cannot start the keyboard hook: {error}"))?;

    match started.recv() {
        Ok(Ok(thread_id)) => Ok(KeyboardHook { thread_id, thread: Some(thread) }),
        Ok(Err(error)) => {
            let _ = thread.join();
            *HANDLER.lock().unwrap_or_else(|poisoned| poisoned.into_inner()) = None;
            Err(error)
        }
        Err(_) => Err("The keyboard hook thread stopped unexpectedly".to_string()),
    }
}

unsafe extern "system" fn hook_proc(code: i32, wparam: WPARAM, lparam: LPARAM) -> LRESULT {
    if code == HC_ACTION as i32 {
        // SAFETY: for WH_KEYBOARD_LL with HC_ACTION, lparam points to a KBDLLHOOKSTRUCT.
        let info = unsafe { &*(lparam.0 as *const KBDLLHOOKSTRUCT) };
        // Our own typed text comes back through the hook: let it pass untouched.
        let injected = info.flags.0 & LLKHF_INJECTED.0 != 0;
        if !injected {
            let message = wparam.0 as u32;
            let event = HookEvent {
                key: hook_key(info.vkCode),
                down: message == WM_KEYDOWN || message == WM_SYSKEYDOWN,
            };
            let swallow = HANDLER
                .lock()
                .ok()
                .and_then(|handler| handler.as_ref().map(|handle| handle(event)))
                .unwrap_or(false);
            if swallow {
                return LRESULT(1);
            }
        }
    }
    // SAFETY: forwarding the untouched event down the hook chain.
    unsafe { CallNextHookEx(None, code, wparam, lparam) }
}

/// Windows virtual-key codes of the keys Quick Accent reacts to.
fn hook_key(virtual_key: u32) -> HookKey {
    match virtual_key {
        0x41..=0x5A => HookKey::Letter(char::from(b'a' + (virtual_key - 0x41) as u8)),
        0x20 => HookKey::Space,
        0x25 => HookKey::Left,
        0x27 => HookKey::Right,
        0x1B => HookKey::Escape,
        0x10 | 0xA0 | 0xA1 => HookKey::Shift,
        0x11 | 0xA2 | 0xA3 => HookKey::Control,
        0x12 | 0xA4 | 0xA5 => HookKey::Alt,
        0x5B | 0x5C => HookKey::Windows,
        _ => HookKey::Other,
    }
}

pub fn caps_lock_on() -> bool {
    // SAFETY: plain key state query; the low bit is the toggle state.
    unsafe { GetKeyState(i32::from(VK_CAPITAL.0)) & 1 == 1 }
}

fn key_input(key: VIRTUAL_KEY, scan: u16, flags: KEYBD_EVENT_FLAGS) -> INPUT {
    INPUT {
        r#type: INPUT_KEYBOARD,
        Anonymous: INPUT_0 { ki: KEYBDINPUT { wVk: key, wScan: scan, dwFlags: flags, time: 0, dwExtraInfo: 0 } },
    }
}

/// Erases `erase` characters with Backspace, then types `character`.
pub fn type_replacing(erase: u32, character: char) -> Result<(), String> {
    let mut inputs = Vec::new();
    for _ in 0..erase {
        inputs.push(key_input(VK_BACK, 0, KEYBD_EVENT_FLAGS(0)));
        inputs.push(key_input(VK_BACK, 0, KEYEVENTF_KEYUP));
    }
    let mut units = [0u16; 2];
    for &unit in character.encode_utf16(&mut units).iter() {
        inputs.push(key_input(VIRTUAL_KEY(0), unit, KEYEVENTF_UNICODE));
        inputs.push(key_input(VIRTUAL_KEY(0), unit, KEYEVENTF_UNICODE | KEYEVENTF_KEYUP));
    }
    // SAFETY: well-formed keyboard events, sized as Windows expects.
    let sent = unsafe { SendInput(&inputs, size_of::<INPUT>() as i32) };
    if sent as usize == inputs.len() {
        Ok(())
    } else {
        Err("Windows blocked the accented character".to_string())
    }
}

#[cfg(test)]
mod tests {
    use super::hook_key;
    use crate::platform::HookKey;

    #[test]
    fn virtual_keys_map_to_hook_keys() {
        assert_eq!(hook_key(0x41), HookKey::Letter('a'));
        assert_eq!(hook_key(0x5A), HookKey::Letter('z'));
        assert_eq!(hook_key(0x20), HookKey::Space);
        assert_eq!(hook_key(0xA1), HookKey::Shift);
        assert_eq!(hook_key(0x31), HookKey::Other);
    }
}
