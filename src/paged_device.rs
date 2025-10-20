use crate::listener_button::button_listener;
use crate::listener_time::TimeManager;
use crate::device_manager::{find_path, KeyDeckDevice};
use crate::event::{DeviceEvent, WaitEventType};
use crate::focus_property::set_focus;
use crate::keyboard::{send_key_combination, send_string};
use crate::pages::{Action, Button, ButtonConfig, FocusChangeRestorePolicy, MacroCall, Page, Pages, TextConfig};
use crate::text_renderer;
use crate::{error_log, verbose_log};
use image::imageops::overlay;
use image::{open, DynamicImage, Rgba, RgbaImage};
use std::cell::RefCell;
use std::collections::HashMap;
use std::sync::atomic::AtomicBool;
use std::sync::mpsc::Sender;
use std::sync::Arc;
use std::time::{Duration, Instant};

/// Represents a queue of actions waiting to be executed after an event occurs.
/// Created when a WaitFor action is executed, and resumed when the corresponding event arrives.
struct PendingActionQueue {
    /// Remaining actions to execute after the event
    actions: Vec<Action>,
    /// Timestamp when this queue was last modified, used for timeout detection
    last_modified: Instant,
    /// Maximum time to wait for the event before dropping the queue
    timeout: Duration,
    /// The event type we're waiting for
    event_type: WaitEventType,
}

pub struct PagedDevice<'a> {
    device: KeyDeckDevice,
    pages: &'a Pages,
    colors: &'a Option<HashMap<String, String>>,
    button_templates: &'a Option<HashMap<String, Button>>,
    macros: &'a Option<HashMap<String, crate::pages::Macro>>,
    image_dir: Option<String>,
    current_page_ref: RefCell<usize>,
    button_images: RefCell<Vec<String>>,
    button_backgrounds: RefCell<Vec<String>>,
    active_events: Arc<AtomicBool>,
    last_active_page: RefCell<Option<String>>,
    current_class: RefCell<String>,
    current_title: RefCell<String>,
    pending_actions: RefCell<Option<PendingActionQueue>>,
    time_manager: &'a TimeManager,
}

