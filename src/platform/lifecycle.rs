// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2025 Panayotis Katsaloulis

//! Per-OS daemon lifecycle management, exposed to the config UI via the single
//! CLI verb `keydeck --daemon <action>`.
//!
//! This keeps the config UI (and any other caller) completely free of
//! OS-specific service knowledge: it never touches `systemctl`, the Windows
//! registry, `launchctl` or POSIX signals — it just runs the daemon binary
//! with the right verb and reads the exit code (and, for `status`, JSON on
//! stdout). All platform detail lives here.
//!
//! The seven verbs are orthogonal along two axes:
//!
//! * **Autostart** (persistent — "does it start at login?"): `install`,
//!   `uninstall`. These only register/unregister; they do not start or stop a
//!   running instance.
//! * **Runtime** (transient — "is it running now?"): `start`, `stop`,
//!   `restart`, plus `reload` (tell the running instance to re-read its
//!   config) and `status` (report both axes).
//!
//! Each OS uses the native mechanism that runs the daemon **inside the
//! interactive desktop session** — required because keyboard injection and
//! window focus need an interactive session on Windows (UIPI) and macOS:
//!
//! * **Linux**   — systemd *user* service (`~/.config/systemd/user`).
//! * **Windows** — `HKCU\…\Run` registry value + a detached process.
//! * **macOS**   — a `LaunchAgent` plist in `~/Library/LaunchAgents`.

use std::env;
use std::io;

/// Reverse-DNS label for the macOS LaunchAgent / Windows Run value.
#[cfg(not(target_os = "linux"))]
const LABEL: &str = "com.teras.keydeck";

/// The lifecycle verbs accepted by `--daemon <action>`.
#[derive(Clone, Copy, Debug)]
pub enum Action {
    Install,
    Uninstall,
    Start,
    Stop,
    Restart,
    Status,
    Reload,
}

impl Action {
    /// Parses a CLI verb into an [`Action`], or `None` if unrecognised.
    pub fn parse(s: &str) -> Option<Action> {
        Some(match s {
            "install" => Action::Install,
            "uninstall" => Action::Uninstall,
            "start" => Action::Start,
            "stop" => Action::Stop,
            "restart" => Action::Restart,
            "status" => Action::Status,
            "reload" => Action::Reload,
            _ => return None,
        })
    }

    /// The set of valid verbs, for help/error messages.
    pub const NAMES: &'static str = "install, uninstall, start, stop, restart, status, reload";
}

/// Absolute path to the running `keydeck` binary, used in autostart entries
/// and when spawning a detached server instance.
fn current_exe() -> io::Result<String> {
    env::current_exe()?
        .to_str()
        .map(|s| s.to_string())
        .ok_or_else(|| {
            io::Error::new(io::ErrorKind::InvalidData, "executable path is not valid UTF-8")
        })
}

/// Runs a lifecycle action, returning the process exit code to propagate.
pub fn run(action: Action) -> io::Result<i32> {
    match action {
        Action::Status => status(),
        other => imp::run(other),
    }
}

/// Prints daemon status as JSON (`running`, `pid`, `enabled`) and returns exit
/// code 0 if the daemon is currently running, 1 otherwise. The `running`/`pid`
/// half is fully cross-platform (lock file + `sysinfo`); only `enabled` is
/// delegated to the per-OS backend.
fn status() -> io::Result<i32> {
    let pid = crate::lock::running_pid();
    let running = pid.is_some();
    let enabled = imp::is_enabled();
    let pid_json = match pid {
        Some(p) => p.to_string(),
        None => "null".to_string(),
    };
    println!("{{\"running\":{},\"pid\":{},\"enabled\":{}}}", running, pid_json, enabled);
    Ok(if running { 0 } else { 1 })
}

// ---------------------------------------------------------------------------
// Linux — systemd user service
// ---------------------------------------------------------------------------
#[cfg(target_os = "linux")]
mod imp {
    use super::{current_exe, Action};
    use std::fs;
    use std::io;
    use std::path::PathBuf;
    use std::process::Command;

    const UNIT: &str = "keydeck.service";

    fn service_path() -> io::Result<PathBuf> {
        let dir = keydeck::get_config_dir()
            .parent()
            .map(|p| p.join("systemd/user"))
            .ok_or_else(|| io::Error::new(io::ErrorKind::NotFound, "cannot locate ~/.config"))?;
        Ok(dir.join(UNIT))
    }

    pub fn run(action: Action) -> io::Result<i32> {
        match action {
            Action::Install => install(),
            Action::Uninstall => uninstall(),
            Action::Start => systemctl(&["start", UNIT]),
            Action::Stop => systemctl(&["stop", UNIT]),
            Action::Restart => systemctl(&["restart", UNIT]),
            Action::Reload => reload(),
            Action::Status => unreachable!("status handled by parent"),
        }
    }

    pub fn is_enabled() -> bool {
        Command::new("systemctl")
            .args(["--user", "is-enabled", UNIT])
            .output()
            .map(|o| o.status.success())
            .unwrap_or(false)
    }

