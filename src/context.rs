// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2025 Panayotis Katsaloulis

//! Shared store of external "context variables".
//!
//! An external process (e.g. a terminal watcher) pushes `key=value` pairs into the
//! running daemon via the control socket (`keydeck --set`). Pages match against them
//! through the unified `when` conditions, and buttons can display them via
//! `${var:name}`. The store is independent of the config file and survives reloads.

use indexmap::IndexMap;
use std::sync::{Arc, RwLock};

/// Thread-shared map of context variable name -> value.
pub type ContextVars = Arc<RwLock<IndexMap<String, String>>>;

/// Creates a new empty context-variable store.
pub fn new_context_vars() -> ContextVars {
    Arc::new(RwLock::new(IndexMap::new()))
}

/// A pull-style context source that the daemon must poke when a matching window
/// gains focus (e.g. an in-daemon D-Bus resolver). The core knows nothing about
/// what the source is — the `pattern` and the `on_focus` hook are supplied by the
/// integration that owns that knowledge. Push sources (external watchers that call
/// `keydeck --set` on their own) do not register here at all.
pub struct PullTrigger {
    /// Lowercase substring matched against the focused window class.
    pub pattern: String,
    /// Fired when a window whose class contains `pattern` gains focus; receives the
    /// focused window's title (caption). A source that manages several windows uses it
    /// to identify *which* one is focused, without racing the app's own lagging
    /// "active window" state.
    pub on_focus: Box<dyn Fn(&str) + Send>,
}
