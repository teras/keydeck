use x11rb::connection::Connection;
use x11rb::protocol::xproto::{Atom, ConnectionExt, Window, InputFocus, PropMode};
use x11rb::protocol::xproto::ClientMessageData;
use x11rb::rust_connection::RustConnection;
use x11rb::protocol::xproto::ClientMessageEvent;

fn get_atom(conn: &RustConnection, name: &str) -> Result<Atom, &'static str> {
    let cookie = conn.intern_atom(false, name.as_bytes()).map_err(|_| "Failed to intern atom")?;
    let reply = cookie.reply().map_err(|_| "Failed to get atom reply")?;
    Ok(reply.atom)
}

// Helper function to check if a window property matches the specified value
fn window_property_matches(
    conn: &RustConnection,
    window: Window,
    property_atom: Atom,
    string_atom: Atom,
    value: &str,
) -> Result<bool, &'static str> {
    let cookie = conn.get_property(false, window, property_atom, string_atom, 0, 1024)
        .map_err(|_| "Failed to get property")?;
    let reply = cookie.reply().map_err(|_| "Failed to get property reply")?;

    let prop_value: Vec<u8> = reply.value8().map(|iter| iter.collect()).unwrap_or_default();
    Ok(String::from_utf8_lossy(&prop_value).contains(value))
}

// Recursive function to search windows and their children
fn recursive_search(
    conn: &RustConnection,
    window: Window,
    wm_name_atom: Atom,
    wm_class_atom: Atom,
    string_atom: Atom,
    title: &str,
    class: &str,
) -> Result<Option<Window>, &'static str> {
    // Check if the current window matches the title or class
    let title_matches = window_property_matches(conn, window, wm_name_atom, string_atom, title).unwrap_or(false);
    let class_matches = window_property_matches(conn, window, wm_class_atom, string_atom, class).unwrap_or(false);

    if title_matches || class_matches {
        return Ok(Some(window));
    }

    // Get the children of the current window to search recursively
    let cookie = conn.query_tree(window).map_err(|_| "Failed to query window tree")?;
    let reply = cookie.reply().map_err(|_| "Failed to get tree reply")?;

    // Recursively search each child window
    for &child in &reply.children {
        if let Some(found) = recursive_search(conn, child, wm_name_atom, wm_class_atom, string_atom, title, class)? {
            return Ok(Some(found));
        }
    }
    Ok(None)
}

pub fn set_focus(title: &str, class: &str) -> Result<(), &'static str> {
    // Connect to X11 and retrieve root window and atoms
    let (conn, screen_num) = RustConnection::connect(None).map_err(|_| "Failed to connect to X11")?;
    let screen = &conn.setup().roots[screen_num];
    let (root, wm_name_atom, wm_class_atom, string_atom, net_active_window_atom) = (
        screen.root,
        get_atom(&conn, "WM_NAME")?,
        get_atom(&conn, "WM_CLASS")?,
        get_atom(&conn, "STRING")?,
        get_atom(&conn, "_NET_ACTIVE_WINDOW")?, // Atom for setting the active window
    );

    // Perform a recursive search starting from the root window
    if let Some(win) = recursive_search(&conn, root, wm_name_atom, wm_class_atom, string_atom, title, class)? {
        // Focus the found window
        conn.set_input_focus(InputFocus::POINTER_ROOT, win, x11rb::CURRENT_TIME)
            .map_err(|_| "Failed to set input focus")?;

        // Raise the window to the top of the stacking order
        conn.configure_window(win, &x11rb::protocol::xproto::ConfigureWindowAux::new().stack_mode(x11rb::protocol::xproto::StackMode::ABOVE))
            .map_err(|_| "Failed to raise window")?;

        // Send _NET_ACTIVE_WINDOW client message to the root window to request focus
        let client_message = ClientMessageEvent {
            response_type: x11rb::protocol::xproto::CLIENT_MESSAGE_EVENT,
            format: 32,
            sequence: 0,
            window: win,
            type_: net_active_window_atom,
            data: ClientMessageData::from([1, x11rb::CURRENT_TIME, 0, 0, 0]),
        };

        conn.send_event(false, root, x11rb::protocol::xproto::EventMask::SUBSTRUCTURE_REDIRECT | x11rb::protocol::xproto::EventMask::SUBSTRUCTURE_NOTIFY, client_message)
            .map_err(|_| "Failed to send _NET_ACTIVE_WINDOW message")?;

        Ok(())
    } else {
        Err("Window with specified title or class not found")
    }
}
