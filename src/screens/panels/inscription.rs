use egui::{Ui, Vec2, RichText, Window, Rect, Pos2, Color32, Stroke, CornerRadius};
use crate::core::img2hex::{self, image_to_hex, hex_to_binary, binary_to_hex};

// Callback type for inscription creation
pub type InscriptionCallback = Box<dyn Fn(String) + 'static>;

pub struct InscriptionPanel {
    // Image dialog state
    show_image_dialog: bool,
    // Imported image hex string
    image_hex: Option<String>,
    // Binary representation for display
    image_binary: Option<String>,
    // Status message for inscription
    inscription_status: String,
    // Callback for inscription creation
    inscription_callback: Option<InscriptionCallback>,
}

impl Default for InscriptionPanel {
    fn default() -> Self {
        Self {
            show_image_dialog: false,
            image_hex: None,
            image_binary: None,
            inscription_status: String::new(),
            inscription_callback: None,
        }
    }
}

impl InscriptionPanel {
    pub fn new() -> Self {
        Self::default()
    }
    
    // Set the callback for inscription creation
    pub fn set_inscription_callback<F>(&mut self, callback: F)
    where
        F: Fn(String) + 'static,
    {
        self.inscription_callback = Some(Box::new(callback));
    }
    
    // Set the inscription status
    pub fn set_inscription_status(&mut self, status: String) {
        self.inscription_status = status;
    }

    // Toggle pixel at given position
    fn toggle_pixel(&mut self, x: usize, y: usize) {
        if let Some(binary) = &mut self.image_binary {
            let index = y * 48 + x;
            if index < binary.len() {
                // Convert to chars for easier manipulation
                let mut chars: Vec<char> = binary.chars().collect();
                // Toggle the bit
                chars[index] = if chars[index] == '1' { '0' } else { '1' };
                // Convert back to string
                *binary = chars.into_iter().collect();
                // Update hex string
                self.image_hex = Some(binary_to_hex(binary));
            }
        }
    }
    
    // Show image import dialog
    fn show_image_dialog(&mut self, ui: &mut Ui) {
        Window::new("New Inscription")
            .default_size(Vec2::new(600.0, 700.0))
            .show(ui.ctx(), |ui| {
                ui.vertical_centered(|ui| {
                    ui.add_space(20.0);
                    // Import button
                    if ui.button("Choose Image").clicked() {
                        if let Some(path) = rfd::FileDialog::new()
                            .add_filter("Image", &["png", "jpg", "jpeg", "gif", "bmp"])
                            .pick_file() 
                        {
                            // Load and convert image
                            if let Ok(img) = image::open(&path) {
                                let hex = image_to_hex(&img);
                                self.image_binary = Some(hex_to_binary(&hex));
                                self.image_hex = Some(hex);
                                self.inscription_status = String::new(); // Clear status when new image is loaded
                            }
                        }
                    }
                    
                    ui.add_space(20.0);
                    
                    // Display pixel art if available
                    if let Some(binary) = &self.image_binary {
                        // Calculate pixel size based on available width
                        let available_width = ui.available_width().min(500.0);
                        let pixel_size = (available_width / 48.0).floor();
                        let total_size = pixel_size * 48.0;
                        
                        // Create a frame for the pixel art
                        let (response, painter) = ui.allocate_painter(
                            Vec2::new(total_size, total_size),
                            egui::Sense::click_and_drag()
                        );
                        
                        let rect = response.rect;
                        
                        // Draw background
                        painter.rect_filled(rect, CornerRadius::default(), Color32::WHITE);
                        
                        // Draw pixels
                        for (i, bit) in binary.chars().enumerate() {
                            if bit == '1' {
                                let x = (i % 48) as f32 * pixel_size;
                                let y = (i / 48) as f32 * pixel_size;
                                
                                painter.rect_filled(
                                    Rect::from_min_size(
                                        Pos2::new(rect.min.x + x, rect.min.y + y),
                                        Vec2::new(pixel_size, pixel_size)
                                    ),
                                    CornerRadius::default(),
                                    Color32::BLACK
                                );
                            }
                        }
                        
                        // Draw grid
                        for i in 0..=48 {
                            let x = rect.min.x + i as f32 * pixel_size;
                            let y = rect.min.y + i as f32 * pixel_size;
                            
                            painter.line_segment(
                                [Pos2::new(x, rect.min.y), Pos2::new(x, rect.max.y)],
                                Stroke::new(0.5, Color32::GRAY)
                            );
                            painter.line_segment(
                                [Pos2::new(rect.min.x, y), Pos2::new(rect.max.x, y)],
                                Stroke::new(0.5, Color32::GRAY)
                            );
                        }

                        // Handle mouse clicks
                        if response.clicked() || response.dragged() {
                            if let Some(pos) = response.hover_pos() {
                                let x = ((pos.x - rect.min.x) / pixel_size).floor() as usize;
                                let y = ((pos.y - rect.min.y) / pixel_size).floor() as usize;
                                if x < 48 && y < 48 {
                                    self.toggle_pixel(x, y);
                                }
                            }
                        }
                        
                        ui.add_space(20.0);
                        
                        // Display hex string
                        if let Some(hex) = &self.image_hex {
                            ui.label("Hex representation:");
                            ui.add_space(5.0);
                            ui.label(RichText::new(hex).monospace());
                            
                            ui.add_space(20.0);
                            
                            // Add the "Inscribe It!" button below the hex string
                            let inscribe_button_size = Vec2::new(250.0, 50.0);
                            let inscribe_button = egui::Button::new(
                                RichText::new("Inscribe It!")
                                    .size(24.0)
                                    .strong()
                                    .color(Color32::WHITE)
                            )
                            .fill(Color32::from_rgb(76, 175, 80)); // Green color
                            
                            if ui.add_sized(inscribe_button_size, inscribe_button).clicked() {
                                // Call the callback with the hex data
                                if let Some(callback) = &self.inscription_callback {
                                    callback(hex.clone());
                                }
                            }
                            
                            // Display status message if any
                            if !self.inscription_status.is_empty() {
                                ui.add_space(10.0);
                                ui.colored_label(
                                    if self.inscription_status.starts_with("Error") { 
                                        Color32::RED 
                                    } else if self.inscription_status.starts_with("Creating") {
                                        Color32::YELLOW
                                    } else { 
                                        Color32::GREEN 
                                    },
                                    &self.inscription_status
                                );
                            }
                        }
                    }
                });
            });
    }
    
    pub fn show(&mut self, ui: &mut Ui) {
        ui.vertical_centered(|ui| {
            ui.heading("Inscription Panel");
            ui.add_space(20.0);
            
            ui.label("Create a new inscription by importing an image.");
            ui.add_space(20.0);
            
            // Button to open the image dialog
            let button_size = Vec2::new(200.0, 50.0);
            if ui.add_sized(button_size, egui::Button::new("New Inscription")).clicked() {
                self.show_image_dialog = true;
            }
            
            // Show the image dialog if needed
            if self.show_image_dialog {
                self.show_image_dialog(ui);
            }
        });
    }
} 