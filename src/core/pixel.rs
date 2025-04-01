use image::{ImageBuffer, Luma};

#[derive(Clone)]
pub struct Pixel {
    pixels: Vec<Vec<bool>>, // true represents black, false represents white
}

impl Pixel {
    pub fn new() -> Self {
        Self {
            pixels: vec![vec![false; 32]; 32]
        }
    }

    // Get pixel state
    pub fn get_pixel(&self, row: usize, col: usize) -> bool {
        self.pixels[row][col]
    }

    // Set pixel state
    pub fn set_pixel(&mut self, row: usize, col: usize, value: bool) {
        self.pixels[row][col] = value;
    }

    // Toggle pixel state
    pub fn toggle_pixel(&mut self, row: usize, col: usize) {
        self.pixels[row][col] = !self.pixels[row][col];
    }

    // Convert pixel art to hex string for profile storage
    pub fn to_hex_string(&self) -> String {
        let mut binary_string = String::with_capacity(1024);
        
        // Convert 2D array to binary string
        for row in &self.pixels {
            for &pixel in row {
                binary_string.push(if pixel { '1' } else { '0' });
            }
        }
        
        // Convert binary string to hex string
        let mut hex_string = String::with_capacity(256);
        for chunk in binary_string.as_bytes().chunks(4) {
            let mut value = 0u8;
            for (i, &bit) in chunk.iter().enumerate() {
                if bit == b'1' {
                    value |= 1 << (3 - i);
                }
            }
            hex_string.push_str(&format!("{:X}", value));
        }
        
        hex_string
    }

    // Create from hex string
    pub fn from_hex_string(hex_string: &str) -> Option<Self> {
        if hex_string.len() != 256 || !hex_string.chars().all(|c| c.is_ascii_hexdigit()) {
            return None;
        }

        let mut pixels = vec![vec![false; 32]; 32];
        let mut pixel_index = 0;

        for hex_char in hex_string.chars() {
            let value = hex_char.to_digit(16)?;
            let binary = format!("{:04b}", value);
            
            for bit in binary.chars() {
                let row = pixel_index / 32;
                let col = pixel_index % 32;
                pixels[row][col] = bit == '1';
                pixel_index += 1;
            }
        }

        Some(Self { pixels })
    }

    // Process image data into pixel art
    pub fn from_image_data(data: &[u8]) -> Result<Self, String> {
        // Load image from bytes
        let img = image::load_from_memory(data)
            .map_err(|e| format!("Failed to load image: {}", e))?;
        
        // Resize to 32x32
        let resized = img.resize_exact(32, 32, image::imageops::FilterType::Lanczos3);
        
        // Convert to grayscale
        let gray = resized.into_luma8();
        
        // Convert to black and white using threshold
        let threshold = 128u8;
        let mut pixel_art = Self::new();
        
        for (x, y, pixel) in gray.enumerate_pixels() {
            pixel_art.pixels[y as usize][x as usize] = pixel[0] < threshold;
        }
        
        Ok(pixel_art)
    }

    // Get dimensions
    pub fn dimensions(&self) -> (usize, usize) {
        (32, 32)
    }

    // Clear all pixels
    pub fn clear(&mut self) {
        for row in self.pixels.iter_mut() {
            for pixel in row.iter_mut() {
                *pixel = false;
            }
        }
    }

    pub fn set_pixels_from_image(&mut self, x: usize, y: usize, is_black: bool) {
        self.pixels[y][x] = is_black;
    }
}

// Add default implementation
impl Default for Pixel {
    fn default() -> Self {
        Self::new()
    }
} 