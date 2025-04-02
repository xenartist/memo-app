use flate2::Compression;
use flate2::write::DeflateEncoder;
use flate2::read::DeflateDecoder;
use base64::{encode, decode};
use std::io::prelude::*;

#[derive(Debug, PartialEq, Clone)]
pub struct Pixel {
    width: usize,
    height: usize,
    data: Vec<bool>,
}

impl Pixel {
    // default create 32x32 pixel art
    pub fn new() -> Self {
        Self::with_size(32, 32)
    }

    // original new method renamed to with_size
    pub fn with_size(width: usize, height: usize) -> Self {
        Self {
            width,
            height,
            data: vec![false; width * height],
        }
    }

    pub fn set(&mut self, x: usize, y: usize, value: bool) {
        if x < self.width && y < self.height {
            self.data[y * self.width + x] = value;
        }
    }

    pub fn get(&self, x: usize, y: usize) -> bool {
        if x < self.width && y < self.height {
            self.data[y * self.width + x]
        } else {
            false
        }
    }

    // Get pixel state
    pub fn get_pixel(&self, row: usize, col: usize) -> bool {
        self.data[row * self.width + col]
    }

    // Set pixel state
    pub fn set_pixel(&mut self, row: usize, col: usize, value: bool) {
        self.data[row * self.width + col] = value;
    }

    // Toggle pixel state
    pub fn toggle_pixel(&mut self, row: usize, col: usize) {
        self.data[row * self.width + col] = !self.data[row * self.width + col];
    }

    // Helper function to map 6 bits to a safe ASCII character
    fn map_to_safe_char(value: u8) -> char {
        assert!(value < 64, "Value must be less than 64");
        let mut ascii = 35 + value;  // start from ASCII 35
        
        // skip ':' and '\'
        if ascii >= 58 { ascii += 1; }  // skip ':'
        if ascii >= 92 { ascii += 1; }  // skip '\'
        
        ascii as char
    }

    fn map_from_safe_char(c: char) -> Option<u8> {
        let ascii = c as u8;
        
        // check special characters
        if c == ':' || c == '\\' || c == '"' {
            return None;
        }
        
        // check range
        if ascii < 35 || ascii > 126 {
            return None;
        }
        
        let mut value = ascii - 35;
        if ascii > 92 { value -= 1; }  // adjust '\'
        if ascii > 58 { value -= 1; }  // adjust ':'
        
        if value >= 64 {
            return None;
        }
        
        Some(value)
    }

    // Convert pixel art to safe string for profile storage
    pub fn to_safe_string(&self) -> String {
        let mut result = String::with_capacity(171);
        let mut current_bits = 0u8;
        let mut bit_count = 0;

        for &pixel in &self.data {
            current_bits = (current_bits << 1) | (pixel as u8);
            bit_count += 1;

            if bit_count == 6 {
                result.push(Self::map_to_safe_char(current_bits));
                current_bits = 0;
                bit_count = 0;
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
        // try to determine size first
        let total_bits = s.len() * 6;
        println!("Input string length: {}, total_bits: {}", s.len(), total_bits);
        
        let size = match total_bits {
            1026..=1029 => (32, 32),  // 32x32: 171 * 6 = 1026
            4092..=4098 => (64, 64),  // 64x64: 683 * 6 = 4098
            _ => {
                println!("Unexpected total bits: {}", total_bits);
                return None
            }
        };
        
        println!("Detected size: {}x{}", size.0, size.1);
        
        let mut pixel = Self::with_size(size.0, size.1);
        let mut bit_pos = 0;
        
        for c in s.chars() {
            let value = match Self::map_from_safe_char(c) {
                Some(v) => v,
                None => {
                    println!("Failed to map char: '{}'", c);
                    return None;
                }
            };
            
            for i in (0..6).rev() {
                let bit = (value & (1 << i)) != 0;
                let x = bit_pos % size.0;
                let y = bit_pos / size.0;
                
                if y < size.1 {
                    pixel.set(x, y, bit);
                }
                bit_pos += 1;
            }
        }
        
        Some(pixel)
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
            pixel_art.data[y as usize * 32 + x as usize] = pixel[0] < threshold;
        }
        
        Ok(pixel_art)
    }

    // Get dimensions
    pub fn dimensions(&self) -> (usize, usize) {
        (self.width, self.height)
    }

    // Clear all pixels
    pub fn clear(&mut self) {
        for pixel in self.data.iter_mut() {
            *pixel = false;
        }
    }

    pub fn set_pixels_from_image(&mut self, x: usize, y: usize, is_black: bool) {
        self.data[y * self.width + x] = is_black;
    }

    // convert to optimal string
    pub fn to_optimal_string(&self) -> String {
        let normal_string = self.to_safe_string();
        
        match self.compress_with_deflate(&normal_string) {
            Ok(compressed_str) => {
                if compressed_str.len() + 2 < normal_string.len() {
                    format!("c:{}", compressed_str)
                } else {
                    format!("n:{}", normal_string)
                }
            }
            Err(e) => {
                format!("n:{}", normal_string)
            }
        }
    }

    // restore from optimal string
    pub fn from_optimal_string(s: &str) -> Option<Self> {
        if s.len() < 2 {
            return None;
        }

        let (prefix, data) = s.split_once(':')?;
        
        match prefix {
            "c" => {
                // process compressed data
                match Self::decompress_with_deflate(data) {
                    Ok(decompressed) => {
                        // print debug information
                        println!("Decompressed length: {}", decompressed.len());
                        println!("Decompressed data: {}", decompressed);
                        Self::from_safe_string(&decompressed)
                    },
                    Err(e) => {
                        println!("Decompression error: {}", e);
                        None
                    }
                }
            },
            "n" => Self::from_safe_string(data),
            _ => None
        }
    }

    // compress string
    fn compress_with_deflate(&self, input: &str) -> Result<String, String> {
        // convert string to raw bytes
        let bytes: Vec<u8> = input.chars()
            .map(|c| c as u8)
            .collect();
        
        let mut encoder = DeflateEncoder::new(Vec::new(), Compression::best());
        encoder.write_all(&bytes)
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
        let mut decompressed = Vec::new();
        
        decoder.read_to_end(&mut decompressed)
            .map_err(|e| format!("Decompression error: {}", e))?;
            
        // convert bytes to string, keep original ASCII values
        let result: String = decompressed.into_iter()
            .map(|b| b as char)
            .collect();
            
        // print debug information
        println!("Decoded base64 length: {}", bytes.len());
        println!("Decompressed bytes length: {}", result.len());
        
        Ok(result)
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
                pixel.set(i, j, (i + j) % 2 == 0);
            }
        }

