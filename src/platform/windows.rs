// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2025 Panayotis Katsaloulis

//! Windows platform backend: window focus (query + activate), sleep/resume
//! notification, exit cleanup and shell selection. Keyboard injection and the
//! config-reload listener are shared with macOS (`platform::keymap` /
//! `platform::reload`).

use crate::event::{send, DeviceEvent};
use crate::{error_log, verbose_log};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc::Sender;
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::Duration;

use windows::core::{w, PWSTR};
use windows::Win32::Foundation::{CloseHandle, BOOL, HINSTANCE, HWND, LPARAM, LRESULT, TRUE, WPARAM};
use windows::Win32::System::LibraryLoader::GetModuleHandleW;
use windows::Win32::System::Threading::{
    OpenProcess, QueryFullProcessImageNameW, PROCESS_NAME_WIN32, PROCESS_QUERY_LIMITED_INFORMATION,
};
use windows::Win32::UI::WindowsAndMessaging::{
    CreateWindowExW, DefWindowProcW, DispatchMessageW, EnumWindows, GetForegroundWindow,
    GetMessageW, GetWindowTextLengthW, GetWindowTextW, GetWindowThreadProcessId, IsWindowVisible,
    RegisterClassW, SetForegroundWindow, TranslateMessage, HWND_MESSAGE, MSG, WINDOW_EX_STYLE,
    WINDOW_STYLE, WM_POWERBROADCAST, WNDCLASSW,
};

// Power-broadcast event codes (from WinUser.h; not re-exported by the
// `windows` crate at a stable path across feature sets).
const PBT_APMSUSPEND: u32 = 0x0004;
const PBT_APMRESUMESUSPEND: u32 = 0x0007;
const PBT_APMRESUMEAUTOMATIC: u32 = 0x0012;

/// Shell used to run `Exec` actions.
pub fn exec_shell() -> (&'static str, &'static str) {
    ("cmd", "/C")
}

/// No Windows-specific teardown is required on exit.
pub fn on_exit_cleanup() {}

// ---------------------------------------------------------------------------
// Window information helpers
// ---------------------------------------------------------------------------

/// Returns the title text of a window.
fn window_title(hwnd: HWND) -> String {
    unsafe {
        let len = GetWindowTextLengthW(hwnd);
        if len <= 0 {
            return String::new();
        }
        let mut buf = vec![0u16; (len + 1) as usize];
        let n = GetWindowTextW(hwnd, &mut buf);
        String::from_utf16_lossy(&buf[..n as usize])
    }
}

/// Returns the executable base name (without `.exe`) of the process owning a
/// window. This is the Windows analogue of the X11 `WM_CLASS`.
fn window_process_name(hwnd: HWND) -> Option<String> {
    unsafe {
        let mut pid: u32 = 0;
        GetWindowThreadProcessId(hwnd, Some(&mut pid));
        if pid == 0 {
            return None;
        }
        let handle = OpenProcess(PROCESS_QUERY_LIMITED_INFORMATION, false, pid).ok()?;
        let mut buf = vec![0u16; 512];
        let mut size = buf.len() as u32;
        let result = QueryFullProcessImageNameW(
            handle,
            PROCESS_NAME_WIN32,
            PWSTR(buf.as_mut_ptr()),
            &mut size,
        );
        let _ = CloseHandle(handle);
        result.ok()?;

        let path = String::from_utf16_lossy(&buf[..size as usize]);
        let name = path.rsplit(['\\', '/']).next().unwrap_or(&path);
        let name = name
            .strip_suffix(".exe")
            .or_else(|| name.strip_suffix(".EXE"))
            .unwrap_or(name);
        Some(name.to_string())
    }
}

/// Returns `(process_name, title)` for the current foreground window.
fn current_foreground() -> Option<(String, String)> {
    unsafe {
        let hwnd = GetForegroundWindow();
        if hwnd.is_invalid() {
            return None;
        }
        let title = window_title(hwnd);
        let class = window_process_name(hwnd).unwrap_or_default();
        if class.is_empty() && title.is_empty() {
            return None;
        }
        Some((class, title))
    }
}

// ---------------------------------------------------------------------------
// Focus listener (polling)
// ---------------------------------------------------------------------------

/// Emits [`DeviceEvent::FocusChanges`] whenever the foreground window's process
/// or title changes. Implemented by polling `GetForegroundWindow` — robust and
/// dependency-free, at negligible cost for a 200 ms interval.
pub fn spawn_focus_listener(tx: &Sender<DeviceEvent>, active: &Arc<AtomicBool>) {
    let tx = tx.clone();
    let active = active.clone();
    thread::spawn(move || {
        verbose_log!("Starting Windows focus listener (polling)");
        let mut last: Option<(String, String)> = None;
        while active.load(Ordering::Relaxed) {
            if let Some(cur) = current_foreground() {
                if last.as_ref() != Some(&cur) {
                    last = Some(cur.clone());
                    let (class, title) = cur;
                    send(&tx, DeviceEvent::FocusChanges { class, title });
                }
            }
            thread::sleep(Duration::from_millis(200));
        }
        verbose_log!("Windows focus listener exiting");
    });
}

