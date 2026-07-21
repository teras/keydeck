"""Push terminal context to the keydeck daemon.

Reports the program running in the focused kitty window (and whether the cwd is a
git repo) to keydeck via `keydeck --set`. Pages can auto-switch on the single
`terminal_app` variable (a known program wins; else "git" at a bare shell inside a
repo; else empty), e.g.:

    Kitty (Claude):
      when: { window: kitty, terminal_app: claude }

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


def _stat_fields(pid):
    """(ppid, pgrp) from /proc/<pid>/stat, skipping the parenthesised comm field
    (which may contain spaces or ')'). Returns (None, None) on failure."""
    try:
        with open("/proc/%d/stat" % pid) as f:
            data = f.read()
        # after comm: state ppid pgrp session ...
        after = data[data.rfind(")") + 1:].split()
        return int(after[1]), int(after[2])
    except (OSError, ValueError, IndexError):
        return None, None


def _deepest_app(group_pid) -> str:
    """The deepest program in APPS within the terminal's foreground process group.
    Scanning the whole group (not just its leader) means a wrapper that launches the
    real app in the same group (script -> mc -> claude) still resolves to the
    innermost known app; unknown descendants the app spawned are ignored. Returns ""
    when the job runs no known program (a bare shell), so callers fall back to git."""
    members = {}  # pid -> ppid, every process in the foreground group
    for entry in os.listdir("/proc"):
        if not entry.isdigit():
            continue
        pid = int(entry)
        ppid, pgrp = _stat_fields(pid)
        if pgrp == group_pid and ppid is not None:
            members[pid] = ppid
    best_name, best_depth = "", -1
    for pid in members:
        name = _proc_name(pid)
        if name not in APPS:
            continue
        depth, cur, seen = 0, pid, set()
        while cur in members and cur not in seen:
            seen.add(cur)
            cur = members[cur]
            depth += 1
        if depth > best_depth:
            best_name, best_depth = name, depth
    return best_name


def _is_git_repo(cwd: str) -> bool:
    path = cwd
    while path and path != "/":
        if os.path.isdir(os.path.join(path, ".git")):
            return True
        path = os.path.dirname(path)
    return False


# Absolute path to the keydeck binary, substituted by the daemon when it writes
# this watcher. kitty's environment PATH frequently does not include keydeck's
# install dir, so a bare "keydeck" call would raise OSError and be swallowed below,
# silently dropping every context update. Falls back to a PATH lookup only if the
# placeholder was left untouched (e.g. running this file directly).
KEYDECK_BIN = "@KEYDECK_BIN@"


def _keydeck() -> str:
    return KEYDECK_BIN if not KEYDECK_BIN.startswith("@") else "keydeck"


def _set(value: str) -> None:
    try:
        subprocess.Popen(
            [_keydeck(), "--set", "terminal_app=%s" % value],
            stdout=subprocess.DEVNULL,
            stderr=subprocess.DEVNULL,
        )
    except OSError:
        pass


# Last value pushed, to avoid redundant `keydeck --set` calls on every shell prompt.
_last = {"terminal_app": None}


def _push(window, force=False) -> None:
    # `force` re-asserts on focus-in even if our own last-pushed value is unchanged:
    # another terminal (konsole) may have written the shared var meanwhile, so gaining
    # focus must overwrite it with ours. The _last throttle still spares the frequent
    # title/cmd events from redundant `keydeck --set` subprocesses.
    pid = _shell_pid(window)
    if pid is None:
        return
    fg = _foreground_pid(pid)
    # Single priority-encoded value (level-2 resolution of the terminal container):
    # the *deepest* known program in the foreground process group wins — so a wrapper
    # that launches the app in the same group (script -> mc -> claude) still resolves
    # to the innermost known app, not the shallow shell/wrapper; else the reserved
    # "git" if the foreground job's cwd is a repo; else empty.
    value = _deepest_app(fg)
    if not value and _is_git_repo(_cwd(fg)):
        value = "git"
    if force or value != _last["terminal_app"]:
        _last["terminal_app"] = value
        _set(value)


def _is_focused(boss, window) -> bool:
    active = boss.active_window
    return active is not None and active.id == window.id


def on_focus_change(boss, window, data):
    if data.get("focused"):
        _push(window, force=True)


def on_cmd_startstop(boss, window, data):
    if _is_focused(boss, window):
        _push(window)


def on_title_change(boss, window, data):
    if _is_focused(boss, window):
        _push(window)
