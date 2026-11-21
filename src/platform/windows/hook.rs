//! Live Windows low-level keyboard (`WH_KEYBOARD_LL`) and mouse (`WH_MOUSE_LL`) hook manager
//! with RAII cleanup, panic-safe FFI boundaries, and synthetic-event loop isolation.

use std::sync::Arc;
#[cfg(target_os = "windows")]
use std::thread::JoinHandle;

use crate::domain::keys::{KeyState, MouseButton};
use crate::platform::windows::keymap::win32_to_keyboard_key;
use crate::platform::{
    HookDecision, InputInterceptor, PlatformError, RawInputEvent, RawKeyEvent, RawMouseButtonEvent,
    CTT_SYNTHETIC_TAG,
};

/// Callback invoked on the hook thread for every physical keyboard or mouse button event.
pub type HookCallback = Arc<dyn Fn(RawInputEvent) -> HookDecision + Send + Sync + 'static>;

/// Returns `true` when `dwExtraInfo` identifies an event synthesized by `ctt-app`.
pub const fn is_synthetic_extra_info(extra_info: usize) -> bool {
    extra_info == CTT_SYNTHETIC_TAG
}

/// Decodes raw Win32 `WH_KEYBOARD_LL` parameters into a normalized [`RawKeyEvent`].
pub fn decode_win32_keyboard_event(
    w_param: usize,
    vk_code: u32,
    scan_code: u32,
    flags: u32,
    extra_info: usize,
) -> Option<RawKeyEvent> {
    const WM_KEYDOWN: usize = 0x0100;
    const WM_KEYUP: usize = 0x0101;
    const WM_SYSKEYDOWN: usize = 0x0104;
    const WM_SYSKEYUP: usize = 0x0105;
    const LLKHF_EXTENDED: u32 = 0x0000_0001;

    let state = match w_param {
        WM_KEYDOWN | WM_SYSKEYDOWN => KeyState::Down,
        WM_KEYUP | WM_SYSKEYUP => KeyState::Up,
        _ => return None,
    };

    let extended = (flags & LLKHF_EXTENDED) != 0;
    let key = win32_to_keyboard_key(vk_code as u16, scan_code as u16, extended);
    let is_injected = is_synthetic_extra_info(extra_info);

    Some(RawKeyEvent {
        key,
        state,
        is_injected,
    })
}

/// Decodes raw Win32 `WH_MOUSE_LL` button parameters into a normalized [`RawMouseButtonEvent`].
///
/// Mouse movement (`WM_MOUSEMOVE`) and scroll wheel events return `None` so they bypass locking.
pub fn decode_win32_mouse_button_event(
    w_param: usize,
    mouse_data: u32,
    extra_info: usize,
) -> Option<RawMouseButtonEvent> {
    const WM_LBUTTONDOWN: usize = 0x0201;
    const WM_LBUTTONUP: usize = 0x0202;
    const WM_RBUTTONDOWN: usize = 0x0204;
    const WM_RBUTTONUP: usize = 0x0205;
    const WM_MBUTTONDOWN: usize = 0x0207;
    const WM_MBUTTONUP: usize = 0x0208;
    const WM_XBUTTONDOWN: usize = 0x020B;
    const WM_XBUTTONUP: usize = 0x020C;

    let (button, state) = match w_param {
        WM_LBUTTONDOWN => (MouseButton::Left, KeyState::Down),
        WM_LBUTTONUP => (MouseButton::Left, KeyState::Up),
        WM_RBUTTONDOWN => (MouseButton::Right, KeyState::Down),
        WM_RBUTTONUP => (MouseButton::Right, KeyState::Up),
        WM_MBUTTONDOWN => (MouseButton::Middle, KeyState::Down),
        WM_MBUTTONUP => (MouseButton::Middle, KeyState::Up),
        WM_XBUTTONDOWN => {
            let btn = match (mouse_data >> 16) as u16 {
                1 => MouseButton::XButton1,
                2 => MouseButton::XButton2,
                _ => return None,
            };
            (btn, KeyState::Down)
        }
        WM_XBUTTONUP => {
            let btn = match (mouse_data >> 16) as u16 {
                1 => MouseButton::XButton1,
                2 => MouseButton::XButton2,
                _ => return None,
            };
            (btn, KeyState::Up)
        }
        _ => return None,
    };

    Some(RawMouseButtonEvent {
        button,
        state,
        is_injected: is_synthetic_extra_info(extra_info),
    })
}

