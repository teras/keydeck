use std::cell::RefCell;
use image::DynamicImage;

pub struct ButtonImage {
    image_name: Vec<String>,
    background_color: String,
    icon: RefCell<Option<DynamicImage>>,
}

impl ButtonImage {
    pub fn new(image_name: Vec<String>, background_color: String) -> Self {
        ButtonImage {
            image_name,
            background_color,
            icon: RefCell::new(None),
        }
    }

    pub fn equal(&self, image_name: Vec<String>, background_color: &str) -> bool {
        self.image_name == image_name && self.background_color == background_color
    }

    pub fn update_icon(&self, image: DynamicImage) {
        self.icon.replace(Some(image));
    }
}