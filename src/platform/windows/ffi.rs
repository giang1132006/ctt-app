//! Minimal zero-dependency Win32 FFI bindings for `SendInput`, low-level input hooks,
//! and high-resolution multimedia timers on Windows.
//!
//! All unsafe FFI declarations and `#[repr(C)]` ABI structures are isolated here.

#![cfg(target_os = "windows")]
#![allow(non_camel_case_types)]

use std::ffi::c_void;

pub type BOOL = i32;
pub type DWORD = u32;
pub type INT = i32;
pub type LONG = i32;
pub type LPARAM = isize;
pub type LRESULT = isize;
pub type UINT = u32;
pub type WORD = u16;
pub type WPARAM = usize;
pub type ULONG_PTR = usize;
pub type HHOOK = *mut c_void;
pub type HINSTANCE = *mut c_void;
pub type HMODULE = *mut c_void;
pub type HWND = *mut c_void;

pub type HOOKPROC =
    unsafe extern "system" fn(code: INT, w_param: WPARAM, l_param: LPARAM) -> LRESULT;

pub const WH_KEYBOARD_LL: INT = 13;
pub const WH_MOUSE_LL: INT = 14;
pub const HC_ACTION: INT = 0;

pub const PM_NOREMOVE: UINT = 0x0000;
pub const WM_QUIT: UINT = 0x0012;

pub const WM_KEYDOWN: WPARAM = 0x0100;
pub const WM_KEYUP: WPARAM = 0x0101;
pub const WM_SYSKEYDOWN: WPARAM = 0x0104;
pub const WM_SYSKEYUP: WPARAM = 0x0105;

pub const WM_MOUSEMOVE: WPARAM = 0x0200;
pub const WM_LBUTTONDOWN: WPARAM = 0x0201;
pub const WM_LBUTTONUP: WPARAM = 0x0202;
pub const WM_RBUTTONDOWN: WPARAM = 0x0204;
pub const WM_RBUTTONUP: WPARAM = 0x0205;
pub const WM_MBUTTONDOWN: WPARAM = 0x0207;
pub const WM_MBUTTONUP: WPARAM = 0x0208;
pub const WM_XBUTTONDOWN: WPARAM = 0x020B;
pub const WM_XBUTTONUP: WPARAM = 0x020C;

pub const LLKHF_EXTENDED: DWORD = 0x0000_0001;

pub const INPUT_MOUSE: DWORD = 0;
pub const INPUT_KEYBOARD: DWORD = 1;

pub const KEYEVENTF_EXTENDEDKEY: DWORD = 0x0001;
pub const KEYEVENTF_KEYUP: DWORD = 0x0002;
pub const KEYEVENTF_SCANCODE: DWORD = 0x0008;

pub const MOUSEEVENTF_LEFTDOWN: DWORD = 0x0002;
pub const MOUSEEVENTF_LEFTUP: DWORD = 0x0004;
pub const MOUSEEVENTF_RIGHTDOWN: DWORD = 0x0008;
pub const MOUSEEVENTF_RIGHTUP: DWORD = 0x0010;
pub const MOUSEEVENTF_MIDDLEDOWN: DWORD = 0x0020;
pub const MOUSEEVENTF_MIDDLEUP: DWORD = 0x0040;
pub const MOUSEEVENTF_XDOWN: DWORD = 0x0080;
pub const MOUSEEVENTF_XUP: DWORD = 0x0100;
pub const MOUSEEVENTF_WHEEL: DWORD = 0x0800;
pub const MOUSEEVENTF_HWHEEL: DWORD = 0x1000;

pub const XBUTTON1: DWORD = 0x0001;
pub const XBUTTON2: DWORD = 0x0002;

pub const TIMERR_NOERROR: UINT = 0;

#[repr(C)]
#[derive(Debug, Clone, Copy, Default)]
pub struct POINT {
    pub x: LONG,
    pub y: LONG,
}

#[repr(C)]
#[derive(Debug, Clone, Copy)]
pub struct MSG {
    pub hwnd: HWND,
    pub message: UINT,
    pub w_param: WPARAM,
    pub l_param: LPARAM,
    pub time: DWORD,
    pub pt: POINT,
}

impl Default for MSG {
    fn default() -> Self {
        Self {
            hwnd: std::ptr::null_mut(),
            message: 0,
            w_param: 0,
            l_param: 0,
            time: 0,
            pt: POINT::default(),
        }
    }
}

#[repr(C)]
#[derive(Debug, Clone, Copy)]
pub struct KBDLLHOOKSTRUCT {
    pub vk_code: DWORD,
    pub scan_code: DWORD,
    pub flags: DWORD,
    pub time: DWORD,
    pub dw_extra_info: ULONG_PTR,
}

#[repr(C)]
#[derive(Debug, Clone, Copy)]
pub struct MSLLHOOKSTRUCT {
    pub pt: POINT,
    pub mouse_data: DWORD,
    pub flags: DWORD,
    pub time: DWORD,
    pub dw_extra_info: ULONG_PTR,
}

