use crate::button_listener::button_listener;
use crate::device_manager::{find_path, KeyDeckDevice};
use crate::event::DeviceEvent;
use crate::focus_property::set_focus;
use crate::keyboard::send_key_combination;
use crate::pages::{Action, Button, ButtonConfig, FocusChangeRestorePolicy, Page, Pages};
use crate::{error_log, verbose_log};
use image::imageops::overlay;
use image::{open, DynamicImage, Rgba, RgbaImage};
use std::cell::RefCell;
use std::collections::HashMap;
use std::sync::atomic::AtomicBool;
use std::sync::mpsc::Sender;
use std::sync::Arc;
use std::thread;
use std::time::Duration;

pub struct PagedDevice<'a> {
    device: KeyDeckDevice,
    pages: &'a Pages,
    colors: &'a Option<HashMap<String, String>>,
    button_templates: &'a Option<HashMap<String, Button>>,
    image_dir: Option<String>,
    current_page_ref: RefCell<usize>,
    button_images: RefCell<Vec<String>>,
    button_backgrounds: RefCell<Vec<String>>,
    active_events: Arc<AtomicBool>,
    last_active_page: RefCell<Option<String>>,
    current_class: RefCell<String>,
    current_title: RefCell<String>,
}

impl<'a> PagedDevice<'a> {
    pub fn new(pages: &'a Pages,
               image_dir: Option<String>,
               colors: &'a Option<HashMap<String, String>>,
               button_templates: &'a Option<HashMap<String, Button>>,
               device: KeyDeckDevice,
               tx: &Sender<DeviceEvent>) -> Self {
        let button_count = { device.get_button_count() as usize };
        let current_page = match &pages.main_page {
            Some(page_name) => pages.pages.get_index_of(page_name).unwrap_or(100),
            None => 0,
        };
        let active_events = Arc::new(AtomicBool::new(true));
        button_listener(&device.serial, tx, &active_events);
        device.clear_all_button_images().unwrap_or_else(|e| { error_log!("Error while clearing button images: {}", e) });
        device.set_brightness(50).unwrap_or_else(|e| { error_log!("Error while setting brightness: {}", e) });
        let paged_device = PagedDevice {
            device,
            pages,
            colors,
            button_templates,
            image_dir,
            current_page_ref: RefCell::new(current_page),
            button_images: RefCell::new(vec![String::new(); button_count]),
            button_backgrounds: RefCell::new(vec![String::new(); button_count]),
            active_events,
            last_active_page: RefCell::new(None),
            current_class: RefCell::new(String::new()),
            current_title: RefCell::new(String::new()),
        };
        paged_device.refresh_page();
        paged_device
    }

    pub fn keep_alive(&self) {
        self.device.keep_alive();
    }

    pub fn disable(&self) {
        self.active_events.store(false, std::sync::atomic::Ordering::Relaxed);
    }

    pub fn terminate(&self) {
        self.disable();
        self.device.shutdown().unwrap_or_else(|e| { error_log!("Error while shutting down device: {}", e) });
    }

    pub fn button_down(&self, _button_id: u8) {}

    pub fn button_up(&self, button_id: u8) {
        let current_page = { self.current_page_ref.borrow().clone() };
        if let Some(button) = self.find_button(current_page, button_id) {
            if let Some(actions) = &button.actions {
                let mut still_active = true;
                for action in actions {
                    match action {
                        Action::Exec { exec } => {
                            std::process::Command::new("bash").arg("-c").arg(exec).spawn().expect("Failed to execute command");
                        }
                        Action::Jump { jump } => {
                            if let Err(e) = self.set_page(jump, false) {
                                still_active = false;
                                error_log!("{}", e);
                            }
                        }
                        Action::AutoJump { autojump: _ } => {
                            let class = { self.current_class.borrow().clone() };
                            let title = { self.current_title.borrow().clone() };
                            self.focus_changed(&class, &title, true)
                        }
                        Action::Focus { focus } => {
                            if let Err(e) = set_focus(focus, &"".to_string()) {
                                still_active = false;
                                error_log!("{}", e);
                            }
                        }
                        Action::Wait { wait } => {
                            thread::sleep(Duration::from_millis((wait * 1000.0) as u64));
                        }
                        Action::Key { key } => {
                            send_key_combination(key).unwrap_or_else(|e| { error_log!("{}", e) });
                        }
                    }
                    if !still_active {
                        break;
                    }
                }
            }
        }
    }

