use crate::listener_button::button_listener;
use crate::device_manager::{find_path, KeyDeckDevice};
use crate::event::DeviceEvent;
use crate::focus_property::set_focus;
use crate::keyboard::{send_key_combination, send_string};
use crate::pages::{Action, Button, ButtonConfig, FocusChangeRestorePolicy, Page, Pages, TextConfig};
use crate::text_renderer;
use crate::{error_log, verbose_log};
use image::imageops::overlay;
use image::{open, DynamicImage, Rgba, RgbaImage};
use std::cell::RefCell;
use std::collections::HashMap;
use std::sync::atomic::AtomicBool;
use std::sync::mpsc::Sender;
use std::sync::Arc;
use std::thread;
use std::time::{Duration, Instant};

/// Represents a queue of actions waiting to be executed after a focus verification event.
/// Created when a verified focus action is executed, and resumed when the corresponding
/// FocusChanges event arrives.
struct PendingActionQueue {
    /// Remaining actions to execute, with VerifyFocus at the front
    actions: Vec<Action>,
    /// Timestamp when this queue was last modified, used for timeout detection
    last_modified: Instant,
    /// Maximum time to wait for the verification event before dropping the queue
    timeout: Duration,
}

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
    pending_actions: RefCell<Option<PendingActionQueue>>,
}

impl<'a> PagedDevice<'a> {
    pub fn new(pages: &'a Pages,
               image_dir: Option<String>,
               colors: &'a Option<HashMap<String, String>>,
               button_templates: &'a Option<HashMap<String, Button>>,
               device: KeyDeckDevice,
               tx: &Sender<DeviceEvent>) -> Self {
        let button_count = { device.get_button_count() as usize };
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
            // Initialize to sentinel value so first set_page() will trigger refresh
            current_page_ref: RefCell::new(usize::MAX),
            button_images: RefCell::new(vec![String::new(); button_count]),
            button_backgrounds: RefCell::new(vec![String::new(); button_count]),
            active_events,
            last_active_page: RefCell::new(None),
            current_class: RefCell::new(String::new()),
            current_title: RefCell::new(String::new()),
            pending_actions: RefCell::new(None),
        };

        // Set the initial page (will trigger refresh because current_page_ref is MAX)
        let main_page_name = match &pages.main_page {
            Some(name) => name.clone(),
            None => pages.pages.get_index(0).map(|(name, _)| name.clone()).unwrap_or_else(|| "".to_string()),
        };
        if !main_page_name.is_empty() {
            let _ = paged_device.set_page(&main_page_name, false);
        }

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
        // Clear any pending actions - new button press cancels waiting actions
        self.pending_actions.borrow_mut().take();

