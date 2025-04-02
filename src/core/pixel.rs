use flate2::Compression;
use flate2::write::DeflateEncoder;
use flate2::read::DeflateDecoder;
use base64::{encode, decode};
use std::io::prelude::*;

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

    // Helper function to map 6 bits to a safe ASCII character
    fn map_to_safe_char(bits: u8) -> char {
        // Start from ASCII 35 (after space, !, ", #)
        // Skip 58 (:) and 92 (\)
        // This gives us exactly 90 usable characters (35-126, excluding 58 and 92)
        let base = 35;
        let mut code = base + bits;
        
        if code >= 58 { code += 1; }  // Skip :
        if code >= 92 { code += 1; }  // Skip \
        
        code as char
    }

    // Convert pixel art to safe string for profile storage
    pub fn to_safe_string(&self) -> String {
        let mut result = String::with_capacity(171);
        let mut current_bits = 0u8;
        let mut bit_count = 0;

        for row in &self.pixels {
            for &pixel in row {
                current_bits = (current_bits << 1) | (pixel as u8);
                bit_count += 1;

                if bit_count == 6 {
                    result.push(Self::map_to_safe_char(current_bits));
                    current_bits = 0;
                    bit_count = 0;
                }
            }
        }

        if bit_count > 0 {
            current_bits <<= (6 - bit_count);
            result.push(Self::map_to_safe_char(current_bits));
        }

        result
    }

    // Restore pixel art from string
    pub fn from_safe_string(s: &str) -> Option<Self> {
        if s.len() > 171 { return None; }

        let mut pixels = vec![vec![false; 32]; 32];
        let mut pixel_index = 0;

        for c in s.chars() {
            // First verify character is valid
            if !c.is_ascii() || c == '"' || c == '\\' || c == ':' { 
                return None; 
            }
            
            // Reverse the mapping
            let char_code = c as u8;
            let bits = match char_code {
                32..=33 => char_code - 32,        // 0-1
                35..=57 => char_code - 33,        // 2-24
                60..=91 => char_code - 33,        // 27-58
                94..=126 => char_code - 34,       // 60-92
                _ => return None
            };

            // Process the 6 bits
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

    // convert to optimal string
    pub fn to_optimal_string(&self) -> String {
        let normal_string = self.to_safe_string();
        
        match self.compress_with_deflate(&normal_string) {
            Ok(compressed_str) => {
                if compressed_str.len() + 2 < normal_string.len() {
                    return format!("c:{}", compressed_str)
                }
            }
            Err(_) => {}
        }
        
        format!("n:{}", normal_string)
    }

    // restore from optimal string
    pub fn from_optimal_string(s: &str) -> Option<Self> {
        if s.len() < 2 {
            return None;
        }

        let (prefix, data) = s.split_once(':')?;
        
        match prefix {
            "c" => {
                match Self::decompress_with_deflate(data) {
                    Ok(decompressed) => Self::from_safe_string(&decompressed),
                    Err(_) => None
                }
            }
            "n" => Self::from_safe_string(data),
            _ => None
        }
    }

    // compress string
    fn compress_with_deflate(&self, input: &str) -> Result<String, String> {
        let mut encoder = DeflateEncoder::new(Vec::new(), Compression::best());
        encoder.write_all(input.as_bytes())
            .map_err(|e| format!("Compression error: {}", e))?;
        
        let compressed = encoder.finish()
            .map_err(|e| format!("Compression finish error: {}", e))?;
            
        Ok(encode(compressed))
    }

    // decompress string
    fn decompress_with_deflate(input: &str) -> Result<String, String> {
        let bytes = decode(input)
            .map_err(|e| format!("Base64 decode error: {}", e))?;
            
        let mut decoder = DeflateDecoder::new(&bytes[..]);
        let mut decompressed = String::new();
        
        decoder.read_to_string(&mut decompressed)
            .map_err(|e| format!("Decompression error: {}", e))?;
            
        Ok(decompressed)
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

    #[test]
    fn test_pixel_compression() {
        // Test 1: All black pattern
        let mut pixel = Pixel::new();
        for i in 0..32 {
            for j in 0..32 {
                pixel.set_pixel(i, j, true);
            }
        }
        
        let encoded = pixel.to_optimal_string();
        println!("All black - Original string length: {}", pixel.to_safe_string().len());
        println!("All black - Compressed length: {}", encoded.len());
        println!("All black - Compression type: {}", if encoded.starts_with("c:") { "compressed" } else { "uncompressed" });
        
        let decoded = Pixel::from_optimal_string(&encoded).unwrap();
        assert_eq!(pixel.pixels, decoded.pixels, "All black pattern test failed");

        // Test 2: Checkerboard pattern
        let mut pixel = Pixel::new();
        for i in 0..32 {
            for j in 0..32 {
                pixel.set_pixel(i, j, (i + j) % 2 == 0);
            }
        }
        
        let encoded = pixel.to_optimal_string();
        println!("\nCheckerboard - Original string length: {}", pixel.to_safe_string().len());
        println!("Checkerboard - Compressed length: {}", encoded.len());
        println!("Checkerboard - Compression type: {}", if encoded.starts_with("c:") { "compressed" } else { "uncompressed" });
        
        let decoded = Pixel::from_optimal_string(&encoded).unwrap();
        assert_eq!(pixel.pixels, decoded.pixels, "Checkerboard pattern test failed");

        // Test 3: Empty (all white) pattern
        let pixel = Pixel::new();
        let encoded = pixel.to_optimal_string();
        println!("\nEmpty pattern - Original string length: {}", pixel.to_safe_string().len());
        println!("Empty pattern - Compressed length: {}", encoded.len());
        println!("Empty pattern - Compression type: {}", if encoded.starts_with("c:") { "compressed" } else { "uncompressed" });
        
        let decoded = Pixel::from_optimal_string(&encoded).unwrap();
        assert_eq!(pixel.pixels, decoded.pixels, "Empty pattern test failed");
    }

    #[test]
    fn test_invalid_input() {
        // Test invalid prefix
        let result = Pixel::from_optimal_string("x:invalid");
        assert!(result.is_none(), "Should reject invalid prefix");

        // Test empty string
        let result = Pixel::from_optimal_string("");
        assert!(result.is_none(), "Should reject empty string");

        // Test invalid compressed data
        let result = Pixel::from_optimal_string("c:invalid_base64");
        assert!(result.is_none(), "Should reject invalid compressed data");

        // Test invalid uncompressed data (contains ':')
        let result = Pixel::from_optimal_string("n:test:colon");
        assert!(result.is_none(), "Should reject string containing colon");
    }

    #[test]
    fn test_compression_efficiency() {
        // Create a highly compressible pattern (all black)
        let mut pixel = Pixel::new();
        for i in 0..32 {
            for j in 0..32 {
                pixel.set_pixel(i, j, true);
            }
        }
        
        let normal_string = pixel.to_safe_string();
        let optimal_string = pixel.to_optimal_string();
        
        println!("Compression efficiency test:");
        println!("Original size: {}", normal_string.len());
        println!("Optimal size: {}", optimal_string.len());
        println!("Compression ratio: {:.2}%", 
            (optimal_string.len() as f64 / normal_string.len() as f64) * 100.0);
        
        // Verify the compressed version is actually smaller
        assert!(optimal_string.starts_with("c:"), 
            "Highly repetitive pattern should use compression");
        assert!(optimal_string.len() < normal_string.len(), 
            "Compressed version should be smaller than original");
    }

    #[test]
    fn print_pixel_to_ascii_mapping() {
        println!("Bits | Dec | ASCII | Code");
        println!("--------------------|-----");
        
        // Test all 64 possible 6-bit values
        for i in 0..64 {
            let c = Pixel::map_to_safe_char(i);
            println!("{:06b} | {:3} | {} | {}", i, i, c, c as u8);
            
            // Verify the character is valid
            assert!(c as u8 >= 35, "Character code too low");
            assert!(c as u8 <= 126, "Character code too high");
            assert_ne!(c as u8, 58, "Found colon");
            assert_ne!(c as u8, 92, "Found backslash");
        }
    }
} 