    fn install() -> io::Result<i32> {
        let exe = current_exe()?;
        let path = service_path()?;
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)?;
        }
        let unit = format!(
            "[Unit]\n\
             Description=KeyDeck Daemon\n\
             After=graphical-session.target\n\
             \n\
             [Service]\n\
             Type=simple\n\
             ExecStart={exe}\n\
             Restart=on-failure\n\
             RestartSec=5\n\
             \n\
             [Install]\n\
             WantedBy=default.target\n",
        );
        fs::write(&path, unit)?;
        run_ok(systemctl(&["daemon-reload"]))?;
        // enable (autostart at login) but do NOT start now — `start` is separate.
        let code = systemctl(&["enable", UNIT])?;
        println!("Autostart installed: {}", path.display());
        Ok(code)
    }

    fn uninstall() -> io::Result<i32> {
        let _ = systemctl(&["disable", UNIT]);
        if let Ok(path) = service_path() {
            if path.exists() {
                fs::remove_file(&path)?;
            }
        }
        let _ = systemctl(&["daemon-reload"]);
        println!("Autostart removed.");
        Ok(0)
    }

    fn reload() -> io::Result<i32> {
        match crate::lock::running_pid() {
            Some(pid) => {
                // The daemon installs a SIGHUP handler that triggers a reload.
                let ok = Command::new("kill")
                    .args(["-HUP", &pid.to_string()])
                    .status()?
                    .success();
                Ok(if ok { 0 } else { 1 })
            }
            None => {
                eprintln!("keydeck is not running");
                Ok(1)
            }
        }
    }

    /// Runs `systemctl --user <args>` and returns its exit code.
    fn systemctl(args: &[&str]) -> io::Result<i32> {
        let status = Command::new("systemctl").arg("--user").args(args).status()?;
        Ok(status.code().unwrap_or(1))
    }

    /// Turns a non-zero exit code from a prerequisite step into an error.
    fn run_ok(code: io::Result<i32>) -> io::Result<()> {
        match code? {
            0 => Ok(()),
            c => Err(io::Error::new(
                io::ErrorKind::Other,
                format!("systemctl step failed with code {c}"),
            )),
        }
    }
}

// ---------------------------------------------------------------------------
// Windows — HKCU\…\Run registry value + detached process
// ---------------------------------------------------------------------------
#[cfg(target_os = "windows")]
mod imp {
    use super::{current_exe, Action, LABEL};
    use std::io;
    use std::os::windows::process::CommandExt;
    use std::process::{Command, Stdio};

    const RUN_KEY: &str = r"HKCU\Software\Microsoft\Windows\CurrentVersion\Run";

    /// `DETACHED_PROCESS`: the spawned server gets no console and is not tied to
    /// the launcher's console/handles, so it keeps running independently after
    /// the controller (and its caller) exit.
    const DETACHED_PROCESS: u32 = 0x0000_0008;

    pub fn run(action: Action) -> io::Result<i32> {
        match action {
            Action::Install => install(),
            Action::Uninstall => uninstall(),
            Action::Start => start(),
            Action::Stop => stop(),
            Action::Restart => {
                stop()?;
                start()
            }
            Action::Reload => {
                // The daemon watches its config file, so it reloads on its own.
                println!("Config reload happens automatically (file watch).");
                Ok(0)
            }
            Action::Status => unreachable!("status handled by parent"),
        }
    }

    pub fn is_enabled() -> bool {
        Command::new("reg")
            .args(["query", RUN_KEY, "/v", LABEL])
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .status()
            .map(|s| s.success())
            .unwrap_or(false)
    }

    fn install() -> io::Result<i32> {
        let exe = current_exe()?;
        // Autostart via `--daemon start` (not `--server` directly) so the
        // login-launched daemon goes through start(), which redirects its
        // stdout/stderr to the log file. Launching `--server` raw from the Run
        // key would discard all output, leaving the log viewer empty for the
        // common (autostarted) case. start() is idempotent, so this is safe.
        let value = format!("\"{exe}\" --daemon start");
        let status = Command::new("reg")
            .args(["add", RUN_KEY, "/v", LABEL, "/t", "REG_SZ", "/d", &value, "/f"])
            .status()?;
        println!("Autostart installed: {RUN_KEY}\\{LABEL}");
        Ok(status.code().unwrap_or(1))
    }

    fn uninstall() -> io::Result<i32> {
        let _ = Command::new("reg")
            .args(["delete", RUN_KEY, "/v", LABEL, "/f"])
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .status();
        println!("Autostart removed.");
        Ok(0)
    }

