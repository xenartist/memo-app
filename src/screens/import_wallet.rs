use egui::{CentralPanel, Context, FontId, TextStyle, Frame, Vec2, TextEdit, Grid, RadioButton, ScrollArea};
use super::Screen;
use crate::core::wallet::Wallet;

pub struct ImportWalletScreen {
    // Whether to use 24 words (otherwise use 12)
    use_24_words: bool,
    // Seed words input by user
    seed_words: Vec<String>,
    // Password for encrypting the wallet
    password: String,
    // Confirm password
    confirm_password: String,
    // Status message
    status_message: String,
}

impl Default for ImportWalletScreen {
    fn default() -> Self {
        // Initialize with empty seed words for 12 words (default)
        let mut seed_words = Vec::new();
        for _ in 0..12 {
            seed_words.push(String::new());
        }
        
        Self {
            use_24_words: false,
            seed_words,
            password: String::new(),
            confirm_password: String::new(),
            status_message: String::new(),
        }
    }
}

impl ImportWalletScreen {
    pub fn new() -> Self {
        Self::default()
    }
    
    // Update the number of seed word inputs based on selection
    fn update_seed_word_count(&mut self) {
        let target_count = if self.use_24_words { 24 } else { 12 };
        
        // Resize the vector to match the target count
        if self.seed_words.len() < target_count {
            // Add empty strings if we need more inputs
            while self.seed_words.len() < target_count {
                self.seed_words.push(String::new());
            }
        } else if self.seed_words.len() > target_count {
            // Remove excess inputs if we need fewer
            self.seed_words.truncate(target_count);
        }
    }
    
    // Get the seed phrase
    pub fn get_seed_phrase(&self) -> String {
        self.seed_words.join(" ")
    }
    
    // Display seed word input fields in a grid
    fn show_seed_word_inputs(&mut self, ui: &mut egui::Ui) {
        let words_per_row = if self.use_24_words { 6 } else { 4 };
        let rows = self.seed_words.len() / words_per_row;
        
        // Create a grid for the seed word inputs
        Grid::new("seed_words_grid")
            .num_columns(words_per_row)
            .spacing([20.0, 10.0])
            .show(ui, |ui| {
                for row in 0..rows {
                    for col in 0..words_per_row {
                        let index = row * words_per_row + col;
                        let word_number = index + 1; // 1-indexed for display
                        
                        ui.vertical(|ui| {
                            ui.label(format!("{}.", word_number));
                            ui.add(TextEdit::singleline(&mut self.seed_words[index])
                                .hint_text("word")
                                .desired_width(100.0));
                        });
                    }
                    ui.end_row();
                }
            });
    }