#[cfg(target_os = "windows")]
mod win32_impl {
    use std::cell::RefCell;
    use std::panic::{catch_unwind, AssertUnwindSafe};
    use std::ptr::{null, null_mut};
    use std::sync::mpsc;
    use std::thread;
    use std::time::Duration;

    use crate::platform::windows::ffi::{
        CallNextHookEx, GetCurrentThreadId, GetLastError, GetMessageW, GetModuleHandleW,
        PeekMessageW, PostThreadMessageW, SetWindowsHookExW, UnhookWindowsHookEx, HC_ACTION,
        KBDLLHOOKSTRUCT, LPARAM, LRESULT, MSG, MSLLHOOKSTRUCT, PM_NOREMOVE, WH_KEYBOARD_LL,
        WH_MOUSE_LL, WM_MOUSEMOVE, WM_QUIT, WPARAM,
    };
    use crate::platform::windows::hook::{
        decode_win32_keyboard_event, decode_win32_mouse_button_event, is_synthetic_extra_info,
        HookCallback,
    };
    use crate::platform::{HookDecision, PlatformError, RawInputEvent};

    thread_local! {
        static HOOK_CALLBACK: RefCell<Option<HookCallback>> = const { RefCell::new(None) };
    }

    unsafe extern "system" fn low_level_keyboard_proc(
        n_code: i32,
        w_param: WPARAM,
        l_param: LPARAM,
    ) -> LRESULT {
        if n_code < HC_ACTION || l_param == 0 {
            return CallNextHookEx(null_mut(), n_code, w_param, l_param);
        }

        let kb = *(l_param as *const KBDLLHOOKSTRUCT);

        // Fast-path loop isolation: ignore self-synthesized events with zero locking.
        if is_synthetic_extra_info(kb.dw_extra_info) {
            return CallNextHookEx(null_mut(), n_code, w_param, l_param);
        }

        let decision = catch_unwind(AssertUnwindSafe(|| {
            let Some(raw) = decode_win32_keyboard_event(
                w_param,
                kb.vk_code,
                kb.scan_code,
                kb.flags,
                kb.dw_extra_info,
            ) else {
                return HookDecision::PassThrough;
            };

            HOOK_CALLBACK.with(|slot| {
                let Ok(guard) = slot.try_borrow() else {
                    return HookDecision::PassThrough;
                };
                match guard.as_ref() {
                    Some(cb) => cb(RawInputEvent::Key(raw)),
                    None => HookDecision::PassThrough,
                }
            })
        }))
        .unwrap_or(HookDecision::PassThrough);

        match decision {
            HookDecision::Suppress => 1,
            HookDecision::PassThrough => CallNextHookEx(null_mut(), n_code, w_param, l_param),
        }
    }

    unsafe extern "system" fn low_level_mouse_proc(
        n_code: i32,
        w_param: WPARAM,
        l_param: LPARAM,
    ) -> LRESULT {
        // Immediate zero-overhead bypass for high-frequency mouse movement & negative hook codes.
        if n_code < HC_ACTION || w_param == WM_MOUSEMOVE || l_param == 0 {
            return CallNextHookEx(null_mut(), n_code, w_param, l_param);
        }

        let ms = *(l_param as *const MSLLHOOKSTRUCT);

        // Fast-path loop isolation for self-synthesized mouse events.
        if is_synthetic_extra_info(ms.dw_extra_info) {
            return CallNextHookEx(null_mut(), n_code, w_param, l_param);
        }

        let decision = catch_unwind(AssertUnwindSafe(|| {
            let Some(raw) =
                decode_win32_mouse_button_event(w_param, ms.mouse_data, ms.dw_extra_info)
            else {
                return HookDecision::PassThrough;
            };

            HOOK_CALLBACK.with(|slot| {
                let Ok(guard) = slot.try_borrow() else {
                    return HookDecision::PassThrough;
                };
                match guard.as_ref() {
                    Some(cb) => cb(RawInputEvent::MouseButton(raw)),
                    None => HookDecision::PassThrough,
                }
            })
        }))
        .unwrap_or(HookDecision::PassThrough);

        match decision {
            HookDecision::Suppress => 1,
            HookDecision::PassThrough => CallNextHookEx(null_mut(), n_code, w_param, l_param),
        }
    }