    pub fn encoder_down(&self, encoder_id: u8) {
        verbose_log!("Encoder down: {}", encoder_id);
    }

    pub fn encoder_up(&self, encoder_id: u8) {
        verbose_log!("Encoder up: {}", encoder_id);
    }

    pub fn encoder_twist(&self, encoder_id: u8, value: i8) {
        verbose_log!("Encoder twist: {} {}", encoder_id, value);
    }

    pub fn touch_point_down(&self, point_id: u8) {
        verbose_log!("Touch point down: {}", point_id);
    }

    pub fn touch_point_up(&self, point_id: u8) {
        verbose_log!("Touch point up: {}", point_id);
    }

    pub fn touch_screen_press(&self, x: u16, y: u16) {
        verbose_log!("Touch screen press: {} {}", x, y);
    }

    pub fn touch_screen_long_press(&self, x: u16, y: u16) {
        verbose_log!("Touch screen long press: {} {}", x, y);
    }

    pub fn touch_screen_swipe(&self, from: (u16, u16), to: (u16, u16)) {
        verbose_log!("Touch screen swipe: {:?} {:?}", from, to);
    }

    pub fn focus_changed(&self, class: &str, title: &str, force_change: bool) {
        {
            self.current_class.replace(class.to_string());
            self.current_title.replace(title.to_string());
        }
        if class.is_empty() && title.is_empty() {
            return;
        }
        if !force_change {
            let old_page = { self.current_page_ref.borrow().clone() };
            if let Some(lock) = self.pages.pages.get_index(old_page).unwrap().1.lock {
                if lock {
                    return;
                }
            }
        }
        for (name, page) in &self.pages.pages {
            if let Some(class_pattern) = &page.window_class {
                if class.contains(class_pattern) {
                    if let Err(error) = self.set_page(name, true) {
                        error_log!("{}", error);
                    }
                    return;
                }
            }
            if let Some(title_pattern) = &page.window_title {
                if title.contains(title_pattern) {
                    if let Err(error) = self.set_page(name, true) {
                        error_log!("{}", error);
                    }
                    return;
                }
            }
        }
        // Roll back last page if no application matches
        let last_active_page = { self.last_active_page.borrow().clone() };
        if let Some(last_active_page) = last_active_page {
            match self.pages.restore_mode {
                FocusChangeRestorePolicy::Last => if let Err(e) = self.set_page(&last_active_page, false) {
                    error_log!("{}", e);
                },
                FocusChangeRestorePolicy::Main => if let Err(e) = self.set_page(self.pages.main_page.as_ref().unwrap(), false) {
                    error_log!("{}", e);
                },
                FocusChangeRestorePolicy::Keep => {}
            }
            self.last_active_page.take();
        } else {
            if force_change {
                let main_page = match &self.pages.main_page {
                    Some(page_name) => page_name,
                    None => self.pages.pages.get_index(0).unwrap().0,
                };
                self.set_page(main_page, false).unwrap_or_else(|e| { error_log!("{}", e) });
            }
        }
    }

    fn update_image(&self, image: &str, image_path: Option<String>, background: Option<String>, button_index: u8, invalid_indices: &mut Vec<u8>) {
        let image_exists = if image.len() > 0 { find_path(image, image_path) } else { Some(image.to_string()) };
        let image = if let Some(image) = image_exists { image } else {
            error_log!("Image not found: {}", image);
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
            invalid_indices.push(button_index);
            return;
        }

        let image_data = if let Ok(image_data) = open(&image) { image_data } else {
            error_log!("Error while opening image: {}", image);
            return;
        };
        if bg_color.len() != 0 {
            match string_to_color(bg_color, &self.colors) {
                Ok((r, g, b, a)) => {
                    let bg_color = Rgba([r, g, b, a]);
                    let mut background = RgbaImage::from_pixel(image_data.width(), image_data.height(), bg_color);
                    overlay(&mut background, &image_data, 0, 0);
                    self.device
                        .set_button_image(button_index - 1, DynamicImage::from(background))
                        .unwrap_or_else(|e| error_log!("Error while setting button image: {}", e));
                }
                Err(e) => {
                    error_log!("{}", e);
                    self.device
                        .set_button_image(button_index - 1, image_data)
                        .unwrap_or_else(|e| error_log!("Error while setting button image: {}", e));
                }
            }
        } else {
            self.device.set_button_image(button_index - 1, image_data).unwrap_or_else(|e| { error_log!("Error while setting button image: {}", e) });
        }
    }