    /// Spawns a detached `keydeck --server`. There is no user-level service
    /// manager on Windows, so the daemon runs as a plain background process.
    fn start() -> io::Result<i32> {
        if crate::lock::running_pid().is_some() {
            println!("keydeck is already running.");
            return Ok(0);
        }
        let exe = current_exe()?;
        // Capture the detached daemon's stdout/stderr to a log file so the
        // config UI can tail it (Windows has no per-service journal). The file
        // is truncated on each explicit start. If it cannot be opened, fall
        // back to discarding output rather than failing to start.
        let log = keydeck::get_log_path();
        if let Some(parent) = log.parent() {
            let _ = std::fs::create_dir_all(parent);
        }
        let (out, err) = match std::fs::File::create(&log) {
            Ok(f) => match f.try_clone() {
                Ok(f2) => (Stdio::from(f), Stdio::from(f2)),
                Err(_) => (Stdio::null(), Stdio::null()),
            },
            Err(_) => (Stdio::null(), Stdio::null()),
        };
        Command::new(&exe)
            .arg("--server")
            .stdin(Stdio::null())
            .stdout(out)
            .stderr(err)
            .creation_flags(DETACHED_PROCESS)
            .spawn()?;
        println!("keydeck started.");
        Ok(0)
    }

    /// Terminates the running daemon (looked up via the lock file).
    fn stop() -> io::Result<i32> {
        match crate::lock::running_pid() {
            Some(pid) => {
                let ok = Command::new("taskkill")
                    .args(["/PID", &pid.to_string(), "/F"])
                    .stdout(Stdio::null())
                    .stderr(Stdio::null())
                    .status()?
                    .success();
                println!("keydeck stopped.");
                Ok(if ok { 0 } else { 1 })
            }
            None => {
                println!("keydeck is not running.");
                Ok(0)
            }
        }
    }
}

// ---------------------------------------------------------------------------
// macOS — LaunchAgent plist
// ---------------------------------------------------------------------------
#[cfg(target_os = "macos")]
mod imp {
    use super::{current_exe, Action, LABEL};
    use std::fs;
    use std::io;
    use std::path::PathBuf;
    use std::process::{Command, Stdio};

    fn plist_path() -> io::Result<PathBuf> {
        let home = std::env::var_os("HOME")
            .ok_or_else(|| io::Error::new(io::ErrorKind::NotFound, "HOME is not set"))?;
        Ok(PathBuf::from(home)
            .join("Library/LaunchAgents")
            .join(format!("{LABEL}.plist")))
    }

    pub fn run(action: Action) -> io::Result<i32> {
        match action {
            Action::Install => install(),
            Action::Uninstall => uninstall(),
            Action::Start => launchctl(&["load", &plist_str()?]),
            Action::Stop => launchctl(&["unload", &plist_str()?]),
            Action::Restart => {
                let p = plist_str()?;
                let _ = launchctl(&["unload", &p]);
                launchctl(&["load", &p])
            }
            Action::Reload => {
                // The daemon watches its config file, so it reloads on its own.
                println!("Config reload happens automatically (file watch).");
                Ok(0)
            }
            Action::Status => unreachable!("status handled by parent"),
        }
    }

    pub fn is_enabled() -> bool {
        plist_path().map(|p| p.exists()).unwrap_or(false)
    }

    fn plist_str() -> io::Result<String> {
        Ok(plist_path()?.to_string_lossy().into_owned())
    }

    fn install() -> io::Result<i32> {
        let exe = current_exe()?;
        let path = plist_path()?;
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)?;
        }
        // Capture the daemon's stdout/stderr to a log file so the config UI can
        // tail it (macOS has no per-service journal). launchd opens these paths
        // before exec, so the directory must already exist.
        let log = keydeck::get_log_path();
        if let Some(parent) = log.parent() {
            fs::create_dir_all(parent)?;
        }
        let log = log.to_string_lossy();
        let plist = format!(
            "<?xml version=\"1.0\" encoding=\"UTF-8\"?>\n\
             <!DOCTYPE plist PUBLIC \"-//Apple//DTD PLIST 1.0//EN\" \"http://www.apple.com/DTDs/PropertyList-1.0.dtd\">\n\
             <plist version=\"1.0\">\n\
             <dict>\n\
             \t<key>Label</key>\n\
             \t<string>{LABEL}</string>\n\
             \t<key>ProgramArguments</key>\n\
             \t<array>\n\
             \t\t<string>{exe}</string>\n\
             \t\t<string>--server</string>\n\
             \t</array>\n\
             \t<key>RunAtLoad</key>\n\
             \t<true/>\n\
             \t<key>KeepAlive</key>\n\
             \t<true/>\n\
             \t<key>StandardOutPath</key>\n\
             \t<string>{log}</string>\n\
             \t<key>StandardErrorPath</key>\n\
             \t<string>{log}</string>\n\
             </dict>\n\
             </plist>\n",
        );
        fs::write(&path, plist)?;
        // Registered for autostart; `start` (launchctl load) runs it now.
        println!("Autostart installed: {}", path.display());
        Ok(0)
    }

    fn uninstall() -> io::Result<i32> {
        if let Ok(path) = plist_path() {
            if path.exists() {
                let _ = Command::new("launchctl")
                    .args(["unload", &path.to_string_lossy()])
                    .stderr(Stdio::null())
                    .status();
                fs::remove_file(&path)?;
            }
        }
        println!("Autostart removed.");
        Ok(0)
    }

    fn launchctl(args: &[&str]) -> io::Result<i32> {
        let status = Command::new("launchctl").args(args).status()?;
        Ok(status.code().unwrap_or(1))
    }
}
