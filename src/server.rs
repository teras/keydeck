use crate::device_manager::{find_device_by_serial, DeviceManager};
use crate::paged_device::PagedDevice;
use crate::pages::Pages;
use std::sync::Arc;
use std::thread;

pub fn start_server(manager: &mut DeviceManager) {
    let pages = Arc::new(Pages::new());
    manager.flush_devices().expect("Unable to flush devices");
    let mut handles = vec![];
    for core_dev in manager.iter_active_devices() {
        let sn = core_dev.get_deck().serial_number().unwrap();
        let pages_handle = pages.clone();
        let thread_handle = thread::spawn(move || {
            if let Some(device) = find_device_by_serial(&sn) {
                PagedDevice::new(pages_handle, device).event_loop();
            } else { panic!("Device not found"); }
        });
        handles.push(thread_handle);
    }
    for handle in handles {
        handle.join().expect("Failed to join thread");
    }
}