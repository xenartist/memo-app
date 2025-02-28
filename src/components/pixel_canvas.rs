use dioxus::prelude::*;

/// A component that renders a 50x50 pixel grid from a hex string
#[component]
pub fn PixelCanvas(hex_string: String) -> Element {
    let grid_size = 50;
    
    // Function to convert hex string to a 2D grid of 0s and 1s
    let generate_grid = |hex_str: &str| -> Vec<Vec<u8>> {
        let mut grid = vec![vec![0; grid_size]; grid_size];
        let hex_str = hex_str.trim().to_uppercase();
        
        let mut hex_index = 0;
        
        // Process each row
        for row in 0..grid_size {
            let mut col = 0;
            while col < grid_size && hex_index < hex_str.len() {
                // Get the hex digit and convert to binary (4 bits)
                if let Some(hex_digit) = hex_str.chars().nth(hex_index) {
                    if let Ok(decimal) = u8::from_str_radix(&hex_digit.to_string(), 16) {
                        // Convert to binary (4 bits)
                        let binary = format!("{:04b}", decimal);
                        
                        // Set the pixels according to binary representation
                        for (bit_idx, bit) in binary.chars().enumerate() {
                            if col + bit_idx < grid_size {
                                if bit == '1' {
                                    grid[row][col + bit_idx] = 1;
                                }
                            }
                        }
                    }
                    hex_index += 1;
                    col += 4; // Move to the next 4 columns
                } else {
                    break;
                }
            }
        }
        
        log::info!("Generated pixel grid from hex string. Used {} of {} hex chars", hex_index, hex_str.len());
        grid
    };
    
    // Generate the grid from the hex string
    let grid = generate_grid(&hex_string);
    
    rsx! {
        div {
            class: "pixel-canvas-container",
            div {
                class: "pixel-grid",
                // Render each pixel in the grid
                {
                    (0..grid_size).map(|row_idx| {
                        rsx! {
                            div {
                                class: "pixel-row",
                                key: "{row_idx}",
                                {
                                    (0..grid_size).map(|col_idx| {
                                        let pixel_class = if grid[row_idx][col_idx] == 1 { "pixel active" } else { "pixel" };
                                        rsx! {
                                            div {
                                                class: "{pixel_class}",
                                                key: "{row_idx}-{col_idx}"
                                            }
                                        }
                                    })
                                }
                            }
                        }
                    })
                }
            }
        }
    }
} 