        let current_page = { self.current_page_ref.borrow().clone() };
        if let Some(button) = self.find_button(current_page, button_id) {
            if let Some(actions) = &button.actions {
                self.execute_actions(actions.clone());
            }
        }
    }

    /// Execute a sequence of actions. Returns when actions are complete, or pauses
    /// when a verified focus action needs to wait for a FocusChanges event.
    fn execute_actions(&self, actions: Vec<Action>) {
        let mut actions_iter = actions.into_iter();

        while let Some(action) = actions_iter.next() {
            match action {
                Action::Exec { exec } => {
                    std::process::Command::new("bash").arg("-c").arg(exec).spawn()
                        .expect("Failed to execute command");
                }
                Action::Jump { jump } => {
                    if let Err(e) = self.set_page(&jump, false) {
                        error_log!("{}", e);
                        return; // Abort sequence on error
                    }
                }
                Action::AutoJump { autojump: _ } => {
                    let class = { self.current_class.borrow().clone() };
                    let title = { self.current_title.borrow().clone() };
                    self.focus_changed(&class, &title, true)
                }
                Action::Focus { focus } => {
                    let target = focus.target();
                    let should_verify = focus.should_verify();

                    // Always check if we already have the correct focus
                    let current_class = { self.current_class.borrow().clone() };
                    let current_title = { self.current_title.borrow().clone() };
                    let target_lower = target.to_lowercase();
                    let already_focused = current_class.to_lowercase().contains(&target_lower) ||
                                        current_title.to_lowercase().contains(&target_lower);

                    if already_focused {
                        verbose_log!("Focus already on '{}', skipping focus request", target);
                        // Continue to next action without requesting focus
                    } else if should_verify {
                        // Not focused, need to request and verify
                        if let Err(e) = set_focus(&target.to_string(), &"".to_string()) {
                            error_log!("{}", e);
                            return; // Abort if focus fails
                        }

                        // Inject VerifyFocus action and pause queue
                        let mut remaining: Vec<Action> = vec![
                            Action::VerifyFocus { verify_focus: target.to_string() }
                        ];
                        remaining.extend(actions_iter);

                        // Store pending queue and wait for FocusChanges event
                        *self.pending_actions.borrow_mut() = Some(PendingActionQueue {
                            actions: remaining,
                            last_modified: Instant::now(),
                            timeout: Duration::from_secs_f64(focus.timeout()),
                        });

                        verbose_log!("Focus action paused, waiting for FocusChanges event for '{}'", target);
                        return; // Pause execution, will resume on FocusChanges
                    } else {
                        // Not focused, no verification: just request and continue
                        if let Err(e) = set_focus(&target.to_string(), &"".to_string()) {
                            error_log!("{}", e);
                            return; // Abort if focus fails
                        }
                    }
                }
                Action::VerifyFocus { verify_focus } => {
                    // Check current focus immediately
                    let current_class = { self.current_class.borrow().clone() };
                    let current_title = { self.current_title.borrow().clone() };

                    let target_lower = verify_focus.to_lowercase();
                    let matches = current_class.to_lowercase().contains(&target_lower) ||
                                 current_title.to_lowercase().contains(&target_lower);

                    if !matches {
                        error_log!("VerifyFocus failed: expected '{}' but current is class='{}' title='{}'",
                                  verify_focus, current_class, current_title);
                        return; // Abort sequence
                    }

                    verbose_log!("VerifyFocus succeeded for '{}'", verify_focus);
                }
                Action::Wait { wait } => {
                    thread::sleep(Duration::from_millis((wait * 1000.0) as u64));
                }
                Action::Key { key } => {
                    send_key_combination(&key).unwrap_or_else(|e| { error_log!("{}", e) });
                }
                Action::Text { text } => {
                    send_string(&text).unwrap_or_else(|e| { error_log!("{}", e) });
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

        // Check if we have pending actions waiting for focus verification
        if let Some(pending) = self.pending_actions.borrow_mut().take() {
            // Check timeout
            if pending.last_modified.elapsed() > pending.timeout {
                verbose_log!("Pending action queue timed out, dropping");
                return; // Don't process normal focus change
            }

            verbose_log!("Resuming pending actions after FocusChanges event");
            // Resume execution with the pending queue
            self.execute_actions(pending.actions);
            return; // Don't process normal focus change
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
                if class.to_lowercase().contains(&class_pattern.to_lowercase()) {
                    if let Err(error) = self.set_page(name, true) {
                        error_log!("{}", error);
                    }
                    return;
                }
            }
            if let Some(title_pattern) = &page.window_title {
                if title.to_lowercase().contains(&title_pattern.to_lowercase()) {
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
                FocusChangeRestorePolicy::Main => {
                    let main_page = match &self.pages.main_page {
                        Some(page_name) => page_name,
                        None => self.pages.pages.get_index(0).unwrap().0,
                    };
                    if let Err(e) = self.set_page(main_page, false) {
                        error_log!("{}", e);
                    }
                }
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

    fn update_button(&self, image: &str, image_path: Option<String>, background: Option<String>, text: Option<TextConfig>, outline: Option<String>, text_color: Option<String>, button_index: u8, invalid_indices: &mut Vec<u8>) {
        // Get the button size from the device's image format
        let (width, height) = {
            let (w, h) = self.device.get_deck().kind().key_image_format().size;
            (w as u32, h as u32)
        };

        // Determine if we're rendering text or using an icon
        let has_text = text.is_some();
        let text_str = if let Some(ref text_cfg) = text {
            match text_cfg {
                TextConfig::Simple(s) => s.clone(),
                TextConfig::Detailed { value, .. } => value.clone(),
            }
        } else {
            String::new()
        };

        // Find the icon path if provided
        let image_exists = if image.len() > 0 { find_path(image, image_path.clone()) } else { Some(image.to_string()) };
        let image_path = if let Some(image) = image_exists {
            verbose_log!("Found image path: {}", image);
            image
        } else {
            if image.len() > 0 {
                error_log!("Image not found: {}", image);
            }
            "".to_string()
        };

        let bg_color_str = if let Some(bg_color) = background.as_ref() { bg_color.as_str() } else { "" };

        // Create cache key including text information
        let cache_key = format!("{}:{}:{}", image_path, bg_color_str, text_str);

        {
            // Check if the button state is the same as the current one
            let mut button_images = self.button_images.borrow_mut();
            let mut button_backgrounds = self.button_backgrounds.borrow_mut();
            if button_images[button_index as usize - 1] == cache_key && button_backgrounds[button_index as usize - 1] == bg_color_str {
                // No need to update the button
                return;
            }
            // Update the cache key
            button_images[button_index as usize - 1] = cache_key;
            button_backgrounds[button_index as usize - 1] = bg_color_str.to_string();
        }

        // If we have text but no icon, render text directly
        if has_text && image_path.is_empty() {
            let font_size = if let Some(TextConfig::Detailed { fontsize, .. }) = text {
                fontsize
            } else {
                None
            };

            // Parse background color or default to black
            let bg_rgb = if let Some(ref bg) = background {
                match string_to_color(bg, &self.colors) {
                    Ok((r, g, b)) => Some([r, g, b]),
                    Err(_) => Some([0, 0, 0]),
                }
            } else {
                Some([0, 0, 0])
            };

            // Parse outline color if provided
            let outline_rgb = if let Some(ref outline_str) = outline {
                match string_to_color(outline_str, &self.colors) {
                    Ok((r, g, b)) => Some([r, g, b]),
                    Err(_) => None,
                }
            } else {
                None
            };

            // Parse text color if provided (defaults to white in renderer)
            let text_color_rgba = if let Some(ref color_str) = text_color {
                match string_to_color(color_str, &self.colors) {
                    Ok((r, g, b)) => Some(image::Rgba([r, g, b, 255u8])),
                    Err(_) => None,
                }
            } else {
                None
            };

            eprintln!("DEBUG: Rendering text '{}' on canvas size {}x{} (button dimensions)", text_str, width, height);
            match text_renderer::render_text(&text_str, font_size, width, height, bg_rgb, text_color_rgba, outline_rgb, None) {
                Ok(rendered_image) => {
                    eprintln!("DEBUG: Text rendered successfully, final image size {}x{}", rendered_image.width(), rendered_image.height());
                    self.device
                        .set_button_image(button_index - 1, rendered_image)
                        .unwrap_or_else(|e| error_log!("Error while setting button image: {}", e));
                }
                Err(e) => {
                    error_log!("Error rendering text: {}", e);
                    invalid_indices.push(button_index);
                }
            }
            return;
        }

        // If no icon and no text, mark as invalid
        if image_path.is_empty() {
            invalid_indices.push(button_index);
            return;
        }

        // Load the icon image
        let mut image_data = if let Ok(image_data) = open(&image_path) {
            image_data
        } else {
            error_log!("Error while opening image: {}", image_path);
            invalid_indices.push(button_index);
            return;
        };

        // Apply background color if provided
        if !bg_color_str.is_empty() {
            match string_to_color(bg_color_str, &self.colors) {
                Ok((r, g, b)) => {
                    let bg_color = Rgba([r, g, b, 255]);
                    let mut background = RgbaImage::from_pixel(image_data.width(), image_data.height(), bg_color);
                    overlay(&mut background, &image_data, 0, 0);
                    image_data = DynamicImage::from(background);
                }
                Err(e) => {
                    error_log!("{}", e);
                }
            }
        }

        // If we have both icon and text, render text on top of the image
        if has_text {
            verbose_log!("Overlaying text '{}' on image", text_str);
            let font_size = if let Some(TextConfig::Detailed { fontsize, .. }) = text {
                fontsize
            } else {
                None
            };

            // Parse outline color if provided
            let outline_rgb = if let Some(ref outline_str) = outline {
                match string_to_color(outline_str, &self.colors) {
                    Ok((r, g, b)) => Some([r, g, b]),
                    Err(_) => None,
                }
            } else {
                None
            };

            // Parse text color if provided (defaults to white in renderer)
            let text_color_rgba = if let Some(ref color_str) = text_color {
                match string_to_color(color_str, &self.colors) {
                    Ok((r, g, b)) => Some(image::Rgba([r, g, b, 255u8])),
                    Err(_) => None,
                }
            } else {
                None
            };

            // Render text with the image as background (button dimensions, image scaled to fit)
            match text_renderer::render_text(&text_str, font_size, width, height, background.as_ref().and_then(|bg| {
                match string_to_color(bg, &self.colors) {
                    Ok((r, g, b)) => Some([r, g, b]),
                    Err(_) => Some([0, 0, 0]),
                }
            }), text_color_rgba, outline_rgb, Some(&image_data)) {
                Ok(combined_image) => {
                    image_data = combined_image;
                }
                Err(e) => {
                    error_log!("Error rendering text with image: {}", e);
                }
            }
        }

        // Set the final button image
        self.device
            .set_button_image(button_index - 1, image_data)
            .unwrap_or_else(|e| error_log!("Error while setting button image: {}", e));
    }

    fn refresh_page(&self) {
        let button_count = self.device.get_button_count();
        let current_page = { self.current_page_ref.borrow().clone() };
        let mut invalid_indices = Vec::new();
        for button_index in 1..=button_count {
            if let Some(button) = self.find_button(current_page, button_index).as_ref() {
                if let Some(icon) = &button.icon {
                    self.update_button(icon, self.image_dir.clone(), button.background.clone(), button.text.clone(), button.outline.clone(), button.text_color.clone(), button_index, &mut invalid_indices);
                } else {
                    self.update_button("", None, button.background.clone(), button.text.clone(), button.outline.clone(), button.text_color.clone(), button_index, &mut invalid_indices);
                }
            } else {
                self.update_button("", None, None, None, None, None, button_index, &mut invalid_indices);
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

fn string_to_color(color: &str, named_colors: &Option<HashMap<String, String>>) -> Result<(u8, u8, u8), String> {
    if (color.len() == 8 || color.len() == 10) && color.starts_with("0x") {
        let offset = if color.len() == 10 { 2 } else { 0 };
        let a = if color.len() == 10 { u8::from_str_radix(&color[2..4], 16).map_err(|_| format!("Invalid color format: {}", color))? } else { 255 };
        let r = u8::from_str_radix(&color[offset + 2..offset + 4], 16).map_err(|_| format!("Invalid color format: {}", color))?;
        let g = u8::from_str_radix(&color[offset + 4..offset + 6], 16).map_err(|_| format!("Invalid color format: {}", color))?;
        let b = u8::from_str_radix(&color[offset + 6..offset + 8], 16).map_err(|_| format!("Invalid color format: {}", color))?;

        // Assuming the background color is 0,0,0
        let alpha = a as f32 / 255.0;
        let final_r = (r as f32 * alpha).round() as u8;
        let final_g = (g as f32 * alpha).round() as u8;
        let final_b = (b as f32 * alpha).round() as u8;
        Ok((final_r, final_g, final_b))
    } else {
        if let Some(idx_named_colors) = named_colors {
            if let Some(idx_color) = idx_named_colors.get(color) {
                return string_to_color(idx_color, named_colors);
            }
        }
        Err(format!("Unable to find named color '{}'", color))
    }
}