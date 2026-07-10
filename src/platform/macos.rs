// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2025 Panayotis Katsaloulis

//! macOS platform backend: window focus (query + activate), sleep/resume
//! notification, exit cleanup and shell selection. Keyboard injection and the
//! config-reload listener are shared with Windows (`platform::keymap` /
//! `platform::reload`).
//!
//! Focus and sleep are fully event-driven via `NSWorkspace` notifications.
//! AppKit only delivers those to a thread running a Cocoa run loop, so the
//! observers are registered and serviced on the process main thread by
//! [`run_main_thread`] (the mpsc event loop runs on a worker thread instead —
//! see `server::start_server`). The listener spawn functions therefore just
//! record the event sender for `run_main_thread` to use.

use crate::event::DeviceEvent;
use crate::verbose_log;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc::Sender;
use std::sync::{Arc, Mutex};

use objc2_app_kit::{NSApplicationActivationOptions, NSWorkspace};

/// Event senders recorded by the listener-spawn functions and consumed by
/// [`run_main_thread`] when it registers the notification observers.
static FOCUS_TX: Mutex<Option<Sender<DeviceEvent>>> = Mutex::new(None);
static SLEEP_TX: Mutex<Option<Sender<DeviceEvent>>> = Mutex::new(None);

/// Shell used to run `Exec` actions.
pub fn exec_shell() -> (&'static str, &'static str) {
    ("/bin/sh", "-c")
}

/// No macOS-specific teardown is required on exit.
pub fn on_exit_cleanup() {}

// ---------------------------------------------------------------------------
// Frontmost application
// ---------------------------------------------------------------------------

/// Returns `(app_name, window_title)` for the frontmost application.
///
/// The class is the frontmost application's localized name (queried fresh from
/// `NSWorkspace`). The window title is left empty: it would require the Screen
/// Recording permission and per-window Accessibility observers, whereas
/// app-level matching covers the common case. Returns `None` when there is no
/// frontmost application.
fn current_focus() -> Option<(String, String)> {
    unsafe {
        let workspace = NSWorkspace::sharedWorkspace();
        let app = workspace.frontmostApplication()?;
        let name = app
            .localizedName()
            .map(|s| s.to_string())
            .unwrap_or_default();
        if name.is_empty() {
            None
        } else {
            Some((name, String::new()))
        }
    }
}

// ---------------------------------------------------------------------------
// Listener registration (senders stored; observers set up in run_main_thread)
// ---------------------------------------------------------------------------

pub fn spawn_focus_listener(tx: &Sender<DeviceEvent>, _active: &Arc<AtomicBool>) {
    *FOCUS_TX.lock().unwrap() = Some(tx.clone());
}

pub fn spawn_sleep_listener(
    tx: &Sender<DeviceEvent>,
    _active: &Arc<AtomicBool>,
    _should_reset: &Arc<AtomicBool>,
) {
    *SLEEP_TX.lock().unwrap() = Some(tx.clone());
}

// ---------------------------------------------------------------------------
// Main-thread Cocoa run loop: focus + sleep notification observers
// ---------------------------------------------------------------------------