    pub fn spawn_hook_thread(
        callback: HookCallback,
    ) -> Result<(u32, thread::JoinHandle<()>), PlatformError> {
        let (tx, rx) = mpsc::sync_channel::<Result<u32, PlatformError>>(1);

        let handle = thread::Builder::new()
            .name("ctt-win32-hooks".to_string())
            .spawn(move || {
                HOOK_CALLBACK.with(|slot| {
                    *slot.borrow_mut() = Some(callback);
                });

                // SAFETY: Calling standard Win32 hook & message-loop APIs on a dedicated OS thread.
                unsafe {
                    let thread_id = GetCurrentThreadId();
                    let hmod = GetModuleHandleW(null());

                    let kb_hook =
                        SetWindowsHookExW(WH_KEYBOARD_LL, low_level_keyboard_proc, hmod, 0);
                    if kb_hook.is_null() {
                        let err = GetLastError();
                        HOOK_CALLBACK.with(|slot| {
                            *slot.borrow_mut() = None;
                        });
                        let _ = tx.send(Err(PlatformError::HookRegistrationFailed(format!(
                            "SetWindowsHookExW(WH_KEYBOARD_LL) failed (Win32 error {err})"
                        ))));
                        return;
                    }

                    let mouse_hook = SetWindowsHookExW(WH_MOUSE_LL, low_level_mouse_proc, hmod, 0);
                    if mouse_hook.is_null() {
                        let err = GetLastError();
                        let _ = UnhookWindowsHookEx(kb_hook);
                        HOOK_CALLBACK.with(|slot| {
                            *slot.borrow_mut() = None;
                        });
                        let _ = tx.send(Err(PlatformError::HookRegistrationFailed(format!(
                            "SetWindowsHookExW(WH_MOUSE_LL) failed (Win32 error {err})"
                        ))));
                        return;
                    }

                    // Ensure this thread's Win32 message queue exists before returning thread_id.
                    let mut msg = MSG::default();
                    let _ = PeekMessageW(&mut msg, null_mut(), 0, 0, PM_NOREMOVE);

                    if tx.send(Ok(thread_id)).is_err() {
                        let _ = UnhookWindowsHookEx(mouse_hook);
                        let _ = UnhookWindowsHookEx(kb_hook);
                        HOOK_CALLBACK.with(|slot| {
                            *slot.borrow_mut() = None;
                        });
                        return;
                    }

                    // Pump Windows messages until WM_QUIT is posted on shutdown/Drop.
                    while GetMessageW(&mut msg, null_mut(), 0, 0) > 0 {}

                    let _ = UnhookWindowsHookEx(mouse_hook);
                    let _ = UnhookWindowsHookEx(kb_hook);
                }

                HOOK_CALLBACK.with(|slot| {
                    *slot.borrow_mut() = None;
                });
            })
            .map_err(|err| {
                PlatformError::HookRegistrationFailed(format!("Failed to spawn hook thread: {err}"))
            })?;

        match rx.recv_timeout(Duration::from_secs(3)) {
            Ok(Ok(thread_id)) => Ok((thread_id, handle)),
            Ok(Err(err)) => {
                let _ = handle.join();
                Err(err)
            }
            Err(_) => Err(PlatformError::HookRegistrationFailed(
                "Timed out waiting for Win32 input hooks to initialize".to_string(),
            )),
        }
    }

    pub fn stop_hook_thread(thread_id: u32, handle: thread::JoinHandle<()>) {
        // Retry briefly in case the message queue is still transitioning into GetMessageW.
        for _ in 0..10 {
            // SAFETY: Posting WM_QUIT to our own dedicated Win32 hook thread ID.
            let posted = unsafe { PostThreadMessageW(thread_id, WM_QUIT, 0, 0) };
            if posted != 0 {
                break;
            }
            thread::sleep(Duration::from_millis(5));
        }
        let _ = handle.join();
    }
}

/// RAII handle for live Windows low-level keyboard (`WH_KEYBOARD_LL`) and mouse (`WH_MOUSE_LL`) hooks.
///
/// Guarantees both hooks are cleanly unhooked and the pump thread joins on [`Self::stop`] or [`Drop`].
#[derive(Debug, Default)]
pub struct WindowsKeyboardHook {
    active: bool,
    #[cfg(target_os = "windows")]
    thread_id: Option<u32>,
    #[cfg(target_os = "windows")]
    thread_handle: Option<JoinHandle<()>>,
}

