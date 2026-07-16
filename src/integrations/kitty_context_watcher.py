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
                        up and visible in /proc, so the tree read is accurate then

Detection walks the process tree under the shell breadth-first (outermost first)
and returns the first program of interest it meets. So for a chain
kitty -> fish -> mc -> claude, `mc` wins (it is the outer/containing program),
even though claude runs underneath. Order of APPS does not matter; tree depth
decides. Extend APPS with your own programs of interest.
"""

import os
import subprocess

# Programs of interest inside a terminal (a set; tree depth decides the winner).
APPS = {"claude", "mc"}


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


def _children(pid):
    try:
        with open("/proc/%d/task/%d/children" % (pid, pid)) as f:
            return [int(x) for x in f.read().split()]
    except (OSError, ValueError):
        return []


def _tree_names(root_pid):
    """Process names under root_pid, breadth-first (outermost first)."""
    names = []
    queue = _children(root_pid)
    seen = set()
    while queue:
        pid = queue.pop(0)
        if pid in seen:
            continue
        seen.add(pid)
        name = _proc_name(pid)
        if name:
            names.append(name)
        queue.extend(_children(pid))
    return names


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


def _detect_context(names) -> str:
    # Outermost program of interest wins. Only known APPS are reported — the shell
    # runs many transient helpers (git/date/… for its prompt) that fire cmd events;
    # reporting those would make the context flicker, so anything not in APPS = "".
    for name in names:
        if name in APPS:
            return name
    return ""


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
    names = _tree_names(pid)
    ctx = _detect_context(names)
    git = "1" if _is_git_repo(_cwd(pid)) else ""
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