    pub fn render(&mut self, ctx: &Context) -> Option<Screen> {
        let mut next_screen = None;

        // Set font style
        let mut style = (*ctx.style()).clone();
        style.text_styles.insert(
            TextStyle::Button,
            FontId::new(22.0, egui::FontFamily::Proportional)
        );
        style.text_styles.insert(
            TextStyle::Heading,
            FontId::new(36.0, egui::FontFamily::Proportional)
        );
        style.text_styles.insert(
            TextStyle::Body,
            FontId::new(22.0, egui::FontFamily::Proportional)
        );
        ctx.set_style(style);

        CentralPanel::default().show(ctx, |ui| {
            // Add some vertical space at the top
            ui.add_space(20.0);
            
            // Use a scroll area to handle content that might not fit
            ScrollArea::vertical().show(ui, |ui| {
                // Create a centered form
                ui.vertical_centered(|ui| {
                    ui.heading("Import Existing Wallet");
                    ui.add_space(20.0);
                    
                    // Set a fixed width for the form
                    let form_width = 600.0;
                    
                    // Create a frame for the form with fixed width
                    Frame::group(ui.style())
                        .inner_margin(30.0)
                        .outer_margin(10.0)
                        .show(ui, |ui| {
                            // Set the maximum width for the content
                            ui.set_max_width(form_width);
                            
                            ui.vertical_centered(|ui| {
                                // Radio buttons to select 12 or 24 words
                                ui.label("Recovery seed length:");
                                ui.add_space(5.0);
                                
                                ui.horizontal(|ui| {
                                    let mut changed = false;
                                    
                                    if ui.add(RadioButton::new(!self.use_24_words, "12 words")).clicked() {
                                        if self.use_24_words {
                                            self.use_24_words = false;
                                            changed = true;
                                        }
                                    }
                                    
                                    ui.add_space(20.0);
                                    
                                    if ui.add(RadioButton::new(self.use_24_words, "24 words")).clicked() {
                                        if !self.use_24_words {
                                            self.use_24_words = true;
                                            changed = true;
                                        }
                                    }
                                    
                                    // Update seed word inputs if count changes
                                    if changed {
                                        self.update_seed_word_count();
                                    }
                                });
                                
                                ui.add_space(20.0);
                                
                                ui.label("Enter your recovery seed phrase:");
                                ui.add_space(10.0);
                                
                                // Display seed word input fields
                                self.show_seed_word_inputs(ui);
                                
                                ui.add_space(20.0);
                                
                                // Password fields for encryption
                                ui.label("Enter password to encrypt your wallet:");
                                ui.add_space(5.0);
                                
                                // Password field
                                ui.horizontal(|ui| {
                                    ui.label("Password:      ");
                                    ui.add(TextEdit::singleline(&mut self.password)
                                        .password(true)
                                        .hint_text("Enter password")
                                        .desired_width(300.0));
                                });
                                
                                ui.add_space(5.0);
                                
                                // Confirm password field
                                ui.horizontal(|ui| {
                                    ui.label("Confirm:       ");
                                    ui.add(TextEdit::singleline(&mut self.confirm_password)
                                        .password(true)
                                        .hint_text("Confirm password")
                                        .desired_width(300.0));
                                });
                                
                                // Display status message if any
                                if !self.status_message.is_empty() {
                                    ui.add_space(10.0);
                                    ui.colored_label(
                                        if self.status_message.starts_with("Error") { 
                                            egui::Color32::RED 
                                        } else { 
                                            egui::Color32::GREEN 
                                        },
                                        &self.status_message
                                    );
                                }
                            });
                            
                            ui.add_space(20.0);
                            
                            // Bottom buttons
                            ui.horizontal(|ui| {
                                // Calculate button sizes and spacing
                                let button_size = Vec2::new(200.0, 40.0);
                                let available_width = ui.available_width();
                                let spacing = (available_width - 2.0 * button_size.x) / 3.0;
                                
                                ui.add_space(spacing);
                                
                                if ui.add_sized(button_size, egui::Button::new("Back")).clicked() {
                                    next_screen = Some(Screen::Login);
                                }
                                
                                ui.add_space(spacing);
                                
                                if ui.add_sized(button_size, egui::Button::new("Import Encrypted Wallet")).clicked() {
                                    // Validate mnemonic
                                    match Wallet::validate_mnemonic(&self.seed_words) {
                                        Ok(seed_phrase) => {
                                            // Validate passwords
                                            if self.password.is_empty() {
                                                self.status_message = "Error: Password cannot be empty".to_string();
                                            } else if self.password != self.confirm_password {
                                                self.status_message = "Error: Passwords do not match".to_string();
                                            } else {
                                                // Save wallet
                                                match Wallet::save_wallet(&seed_phrase, &self.password) {
                                                    Ok(_) => {
                                                        self.status_message = "Wallet imported successfully!".to_string();
                                                        // Navigate to main screen after successful import
                                                        next_screen = Some(Screen::MainScreen);
                                                    }
                                                    Err(e) => {
                                                        self.status_message = format!("Error: {}", e);
                                                    }
                                                }
                                            }
                                        }
                                        Err(e) => {
                                            self.status_message = format!("Error: {}", e);
                                        }
                                    }
                                }
                                
                                ui.add_space(spacing);
                            });
                        });
                    
                    ui.add_space(20.0);
                });
            });
        });

        next_screen
    }
} 