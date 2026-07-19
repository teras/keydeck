"""Push terminal context to the keydeck daemon.

Reports the program running in the focused kitty window (and whether the cwd is a
git repo) to keydeck via `keydeck --set`. Pages can auto-switch on the `context` /
`git` variables, e.g.:

    Kitty (Claude):
      when: { window: kitty, context: claude }

Installed and registered by `keydeck --integration kitty install`. This file is
managed by keydeck and refreshed on daemon startup — edit APPS below, but expect
manual changes elsewhere to be overwritten on upgrade.

Event-driven, mirroring kitty's layout_switch.py convention:
  * on_focus_change   — switching to a window/tab
  * on_cmd_startstop  — launching/exiting a program in the focused window
                        (requires kitty shell integration; ignored if unavailable)
  * on_title_change   — catches in-place launches: an app sets its title once it is
                        up and in the foreground, so the /proc read is accurate then

Detection reports the terminal's foreground program — the process group in the
foreground of the shell's controlling terminal (tpgid). So for a chain
kitty -> fish -> mc -> claude, `claude` wins (it is what you are interacting
with); a Ctrl-Z'd background job does not count. This matches the konsole
integration, which reads the same foreground process. Only programs listed in
APPS are reported; extend it with your own.
"""

import os
import subprocess

# Programs of interest inside a terminal: only these are reported as `context`
# (anything else, including the bare shell, reports empty). Kept in sync with the
# konsole integration's default set; extend with your own.
APPS = {"claude", "codex", "opencode", "mc"}


def _proc_name(pid):
    """argv[0] basename of a process (matches what the user typed)."""
    try:
        with open("/proc/%d/cmdline" % pid, "rb") as f:
            first = f.read().split(b"\x00", 1)[0]
        if first:
            return os.path.basename(first.decode("utf-8", "replace"))
    except OSError:
        pass
    return ""


def _foreground_pid(shell_pid):
    """Foreground process-group leader on the shell's controlling terminal (tpgid) —
    the program you are actually interacting with. Ignores Ctrl-Z'd background jobs
    and matches what konsole's foregroundProcessId returns. Falls back to the shell."""
    try:
        with open("/proc/%d/stat" % shell_pid) as f:
            data = f.read()
        # comm (field 2) is parenthesised and may contain spaces or ')', so split
        # after the last ')' to keep the remaining field offsets stable:
        # state ppid pgrp session tty_nr tpgid ...
        after = data[data.rfind(")") + 1:].split()
        tpgid = int(after[5])
        if tpgid > 0:
            return tpgid
    except (OSError, ValueError, IndexError):
        pass
    return shell_pid


def _shell_pid(window):
    try:
        return window.child.pid
    except Exception:
        return None


def _cwd(pid):
    try:
        return os.readlink("/proc/%d/cwd" % pid)
    except OSError:
        return ""


def _detect_context(name) -> str:
    # Only known APPS are reported. At the bare shell the foreground program is the
    # shell itself (and transient prompt helpers like git/date flit through it);
    # reporting those would make the context flicker, so anything not in APPS = "".
    return name if name in APPS else ""


def _is_git_repo(cwd: str) -> bool:
    path = cwd
    while path and path != "/":
        if os.path.isdir(os.path.join(path, ".git")):
            return True
        path = os.path.dirname(path)
    return False


def _set(key: str, value: str) -> None:
    try:
        subprocess.Popen(
            ["keydeck", "--set", f"{key}={value}"],
            stdout=subprocess.DEVNULL,
            stderr=subprocess.DEVNULL,
        )
    except OSError:
        pass


# Last values pushed, to avoid redundant `keydeck --set` calls on every shell prompt.
_last = {"context": None, "git": None}


def _push(window) -> None:
    pid = _shell_pid(window)
    if pid is None:
        return
    fg = _foreground_pid(pid)
    ctx = _detect_context(_proc_name(fg))
    git = "1" if _is_git_repo(_cwd(fg)) else ""
    if ctx != _last["context"]:
        _last["context"] = ctx
        _set("context", ctx)
    if git != _last["git"]:
        _last["git"] = git
        _set("git", git)


def _is_focused(boss, window) -> bool:
    active = boss.active_window
    return active is not None and active.id == window.id


def on_focus_change(boss, window, data):
    if data.get("focused"):
        _push(window)


def on_cmd_startstop(boss, window, data):
    if _is_focused(boss, window):
        _push(window)


def on_title_change(boss, window, data):
    if _is_focused(boss, window):
        _push(window)
