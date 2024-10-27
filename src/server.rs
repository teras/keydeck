use crate::device_manager::{find_device_by_serial, find_path, DeviceManager, StreamDeckDevice};
use crate::pages::{Action, Button, Page, Pages};
use crate::set_focus::set_focus;
use elgato_streamdeck::DeviceStateUpdate;
use image::imageops::overlay;
use image::{open, DynamicImage, Rgba, RgbaImage};
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
    button_images: RefCell<Vec<String>>,
    button_backgrounds: RefCell<Vec<String>>,
}

impl PagedDevice {
    pub fn new(pages: Arc<Pages>, device: StreamDeckDevice) -> Self {
        let current_page = match &pages.main_page {
            Some(page_name) => pages.pages.get_index_of(page_name).unwrap_or(0),
            None => 0,
        };
        let button_count = { device.get_button_count() as usize };
        PagedDevice {
            device,
            pages,
            current_page_ref: RefCell::new(current_page),
            button_images: RefCell::new(vec![String::new(); button_count]),
            button_backgrounds: RefCell::new(vec![String::new(); button_count]),
        }
    }

    pub fn event_loop(&self) {
        self.device.clear_all_button_images().unwrap_or_else(|e| { eprintln!("Error while clearing button images: {}", e) });
        self.device.set_brightness(50).unwrap_or_else(|e| { eprintln!("Error while setting brightness: {}", e) });
        self.refresh_page();
        loop {
            if let Ok(updates) = self.device.get_reader().read(Some(Duration::from_secs_f64(60.0))) {
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

    fn update_image(&self, image: &str, image_path: Option<String>, background: Option<String>, button_index: u8) {
        let image_exists = if image.len() > 0 { find_path(image, image_path) } else { Some(image.to_string()) };
        let image = if let Some(image) = image_exists { image } else {
            eprintln!("Image not found: {}", image);
            "".to_string()
        };
        let bg_color = if let Some(bg_color) = background.as_ref() { bg_color } else { "" };
        {
            // Check if the image and background color are the same as the current ones
            let mut button_images = self.button_images.borrow_mut();
            let mut button_backgrounds = self.button_backgrounds.borrow_mut();
            if button_images[button_index as usize - 1] == image && button_backgrounds[button_index as usize - 1] == bg_color {
                // No need to update the image
                return;
            }
            // Update the image and background color in the cache
            button_images[button_index as usize - 1] = image.clone();
            button_backgrounds[button_index as usize - 1] = bg_color.to_string();
        }
        if image.len() == 0 {
            // Clear the button image instead, since the image is empty
            self.device.clear_button_image(button_index - 1).unwrap_or_else(|e| { eprintln!("Error while clearing button image: {}", e) });
            return;
        }

        let image_data = if let Ok(image_data) = open(&image) { image_data } else {
            eprintln!("Error while opening image: {}", image);
            return;
        };
        if bg_color.len() != 0 {
            if let Ok((r, g, b, a)) = string_to_color(bg_color) {
                let bg_color = Rgba([r, g, b, a]);
                let mut background = RgbaImage::from_pixel(image_data.width(), image_data.height(), bg_color);
                overlay(&mut background, &image_data, 0, 0);
                self.device.set_button_image(button_index - 1, DynamicImage::from(background)).unwrap_or_else(|e| { eprintln!("Error while setting button image: {}", e) });
            } else {
                self.device.set_button_image(button_index - 1, image_data).unwrap_or_else(|e| { eprintln!("Error while setting button image: {}", e) });
            }
        } else {
            self.device.set_button_image(button_index - 1, image_data).unwrap_or_else(|e| { eprintln!("Error while setting button image: {}", e) });
        }
    }

    fn refresh_page(&self) {
        let button_count = self.device.get_button_count();
        let current_page = { self.current_page_ref.borrow().clone() };
        for button_index in 1..=button_count {
            if let Some(button) = self.find_button(current_page, button_index).as_ref() {
                if let Some(icon) = &button.icon {
                    self.update_image(icon, self.pages.image_dir.clone(), button.background.clone(), button_index);
                } else {
                    self.update_image("", None, button.background.clone(), button_index);
                }
            } else {
                self.update_image("", None, None, button_index);
            }
        }
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
        } else {
            eprintln!("Page not found: {}", page_name);
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
            if let Some(actions) = &button.actions {
                for action in actions {
                    match action {
                        Action::Exec { exec } => {
                            std::process::Command::new("bash").arg("-c").arg(exec).spawn().expect("Failed to execute command");
                        }
                        Action::Jump { jump } => {
                            self.set_page(jump);
                        }
                        Action::Focus { focus } => {
                            set_focus(focus, &"".to_string()).unwrap_or_else(|e| { eprintln!("Error: {}", e); });
                        }
                        Action::Wait { wait } => {
                            thread::sleep(Duration::from_millis((wait * 1000.0) as u64));
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