/// Registers the `NSWorkspace` notification observers and services them on the
/// current (main) thread's run loop until `active` becomes false.
///
/// * `NSWorkspaceDidActivateApplicationNotification` → [`DeviceEvent::FocusChanges`]
/// * `NSWorkspaceWillSleepNotification` → `Sleep { sleep: true }`
/// * `NSWorkspaceDidWakeNotification` → `Sleep { sleep: false }`
pub fn run_main_thread(active: &Arc<AtomicBool>) {
    use block2::RcBlock;
    use objc2_foundation::{NSDate, NSNotification, NSRunLoop, NSString};
    use std::ptr::NonNull;

    unsafe {
        let workspace = NSWorkspace::sharedWorkspace();
        let center = workspace.notificationCenter();

        let focus_block = RcBlock::new(|_note: NonNull<NSNotification>| {
            if let Some((class, title)) = current_focus() {
                if let Ok(guard) = FOCUS_TX.lock() {
                    if let Some(tx) = guard.as_ref() {
                        verbose_log!("Focus change: class='{}' title='{}'", class, title);
                        let _ = tx.send(DeviceEvent::FocusChanges { class, title });
                    }
                }
            }
        });
        let sleep_block = RcBlock::new(|_note: NonNull<NSNotification>| {
            if let Ok(guard) = SLEEP_TX.lock() {
                if let Some(tx) = guard.as_ref() {
                    let _ = tx.send(DeviceEvent::Sleep { sleep: true });
                }
            }
        });
        let wake_block = RcBlock::new(|_note: NonNull<NSNotification>| {
            if let Ok(guard) = SLEEP_TX.lock() {
                if let Some(tx) = guard.as_ref() {
                    let _ = tx.send(DeviceEvent::Sleep { sleep: false });
                }
            }
        });

        let activate = NSString::from_str("NSWorkspaceDidActivateApplicationNotification");
        let will_sleep = NSString::from_str("NSWorkspaceWillSleepNotification");
        let did_wake = NSString::from_str("NSWorkspaceDidWakeNotification");

        let _focus_obs = center.addObserverForName_object_queue_usingBlock(
            Some(&activate),
            None,
            None,
            &focus_block,
        );
        let _sleep_obs = center.addObserverForName_object_queue_usingBlock(
            Some(&will_sleep),
            None,
            None,
            &sleep_block,
        );
        let _wake_obs = center.addObserverForName_object_queue_usingBlock(
            Some(&did_wake),
            None,
            None,
            &wake_block,
        );

        // Emit the current focus once so the correct page is shown on startup.
        if let Some((class, title)) = current_focus() {
            if let Ok(guard) = FOCUS_TX.lock() {
                if let Some(tx) = guard.as_ref() {
                    let _ = tx.send(DeviceEvent::FocusChanges { class, title });
                }
            }
        }

        verbose_log!("macOS main run loop started (focus + sleep observers)");

        let run_loop = NSRunLoop::currentRunLoop();
        while active.load(Ordering::Relaxed) {
            let until = NSDate::dateWithTimeIntervalSinceNow(0.5);
            run_loop.runUntilDate(&until);
        }
        verbose_log!("macOS main run loop exiting");
    }
}

// ---------------------------------------------------------------------------
// set_focus: activate a matching application
// ---------------------------------------------------------------------------

/// Activates an application matching `class` (app name / bundle id) and/or
/// `title`. macOS activation is application-scoped; matching semantics mirror
/// the Linux backend (substring, case-insensitive; OR when the same string is
/// supplied for both, which the `Focus` action does).
pub fn set_focus(class: &String, title: &String) -> Result<(), String> {
    if class.is_empty() && title.is_empty() {
        return Err("At least one of class or title must be specified".to_string());
    }

    let needles: Vec<String> = [class, title]
        .iter()
        .filter(|s| !s.is_empty())
        .map(|s| s.to_lowercase())
        .collect();

    unsafe {
        let workspace = NSWorkspace::sharedWorkspace();
        let apps = workspace.runningApplications();
        for i in 0..apps.count() {
            let app = apps.objectAtIndex(i);
            let name = app
                .localizedName()
                .map(|s| s.to_string())
                .unwrap_or_default()
                .to_lowercase();
            let bundle = app
                .bundleIdentifier()
                .map(|s| s.to_string())
                .unwrap_or_default()
                .to_lowercase();

            let hit = needles
                .iter()
                .any(|n| name.contains(n) || bundle.contains(n));
            if hit {
                app.activateWithOptions(
                    NSApplicationActivationOptions::NSApplicationActivateAllWindows,
                );
                return Ok(());
            }
        }
    }

    Err(format!(
        "No matching application found using class '{}' and title '{}'",
        class, title
    ))
}
