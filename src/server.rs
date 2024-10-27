use crate::device_manager::{find_device_by_serial, load_image, DeviceManager, StreamDeckDevice};
use crate::pages::{Action, Button, Page, Pages};
use crate::set_focus::set_focus;
use elgato_streamdeck::DeviceStateUpdate;
use image::imageops::overlay;
use image::{DynamicImage, Rgba, RgbaImage};
use std::cell::RefCell;
use std::num::ParseIntError;
use std::sync::Arc;
use std::thread;
use std::time::Duration;

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

pub struct PagedDevice {
    device: StreamDeckDevice,
    pages: Arc<Pages>,
    current_page_ref: RefCell<usize>,
}

impl PagedDevice {
    pub fn new(pages: Arc<Pages>, device: StreamDeckDevice) -> Self {
        let current_page = match &pages.main_page {
            Some(page_name) => pages.pages.get_index_of(page_name).unwrap_or(0),
            None => 0,
        };
        PagedDevice {
            device,
            pages,
            current_page_ref: RefCell::new(current_page),
        }
    }

    pub fn event_loop(&self) {
        self.refresh_page();
        self.device.set_brightness(50).unwrap_or_else(|e| { eprintln!("Error while setting brightness: {}", e) });
        loop {
            if let Ok(updates) = self.device.get_reader().read(Some(Duration::from_secs_f64(100.0))) {
                for update in updates {
                    match update {
                        DeviceStateUpdate::ButtonDown(button_id) => self.button_down(button_id + 1),
                        DeviceStateUpdate::ButtonUp(button_id) => self.button_up(button_id + 1),
                        DeviceStateUpdate::EncoderDown(encoder_id) => self.encoder_down(encoder_id + 1),
                        DeviceStateUpdate::EncoderUp(encoder_id) => self.encoder_up(encoder_id + 1),
                        DeviceStateUpdate::EncoderTwist(encoder_id, value) => self.encoder_twist(encoder_id + 1, value),
                        DeviceStateUpdate::TouchPointDown(point_id) => self.touch_point_down(point_id + 1),
                        DeviceStateUpdate::TouchPointUp(point_id) => self.touch_point_up(point_id + 1),
                        DeviceStateUpdate::TouchScreenPress(x, y) => self.touch_screen_press(x, y),
                        DeviceStateUpdate::TouchScreenLongPress(x, y) => self.touch_screen_long_press(x, y),
                        DeviceStateUpdate::TouchScreenSwipe(from, to) => self.touch_screen_swipe(from, to),
                    }
                }
            }
        }
    }

    fn refresh_page(&self) {
        // Separate `get_button_count` to avoid repeated borrowing
        let button_count = self.device.get_button_count();
        let current_page = { self.current_page_ref.borrow().clone() };

        for button_index in 1..=button_count {
            if let Some(button) = self.find_button(current_page, button_index).as_ref() {
                if let Some(icon) = &button.icon {
                    if let Ok(image_data) = load_image(icon.as_str(), Some(self.pages.image_dir.clone().unwrap())) {
                        if let Some(bg_color) = button.background.clone() {
                            if let Ok((r, g, b, a)) = string_to_color(bg_color.as_str()) {
                                let bg_color = Rgba([r, g, b, a]);
                                let mut background = RgbaImage::from_pixel(image_data.width(), image_data.height(), bg_color);
                                overlay(&mut background, &image_data, 0, 0);
                                self.device.set_button_image_cached(button_index - 1, DynamicImage::from(background)).unwrap_or_else(|e| { eprintln!("Error while setting button image: {}", e) });
                            } else {
                                self.device.set_button_image_cached(button_index - 1, image_data).unwrap_or_else(|e| { eprintln!("Error while setting button image: {}", e) });
                            }
                        } else {
                            self.device.set_button_image_cached(button_index - 1, image_data).unwrap_or_else(|e| { eprintln!("Error while setting button image: {}", e) });
                        }
                    } else {
                        eprintln!("Error while loading image: {}", icon);
                    }
                }
                // if let Some(text) = &button.text {
                //     match text {
                //         Text::Simple(value) => self.device.set_button_image("").unwrap(),
                //         Text::Detailed { value, fontsize } => self.device.set_button_text(q, value).unwrap(),
                //     }
                // }
            }
        }
        self.device.clear_all_button_images().unwrap_or_else(|e| { eprintln!("Error while clearing button images: {}", e) });
        self.device.flush().unwrap_or_else(|e| { eprintln!("Error while flushing device: {}", e) });
    }

    fn set_page(&self, page_name: &String) {
        let page = self.pages.pages.get_index_of(page_name);
        if let Some(page) = page {
            let old_page = { self.current_page_ref.borrow_mut().clone() };
            if page != old_page {
                self.current_page_ref.replace(page);
                self.refresh_page();
            }
        }
    }

    fn find_page(&self, page_id: usize) -> &Page {
        let (_, page) = self.pages.pages.get_index(page_id).unwrap_or_else(|| {
            eprintln!("Page not found: {}", page_id);
            std::process::exit(1);
        });
        page
    }

    fn find_button(&self, page_id: usize, button_id: u8) -> &Option<Button> {
        let key = format!("button{}", button_id); // Generate the key based on button_id
        self.find_page(page_id).buttons.get(&key).unwrap_or(&None)
    }

    fn button_down(&self, _button_id: u8) {}

    fn button_up(&self, button_id: u8) {
        let current_page = { self.current_page_ref.borrow().clone() };
        if let Some(button) = self.find_button(current_page, button_id) {
            if let Some(actions) = &button.actions { // Handle Option<Vec<Action>>
                for action in actions {
                    match action {
                        Action::Exec { exec } => {
                            println!("Executing: {}", exec);
                            // std::process::Command::new(exec).spawn().expect("Failed to execute command");
                        }
                        Action::Jump { jump } => {
                            self.set_page(jump);
                        }
                        Action::Focus { focus } => {
                            set_focus(focus, &"".to_string()).unwrap_or_else(|e| { eprintln!("Error: {}", e); });
                        }
                    }
                }
            }
        }
    }

    fn encoder_down(&self, encoder_id: u8) {
        println!("Encoder down: {}", encoder_id);
    }

    fn encoder_up(&self, encoder_id: u8) {
        println!("Encoder up: {}", encoder_id);
    }

    fn encoder_twist(&self, encoder_id: u8, value: i8) {
        println!("Encoder twist: {} {}", encoder_id, value);
    }

    fn touch_point_down(&self, point_id: u8) {
        println!("Touch point down: {}", point_id);
    }

    fn touch_point_up(&self, point_id: u8) {
        println!("Touch point up: {}", point_id);
    }

    fn touch_screen_press(&self, x: u16, y: u16) {
        println!("Touch screen press: {} {}", x, y);
    }

    fn touch_screen_long_press(&self, x: u16, y: u16) {
        println!("Touch screen long press: {} {}", x, y);
    }

    fn touch_screen_swipe(&self, from: (u16, u16), to: (u16, u16)) {
        println!("Touch screen swipe: {:?} {:?}", from, to);
    }
}

fn string_to_color(color: &str) -> Result<(u8, u8, u8, u8), ParseIntError> {
    let r = u8::from_str_radix(&color[0..2], 16)?;
    let g = u8::from_str_radix(&color[2..4], 16)?;
    let b = u8::from_str_radix(&color[4..6], 16)?;
    let a = u8::from_str_radix(&color[6..8], 16)?;
    Ok((r, g, b, a))
}