use windows::core::PWSTR;
use windows::Win32::System::Com::{
    CoInitializeEx, CoTaskMemFree, CoUninitialize, COINIT_APARTMENTTHREADED, COINIT_DISABLE_OLE1DDE,
};

/// RAII guard for a single-threaded COM apartment.
///
/// Shell objects (AppsFolder, image factories) expect an STA, so every thread
/// that uses them enters one through this guard.
pub struct ComApartment {
    should_uninitialize: bool,
}

pub fn enter_com() -> ComApartment {
    // SAFETY: plain COM initialisation for the current thread.
    let result = unsafe { CoInitializeEx(None, COINIT_APARTMENTTHREADED | COINIT_DISABLE_OLE1DDE) };
    // S_OK and S_FALSE both need a matching CoUninitialize; RPC_E_CHANGED_MODE
    // means another apartment model is already active and must be left alone.
    ComApartment { should_uninitialize: result.is_ok() }
}

impl Drop for ComApartment {
    fn drop(&mut self) {
        if self.should_uninitialize {
            // SAFETY: balanced with the successful CoInitializeEx above.
            unsafe { CoUninitialize() };
        }
    }
}

/// Converts a COM-allocated wide string into a Rust `String` and frees it.
pub fn take_com_string(value: PWSTR) -> String {
    if value.is_null() {
        return String::new();
    }
    // SAFETY: the pointer comes from a successful shell call and is freed once.
    unsafe {
        let text = value.to_string().unwrap_or_default();
        CoTaskMemFree(Some(value.0 as *const _));
        text
    }
}
