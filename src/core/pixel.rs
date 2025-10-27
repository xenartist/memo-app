use flate2::Compression;
use flate2::write::DeflateEncoder;
use flate2::read::DeflateDecoder;
use base64::{encode, decode};
use std::io::prelude::*;
use rand::Rng;

#[derive(Debug, PartialEq, Clone)]
pub struct Pixel {
    width: usize,
    height: usize,
    data: Vec<bool>,
}

impl Pixel {
    // default create 32x32 pixel art
    pub fn new_with_size(size: usize) -> Self {
        Self::with_size(size, size)
    }

    // default create 32x32 pixel art
    pub fn new() -> Self {
        Self::new_with_size(32)  // default create 32x32 pixel art
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

    // Check if all pixels are false (blank image)
    pub fn is_blank(&self) -> bool {
        self.data.iter().all(|&p| !p)
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
            // 8x8: 64 pixels, need 11 chars (66 bits)
            60..=72 => (8, 8),
            // 16x16: 256 pixels, need 43 chars (258 bits)  
            252..=264 => (16, 16),
            // 32x32: 1024 pixels, need 171 chars (1026 bits)
            1020..=1032 => (32, 32),
            // 64x64: 4096 pixels, need 683 chars (4098 bits)
            4092..=4104 => (64, 64),
            // 96x96: 9216 pixels, need 1536 chars (9216 bits)
            9210..=9222 => (96, 96),
            // 128x128: 16384 pixels, need 2731 chars (16386 bits)
            16380..=16392 => (128, 128),
            // 256x256: 65536 pixels, need 10923 chars (65538 bits)
            65532..=65544 => (256, 256),
            // 512x512: 262144 pixels, need 43691 chars (262146 bits)
            262140..=262152 => (512, 512),
            // 1024x1024: 1048576 pixels, need 174763 chars (1048578 bits)
            1048572..=1048584 => (1024, 1024),
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

    // modify the image import method, support larger sizes
    pub fn from_image_data_with_size(data: &[u8], size: usize) -> Result<Self, String> {
        if size > 1024 {
            return Err("Maximum supported size is 1024x1024".to_string());
        }

        // Load image from bytes
        let img = image::load_from_memory(data)
            .map_err(|e| format!("Failed to load image: {}", e))?;
        
        // Resize to specified size
        let resized = img.resize_exact(size as u32, size as u32, image::imageops::FilterType::Lanczos3);
        
        // Convert to grayscale
        let gray = resized.into_luma8();
        
        // Convert to black and white using threshold
        let threshold = 128u8;
        let mut pixel_art = Self::new_with_size(size);
        
        for (x, y, pixel) in gray.enumerate_pixels() {
            pixel_art.data[y as usize * size + x as usize] = pixel[0] < threshold;
        }
        
        Ok(pixel_art)
    }

    // keep the original method as backward compatibility
    pub fn from_image_data(data: &[u8]) -> Result<Self, String> {
        Self::from_image_data_with_size(data, 32)
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
                    format!("c:{}x{}:{}", self.width, self.height, compressed_str)
                } else {
                    format!("n:{}x{}:{}", self.width, self.height, normal_string)
                }
            }
            Err(e) => {
                format!("n:{}x{}:{}", self.width, self.height, normal_string)
            }
        }
    }

    // restore from optimal string
    pub fn from_optimal_string(s: &str) -> Option<Self> {
        if s.len() < 2 {
            return None;
        }

        // Try new format first: type:widthxheight:data
        let parts: Vec<&str> = s.splitn(3, ':').collect();
        
        if parts.len() == 3 {
            // New format: type:widthxheight:data
            let format_type = parts[0];
            let size_str = parts[1];
            let data = parts[2];
            
            // Parse size: "32x32" -> (32, 32)
            let size_parts: Vec<&str> = size_str.split('x').collect();
            if size_parts.len() != 2 {
                return None;
            }
            
            let width = size_parts[0].parse::<usize>().ok()?;
            let height = size_parts[1].parse::<usize>().ok()?;
            
            // Process data based on format type
            match format_type {
                "c" => {
                    // Process compressed data
                    match Self::decompress_with_deflate(data) {
                        Ok(decompressed) => {
                            Self::from_safe_string_with_size(&decompressed, width, height)
                        },
                        Err(e) => {
                            println!("Decompression error: {}", e);
                            None
                        }
                    }
                },
                "n" => Self::from_safe_string_with_size(data, width, height),
                _ => None
            }
        } else if parts.len() == 2 {
            // Old format for backward compatibility: type:data
            let (prefix, data) = s.split_once(':')?;
            
            match prefix {
                "c" => {
                    // Process compressed data (old format)
                    match Self::decompress_with_deflate(data) {
                        Ok(decompressed) => {
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
        } else {
            None
        }
    }

    // New helper function: restore from safe string with specified dimensions
    pub fn from_safe_string_with_size(s: &str, width: usize, height: usize) -> Option<Self> {
        let expected_pixels = width * height;
        let expected_chars = (expected_pixels + 5) / 6; // Round up division
        
        // Validate string length makes sense for the given dimensions
        if s.len() < expected_chars || s.len() > expected_chars + 1 {
            println!("String length {} doesn't match expected {} for {}x{}", 
                s.len(), expected_chars, width, height);
            return None;
        }
        
        let mut pixel = Self::with_size(width, height);
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
                if bit_pos >= expected_pixels {
                    break; // Stop if we've filled all pixels
                }
                
                let bit = (value & (1 << i)) != 0;
                let x = bit_pos % width;
                let y = bit_pos / width;
                
                pixel.set(x, y, bit);
                bit_pos += 1;
            }
            
            if bit_pos >= expected_pixels {
                break;
            }
        }
        
        Some(pixel)
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
        // test different size patterns
        let blank_32 = Pixel::new_with_size(32);
        let blank_64 = Pixel::new_with_size(64);
        let blank_96 = Pixel::new_with_size(96);

        println!("32x32 blank optimal: {}", blank_32.to_optimal_string());
        println!("64x64 blank optimal: {}", blank_64.to_optimal_string());
        println!("96x96 blank optimal: {}", blank_96.to_optimal_string());

        // test different size black pixel art
        let mut black_32 = Pixel::new_with_size(32);
        let mut black_64 = Pixel::new_with_size(64);
        let mut black_96 = Pixel::new_with_size(96);

        for i in 0..(32*32) {
            black_32.data[i] = true;
        }
        for i in 0..(64*64) {
            black_64.data[i] = true;
        }
        for i in 0..(96*96) {
            black_96.data[i] = true;
        }

        println!("32x32 black optimal: {}", black_32.to_optimal_string());
        println!("64x64 black optimal: {}", black_64.to_optimal_string());
        println!("96x96 black optimal: {}", black_96.to_optimal_string());

        // test different size checkerboard pattern
        let mut checker_32 = Pixel::new_with_size(32);
        let mut checker_64 = Pixel::new_with_size(64);
        let mut checker_96 = Pixel::new_with_size(96);

        for y in 0..32 {
            for x in 0..32 {
                checker_32.set_pixel(x, y, (x + y) % 2 == 0);
            }
        }
        for y in 0..64 {
            for x in 0..64 {
                checker_64.set_pixel(x, y, (x + y) % 2 == 0);
            }
        }
        for y in 0..96 {
            for x in 0..96 {
                checker_96.set_pixel(x, y, (x + y) % 2 == 0);
            }
        }

        println!("32x32 checker optimal: {}", checker_32.to_optimal_string());
        println!("64x64 checker optimal: {}", checker_64.to_optimal_string());
        println!("96x96 checker optimal: {}", checker_96.to_optimal_string());

        // test different size diagonal pattern
        let mut diagonal_32 = Pixel::new_with_size(32);
        let mut diagonal_64 = Pixel::new_with_size(64);
        let mut diagonal_96 = Pixel::new_with_size(96);

        for i in 0..32 {
            diagonal_32.set_pixel(i, i, true);
        }
        for i in 0..64 {
            diagonal_64.set_pixel(i, i, true);
        }
        for i in 0..96 {
            diagonal_96.set_pixel(i, i, true);
        }

        println!("32x32 diagonal optimal: {}", diagonal_32.to_optimal_string());
        println!("64x64 diagonal optimal: {}", diagonal_64.to_optimal_string());
        println!("96x96 diagonal optimal: {}", diagonal_96.to_optimal_string());

        // test different size border pattern
        let mut border_32 = Pixel::new_with_size(32);
        let mut border_64 = Pixel::new_with_size(64);
        let mut border_96 = Pixel::new_with_size(96);

        // 32x32 border
        for i in 0..32 {
            border_32.set_pixel(0, i, true);
            border_32.set_pixel(31, i, true);
            border_32.set_pixel(i, 0, true);
            border_32.set_pixel(i, 31, true);
        }

        // 64x64 border
        for i in 0..64 {
            border_64.set_pixel(0, i, true);
            border_64.set_pixel(63, i, true);
            border_64.set_pixel(i, 0, true);
            border_64.set_pixel(i, 63, true);
        }

        // 96x96 border
        for i in 0..96 {
            border_96.set_pixel(0, i, true);
            border_96.set_pixel(95, i, true);
            border_96.set_pixel(i, 0, true);
            border_96.set_pixel(i, 95, true);
        }

        println!("32x32 border optimal: {}", border_32.to_optimal_string());
        println!("64x64 border optimal: {}", border_64.to_optimal_string());
        println!("96x96 border optimal: {}", border_96.to_optimal_string());

        // print compression ratio of each pattern
        let print_compression_ratio = |name: &str, pixel: &Pixel| {
            let normal = pixel.to_safe_string();
            let optimal = pixel.to_optimal_string();
            println!("{} size: {}x{}", name, pixel.width, pixel.height);
            println!("Normal length: {}", normal.len());
            println!("Optimal length: {}", optimal.len());
            println!("Compression ratio: {:.2}%", 
                (1.0 - optimal.len() as f64 / normal.len() as f64) * 100.0);
            println!("---");
        };

        print_compression_ratio("32x32 Blank", &blank_32);
        print_compression_ratio("64x64 Blank", &blank_64);
        print_compression_ratio("96x96 Blank", &blank_96);

        print_compression_ratio("32x32 Black", &black_32);
        print_compression_ratio("64x64 Black", &black_64);
        print_compression_ratio("96x96 Black", &black_96);

        print_compression_ratio("32x32 Checker", &checker_32);
        print_compression_ratio("64x64 Checker", &checker_64);
        print_compression_ratio("96x96 Checker", &checker_96);

        print_compression_ratio("32x32 Diagonal", &diagonal_32);
        print_compression_ratio("64x64 Diagonal", &diagonal_64);
        print_compression_ratio("96x96 Diagonal", &diagonal_96);

        print_compression_ratio("32x32 Border", &border_32);
        print_compression_ratio("64x64 Border", &border_64);
        print_compression_ratio("96x96 Border", &border_96);

        // test different size random pattern
        let mut rng = rand::thread_rng();
        let mut random_32 = Pixel::new_with_size(32);
        let mut random_64 = Pixel::new_with_size(64);
        let mut random_96 = Pixel::new_with_size(96);

        // 32x32 random fill
        for i in 0..(32*32) {
            random_32.data[i] = rng.gen_bool(0.5);  // 50% 的概率为黑色
        }

        // 64x64 random fill
        for i in 0..(64*64) {
            random_64.data[i] = rng.gen_bool(0.5);
        }

        // 96x96 random fill
        for i in 0..(96*96) {
            random_96.data[i] = rng.gen_bool(0.5);
        }

        println!("\n=== Random Pattern Results ===");
        println!("32x32 random optimal: {}", random_32.to_optimal_string());
        println!("64x64 random optimal: {}", random_64.to_optimal_string());
        println!("96x96 random optimal: {}", random_96.to_optimal_string());

        print_compression_ratio("32x32 Random", &random_32);
        print_compression_ratio("64x64 Random", &random_64);
        print_compression_ratio("96x96 Random", &random_96);

        // print black pixel ratio of random pattern
        let count_black_pixels = |pixel: &Pixel| {
            pixel.data.iter().filter(|&&x| x).count() as f64 / pixel.data.len() as f64 * 100.0
        };

        println!("\n=== Random Pattern Black Pixel Ratios ===");
        println!("32x32 Black pixel ratio: {:.2}%", count_black_pixels(&random_32));
        println!("64x64 Black pixel ratio: {:.2}%", count_black_pixels(&random_64));
        println!("96x96 Black pixel ratio: {:.2}%", count_black_pixels(&random_96));
    }
} 