        // Convert to string and validate length
        let encoded = pixel.to_safe_string();
        assert!(encoded.len() <= 171);

        // Validate string contains no illegal characters
        assert!(!encoded.contains('"'));
        assert!(!encoded.contains('\\'));
        assert!(encoded.chars().all(|c| c.is_ascii() && c >= ' ' && c <= '~'));

        // Validate can be correctly restored
        let decoded = Pixel::from_safe_string(&encoded).unwrap();
        assert_eq!(pixel.data, decoded.data);
    }

    #[test]
    fn test_pixel_compression() {
        // Test 1: All black pattern
        let mut pixel = Pixel::new();
        for i in 0..32 {
            for j in 0..32 {
                pixel.set(i, j, true);
            }
        }
        
        let encoded = pixel.to_optimal_string();
        
        let decoded = Pixel::from_optimal_string(&encoded).unwrap();
        assert_eq!(pixel.data, decoded.data, "All black pattern test failed");

        // Test 2: Checkerboard pattern
        let mut pixel = Pixel::new();
        for i in 0..32 {
            for j in 0..32 {
                pixel.set(i, j, (i + j) % 2 == 0);
            }
        }
        
        let encoded = pixel.to_optimal_string();
        
        let decoded = Pixel::from_optimal_string(&encoded).unwrap();
        assert_eq!(pixel.data, decoded.data, "Checkerboard pattern test failed");

        // Test 3: Empty (all white) pattern
        let pixel = Pixel::new();
        let encoded = pixel.to_optimal_string();
        
        let decoded = Pixel::from_optimal_string(&encoded).unwrap();
        assert_eq!(pixel.data, decoded.data, "Empty pattern test failed");
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
        // best compression scenario: all black image
        let mut best_case = Pixel::new();
        for x in 0..32 {
            for y in 0..32 {
                best_case.set(x, y, true);
            }
        }
        
        // test uncompressed string
        let safe_str = best_case.to_safe_string();
        println!("\nUncompressed string (len={}):\n{}", safe_str.len(), safe_str);
        
        // test compressed string
        let compressed_str = best_case.to_optimal_string();
        println!("\nOptimal string (len={}):\n{}", compressed_str.len(), compressed_str);
        
        // verify correctness
        let decoded_best = Pixel::from_optimal_string(&compressed_str).unwrap();
        assert_eq!(best_case.data, decoded_best.data, "Compression/decompression failed");
        
        // calculate compression ratio
        if compressed_str.starts_with("c:") {
            let compression_ratio = (compressed_str.len() as f64 / safe_str.len() as f64) * 100.0;
            println!("Compression ratio: {:.2}%", compression_ratio);
        }
    }

    #[test]
    fn print_pixel_to_ascii_mapping() {
        println!("Bits | Dec | ASCII | Code");
        println!("--------------------|-----");
        
        // test all 64 possible 6-bit values
        for i in 0..64 {
            let c = Pixel::map_to_safe_char(i);
            println!("{:06b} | {:3} | {} | {}", i, i, c, c as u8);
            
            // verify the character is valid
            assert!(c as u8 >= 35, "Character code too low");
            assert!(c as u8 <= 126, "Character code too high");
            assert_ne!(c as u8, 58, "Found colon");
            assert_ne!(c as u8, 92, "Found backslash");
        }
    }

    #[test]
    fn test_special_chars_skipped() {
        // test all mapped characters
        for i in 0..64 {
            let c = Pixel::map_to_safe_char(i);
            assert_ne!(c, '"', "Double quote found in mapping");
            assert_ne!(c, ':', "Colon found in mapping");
            assert_ne!(c, '\\', "Backslash found in mapping");
            
            // verify reverse mapping
            let decoded = Pixel::map_from_safe_char(c).unwrap();
            assert_eq!(i, decoded, "Mapping failed for value {}", i);
        }
        
        // print mapping table for inspection
        println!("Character mapping table:");
        for i in 0..64 {
            let c = Pixel::map_to_safe_char(i);
            println!("Value {:2} -> Char '{}' (ASCII {})", i, c, c as u8);
        }
    }

    #[test]
    fn test_char_mapping() {
        // test all possible input values (0-63)
        for value in 0..64 {
            let c = Pixel::map_to_safe_char(value);
            println!("Testing value {}: mapped to '{}' (ASCII {})", value, c, c as u8);
            
            // test reverse mapping
            let decoded = Pixel::map_from_safe_char(c).unwrap();
            println!("  Reverse mapping: '{}' -> {}", c, decoded);
            
            // verify mapping
            assert_eq!(value, decoded, 
                "Mapping failed: {} -> '{}' -> {}", 
                value, c, decoded);
        }
        
        // test some special characters
        let special_chars = vec![':', '\\', '"'];
        for &c in &special_chars {
            let result = Pixel::map_from_safe_char(c);
            assert!(result.is_none(), 
                "Special character '{}' should not be mapped", c);
        }
    }

    #[test]
    fn test_compression_patterns() {
        // test two sizes
        let sizes = vec![(32, 32, "32x32"), (64, 64, "64x64")];
        
        for &(width, height, size_name) in &sizes {
            println!("\n\n=== Testing {} patterns ===", size_name);
            
            // 1. best compression case: all black image
            let mut best_case = Pixel::with_size(width, height);
            for x in 0..width {
                for y in 0..height {
                    best_case.set(x, y, true);
                }
            }
            
            // 2. checkerboard pattern
            let mut checkerboard = Pixel::with_size(width, height);
            for x in 0..width {
                for y in 0..height {
                    checkerboard.set(x, y, (x + y) % 2 == 0);
                }
            }
            
            // 3. random noise pattern
            let mut random_pattern = Pixel::with_size(width, height);
            for x in 0..width {
                for y in 0..height {
                    random_pattern.set(x, y, (x ^ y) % 3 == 0);
                }
            }

            let test_cases = vec![
                ("Best case (all black)", best_case),
                ("Checkerboard pattern", checkerboard),
                ("Random noise pattern", random_pattern),
            ];

            for (pattern_name, pixel) in test_cases {
                println!("\n=== {} - {} ===", size_name, pattern_name);
                
                // 1. original uncompressed string
                let uncompressed = pixel.to_safe_string();
                println!("\nOriginal uncompressed string (len={}):\n{}", 
                    uncompressed.len(), uncompressed);
                
                // 2. optimal (possibly compressed) string
                let optimal = pixel.to_optimal_string();
                println!("\nOptimal string (len={}):\n{}", 
                    optimal.len(), optimal);
                
                // 3. decompressed string
                let decoded = Pixel::from_optimal_string(&optimal).unwrap();
                let decompressed = decoded.to_safe_string();
                println!("\nDecompressed string (len={}):\n{}", 
                    decompressed.len(), decompressed);
                
                // verify strings are identical
                assert_eq!(uncompressed, decompressed, 
                    "Decompressed string doesn't match original for {} - {}", 
                    size_name, pattern_name);
                
                // show compression information
                if optimal.starts_with("c:") {
                    let ratio = (optimal.len() as f64 / uncompressed.len() as f64) * 100.0;
                    println!("\nUsing compression, ratio: {:.2}%", ratio);
                } else {
                    println!("\nNo compression used (would increase size)");
                }
                
                println!("\n{}", "=".repeat(50));
            }
        }
    }
} 