// ---------------------------------------------------------------------------
// set_focus: find a matching top-level window and bring it to the foreground
// ---------------------------------------------------------------------------

unsafe extern "system" fn enum_windows_cb(hwnd: HWND, lparam: LPARAM) -> BOOL {
    let windows = &mut *(lparam.0 as *mut Vec<HWND>);
    if IsWindowVisible(hwnd).as_bool() {
        windows.push(hwnd);
    }
    TRUE
}

/// Activates a window matching `class` (process name) and/or `title`.
///
/// Matching mirrors the Linux backend: substring, case-insensitive; when the
/// same string is passed for both (the `Focus` action does this) either a
/// class or a title match suffices, otherwise both must match.
pub fn set_focus(class: &String, title: &String) -> Result<(), String> {
    if class.is_empty() && title.is_empty() {
        return Err("At least one of class or title must be specified".to_string());
    }

    let mut windows: Vec<HWND> = Vec::new();
    unsafe {
        EnumWindows(
            Some(enum_windows_cb),
            LPARAM(&mut windows as *mut _ as isize),
        )
        .map_err(|e| format!("EnumWindows failed: {e}"))?;
    }

    let use_or = !class.is_empty() && !title.is_empty() && class == title;
    let class_lower = class.to_lowercase();
    let title_lower = title.to_lowercase();

    for hwnd in windows {
        let wtitle = window_title(hwnd);
        let wclass = window_process_name(hwnd).unwrap_or_default();

        let class_match = if !class.is_empty() {
            wclass.to_lowercase().contains(&class_lower)
        } else {
            !use_or
        };
        let title_match = if !title.is_empty() {
            wtitle.to_lowercase().contains(&title_lower)
        } else {
            !use_or
        };
        let matches = if use_or {
            class_match || title_match
        } else {
            class_match && title_match
        };

        if matches {
            unsafe {
                let _ = SetForegroundWindow(hwnd);
            }
            return Ok(());
        }
    }

    Err(format!(
        "No matching window found using class '{}' and title '{}'",
        class, title
    ))
}

// ---------------------------------------------------------------------------
// Sleep/resume listener (WM_POWERBROADCAST via a message-only window)
// ---------------------------------------------------------------------------

static SLEEP_TX: Mutex<Option<Sender<DeviceEvent>>> = Mutex::new(None);

unsafe extern "system" fn power_wndproc(
    hwnd: HWND,
    msg: u32,
    wparam: WPARAM,
    lparam: LPARAM,
) -> LRESULT {
    if msg == WM_POWERBROADCAST {
        let sleep = match wparam.0 as u32 {
            PBT_APMSUSPEND => Some(true),
            PBT_APMRESUMEAUTOMATIC | PBT_APMRESUMESUSPEND => Some(false),
            _ => None,
        };
        if let Some(sleep) = sleep {
            if let Ok(guard) = SLEEP_TX.lock() {
                if let Some(tx) = guard.as_ref() {
                    let _ = tx.send(DeviceEvent::Sleep { sleep });
                }
            }
        }
        return LRESULT(1);
    }
    DefWindowProcW(hwnd, msg, wparam, lparam)
}

/// Listens for system suspend/resume and emits [`DeviceEvent::Sleep`].
pub fn spawn_sleep_listener(
    tx: &Sender<DeviceEvent>,
    active: &Arc<AtomicBool>,
    _should_reset: &Arc<AtomicBool>,
) {
    *SLEEP_TX.lock().unwrap() = Some(tx.clone());
    let active = active.clone();

    thread::spawn(move || unsafe {
        let hmodule = match GetModuleHandleW(None) {
            Ok(h) => h,
            Err(e) => {
                error_log!("Failed to get module handle for power listener: {}", e);
                return;
            }
        };
        let hinstance = HINSTANCE(hmodule.0);
        let class_name = w!("KeyDeckPowerListener");
        let wc = WNDCLASSW {
            lpfnWndProc: Some(power_wndproc),
            hInstance: hinstance,
            lpszClassName: class_name,
            ..Default::default()
        };
        RegisterClassW(&wc);

        let hwnd = match CreateWindowExW(
            WINDOW_EX_STYLE(0),
            class_name,
            w!("KeyDeck"),
            WINDOW_STYLE(0),
            0,
            0,
            0,
            0,
            HWND_MESSAGE,
            None,
            hinstance,
            None,
        ) {
            Ok(h) => h,
            Err(e) => {
                error_log!("Failed to create power listener window: {}", e);
                return;
            }
        };

        verbose_log!("Windows power listener started");

        let mut msg = MSG::default();
        while active.load(Ordering::Relaxed) && GetMessageW(&mut msg, hwnd, 0, 0).as_bool() {
            let _ = TranslateMessage(&msg);
            DispatchMessageW(&msg);
        }

        verbose_log!("Windows power listener exiting");
    });
}