    fn refresh_page(&self) {
        let button_count = self.device.get_button_count();
        let current_page = { self.current_page_ref.borrow().clone() };
        let mut invalid_indices = Vec::new();
        for button_index in 1..=button_count {
            if let Some(button) = self.find_button(current_page, button_index).as_ref() {
                if let Some(icon) = &button.icon {
                    self.update_image(icon, self.image_dir.clone(), button.background.clone(), button_index, &mut invalid_indices);
                } else {
                    self.update_image("", None, button.background.clone(), button_index, &mut invalid_indices);
                }
            } else {
                self.update_image("", None, None, button_index, &mut invalid_indices);
            }
        }
        self.device.flush().unwrap_or_else(|e| { error_log!("Error while flushing device: {}", e) });
        // Process all invalid button indices
        for &button_index in &invalid_indices {
            self.device.clear_button_image(button_index - 1).unwrap_or_else(|e| {
                error_log!("Error while clearing button image: {}", e);
            });
        }
    }

    fn set_page(&self, page_name: &String, is_auto: bool) -> Result<(), String> {
        let page = self.pages.pages.get_index_of(page_name);
        if let Some(page) = page {
            let old_page = { self.current_page_ref.borrow_mut().clone() };
            if page != old_page {
                verbose_log!("Setting page to {}", page_name);
                if is_auto {
                    if self.last_active_page.borrow().is_none() {
                        // only if the page that the old_page refers to is not locked, update the active page
                        if self.pages.pages.get_index(old_page).map_or(true, |(_, target_page)| !target_page.lock.unwrap_or(false)) {
                            self.last_active_page.replace(Some(self.pages.pages.get_index(old_page).unwrap().0.clone()));
                        }
                    }
                } else {
                    if self.pages.pages.get_index(page).map_or(true, |(_, target_page)| !target_page.lock.unwrap_or(false)) {
                        self.last_active_page.take();
                    }
                }
                self.current_page_ref.replace(page);
                self.refresh_page();
            }
            Ok(())
        } else {
            Err(format!("Page not found: {}", page_name))
        }
    }

    fn find_page(&self, page_id: usize) -> &Page {
        let (_, page) = self.pages.pages.get_index(page_id).unwrap_or_else(|| {
            error_log!("Page not found: {}", page_id);
            std::process::exit(1);
        });
        page
    }

    fn find_button(&self, page_id: usize, button_id: u8) -> Option<&Button> {
        let key = format!("button{}", button_id); // Generate the key based on button_id
        if let Some(bc) = self.find_page(page_id).buttons.get(&key) {
            match bc {
                ButtonConfig::Template(template) => {
                    match self.button_templates.as_ref()?.get(template) {
                        Some(button) => Some(button),
                        None => {
                            println!("Warning: Button template '{}' not found", template);
                            None
                        }
                    }
                }
                ButtonConfig::Detailed(bc) => Some(bc),
            }
        } else {
            None
        }
    }
}

fn string_to_color(color: &str, named_colors: &Option<HashMap<String, String>>) -> Result<(u8, u8, u8, u8), String> {
    if (color.len() == 8 || color.len() == 10) && color.starts_with("0x") {
        let r = u8::from_str_radix(&color[2..4], 16).map_err(|_| format!("Invalid color format: {}", color))?;
        let g = u8::from_str_radix(&color[4..6], 16).map_err(|_| format!("Invalid color format: {}", color))?;
        let b = u8::from_str_radix(&color[6..8], 16).map_err(|_| format!("Invalid color format: {}", color))?;
        let a = if color.len() == 10 { u8::from_str_radix(&color[8..10], 16).map_err(|_| format!("Invalid color format: {}", color))? } else { 255 };
        Ok((r, g, b, a))
    } else {
        if let Some(idx_named_colors) = named_colors {
            if let Some(idx_color) = idx_named_colors.get(color) {
                return string_to_color(idx_color, named_colors);
            }
        }
        Err(format!("Unable to find named color '{}'", color))
    }
}