impl WindowsKeyboardHook {
    /// Creates an inactive hook controller.
    pub const fn new() -> Self {
        Self {
            active: false,
            #[cfg(target_os = "windows")]
            thread_id: None,
            #[cfg(target_os = "windows")]
            thread_handle: None,
        }
    }

    /// Installs global `WH_KEYBOARD_LL` and `WH_MOUSE_LL` hooks forwarding events to `callback`.
    pub fn start_with_callback(&mut self, callback: HookCallback) -> Result<(), PlatformError> {
        if self.active {
            return Ok(());
        }

        #[cfg(target_os = "windows")]
        {
            let (thread_id, handle) = win32_impl::spawn_hook_thread(callback)?;
            self.thread_id = Some(thread_id);
            self.thread_handle = Some(handle);
            self.active = true;
            Ok(())
        }

        #[cfg(not(target_os = "windows"))]
        {
            let _ = callback;
            Err(PlatformError::UnsupportedPlatform)
        }
    }

    /// Starts with a pass-through no-op callback (retained for API compatibility).
    pub fn start(&mut self) -> Result<(), PlatformError> {
        self.start_with_callback(Arc::new(|_| HookDecision::PassThrough))
    }
}

impl InputInterceptor for WindowsKeyboardHook {
    fn is_active(&self) -> bool {
        self.active
    }

    fn stop(&mut self) -> Result<(), PlatformError> {
        if !self.active {
            return Ok(());
        }
        self.active = false;

        #[cfg(target_os = "windows")]
        if let (Some(thread_id), Some(handle)) = (self.thread_id.take(), self.thread_handle.take())
        {
            win32_impl::stop_hook_thread(thread_id, handle);
        }

        Ok(())
    }
}

impl Drop for WindowsKeyboardHook {
    fn drop(&mut self) {
        let _ = self.stop();
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::keys::KeyboardKey;

    #[test]
    fn detects_ctt_synthetic_extra_info_tag() {
        assert!(is_synthetic_extra_info(CTT_SYNTHETIC_TAG));
        assert!(!is_synthetic_extra_info(0));
        assert!(!is_synthetic_extra_info(0x1234));
    }

    #[test]
    fn decodes_win32_keyboard_messages_including_syskey_extended_and_synthetic_tag() {
        // Physical NumpadEnter down (VK_RETURN=0x0D, scan=0x1C, extended=1)
        let ev = decode_win32_keyboard_event(0x0100, 0x0D, 0x1C, 0x01, 0)
            .expect("WM_KEYDOWN should decode");
        assert_eq!(
            ev,
            RawKeyEvent::physical(KeyboardKey::NumpadEnter, KeyState::Down)
        );

        // Alt-chord WM_SYSKEYUP with CTT synthetic tag
        let synth_ev = decode_win32_keyboard_event(0x0105, 0x7B, 0x58, 0, CTT_SYNTHETIC_TAG)
            .expect("WM_SYSKEYUP should decode");
        assert_eq!(
            synth_ev,
            RawKeyEvent::injected(KeyboardKey::F12, KeyState::Up)
        );

        // Unknown message ignored
        assert!(decode_win32_keyboard_event(0x0200, 0, 0, 0, 0).is_none());
    }

    #[test]
    fn decodes_win32_mouse_button_messages_and_ignores_mousemove() {
        assert!(decode_win32_mouse_button_event(0x0200, 0, 0).is_none());

        assert_eq!(
            decode_win32_mouse_button_event(0x0201, 0, 0),
            Some(RawMouseButtonEvent::physical(
                MouseButton::Left,
                KeyState::Down
            ))
        );
        assert_eq!(
            decode_win32_mouse_button_event(0x0208, 0, CTT_SYNTHETIC_TAG),
            Some(RawMouseButtonEvent::injected(
                MouseButton::Middle,
                KeyState::Up
            ))
        );
        assert_eq!(
            decode_win32_mouse_button_event(0x020B, 1 << 16, 0),
            Some(RawMouseButtonEvent::physical(
                MouseButton::XButton1,
                KeyState::Down
            ))
        );
        assert_eq!(
            decode_win32_mouse_button_event(0x020C, 2 << 16, 0),
            Some(RawMouseButtonEvent::physical(
                MouseButton::XButton2,
                KeyState::Up
            ))
        );
    }
}
