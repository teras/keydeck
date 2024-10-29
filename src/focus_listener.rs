use crate::event::{send, DeviceEvent};
use crate::verbose_log;
use std::error::Error;
use std::sync::atomic::AtomicBool;
use std::sync::mpsc::Sender;
use std::sync::Arc;
use std::thread;
use x11rb::atom_manager;
use x11rb::connection::Connection;
use x11rb::protocol::xproto::{AtomEnum, ConnectionExt, EventMask, PropertyNotifyEvent, Window};
use x11rb::rust_connection::RustConnection;

pub fn focus_listener(tx: &Sender<DeviceEvent>, active: &Arc<AtomicBool>) {
    let active = active.clone();
    let tx = tx.clone();
    thread::spawn(move || {
        let mut listener = match X11FocusListener::new() {
            Ok(l) => l,
            Err(e) => {
                eprintln!("Error while creating focus listener: {}", e);
                return;
            }
        };
        verbose_log!("Starting focus listener");
        while active.load(std::sync::atomic::Ordering::Relaxed) {
            if let Ok((class, title)) = listener.get_next_focus_change() {
                send(&tx, DeviceEvent::FocusChanges { class, title });
            } else {
                eprintln!("Error while getting next focus change");
                return;
            }
        }
        verbose_log!("Exiting focus listener");
    });
}

atom_manager! {
    pub Atoms: AtomsCookie {
        _NET_ACTIVE_WINDOW,
        WM_CLASS,
        _NET_WM_NAME,
        WM_NAME,
        UTF8_STRING,
    }
}

struct X11FocusListener {
    conn: RustConnection,
    root: Window,
    atoms: Atoms,
    last_active_window: Option<Window>,
}

impl X11FocusListener {
    fn new() -> Result<Self, Box<dyn Error>> {
        let (conn, screen_num) = RustConnection::connect(None)?;
        let screen = &conn.setup().roots[screen_num];
        let root = screen.root;

        let atoms = Atoms::new(&conn)?.reply()?;

        conn.change_window_attributes(root, &x11rb::protocol::xproto::ChangeWindowAttributesAux::new()
            .event_mask(EventMask::PROPERTY_CHANGE | EventMask::SUBSTRUCTURE_NOTIFY))?;

        for child in conn.query_tree(root)?.reply()?.children {
            conn.change_window_attributes(child, &x11rb::protocol::xproto::ChangeWindowAttributesAux::new()
                .event_mask(EventMask::PROPERTY_CHANGE))?;
        }

        Ok(Self {
            conn,
            root,
            atoms,
            last_active_window: None,
        })
    }

    fn get_next_focus_change(&mut self) -> Result<(String, String), Box<dyn Error>> {
        loop {
            let event = self.conn.wait_for_event()?;
            if let x11rb::protocol::Event::PropertyNotify(PropertyNotifyEvent { atom, .. }) = event {
                if atom == self.atoms._NET_ACTIVE_WINDOW {
                    if let Ok(reply) = self.conn.get_property(false, self.root, self.atoms._NET_ACTIVE_WINDOW, AtomEnum::WINDOW, 0, 1)?.reply() {
                        if let Some(window_id) = reply.value32().and_then(|mut v| v.next()) {
                            if self.last_active_window != Some(window_id) {
                                self.last_active_window = Some(window_id);
                                let wm_class = self.get_window_class(window_id)?;
                                let wm_title = self.get_window_title(window_id)?;
                                return Ok((wm_class.unwrap_or_default(), wm_title.unwrap_or_default()));
                            }
                        }
                    }
                }
            }
        }
    }

    fn get_window_class(&self, window: Window) -> Result<Option<String>, Box<dyn Error>> {
        if let Ok(reply) = self.conn.get_property(false, window, self.atoms.WM_CLASS, AtomEnum::STRING, 0, 1024)?.reply() {
            if let Some(value) = reply.value8() {
                return Ok(Some(String::from_utf8_lossy(&value.collect::<Vec<u8>>()).split('\0').filter(|s| !s.is_empty()).collect::<Vec<_>>().join(".")));
            }
        }
        Ok(None)
    }

    fn get_window_title(&self, window: Window) -> Result<Option<String>, Box<dyn Error>> {
        if let Ok(reply) = self.conn.get_property(false, window, self.atoms._NET_WM_NAME, self.atoms.UTF8_STRING, 0, 1024)?.reply() {
            if let Some(value) = reply.value8() {
                return Ok(Some(String::from_utf8_lossy(&value.collect::<Vec<u8>>()).to_string()));
            }
        }
        if let Ok(reply) = self.conn.get_property(false, window, self.atoms.WM_NAME, AtomEnum::STRING, 0, 1024)?.reply() {
            if let Some(value) = reply.value8() {
                return Ok(Some(String::from_utf8_lossy(&value.collect::<Vec<u8>>()).to_string()));
            }
        }
        Ok(None)
    }
}