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