impl<'a> PagedDevice<'a> {
    pub fn new(pages: &'a Pages,
               image_dir: Option<String>,
               colors: &'a Option<HashMap<String, String>>,
               button_templates: &'a Option<HashMap<String, Button>>,
               macros: &'a Option<HashMap<String, crate::pages::Macro>>,
               device: KeyDeckDevice,
               tx: &Sender<DeviceEvent>,
               time_manager: &'a TimeManager) -> Self {
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
            macros,
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
            time_manager,
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

    pub fn handle_tick(&self) {
        let current_page = { self.current_page_ref.borrow().clone() };
        let page = self.find_page(current_page);

        if let Some(actions) = &page.on_tick {
            if let Err(e) = self.execute_actions(actions.clone()) {
                error_log!("Error executing tick actions: {}", e);
            }
        }
    }

    pub fn disable(&self) {
        self.active_events.store(false, std::sync::atomic::Ordering::Relaxed);
    }

    pub fn terminate(&self) {
        self.disable();
        self.device.shutdown().unwrap_or_else(|e| { error_log!("Error while shutting down device: {}", e) });
    }

    /// Check if there are pending actions waiting for a specific event.
    /// If event type matches, resume action execution. Returns true if event was consumed.
    pub fn check_pending_event(&self, event_type: &WaitEventType) -> bool {
        // Take the pending actions if any exist
        let pending = {
            self.pending_actions.borrow_mut().take()
        }; // Borrow ends here

        if let Some(pending) = pending {
            // Check timeout
            if pending.last_modified.elapsed() > pending.timeout {
                verbose_log!("Pending action queue timed out for event '{}'", pending.event_type.as_str());
                return false;
            }

            // Check if event type matches
            if &pending.event_type != event_type {
                // Different event type, put queue back
                *self.pending_actions.borrow_mut() = Some(pending);
                return false;
            }

            // Event type matches, resume actions
            verbose_log!("WaitFor condition met for event '{}', resuming actions", event_type.as_str());
            if let Err(e) = self.execute_actions(pending.actions) {
                error_log!("{}", e);
            }
            return true;
        }
        false
    }

    /// Cancels any pending action queue.
    /// This is called when user interacts with the device (button press, encoder twist, etc.)
    /// to clear any actions that were waiting for events. Provides a central location
    /// for future conditional logic if needed.
    fn cancel_pending_actions(&self) {
        if let Some(pending) = self.pending_actions.borrow_mut().take() {
            verbose_log!("Canceling pending actions that were waiting for event '{}'",
                       pending.event_type.as_str());
        }
    }

    pub fn button_down(&self, _button_id: u8) {}

    pub fn button_up(&self, button_id: u8) {
        self.cancel_pending_actions();
        let current_page = { self.current_page_ref.borrow().clone() };
        if let Some(button) = self.find_button(current_page, button_id) {
            if let Some(actions) = &button.actions {
                if let Err(e) = self.execute_actions(actions.clone()) {
                    error_log!("{}", e);
                }
            }
        }
    }

    /// Recursively substitutes ${param} placeholders in a YAML Value with provided parameters.
    fn substitute_in_value(value: &mut serde_yaml_ng::Value, params: &HashMap<String, String>) {
        match value {
            serde_yaml_ng::Value::String(s) => {
                // Replace all ${param} patterns in the string
                for (key, val) in params {
                    let pattern = format!("${{{}}}", key);
                    *s = s.replace(&pattern, val);
                }
            }
            serde_yaml_ng::Value::Sequence(seq) => {
                for item in seq {
                    Self::substitute_in_value(item, params);
                }
            }
            serde_yaml_ng::Value::Mapping(map) => {
                for (_, v) in map {
                    Self::substitute_in_value(v, params);
                }
            }
            _ => {}
        }
    }

    /// Expands a single macro call into a sequence of actions.
    /// This performs parameter substitution and parses the macro's actions.
    fn expand_single_macro(&self, macro_call: MacroCall) -> Result<Vec<Action>, String> {
        // Extract macro name and provided parameters
        let macro_name = macro_call.name;
        let provided_params = macro_call.params;

        // Find the macro definition
        let macros = self.macros.as_ref()
            .ok_or_else(|| format!("No macros defined"))?;
        let macro_def = macros.get(&macro_name)
            .ok_or_else(|| format!("Macro '{}' not found", macro_name))?;

        // Merge provided params with default params (provided params override defaults)
        let mut final_params = macro_def.params.clone().unwrap_or_default();
        for (key, value) in provided_params {
            final_params.insert(key, value);
        }

        // Clone the macro's actions Value for substitution
        let mut actions_value = macro_def.actions.clone();

        // Substitute parameters in the YAML value
        Self::substitute_in_value(&mut actions_value, &final_params);

        // Parse the substituted YAML into Vec<Action>
        let actions: Vec<Action> = serde_yaml_ng::from_value(actions_value)
            .map_err(|e| format!("Failed to parse macro '{}' actions after parameter substitution: {}", macro_name, e))?;

        verbose_log!("Expanded macro '{}' with {} actions", macro_name, actions.len());
        Ok(actions)
    }

    /// Execute a sequence of actions. Returns when actions are complete, or pauses
    /// when a waitFor action needs to wait for an event to occur.
    /// Returns Ok(()) if all actions succeed, Err(message) on failure.
    fn execute_actions(&self, actions: Vec<Action>) -> Result<(), String> {
        let mut actions_iter = actions.into_iter();

        while let Some(action) = actions_iter.next() {
            match action {
                Action::Exec { exec, wait } => {
                    if wait.unwrap_or(false) {
                        // Synchronous: wait for command to complete and check exit status
                        let output = std::process::Command::new("bash")
                            .arg("-c")
                            .arg(&exec)
                            .output()
                            .map_err(|e| format!("Failed to execute command '{}': {}", exec, e))?;

                        if !output.status.success() {
                            let stderr = String::from_utf8_lossy(&output.stderr);
                            let exit_code = output.status.code().map_or("unknown".to_string(), |c| c.to_string());
                            return Err(format!(
                                "Command '{}' failed with exit code {}: {}",
                                exec, exit_code, stderr.trim()
                            ));
                        }
                    } else {
                        // Asynchronous: fire and forget (original behavior)
                        std::process::Command::new("bash")
                            .arg("-c")
                            .arg(&exec)
                            .spawn()
                            .map_err(|e| format!("Failed to execute command '{}': {}", exec, e))?;
                    }
                }
                Action::Jump { jump } => {
                    self.set_page(&jump, false)?;
                }
                Action::AutoJump { autojump: _ } => {
                    let class = { self.current_class.borrow().clone() };
                    let title = { self.current_title.borrow().clone() };
                    self.focus_changed(&class, &title, true)
                }
                Action::Focus { focus } => {
                    // Simple focus action: just request focus, no verification
                    set_focus(&focus, &"".to_string())?;
                    verbose_log!("Requested focus for '{}'", focus);
                }
                Action::WaitFor { wait_for } => {
                    let event_type = WaitEventType::from_str(wait_for.event())?;

                    // Pause and wait for the event to occur
                    let remaining: Vec<Action> = actions_iter.collect();

                    *self.pending_actions.borrow_mut() = Some(PendingActionQueue {
                        actions: remaining,
                        last_modified: Instant::now(),
                        timeout: Duration::from_secs_f64(wait_for.timeout()),
                        event_type: event_type.clone(),
                    });

                    verbose_log!("WaitFor paused, waiting for event '{}' (timeout: {}s)",
                               event_type.as_str(), wait_for.timeout());
                    return Ok(()); // Pause execution, will resume when event arrives
                }
                Action::Wait { wait } => {
                    // Schedule an async timer event instead of blocking
                    self.time_manager.schedule_timer(
                        self.device.serial.clone(),
                        Duration::from_secs_f64(wait as f64)
                    );

                    // Pause and wait for the TimerComplete event
                    let remaining: Vec<Action> = actions_iter.collect();

                    *self.pending_actions.borrow_mut() = Some(PendingActionQueue {
                        actions: remaining,
                        last_modified: Instant::now(),
                        timeout: Duration::from_secs_f64((wait as f64) * 2.0), // Generous timeout
                        event_type: WaitEventType::Timer,
                    });

                    verbose_log!("Wait scheduled for {}s (non-blocking)", wait);
                    return Ok(()); // Non-blocking return, will resume when TimerComplete arrives
                }
                Action::Key { key } => {
                    send_key_combination(&key)?;
                }
                Action::Text { text } => {
                    send_string(&text)?;
                }
                Action::Try { try_actions, else_actions } => {
                    // Execute try block
                    let try_result = self.execute_actions(try_actions);

                    if try_result.is_err() {
                        // Try block failed
                        if let Some(error_msg) = try_result.as_ref().err() {
                            verbose_log!("Try block failed: {}", error_msg);
                        }

                        if let Some(else_acts) = else_actions {
                            verbose_log!("Executing else block");
                            // Execute else block - errors propagate
                            self.execute_actions(else_acts)?;
                        } else {
                            // No else block, swallow error and continue
                            verbose_log!("No else block, continuing");
                        }
                    }
                    // If try succeeded, skip else block and continue
                }
                Action::Macro(macro_call) => {
                    // Expand this macro only
                    let expanded_actions = self.expand_single_macro(macro_call.clone())?;

                    // Prepend expanded actions to remaining actions
                    let remaining: Vec<Action> = actions_iter.collect();
                    let mut new_queue = expanded_actions;
                    new_queue.extend(remaining);

                    // Recursively execute the new queue
                    return self.execute_actions(new_queue);
                }
                Action::Return { .. } => {
                    verbose_log!("Return action: stopping execution successfully");
                    return Ok(());
                }
                Action::Fail { .. } => {
                    verbose_log!("Fail action: stopping execution with error");
                    return Err("Fail action executed".to_string());
                }
                Action::And { and_actions } => {
                    // Execute all actions sequentially, short-circuit on first error
                    verbose_log!("AND: executing {} conditions", and_actions.len());
                    for action in and_actions {
                        self.execute_actions(vec![action])?;  // Propagate first error
                    }
                    verbose_log!("AND: all conditions succeeded");
                    // All succeeded, continue
                }
                Action::Or { or_actions } => {
                    // Execute actions sequentially until one succeeds
                    verbose_log!("OR: trying {} conditions", or_actions.len());
                    let mut last_error = None;
                    for (idx, action) in or_actions.into_iter().enumerate() {
                        match self.execute_actions(vec![action]) {
                            Ok(_) => {
                                verbose_log!("OR: condition {} succeeded", idx + 1);
                                return Ok(());  // First success, stop and succeed
                            }
                            Err(e) => {
                                verbose_log!("OR: condition {} failed: {}", idx + 1, e);
                                last_error = Some(e);  // Store error, try next
                            }
                        }
                    }
                    // All failed, return last error
                    return Err(last_error.unwrap_or_else(|| "All OR conditions failed".to_string()));
                }
                Action::Not { not_action } => {
                    // Invert the result of the action
                    verbose_log!("NOT: inverting action result");
                    match self.execute_actions(vec![*not_action]) {
                        Ok(_) => {
                            verbose_log!("NOT: action succeeded, inverting to failure");
                            return Err("NOT condition: action succeeded (inverted to failure)".to_string());
                        }
                        Err(e) => {
                            verbose_log!("NOT: action failed ({}), inverting to success", e);
                            // Action failed, NOT inverts to success, continue
                        }
                    }
                }
            }
        }
        Ok(())
    }

    pub fn encoder_down(&self, _encoder_id: u8) {
        self.cancel_pending_actions();
    }

    pub fn encoder_up(&self, _encoder_id: u8) {
        self.cancel_pending_actions();
    }

    pub fn encoder_twist(&self, _encoder_id: u8, _value: i8) {
        self.cancel_pending_actions();
    }

    pub fn touch_point_down(&self, _point_id: u8) {
        self.cancel_pending_actions();
    }

    pub fn touch_point_up(&self, _point_id: u8) {
        self.cancel_pending_actions();
    }

    pub fn touch_screen_press(&self, _x: u16, _y: u16) {
        self.cancel_pending_actions();
    }

    pub fn touch_screen_long_press(&self, _x: u16, _y: u16) {
        self.cancel_pending_actions();
    }

    pub fn touch_screen_swipe(&self, _from: (u16, u16), _to: (u16, u16)) {
        self.cancel_pending_actions();
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
            if let Some((_, page)) = self.pages.pages.get_index(old_page) {
                if page.lock.unwrap_or(false) {
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
                        Some(page_name) => Some(page_name),
                        None => self.pages.pages.get_index(0).map(|(name, _)| name),
                    };
                    if let Some(main_page) = main_page {
                        if let Err(e) = self.set_page(main_page, false) {
                            error_log!("{}", e);
                        }
                    } else {
                        error_log!("Cannot restore to main page: no pages available");
                    }
                }
                FocusChangeRestorePolicy::Keep => {}
            }
            self.last_active_page.take();
        } else {
            if force_change {
                let main_page = match &self.pages.main_page {
                    Some(page_name) => Some(page_name),
                    None => self.pages.pages.get_index(0).map(|(name, _)| name),
                };
                if let Some(main_page) = main_page {
                    self.set_page(main_page, false).unwrap_or_else(|e| { error_log!("{}", e) });
                } else {
                    error_log!("Cannot force change to main page: no pages available");
                }
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
                        if let Some((name, target_page)) = self.pages.pages.get_index(old_page) {
                            if !target_page.lock.unwrap_or(false) {
                                self.last_active_page.replace(Some(name.clone()));
                            }
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