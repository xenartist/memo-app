use egui::{Ui, Vec2, RichText, Window, Rect, Pos2, Color32, Stroke, Rounding};
use crate::core::img2hex::{self, image_to_hex};

pub struct InscriptionPanel {
    // Image dialog state
    show_image_dialog: bool,
    // Imported image hex string
    image_hex: Option<String>,
    // Binary representation for display
    image_binary: Option<String>,
}

impl Default for InscriptionPanel {
    fn default() -> Self {
        Self {
            show_image_dialog: false,
            image_hex: None,
            image_binary: None,
        }
    }
}

impl InscriptionPanel {
    pub fn new() -> Self {
        Self::default()
    }

    // Show image import dialog
    fn show_image_dialog(&mut self, ui: &mut Ui) {
        Window::new("New Inscription")
            .default_size(Vec2::new(600.0, 700.0))
            .show(ui.ctx(), |ui| {
                ui.vertical_centered(|ui| {
                    ui.heading("Import and Convert Image");
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
                                self.image_binary = Some(img2hex::hex_to_binary(&hex));
                                self.image_hex = Some(hex);
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
                            egui::Sense::hover()
                        );
                        
                        let rect = response.rect;
                        
                        // Draw background
                        painter.rect_filled(rect, Rounding::default(), Color32::WHITE);
                        
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
                                    Rounding::default(),
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
                        
                        ui.add_space(20.0);
                        
                        // Display hex string
                        if let Some(hex) = &self.image_hex {
                            ui.label("Hex representation:");
                            ui.add_space(5.0);
                            ui.label(RichText::new(hex).monospace());
                        }
                    }
                });
            });
    }

    pub fn show(&mut self, ui: &mut Ui) {
        ui.vertical_centered(|ui| {
            ui.heading("Inscription");
            ui.add_space(20.0);
            
            // Image import button
            let button_size = Vec2::new(200.0, 40.0);
            if ui.add_sized(button_size, egui::Button::new("New Inscription")).clicked() {
                self.show_image_dialog = true;
            }
            
            // Show image dialog if open
            if self.show_image_dialog {
                self.show_image_dialog(ui);
            }
        });
    }
} 