#[repr(C)]
#[derive(Debug, Clone, Copy)]
pub struct MOUSEINPUT {
    pub dx: LONG,
    pub dy: LONG,
    pub mouse_data: DWORD,
    pub dw_flags: DWORD,
    pub time: DWORD,
    pub dw_extra_info: ULONG_PTR,
}

#[repr(C)]
#[derive(Debug, Clone, Copy)]
pub struct KEYBDINPUT {
    pub w_vk: WORD,
    pub w_scan: WORD,
    pub dw_flags: DWORD,
    pub time: DWORD,
    pub dw_extra_info: ULONG_PTR,
}

#[repr(C)]
#[derive(Debug, Clone, Copy)]
pub struct HARDWAREINPUT {
    pub u_msg: DWORD,
    pub w_param_l: WORD,
    pub w_param_h: WORD,
}

#[repr(C)]
#[derive(Clone, Copy)]
pub union INPUT_UNION {
    pub mi: MOUSEINPUT,
    pub ki: KEYBDINPUT,
    pub hi: HARDWAREINPUT,
}

#[repr(C)]
#[derive(Clone, Copy)]
pub struct INPUT {
    pub input_type: DWORD,
    pub union: INPUT_UNION,
}

#[link(name = "user32")]
extern "system" {
    pub fn SendInput(c_inputs: UINT, p_inputs: *const INPUT, cb_size: INT) -> UINT;
    pub fn SetWindowsHookExW(
        id_hook: INT,
        lpfn: HOOKPROC,
        hmod: HINSTANCE,
        dw_thread_id: DWORD,
    ) -> HHOOK;
    pub fn UnhookWindowsHookEx(hhk: HHOOK) -> BOOL;
    pub fn CallNextHookEx(hhk: HHOOK, n_code: INT, w_param: WPARAM, l_param: LPARAM) -> LRESULT;
    pub fn GetMessageW(
        lp_msg: *mut MSG,
        hwnd: HWND,
        w_msg_filter_min: UINT,
        w_msg_filter_max: UINT,
    ) -> BOOL;
    pub fn PeekMessageW(
        lp_msg: *mut MSG,
        hwnd: HWND,
        w_msg_filter_min: UINT,
        w_msg_filter_max: UINT,
        w_remove_msg: UINT,
    ) -> BOOL;
    pub fn PostThreadMessageW(
        id_thread: DWORD,
        msg: UINT,
        w_param: WPARAM,
        l_param: LPARAM,
    ) -> BOOL;
}

#[link(name = "kernel32")]
extern "system" {
    pub fn GetCurrentThreadId() -> DWORD;
    pub fn GetModuleHandleW(lp_module_name: *const u16) -> HMODULE;
    pub fn GetLastError() -> DWORD;
}

#[link(name = "winmm")]
extern "system" {
    pub fn timeBeginPeriod(u_period: UINT) -> UINT;
    pub fn timeEndPeriod(u_period: UINT) -> UINT;
}

/// RAII guard requesting 1 ms Windows multimedia timer resolution during active macro playback.
pub struct HighResTimerGuard {
    active: bool,
    period_ms: UINT,
}

impl HighResTimerGuard {
    /// Requests `period_ms` timer granularity (typically `1` ms) and restores it on [`Drop`].
    pub fn acquire(period_ms: UINT) -> Self {
        // SAFETY: `timeBeginPeriod` is thread-safe and balanced by `timeEndPeriod` on Drop.
        let res = unsafe { timeBeginPeriod(period_ms) };
        Self {
            active: res == TIMERR_NOERROR,
            period_ms,
        }
    }
}

impl Drop for HighResTimerGuard {
    fn drop(&mut self) {
        if self.active {
            // SAFETY: Paired with the successful `timeBeginPeriod(self.period_ms)` call in `acquire`.
            unsafe {
                timeEndPeriod(self.period_ms);
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::mem::size_of;

    #[test]
    fn win32_ffi_struct_sizes_match_platform_abi() {
        assert_eq!(size_of::<POINT>(), 8);

        #[cfg(target_pointer_width = "64")]
        {
            assert_eq!(size_of::<MSG>(), 48);
            assert_eq!(size_of::<KBDLLHOOKSTRUCT>(), 24);
            assert_eq!(size_of::<MSLLHOOKSTRUCT>(), 32);
            assert_eq!(size_of::<MOUSEINPUT>(), 32);
            assert_eq!(size_of::<KEYBDINPUT>(), 24);
            assert_eq!(size_of::<INPUT>(), 40);
        }

        #[cfg(target_pointer_width = "32")]
        {
            assert_eq!(size_of::<MSG>(), 28);
            assert_eq!(size_of::<KBDLLHOOKSTRUCT>(), 20);
            assert_eq!(size_of::<MSLLHOOKSTRUCT>(), 24);
            assert_eq!(size_of::<MOUSEINPUT>(), 24);
            assert_eq!(size_of::<KEYBDINPUT>(), 16);
            assert_eq!(size_of::<INPUT>(), 28);
        }
    }
}
