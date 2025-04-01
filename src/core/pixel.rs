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

    // Convert pixel art to safe string for profile storage
    pub fn to_safe_string(&self) -> String {
        let mut result = String::with_capacity(171); // ⌈1024/6⌉ = 171
        let mut current_bits = 0u8;
        let mut bit_count = 0;

        // Iterate through all pixels
        for row in &self.pixels {
            for &pixel in row {
                // Add pixel value to current bit group
                current_bits = (current_bits << 1) | (pixel as u8);
                bit_count += 1;

                // When 6 bits are collected, convert to character
                if bit_count == 6 {
                    // Convert 6 bits to 0-63 and map to available ASCII range (32-126, excluding 34 and 92)
                    let char_code = match current_bits {
                        0..=33 => current_bits + 32,        // 32-65
                        34 => 35,                           // Skip double quotes (34)
                        35..=91 => current_bits + 33,       // 66-124
                        92 => 93,                           // Skip backslash (92)
                        _ => current_bits + 34              // 125-126
                    };
                    result.push(char_code as char);
                    current_bits = 0;
                    bit_count = 0;
                }
            }
        }

        // Process remaining bits (if any)
        if bit_count > 0 {
            // Shift to align to 6 bits
            current_bits <<= (6 - bit_count);
            let char_code = match current_bits {
                0..=33 => current_bits + 32,
                34 => 35,
                35..=91 => current_bits + 33,
                92 => 93,
                _ => current_bits + 34
            };
            result.push(char_code as char);
        }

        result
    }

    // Restore pixel art from string
    pub fn from_safe_string(s: &str) -> Option<Self> {
        if s.len() > 171 { return None; }  // Validate length

        let mut pixels = vec![vec![false; 32]; 32];
        let mut pixel_index = 0;

        for c in s.chars() {
            // Validate character range
            if !c.is_ascii() || c == '"' || c == '\\' { return None; }
            
            // Convert character back to 6-bit binary
            let char_code = c as u8;
            let bits = match char_code {
                32..=34 => char_code - 32,        // 32-34 -> 0-2
                35..=92 => char_code - 33,        // 35-92 -> 2-59
                93..=126 => char_code - 34,       // 93-126 -> 59-92
                _ => return None
            };

            // Parse 6 bits
            for bit_position in (0..6).rev() {
                if pixel_index >= 1024 { break; }
                let row = pixel_index / 32;
                let col = pixel_index % 32;
                pixels[row][col] = ((bits >> bit_position) & 1) == 1;
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_safe_string_conversion() {
        let mut pixel = Pixel::new();
        
        // Create a test pattern
        for i in 0..32 {
            for j in 0..32 {
                pixel.set_pixel(i, j, (i + j) % 2 == 0);
            }
        }

        // Convert to string and validate length
        let encoded = pixel.to_safe_string();
        println!("Encoded length: {}", encoded.len());
        assert!(encoded.len() <= 171);

        // Validate string contains no illegal characters
        assert!(!encoded.contains('"'));
        assert!(!encoded.contains('\\'));
        assert!(encoded.chars().all(|c| c.is_ascii() && c >= ' ' && c <= '~'));

        // Validate can be correctly restored
        let decoded = Pixel::from_safe_string(&encoded).unwrap();
        assert_eq!(pixel.pixels, decoded.pixels);
    }
} 