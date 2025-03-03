use image::{DynamicImage, GenericImageView, ImageBuffer, Luma};

// Convert image to hex string
pub fn image_to_hex(img: &DynamicImage) -> String {
    // Step 1: Resize image to 48x48
    let resized = img.resize_exact(48, 48, image::imageops::FilterType::Lanczos3);
    
    // Step 2: Convert to grayscale
    let gray_img = resized.to_luma8();
    
    // Step 3: Convert to binary (black and white)
    let mut binary_string = String::with_capacity(48 * 48);
    for pixel in gray_img.pixels() {
        // If pixel is darker than mid-gray (128), consider it black (1)
        // Otherwise, consider it white (0)
        if pixel[0] < 128 {
            binary_string.push('1');
        } else {
            binary_string.push('0');
        }
    }
    
    // Step 4: Convert binary string to hex
    binary_to_hex(&binary_string)
}

// Convert binary string to hex string
fn binary_to_hex(binary: &str) -> String {
    let mut hex = String::with_capacity(binary.len() / 4);
    let mut chars = binary.chars();
    
    // Process 4 bits at a time
    while let Some(bits) = chars.next().map(|a| {
        chars.next().map(|b| {
            chars.next().map(|c| {
                chars.next().map(|d| {
                    // Convert 4 binary digits to hex
                    let value = (a.to_digit(2).unwrap_or(0) << 3) |
                              (b.to_digit(2).unwrap_or(0) << 2) |
                              (c.to_digit(2).unwrap_or(0) << 1) |
                              (d.to_digit(2).unwrap_or(0));
                    format!("{:X}", value)
                })
            })
        })
    }).flatten().flatten().flatten() {
        hex.push_str(&bits);
    }
    
    hex
}

// Convert hex string back to binary string
pub fn hex_to_binary(hex: &str) -> String {
    let mut binary = String::with_capacity(hex.len() * 4);
    
    for c in hex.chars() {
        if let Some(value) = c.to_digit(16) {
            // Convert each hex digit to 4 binary digits
            binary.push_str(&format!("{:04b}", value));
        }
    }
    
    binary
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_binary_to_hex() {
        assert_eq!(binary_to_hex("1111"), "F");
        assert_eq!(binary_to_hex("0000"), "0");
        assert_eq!(binary_to_hex("10101010"), "AA");
        assert_eq!(binary_to_hex("11110000"), "F0");
    }
    
    #[test]
    fn test_hex_to_binary() {
        assert_eq!(hex_to_binary("F"), "1111");
        assert_eq!(hex_to_binary("0"), "0000");
        assert_eq!(hex_to_binary("AA"), "10101010");
        assert_eq!(hex_to_binary("F0"), "11110000");
    }
} 